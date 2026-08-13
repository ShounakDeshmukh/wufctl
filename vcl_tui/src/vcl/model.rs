#[derive(Debug, Clone)]
pub struct Image {
    pub id: i64,
    pub name: String,
    pub ostype: String,
    pub usage: String,
    pub description: String,
}

impl Image {
    /// `false` for every observed Windows image (all AVD pool resources,
    /// not per-request VMs) - branches the Details pane and `n` keybind.
    pub fn is_reservable(&self) -> bool {
        !self.ostype.eq_ignore_ascii_case("windows")
    }
}

/// `status` stays a raw `String`, not an enum - RESPONSE_SHAPES.md warns
/// the vocabulary isn't confirmed exhaustive. `time` is present in
/// `loading` but absent in `ready`.
#[derive(Debug, Clone)]
pub struct RequestStatus {
    pub status: String,
    pub time: Option<i64>,
}

/// Shared success/error envelope for `add_request`/`extend_request`/
/// `end_request` - only `add_request`'s success carries a `requestid`.
#[derive(Debug, Clone)]
pub enum ActionResult {
    Success { requestid: Option<i64> },
    Error { errorcode: i64, errormsg: String },
}

/// One `requests[]` element from `get_request_ids()` - confirmed live to
/// be a full struct, not a bare id. `requestid` (not `id`) is the field
/// name here, a different key than `get_request_status`'s response.
#[derive(Debug, Clone)]
pub struct RequestListEntry {
    pub requestid: i64,
    pub imagename: String,
}
