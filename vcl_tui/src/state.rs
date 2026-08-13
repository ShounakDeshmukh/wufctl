use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Setup,
    Images,
    Reservations,
    Connect,
}

#[derive(Debug, Default)]
pub enum SetupState {
    #[default]
    Idle,
    Validating,
    Error(String),
}

#[derive(Debug)]
pub struct SetupUiState {
    pub input: String,
    /// Char index (not byte index) of the insertion point within `input`.
    pub cursor: usize,
    /// `true` renders `input` as bullets; toggled with Ctrl+R.
    pub masked: bool,
    pub state: SetupState,
}

impl Default for SetupUiState {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            masked: true,
            state: SetupState::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ImagesUiState {
    pub images: Vec<crate::vcl::Image>,
    pub list_state: ratatui::widgets::ListState,
    /// No toast system yet - shown inline in place of the list.
    pub error: Option<String>,
    /// Set once `get_images()` has been attempted, success or failure, so
    /// `run()` doesn't refetch every tick on a genuinely empty catalog.
    pub loaded: bool,
    /// Inline feedback for the last `n` press (e.g. AVD guide opened),
    /// `Ok` styled as success and `Err` as danger.
    pub message: Option<Result<String, String>>,
    /// Debounces `n` so spamming it can't spawn a browser per keypress.
    pub last_avd_open: Option<std::time::Instant>,
}

/// Local view-model row, not a wire-format struct - combines
/// `get_request_ids()`'s list-level fields (id + image name) with the
/// finer-grained loading/ready state from a separate per-id
/// `get_request_status()` call.
#[derive(Debug, Clone)]
pub struct Reservation {
    pub id: i64,
    pub image_name: String,
    pub status: crate::vcl::RequestStatus,
}

#[derive(Debug, Default)]
pub struct ReservationsUiState {
    pub reservations: Vec<Reservation>,
    pub list_state: ratatui::widgets::ListState,
    /// No toast system yet - shown inline in place of the list.
    pub error: Option<String>,
    /// Set once a load has been attempted, success or failure, so `run()`
    /// doesn't refetch every tick on a genuinely empty list.
    pub loaded: bool,
    /// Inline feedback for the last `x`/create-reservation action.
    pub message: Option<Result<String, String>>,
}

pub struct App {
    pub screen: Screen,
    pub config: Option<Config>,
    pub exit: bool,
    pub async_runtime: tokio::runtime::Runtime,
    pub client: Option<vcl_lib::VclClient>,
    pub setup: SetupUiState,
    pub images: ImagesUiState,
    pub reservations: ReservationsUiState,
}
