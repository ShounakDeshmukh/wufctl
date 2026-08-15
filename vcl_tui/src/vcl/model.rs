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
