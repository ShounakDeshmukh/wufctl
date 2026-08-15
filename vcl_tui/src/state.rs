use crate::config::Config;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

/// Only two real screens Images/Connect are popups or redirects
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
    ExtendForm {
        id: i64,
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
    /// Inline error (no toast UI).
    pub error: Option<String>,
    /// Last load time; gates 10-minute cache.
    pub last_loaded: Option<std::time::Instant>,
    /// Last `n` feedback (`Ok` success, `Err` danger).
    pub message: Option<Result<String, String>>,
    /// Debounce for `n` browser opens.
    pub last_avd_open: Option<std::time::Instant>,
    /// `/` filter; `list_state` indexes `visible_indices()`.
    pub search: String,
    /// Cursor char index in `search`.
    pub search_cursor: usize,
    /// True while editing `search`.
    pub searching: bool,
}

impl ImagesUiState {
    /// Indices into `images` matching `search`, or all of them when it's empty.
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

/// View-model row combining `get_request_ids()`'s fields with per-id status; `start` doubles as "requested at".
#[derive(Debug, Clone)]
pub struct Reservation {
    pub id: i64,
    pub image_name: String,
    pub status: crate::vcl::RequestStatus,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Default)]
pub struct ReservationsUiState {
    pub reservations: Vec<Reservation>,
    pub list_state: ratatui::widgets::ListState,
    /// No toast system yet - shown inline in place of the list.
    pub error: Option<String>,
    /// None until the first load; also gates the 20s auto-refresh and the `r` debounce.
    pub last_poll: Option<std::time::Instant>,
    pub message: Option<Result<String, String>>,
}
/// Notification toast
pub struct Toast {
    pub text: Result<String, String>,
    pub expires_at: std::time::Instant,
}

pub const DURATION_PRESETS: &[(i64, &str)] = &[
    (30, "30 min"),
    (45, "45 min"),
    (60, "1 hr"),
    (120, "2 hr"),
    (240, "4 hr"),
    (360, "6 hr"),
    (480, "8 hr"),
    (600, "10 hr"),
];

pub const MINUTE_STEPS: [u8; 4] = [0, 15, 30, 45];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartChoice {
    Now,
    Later,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmPm {
    Am,
    Pm,
}

/// One row of the New Reservation form; only a subset is visible at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormRow {
    Start,
    Day,
    Hour,
    Minute,
    AmPm,
    Duration,
    CustomMinutes,
    Create,
}

/// Spinner form; resets to `default()` when left via `Esc`.
pub struct NewReservationFormState {
    pub focus: FormRow,
    pub start: StartChoice,
    /// 0 = today, up to 7 days ahead.
    pub day_offset: u8,
    pub hour: u8,
    pub minute_idx: u8,
    pub am_pm: AmPm,
    /// Index into `DURATION_PRESETS`; `.len()` = "Custom".
    pub duration_idx: usize,
    pub custom_minutes: String,
    /// Char index of the cursor within `custom_minutes`.
    pub custom_cursor: usize,
    /// Last submit error.
    pub message: Option<String>,
}

impl Default for NewReservationFormState {
    fn default() -> Self {
        Self {
            focus: FormRow::Start,
            start: StartChoice::Now,
            day_offset: 0,
            hour: 12,
            minute_idx: 0,
            am_pm: AmPm::Pm,
            duration_idx: 2, // 1 hr
            custom_minutes: String::new(),
            custom_cursor: 0,
            message: None,
        }
    }
}

impl NewReservationFormState {
    pub fn visible_rows(&self) -> Vec<FormRow> {
        let mut rows = vec![FormRow::Start];
        if self.start == StartChoice::Later {
            rows.extend([FormRow::Day, FormRow::Hour, FormRow::Minute, FormRow::AmPm]);
        }
        rows.push(FormRow::Duration);
        if self.duration_idx == DURATION_PRESETS.len() {
            rows.push(FormRow::CustomMinutes);
        }
        rows.push(FormRow::Create);
        rows
    }

    /// None for invalid/empty values; caller shows a form error.
    pub fn duration_minutes(&self) -> Option<i64> {
        if self.duration_idx == DURATION_PRESETS.len() {
            self.custom_minutes
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|&m| m > 0)
        } else {
            DURATION_PRESETS.get(self.duration_idx).map(|(m, _)| *m)
        }
    }

    /// "now", or a timestamp for the configured time; None if it's a DST gap or already past.
    pub fn start_value(&self) -> Option<String> {
        match self.start {
            StartChoice::Now => Some("now".to_string()),
            StartChoice::Later => self.later_timestamp().map(|ts| ts.to_string()),
        }
    }

    fn later_timestamp(&self) -> Option<i64> {
        use chrono::{Days, Local, NaiveTime, TimeZone};
        let today = Local::now().date_naive();
        let date = today.checked_add_days(Days::new(self.day_offset as u64))?;
        let hour_24 = match self.am_pm {
            AmPm::Am if self.hour == 12 => 0,
            AmPm::Am => self.hour,
            AmPm::Pm if self.hour == 12 => 12,
            AmPm::Pm => self.hour + 12,
        };
        let minute = MINUTE_STEPS[self.minute_idx as usize];
        let time = NaiveTime::from_hms_opt(hour_24 as u32, minute as u32, 0)?;
        // `.earliest()` picks the valid interpretation; only a DST gap returns None.
        let local = Local.from_local_datetime(&date.and_time(time)).earliest()?;
        if local.timestamp() <= Local::now().timestamp() {
            return None;
        }
        Some(local.timestamp())
    }
}

pub const EXTEND_PRESETS: &[(i64, &str)] = &[
    (15, "15 min"),
    (30, "30 min"),
    (45, "45 min"),
    (60, "1 hr"),
    (120, "2 hr"),
    (240, "4 hr"),
    (360, "6 hr"),
    (480, "8 hr"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendRow {
    Duration,
    Confirm,
}

pub struct ExtendFormState {
    pub focus: ExtendRow,
    pub duration_idx: usize,
    /// Feedback for the last submit attempt - only ever an error.
    pub message: Option<String>,
}

impl Default for ExtendFormState {
    fn default() -> Self {
        Self {
            focus: ExtendRow::Duration,
            duration_idx: 0, // 15 min - the web UI's own default
            message: None,
        }
    }
}

/// A background network call, polled once per tick; at most one in flight app-wide.
pub enum PendingOp {
    SetupTest {
        token: String,
        rx: Receiver<color_eyre::Result<vcl_lib::VclClient>>,
    },
    LoadImages(Receiver<color_eyre::Result<Vec<crate::vcl::Image>>>),
    LoadReservations(Receiver<color_eyre::Result<Vec<Reservation>>>),
    EndReservation {
        id: i64,
        index: usize,
        rx: Receiver<color_eyre::Result<crate::vcl::ActionResult>>,
    },
    AddRequest(Receiver<color_eyre::Result<crate::vcl::ActionResult>>),
    ExtendRequest {
        id: i64,
        rx: Receiver<color_eyre::Result<crate::vcl::ActionResult>>,
    },
}

pub struct App {
    pub screen: Screen,
    pub popup: Popup,
    pub config: Option<Config>,
    pub exit: bool,
    pub async_runtime: Arc<tokio::runtime::Runtime>,
    pub client: Option<Arc<vcl_lib::VclClient>>,
    pub setup: SetupUiState,
    pub images: ImagesUiState,
    pub reservations: ReservationsUiState,
    pub new_reservation: NewReservationFormState,
    pub extend: ExtendFormState,
    pub toast: Option<Toast>,
    pub pending: Option<PendingOp>,
    pub throbber_state: throbber_widgets_tui::ThrobberState,
}
