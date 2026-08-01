use crate::security::artifact_manifest::{
    ArtifactId, ArtifactPlatform, ArtifactRole, NativeArtifactEntry, NativeArtifactManifest,
    Sha256Digest,
};
use crate::security::artifact_path::{validate_and_resolve_resource_path, PathSecurityError};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactIntegrityStatusDto {
    pub status: String,
    pub target: String,
    pub verified_artifacts: usize,
    pub failed_artifact_id: Option<String>,
    pub error_code: Option<String>,
    pub last_verified_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum ArtifactIntegrityError {
    #[error("Manifest parse failed: {0}")]
    ManifestParseFailed(String),

    #[error("Unsupported manifest version: {0}")]
    UnsupportedManifestVersion(u32),

    #[error("Manifest version mismatch: {0}")]
    ManifestVersionMismatch(String),

    #[error("Duplicate artifact ID: {0}")]
    DuplicateArtifactId(String),

    #[error("Duplicate artifact path: {0}")]
    DuplicateArtifactPath(String),

    #[error("Unsafe artifact path: {0}")]
    UnsafeArtifactPath(String),

    #[error("Artifact missing: {0}")]
    ArtifactMissing(String),

    #[error("Artifact is not a regular file: {0:?}")]
    ArtifactNotRegularFile(PathBuf),

    #[error("Artifact path is outside resource root: {0:?}")]
    ArtifactOutsideResourceRoot(PathBuf),

    #[error("Symbolic link rejected for artifact: {0:?}")]
    SymbolicLinkRejected(PathBuf),

    #[error("Reparse point rejected for artifact: {0:?}")]
    ReparsePointRejected(PathBuf),

    #[error("Artifact size mismatch for {id}: expected {expected}, got {actual}")]
    ArtifactSizeMismatch {
        id: String,
        expected: u64,
        actual: u64,
    },

    #[error("Artifact SHA-256 mismatch for {id}: expected {expected}, got {actual}")]
    ArtifactHashMismatch {
        id: String,
        expected: String,
        actual: String,
    },

    #[error("Artifact changed during verification (TOCTOU failure) for {0:?}")]
    ArtifactChangedDuringVerification(PathBuf),

    #[error("Artifact target platform mismatch: expected {expected:?}, got {actual:?}")]
    ArtifactTargetMismatch {
        expected: ArtifactPlatform,
        actual: ArtifactPlatform,
    },

    #[error("Required dependency missing: {0}")]
    RequiredDependencyMissing(String),

    #[error("Invalid executable permissions for {0:?}: {1}")]
    InvalidExecutablePermissions(PathBuf, String),

    #[error("Signature invalid for {0:?}: {1}")]
    SignatureInvalid(PathBuf, String),

    #[error("IO error during verification of {0:?}: {1}")]
    Io(PathBuf, std::io::Error),
}

impl From<PathSecurityError> for ArtifactIntegrityError {
    fn from(err: PathSecurityError) -> Self {
        match err {
            PathSecurityError::OutsideResourceRoot(p) => {
                ArtifactIntegrityError::ArtifactOutsideResourceRoot(p)
            }
            PathSecurityError::SymlinkOrReparsePointRejected(p) => {
                ArtifactIntegrityError::SymbolicLinkRejected(p)
            }
            PathSecurityError::NotRegularFile(p) => {
                ArtifactIntegrityError::ArtifactNotRegularFile(p)
            }
            PathSecurityError::MetadataFailed(p, e) => ArtifactIntegrityError::Io(p, e),
            PathSecurityError::CanonicalizeFailed(p, e) => ArtifactIntegrityError::Io(p, e),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactFileIdentity {
    pub size: u64,
    pub modified: Option<SystemTime>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct VerifiedBinaryArtifact {
    pub id: ArtifactId,
    pub role: ArtifactRole,
    pub canonical_path: PathBuf,
    pub size: u64,
    pub sha256: Sha256Digest,
    pub file_identity: ArtifactFileIdentity,
    pub verified_at: SystemTime,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct VerifiedArtifactGroup {
    pub platform: ArtifactPlatform,
    pub executable: VerifiedBinaryArtifact,
    pub dependencies: Vec<VerifiedBinaryArtifact>,
}

impl VerifiedArtifactGroup {
    pub(crate) fn new(
        platform: ArtifactPlatform,
        executable: VerifiedBinaryArtifact,
        dependencies: Vec<VerifiedBinaryArtifact>,
    ) -> Self {
        Self {
            platform,
            executable,
            dependencies,
        }
    }
}

pub(crate) trait ArtifactIntegrityVerifier: Send + Sync {
    fn verify_current_platform_group(
        &self,
        resource_root: &Path,
    ) -> Result<VerifiedArtifactGroup, ArtifactIntegrityError>;

    fn verify_single_entry(
        &self,
        resource_root: &Path,
        entry: &NativeArtifactEntry,
    ) -> Result<VerifiedBinaryArtifact, ArtifactIntegrityError>;
}

pub(crate) struct Sha256ArtifactIntegrityVerifier {
    manifest: NativeArtifactManifest,
}

impl Sha256ArtifactIntegrityVerifier {
    pub fn new(manifest: NativeArtifactManifest) -> Self {
        Self { manifest }
    }

    pub fn from_embedded() -> Result<Self, ArtifactIntegrityError> {
        let manifest = NativeArtifactManifest::load_embedded()
            .map_err(ArtifactIntegrityError::ManifestParseFailed)?;
        Ok(Self::new(manifest))
    }
}

impl ArtifactIntegrityVerifier for Sha256ArtifactIntegrityVerifier {
    fn verify_current_platform_group(
        &self,
        resource_root: &Path,
    ) -> Result<VerifiedArtifactGroup, ArtifactIntegrityError> {
        let current_platform = ArtifactPlatform::current().ok_or_else(|| {
            ArtifactIntegrityError::ManifestParseFailed("Unsupported OS/Arch platform".into())
        })?;

        let entries = self.manifest.entries_for_platform(current_platform);
        if entries.is_empty() {
            return Err(ArtifactIntegrityError::RequiredDependencyMissing(format!(
                "No artifact entries found for platform {:?}",
                current_platform
            )));
        }

        let mut executable: Option<VerifiedBinaryArtifact> = None;
        let mut dependencies = Vec::new();

        for entry in entries {
            let verified = self.verify_single_entry(resource_root, entry)?;
            if entry.role == ArtifactRole::EngineExecutable {
                executable = Some(verified);
            } else {
                dependencies.push(verified);
            }
        }

        let executable = executable.ok_or_else(|| {
            ArtifactIntegrityError::RequiredDependencyMissing(format!(
                "Engine executable missing for platform {:?}",
                current_platform
            ))
        })?;

        Ok(VerifiedArtifactGroup::new(
            current_platform,
            executable,
            dependencies,
        ))
    }

    fn verify_single_entry(
        &self,
        resource_root: &Path,
        entry: &NativeArtifactEntry,
    ) -> Result<VerifiedBinaryArtifact, ArtifactIntegrityError> {
        let rel_path = entry.relative_path.to_path_buf();
        let canonical_path = validate_and_resolve_resource_path(resource_root, &rel_path)?;

        // Open handle for streaming hash and metadata checks
        let mut file = File::open(&canonical_path)
            .map_err(|e| ArtifactIntegrityError::Io(canonical_path.clone(), e))?;

        let initial_meta = file
            .metadata()
            .map_err(|e| ArtifactIntegrityError::Io(canonical_path.clone(), e))?;

        let initial_size = initial_meta.len();
        let initial_mtime = initial_meta.modified().ok();

        if initial_size != entry.size {
            return Err(ArtifactIntegrityError::ArtifactSizeMismatch {
                id: entry.id.0.clone(),
                expected: entry.size,
                actual: initial_size,
            });
        }

        // Platform specific permission checks (Linux)
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = initial_meta.permissions().mode();

            if entry.role == ArtifactRole::EngineExecutable && (mode & 0o111) == 0 {
                return Err(ArtifactIntegrityError::InvalidExecutablePermissions(
                    canonical_path.clone(),
                    "Missing executable bit".into(),
                ));
            }

            if (mode & 0o002) != 0 {
                return Err(ArtifactIntegrityError::InvalidExecutablePermissions(
                    canonical_path.clone(),
                    "World-writable permissions forbidden".into(),
                ));
            }

            if (mode & 0o6000) != 0 {
                return Err(ArtifactIntegrityError::InvalidExecutablePermissions(
                    canonical_path.clone(),
                    "setuid/setgid bits forbidden".into(),
                ));
            }
        }

        // Streaming SHA-256 computation
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 65536];
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

        let computed_hash_hex: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        // Final metadata check for TOCTOU verification
        let final_meta = file
            .metadata()
            .map_err(|e| ArtifactIntegrityError::Io(canonical_path.clone(), e))?;

        if final_meta.len() != initial_size
            || final_meta.modified().ok() != initial_mtime
            || read_bytes != entry.size
        {
            return Err(ArtifactIntegrityError::ArtifactChangedDuringVerification(
                canonical_path,
            ));
        }

        if computed_hash_hex != entry.sha256.0.to_lowercase() {
            return Err(ArtifactIntegrityError::ArtifactHashMismatch {
                id: entry.id.0.clone(),
                expected: entry.sha256.0.to_lowercase(),
                actual: computed_hash_hex,
            });
        }

        Ok(VerifiedBinaryArtifact {
            id: entry.id.clone(),
            role: entry.role,
            canonical_path,
            size: read_bytes,
            sha256: Sha256Digest(computed_hash_hex),
            file_identity: ArtifactFileIdentity {
                size: read_bytes,
                modified: initial_mtime,
            },
            verified_at: SystemTime::now(),
        })
    }
}

#[allow(dead_code)]
pub(crate) struct FakeArtifactIntegrityVerifier {
    pub should_fail: bool,
    pub fail_error: Option<String>,
}

#[allow(dead_code)]
impl FakeArtifactIntegrityVerifier {
    pub fn new_passing() -> Self {
        Self {
            should_fail: false,
            fail_error: None,
        }
    }

    pub fn new_failing(err: impl Into<String>) -> Self {
        Self {
            should_fail: true,
            fail_error: Some(err.into()),
        }
    }
}

impl ArtifactIntegrityVerifier for FakeArtifactIntegrityVerifier {
    fn verify_current_platform_group(
        &self,
        resource_root: &Path,
    ) -> Result<VerifiedArtifactGroup, ArtifactIntegrityError> {
        if self.should_fail {
            let _msg = self
                .fail_error
                .clone()
                .unwrap_or_else(|| "Fake fail".into());
            return Err(ArtifactIntegrityError::ArtifactHashMismatch {
                id: "fake-id".into(),
                expected: "expected_hash".into(),
                actual: "actual_hash".into(),
            });
        }

        let real = Sha256ArtifactIntegrityVerifier::from_embedded()?;
        real.verify_current_platform_group(resource_root)
    }

    fn verify_single_entry(
        &self,
        resource_root: &Path,
        entry: &NativeArtifactEntry,
    ) -> Result<VerifiedBinaryArtifact, ArtifactIntegrityError> {
        if self.should_fail {
            return Err(ArtifactIntegrityError::ArtifactHashMismatch {
                id: entry.id.0.clone(),
                expected: entry.sha256.0.clone(),
                actual: "tampered_hash".into(),
            });
        }

        let real = Sha256ArtifactIntegrityVerifier::from_embedded()?;
        real.verify_single_entry(resource_root, entry)
    }
}
