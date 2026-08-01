use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[allow(dead_code)]
const TASK_NAME: &str = "Vane";

#[cfg(target_os = "linux")]
const SYSTEMD_SERVICE_NAME: &str = "vane";

// ─── Windows ────────────────────────────────────────────────────────────────

/// Registers Vane in Windows Task Scheduler to run at logon with highest privileges.
/// Using Task Scheduler instead of `HKCU\Run` registry key is deliberate:
/// - `HKCU\Run` cannot request elevated privileges — UAC would prompt on every boot.
/// - Task Scheduler with `/rl highest` silently elevates on logon (no UAC prompt).
#[cfg(target_os = "windows")]
pub fn enable_autostart(exe_path: &str) -> Result<(), String> {
    if exe_path.contains('\n') || exe_path.contains('\r') || exe_path.contains('\0') {
        return Err("Geçersiz exe_path karakterleri tespit edildi".to_string());
    }
    // Escape any double quotes in path by doubling them
    let escaped_path = exe_path.replace('"', "\"\"");
    let task_run = format!("\"{}\" --autostart", escaped_path);
    let mut cmd = Command::new("schtasks");
    cmd.args([
        "/create", "/tn", TASK_NAME, "/tr", &task_run, "/sc", "onlogon", "/rl", "highest", "/f",
    ])
    .creation_flags(CREATE_NO_WINDOW);

    let status = cmd
        .status()
        .map_err(|e| format!("schtasks çalıştırılamadı: {}", e))?;

    if status.success() {
        tracing::info!("Auto-start görevi oluşturuldu: \"{}\"", TASK_NAME);
        Ok(())
    } else {
        Err(format!(
            "schtasks /create başarısız (exit: {:?}). Yönetici yetkisi gereklidir.",
            status.code()
        ))
    }
}

#[cfg(target_os = "windows")]
pub fn disable_autostart() -> Result<(), String> {
    let mut cmd = Command::new("schtasks");
    cmd.args(["/delete", "/tn", TASK_NAME, "/f"])
        .creation_flags(CREATE_NO_WINDOW);

    let status = cmd
        .status()
        .map_err(|e| format!("schtasks çalıştırılamadı: {}", e))?;

    // Exit code 1 means the task was not found — treat as success (idempotent).
    if status.success() || status.code() == Some(1) {
        tracing::info!("Auto-start görevi silindi: \"{}\"", TASK_NAME);
        Ok(())
    } else {
        Err(format!(
            "schtasks /delete başarısız (exit: {:?})",
            status.code()
        ))
    }
}

#[cfg(target_os = "windows")]
pub fn is_autostart_enabled() -> bool {
    let mut cmd = Command::new("schtasks");
    cmd.args(["/query", "/tn", TASK_NAME])
        .creation_flags(CREATE_NO_WINDOW);
    match cmd.output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

// ─── Linux (systemd user service) ───────────────────────────────────────────

/// Writes a systemd user service unit and enables it.
/// Service is installed at: `~/.config/systemd/user/vane.service`
/// No root required — systemd --user runs in the user's own session.
#[cfg(target_os = "linux")]
pub fn enable_autostart(exe_path: &str) -> Result<(), String> {
    if exe_path.contains('\n') || exe_path.contains('\r') || exe_path.contains('\0') {
        return Err("Geçersiz exe_path karakterleri tespit edildi".to_string());
    }

    let escaped_path = exe_path.replace('\\', "\\\\").replace('"', "\\\"");

    let service_dir = systemd_user_dir()?;
    std::fs::create_dir_all(&service_dir)
        .map_err(|e| format!("Systemd servis dizini oluşturulamadı: {}", e))?;

    let service_content = format!(
        "[Unit]\n\
        Description=Vane DPI Bypass Engine\n\
        After=network.target\n\n\
        [Service]\n\
        ExecStart=\"{escaped_path}\" --autostart\n\
        Restart=no\n\
        Type=simple\n\n\
        [Install]\n\
        WantedBy=default.target\n"
    );

    let service_path = service_dir.join(format!("{}.service", SYSTEMD_SERVICE_NAME));
    crate::settings::atomic_replace_bytes(&service_path, service_content.as_bytes())
        .map_err(|e| format!("Servis dosyası atomik olarak yazılamadı: {}", e))?;

    systemctl_user(&["daemon-reload"])
        .map_err(|e| tracing::warn!("daemon-reload başarısız: {}", e))
        .ok();
    systemctl_user(&["enable", SYSTEMD_SERVICE_NAME])?;

    tracing::info!(
        "Linux systemd auto-start aktif edildi: {}.service",
        SYSTEMD_SERVICE_NAME
    );
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn disable_autostart() -> Result<(), String> {
    let _ = systemctl_user(&["disable", SYSTEMD_SERVICE_NAME]);

    let service_dir = systemd_user_dir()?;
    let service_path = service_dir.join(format!("{}.service", SYSTEMD_SERVICE_NAME));
    if service_path.exists() {
        std::fs::remove_file(&service_path)
            .map_err(|e| format!("Servis dosyası silinemedi: {}", e))?;
    }

    let _ = systemctl_user(&["daemon-reload"]);
    tracing::info!("Linux systemd auto-start devre dışı bırakıldı.");
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn is_autostart_enabled() -> bool {
    match Command::new("systemctl")
        .args(["--user", "is-enabled", SYSTEMD_SERVICE_NAME])
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim() == "enabled",
        Err(_) => false,
    }
}

#[cfg(target_os = "linux")]
fn systemd_user_dir() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME ortam değişkeni bulunamadı.".to_string())?;
    Ok(std::path::PathBuf::from(home)
        .join(".config")
        .join("systemd")
        .join("user"))
}

#[cfg(target_os = "linux")]
fn systemctl_user(args: &[&str]) -> Result<(), String> {
    let mut all_args = vec!["--user"];
    all_args.extend_from_slice(args);
    let status = Command::new("systemctl")
        .args(&all_args)
        .status()
        .map_err(|e| format!("systemctl çalıştırılamadı: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "systemctl {:?} başarısız (exit: {:?})",
            args,
            status.code()
        ))
    }
}
