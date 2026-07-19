use crate::config::preset::Preset;
use futures::StreamExt;
use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use std::path::Path;

async fn read_limited_text(response: reqwest::Response, maximum: usize) -> Result<String, String> {
    if response
        .content_length()
        .is_some_and(|length| length > maximum as u64)
    {
        return Err(format!("response exceeds {maximum} bytes"));
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        if bytes.len() + chunk.len() > maximum {
            return Err(format!("response exceeds {maximum} bytes"));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// The remote URL serving the dynamic presets manifest.
/// Points to the raw presets.json in the lulushuz/Vane-Presets GitHub repository.
pub const REMOTE_PRESETS_URL: &str =
    "https://raw.githubusercontent.com/lulushuz/Vane-Presets/main/presets.json";

/// The signature URL (Minisign ed25519 signature of the presets.json file).
/// Will be used once a signing pipeline is set up in the presets repo.
pub const REMOTE_PRESETS_SIG_URL: &str =
    "https://raw.githubusercontent.com/lulushuz/Vane-Presets/main/presets.json.minisig";

/// Embedded Minisign public key for verifying the remote JSON.
pub const MANIFEST_PUBLIC_KEY: &str = "RWQo/mHXZdRyPdQNeH2YrFR8+knwokccmntw3cd24APtrqtxnbHGGaY7";

/// The manifest fetched from the remote endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePresetsManifest {
    /// Semantic version string. Used to diff against cached version.
    pub version: String,
    /// ISO date string for display purposes (e.g. "2026-04-18").
    pub updated_at: String,
    pub presets: Vec<Preset>,
}

pub type PresetManifest = RemotePresetsManifest;

#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    #[error("I/O Hatası: {0}")]
    Io(#[from] std::io::Error),
    #[error("Doğrulama Hatası: {0}")]
    Verification(String),
    #[error("JSON Serileştirme Hatası: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Genel Hata: {0}")]
    General(String),
}

/// Categorizes the outcome of a remote fetch attempt.
#[derive(Debug)]
pub enum RemoteFetchOutcome {
    /// New presets downloaded and cached. (manifest, signature, raw_json)
    Updated(RemotePresetsManifest, String, String),
    /// Remote version matches cache — no write needed.
    VersionUnchanged,
    /// Network is unreachable — caller should surface "offline" badge.
    Offline,
    /// Response received but JSON parsing failed.
    ParseError(String),
    /// Signature verification failed. Potential compromise or tampering.
    SignatureInvalid,
}

/// Fetches the remote presets manifest with a strict timeout.
///
/// Returns `RemoteFetchOutcome::Offline` for any network error,
/// so callers can always fall back to the local cache without panicking.
pub async fn fetch_remote_presets(
    client: &reqwest::Client,
    cached_version: Option<&str>,
) -> RemoteFetchOutcome {
    let response = match client
        .get(REMOTE_PRESETS_URL)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Remote presets fetch failed (offline?): {}", e);
            return RemoteFetchOutcome::Offline;
        }
    };

    let text = match read_limited_text(response, 1024 * 1024).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Failed to read remote presets response: {}", e);
            return RemoteFetchOutcome::Offline;
        }
    };

    let mut signature_data = String::new();

    // ─── CVE-5: Minisign ED25519 Verification ───
    // If MANIFEST_PUBLIC_KEY is empty, signature verification is disabled (setup pending).
    // Once a signing pipeline is configured for the presets repo, set the key to enable.
    if !MANIFEST_PUBLIC_KEY.is_empty() {
        let sig_response = match client
            .get(REMOTE_PRESETS_SIG_URL)
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Signature fetch failed (Network or file missing): {}", e);
                return RemoteFetchOutcome::SignatureInvalid;
            }
        };

        let sig_text = match read_limited_text(sig_response, 64 * 1024).await {
            Ok(t) => t,
            Err(_) => return RemoteFetchOutcome::SignatureInvalid,
        };

        let pub_key = match PublicKey::from_base64(MANIFEST_PUBLIC_KEY) {
            Ok(k) => k,
            Err(e) => {
                tracing::error!("Embedded public key is invalid: {}", e);
                return RemoteFetchOutcome::SignatureInvalid;
            }
        };

        let signature = match Signature::decode(&sig_text) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Signature file (sig) could not be parsed: {}", e);
                return RemoteFetchOutcome::SignatureInvalid;
            }
        };

        if let Err(e) = pub_key.verify(text.as_bytes(), &signature, false) {
            tracing::error!(
                "CRITICAL WARNING (CVE-5): Presets JSON signature verification failed! Tampering detected: {}",
                e
            );
            return RemoteFetchOutcome::SignatureInvalid;
        }
        tracing::debug!("Remote presets signature verification passed.");
        signature_data = sig_text;
    } else {
        tracing::warn!(
            "Remote presets signature verification is DISABLED (MANIFEST_PUBLIC_KEY not set)."
        );
    }
    // ────────────────────────────────────────────

    let manifest: RemotePresetsManifest = match serde_json::from_str(&text) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Remote presets JSON parse error: {}", e);
            return RemoteFetchOutcome::ParseError(e.to_string());
        }
    };

    // Skip disk write if the version hasn't changed — reduces I/O churn.
    if let Some(cached) = cached_version {
        if cached == manifest.version {
            tracing::debug!(
                "Remote presets are up to date (v{}), cache is valid.",
                manifest.version
            );
            return RemoteFetchOutcome::VersionUnchanged;
        }
    }

    tracing::info!(
        "Remote presets updated: v{} ({})",
        manifest.version,
        manifest.updated_at
    );
    RemoteFetchOutcome::Updated(manifest, signature_data, text)
}

/// Loads the cached preset manifest from disk.
///
/// Returns `None` if the cache file doesn't exist or is corrupted.
/// Corruption results in a warning log — not a hard failure.
pub fn load_cached_presets(app_data: &Path) -> Option<RemotePresetsManifest> {
    let manifest = if !MANIFEST_PUBLIC_KEY.is_empty() {
        if let Ok(pub_key) = PublicKey::from_base64(MANIFEST_PUBLIC_KEY) {
            load_cached_presets_verified(app_data, &pub_key).ok()
        } else {
            None
        }
    } else {
        let cache_path = app_data.join("remote_presets_cache.json");
        let content = std::fs::read_to_string(cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }?;
    crate::config::loader::validate_remote_presets(&manifest.presets).ok()?;
    Some(manifest)
}

/// Atomically writes the manifest and signature to the cache files.
///
/// Uses a temp file + rename pattern to prevent partial writes from
/// leaving the cache in a corrupted state on power loss or crash.
pub fn save_cached_presets(
    manifest: &RemotePresetsManifest,
    signature: &str,
    app_data: &Path,
) -> Result<(), String> {
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Serialization error: {}", e))?;

    // We block_on the async save_cached_presets_with_sig for compatibility
    tauri::async_runtime::block_on(async {
        save_cached_presets_with_sig(manifest, &content, signature, app_data).await
    })
    .map_err(|e| e.to_string())
}

pub async fn save_cached_presets_with_sig(
    _manifest: &PresetManifest,
    raw_json: &str,
    signature: &str,
    dir: &Path,
) -> Result<(), PresetError> {
    let cache_path = dir.join("remote_presets_cache.json");
    let sig_path = dir.join("remote_presets_cache.json.minisig");
    crate::settings::atomic_replace_bytes(&sig_path, signature.as_bytes())
        .map_err(PresetError::General)?;
    crate::settings::atomic_replace_bytes(&cache_path, raw_json.as_bytes())
        .map_err(PresetError::General)?;

    tracing::debug!("Remote presets cache and signature successfully written.");
    Ok(())
}

pub fn load_cached_presets_verified(
    dir: &Path,
    public_key: &minisign_verify::PublicKey,
) -> Result<PresetManifest, PresetError> {
    let cache_path = dir.join("remote_presets_cache.json");
    let sig_path = dir.join("remote_presets_cache.json.minisig");

    let cache_exists = cache_path.exists();
    let sig_exists = sig_path.exists();

    match (cache_exists, sig_exists) {
        (false, false) => Err(PresetError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Önbellek dosyası bulunamadı",
        ))),
        (true, false) => {
            // JSON exists but signature missing - clean it up (legacy or tamper)
            let _ = std::fs::remove_file(&cache_path);
            Err(PresetError::Verification(
                "Önbellek imza dosyası eksik, önbellek silindi".into(),
            ))
        }
        (false, true) => {
            // Signature exists but JSON missing - clean it up
            let _ = std::fs::remove_file(&sig_path);
            Err(PresetError::Verification(
                "Önbellek JSON dosyası eksik, imza silindi".into(),
            ))
        }
        (true, true) => {
            let content = std::fs::read_to_string(&cache_path)?;
            let sig_content = std::fs::read_to_string(&sig_path)?;

            let signature = Signature::decode(&sig_content).map_err(|e| {
                let _ = std::fs::remove_file(&cache_path);
                let _ = std::fs::remove_file(&sig_path);
                PresetError::Verification(format!(
                    "Önbellek imzası çözümlenemedi, dosyalar silindi: {}",
                    e
                ))
            })?;

            if let Err(e) = public_key.verify(content.as_bytes(), &signature, false) {
                let _ = std::fs::remove_file(&cache_path);
                let _ = std::fs::remove_file(&sig_path);
                return Err(PresetError::Verification(format!(
                    "CRITICAL WARNING (Cache Tampering Detected): Önbellek imza doğrulaması başarısız! Dosyalar silindi: {}",
                    e
                )));
            }

            let manifest = serde_json::from_str::<PresetManifest>(&content)?;
            tracing::info!(
                "Verified remote presets loaded from cache: v{}",
                manifest.version
            );
            Ok(manifest)
        }
    }
}
