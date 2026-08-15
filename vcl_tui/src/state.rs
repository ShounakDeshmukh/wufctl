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

/// Preset duration options, mirroring the exact `<select>` the VCL web UI
/// rendered for this account (confirmed live, not a guess) - (minutes,
/// display label). No client-side max beyond this list; picking a value
/// the account can't actually use is caught by the server's error
/// response, same as any other `add_request` rejection.
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
/// 15-minute steps, matching the web UI's minute dropdown.
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

/// One row of the New Reservation form. Only a subset is ever visible at
/// once - see `NewReservationFormState::visible_rows`.
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

/// Spinner-style form: `Up`/`Down` moves `focus` through `visible_rows()`,
/// `Left`/`Right` cycles the focused row's value (or moves the text
/// cursor, for `CustomMinutes`). Reset to `default()` every time the form
/// is left via `Esc`, so a stale attempt never lingers into the next one.
pub struct NewReservationFormState {
    pub focus: FormRow,
    pub start: StartChoice,
    /// 0 = today, up to 7 - matches the confirmed "up to 7 days ahead"
    /// window.
    pub day_offset: u8,
    pub hour: u8,
    /// Index into `MINUTE_STEPS`.
    pub minute_idx: u8,
    pub am_pm: AmPm,
    /// Index into `DURATION_PRESETS`; `DURATION_PRESETS.len()` means
    /// "Custom" (see `custom_minutes`).
    pub duration_idx: usize,
    pub custom_minutes: String,
    /// Char index (not byte index) of the insertion point within
    /// `custom_minutes`.
    pub custom_cursor: usize,
    /// Inline feedback for the last submit attempt - only ever an error;
    /// success closes the popup and shows a toast instead.
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
            duration_idx: 2, // 1 hr - matches the web UI's own default
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

    /// `None` for an empty/zero/unparseable custom value - the caller
    /// turns that into a form error rather than submitting garbage.
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

    /// `"now"`, or a Unix timestamp string for the configured day/time.
    /// `None` only for a local time that doesn't exist (a spring-forward
    /// DST gap landing exactly on the chosen minute) - rare enough that
    /// surfacing it as a form error is fine rather than special-casing it.
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
        // `.earliest()` picks the earliest valid interpretation for an
        // ambiguous (fall-back) local time; only a nonexistent
        // (spring-forward gap) time returns None.
        let local = Local.from_local_datetime(&date.and_time(time)).earliest()?;
        Some(local.timestamp())
    }
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
    pub new_reservation: NewReservationFormState,
    pub toast: Option<Toast>,
}
