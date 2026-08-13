# vcl_tui implementation plan

## Context

`vcl_tui` is currently an unmodified `ratatui-cli new` boilerplate (a toy
Counter App) - nothing real has been built. The rest of the workspace is
ready to build against it:

- `vcl_lib` is a deliberately "dumb" XML-RPC client: every method returns
  the raw, untyped `vcl_lib::Value` enum. This was an explicit decision to
  keep `vcl_lib` thin and put response deserialization in the consumer
  (`vcl_tui`) instead. `vcl_lib` is not being touched by this plan.
- `vcl_tui/RESPONSE_SHAPES.md` documents the exact field-level shape of
  every response this TUI needs, captured live against `vcl.ncsu.edu`
  (including both `loading`/`ready` states of `get_request_status` and
  `notready`/`ready` states of `get_request_connect_data`, via a poll loop
  added to `vcl_probe`).
- A Claude Artifact (HTML/JS mockup, already approved) fully specifies the
  UI: 4 screens (Setup, Images, Reservations, Connect), a New Reservation
  popup, an SSH connect flow that suspends the TUI for a real `ssh` session,
  and - notably - a ratatui widget-mapping table the mockup itself included
  (Paragraph for bars, Tabs for the tab strip, List+ListState for the two
  list screens, Clear+centered Rect for the popup, etc). That mapping is
  treated as authoritative below, not re-derived.
- **Scoping finding from this session**: live `get_images()` data shows
  every `ostype: "windows"` image in this account's catalog is an Azure
  Virtual Desktop (AVD) pool resource, not a real per-request VM. Confirmed
  directly on the VCL web UI (2026-08-08): selecting an AVD image and
  clicking Connect never creates a reservation at all - it just shows
  static instructions to log into the AVD web portal with NCSU/Unity
  credentials (see the official guide:
  https://vcl.ncsu.edu/accessing-environments-in-windows-virtual-desktop/).
  There is no `add_request`/`get_request_status`/`get_request_connect_data`
  lifecycle involved for these, and no non-AVD Windows image exists in the
  catalog. **The real `add_request`/RDP reservation flow is therefore
  dropped from v1 entirely** - not deferred, there's nothing in this API to
  build it against. AVD images stay visible in the Images list (not
  filtered out), but selecting one shows an explanation instead of normal
  reserve details, with an option to open NCSU's AVD guide in the user's
  browser - see Scope decisions below. The mockup's Windows/RDP example
  (`Win11-MATLAB-R2025b`) was illustrative only and doesn't correspond to
  anything reservable in this account's real catalog.

The goal of this plan is to turn the approved mockup + confirmed response
shapes into a working `vcl_tui` binary, scoped to what's actually real:
Linux/SSH images, immediate ("now") reservations, and the 9
confirmed-working XML-RPC methods listed in `xmlrpcWrappers.md`.

## Module layout (`vcl_tui/src/`)

```
main.rs           # entry point: build Config/Runtime/App, ratatui::init/restore, run loop
app.rs            # App struct, Screen/Popup/SetupState/RunState enums, key dispatch, on_tick
config.rs         # Config struct, load/save at dirs::config_dir()/vcl_tui/config.toml
                  # (0600 perms on Unix, best-effort - no equivalent bit on Windows)
vcl/
  mod.rs          # VclUiError, thin helpers around vcl_lib::VclClient
  model.rs        # Image, RequestStatus, ConnectData, ConnectMethod, ActionResult
  convert.rs       # TryFrom<&Value> impls, coerce_i64 numeric-quirk helper, ConvertError
event.rs          # crossterm tick/key poll loop (AppEvent enum)
ui/
  mod.rs          # draw() dispatcher, shared chrome (top bar, tab strip, status/hint bar), centered_rect()
  setup.rs        # Setup screen
  images.rs       # Images screen + New Reservation popup
  reservations.rs # Reservations screen
  connect.rs      # Connect screen (SSH only)
connect_action.rs # touches raw terminal/process state: SSH suspend/resume, plus a small
                  # detached open_url() helper reused for the AVD guide handoff
```

Rationale: `vcl/convert.rs` is split from `vcl/model.rs` so the
Int-or-String/optional-field defensive parsing (the actual hard part, and
unit-testable in isolation with hand-built `Value` trees, no network
needed) doesn't clutter the plain data structs. `ui/*.rs` is one file per
screen because that's the natural unit the approved mockup already
defines - not over-fragmentation. `connect_action.rs` is isolated because
it's the only code that manipulates real terminal/process state instead of
drawing, and it's the one piece that's essentially unverifiable except by
hand.

## Core data model

### Enums (`app.rs`)

```rust
pub enum Screen { Setup, Images, Reservations, Connect }

/// Only one popup open at a time.
pub enum Popup { None, NewReservation { image_idx: usize } }

pub enum SetupState { Idle, Validating, Error(String) }

/// Checked at the top of the main loop; skips drawing/ticking entirely
/// while a child `ssh` process owns the real terminal.
pub enum RunState { Interactive, SuspendedForSsh }
```

### `App` struct (`app.rs`)

**Revised after starting implementation**: a fully flat `App` with every
screen's fields on it directly gets unwieldy fast (~16+ fields, most
meaningless outside their one screen). Split into global/shared fields
plus one small named sub-struct per screen, instead of either a fully flat
struct or a stricter `enum ScreenState` that only holds the current
screen's data - the stricter enum doesn't fit here because the Connect
screen needs simultaneous read access to the Reservations screen's list
*and* selection (they're not mutually exclusive), so `reservations` can't
be scoped inside a per-screen variant.

Global/shared, flat on `App`: `screen: Screen`, `popup: Popup`,
`run_state: RunState`, `exit: bool`, `client: Option<vcl_lib::VclClient>`
(`None` until Setup succeeds), `rt: tokio::runtime::Runtime` (see Event
Loop below - built via `Builder::new_current_thread().enable_all().build()`,
NOT `Runtime::new()`, which builds a multi-threaded runtime and would only
compile here by accident of `vcl_lib`'s own `tokio` feature flags being
unified in transitively - `vcl_tui`'s own `Cargo.toml` should be
self-sufficient), `config: Option<Config>`, `toast: Option<Toast>`.

Per-screen, one sub-struct per screen as its own field on `App`:
- `setup: SetupUiState { input: String, state: SetupState }`
- `images: ImagesUiState { images: Vec<Image>, list_state: ListState }`
- `reservations: ReservationsUiState { reservations: Vec<Reservation>, list_state: ListState, last_poll: Instant }`
  (shared by both the Reservations and Connect screens)
- `connect: ConnectUiState { cache: Option<(i64, ConnectData)>, my_ip: Option<String> }`

```rust
/// Local view-model row. get_request_status alone never carries an image
/// name (see RESPONSE_SHAPES.md), so this is populated at creation time
/// from the New Reservation flow; a reservation discovered fresh via
/// get_request_ids() on a new session has no known image name and should
/// render as "Reservation #<id>" instead of guessing.
pub struct Reservation {
    pub id: i64,
    pub image_name: Option<String>,
    pub status: RequestStatus,
}
```

`App` cannot derive `Default` (building the Runtime is fallible) - build it
via `App::new() -> anyhow::Result<Self>` in `main.rs`.

### Typed response structs (`vcl/model.rs`)

One struct per confirmed shape in `RESPONSE_SHAPES.md`:

```rust
pub struct Image { pub id: i64, pub name: String, pub ostype: String, pub usage: String, pub description: String }
impl Image {
    /// `false` for every observed Windows image in this account's catalog
    /// (all are AVD pool resources, see Context) - used to branch the
    /// Images screen's Details pane and `n` keybinding, NOT to filter the
    /// list. AVD images stay visible.
    pub fn is_reservable(&self) -> bool { !self.ostype.eq_ignore_ascii_case("windows") }
}

/// `status` stays a raw String (not an enum) - RESPONSE_SHAPES.md
/// explicitly warns not to assume an exhaustive vocabulary. `time` is
/// present in "loading" but absent in "ready" - keep it optional.
pub struct RequestStatus { pub status: String, pub time: Option<i64> }

/// The `ready` branch only. `notready` is represented as the call site
/// returning `Ok(None)`, not a struct variant - there's nothing else to carry.
pub struct ConnectData {
    pub server_ip: String,
    pub user: String,
    pub password: String,
    pub connect_port: i64,
    pub connect_methods: Vec<ConnectMethod>,
}
pub struct ConnectMethod { pub id: String, pub description: String, pub connecttext: String, pub connectports: Vec<String> }

/// Shared success/error envelope - add_request/extend_request/end_request
/// all confirmed to share this shape.
pub enum ActionResult {
    Success { requestid: Option<i64> },
    Error { errorcode: i64, errormsg: String },
}
```

`is_reservable()` is the filter the Images screen uses to drop AVD/Windows
entries (see Scoping above) - implemented as "not windows" rather than
matching on the AVD description text, since every observed Windows image
in this catalog is AVD; simpler and doesn't depend on parsing free text.

### `Value` -> typed struct conversion (`vcl/convert.rs`)

**Error-crate decision (revised twice, this is the final state): `vcl_tui`
uses `color-eyre`'s `eyre::Result`/`eyre::Context` exclusively, everywhere
in this crate - no `thiserror`, no `anyhow`.** History: `thiserror` was
ruled out first (its value is typed, matchable error variants for callers
outside the defining crate; nothing outside `vcl_tui` ever needs to
distinguish a `MissingField` from a `WrongType`, it only ever needs to
become a readable toast string or a top-level panic/error report).
`anyhow` was the next choice, on the same reasoning, then superseded by
`color-eyre` once that was adopted for panic/error *display* (see the
Developer-facing logging section below) - `eyre::Result` fills exactly the
role `anyhow::Result` was filling, so running both would just be the same
"two crates, one job" problem again. `vcl_lib` keeps `thiserror` for
`VclError` - that's a separate, actual library crate with external
consumers, unrelated to this decision and not being changed.

One numeric-coercion helper, `TryFrom<&Value>` per struct, `eyre::Result`
throughout. Note `eyre`'s context trait method is `.wrap_err(...)`, not
`anyhow`'s `.context(...)` - different name, same behavior:

```rust
use color_eyre::eyre::{Context, Result, bail};

/// Accepts Value::Int or a parseable Value::String - the server's
/// inconsistent numeric typing documented in RESPONSE_SHAPES.md.
pub fn coerce_i64(v: &Value, field: &'static str) -> Result<i64> {
    v.as_i64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
        .wrap_err_with(|| format!("field `{field}` has an unexpected type"))
}
```

`Image::try_from`, `RequestStatus::try_from`, `ConnectMethod::try_from`,
`ActionResult::try_from` each return `eyre::Result<Self>`, using
`coerce_i64` plus small `field`/`field_str` helpers (`v.get(key)
.wrap_err_with(|| format!("missing field `{key}`"))`) for required-field
lookup. `RequestStatus::try_from` reads `time` via
`.map(...).transpose()?` so its absence in the `ready` state is not an
error. `ConnectData` is only constructed after the caller checks `status
== "ready"`; its `connect_methods` map is the one place outside `Value`'s
own accessors that matches `Value::Struct(map)` directly (iterating the
id-keyed map, sorted by key for stable rendering), because `Value` has no
generic "iterate a struct's entries" accessor.

`get_images()` becomes:
`value.as_array().wrap_err("expected an array")?.iter().map(Image::try_from).collect::<Result<Vec<_>>>()`.

### Error wrapping (`vcl/mod.rs`)

No dedicated error enum - `vcl_lib::VclError` and the conversion errors
above both flow through as plain `eyre::Report` (via `?` and `eyre`'s
blanket `From<E: std::error::Error>` impl, which covers `VclError` for
free since it's a `thiserror` type with a real `std::error::Error` impl).
Call sites use `eyre::Result<T>` and turn the final error into a toast via
`.to_string()` (or `format!("{e:#}")` if you want the full `.wrap_err()`
chain shown, which is usually more useful for a toast than just the
top-level message).

Toasts remain the only *user-facing in-app* error surface for recoverable
errors - nothing here changes that. Unrecoverable errors (panics, or an
error propagated all the way out of `main()`) are a separate path, handled
by `color-eyre`'s hooks - see below.

### Developer-facing logging (file only, never stdout/stderr)

Addendum, added after initial planning: set this up early (before step 2 in
Build/verification order, i.e. before the first real `client.test()` call),
following ratatui's log-with-tracing recipe
(https://ratatui.rs/recipes/apps/log-with-tracing/) with one adjustment.
`vcl_lib::api` already emits useful `log::debug!` instrumentation (method
name, full XML-RPC request body, response byte count) that is otherwise
silently discarded with no subscriber installed - bridge it in with
`tracing_log::LogTracer::init()` rather than only capturing `vcl_tui`'s own
`tracing` calls. Two things this buys, both purely for the developer, not
the user:

- Visibility into the exact request/response XML while building the
  `TryFrom<&Value>` conversions - far faster to debug a shape mismatch by
  reading the real wire bytes than by guessing from a toast's one-line
  error string.
- Panic safety: install a panic hook that calls `ratatui::restore()` before
  logging/printing the panic payload, so a bug never leaves the terminal
  stuck in raw-mode/alt-screen with the actual error invisible underneath it.

New deps for this: `tracing`, `tracing-subscriber` (`fmt` feature is
enough, `env-filter` optional), `tracing-appender` (non-blocking file
writer), `tracing-log` (the `log` bridge). Log file location: reuse
`dirs::config_dir()` (already resolved once for `config.rs`, see below) and
put `vcl_tui.log` in the same `vcl_tui` subdirectory as `config.toml`,
rather than resolving a second, different platform directory for it.
`config_dir()` is guaranteed `Some` on Linux, macOS, and Windows alike
(unlike `dirs::state_dir()`, which only exists on Linux and returns `None`
everywhere else - don't use it here) - one directory-resolution codepath,
shared by both files, works identically on all three platforms. This is
additive to the "not added: log/tracing" line under New dependencies below
- that line was about `vcl_tui` using logging as an *error-reporting*
mechanism to replace toasts, which is still correctly rejected; this is a
separate, purely-diagnostic file sink.

## Event loop design

### Async integration

One `tokio::runtime::Runtime`, built **once** in `main.rs` via:

```rust
tokio::runtime::Builder::new_current_thread().enable_all().build()?
```

(`enable_all()`, not a cherry-picked driver list - simplest correct choice,
avoids having to reason precisely about which of reqwest's internal IO/time
driver needs are satisfied by which specific feature flags). `Cargo.toml`
needs `tokio = { version = "1", features = ["rt", "time"] }` for this
(`macros`/`rt-multi-thread` aren't needed - no `#[tokio::main]`, and only
one call is ever in flight at a time).

Every user-triggered API call is a direct, synchronous
`self.rt.block_on(future)` call inline in the key handler, e.g.:

```rust
fn on_key_reserve(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
    self.toast = Some(Toast::info("Reserving..."));
    terminal.draw(|f| self.draw(f))?;   // force the busy toast onto the real screen first
    let outcome = self.rt.block_on(client.add_request(image_id, "now", 30, None, false));
    self.apply_add_request_result(outcome);
    Ok(())
}
```

This is a deliberate simplification: draw the busy state, *then* block,
rather than building a channel/background-task architecture. Justified by
confirmed sub-second live latency for every call and the 30s hard timeout
already inside `vcl_lib::VclClient::call` - a background-task architecture
would be speculative complexity for this app's real usage pattern. The one
accepted tradeoff: if a call ever hangs for the full 30s, the UI is
unresponsive (no cancel) for that long - acceptable for a personal
reservation tool, worth a one-line code comment rather than a design change.

### Tick / background poll (`event.rs`)

```rust
pub enum AppEvent { Key(KeyEvent), Tick }
const TICK_RATE: Duration = Duration::from_millis(250);

pub fn next_event() -> io::Result<AppEvent> {
    if crossterm::event::poll(TICK_RATE)? {
        match crossterm::event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => Ok(AppEvent::Key(k)),
            _ => Ok(AppEvent::Tick),
        }
    } else { Ok(AppEvent::Tick) }
}
```

`App::on_tick()` re-polls reservation status in the background:

```rust
fn on_tick(&mut self) {
    if self.run_state == RunState::SuspendedForSsh { return; }
    if !matches!(self.screen, Screen::Reservations | Screen::Connect) { return; }
    let has_pending = self.reservations.iter().any(|r| r.status.status != "ready");
    if has_pending && self.last_res_poll.elapsed() >= Duration::from_secs(20) {
        self.refresh_reservation_statuses();
        self.last_res_poll = Instant::now();
    }
}
```

20s matches the production cadence `RESPONSE_SHAPES.md`/`vcl_probe` already
settled on (the VCL web UI's own poll interval; `vcl_probe`'s 35s was only
a dev-iteration slowdown). The 250ms `TICK_RATE` is just input-poll
granularity, decoupled from the 20s status cadence. No polling happens with
an empty/all-ready list, or off the two screens that show reservation state.

### SSH suspend/resume (`connect_action.rs`)

Switch `main.rs` off the closure-based `ratatui::run(...)` helper (which
owns init/restore internally and can't be interrupted mid-run) to the
explicit form:

```rust
fn main() -> anyhow::Result<()> {
    let mut app = App::new()?;
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}
```

```rust
pub fn connect_ssh(user: &str, port: i64, server_ip: &str) -> io::Result<ExitStatus> {
    ratatui::restore();                 // leave alt screen, disable raw mode
    let status = Command::new("ssh")
        .arg("-p").arg(port.to_string())
        .arg(format!("{user}@{server_ip}"))
        .status();                      // blocking, inherits stdio
    let _ = ratatui::init();            // re-enter alt screen + raw mode
    status
}
```

This relies on an `ssh` binary being on `PATH`, which is a reasonable
cross-platform assumption today - it's preinstalled on Linux and macOS, and
Windows 10+ ships OpenSSH client as an optional feature that's on by
default in current installs. If `Command::new("ssh")` fails to spawn (not
found), surface that as an error toast rather than a panic, same as any
other fallible external-process call in this app - don't special-case one
OS's failure mode over another's.

`ratatui::restore()`/`init()` are free functions operating on the terminal
mode via the shared `stdout` handle, not on any specific `Terminal` struct
instance - the caller's existing `terminal: &mut DefaultTerminal` value
stays valid across the suspend/resume and does **not** need to be replaced
with the second `init()` call's return value (discarding it is intentional,
not an oversight). What *is* required: call `terminal.clear()` immediately
after resuming, before the next `draw()` - ratatui's internal diff buffer
has no idea the physical screen was overwritten by the ssh session, and
without a forced clear the next frame may only paint a diff against stale
internal state, leaving garbage on screen.

Call site sets `run_state = RunState::SuspendedForSsh` before calling this
(so `on_tick` never fires mid-session) and back to `Interactive` after,
followed by `terminal.clear()?` and a toast
("vcl_tui resumed - ssh session ended").

`connect_action.rs` also holds a small, unrelated detached-handoff helper
for the AVD guide redirect (Scope decisions above) - no terminal
suspend/resume needed here, it's fire-and-forget:

```rust
const AVD_GUIDE_URL: &str = "https://vcl.ncsu.edu/accessing-environments-in-windows-virtual-desktop/";

pub fn open_url(url: &str) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    { Command::new("xdg-open").arg(url).spawn()?; }
    #[cfg(target_os = "macos")]
    { Command::new("open").arg(url).spawn()?; }
    #[cfg(target_os = "windows")]
    { Command::new("cmd").args(["/C", "start", ""]).arg(url).spawn()?; }
    Ok(())
}
```

All three branches are first-class, not a primary-plus-fallback - the app
is meant to run identically on Linux, macOS, and Windows, so each platform
gets its own native handoff command rather than assuming one OS and hoping
the others happen to work. Development so far has only exercised the
Linux (`xdg-open`) branch live; the macOS/Windows branches follow the same
`Command::spawn()` shape and should get a real manual check on those
platforms before considering this done, same as any other
`#[cfg(target_os = ...)]` split. A missing handoff binary on any platform
surfaces as an `Err` from `spawn()`, which the caller turns into an error
toast rather than a panic.

## Scope decisions

1. **RDP/Windows reservations: dropped entirely for v1** (see Context above
   - confirmed live, not a hypothesis); **AVD images stay visible in the
   list, branched instead of hidden.** The Images list shows every image
   unfiltered. When the selected image has `is_reservable() == false`
   (AVD/Windows), the Details pane replaces the normal Name/ID/OS/
   Description kv view with a short explanation - e.g. "This is an Azure
   Virtual Desktop (AVD) resource - not reservable through this tool. See
   NCSU's AVD guide." - and the `n`/Enter hint changes from "Reserve this
   image" to "Open AVD guide in browser". Pressing `n` on an AVD image does
   **not** call `add_request` at all; it calls the same detached
   `open_url()` handoff described below against a hardcoded constant
   (`https://vcl.ncsu.edu/accessing-environments-in-windows-virtual-desktop/`),
   shows a toast ("Opened AVD guide in browser"), and does nothing else -
   no popup, no reservation created. Connect screen still only implements
   SSH; a reservation can never end up needing an RDP connect step because
   one can never be created through this tool.
2. **New Reservation popup Length: hardcoded at 30, not editable.** Start is
   already fixed to `"now"` (the only confirmed-working value for this
   account type); making Length editable while Start stays fixed would be
   an asymmetric half-feature needing a new input widget + validation for
   low practical value, since `extend_request +15m` is already a first-class
   action for adjusting after the fact.
3. **Ended reservations: removed from the local list on `end_request`
   success**, not kept with an "ended" tag. The server is the source of
   truth and the next real `get_request_ids()` refresh won't include it
   either way - a persistent client-only "ended" row is state that a
   refresh will silently erase later. `self.reservations.retain(|r| r.id != ended_id)`,
   confirmed via the success toast instead.
4. **Setup screen has no cancel/escape once entered** (matches the approved
   mockup exactly - pressing `t` from any signed-in screen is a hard
   re-auth gate; only `Enter` with a valid token gets you back out). Not
   revisited here since it's a direct mockup-fidelity choice, not an
   oversight - flag to the user if this proves annoying in practice.
5. **Revised after Images/Reservations were both underway: Images is not a
   persistent tab.** The approved mockup gives Images/Reservations/Connect
   three equal, always-present tabs with Images as the post-sign-in landing
   screen - deliberately dropped. Instead: **Reservations is the landing
   screen and the only persistent "home"**; `Screen::Images` is entered
   only as a picker, reached via a "new reservation" key from Reservations,
   and exits back to Reservations either on cancel (no reservation made) or
   automatically after a successful `add_request` (not staying on Images).
   The Images screen's own code (list, details, AVD branch) is unchanged by
   this - only its entry point (a keypress from Reservations, not a
   standalone tab) and its exit (always returns to Reservations) differ
   from the original mockup. Connect is unaffected - still its own tab,
   reachable from a ready row in Reservations.

## Colors/styling

The mockup's own hex colors (in the original artifact) are illustrative
only - the real app uses NC State's official brand palette instead
(https://brand.ncsu.edu/designing-for-nc-state/color/), centralized in
`vcl_tui/src/theme.rs` (constants + small `Style`-returning helper
functions) and reused across every screen rather than hardcoded per file:

- accent (selection highlight, borders, active tab, hint-key labels) =
  **Bio-Indigo** `#4156A1`. **Revised after initial planning**: the first
  pass reused **Wolfpack Red** `#CC0000` for both accent and danger,
  reasoning NC State's palette had no separate color for the role - that
  premise was wrong, the brand's expanded secondary palette does include
  additional colors (Reynolds Red, Pyroman Flame, Carmichael Aqua,
  Bio-Indigo). Painting every border/selection/hint-key in full-saturation
  red read as constantly alarming in practice, so accent moved to
  Bio-Indigo and red is now reserved solely for actual errors, where its
  conventional "stop/danger" reading is earned.
- danger/error = **Wolfpack Red** `#CC0000`.
- success/ready = **Genomic Green** `#6F7D1C`.
- pending/loading = **Hunt Yellow** `#FAC800`.
- info/future/busy (e.g. "Validating...") = **Innovation Blue** `#427E93`.
- dim/secondary text = *not* a gray `Color` value - NC State's palette has
  no official neutral gray, so this uses the terminal's own dim/faint
  rendering attribute (`Modifier::DIM`) on the default foreground instead
  of picking an off-brand color.
- Selected list row (once list screens exist) = reverse-video style
  (accent background, dark text).

## New dependencies (`vcl_tui/Cargo.toml`)

```toml
[dependencies]
crossterm = "0.29.0"
ratatui = { version = "0.30.0", features = ["serde"] }
vcl_lib = { path = "../vcl_lib" }
tokio = { version = "1", features = ["rt", "time"] }
anyhow = "1"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "5"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt"] }
tracing-appender = "0.2"
```

No `thiserror` here - see the error-crate decision under `vcl/convert.rs`
above: `vcl_tui` uses `anyhow` exclusively throughout (`vcl_lib` keeps
`thiserror` for `VclError`, a separate crate, unrelated and unchanged). No
direct `tracing-log` dependency either - `tracing-subscriber`'s default
features already include it and wire up the `log`-to-`tracing` bridge
automatically during `.init()`; an explicit `tracing_log::LogTracer::init()`
call on top of that tries to install a second global logger and panics
(`SetLoggerError`) - this was actually hit and fixed during implementation,
don't reintroduce it. `tracing`/`tracing-subscriber`/`tracing-appender` are
for the developer-facing file log + panic hook only (see the addendum
above) - they do not become a user-facing error-reporting mechanism, toasts
still cover that. Not added: `tui-input` (hand-roll the one masked
single-line token field - backspace/insert only, no need for the extra
crate), any HTML-stripping crate (strip `<br>`/tags from
`description`/`connecttext` with a small manual scan - cosmetic line-wrap
concern, not worth a dependency), `clap` (no CLI args - the binary launches
straight into the TUI).

## Build/verification order

1. **Config + Setup screen, no network yet.** `Config::load()`/`save()`
   round-tripping against `dirs::config_dir()/vcl_tui/config.toml` (resolves
   to `~/.config/vcl_tui/` on Linux, `~/Library/Application Support/vcl_tui/`
   on macOS, `%APPDATA%\vcl_tui\` on Windows - don't hardcode any of these,
   always go through `dirs::config_dir()`). On Unix (Linux/macOS), verify
   the file was written with `0600` perms (`ls -l`); Windows has no
   equivalent permission bit, so the `#[cfg(unix)]`-gated chmod call is
   simply skipped there - nothing to verify on that platform. Get the Setup
   screen's masked input + busy-state rendering working before wiring in
   `client.test()`.
2. **Set up the file logger + panic hook (see "Developer-facing logging"
   above), then wire `client.test()` into Setup validation** using the same
   `.env`-sourced token/endpoint `vcl_probe` already uses - confirms the
   `block_on`-inside-event-loop mechanism end to end on the lowest-risk
   call, with the request/response now visible in the log file if anything
   looks wrong.
3. **Images screen with real `get_images()`** - build
   `vcl/convert.rs::Image::try_from` and the list+details layout against
   live data. First real exercise of the `TryFrom<&Value>` pattern before
   repeating it for the other shapes. Also verify the AVD branch here:
   select an AVD image (e.g. `MATLAB (AVD)`, id 5328), confirm the Details
   pane shows the explanation instead of normal fields, and that `n`
   actually opens the guide URL via the OS handoff (`xdg-open` on whichever
   machine you're developing on - re-check the `open_url()` branch for
   that platform, per the cross-platform note above) rather than attempting
   `add_request`.
4. **New Reservation popup + `add_request`** - mutates real state on the
   live server; confirm a 30-minute reservation is actually accepted for
   this account (per `vcl_probe`'s own note that the server may fault below
   some configured minimum).
5. **Reservations screen**: `get_request_ids()` + per-id
   `get_request_status()`, plus the 20s tick-driven background poll -
   manually verify a `loading` reservation flips to `ready` on screen with
   no keypress.
6. **Connect screen (data only)**: `get_ip()` + `get_request_connect_data()`,
   rendering Host/Your IP/Command preview, before touching process suspension.
7. **SSH connect (suspend/resume)** - the single riskiest/most novel
   mechanism in the app (terminal handoff to a child process). Test the
   exact `restore()` -> `Command::status()` -> `init()` -> `terminal.clear()`
   sequence against a real `ssh` session to a real `ready` reservation as
   soon as step 5/6 produce one to test against - don't defer this
   arbitrarily long, it's the step most likely to reveal a wrong assumption.
8. **Polish**: toast fade timing, `clippy -D warnings` + `rustfmt` clean,
   delete the leftover Counter App boilerplate entirely.

## Critical files

- `vcl_tui/Cargo.toml`, `vcl_tui/src/main.rs` (to be replaced per this plan)
- `vcl_tui/RESPONSE_SHAPES.md` (authoritative field-level spec)
- `vcl_lib/src/value.rs`, `vcl_lib/src/api.rs` (the `Value` enum and every
  method signature this TUI calls)
- `vcl_probe/src/main.rs` (precedent for the poll-loop pattern and the
  `.env`-based credential setup used for manual testing)

## Verification

No automated test suite is proposed for the UI rendering itself (a ratatui
`Buffer`-diffing unit test per screen would be legitimate but is more
scaffolding than this app's size warrants - CLAUDE.md's Simplicity First).
`vcl/convert.rs`'s `TryFrom<&Value>` impls **are** worth unit-testing
directly (`#[cfg(test)]`, hand-built `Value::Struct(...)` trees matching
`RESPONSE_SHAPES.md`'s documented shapes, including the Int-vs-String
numeric quirk and the missing-`time`-in-`ready` case) since they need no
network and are the one place a real bug (a wrong field name, a missed
optional field) would otherwise only surface live. End-to-end verification
is manual, using the real `.env` credentials already used by `vcl_probe`,
following the build order above - each step's "how to know it worked" is
already stated inline in that section (perms check, a real reservation
appearing/transitioning in the VCL web UI in parallel, a real `ssh` session
opening and the TUI cleanly resuming after).
