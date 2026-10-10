//! Who may assign or remove which role (members.md, "Who may do what").
//! Pure: the handlers pass the caller's effective role and the owner count
//! read under the space's membership lock.

use crate::error::AppError;
use crate::space::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    /// Below admin, or an admin touching an owner (or an owner invitation).
    Forbidden,
    /// Granting above the caller's own role (admin → at most admin).
    TooHigh,
    /// The change would leave the space without an owner.
    LastOwner,
}

impl Denied {
    /// The client error: `last_owner` is a 422 `role` field where a role is
    /// being set (`in_field`), else the 409 `last_owner` (remove, leave).
    pub fn error(self, in_field: bool) -> AppError {
        match self {
            Self::Forbidden => AppError::Forbidden,
            Self::TooHigh => AppError::field("role", "too_high"),
            Self::LastOwner if in_field => AppError::field("role", "last_owner"),
            Self::LastOwner => AppError::LastOwner,
        }
    }
}

/// The roles `role` may not grant (a member's invitations of such a role die
/// with a demotion).
pub fn not_grantable(role: Role) -> Vec<Role> {
    [Role::Accountant, Role::Member, Role::Admin, Role::Owner]
        .into_iter()
        .filter(|r| can_invite(role, None, *r).is_err())
        .collect()
}

/// May `caller` touch (resend, revoke, replace) a pending owner invitation?
pub fn may_touch_owner_invite(caller: Role) -> bool {
    can_invite(caller, Some(Role::Owner), Role::Owner).is_ok()
}

/// Change a member (or a pending invitation) whose current role is
/// `current` (`None`: a new invitation) to `new`. `owners` = the owners of
/// the space now.
pub fn can_assign(
    caller: Role,
    current: Option<Role>,
    new: Role,
    owners: u64,
) -> Result<(), Denied> {
    if caller < Role::Admin || (current == Some(Role::Owner) && caller != Role::Owner) {
        return Err(Denied::Forbidden);
    }
    if new > caller {
        return Err(Denied::TooHigh);
    }
    if current == Some(Role::Owner) && new != Role::Owner && owners <= 1 {
        return Err(Denied::LastOwner);
    }
    Ok(())
}

/// Remove a member whose role is `target`.
pub fn can_remove(caller: Role, target: Role, owners: u64) -> Result<(), Denied> {
    if caller < Role::Admin || (target == Role::Owner && caller != Role::Owner) {
        return Err(Denied::Forbidden);
    }
    can_leave(target, owners)
}

/// Leave the space holding `role` (any role may, except the last owner).
pub fn can_leave(role: Role, owners: u64) -> Result<(), Denied> {
    if role == Role::Owner && owners <= 1 {
        return Err(Denied::LastOwner);
    }
    Ok(())
}

/// Invite with `new`, replacing a pending invitation of role `pending`.
/// Owner invitations are an owner's business, like owner members.
pub fn can_invite(caller: Role, pending: Option<Role>, new: Role) -> Result<(), Denied> {
    can_assign(caller, pending, new, u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Role::{Accountant, Admin, Member, Owner};

    #[test]
    fn admin_assigns_up_to_admin_and_never_touches_owners() {
        assert_eq!(can_assign(Admin, Some(Member), Admin, 1), Ok(()));
        assert_eq!(can_assign(Admin, Some(Admin), Accountant, 1), Ok(()));
        assert_eq!(can_assign(Admin, None, Admin, 1), Ok(()));
        assert_eq!(
            can_assign(Admin, Some(Member), Owner, 1),
            Err(Denied::TooHigh)
        );
        assert_eq!(can_assign(Admin, None, Owner, 1), Err(Denied::TooHigh));
        assert_eq!(
            can_assign(Admin, Some(Owner), Member, 2),
            Err(Denied::Forbidden)
        );
        assert_eq!(
            can_assign(Admin, Some(Owner), Owner, 2),
            Err(Denied::Forbidden)
        );
    }

    #[test]
    fn owner_assigns_everything_but_keeps_one_owner() {
        assert_eq!(can_assign(Owner, Some(Member), Owner, 1), Ok(()));
        assert_eq!(can_assign(Owner, Some(Owner), Admin, 2), Ok(()));
        assert_eq!(
            can_assign(Owner, Some(Owner), Admin, 1),
            Err(Denied::LastOwner)
        );
        assert_eq!(can_assign(Owner, Some(Owner), Owner, 1), Ok(()));
        assert_eq!(can_assign(Owner, None, Owner, 1), Ok(()));
    }

    #[test]
    fn below_admin_is_forbidden() {
        assert_eq!(
            can_assign(Member, Some(Accountant), Accountant, 1),
            Err(Denied::Forbidden)
        );
        assert_eq!(
            can_assign(Accountant, None, Accountant, 1),
            Err(Denied::Forbidden)
        );
        assert_eq!(can_remove(Member, Accountant, 1), Err(Denied::Forbidden));
    }

    #[test]
    fn removal_and_leaving() {
        assert_eq!(can_remove(Admin, Admin, 1), Ok(()));
        assert_eq!(can_remove(Admin, Owner, 2), Err(Denied::Forbidden));
        assert_eq!(can_remove(Owner, Owner, 2), Ok(()));
        assert_eq!(can_remove(Owner, Owner, 1), Err(Denied::LastOwner));
        assert_eq!(can_leave(Accountant, 0), Ok(()));
        assert_eq!(can_leave(Owner, 2), Ok(()));
        assert_eq!(can_leave(Owner, 1), Err(Denied::LastOwner));
    }

    #[test]
    fn errors_and_grantable_roles() {
        let code = |e: AppError| format!("{e:?}");
        assert!(matches!(Denied::Forbidden.error(true), AppError::Forbidden));
        assert!(matches!(
            Denied::LastOwner.error(false),
            AppError::LastOwner
        ));
        assert!(code(Denied::LastOwner.error(true)).contains("last_owner"));
        assert!(code(Denied::TooHigh.error(false)).contains("too_high"));
        assert_eq!(not_grantable(Owner), []);
        assert_eq!(not_grantable(Admin), [Owner]);
        assert_eq!(not_grantable(Member), [Accountant, Member, Admin, Owner]);
        assert!(may_touch_owner_invite(Owner));
        assert!(!may_touch_owner_invite(Admin));
    }

    #[test]
    fn invitations() {
        assert_eq!(can_invite(Admin, None, Member), Ok(()));
        assert_eq!(can_invite(Admin, Some(Member), Admin), Ok(()));
        assert_eq!(can_invite(Admin, None, Owner), Err(Denied::TooHigh));
        assert_eq!(
            can_invite(Admin, Some(Owner), Member),
            Err(Denied::Forbidden)
        );
        assert_eq!(can_invite(Owner, Some(Owner), Member), Ok(()));
    }
}
