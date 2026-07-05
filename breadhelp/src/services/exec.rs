use std::process::Command;

/// Fire-and-forget: runs `command` via a shell, matching the idiom already
/// used throughout bos-settings' views (`let _ = Command::new(...).spawn()`).
pub fn run(command: &str) {
    if let Err(e) = Command::new("sh").arg("-c").arg(command).spawn() {
        eprintln!("breadhelp: failed to run `{command}`: {e}");
    }
}

/// Runs `command` on a background thread and delivers success/failure back
/// onto the GTK main loop via `on_done` — plain `spawn()` only confirms the
/// process *launched*, not how it exited, and one-click fixes need to report
/// that back to the user (a toast), so this blocks a throwaway thread on
/// `Command::status()` instead of the UI thread.
pub fn run_reporting(command: &str, on_done: impl Fn(bool) + 'static) {
    let command = command.to_string();
    let (tx, rx) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let status = Command::new("sh").arg("-c").arg(&command).status();
        let _ = tx.send_blocking(status.map(|s| s.success()).unwrap_or(false));
    });
    glib::MainContext::default().spawn_local(async move {
        if let Ok(success) = rx.recv().await {
            on_done(success);
        }
    });
}
