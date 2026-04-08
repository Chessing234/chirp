use crate::db::{self, Database};
use crate::parser;
use chrono::Utc;
use notify_rust::Notification;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::thread;
use std::time::Duration;

pub(crate) fn pid_path() -> PathBuf {
    db::data_dir().join("chirp.pid")
}

#[cfg(target_os = "macos")]
fn log_path() -> PathBuf {
    db::data_dir().join("chirp-daemon.log")
}

/// Check if the daemon process is alive.
pub fn is_running() -> bool {
    let path = pid_path();
    let pid_str = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => return false,
    };

    #[cfg(unix)]
    {
        process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    {
        let out = process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();
        match out {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout);
                text.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
}

fn write_pid() {
    let path = pid_path();
    if let Ok(mut f) = fs::File::create(&path) {
        let _ = write!(f, "{}", process::id());
    }
}

fn remove_pid() {
    let _ = fs::remove_file(pid_path());
}

/// Run the daemon foreground loop.
pub fn run() {
    if is_running() {
        eprintln!("chirp: daemon is already running");
        process::exit(1);
    }

    write_pid();

    struct PidGuard;
    impl Drop for PidGuard {
        fn drop(&mut self) { remove_pid(); }
    }
    let _guard = PidGuard;

    eprintln!("chirp daemon started (PID {})", process::id());

    loop {
        check_pings();
        thread::sleep(Duration::from_secs(5));
    }
}

fn check_pings() {
    let db = match Database::new() {
        Ok(db) => db,
        Err(_) => return,
    };

    let now = Utc::now().timestamp_millis();
    let tasks = db.get_pingable_tasks();

    for task in &tasks {
        let interval_ms = task.ping_interval.unwrap_or(0) * 60 * 1000;
        if interval_ms <= 0 { continue; }

        let baseline = if let Some(last) = task.last_ping_at {
            last
        } else if let Some(due) = task.due_at {
            if now >= due { due } else { continue; }
        } else {
            db.update_last_ping_at(&task.id, now);
            continue;
        };

        if now - baseline >= interval_ms {
            let body = if let Some(due) = task.due_at {
                if parser::is_overdue(due) {
                    format!("{} (overdue)", task.content)
                } else {
                    format!("{} (due {})", task.content, parser::format_due_date(due))
                }
            } else {
                task.content.clone()
            };

            // Send persistent notification (with fallback on Linux)
            let notif_result = Notification::new()
                .summary("Chirp")
                .body(&body)
                .sound_name("Glass")
                .timeout(notify_rust::Timeout::Never)
                .show();

            // If notify-rust failed (e.g. no libdbus), fall back to notify-send
            if notif_result.is_err() {
                let _ = process::Command::new("notify-send")
                    .args(["--urgency=critical", "Chirp", &body])
                    .stdout(process::Stdio::null())
                    .stderr(process::Stdio::null())
                    .spawn();
            }

            // For p1 overdue tasks, open terminal in the user's face
            let is_urgent = task.priority == Some(1)
                && task.due_at.map(parser::is_overdue).unwrap_or(false);

            if is_urgent {
                open_terminal();
            }

            db.update_last_ping_at(&task.id, now);
        }
    }
}

/// Open a terminal window with chirp. Platform-specific.
fn open_terminal() {
    #[cfg(target_os = "macos")]
    {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "chirp".to_string());

        let term = std::env::var("TERM_PROGRAM").unwrap_or_default();

        // Terminal.app and iTerm2 support AppleScript `do script`
        let scriptable = matches!(term.as_str(), "" | "Apple_Terminal" | "iTerm.app");

        if scriptable {
            let app_name = if term == "iTerm.app" { "iTerm" } else { "Terminal" };
            let script = format!(
                r#"tell application "{app_name}"
                    activate
                    do script "{exe}"
                end tell"#,
            );
            let _ = process::Command::new("osascript")
                .args(["-e", &script])
                .stdout(process::Stdio::null())
                .stderr(process::Stdio::null())
                .spawn();
        } else {
            // Alacritty, Kitty, WezTerm, etc. — use `open -a` with args
            let _ = process::Command::new("open")
                .args(["-a", &term, "--args", "-e", &exe])
                .stdout(process::Stdio::null())
                .stderr(process::Stdio::null())
                .spawn();
        }
    }

    #[cfg(target_os = "linux")]
    {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "chirp".to_string());

        // Try common terminal emulators
        let terminals = [
            ("gnome-terminal", vec!["--".to_string(), exe.clone()]),
            ("konsole", vec!["-e".to_string(), exe.clone()]),
            ("xfce4-terminal", vec!["-e".to_string(), exe.clone()]),
            ("alacritty", vec!["-e".to_string(), exe.clone()]),
            ("kitty", vec![exe.clone()]),
            ("xterm", vec!["-e".to_string(), exe.clone()]),
        ];

        for (term, args) in &terminals {
            if process::Command::new(term)
                .args(args)
                .stdout(process::Stdio::null())
                .stderr(process::Stdio::null())
                .spawn()
                .is_ok()
            {
                break;
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "chirp.exe".to_string());

        let _ = process::Command::new("cmd")
            .args(["/c", "start", "Chirp", &exe])
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .spawn();
    }
}

/// Stop the daemon by killing the PID from the PID file.
pub fn stop() {
    let path = pid_path();
    let pid_str = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            println!("chirp daemon is not running (no PID file)");
            return;
        }
    };
    let pid = match pid_str.trim().parse::<u32>() {
        Ok(p) => p,
        Err(_) => {
            println!("invalid PID file, removing");
            remove_pid();
            return;
        }
    };

    #[cfg(unix)]
    {
        let killed = process::Command::new("kill")
            .arg(pid.to_string())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if killed {
            remove_pid();
            println!("chirp daemon stopped (PID {})", pid);
        } else {
            println!("process {} not found, cleaning up", pid);
            remove_pid();
        }
    }

    #[cfg(windows)]
    {
        let killed = process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if killed {
            remove_pid();
            println!("chirp daemon stopped (PID {})", pid);
        } else {
            println!("process {} not found, cleaning up", pid);
            remove_pid();
        }
    }
}

/// Restart the daemon (stop + run).
pub fn restart() {
    if is_running() {
        stop();
        // Brief pause for PID cleanup
        thread::sleep(Duration::from_millis(500));
    }
    run();
}

/// Print daemon status.
pub fn status() {
    if is_running() {
        let pid = fs::read_to_string(pid_path()).unwrap_or_default();
        println!("chirp daemon is running (PID {})", pid.trim());
    } else {
        println!("chirp daemon is not running");
    }
}

/// Auto-install the daemon if not already installed.
pub fn auto_install() {
    if is_installed() || is_running() { return; }

    let exe = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return,
    };

    #[cfg(target_os = "macos")]
    install_launchd(&exe);

    #[cfg(target_os = "linux")]
    install_systemd(&exe);

    #[cfg(target_os = "windows")]
    install_windows_task(&exe);
}

/// Check if daemon service is installed.
fn is_installed() -> bool {
    #[cfg(target_os = "macos")]
    return launchd_plist_path().exists();

    #[cfg(target_os = "linux")]
    return systemd_service_path().exists();

    #[cfg(target_os = "windows")]
    return process::Command::new("schtasks")
        .args(["/Query", "/TN", "ChirpDaemon"])
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return false;
}

/// Install service and print to stderr (called from TUI context).
pub fn install() {
    let exe = std::env::current_exe()
        .expect("could not determine chirp binary path")
        .to_string_lossy()
        .to_string();

    #[cfg(target_os = "macos")]
    install_launchd(&exe);

    #[cfg(target_os = "linux")]
    install_systemd(&exe);

    #[cfg(target_os = "windows")]
    install_windows_task(&exe);
}

/// Remove the system service.
pub fn uninstall() {
    #[cfg(target_os = "macos")]
    uninstall_launchd();

    #[cfg(target_os = "linux")]
    uninstall_systemd();

    #[cfg(target_os = "windows")]
    uninstall_windows_task();
}

// === macOS launchd ===

#[cfg(target_os = "macos")]
fn launchd_plist_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join("Library/LaunchAgents/com.chirp.daemon.plist")
}

#[cfg(target_os = "macos")]
fn install_launchd(exe: &str) {
    let log = log_path();
    let plist = format!(
r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.chirp.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
</dict>
</plist>"#,
        exe = exe,
        log = log.display(),
    );

    let path = launchd_plist_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, plist).expect("failed to write plist");

    let status = process::Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&path)
        .status()
        .expect("failed to run launchctl");

    if status.success() {
        println!("Installed chirp daemon (launchd)");
    } else {
        eprintln!("launchctl load failed (exit {})", status);
    }
}

#[cfg(target_os = "macos")]
fn uninstall_launchd() {
    let path = launchd_plist_path();
    if !path.exists() {
        println!("No launch agent found");
        return;
    }
    let _ = process::Command::new("launchctl").args(["unload", "-w"]).arg(&path).status();
    let _ = fs::remove_file(&path);
    remove_pid();
    println!("Uninstalled chirp daemon");
}

// === Linux systemd ===

#[cfg(target_os = "linux")]
fn systemd_service_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".config/systemd/user/chirp.service")
}

#[cfg(target_os = "linux")]
fn install_systemd(exe: &str) {
    let service = format!(
r#"[Unit]
Description=Chirp task reminder daemon

[Service]
ExecStart={exe} daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target"#,
        exe = exe,
    );

    let path = systemd_service_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, service).expect("failed to write service file");

    let reload = process::Command::new("systemctl")
        .args(["--user", "daemon-reload"]).status();
    let enable = process::Command::new("systemctl")
        .args(["--user", "enable", "--now", "chirp"]).status();

    match (reload, enable) {
        (Ok(r), Ok(e)) if r.success() && e.success() => {
            println!("Installed chirp daemon (systemd)");
        }
        _ => eprintln!("systemctl commands failed"),
    }
}

#[cfg(target_os = "linux")]
fn uninstall_systemd() {
    let path = systemd_service_path();
    if !path.exists() {
        println!("No systemd service found");
        return;
    }
    let _ = process::Command::new("systemctl").args(["--user", "disable", "--now", "chirp"]).status();
    let _ = fs::remove_file(&path);
    let _ = process::Command::new("systemctl").args(["--user", "daemon-reload"]).status();
    remove_pid();
    println!("Uninstalled chirp daemon");
}

// === Windows Task Scheduler ===

#[cfg(target_os = "windows")]
fn install_windows_task(exe: &str) {
    let status = process::Command::new("schtasks")
        .args([
            "/Create", "/SC", "ONLOGON",
            "/TN", "ChirpDaemon",
            "/TR", &format!("\"{}\" daemon", exe),
            "/RL", "LIMITED",
            "/F",
        ])
        .status()
        .expect("failed to run schtasks");

    if status.success() {
        println!("Installed chirp daemon (Task Scheduler)");
        // Also start it now
        let _ = process::Command::new("schtasks")
            .args(["/Run", "/TN", "ChirpDaemon"])
            .status();
    } else {
        eprintln!("schtasks /Create failed");
    }
}

#[cfg(target_os = "windows")]
fn uninstall_windows_task() {
    let status = process::Command::new("schtasks")
        .args(["/Delete", "/TN", "ChirpDaemon", "/F"])
        .status();

    match status {
        Ok(s) if s.success() => {
            remove_pid();
            println!("Uninstalled chirp daemon");
        }
        _ => println!("No scheduled task found"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_file_stale_detection() {
        let path = pid_path();
        // Ensure data dir exists (may not on CI)
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let backup = fs::read_to_string(&path).ok();

        // Write a definitely-dead PID (must stay positive when interpreted
        // as signed — 4294967295 wraps to -1, and `kill -0 -1` signals ALL
        // processes on Linux, which succeeds)
        fs::write(&path, "99999999").unwrap();
        assert!(!is_running(), "stale PID should not be detected as running");

        // Write our own PID
        fs::write(&path, format!("{}", process::id())).unwrap();
        assert!(is_running(), "our own PID should be detected as running");

        // Missing file
        let _ = fs::remove_file(&path);
        assert!(!is_running(), "missing PID file should not be detected as running");

        if let Some(content) = backup {
            fs::write(&path, content).ok();
        }
    }

    #[test]
    fn test_pid_path_in_data_dir() {
        let path = pid_path();
        assert!(path.ends_with("chirp.pid"));
        assert!(path.to_string_lossy().contains("Chirp"));
    }
}
