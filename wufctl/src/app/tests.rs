use super::*;
use crate::state::Reservation;
use crate::vcl::RequestStatus;
use crossterm::event::KeyModifiers;

fn test_app() -> App {
    let rt = Builder::new_current_thread().enable_all().build().unwrap();
    App {
        screen: Screen::Reservations,
        popup: Popup::None,
        config: None,
        exit: false,
        async_runtime: Arc::new(rt),
        client: None,
        setup: SetupUiState::default(),
        images: ImagesUiState::default(),
        reservations: ReservationsUiState::default(),
        new_reservation: NewReservationFormState::default(),
        extend: ExtendFormState::default(),
        connect: ConnectUiState::default(),
        toast: None,
        pending: None,
        throbber_state: ThrobberState::default(),
    }
}

fn ready_reservation(id: i64) -> Reservation {
    Reservation {
        id,
        image_name: "Ubuntu".to_string(),
        status: RequestStatus {
            status: "ready".to_string(),
        },
        start: 0,
        end: 0,
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn reject_if_busy_toasts_and_returns_true_when_pending() {
    let mut app = test_app();
    app.pending = Some(PendingOp::LoadReservations(mpsc::channel().1));
    assert!(app.reject_if_busy());
    assert!(app.toast.is_some());
}

#[test]
fn reject_if_busy_is_false_when_idle() {
    let mut app = test_app();
    assert!(!app.reject_if_busy());
    assert!(app.toast.is_none());
}

/// Regression test for the bug fixed by `reject_if_busy`: pressing `r` while another
/// background op is already running used to silently drop that op's result.
#[test]
fn refresh_key_does_not_clobber_a_pending_op() {
    let mut app = test_app();
    app.pending = Some(PendingOp::EndReservation {
        id: 42,
        index: 0,
        rx: mpsc::channel().1,
    });
    app.handle_reservations_key(key(KeyCode::Char('r')))
        .unwrap();
    assert!(matches!(
        app.pending,
        Some(PendingOp::EndReservation { id: 42, .. })
    ));
}

#[test]
fn connect_key_does_not_clobber_a_pending_op() {
    let mut app = test_app();
    app.reservations.reservations.push(ready_reservation(1));
    app.reservations.list_state.select(Some(0));
    app.pending = Some(PendingOp::EndReservation {
        id: 42,
        index: 0,
        rx: mpsc::channel().1,
    });
    app.handle_reservations_key(key(KeyCode::Char('c')))
        .unwrap();
    assert!(matches!(
        app.pending,
        Some(PendingOp::EndReservation { id: 42, .. })
    ));
}

#[test]
fn confirm_end_enter_does_not_clobber_a_pending_op() {
    let mut app = test_app();
    app.popup = Popup::ConfirmEnd { index: 0 };
    app.pending = Some(PendingOp::LoadReservations(mpsc::channel().1));
    app.handle_confirm_end_key(key(KeyCode::Enter), 0).unwrap();
    assert!(matches!(app.pending, Some(PendingOp::LoadReservations(_))));
    assert_eq!(app.popup, Popup::ConfirmEnd { index: 0 });
}

#[test]
fn new_reservation_create_enter_does_not_clobber_a_pending_op() {
    let mut app = test_app();
    app.new_reservation.focus = FormRow::Create;
    app.pending = Some(PendingOp::LoadReservations(mpsc::channel().1));
    app.handle_new_reservation_key(key(KeyCode::Enter), 0)
        .unwrap();
    assert!(matches!(app.pending, Some(PendingOp::LoadReservations(_))));
}

#[test]
fn extend_confirm_enter_does_not_clobber_a_pending_op() {
    let mut app = test_app();
    app.extend.focus = crate::state::ExtendRow::Confirm;
    app.pending = Some(PendingOp::LoadReservations(mpsc::channel().1));
    app.handle_extend_key(key(KeyCode::Enter), 1).unwrap();
    assert!(matches!(app.pending, Some(PendingOp::LoadReservations(_))));
}

#[test]
fn describe_rejected_connection_blames_client_network_for_timeout() {
    let err = Report::new(VclError::Timeout);
    let msg = describe_rejected_connection(&err, "hint");
    assert!(msg.contains("timed out"));
    assert!(!msg.contains("Server rejected"));
}

#[test]
fn describe_rejected_connection_blames_client_network_for_connection_failure() {
    let err = Report::new(VclError::ConnectionFailed("refused".to_string()));
    let msg = describe_rejected_connection(&err, "hint");
    assert!(msg.contains("Couldn't reach"));
    assert!(!msg.contains("Server rejected"));
}

#[test]
fn describe_rejected_connection_blames_server_for_status_error() {
    let err = Report::new(VclError::NonSuccessStatus {
        status: 500,
        body: String::new(),
    });
    let msg = describe_rejected_connection(&err, "hint text");
    assert!(msg.contains("Server rejected the request"));
    assert!(msg.contains("hint text"));
}

#[test]
fn describe_rejected_connection_passes_through_other_errors() {
    let err = Report::new(VclError::MissingToken);
    let msg = describe_rejected_connection(&err, "hint");
    assert!(!msg.contains("Server rejected"));
    assert!(!msg.contains("timed out"));
}
