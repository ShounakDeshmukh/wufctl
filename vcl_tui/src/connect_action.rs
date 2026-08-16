use std::io;
use std::process::{Command, ExitStatus, Stdio};

use crossterm::cursor::Show;
use crossterm::execute;

pub const AVD_GUIDE_URL: &str =
    "https://vcl.ncsu.edu/accessing-environments-in-windows-virtual-desktop/";

/// Inherits stdio so password/key prompts work; Windows' OpenSSH Client needs no branching here.
pub fn connect_ssh(user: &str, server_ip: &str, port: &str) -> io::Result<ExitStatus> {
    ratatui::restore();
    // restore() doesn't re-show the cursor ratatui hides, so the ssh session would inherit it hidden.
    let _ = execute!(io::stdout(), Show);
    let status = Command::new("ssh")
        .arg("-p")
        .arg(port)
        .arg(format!("{user}@{server_ip}"))
        .status();
    ratatui::init();
    status
}

/// Translates a `connect_ssh` spawn failure into OS specific hints.
pub fn describe_ssh_error(err: &io::Error) -> String {
    if err.kind() != io::ErrorKind::NotFound {
        return format!("Couldn't start ssh: {err}");
    }
    #[cfg(target_os = "windows")]
    let hint = "ssh not found - install the OpenSSH Client optional feature \
                (Settings > Apps > Optional features), or run \
                `Add-WindowsCapability -Online -Name OpenSSH.Client~~~~0.0.1.0` in PowerShell.";
    #[cfg(target_os = "linux")]
    let hint = "ssh not found - install it via your package manager, e.g. `sudo apt install openssh-client`.";
    #[cfg(target_os = "macos")]
    let hint = "ssh not found, which is unusual since macOS ships it by default - check your PATH.";
    hint.to_string()
}

/// Opens a URL in the default browser, cross-platform. Does not wait for the browser to exit.
pub fn open_url(url: &str) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }
    Ok(())
}
