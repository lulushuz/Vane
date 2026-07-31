use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

static EMBEDDED_NATIVE_MANIFEST: &str = include_str!("../../security/native-artifacts.json");
#[allow(dead_code)]
static EMBEDDED_CONTENT_MANIFEST: &str = include_str!("../../security/content-artifacts.json");

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ArtifactId(pub String);

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ArtifactPlatform {
    #[serde(rename = "windows-x86_64")]
    WindowsX86_64,
    #[serde(rename = "linux-x86_64")]
    LinuxX86_64,
}

impl ArtifactPlatform {
    pub fn current() -> Option<Self> {
        if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Some(Self::WindowsX86_64)
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Some(Self::LinuxX86_64)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ArtifactRole {
    EngineExecutable,
    Driver,
    DynamicLibrary,
    RuntimeDependency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Sha256Digest(pub String);

impl Sha256Digest {
    pub fn validate(&self) -> bool {
        self.0.len() == 64 && self.0.chars().all(|c| c.is_ascii_hexdigit())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SafeRelativeArtifactPath(pub String);

impl SafeRelativeArtifactPath {
    pub fn validate(&self) -> bool {
        let p = Path::new(&self.0);
        if p.is_absolute() {
            return false;
        }
        for comp in p.components() {
            match comp {
                std::path::Component::Normal(_) => {}
                _ => return false,
            }
        }
        true
    }

    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(&self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeArtifactEntry {
    pub id: ArtifactId,
    pub platform: ArtifactPlatform,
    pub role: ArtifactRole,
    pub relative_path: SafeRelativeArtifactPath,
    pub size: u64,
    pub sha256: Sha256Digest,
    pub required: bool,
    pub component: String,
    pub component_version: String,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeArtifactManifest {
    pub schema_version: u32,
    pub application_version: String,
    pub artifacts: Vec<NativeArtifactEntry>,
}

impl NativeArtifactManifest {
    pub fn load_embedded() -> Result<Self, String> {
        let manifest: Self = serde_json::from_str(EMBEDDED_NATIVE_MANIFEST)
            .map_err(|e| format!("Failed to parse embedded native manifest: {e}"))?;

        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "Unsupported manifest schema version: {}",
                self.schema_version
            ));
        }

        if self.application_version.is_empty() {
            return Err("Manifest applicationVersion cannot be empty.".into());
        }

        let mut seen_ids = HashSet::new();
        let mut seen_paths = HashSet::new();

        for entry in &self.artifacts {
            if entry.id.0.is_empty() {
                return Err("Artifact ID cannot be empty.".into());
            }

            if !seen_ids.insert(entry.id.clone()) {
                return Err(format!("Duplicate artifact ID: {}", entry.id));
            }

            if !entry.relative_path.validate() {
                return Err(format!(
                    "Unsafe relative path in manifest: {}",
                    entry.relative_path.0
                ));
            }

            if !seen_paths.insert(entry.relative_path.0.clone()) {
                return Err(format!(
                    "Duplicate artifact relative path: {}",
                    entry.relative_path.0
                ));
            }

            if !entry.sha256.validate() {
                return Err(format!("Invalid SHA-256 digest format for {}", entry.id));
            }

            if entry.size == 0 {
                return Err(format!("Artifact size cannot be zero for {}", entry.id));
            }

            if entry.component.is_empty() || entry.component_version.is_empty() {
                return Err(format!(
                    "Component and component_version cannot be empty for {}",
                    entry.id
                ));
            }
        }

        Ok(())
    }

    pub fn entries_for_platform(&self, platform: ArtifactPlatform) -> Vec<&NativeArtifactEntry> {
        self.artifacts
            .iter()
            .filter(|a| a.platform == platform)
            .collect()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContentArtifactEntry {
    pub id: ArtifactId,
    pub relative_path: SafeRelativeArtifactPath,
    pub size: u64,
    pub sha256: Sha256Digest,
    pub required: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContentArtifactManifest {
    pub schema_version: u32,
    pub application_version: String,
    pub artifacts: Vec<ContentArtifactEntry>,
}

#[allow(dead_code)]
impl ContentArtifactManifest {
    pub fn load_embedded() -> Result<Self, String> {
        let manifest: Self = serde_json::from_str(EMBEDDED_CONTENT_MANIFEST)
            .map_err(|e| format!("Failed to parse embedded content manifest: {e}"))?;

        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "Unsupported content manifest schema version: {}",
                self.schema_version
            ));
        }

        for entry in &self.artifacts {
            if !entry.relative_path.validate() {
                return Err(format!(
                    "Unsafe content relative path: {}",
                    entry.relative_path.0
                ));
            }

            if !entry.sha256.validate() {
                return Err(format!("Invalid content SHA-256 digest for {}", entry.id));
            }
        }

        Ok(())
    }
}
