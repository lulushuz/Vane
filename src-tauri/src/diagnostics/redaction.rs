use regex::Regex;
use std::sync::OnceLock;

static IP_REGEX: OnceLock<Regex> = OnceLock::new();
static PATH_REGEX: OnceLock<Regex> = OnceLock::new();
static KEY_VALUE_SECRET_REGEX: OnceLock<Regex> = OnceLock::new();

fn ip_regex() -> &'static Regex {
    IP_REGEX.get_or_init(|| {
        Regex::new(
            r"\b(?:\d{1,3}\.){3}\d{1,3}\b|(?:::[a-fA-F0-9]{1,4}|[a-fA-F0-9]{1,4}:[a-fA-F0-9:]+)\b",
        )
        .expect("IP regex compilation failed")
    })
}

fn path_regex() -> &'static Regex {
    PATH_REGEX.get_or_init(|| {
        Regex::new(r"(?i)(?:[a-z]:\\[^:\s]+|/(?:Users|home|root|var|etc|tmp)/[^\s]+)")
            .expect("Path regex compilation failed")
    })
}

fn key_value_secret_regex() -> &'static Regex {
    KEY_VALUE_SECRET_REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(password|secret|key|token|auth|credential|proxy_pass|auth_token)\s*[:=]\s*([^\s,;&]+)")
            .expect("Secret regex compilation failed")
    })
}

pub struct DiagnosticRedactor;

impl DiagnosticRedactor {
    pub fn sanitize_text(text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }

        // 1. Redact key-value secrets (e.g. password=xyz)
        let step1 = key_value_secret_regex().replace_all(text, "$1=[REDACTED_SECRET]");

        // 2. Redact file paths
        let step2 = path_regex().replace_all(&step1, "[REDACTED_PATH]");

        // 3. Redact IP addresses
        let step3 = ip_regex().replace_all(&step2, "[REDACTED_IP]");

        step3.to_string()
    }

    pub fn sanitize_domain_list(domains: &[String]) -> String {
        format!("Count: {}", domains.len())
    }

    pub fn sanitize_cli_args(_args: &[String]) -> String {
        "[REDACTED_CLI_ARGS]".to_string()
    }
}
