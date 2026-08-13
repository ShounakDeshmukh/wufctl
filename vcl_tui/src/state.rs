use crate::config::Config;

/// Only two real screens now - Images/Connect are popups or redirects
/// over Reservations (see PLAN.md's nav-redesign scope decision), not
/// standalone tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Setup,
    Reservations,
}

/// At most one popup open at a time, layered over `Screen::Reservations`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Popup {
    #[default]
    None,
    ImagePicker,
    NewReservationForm {
        image_idx: usize,
    },
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
    /// `/`-triggered incremental filter, vim/less-style. `list_state`
    /// indexes into `visible_indices()`, not directly into `images`.
    pub search: String,
    /// Char index (not byte index) of the insertion point within `search`.
    pub search_cursor: usize,
    /// `true` while actively editing `search` (keys go to the query
    /// instead of list navigation); the filter still applies when `false`.
    pub searching: bool,
}

impl ImagesUiState {
    /// Indices into `images` matching `search` (case-insensitive substring
    /// of the name), or every index when `search` is empty.
    pub fn visible_indices(&self) -> Vec<usize> {
        if self.search.is_empty() {
            return (0..self.images.len()).collect();
        }
        let query = self.search.to_lowercase();
        self.images
            .iter()
            .enumerate()
            .filter(|(_, img)| img.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect()
    }
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
    /// `None` until the first load; also gates the 20s auto-refresh and the
    /// `r` manual-refresh debounce, so both share one cooldown clock.
    pub last_poll: Option<std::time::Instant>,
    /// Inline feedback for the last `x`/create-reservation action.
    pub message: Option<Result<String, String>>,
}

/// A transient, self-expiring notification shown in the bottom-right
/// corner, for feedback that shouldn't linger in a details pane (e.g. the
/// refresh debounce warning).
pub struct Toast {
    pub text: Result<String, String>,
    pub expires_at: std::time::Instant,
}

pub struct App {
    pub screen: Screen,
    pub popup: Popup,
    pub config: Option<Config>,
    pub exit: bool,
    pub async_runtime: tokio::runtime::Runtime,
    pub client: Option<vcl_lib::VclClient>,
    pub setup: SetupUiState,
    pub images: ImagesUiState,
    pub reservations: ReservationsUiState,
    pub toast: Option<Toast>,
}
