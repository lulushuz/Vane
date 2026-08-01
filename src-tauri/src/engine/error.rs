use thiserror::Error;

/// Type-safe enum representing engine errors across the application.
/// Automatically serialized to JSON over Tauri IPC via `serde::Serialize`.
#[derive(Debug, Error, Clone)]
pub enum EngineError {
    #[error("Yetersiz yetki: Uygulamanın Yönetici (Administrator) olarak çalışması gerekiyor.")]
    InsufficientPrivileges,

    #[error("Motor zaten çalışıyor.")]
    AlreadyRunning,

    #[error("Motor çalışmıyor.")]
    NotRunning,

    #[error("Geçersiz preset: '{0}'")]
    InvalidPreset(String),

    #[error("Mevcut işlem için geçersiz ID: '{0}'")]
    InvalidId(String),

    #[error("Process start error: {0}")]
    SpawnFailed(String),

    #[error("Configuration parse error: {0}")]
    ConfigParseError(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Binary not found: {0}")]
    BinaryNotFound(String),

    #[error("Authorization error: {0}")]
    AuthorizationFailed(String),

    #[error("Vane motor dosyalarının bütünlük doğrulaması başarısız oldu: {0}")]
    ArtifactIntegrityError(String),
}

impl EngineError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InsufficientPrivileges => "INSUFFICIENT_PRIVILEGES",
            Self::AlreadyRunning => "ALREADY_RUNNING",
            Self::NotRunning => "NOT_RUNNING",
            Self::InvalidPreset(_) => "INVALID_PRESET",
            Self::InvalidId(_) => "INVALID_ID",
            Self::SpawnFailed(_) => "SPAWN_FAILED",
            Self::ConfigParseError(_) => "CONFIG_PARSE_ERROR",
            Self::IoError(_) => "IO_ERROR",
            Self::BinaryNotFound(_) => "BINARY_NOT_FOUND",
            Self::AuthorizationFailed(_) => "AUTHORIZATION_FAILED",
            Self::ArtifactIntegrityError(_) => "ARTIFACT_INTEGRITY_FAILED",
        }
    }
}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}

impl From<crate::security::ArtifactIntegrityError> for EngineError {
    fn from(e: crate::security::ArtifactIntegrityError) -> Self {
        Self::ArtifactIntegrityError(e.to_string())
    }
}

#[derive(serde::Serialize)]
struct ErrorPayload {
    code: String,
    message: String,
}

impl serde::Serialize for EngineError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let payload = ErrorPayload {
            code: self.code().to_string(),
            message: self.to_string(),
        };
        payload.serialize(serializer)
    }
}
