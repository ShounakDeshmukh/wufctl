use std::process::{Command, Stdio};

pub const AVD_GUIDE_URL: &str =
    "https://vcl.ncsu.edu/accessing-environments-in-windows-virtual-desktop/";

/// The launched app (e.g. a browser) inherits our stdio by default, and its
/// own stderr chatter (GTK theme warnings, etc.) would otherwise corrupt
/// the raw-mode terminal - so every branch redirects stdout/stderr to null.
pub fn open_url(url: &str) -> std::io::Result<()> {
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
