#[cfg(test)]
mod tests {
    use crate::characterization::TempTestDir;
    use crate::diagnostics::{
        create_diagnostics_bundle, export_bundle_to_file, perform_local_consistency_checks,
        DiagnosticComponent, DiagnosticEvent, DiagnosticEventCode, DiagnosticEventStore,
        DiagnosticRedactor, DiagnosticSeverity, DpiBypassAssessment, HealthState,
        SafeDiagnosticValue, TrafficProbeRunner,
    };

    #[test]
    fn group_a01_monotonic_sequence_and_event_fields() {
        let e1 = DiagnosticEvent::new(
            DiagnosticComponent::Engine,
            DiagnosticEventCode::EngStartInit,
            DiagnosticSeverity::Info,
        )
        .with_field("preset", SafeDiagnosticValue::Text("tr-1".into()));

        let e2 = DiagnosticEvent::new(
            DiagnosticComponent::Config,
            DiagnosticEventCode::PatCacheUpdated,
            DiagnosticSeverity::Debug,
        );

        assert!(e2.sequence > e1.sequence);
        assert_eq!(e1.code.as_str(), "ENG_START_INIT");
        assert_eq!(
            e1.fields.get("preset"),
            Some(&SafeDiagnosticValue::Text("tr-1".into()))
        );
    }

    #[test]
    fn group_b01_diagnostic_redactor_strips_sensitive_data() {
        let raw_text = "Failed connecting to 192.168.1.100 with password=secret_pass123 in C:\\Users\\Administrator\\AppData\\Local";
        let sanitized = DiagnosticRedactor::sanitize_text(raw_text);

        assert!(!sanitized.contains("192.168.1.100"));
        assert!(!sanitized.contains("secret_pass123"));
        assert!(!sanitized.contains("Administrator"));
        assert!(sanitized.contains("[REDACTED_IP]"));
        assert!(sanitized.contains("[REDACTED_SECRET]"));
        assert!(sanitized.contains("[REDACTED_PATH]"));
    }

    #[tokio::test]
    async fn group_c01_event_store_capacity_ring_buffer_and_drop_counter() {
        let store = DiagnosticEventStore::new(5);

        for i in 0..10 {
            let event = DiagnosticEvent::new(
                DiagnosticComponent::System,
                DiagnosticEventCode::HealthCheckLocal,
                DiagnosticSeverity::Info,
            )
            .with_field("index", SafeDiagnosticValue::Int(i));

            store.push(event).await;
        }

        let events = store.get_events(None).await;
        assert_eq!(events.len(), 5);
        assert_eq!(store.dropped_count(), 5);
        assert_eq!(
            events.last().unwrap().fields.get("index"),
            Some(&SafeDiagnosticValue::Int(9))
        );
    }

    #[test]
    fn group_d01_subsystem_health_combination_logic() {
        let h1 = HealthState::Healthy;
        let h2 = HealthState::Degraded;
        let h3 = HealthState::Unhealthy;

        assert_eq!(h1.combine(h2), HealthState::Degraded);
        assert_eq!(h2.combine(h3), HealthState::Unhealthy);
        assert_eq!(h1.combine(h1), HealthState::Healthy);
    }

    #[test]
    fn group_e01_local_consistency_checks_are_side_effect_free() {
        let temp = TempTestDir::new("diag-local");
        let snapshot = perform_local_consistency_checks(temp.path());

        assert!(!snapshot.subsystems.is_empty());
        assert!(snapshot.timestamp_ms > 0);
    }

    #[tokio::test]
    async fn group_f01_traffic_probe_returns_inconclusive_assessment() {
        let runner = TrafficProbeRunner::new();
        let res = runner.run_probes(&["example.com".into()]).await;

        assert!(res.is_ok());
        let report = res.unwrap();
        assert_eq!(report.assessment, DpiBypassAssessment::Inconclusive);
    }

    #[test]
    fn group_g01_diagnostics_bundle_creation_and_atomic_file_write() {
        let temp = TempTestDir::new("diag-bundle");
        let snapshot = perform_local_consistency_checks(temp.path());
        let event = DiagnosticEvent::new(
            DiagnosticComponent::Security,
            DiagnosticEventCode::SecArtifactVerified,
            DiagnosticSeverity::Info,
        )
        .with_field(
            "path",
            SafeDiagnosticValue::Text("C:\\Users\\admin\\secret.txt".into()),
        );

        let bundle = create_diagnostics_bundle(snapshot, vec![event], 0);
        assert!(bundle.secret_scanner_passed);
        assert_eq!(bundle.schema_version, "1.0");

        let target_file = temp.path().join("test-diag.vane-diag.json");
        let write_res = export_bundle_to_file(&bundle, &target_file);
        assert!(write_res.is_ok());
        assert!(target_file.exists());
    }
}
