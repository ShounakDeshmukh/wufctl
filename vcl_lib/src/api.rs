use log::debug;
use reqwest::Client;

use crate::errors::{Result, VclError};
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
        if message.is_empty() {
            return Err(VclError::InvalidParameter("Message cannot be empty".into()));
        }
        self.call("XMLRPCtest", vec![Value::String(message.to_string())])
            .await
    }

    // ========== READ-ONLY METHODS (IMPLEMENTED) ==========

    /// Get client IP address as seen by the VCL server
    ///
    /// Returns a struct with `ip` field containing the client IP
    pub async fn get_ip(&self) -> Result<Value> {
        self.call("XMLRPCgetIP", vec![]).await
    }

    /// Get all available VM images for reservation
    ///
    /// Returns an array of image structs with fields like `id`, `name`, `prettyname`
    pub async fn get_images(&self) -> Result<Value> {
        self.call("XMLRPCgetImages", vec![]).await
    }

    /// Get all reservation IDs for the current user
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
        if request_id <= 0 {
            return Err(VclError::InvalidParameter(
                "Request ID must be positive".into(),
            ));
        }
        self.call("XMLRPCgetRequestStatus", vec![Value::Int(request_id)])
            .await
    }

    /// Get connection data for a reservation
    ///
    /// # Arguments
    /// * `request_id` - The reservation ID
    /// * `client_ip` - The client's IP address (for authentication)
    ///
    /// Returns a struct with connection details: `ip`, `hostname`, `password`, `portNumber`
    pub async fn get_request_connect_data(
        &self,
        request_id: i64,
        client_ip: &str,
    ) -> Result<Value> {
        if request_id <= 0 {
            return Err(VclError::InvalidParameter(
                "Request ID must be positive".into(),
            ));
        }
        if client_ip.is_empty() {
            return Err(VclError::InvalidParameter(
                "Client IP cannot be empty".into(),
            ));
        }
        self.call(
            "XMLRPCgetRequestConnectData",
            vec![Value::Int(request_id), Value::String(client_ip.to_string())],
        )
        .await
    }

    /// Create a new reservation for a VM image
    ///
    /// # Arguments
    /// * `image_id` - The image ID to reserve
    /// * `start` - Start time: "now" or Unix timestamp as string
    /// * `duration` - Duration in minutes
    ///
    /// Returns a struct with `requestid` field containing the new reservation ID
    pub async fn add_request(&self, image_id: i64, start: &str, duration: i64) -> Result<Value> {
        if image_id <= 0 {
            return Err(VclError::InvalidParameter(
                "Image ID must be positive".into(),
            ));
        }
        if start.is_empty() {
            return Err(VclError::InvalidParameter(
                "Start time cannot be empty".into(),
            ));
        }
        if duration <= 0 {
            return Err(VclError::InvalidParameter(
                "Duration must be positive".into(),
            ));
        }
        self.call(
            "XMLRPCaddRequest",
            vec![
                Value::Int(image_id),
                Value::String(start.to_string()),
                Value::Int(duration),
            ],
        )
        .await
    }

    /// End/release a reservation immediately
    ///
    /// # Arguments
    /// * `request_id` - The reservation ID to end
    pub async fn end_request(&self, request_id: i64) -> Result<Value> {
        if request_id <= 0 {
            return Err(VclError::InvalidParameter(
                "Request ID must be positive".into(),
            ));
        }
        self.call("XMLRPCendRequest", vec![Value::Int(request_id)])
            .await
    }

    // ========== NOT IMPLEMENTED YET ==========

    /// Create a new reservation with specific end time
    pub async fn add_request_with_ending(
        &self,
        _image_id: i64,
        _start_unix: i64,
        _end_unix: i64,
    ) -> Result<Value> {
        todo!()
    }

    /// Extend an existing reservation by additional minutes
    pub async fn extend_request(&self, _request_id: i64, _duration: i64) -> Result<Value> {
        todo!()
    }

    /// Change the end time of a reservation
    pub async fn set_request_ending(&self, _request_id: i64, _end_unix: i64) -> Result<Value> {
        todo!()
    }

    /// Deploy server for a reservation
    pub async fn deploy_server(&self, _request_id: i64) -> Result<Value> {
        todo!()
    }
}
