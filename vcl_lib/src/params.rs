//! Parameter structs for VCL API calls that take more than a handful of
//! optional fields. Grouping these into structs keeps `VclClient` method
//! signatures short instead of stacking five-plus positional arguments.

/// Optional parameters for [`crate::VclClient::deploy_server`]
/// (`XMLRPCdeployServer`). Every field mirrors a VCL PHP default: unset
/// strings default to empty, `monitored` defaults to `false`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeployServerOptions<'a> {
    pub admin_group: Option<&'a str>,
    pub login_group: Option<&'a str>,
    pub ip_addr: Option<&'a str>,
    pub mac_addr: Option<&'a str>,
    pub monitored: bool,
    pub for_user: Option<&'a str>,
    pub name: Option<&'a str>,
    pub user_data: Option<&'a str>,
}

/// Time-limit fields required by [`crate::VclClient::add_user_group`]
/// (`XMLRPCaddUserGroup`).
#[derive(Debug, Clone, Copy)]
pub struct UserGroupMaxTimes {
    pub initial_max_time: i64,
    pub total_max_time: i64,
    pub max_extend_time: i64,
    /// Corresponds to the PHP `$custom` flag (VCL default: `true`).
    pub custom: bool,
}

/// Optional fields to change via [`crate::VclClient::edit_user_group`]
/// (`XMLRPCeditUserGroup`). Any field left as `None` is left unchanged,
/// matching the VCL PHP convention of passing an empty string.
#[derive(Debug, Clone, Copy, Default)]
pub struct UserGroupEdits<'a> {
    pub owner: Option<&'a str>,
    pub managing_group: Option<&'a str>,
    pub initial_max_time: Option<&'a str>,
    pub total_max_time: Option<&'a str>,
    pub max_extend_time: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_server_options_default_is_unset() {
        let opts = DeployServerOptions::default();
        assert_eq!(opts.admin_group, None);
        assert_eq!(opts.login_group, None);
        assert_eq!(opts.ip_addr, None);
        assert_eq!(opts.mac_addr, None);
        assert!(!opts.monitored);
        assert_eq!(opts.for_user, None);
        assert_eq!(opts.name, None);
        assert_eq!(opts.user_data, None);
    }

    #[test]
    fn test_user_group_edits_default_is_unset() {
        let edits = UserGroupEdits::default();
        assert_eq!(edits.owner, None);
        assert_eq!(edits.managing_group, None);
        assert_eq!(edits.initial_max_time, None);
        assert_eq!(edits.total_max_time, None);
        assert_eq!(edits.max_extend_time, None);
    }
}
