use std::fs;
use std::io;
use std::path::PathBuf;
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

/// RDP file template, from vcl webui . MIght break need to be updated if the webui changes its template.
pub fn save_rdp_file(image_name: &str, host: &str, port: &str, user: &str) -> io::Result<PathBuf> {
    let dir = dirs::download_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no Downloads folder found"))?;
    let path = dir.join(format!("{image_name}.rdp"));
    let contents = format!(
        "screen mode id:i:1\r\n\
         desktopwidth:i:1024\r\n\
         desktopheight:i:768\r\n\
         session bpp:i:24\r\n\
         winposstr:s:0,1,0,0,5000,4000\r\n\
         full address:s:{host}:{port}\r\n\
         compression:i:1\r\n\
         keyboardhook:i:2\r\n\
         audiomode:i:0\r\n\
         redirectdrives:i:1\r\n\
         redirectprinters:i:1\r\n\
         redirectcomports:i:0\r\n\
         redirectsmartcards:i:1\r\n\
         displayconnectionbar:i:1\r\n\
         autoreconnection enabled:i:1\r\n\
         username:s:{user}\r\n\
         clear password:s:\r\n\
         domain:s:\r\n\
         alternate shell:s:\r\n\
         shell working directory:s:\r\n\
         disable wallpaper:i:1\r\n\
         disable full window drag:i:1\r\n\
         disable menu anims:i:1\r\n\
         disable themes:i:0\r\n\
         disable cursor setting:i:0\r\n\
         bitmapcachepersistenable:i:1\r\n\
         dynamic resolution:i:1\r\n"
    );
    fs::write(&path, contents)?;
    Ok(path)
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
