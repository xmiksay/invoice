//! Memberships of a space: list, role change, removal, leaving. Changes
//! that can lower the owner count run under the space's membership lock
//! (`lock`), so two concurrent demotions cannot leave the space ownerless.

use chrono::{DateTime, FixedOffset};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    FromQueryResult, QueryFilter, Statement, TransactionTrait,
};
use uuid::Uuid;

use super::entity::space_invite;
use super::rules;
use crate::auth::entity::{api_token, session};
use crate::error::AppError;
use crate::space::entity::member;
use crate::space::{Role, SpaceId};

#[derive(Debug, Clone, FromQueryResult)]
struct Row {
    user_id: Uuid,
    email: String,
    display_name: String,
    role: String,
    joined_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone)]
pub struct MemberRow {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: Role,
    pub joined_at: DateTime<FixedOffset>,
}

/// Members of `space` (only `user` when given), by role (owner first),
/// then e-mail.
async fn query(
    db: &impl ConnectionTrait,
    space: SpaceId,
    user: Option<Uuid>,
) -> Result<Vec<MemberRow>, AppError> {
    let sql = "SELECT m.user_id, u.email, u.display_name, m.role, m.created_at AS joined_at \
         FROM space_members m JOIN users u ON u.id = m.user_id \
         WHERE m.space_id = $1 AND ($2::uuid IS NULL OR m.user_id = $2) \
         ORDER BY CASE m.role WHEN 'owner' THEN 0 WHEN 'admin' THEN 1 WHEN 'member' THEN 2 ELSE 3 END, \
         u.email";
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        vec![space.into(), user.into()],
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            Some(MemberRow {
                role: Role::parse(&r.role)?,
                user_id: r.user_id,
                email: r.email,
                display_name: r.display_name,
                joined_at: r.joined_at,
            })
        })
        .collect())
}

pub async fn list(db: &DatabaseConnection, space: SpaceId) -> Result<Vec<MemberRow>, AppError> {
    query(db, space, None).await
}

async fn find(
    db: &impl ConnectionTrait,
    space: SpaceId,
    user: Uuid,
) -> Result<Option<MemberRow>, AppError> {
    Ok(query(db, space, Some(user)).await?.into_iter().next())
}

/// Serialize membership changes of `space` (the space row, `NO KEY`: inserts
/// referencing the space are not blocked) and count its owners who can
/// still sign in (a disabled owner does not keep the space administered).
pub(super) async fn lock(txn: &DatabaseTransaction, space: SpaceId) -> Result<u64, AppError> {
    let backend = txn.get_database_backend();
    let locked = txn
        .query_one(Statement::from_sql_and_values(
            backend,
            "SELECT id FROM spaces WHERE id = $1 FOR NO KEY UPDATE",
            vec![space.into()],
        ))
        .await?;
    if locked.is_none() {
        return Err(AppError::NotFound);
    }
    let owners = txn
        .query_one(Statement::from_sql_and_values(
            backend,
            "SELECT count(*) AS n FROM space_members m JOIN users u ON u.id = m.user_id \
             WHERE m.space_id = $1 AND m.role = 'owner' AND NOT u.disabled",
            vec![space.into()],
        ))
        .await?
        .ok_or_else(|| anyhow::anyhow!("owner count returned no row"))?
        .try_get::<i64>("", "n")?;
    Ok(u64::try_from(owners).unwrap_or_default())
}

/// The caller's effective role re-read under the lock: a caller demoted or
/// removed meanwhile acts with what is left.
async fn caller_role(
    txn: &DatabaseTransaction,
    space: SpaceId,
    caller: Uuid,
    effective: Role,
) -> Result<Role, AppError> {
    let now = crate::space::repo::member_role(txn, space, caller)
        .await?
        .ok_or(AppError::Forbidden)?;
    Ok(effective.min(now))
}

/// Change `target`'s role to `new`. 404 unknown / foreign user; 403
/// `forbidden`, 422 `role: too_high | last_owner` by the role rules.
pub async fn change_role(
    db: &DatabaseConnection,
    space: SpaceId,
    (caller, effective): (Uuid, Role),
    target: Uuid,
    new: Role,
) -> Result<MemberRow, AppError> {
    let txn = db.begin().await?;
    let owners = lock(&txn, space).await?;
    let current = find(&txn, space, target).await?.ok_or(AppError::NotFound)?;
    let caller = caller_role(&txn, space, caller, effective).await?;
    rules::can_assign(caller, Some(current.role), new, owners).map_err(|d| d.error(true))?;
    member::Entity::update_many()
        .col_expr(member::Column::Role, Expr::value(new.as_str()))
        .filter(member::Column::SpaceId.eq(space))
        .filter(member::Column::UserId.eq(target))
        .exec(&txn)
        .await?;
    // Pending invitations the member may no longer grant die with the demotion.
    let lost: Vec<&str> = rules::not_grantable(new)
        .into_iter()
        .map(Role::as_str)
        .collect();
    if !lost.is_empty() {
        space_invite::Entity::delete_many()
            .filter(space_invite::Column::SpaceId.eq(space.uuid()))
            .filter(space_invite::Column::InvitedBy.eq(target))
            .filter(space_invite::Column::Role.is_in(lost))
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(MemberRow {
        role: new,
        ..current
    })
}

/// Remove `target` (not the caller: that is leaving). 404 unknown / foreign
/// user, 403 `forbidden`, 409 `last_owner`.
pub async fn remove(
    db: &DatabaseConnection,
    space: SpaceId,
    (caller, effective): (Uuid, Role),
    target: Uuid,
) -> Result<(), AppError> {
    let txn = db.begin().await?;
    let owners = lock(&txn, space).await?;
    let current = find(&txn, space, target).await?.ok_or(AppError::NotFound)?;
    let caller = caller_role(&txn, space, caller, effective).await?;
    rules::can_remove(caller, current.role, owners).map_err(|d| d.error(false))?;
    purge(&txn, space, target).await?;
    txn.commit().await?;
    Ok(())
}

/// The caller leaves `space`; 409 `last_owner`.
pub async fn leave(db: &DatabaseConnection, space: SpaceId, user: Uuid) -> Result<(), AppError> {
    let txn = db.begin().await?;
    let owners = lock(&txn, space).await?;
    let role = crate::space::repo::member_role(&txn, space, user)
        .await?
        .ok_or(AppError::Unauthorized)?;
    rules::can_leave(role, owners).map_err(|d| d.error(false))?;
    purge(&txn, space, user).await?;
    txn.commit().await?;
    Ok(())
}

/// May `inviter` (an active member of `space`) still grant `role`? Read
/// under [`lock`] when an invitation is accepted.
pub(super) async fn may_grant(
    txn: &DatabaseTransaction,
    space: SpaceId,
    inviter: Uuid,
    role: Role,
) -> Result<bool, AppError> {
    let row = txn
        .query_one(Statement::from_sql_and_values(
            txn.get_database_backend(),
            "SELECT m.role FROM space_members m JOIN users u ON u.id = m.user_id \
             WHERE m.space_id = $1 AND m.user_id = $2 AND NOT u.disabled",
            vec![space.into(), inviter.into()],
        ))
        .await?;
    let current = match row {
        Some(r) => Role::parse(&r.try_get::<String>("", "role")?),
        None => None,
    };
    Ok(current.is_some_and(|c| rules::can_invite(c, None, role).is_ok()))
}

/// Add `user` to `space` with `role`; an existing membership keeps its role.
pub async fn join(
    txn: &DatabaseTransaction,
    space: SpaceId,
    user: Uuid,
    role: &str,
) -> Result<(), AppError> {
    txn.execute(Statement::from_sql_and_values(
        txn.get_database_backend(),
        "INSERT INTO space_members (space_id, user_id, role) VALUES ($1, $2, $3) \
         ON CONFLICT (space_id, user_id) DO NOTHING",
        vec![space.into(), user.into(), role.into()],
    ))
    .await?;
    Ok(())
}

/// Revoke `user`'s access to `space` at once: the membership, the sessions
/// on the space's host, the API tokens in the space and the invitations
/// they sent (a kept link must not bring them back).
async fn purge(txn: &DatabaseTransaction, space: SpaceId, user: Uuid) -> Result<(), AppError> {
    member::Entity::delete_many()
        .filter(member::Column::SpaceId.eq(space))
        .filter(member::Column::UserId.eq(user))
        .exec(txn)
        .await?;
    session::Entity::delete_many()
        .filter(session::Column::SpaceId.eq(space.uuid()))
        .filter(session::Column::UserId.eq(user))
        .exec(txn)
        .await?;
    api_token::Entity::delete_many()
        .filter(api_token::Column::SpaceId.eq(space.uuid()))
        .filter(api_token::Column::UserId.eq(user))
        .exec(txn)
        .await?;
    space_invite::Entity::delete_many()
        .filter(space_invite::Column::SpaceId.eq(space.uuid()))
        .filter(space_invite::Column::InvitedBy.eq(user))
        .exec(txn)
        .await?;
    Ok(())
}
