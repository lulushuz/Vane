#[cfg(test)]
mod tests {
    use crate::characterization::TempTestDir;
    use crate::presets::remote::{
        load_cached_presets_verified, MANIFEST_PUBLIC_KEY, REMOTE_PRESETS_URL,
    };
    use minisign_verify::PublicKey;
    use std::fs;

    #[test]
    fn m01_manifest_public_key_is_valid_base64() {
        assert!(!MANIFEST_PUBLIC_KEY.is_empty());
        let pub_key = PublicKey::from_base64(MANIFEST_PUBLIC_KEY);
        assert!(pub_key.is_ok(), "Embedded Minisign public key must parse");
    }

    #[test]
    fn m02_remote_presets_url_points_to_official_github_repo() {
        assert_eq!(
            REMOTE_PRESETS_URL,
            "https://raw.githubusercontent.com/lulushuz/Vane-Presets/main/presets.json"
        );
    }

    #[test]
    fn m06_load_cached_presets_verified_cleans_up_missing_signature() {
        let temp = TempTestDir::new("m06");
        let cache_path = temp.path().join("remote_presets_cache.json");
        fs::write(
            &cache_path,
            r#"{"version":"1.0.0","updatedAt":"2026-07-29","presets":[]}"#,
        )
        .unwrap();

        let pub_key = PublicKey::from_base64(MANIFEST_PUBLIC_KEY).unwrap();
        let res = load_cached_presets_verified(temp.path(), &pub_key);

        assert!(res.is_err());
        // Cache file without sig should be cleaned up
        assert!(!cache_path.exists());
    }

    #[test]
    fn m07_modified_manifest_byte_fails_verification() {
        let pub_key = PublicKey::from_base64(MANIFEST_PUBLIC_KEY).unwrap();
        let temp = TempTestDir::new("m07");

        // Create corrupt sig and cache files
        let cache_path = temp.path().join("remote_presets_cache.json");
        let sig_path = temp.path().join("remote_presets_cache.json.minisig");
        fs::write(&cache_path, b"corrupt content").unwrap();
        fs::write(&sig_path, b"untrusted signature").unwrap();

        let res = load_cached_presets_verified(temp.path(), &pub_key);
        assert!(res.is_err());
    }
}
