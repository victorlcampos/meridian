//! Opening web addresses and System Settings pages outside the terminal.

use std::process::{Command, Stdio};

/// Opens `target` with the desktop's handler: the browser for web addresses,
/// System Settings for its `x-apple.systempreferences:` pages. Returns at
/// once, with whether the handler started.
pub fn open(target: &str) -> bool {
    let mut command = if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(windows) {
        let mut command = Command::new("rundll32");
        command.arg("url.dll,FileProtocolHandler");
        command
    } else {
        Command::new("xdg-open")
    };
    let started = command
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = started else {
        return false;
    };
    // Without a desktop, xdg-open stays until the browser it started quits:
    // waiting aside keeps the clock going.
    std::thread::spawn(move || child.wait());
    true
}
