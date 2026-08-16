use super::*;
use color_eyre::eyre::Report;

#[test]
fn fault_code_3_is_an_auth_error() {
    let err = Report::new(VclError::ApiError {
        code: 3,
        message: "Access denied".to_string(),
    });
    assert!(is_auth_error(&err));
}

#[test]
fn other_fault_codes_are_not_auth_errors() {
    let err = Report::new(VclError::ApiError {
        code: 2,
        message: "Reservation failed".to_string(),
    });
    assert!(!is_auth_error(&err));
}

#[test]
fn non_api_errors_are_not_auth_errors() {
    let err = Report::new(VclError::Timeout);
    assert!(!is_auth_error(&err));
}
