#[derive(Debug, Clone)]
pub struct Image {
    pub id: i64,
    pub name: String,
    pub ostype: String,
    pub usage: String,
    pub description: String,
}

impl Image {
    /// False for every observed Windows image
    pub fn is_reservable(&self) -> bool {
        !self.ostype.eq_ignore_ascii_case("windows")
    }
}

/// `status` stays a raw String
#[derive(Debug, Clone)]
pub struct RequestStatus {
    pub status: String,
    pub time: Option<i64>,
}

/// Shared success/error envelope
#[derive(Debug, Clone)]
pub enum ActionResult {
    Success { requestid: Option<i64> },
    Error { errorcode: i64, errormsg: String },
}

/// One `requests[]` element from `get_request_ids()`
#[derive(Debug, Clone)]
pub struct RequestListEntry {
    pub requestid: i64,
    pub imagename: String,
    pub start: i64,
    pub end: i64,
}

/// One `connectMethods` entry from `get_request_connect_data()`; `id` is the map key, not a field.
#[derive(Debug, Clone)]
pub struct ConnectMethod {
    pub id: String,
    pub description: String,
    pub connectports: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConnectData {
    pub server_ip: String,
    pub user: String,
    pub password: String,
    pub connect_port: String,
    pub connect_methods: Vec<ConnectMethod>,
}

/// `notready` briefly precedes `ready`, same status/data split as `ActionResult`.
#[derive(Debug, Clone)]
pub enum ConnectDataResult {
    NotReady,
    Ready(ConnectData),
}
