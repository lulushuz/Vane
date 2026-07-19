use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::preset::{Preset, builtin_presets};
use crate::privilege::checker::is_elevated;
#[cfg(target_os = "windows")]
use crate::engine::job::JobObjectGuard;
use std::process::Stdio;
use tokio::process::Child as AsyncChild;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizePayload {
    pub step: String,
    pub preset_name: String,
    pub progress_pct: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum OptimizeError {
    #[error("Yeterli yetki yok")]
    InsufficientPrivileges,
    #[error("Çalışan profil bulunamadı")]
    NoWorkingPresetFound,
    #[error("Motor başlatılamadı: {0}")]
    EngineError(String),
    #[error("Test hatası: {0}")]
    TestError(String),
}

/* 
   RAII guard: kill test process under all conditions.
   Optimizer runs a test loop; if future is cancelled or
   panics at any await point, this struct terminates
   the underlying winws instance upon Drop. 
*/
struct ChildGuard {
    child: Option<AsyncChild>,
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    job_guard: Option<JobObjectGuard>,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            tracing::debug!("Optimizer RAII: test süreci sonlandırılıyor.");
            let _ = child.start_kill();
        }
    }
}

pub struct Optimizer {
    app: AppHandle,
    // Target sites to test. Accessibility is measured for each preset.
    targets: Vec<String>,
}

impl Optimizer {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            targets: vec![
                "https://www.youtube.com".into(),
                "https://discord.com".into(),
                "https://x.com".into(),
            ],
        }
    }

    pub async fn run_heuristic_scan(&self) -> Result<Preset, OptimizeError> {
        if !is_elevated() {
            return Err(OptimizeError::InsufficientPrivileges);
        }

        let mut presets = builtin_presets();
        // Sort by priority (Low number = Tried first)
        presets.sort_by_key(|p| p.priority);

        let total = presets.len();
        let mut best_preset: Option<(Preset, u32)> = None;

        for (idx, preset) in presets.into_iter().enumerate() {
            let progress = ((idx as f32 / total as f32) * 100.0) as u8;

            let _ = self.app.emit(
                "optimize_progress",
                OptimizePayload {
                    step: format!("Test Yapılıyor ({}/{})", idx + 1, total),
                    preset_name: preset.label.clone(),
                    progress_pct: progress,
                },
            );

            tracing::debug!("Preset test ediliyor: {} {:?}", preset.label, preset.args);

            // Run from Resource folder due to Native sidecar issues
            #[cfg(target_os = "windows")]
            let binary_name = "binaries/winws-x86_64-pc-windows-msvc.exe";
            #[cfg(target_os = "linux")]
            let binary_name = "binaries/nfqws-x86_64-unknown-linux-gnu";

            let bin_path_result = self.app.path().resolve(binary_name, tauri::path::BaseDirectory::Resource);
            
            let spawn_result = match bin_path_result {
                Ok(path) => {
                    let working_dir = path.parent()
                        .ok_or_else(|| OptimizeError::EngineError(
                            format!("Binary path'in parent klasörü alınamadı: {:?}", path)
                        ))?;
                    let mut cmd = tokio::process::Command::new(&path);
                    cmd.args(&preset.args)
                        .current_dir(working_dir)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null());

                    #[cfg(target_os = "windows")]
                    cmd.creation_flags(0x08000000);
                        
                    cmd.spawn()
                        .map_err(|e| OptimizeError::EngineError(e.to_string()))
                },
                Err(e) => Err(OptimizeError::EngineError(e.to_string())),
            };

            let child = match spawn_result {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Preset başlatılamadı ({}): {:?}", preset.label, e);
                    continue;
                }
            };

            let pid = child.id().unwrap_or(0);
            
            // RAII guard: child is killed and job handle closed upon exiting scope.
            let guard = ChildGuard {
                child: Some(child),
                #[cfg(target_os = "windows")]
                job_guard: {
                    /* 
                       Assign child to Job Object for kernel-level cleanup guarantee.
                       If Vane crashes during optimization, winws is killed by Windows. 
                    */
                    JobObjectGuard::new()
                        .and_then(|j| j.assign(pid).map(|_| j))
                        .ok()
                },
            };
            #[cfg(not(target_os = "windows"))]
            let _ = pid;

            // Wait time for WinDivert kernel filter injection
            tokio::time::sleep(Duration::from_millis(3000)).await;

            /* 
               Create a FRESH client with zero history for each preset.
               BUG PREVENTION: Reusing client pool with failed history
               caches bad connections, causing false-negatives in later preset tests. 
            */
            let fresh_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .tcp_keepalive(None)
                .pool_max_idle_per_host(0)
                // Use known IPs to bypass ISP DNS poisoning.
                .resolve("discord.com", "162.159.135.232:443".parse().unwrap())
                .resolve("www.discord.com", "162.159.135.232:443".parse().unwrap())
                .resolve("youtube.com", "142.250.185.14:443".parse().unwrap())
                .resolve("www.youtube.com", "142.250.185.14:443".parse().unwrap())
                .resolve("x.com", "104.244.42.65:443".parse().unwrap())
                .user_agent(
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                    AppleWebKit/537.36 (KHTML, like Gecko) \
                    Chrome/120.0.0.0 Safari/537.36",
                )
                .build()
                .map_err(|e| OptimizeError::TestError(e.to_string()))?;

            let score = self.test_targets(&fresh_client).await;

            /* 
               Wait for OS to release WinDivert handles,
               then RAII guard kills child (via drop). 
            */
            tokio::time::sleep(Duration::from_millis(800)).await;
            drop(guard); // Explicit drop for clarity

            /* 
               Guard dropped = child already killed
               (no access after drop, just for readability) 
            */

            if let Some(s) = score {
                tracing::info!("Preset '{}' skoru: {}", preset.label, s);

                let is_better = best_preset.as_ref().is_none_or(|(_, best_s)| s > *best_s);
                if is_better {
                    best_preset = Some((preset, s));
                }

                /* 
                   Early exit if all 3 targets succeed and latency is below 3 seconds.
                   Score = (3 * 10000) - latency_ms → 30000 - 3000 = 27000 
                */
                if s > 27000 {
                    tracing::info!("Mükemmel preset bulundu, erken çıkılıyor!");
                    break;
                }
            } else {
                tracing::warn!("Preset '{}' testte başarısız oldu.", preset.label);
            }
        }

        let _ = self.app.emit(
            "optimize_progress",
            OptimizePayload {
                step: "Tamamlandı".into(),
                preset_name: "Bitti".into(),
                progress_pct: 100,
            },
        );

        if let Some((preset, _)) = best_preset {
            let _ = Self::save_state(&self.app, &preset);
            Ok(preset)
        } else {
            Err(OptimizeError::NoWorkingPresetFound)
        }
    }

    pub fn save_state(app: &AppHandle, preset: &Preset) -> std::io::Result<()> {
        if let Ok(app_data) = app.path().app_data_dir() {
            let file_path = app_data.join("optimizer_state.json");
            let data: String = serde_json::to_string(preset)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            std::fs::write(file_path, data)?;
        }
        Ok(())
    }

    pub fn load_state(app: &AppHandle) -> Option<Preset> {
        if let Ok(app_data) = app.path().app_data_dir() {
            let file_path = app_data.join("optimizer_state.json");
            if let Ok(data) = std::fs::read_to_string(&file_path) {
                if let Ok(preset) = serde_json::from_str::<Preset>(&data) {
                    return Some(preset);
                }
            }
        }
        None
    }

    /* 
       Gets a fresh client for each preset test. This prevents previous failed
       connection attempts from caching in the pool and causing false-negatives. 
    */
    async fn test_targets(&self, client: &reqwest::Client) -> Option<u32> {
        let mut tasks = Vec::new();

        for url in &self.targets {
            let client_clone = client.clone();
            let url_clone = url.clone();

            tasks.push(tokio::spawn(async move {
                let start = Instant::now();
                // HEAD downloads much less data than GET; sufficient for DPI testing.
                let res = client_clone.head(&url_clone).send().await;
                let elapsed = start.elapsed().as_millis() as u32;
                let ok = res
                    .map(|r| r.status().is_success() || r.status().as_u16() < 400)
                    .unwrap_or(false);

                (ok, elapsed)
            }));
        }

        let results = futures::future::join_all(tasks).await;

        let mut success_count: u32 = 0;
        let mut total_latency: u32 = 0;

        for (success, latency) in results.into_iter().flatten() {
            if success {
                success_count += 1;
                total_latency += latency;
            }
        }

        let avg_latency = total_latency.checked_div(success_count)?;
        let score = (success_count * 10000).saturating_sub(avg_latency);
        Some(score)
    }
}
