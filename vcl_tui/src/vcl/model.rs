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
