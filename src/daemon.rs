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

fn pid_path() -> PathBuf {
    db::data_dir().join("chirp.pid")
}

fn log_path() -> PathBuf {
    db::data_dir().join("chirp-daemon.log")
}

/// Check if the daemon process is alive by reading the PID file and signaling it.
pub fn is_running() -> bool {
    let path = pid_path();
    if let Ok(pid_str) = fs::read_to_string(&path) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // kill -0 checks if process exists without sending a signal
            return process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .stdout(process::Stdio::null())
                .stderr(process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }
    }
    false
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

/// Run the daemon foreground loop. Used by `chirp daemon` and by launchd/systemd.
pub fn run() {
    if is_running() {
        eprintln!("chirp: daemon is already running");
        process::exit(1);
    }

    write_pid();

    // Ensure PID file is cleaned up on normal exit
    // (For SIGKILL/crash, is_running() handles stale PIDs via kill -0)
    struct PidGuard;
    impl Drop for PidGuard {
        fn drop(&mut self) { remove_pid(); }
    }
    let _guard = PidGuard;

    eprintln!("chirp daemon started (PID {})", process::id());

    loop {
        check_pings();
        thread::sleep(Duration::from_secs(10));
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

            let _ = Notification::new()
                .summary("Chirp")
                .body(&body)
                .sound_name("Glass")
                .timeout(notify_rust::Timeout::Milliseconds(8000))
                .show();

            db.update_last_ping_at(&task.id, now);
        }
    }
}

/// Print whether the daemon is running.
pub fn status() {
    if is_running() {
        let pid = fs::read_to_string(pid_path()).unwrap_or_default();
        println!("chirp daemon is running (PID {})", pid.trim());
    } else {
        println!("chirp daemon is not running");
    }
}

/// Install a system service so the daemon auto-starts on login.
pub fn install() {
    let exe = std::env::current_exe()
        .expect("could not determine chirp binary path")
        .to_string_lossy()
        .to_string();

    if cfg!(target_os = "macos") {
        install_launchd(&exe);
    } else {
        install_systemd(&exe);
    }
}

/// Remove the system service.
pub fn uninstall() {
    if cfg!(target_os = "macos") {
        uninstall_launchd();
    } else {
        uninstall_systemd();
    }
}

// === macOS launchd ===

fn launchd_plist_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join("Library/LaunchAgents/com.chirp.daemon.plist")
}

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
        println!("Installed and started chirp daemon");
        println!("  plist: {}", path.display());
        println!("  log:   {}", log.display());
    } else {
        eprintln!("launchctl load failed (exit {})", status);
    }
}

fn uninstall_launchd() {
    let path = launchd_plist_path();
    if !path.exists() {
        println!("No launch agent found at {}", path.display());
        return;
    }

    let _ = process::Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(&path)
        .status();

    let _ = fs::remove_file(&path);
    remove_pid();
    println!("Uninstalled chirp daemon");
}

// === Linux systemd ===

fn systemd_service_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".config/systemd/user/chirp.service")
}

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
        .args(["--user", "daemon-reload"])
        .status();

    let enable = process::Command::new("systemctl")
        .args(["--user", "enable", "--now", "chirp"])
        .status();

    match (reload, enable) {
        (Ok(r), Ok(e)) if r.success() && e.success() => {
            println!("Installed and started chirp daemon");
            println!("  service: {}", path.display());
            println!("  logs:    journalctl --user -u chirp -f");
        }
        _ => eprintln!("systemctl commands failed — check systemd logs"),
    }
}

fn uninstall_systemd() {
    let path = systemd_service_path();
    if !path.exists() {
        println!("No systemd service found at {}", path.display());
        return;
    }

    let _ = process::Command::new("systemctl")
        .args(["--user", "disable", "--now", "chirp"])
        .status();

    let _ = fs::remove_file(&path);

    let _ = process::Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    remove_pid();
    println!("Uninstalled chirp daemon");
}
