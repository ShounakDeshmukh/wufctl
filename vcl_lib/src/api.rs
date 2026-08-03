use log::debug;
use reqwest::Client;

use crate::errors::{Result, VclError};
use crate::params::{DeployServerOptions, UserGroupEdits, UserGroupMaxTimes};
use crate::value::Value;
use crate::xml_rpc::{build_request, parse_response};

pub struct VclClient {
    endpoint: String,
    token: String,
    http_client: Client,
}

impl VclClient {
    /// Create a new VCL client
    ///
    /// # Arguments
    /// * `endpoint` - XML-RPC endpoint URL (typically `https://vcl.ncsu.edu/scheduling/index.php?mode=xmlrpccall`)
    /// * `token` - Bearer token for authentication
    pub fn new(endpoint: impl Into<String>, token: impl Into<String>) -> Self {
        VclClient {
            endpoint: endpoint.into(),
            token: token.into(),
            http_client: Client::new(),
        }
    }

    /// Get the endpoint URL
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Make an XML-RPC call to the VCL API
    ///
    /// Handles all communication with the remote VCL API including:
    /// - XML-RPC serialization
    /// - Bearer token authentication
    /// - XML-RPC response parsing
    /// - Fault detection and reporting
    pub async fn call(&self, method: &str, args: Vec<Value>) -> Result<Value> {
        // Validate method name
        if method.is_empty() {
            return Err(VclError::InvalidParameter(
                "Method name cannot be empty".into(),
            ));
        }

        if self.token.is_empty() {
            return Err(VclError::MissingToken);
        }

        debug!(
            "Calling VCL API method: {} with {} args",
            method,
            args.len()
        );

        // Build XML-RPC request
        let body = build_request(method, args);
        debug!("Request body:\n{}", body);

        // Make HTTP request with timeout and proper headers
        let response = self
            .http_client
            .post(&self.endpoint)
            .header("Content-Type", "text/xml")
            .header("X-Authorization", format!("Bearer {}", self.token))
            .header("X-APIVERSION", "2")
            .timeout(std::time::Duration::from_secs(30))
            .body(body)
            .send()
            .await
            .map_err(|e| {
                debug!("HTTP request error: {}", e);
                VclError::HttpError(e.to_string())
            })?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            debug!("HTTP {} response body:\n{}", status, text);
            return Err(VclError::HttpError(format!(
                "HTTP {}: {}",
                status,
                text.chars().take(200).collect::<String>()
            )));
        }

        let response_text = response
            .text()
            .await
            .map_err(|e| VclError::HttpError(e.to_string()))?;

        debug!("Response received: {} bytes", response_text.len());

        // Parse response
        parse_response(&response_text)
    }

    /// Test RPC connection
    ///
    /// Useful for verifying the connection to the VCL API server.
    pub async fn test(&self, message: &str) -> Result<Value> {
        require_non_empty(message, "Message")?;
        self.call("XMLRPCtest", vec![Value::String(message.to_string())])
            .await
    }

    /// Get client IP address as seen by the VCL server
    ///
    /// Returns a struct with `ip` field containing the client IP.
    ///
    /// Note: `XMLRPCgetIP` is not documented in the NCSU VCL XML-RPC
    /// wrapper reference this client was built against; verify it is
    /// available on the target server before relying on it.
    pub async fn get_ip(&self) -> Result<Value> {
        self.call("XMLRPCgetIP", vec![]).await
    }

    /// Get all affiliations for which users can log in to VCL
    ///
    /// The only VCL API call that does not require authentication headers.
    pub async fn affiliations(&self) -> Result<Value> {
        self.call("XMLRPCaffiliations", vec![]).await
    }

    /// Get all available VM images for reservation
    ///
    /// Returns an array of image structs with fields like `id`, `name`, `prettyname`
    pub async fn get_images(&self) -> Result<Value> {
        self.call("XMLRPCgetImages", vec![]).await
    }

    /// Create a new reservation for a VM image
    ///
    /// # Arguments
    /// * `image_id` - The image ID to reserve
    /// * `start` - Start time: "now" or Unix timestamp as string
    /// * `length` - Duration in minutes
    /// * `for_user` - Make the request on behalf of this user, if permitted
    /// * `no_user_check` - Skip the user validity check
    ///
    /// Returns a struct with `requestid` field containing the new reservation ID
    pub async fn add_request(
        &self,
        image_id: i64,
        start: &str,
        length: i64,
        for_user: Option<&str>,
        no_user_check: bool,
    ) -> Result<Value> {
        require_positive(image_id, "Image ID")?;
        require_non_empty(start, "Start time")?;
        require_positive(length, "Length")?;
        self.call(
            "XMLRPCaddRequest",
            vec![
                Value::Int(image_id),
                Value::String(start.to_string()),
                Value::Int(length),
                optional_string_arg(for_user),
                bool_arg(no_user_check),
            ],
        )
        .await
    }

    /// Create a new reservation with a specific ending time
    ///
    /// # Arguments
    /// * `image_id` - The image ID to reserve
    /// * `start` - Start time: "now" or Unix timestamp as string
    /// * `end` - Unix timestamp for the end of the reservation
    /// * `for_user` - Make the request on behalf of this user, if permitted
    /// * `no_user_check` - Skip the user validity check
    pub async fn add_request_with_ending(
        &self,
        image_id: i64,
        start: &str,
        end: i64,
        for_user: Option<&str>,
        no_user_check: bool,
    ) -> Result<Value> {
        require_positive(image_id, "Image ID")?;
        require_non_empty(start, "Start time")?;
        require_positive(end, "End time")?;
        self.call(
            "XMLRPCaddRequestWithEnding",
            vec![
                Value::Int(image_id),
                Value::String(start.to_string()),
                Value::Int(end),
                optional_string_arg(for_user),
                bool_arg(no_user_check),
            ],
        )
        .await
    }

    /// Request deployment of a server reservation
    ///
    /// # Arguments
    /// * `image_id` - The image ID to reserve
    /// * `start` - Start time: "now" or Unix timestamp as string
    /// * `end` - Unix timestamp for the end of the reservation
    /// * `options` - Optional server parameters (admin/login group, networking, etc.)
    pub async fn deploy_server(
        &self,
        image_id: i64,
        start: &str,
        end: i64,
        options: DeployServerOptions<'_>,
    ) -> Result<Value> {
        require_positive(image_id, "Image ID")?;
        require_non_empty(start, "Start time")?;
        require_positive(end, "End time")?;
        self.call(
            "XMLRPCdeployServer",
            vec![
                Value::Int(image_id),
                Value::String(start.to_string()),
                Value::Int(end),
                optional_string_arg(options.admin_group),
                optional_string_arg(options.login_group),
                optional_string_arg(options.ip_addr),
                optional_string_arg(options.mac_addr),
                bool_arg(options.monitored),
                optional_string_arg(options.for_user),
                optional_string_arg(options.name),
                optional_string_arg(options.user_data),
            ],
        )
        .await
    }

    /// Get information about all of the current user's requests
    ///
    /// Returns a struct with `requests` array containing reservation IDs
    pub async fn get_request_ids(&self) -> Result<Value> {
        self.call("XMLRPCgetRequestIds", vec![]).await
    }

    /// Get status of a specific reservation
    ///
    /// # Arguments
    /// * `request_id` - The reservation ID to query
    ///
    /// Returns a struct with fields like `status`, `imageid`, `prettyimage`, etc.
    pub async fn get_request_status(&self, request_id: i64) -> Result<Value> {
        require_positive(request_id, "Request ID")?;
        self.call("XMLRPCgetRequestStatus", vec![Value::Int(request_id)])
            .await
    }

    /// Get connection data for a reservation
    ///
    /// If the request is ready, adds the connecting user's computer to the
    /// request and returns info about how to connect to it.
    ///
    /// # Arguments
    /// * `request_id` - The reservation ID
    /// * `remote_ip` - The connecting client's IP address
    ///
    /// Returns a struct with connection details: `ip`, `hostname`, `password`, `portNumber`
    pub async fn get_request_connect_data(
        &self,
        request_id: i64,
        remote_ip: &str,
    ) -> Result<Value> {
        require_positive(request_id, "Request ID")?;
        require_non_empty(remote_ip, "Remote IP")?;
        self.call(
            "XMLRPCgetRequestConnectData",
            vec![Value::Int(request_id), Value::String(remote_ip.to_string())],
        )
        .await
    }

    /// Extend the length of an active reservation
    ///
    /// If the request has not started yet, delete it and submit a new one
    /// instead of extending it.
    ///
    /// # Arguments
    /// * `request_id` - The reservation ID
    /// * `extend_time` - Minutes to extend the reservation by
    pub async fn extend_request(&self, request_id: i64, extend_time: i64) -> Result<Value> {
        require_positive(request_id, "Request ID")?;
        require_positive(extend_time, "Extend time")?;
        self.call(
            "XMLRPCextendRequest",
            vec![Value::Int(request_id), Value::Int(extend_time)],
        )
        .await
    }

    /// Modify the end time of an active reservation
    ///
    /// If the request has not started yet, delete it and submit a new one
    /// instead of modifying it.
    ///
    /// # Arguments
    /// * `request_id` - The reservation ID
    /// * `end` - Unix timestamp for the end of the reservation (rounded up
    ///   to the nearest 15 minute increment by the server)
    pub async fn set_request_ending(&self, request_id: i64, end: i64) -> Result<Value> {
        require_positive(request_id, "Request ID")?;
        require_positive(end, "End time")?;
        self.call(
            "XMLRPCsetRequestEnding",
            vec![Value::Int(request_id), Value::Int(end)],
        )
        .await
    }

    /// End/release a reservation immediately
    ///
    /// # Arguments
    /// * `request_id` - The reservation ID to end
    pub async fn end_request(&self, request_id: i64) -> Result<Value> {
        require_positive(request_id, "Request ID")?;
        self.call("XMLRPCendRequest", vec![Value::Int(request_id)])
            .await
    }

    /// Capture a reservation's environment as a new image
    ///
    /// Creates entries in the appropriate tables and sets the request
    /// state to `image`.
    ///
    /// # Arguments
    /// * `request_id` - The reservation ID to capture
    pub async fn auto_capture(&self, request_id: i64) -> Result<Value> {
        require_positive(request_id, "Request ID")?;
        self.call("XMLRPCautoCapture", vec![Value::Int(request_id)])
            .await
    }

    /// Get all images in a resource group
    ///
    /// # Arguments
    /// * `name` - The resource group name
    pub async fn get_group_images(&self, name: &str) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        self.call(
            "XMLRPCgetGroupImages",
            vec![Value::String(name.to_string())],
        )
        .await
    }

    /// Add an image to a resource group
    ///
    /// # Arguments
    /// * `name` - The resource group name
    /// * `image_id` - The image ID to add
    pub async fn add_image_to_group(&self, name: &str, image_id: i64) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_positive(image_id, "Image ID")?;
        self.call(
            "XMLRPCaddImageToGroup",
            vec![Value::String(name.to_string()), Value::Int(image_id)],
        )
        .await
    }

    /// Remove an image from a resource group
    ///
    /// # Arguments
    /// * `name` - The resource group name
    /// * `image_id` - The image ID to remove
    pub async fn remove_image_from_group(&self, name: &str, image_id: i64) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_positive(image_id, "Image ID")?;
        self.call(
            "XMLRPCremoveImageFromGroup",
            vec![Value::String(name.to_string()), Value::Int(image_id)],
        )
        .await
    }

    /// Map an image group to a computer group
    ///
    /// # Arguments
    /// * `image_group` - The image group name
    /// * `computer_group` - The computer group name
    pub async fn add_image_group_to_computer_group(
        &self,
        image_group: &str,
        computer_group: &str,
    ) -> Result<Value> {
        require_non_empty(image_group, "Image group")?;
        require_non_empty(computer_group, "Computer group")?;
        self.call(
            "XMLRPCaddImageGroupToComputerGroup",
            vec![
                Value::String(image_group.to_string()),
                Value::String(computer_group.to_string()),
            ],
        )
        .await
    }

    /// Remove the mapping of an image group to a computer group
    ///
    /// # Arguments
    /// * `image_group` - The image group name
    /// * `computer_group` - The computer group name
    pub async fn remove_image_group_from_computer_group(
        &self,
        image_group: &str,
        computer_group: &str,
    ) -> Result<Value> {
        require_non_empty(image_group, "Image group")?;
        require_non_empty(computer_group, "Computer group")?;
        self.call(
            "XMLRPCremoveImageGroupFromComputerGroup",
            vec![
                Value::String(image_group.to_string()),
                Value::String(computer_group.to_string()),
            ],
        )
        .await
    }

    /// Get all nodes in the privilege tree
    ///
    /// # Arguments
    /// * `root` - Root node to list from; `None` lists from the tree root
    pub async fn get_nodes(&self, root: Option<&str>) -> Result<Value> {
        self.call("XMLRPCgetNodes", vec![optional_string_or_null_arg(root)])
            .await
    }

    /// Check whether a node already exists at a location in the privilege tree
    ///
    /// # Arguments
    /// * `node_name` - The node name to check for
    /// * `parent_node` - The parent node to check under
    pub async fn node_exists(&self, node_name: &str, parent_node: &str) -> Result<Value> {
        require_non_empty(node_name, "Node name")?;
        require_non_empty(parent_node, "Parent node")?;
        self.call(
            "XMLRPCnodeExists",
            vec![
                Value::String(node_name.to_string()),
                Value::String(parent_node.to_string()),
            ],
        )
        .await
    }

    /// Add a node to the privilege tree as a child of the given parent
    ///
    /// # Arguments
    /// * `node_name` - The new node's name
    /// * `parent_node` - The parent node to attach to
    pub async fn add_node(&self, node_name: &str, parent_node: &str) -> Result<Value> {
        require_non_empty(node_name, "Node name")?;
        require_non_empty(parent_node, "Parent node")?;
        self.call(
            "XMLRPCaddNode",
            vec![
                Value::String(node_name.to_string()),
                Value::String(parent_node.to_string()),
            ],
        )
        .await
    }

    /// Delete a node from the privilege tree
    ///
    /// # Arguments
    /// * `node_id` - The ID of the node to remove
    pub async fn remove_node(&self, node_id: i64) -> Result<Value> {
        require_positive(node_id, "Node ID")?;
        self.call("XMLRPCremoveNode", vec![Value::Int(node_id)])
            .await
    }

    /// Get a user group's privileges at a node in the privilege tree
    ///
    /// # Arguments
    /// * `name` - The user group name
    /// * `affiliation` - The user group's affiliation
    /// * `node_id` - The node ID to query
    pub async fn get_user_group_privs(
        &self,
        name: &str,
        affiliation: &str,
        node_id: i64,
    ) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_non_empty(affiliation, "Affiliation")?;
        require_positive(node_id, "Node ID")?;
        self.call(
            "XMLRPCgetUserGroupPrivs",
            vec![
                Value::String(name.to_string()),
                Value::String(affiliation.to_string()),
                Value::Int(node_id),
            ],
        )
        .await
    }

    /// Add privileges for a user group at a node in the privilege tree
    ///
    /// # Arguments
    /// * `name` - The user group name
    /// * `affiliation` - The user group's affiliation
    /// * `node_id` - The node ID to grant privileges at
    /// * `permissions` - Comma-separated privilege names to add
    pub async fn add_user_group_priv(
        &self,
        name: &str,
        affiliation: &str,
        node_id: i64,
        permissions: &str,
    ) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_non_empty(affiliation, "Affiliation")?;
        require_positive(node_id, "Node ID")?;
        require_non_empty(permissions, "Permissions")?;
        self.call(
            "XMLRPCaddUserGroupPriv",
            vec![
                Value::String(name.to_string()),
                Value::String(affiliation.to_string()),
                Value::Int(node_id),
                Value::String(permissions.to_string()),
            ],
        )
        .await
    }

    /// Remove privileges for a user group at a node in the privilege tree
    ///
    /// # Arguments
    /// * `name` - The user group name
    /// * `affiliation` - The user group's affiliation
    /// * `node_id` - The node ID to revoke privileges at
    /// * `permissions` - Comma-separated privilege names to remove
    pub async fn remove_user_group_priv(
        &self,
        name: &str,
        affiliation: &str,
        node_id: i64,
        permissions: &str,
    ) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_non_empty(affiliation, "Affiliation")?;
        require_positive(node_id, "Node ID")?;
        require_non_empty(permissions, "Permissions")?;
        self.call(
            "XMLRPCremoveUserGroupPriv",
            vec![
                Value::String(name.to_string()),
                Value::String(affiliation.to_string()),
                Value::Int(node_id),
                Value::String(permissions.to_string()),
            ],
        )
        .await
    }

    /// Get a resource group's privileges at a node in the privilege tree
    ///
    /// # Arguments
    /// * `name` - The resource group name
    /// * `resource_type` - The resource type
    /// * `node_id` - The node ID to query
    pub async fn get_resource_group_privs(
        &self,
        name: &str,
        resource_type: &str,
        node_id: i64,
    ) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_non_empty(resource_type, "Resource type")?;
        require_positive(node_id, "Node ID")?;
        self.call(
            "XMLRPCgetResourceGroupPrivs",
            vec![
                Value::String(name.to_string()),
                Value::String(resource_type.to_string()),
                Value::Int(node_id),
            ],
        )
        .await
    }

    /// Add privileges for a resource group at a node in the privilege tree
    ///
    /// # Arguments
    /// * `name` - The resource group name
    /// * `resource_type` - The resource type
    /// * `node_id` - The node ID to grant privileges at
    /// * `permissions` - Comma-separated privilege names to add
    pub async fn add_resource_group_priv(
        &self,
        name: &str,
        resource_type: &str,
        node_id: i64,
        permissions: &str,
    ) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_non_empty(resource_type, "Resource type")?;
        require_positive(node_id, "Node ID")?;
        require_non_empty(permissions, "Permissions")?;
        self.call(
            "XMLRPCaddResourceGroupPriv",
            vec![
                Value::String(name.to_string()),
                Value::String(resource_type.to_string()),
                Value::Int(node_id),
                Value::String(permissions.to_string()),
            ],
        )
        .await
    }

    /// Remove privileges for a resource group from a node in the privilege tree
    ///
    /// # Arguments
    /// * `name` - The resource group name
    /// * `resource_type` - The resource type
    /// * `node_id` - The node ID to revoke privileges at
    /// * `permissions` - Comma-separated privilege names to remove
    pub async fn remove_resource_group_priv(
        &self,
        name: &str,
        resource_type: &str,
        node_id: i64,
        permissions: &str,
    ) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_non_empty(resource_type, "Resource type")?;
        require_positive(node_id, "Node ID")?;
        require_non_empty(permissions, "Permissions")?;
        self.call(
            "XMLRPCremoveResourceGroupPriv",
            vec![
                Value::String(name.to_string()),
                Value::String(resource_type.to_string()),
                Value::Int(node_id),
                Value::String(permissions.to_string()),
            ],
        )
        .await
    }

    /// Build a list of user groups
    ///
    /// # Arguments
    /// * `group_type` - Filter by group type, or `0` for all types
    /// * `affiliation_id` - Filter by affiliation ID, or `0` for all affiliations
    pub async fn get_user_groups(&self, group_type: i64, affiliation_id: i64) -> Result<Value> {
        self.call(
            "XMLRPCgetUserGroups",
            vec![Value::Int(group_type), Value::Int(affiliation_id)],
        )
        .await
    }

    /// Get information about a user group
    ///
    /// # Arguments
    /// * `name` - The user group name
    /// * `affiliation` - The user group's affiliation
    pub async fn get_user_group_attributes(&self, name: &str, affiliation: &str) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_non_empty(affiliation, "Affiliation")?;
        self.call(
            "XMLRPCgetUserGroupAttributes",
            vec![
                Value::String(name.to_string()),
                Value::String(affiliation.to_string()),
            ],
        )
        .await
    }

    /// Create a new user group
    ///
    /// # Arguments
    /// * `name` - The group name
    /// * `affiliation` - The group's affiliation
    /// * `owner` - The group owner
    /// * `managing_group` - The managing group
    /// * `max_times` - Time-limit fields for the group
    pub async fn add_user_group(
        &self,
        name: &str,
        affiliation: &str,
        owner: &str,
        managing_group: &str,
        max_times: UserGroupMaxTimes,
    ) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_non_empty(affiliation, "Affiliation")?;
        require_non_empty(owner, "Owner")?;
        require_non_empty(managing_group, "Managing group")?;
        self.call(
            "XMLRPCaddUserGroup",
            vec![
                Value::String(name.to_string()),
                Value::String(affiliation.to_string()),
                Value::String(owner.to_string()),
                Value::String(managing_group.to_string()),
                Value::Int(max_times.initial_max_time),
                Value::Int(max_times.total_max_time),
                Value::Int(max_times.max_extend_time),
                bool_arg(max_times.custom),
            ],
        )
        .await
    }

    /// Modify attributes of a user group
    ///
    /// Any field left `None` in `edits` is left unchanged by the server.
    ///
    /// # Arguments
    /// * `name` - The group name
    /// * `affiliation` - The group's affiliation
    /// * `new_name` - The group's new name
    /// * `new_affiliation` - The group's new affiliation
    /// * `edits` - Optional fields to update
    pub async fn edit_user_group(
        &self,
        name: &str,
        affiliation: &str,
        new_name: &str,
        new_affiliation: &str,
        edits: UserGroupEdits<'_>,
    ) -> Result<Value> {
        require_non_empty(name, "Group name")?;
        require_non_empty(affiliation, "Affiliation")?;
        require_non_empty(new_name, "New name")?;
        require_non_empty(new_affiliation, "New affiliation")?;
        self.call(
            "XMLRPCeditUserGroup",
            vec![
                Value::String(name.to_string()),
                Value::String(affiliation.to_string()),
                Value::String(new_name.to_string()),
                Value::String(new_affiliation.to_string()),
                optional_string_arg(edits.owner),
                optional_string_arg(edits.managing_group),
                optional_string_arg(edits.initial_max_time),
                optional_string_arg(edits.total_max_time),
                optional_string_arg(edits.max_extend_time),
            ],
        )
        .await
    }
}

/// Reject non-positive IDs/counts before making a network call.
fn require_positive(value: i64, field: &str) -> Result<()> {
    if value <= 0 {
        return Err(VclError::InvalidParameter(format!(
            "{} must be positive",
            field
        )));
    }
    Ok(())
}

/// Reject empty required strings before making a network call.
fn require_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        return Err(VclError::InvalidParameter(format!(
            "{} cannot be empty",
            field
        )));
    }
    Ok(())
}

/// Encode an optional string arg as XML-RPC, using VCL's `''`-default convention.
fn optional_string_arg(value: Option<&str>) -> Value {
    Value::String(value.unwrap_or("").to_string())
}

/// Encode an optional string arg as XML-RPC, using VCL's `NULL`-default convention.
fn optional_string_or_null_arg(value: Option<&str>) -> Value {
    match value {
        Some(s) => Value::String(s.to_string()),
        None => Value::Null,
    }
}

/// Encode a boolean flag as the `0`/`1` integer VCL's PHP API expects.
fn bool_arg(value: bool) -> Value {
    Value::Int(if value { 1 } else { 0 })
}
