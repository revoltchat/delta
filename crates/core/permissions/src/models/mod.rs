mod channel;
mod server;
mod user;

pub use channel::*;
use revolt_result::{create_error, Result};
pub use server::*;
pub use user::*;

/// Holds a permission value to manipulate.
#[derive(Copy, Clone, Debug)]
pub struct PermissionValue(u64);

impl PermissionValue {
    /// Apply a given override to this value
    pub fn apply(&mut self, v: Override) {
        self.allow(v.allow);
        self.revoke(v.deny);
    }

    /// Allow given permissions
    pub fn allow(&mut self, v: u64) {
        self.0 |= v;
    }

    /// Revoke given permissions
    pub fn revoke(&mut self, v: u64) {
        self.0 &= !v;
    }

    /// Revoke all permissions
    pub fn revoke_all(&mut self) {
        self.0 = 0;
    }

    /// Restrict to given permissions
    pub fn restrict(&mut self, v: u64) {
        self.0 &= v;
    }

    /// Check whether certain a permission has been granted
    pub fn has(&self, v: u64) -> bool {
        (self.0 & v) == v
    }

    /// Check whether certain a user permission has been granted
    pub fn has_user_permission(&self, permission: UserPermission) -> bool {
        self.has(permission as u64)
    }

    /// Check whether certain a channel permission has been granted
    pub fn has_channel_permission(&self, permission: ChannelPermission) -> bool {
        self.has(permission as u64)
    }

    /// Throw if missing user permission
    pub fn throw_if_lacking_user_permission(&self, permission: UserPermission) -> Result<()> {
        if self.has_user_permission(permission) {
            Ok(())
        } else {
            Err(create_error!(MissingPermission {
                permission: permission.to_string()
            }))
        }
    }

    /// Throw if missing channel permission
    pub fn throw_if_lacking_channel_permission(&self, permission: ChannelPermission) -> Result<()> {
        if self.has_channel_permission(permission) {
            Ok(())
        } else {
            Err(create_error!(MissingPermission {
                permission: permission.to_string()
            }))
        }
    }

    /// Throw an error if we cannot grant permissions on either allows or denies
    /// going from the previous given value to the next given value.
    ///
    /// We need to check any:
    /// - allows added (permissions now granted)
    /// - denies removed (permissions now neutral or granted)
    pub async fn throw_permission_override<C>(
        &self,
        current_value: C,
        next_value: &Override,
    ) -> Result<()>
    where
        C: Into<Option<Override>>,
    {
        let current_value = current_value.into();

        if let Some(current_value) = current_value {
            if !self.has(!current_value.allows() & next_value.allows())
                || !self.has(current_value.denies() & !next_value.denies())
            {
                return Err(create_error!(CannotGiveMissingPermissions));
            }
        } else if !self.has(next_value.allows()) {
            return Err(create_error!(CannotGiveMissingPermissions));
        }

        Ok(())
    }
}

impl From<i64> for PermissionValue {
    fn from(v: i64) -> Self {
        Self(v as u64)
    }
}

impl From<u64> for PermissionValue {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

impl From<PermissionValue> for u64 {
    fn from(v: PermissionValue) -> Self {
        v.0
    }
}

impl From<ChannelPermission> for PermissionValue {
    fn from(v: ChannelPermission) -> Self {
        (v as u64).into()
    }
}
