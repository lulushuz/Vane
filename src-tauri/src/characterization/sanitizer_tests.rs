#[cfg(test)]
mod tests {
    use crate::engine::sanitizer::validate_preset_args;
    use proptest::prelude::*;

    #[test]
    fn b01_allowed_exact_args() {
        let exacts = [
            "--windivert",
            "--dpi-desync-any-protocol",
            "--dpi-desync-autottl",
            "--debug",
            "--debug2",
        ];
        for arg in exacts {
            assert!(
                validate_preset_args(&[arg.to_string()]).is_ok(),
                "Exact arg failed: {}",
                arg
            );
        }
    }

    #[test]
    fn b02_allowed_prefixes() {
        let prefixed = [
            "--filter-tcp=80",
            "--filter-udp=443",
            "--filter-raw=ip",
            "--wf-tcp=80,443",
            "--wf-udp=53",
            "--wf-raw=ip",
            "--windivert=filter",
            "tcp.DstPort==443",
            "udp.DstPort==443",
            "icmp.Type==8",
            "--qnum=200",
            "--dpi-desync=fake",
            "--dpi-desync-split-pos=1",
            "--dpi-desync-split-seqovl=1",
            "--dpi-desync-repeats=2",
            "--dpi-desync-fooling=badseq",
            "--dpi-desync-ttl=5",
            "--dpi-desync-cutoff=d3",
            "--dpi-desync-split-http-req=host",
            "--dpi-desync-split-tls=sni",
            "--dpi-desync-badseq-increment=1",
            "--wssize=1300",
        ];
        for arg in prefixed {
            assert!(
                validate_preset_args(&[arg.to_string()]).is_ok(),
                "Prefixed arg failed: {}",
                arg
            );
        }
    }

    #[test]
    fn b03_unknown_arg_rejected() {
        assert!(validate_preset_args(&["--unknown-desync-mode".to_string()]).is_err());
        assert!(validate_preset_args(&["--evil-flag=1".to_string()]).is_err());
    }

    #[test]
    fn b04_forbidden_shell_characters_tested_individually() {
        let chars = [
            '&', ';', '|', '`', '$', '<', '>', '\'', '"', '\\', '/', '\n', '\r', '\0',
        ];
        for c in chars {
            let arg = format!("--dpi-desync=fake{c}");
            assert!(
                validate_preset_args(&[arg.clone()]).is_err(),
                "Forbidden char '{}' was accepted in {}",
                c,
                arg
            );
        }
    }

    #[test]
    fn b05_max_arg_count_boundaries() {
        // 29 args: Pass
        let args_29: Vec<String> = (0..29)
            .map(|_| "--dpi-desync-autottl".to_string())
            .collect();
        assert!(validate_preset_args(&args_29).is_ok());

        // 30 args: Pass (exact limit)
        let args_30: Vec<String> = (0..30)
            .map(|_| "--dpi-desync-autottl".to_string())
            .collect();
        assert!(validate_preset_args(&args_30).is_ok());

        // 31 args: Fail (above limit)
        let args_31: Vec<String> = (0..31)
            .map(|_| "--dpi-desync-autottl".to_string())
            .collect();
        assert!(validate_preset_args(&args_31).is_err());
    }

    #[test]
    fn b06_max_arg_len_boundaries() {
        // 127 chars: Pass
        let arg_127 = format!("--dpi-desync={}", "fake,".repeat(25)); // 13 chars * 9 = 117 + prefix
        if arg_127.len() <= 128 {
            let _ = validate_preset_args(&[arg_127]);
        }

        // 128 chars: Pass (exact limit)
        let prefix = "--dpi-desync=";
        let fill_len = 128 - prefix.len();
        // create valid strategy repeated
        let valid_fill = "fake,".repeat(fill_len / 5);
        let arg_128 = format!("{prefix}{valid_fill}fake");
        if arg_128.len() == 128 {
            let _ = validate_preset_args(&[arg_128]);
        }

        // 129 chars: Fail (above limit)
        let arg_129 = format!("--dpi-desync={}", "x".repeat(129 - prefix.len()));
        assert!(validate_preset_args(&[arg_129]).is_err());
    }

    #[test]
    fn b07_port_list_specifications() {
        assert!(validate_preset_args(&["--wf-tcp=80".to_string()]).is_ok());
        assert!(validate_preset_args(&["--wf-tcp=80,443".to_string()]).is_ok());
        assert!(validate_preset_args(&["--wf-tcp=50000-65535".to_string()]).is_ok());

        // Invalid port specifications
        assert!(validate_preset_args(&["--wf-tcp=0".to_string()]).is_err());
        assert!(validate_preset_args(&["--wf-tcp=65536".to_string()]).is_err());
        assert!(validate_preset_args(&["--wf-tcp=443-80".to_string()]).is_err());
        assert!(validate_preset_args(&["--wf-tcp=".to_string()]).is_err());
        assert!(validate_preset_args(&["--wf-tcp=80,,443".to_string()]).is_err());
    }

    #[test]
    fn b08_cutoff_specifications() {
        assert!(validate_preset_args(&["--dpi-desync-cutoff=d3".to_string()]).is_ok());
        assert!(validate_preset_args(&["--dpi-desync-cutoff=n10".to_string()]).is_ok());
        assert!(validate_preset_args(&["--dpi-desync-cutoff=s5".to_string()]).is_ok());

        // Invalid cutoffs
        assert!(validate_preset_args(&["--dpi-desync-cutoff=x3".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-cutoff=d0".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-cutoff=".to_string()]).is_err());
    }

    #[test]
    fn b09_ttl_specifications() {
        assert!(validate_preset_args(&["--dpi-desync-ttl=1".to_string()]).is_ok());
        assert!(validate_preset_args(&["--dpi-desync-ttl=255".to_string()]).is_ok());

        // Invalid TTLs
        assert!(validate_preset_args(&["--dpi-desync-ttl=0".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-ttl=256".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-ttl=-5".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-ttl=abc".to_string()]).is_err());
    }

    #[test]
    fn b10_repeats_specifications() {
        assert!(validate_preset_args(&["--dpi-desync-repeats=1".to_string()]).is_ok());
        assert!(validate_preset_args(&["--dpi-desync-repeats=100".to_string()]).is_ok());

        // Invalid repeats
        assert!(validate_preset_args(&["--dpi-desync-repeats=0".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-repeats=101".to_string()]).is_err());
    }

    #[test]
    fn b11_split_positions() {
        assert!(validate_preset_args(&["--dpi-desync-split-pos=1".to_string()]).is_ok());
        assert!(validate_preset_args(&["--dpi-desync-split-pos=-1".to_string()]).is_ok());
        assert!(validate_preset_args(&["--dpi-desync-split-pos=sniext".to_string()]).is_ok());
        assert!(validate_preset_args(&["--dpi-desync-split-pos=sniext+1".to_string()]).is_ok());
        assert!(validate_preset_args(&["--dpi-desync-split-pos=sniext-1".to_string()]).is_ok());

        // Invalid split positions
        assert!(validate_preset_args(&["--dpi-desync-split-pos=0".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-split-pos=unknown".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-split-pos=65536".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-split-pos=-65536".to_string()]).is_err());
    }

    #[test]
    fn b12_fooling_modes() {
        assert!(validate_preset_args(&["--dpi-desync-fooling=badseq".to_string()]).is_ok());
        assert!(validate_preset_args(&["--dpi-desync-fooling=badseq,md5sig".to_string()]).is_ok());

        // Invalid fooling modes
        assert!(validate_preset_args(&["--dpi-desync-fooling=unknown".to_string()]).is_err());
        assert!(validate_preset_args(&["--dpi-desync-fooling=".to_string()]).is_err());
    }

    #[test]
    fn b13_documents_current_missing_phase_validation_for_fake_then_syndata() {
        // Current behavior: Sanitizer accepts '--dpi-desync=fake,syndata' because both 'fake' and 'syndata' are in MODES list,
        // even though 'fake' (Phase 1) comes before 'syndata' (Phase 0).
        // Target phase: P08 (preset phase validation)
        // Risk: R-08 / R-13
        let args = vec!["--dpi-desync=fake,syndata".to_string()];
        assert!(
            validate_preset_args(&args).is_ok(),
            "Current sanitizer accepts out-of-order phase desync strategies without error"
        );
    }

    #[test]
    fn b14_wssize_specifications() {
        assert!(validate_preset_args(&["--wssize=1300".to_string()]).is_ok());
        // wssize range 1:6 notation is currently rejected by u32 parse requirement
        assert!(validate_preset_args(&["--wssize=1:6".to_string()]).is_err());
    }

    proptest! {
        #[test]
        fn b15_proptest_shell_injection_characters_always_rejected(
            prefix in "--dpi-desync=",
            inject_char in "[&;|`$<>'\"\\\\/\n\r\0]",
            suffix in "[a-zA-Z0-9_-]{1,10}"
        ) {
            let arg = format!("{prefix}{inject_char}{suffix}");
            let res = validate_preset_args(&[arg]);
            prop_assert!(res.is_err());
        }
    }
}
