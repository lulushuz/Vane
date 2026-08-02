use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub(crate) enum PathSecurityError {
    #[error("Artifact path must be a normalized relative path: {0:?}")]
    InvalidRelativePath(PathBuf),
    #[error("Path is outside resource root: {0:?}")]
    OutsideResourceRoot(PathBuf),
    #[error("Path contains forbidden symlink or reparse point: {0:?}")]
    SymlinkOrReparsePointRejected(PathBuf),
    #[error("Path is not a regular file: {0:?}")]
    NotRegularFile(PathBuf),
    #[error("Failed to query metadata for path {0:?}: {1}")]
    MetadataFailed(PathBuf, std::io::Error),
    #[error("Failed to canonicalize path {0:?}: {1}")]
    CanonicalizeFailed(PathBuf, std::io::Error),
}

pub(crate) fn validate_and_resolve_resource_path(
    resource_root: &Path,
    relative_path: &Path,
) -> Result<PathBuf, PathSecurityError> {
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(PathSecurityError::InvalidRelativePath(
            relative_path.to_path_buf(),
        ));
    }
    let canonical_root = resource_root
        .canonicalize()
        .map_err(|e| PathSecurityError::CanonicalizeFailed(resource_root.to_path_buf(), e))?;

    let mut full_target = resource_root.to_path_buf();
    for component in relative_path.components() {
        let std::path::Component::Normal(segment) = component else {
            unreachable!()
        };
        full_target.push(segment);
        let segment_meta = fs::symlink_metadata(&full_target)
            .map_err(|e| PathSecurityError::MetadataFailed(full_target.clone(), e))?;
        if segment_meta.file_type().is_symlink() {
            return Err(PathSecurityError::SymlinkOrReparsePointRejected(
                full_target,
            ));
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if segment_meta.file_attributes() & 0x400 != 0 {
                return Err(PathSecurityError::SymlinkOrReparsePointRejected(
                    full_target,
                ));
            }
        }
    }

    // Re-read the final entry immediately before canonicalization.
    let symlink_meta = fs::symlink_metadata(&full_target)
        .map_err(|e| PathSecurityError::MetadataFailed(full_target.clone(), e))?;

    if symlink_meta.file_type().is_symlink() {
        return Err(PathSecurityError::SymlinkOrReparsePointRejected(
            full_target,
        ));
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if (symlink_meta.file_attributes() & 0x400) != 0 {
            return Err(PathSecurityError::SymlinkOrReparsePointRejected(
                full_target,
            ));
        }
    }

    // 2. Canonicalize target
    let canonical_target = full_target
        .canonicalize()
        .map_err(|e| PathSecurityError::CanonicalizeFailed(full_target.clone(), e))?;

    // 3. Containment check
    if !canonical_target.starts_with(&canonical_root) {
        return Err(PathSecurityError::OutsideResourceRoot(canonical_target));
    }

    // 4. Regular file check
    let file_meta = fs::metadata(&canonical_target)
        .map_err(|e| PathSecurityError::MetadataFailed(canonical_target.clone(), e))?;

    if !file_meta.is_file() {
        return Err(PathSecurityError::NotRegularFile(canonical_target));
    }

    Ok(canonical_target)
}
