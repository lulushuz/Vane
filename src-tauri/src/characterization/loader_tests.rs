#[cfg(test)]
mod tests {
    use crate::characterization::TempTestDir;
    use crate::config::loader::ConfigLoader;
    use crate::config::preset::Preset;
    use std::fs;

    #[test]
    fn d01_builtin_presets_loaded_by_default() {
        let loader = ConfigLoader::new();
        assert!(!loader.all_presets().is_empty());
        assert!(loader.find_preset("default").is_some());
    }

    #[test]
    fn d02_load_custom_preset_from_temp_dir() {
        let temp = TempTestDir::new("d02");
        let preset_path = temp.path().join("my-custom.json");
        let custom_json = r#"{
            "id": "my-custom",
            "label": "My Custom",
            "description": "Test",
            "icon": "zap",
            "args": ["--wf-tcp=80,443"],
            "isCustom": true
        }"#;
        fs::write(&preset_path, custom_json).unwrap();

        let mut loader = ConfigLoader::new();
        loader.load_custom_presets_from(temp.path());

        let found = loader.find_preset("my-custom");
        assert!(found.is_some());
        assert!(found.unwrap().is_custom);
    }

    #[test]
    fn d03_corrupt_json_custom_preset_handling() {
        let temp = TempTestDir::new("d03");
        let corrupt_path = temp.path().join("corrupt.json");
        fs::write(&corrupt_path, "{ malformed json ...").unwrap();

        let mut loader = ConfigLoader::new();
        // Should not panic, continues functioning
        loader.load_custom_presets_from(temp.path());
        assert!(loader.find_preset("corrupt").is_none());
    }

    #[test]
    fn d04_backup_recovery_when_primary_json_damaged() {
        let temp = TempTestDir::new("d04");
        let primary_path = temp.path().join("recover-me.json");
        let backup_path = temp.path().join("recover-me.json.bak");

        let valid_json = r#"{
            "id": "recover-me",
            "label": "Recover Me",
            "description": "Desc",
            "icon": "zap",
            "args": ["--wf-tcp=80"],
            "isCustom": true
        }"#;

        fs::write(&primary_path, "{ corrupt content").unwrap();
        fs::write(&backup_path, valid_json).unwrap();

        let mut loader = ConfigLoader::new();
        loader.load_custom_presets_from(temp.path());

        let found = loader.find_preset("recover-me");
        assert!(found.is_some());
    }

    #[test]
    fn d05_invalid_id_quarantine() {
        let temp = TempTestDir::new("d05");
        let invalid_ids = ["../evil", "evil/path", "", "space id"];

        let mut loader = ConfigLoader::new();
        for (i, id) in invalid_ids.iter().enumerate() {
            let file_path = temp.path().join(format!("test{i}.json"));
            let json = format!(
                r#"{{
                "id": "{id}",
                "label": "Invalid ID",
                "description": "",
                "icon": "zap",
                "args": ["--wf-tcp=80"],
                "isCustom": true
            }}"#
            );
            fs::write(&file_path, json).unwrap();
        }

        loader.load_custom_presets_from(temp.path());
        // None of the invalid IDs should be present
        for id in invalid_ids {
            assert!(loader.find_preset(id).is_none());
        }
    }

    #[test]
    fn d06_unsafe_args_quarantine() {
        let temp = TempTestDir::new("d06");
        let unsafe_path = temp.path().join("unsafe.json");
        let unsafe_json = r#"{
            "id": "unsafe",
            "label": "Unsafe",
            "description": "",
            "icon": "zap",
            "args": ["--dpi-desync=fake; calc.exe"],
            "isCustom": true
        }"#;
        fs::write(&unsafe_path, unsafe_json).unwrap();

        let mut loader = ConfigLoader::new();
        loader.load_custom_presets_from(temp.path());

        assert!(loader.find_preset("unsafe").is_none());
    }

    #[test]
    fn d07_builtin_id_collision_prevention() {
        let temp = TempTestDir::new("d07");
        let collision_path = temp.path().join("default.json");
        let collision_json = r#"{
            "id": "default",
            "label": "Overridden Default",
            "description": "",
            "icon": "zap",
            "args": ["--wf-tcp=80"],
            "isCustom": true
        }"#;
        fs::write(&collision_path, collision_json).unwrap();

        let mut loader = ConfigLoader::new();
        loader.load_custom_presets_from(temp.path());

        let found = loader.find_preset("default").unwrap();
        // Built-in should NOT be overridden by custom file
        assert!(!found.is_custom);
    }

    #[test]
    fn d08_save_custom_preset_atomic_and_backup() {
        let temp = TempTestDir::new("d08");
        let mut loader = ConfigLoader::new();

        let new_preset = Preset {
            id: "saved-p1".to_string(),
            label: "Saved P1".to_string(),
            description: "Desc".to_string(),
            icon: "zap".to_string(),
            args: vec!["--wf-tcp=80,443".to_string()],
            is_custom: true,
            priority: 0,
            category: Default::default(),
        };

        // First save
        let res = loader.save_custom_preset(new_preset.clone(), temp.path());
        assert!(res.is_ok());
        assert!(temp.path().join("saved-p1.json").exists());

        // Update save (should create backup)
        let mut updated = new_preset;
        updated.label = "Saved P1 Updated".to_string();
        let res_update = loader.save_custom_preset(updated, temp.path());
        assert!(res_update.is_ok());
        assert!(temp.path().join("saved-p1.json.bak").exists());
    }

    #[test]
    fn d09_delete_custom_preset() {
        let temp = TempTestDir::new("d09");
        let mut loader = ConfigLoader::new();

        let preset = Preset {
            id: "to-delete".to_string(),
            label: "Delete Me".to_string(),
            description: "".to_string(),
            icon: "zap".to_string(),
            args: vec!["--wf-tcp=80".to_string()],
            is_custom: true,
            priority: 0,
            category: Default::default(),
        };

        loader.save_custom_preset(preset, temp.path()).unwrap();
        assert!(loader.find_preset("to-delete").is_some());

        loader
            .delete_custom_preset("to-delete", temp.path())
            .unwrap();
        assert!(loader.find_preset("to-delete").is_none());
        assert!(!temp.path().join("to-delete.json").exists());
    }

    #[test]
    fn d10_remote_presets_loading_and_validation() {
        let mut loader = ConfigLoader::new();
        let remote = vec![
            Preset {
                id: "remote-1".to_string(),
                label: "Remote 1".to_string(),
                description: "".to_string(),
                icon: "zap".to_string(),
                args: vec!["--wf-tcp=80".to_string()],
                is_custom: false,
                priority: 0,
                category: Default::default(),
            },
            Preset {
                id: "default".to_string(), // Collision with built-in
                label: "Malicious Overwrite".to_string(),
                description: "".to_string(),
                icon: "zap".to_string(),
                args: vec!["--wf-tcp=80".to_string()],
                is_custom: false,
                priority: 0,
                category: Default::default(),
            },
        ];

        loader.load_remote_presets(remote);
        assert!(loader.find_preset("remote-1").is_some());
        // Default built-in preserved
        assert_ne!(
            loader.find_preset("default").unwrap().label,
            "Malicious Overwrite"
        );
    }
}
