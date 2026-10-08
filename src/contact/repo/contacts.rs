use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::sea_query::{Condition, Expr};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Select, Set,
};
use uuid::Uuid;

use crate::contact::entity::contact::{self, ActiveModel, Column, Entity};
use crate::contact::handlers::dto::{ContactInput, like_pattern};
use crate::error::{AppError, unique_violation};

/// One page of contacts matching `q` (ordered by name) and the total match count.
pub async fn list(
    db: &DatabaseConnection,
    q: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<(Vec<contact::Model>, u64), AppError> {
    let total = search(q).count(db).await?;
    let items = search(q)
        .order_by_asc(Column::Name)
        .order_by_asc(Column::Id)
        .limit(limit)
        .offset(offset)
        .all(db)
        .await?;
    Ok((items, total))
}

fn search(q: Option<&str>) -> Select<Entity> {
    let Some(term) = q else {
        return Entity::find();
    };
    let pattern = like_pattern(term);
    let cond = [Column::Name, Column::Ico, Column::Dic, Column::City]
        .into_iter()
        .fold(Condition::any(), |c, col| {
            c.add(Expr::col(col).ilike(pattern.clone()))
        });
    Entity::find().filter(cond)
}

pub async fn get(db: &DatabaseConnection, id: Uuid) -> Result<contact::Model, AppError> {
    Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound)
}

/// `input` must already be validated.
pub async fn create(
    db: &DatabaseConnection,
    input: ContactInput,
) -> Result<contact::Model, AppError> {
    let now = chrono::Utc::now().into();
    let mut row = active(input);
    row.id = Set(Uuid::new_v4());
    row.created_at = Set(now);
    row.updated_at = Set(now);
    row.insert(db).await.map_err(map_duplicate)
}

/// `input` must already be validated; replaces every editable field.
pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: ContactInput,
) -> Result<contact::Model, AppError> {
    let existing = get(db, id).await?;
    let mut row = active(input);
    row.id = Set(id);
    row.created_at = Set(existing.created_at);
    row.updated_at = Set(chrono::Utc::now().into());
    row.update(db).await.map_err(map_duplicate)
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<(), AppError> {
    let res = Entity::delete_by_id(id).exec(db).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

fn active(c: ContactInput) -> ActiveModel {
    ActiveModel {
        name: Set(c.name),
        ico: Set(c.ico),
        dic: Set(c.dic),
        street: Set(c.street),
        city: Set(c.city),
        zip: Set(c.zip),
        country: Set(c.country),
        email: Set(c.email),
        phone: Set(c.phone),
        note: Set(c.note),
        default_due_days: Set(c.default_due_days),
        default_locale: Set(c.default_locale),
        default_currency: Set(c.default_currency),
        ..Default::default()
    }
}

fn map_duplicate(err: DbErr) -> AppError {
    match unique_violation(&err) {
        Some(constraint) if constraint == "contacts_ico_key" => AppError::field("ico", "duplicate"),
        _ => err.into(),
    }
}
