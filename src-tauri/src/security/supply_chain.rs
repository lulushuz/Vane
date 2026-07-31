use crate::security::artifact_integrity::ArtifactIntegrityError;
use crate::security::artifact_manifest::ContentArtifactManifest;
use crate::security::artifact_path::validate_and_resolve_resource_path;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct VerifiedContentArtifact {
    pub id: String,
    pub relative_path: String,
    pub size: u64,
    pub sha256: String,
}

#[allow(dead_code)]
pub(crate) fn verify_content_artifacts(
    resource_root: &Path,
) -> Result<Vec<VerifiedContentArtifact>, ArtifactIntegrityError> {
    let manifest = ContentArtifactManifest::load_embedded()
        .map_err(ArtifactIntegrityError::ManifestParseFailed)?;

    let mut results = Vec::new();

    for entry in &manifest.artifacts {
        let rel_path = entry.relative_path.to_path_buf();
        let canonical_path = validate_and_resolve_resource_path(resource_root, &rel_path)?;

        let mut file = File::open(&canonical_path)
            .map_err(|e| ArtifactIntegrityError::Io(canonical_path.clone(), e))?;

        let meta = file
            .metadata()
            .map_err(|e| ArtifactIntegrityError::Io(canonical_path.clone(), e))?;

        if meta.len() != entry.size {
            return Err(ArtifactIntegrityError::ArtifactSizeMismatch {
                id: entry.id.0.clone(),
                expected: entry.size,
                actual: meta.len(),
            });
        }

        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 16384];
        let mut read_bytes: u64 = 0;

        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|e| ArtifactIntegrityError::Io(canonical_path.clone(), e))?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
            read_bytes += count as u64;
        }

        let computed_hash: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        if computed_hash != entry.sha256.0.to_lowercase() {
            return Err(ArtifactIntegrityError::ArtifactHashMismatch {
                id: entry.id.0.clone(),
                expected: entry.sha256.0.to_lowercase(),
                actual: computed_hash,
            });
        }

        results.push(VerifiedContentArtifact {
            id: entry.id.0.clone(),
            relative_path: entry.relative_path.0.clone(),
            size: read_bytes,
            sha256: computed_hash,
        });
    }

    Ok(results)
}

/// Public key role taxonomy definition & verification
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicKeyRole {
    TauriUpdaterRelease,
    RemotePresetSignature,
    SecurityDisclosureIdentity,
}

#[allow(dead_code)]
pub(crate) struct PublicKeyRoleMapping {
    pub role: PublicKeyRole,
    pub identifier: &'static str,
    pub source_path: &'static str,
    pub expected_usage: &'static str,
}

#[allow(dead_code)]
pub(crate) fn get_public_key_role_mappings() -> Vec<PublicKeyRoleMapping> {
    vec![
        PublicKeyRoleMapping {
            role: PublicKeyRole::TauriUpdaterRelease,
            identifier: "tauri-updater-pubkey",
            source_path: "tauri.conf.json -> plugins.updater.pubkey",
            expected_usage: "Verifies signed application update archives during release updates.",
        },
        PublicKeyRoleMapping {
            role: PublicKeyRole::RemotePresetSignature,
            identifier: "remote-preset-minisign-pubkey",
            source_path: "src-tauri/src/presets/mod.rs -> MINISIGN_PUBLIC_KEY",
            expected_usage: "Verifies remote preset payload signatures before JSON parsing.",
        },
    ]
}
