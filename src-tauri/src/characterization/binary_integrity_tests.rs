#[cfg(test)]
mod tests {
    use crate::security::artifact_integrity::{
        ArtifactIntegrityError, ArtifactIntegrityVerifier, Sha256ArtifactIntegrityVerifier,
    };
    use crate::security::artifact_manifest::{
        ArtifactId, ArtifactPlatform, ArtifactRole, NativeArtifactEntry, NativeArtifactManifest,
        SafeRelativeArtifactPath, Sha256Digest,
    };
    use crate::security::artifact_path::validate_and_resolve_resource_path;
    use crate::security::supply_chain::{
        get_public_key_role_mappings, verify_content_artifacts, PublicKeyRole,
    };
    use std::fs::File;
    use std::io::Write;

    #[cfg(unix)]
    fn make_executable(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &std::path::Path) {}

    // ─── Group A: Manifest Validation ───

    #[test]
    fn group_a01_valid_embedded_manifest() {
        let manifest = NativeArtifactManifest::load_embedded().unwrap();
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.application_version, "1.0.0-rc.1");
        assert!(!manifest.artifacts.is_empty());
    }

    #[test]
    fn group_a02_reject_unknown_schema_version() {
        let mut manifest = NativeArtifactManifest::load_embedded().unwrap();
        manifest.schema_version = 999;
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn group_a03_reject_duplicate_artifact_id() {
        let mut manifest = NativeArtifactManifest::load_embedded().unwrap();
        let dup = manifest.artifacts[0].clone();
        manifest.artifacts.push(dup);
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn group_a04_reject_parent_path_traversal() {
        let unsafe_path = SafeRelativeArtifactPath("../binaries/malware.exe".into());
        assert!(!unsafe_path.validate());
    }

    #[test]
    fn group_a05_reject_invalid_sha_digest() {
        let invalid = Sha256Digest("not_64_hex_chars".into());
        assert!(!invalid.validate());
    }

    // ─── Group B: Hash & Size Verification ───

    #[test]
    fn group_b01_verify_embedded_artifacts_match_disk_manifest() {
        let temp = crate::characterization::TempTestDir::new("manifest-test");

        // Copy actual binaries to temp dir
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let binaries_dir = repo_root.join("binaries");
        let temp_binaries = temp.path().join("binaries");
        std::fs::create_dir_all(&temp_binaries).unwrap();

        if let Ok(entries) = std::fs::read_dir(&binaries_dir) {
            for entry in entries.flatten() {
                let dest = temp_binaries.join(entry.file_name());
                let _ = std::fs::copy(entry.path(), dest);
            }
        }

        let verifier = Sha256ArtifactIntegrityVerifier::from_embedded().unwrap();
        let res = verifier.verify_current_platform_group(temp.path());

        assert!(res.is_ok(), "Verification failed: {:?}", res.err());
        let group = res.unwrap();
        assert_eq!(group.executable.sha256.0.len(), 64);
    }

    #[test]
    fn group_b02_detect_tampered_one_byte_modification() {
        let temp = crate::characterization::TempTestDir::new("tamper-test");

        // Prepare dummy binary
        let bin_dir = temp.path().join("binaries");
        std::fs::create_dir_all(&bin_dir).unwrap();

        let dummy_path = bin_dir.join("dummy.exe");
        {
            let mut f = File::create(&dummy_path).unwrap();
            f.write_all(b"original content").unwrap();
        }
        make_executable(&dummy_path);

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"original content");
        let hash_hex = format!("{:x}", hasher.finalize());

        let entry = NativeArtifactEntry {
            id: ArtifactId("test-bin".into()),
            platform: ArtifactPlatform::current().unwrap(),
            role: ArtifactRole::EngineExecutable,
            relative_path: SafeRelativeArtifactPath("binaries/dummy.exe".into()),
            size: 16,
            sha256: Sha256Digest(hash_hex),
            required: true,
            component: "test".into(),
            component_version: "1.0".into(),
            license: None,
        };

        let verifier = Sha256ArtifactIntegrityVerifier::from_embedded().unwrap();
        let valid_res = verifier.verify_single_entry(temp.path(), &entry);
        assert!(
            valid_res.is_ok(),
            "Expected valid_res ok, got {:?}",
            valid_res.err()
        );

        // Tamper with one byte
        {
            let mut f = File::create(&dummy_path).unwrap();
            f.write_all(b"xriginal content").unwrap();
        }

        let tampered_res = verifier.verify_single_entry(temp.path(), &entry);
        assert!(matches!(
            tampered_res,
            Err(ArtifactIntegrityError::ArtifactHashMismatch { .. })
        ));
    }

    // ─── Group C: Path Security ───

    #[test]
    fn group_c01_reject_outside_resource_root() {
        let root = std::path::Path::new("/tmp/root");
        let rel = std::path::Path::new("../etc/passwd");
        let res = validate_and_resolve_resource_path(root, rel);
        assert!(res.is_err());
    }

    // ─── Group D: Supply Chain & Key Roles ───

    #[test]
    fn group_d01_verify_public_key_role_mappings() {
        let mappings = get_public_key_role_mappings();
        assert_eq!(mappings.len(), 2);
        assert!(mappings
            .iter()
            .any(|m| m.role == PublicKeyRole::TauriUpdaterRelease));
        assert!(mappings
            .iter()
            .any(|m| m.role == PublicKeyRole::RemotePresetSignature));
    }

    #[test]
    fn group_d02_verify_content_artifacts_manifest() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let res = verify_content_artifacts(&repo_root);
        assert!(res.is_ok(), "Content verification failed: {:?}", res.err());
        let verified = res.unwrap();
        assert_eq!(verified.len(), 2);
    }

    // ─── Group E: Build-time Test Gate ───

    #[test]
    fn bundled_native_artifacts_match_the_trusted_manifest() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let verifier = Sha256ArtifactIntegrityVerifier::from_embedded().unwrap();
        let res = verifier.verify_current_platform_group(&repo_root);
        assert!(
            res.is_ok(),
            "Bundled native artifacts do not match trusted manifest: {:?}",
            res.err()
        );
    }
}
