//! Parameter structs for VCL API calls with more optional fields than fit as positional args.

/// Mirrors [`crate::VclClient::deploy_server`]'s PHP defaults: unset strings are empty, `monitored` is `false`.
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

/// Time-limit fields required by [`crate::VclClient::add_user_group`].
#[derive(Debug, Clone, Copy)]
pub struct UserGroupMaxTimes {
    pub initial_max_time: i64,
    pub total_max_time: i64,
    pub max_extend_time: i64,
    /// Corresponds to the PHP `$custom` flag (VCL default: `true`).
    pub custom: bool,
}

/// `None` fields are left unchanged by [`crate::VclClient::edit_user_group`], PHP's empty-string convention.
#[derive(Debug, Clone, Copy, Default)]
pub struct UserGroupEdits<'a> {
    pub owner: Option<&'a str>,
    pub managing_group: Option<&'a str>,
    pub initial_max_time: Option<i64>,
    pub total_max_time: Option<i64>,
    pub max_extend_time: Option<i64>,
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
