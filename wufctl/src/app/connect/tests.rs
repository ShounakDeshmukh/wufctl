use super::*;
use crate::vcl::model::ConnectMethod;

fn method(description: &str, connectports: &[&str]) -> ConnectMethod {
    ConnectMethod {
        id: "1".to_string(),
        description: description.to_string(),
        connectports: connectports.iter().map(|p| p.to_string()).collect(),
    }
}

fn connect_data(methods: Vec<ConnectMethod>) -> ConnectData {
    ConnectData {
        server_ip: "1.2.3.4".to_string(),
        user: "user".to_string(),
        password: "pw".to_string(),
        connect_port: "22".to_string(),
        connect_methods: methods,
    }
}

#[test]
fn rdp_port_extracts_remote_port_case_insensitively() {
    let data = connect_data(vec![
        method("SSH (Secure Shell) on Port 22", &["TCP:22:22"]),
        method("Remote Desktop (RDP)", &["TCP:3389:3389"]),
    ]);
    assert_eq!(rdp_port(&data), Some("3389"));
}

#[test]
fn rdp_port_is_none_without_an_rdp_method() {
    let data = connect_data(vec![method(
        "SSH (Secure Shell) on Port 22",
        &["TCP:22:22"],
    )]);
    assert_eq!(rdp_port(&data), None);
}

#[test]
fn rdp_port_is_none_with_no_methods() {
    let data = connect_data(vec![]);
    assert_eq!(rdp_port(&data), None);
}
