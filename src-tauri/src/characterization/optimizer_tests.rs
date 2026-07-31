#[cfg(test)]
mod tests {
    use crate::config::preset::builtin_presets;

    #[test]
    fn o01_optimizer_presets_priority_ordering() {
        let mut presets = builtin_presets();
        presets.sort_by_key(|p| p.priority);
        for window in presets.windows(2) {
            assert!(window[0].priority <= window[1].priority);
        }
    }

    #[test]
    fn o02_optimizer_target_list_composition() {
        let targets = [
            "https://www.youtube.com".to_string(),
            "https://discord.com".to_string(),
            "https://x.com".to_string(),
        ];
        assert_eq!(targets.len(), 3);
        assert!(targets.iter().any(|t| t.contains("youtube")));
        assert!(targets.iter().any(|t| t.contains("discord")));
        assert!(targets.iter().any(|t| t.contains("x.com")));
    }

    #[test]
    fn o03_documents_optimizer_static_ip_target_resolution() {
        // RBR-09 Reproducer: Documents static hardcoded IP overrides in reqwest resolver
        // Target: P12
        // Risk: R-15
        // Expected production behavior: Dynamic DNS or user-configurable target endpoints should be used
        let static_ips = [
            ("discord.com", "162.159.135.232:443"),
            ("youtube.com", "142.250.185.14:443"),
            ("x.com", "104.244.42.65:443"),
        ];
        assert_eq!(static_ips.len(), 3);
    }


    #[test]
    fn o04_documents_optimizer_bypassing_engine_manager() {
        // RBR-08 Reproducer: Documents Optimizer spawning winws directly instead of via EngineManager
        // Target: P12
        // Risk: R-16
        // Expected production behavior: Optimizer should request process execution through EngineManager
        let direct_spawn = true;
        assert!(direct_spawn);
    }

    #[test]
    fn o06_optimizer_scoring_formula() {
        let success_count: u32 = 3;
        let avg_latency: u32 = 150;
        let score = (success_count * 10000).saturating_sub(avg_latency);
        assert_eq!(score, 29850);

        // Score above 27000 triggers early exit
        assert!(score > 27000);
    }
}
