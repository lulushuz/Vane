use std::collections::HashSet;
use std::fmt;
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainRuleError {
    Empty,
    NonAscii,
    UrlOrPath,
    IpLiteral,
    InvalidWildcard,
    InvalidLength,
    InvalidLabel,
}

impl fmt::Display for DomainRuleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Empty => "domain rule is empty",
            Self::NonAscii => "domain rule must use ASCII or explicit punycode",
            Self::UrlOrPath => "domain rule must be a hostname, not a URL, path, or host:port",
            Self::IpLiteral => "IP addresses are not valid domain rules",
            Self::InvalidWildcard => "wildcard is only allowed once as the '*.' prefix",
            Self::InvalidLength => "domain rule or one of its labels has an invalid length",
            Self::InvalidLabel => "domain rule contains an invalid label",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for DomainRuleError {}

fn canonicalize_domain_rule(input: &str, allow_wildcard: bool) -> Result<String, DomainRuleError> {
    let normalized = input.trim().trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(DomainRuleError::Empty);
    }
    if !normalized.is_ascii() {
        return Err(DomainRuleError::NonAscii);
    }
    if normalized.contains("://")
        || normalized.contains('/')
        || normalized.contains('\\')
        || normalized.contains(':')
        || normalized.contains('[')
        || normalized.contains(']')
    {
        return Err(DomainRuleError::UrlOrPath);
    }

    let hostname = if let Some(hostname) = normalized.strip_prefix("*.") {
        if !allow_wildcard || hostname.contains('*') {
            return Err(DomainRuleError::InvalidWildcard);
        }
        hostname
    } else {
        if normalized.contains('*') {
            return Err(DomainRuleError::InvalidWildcard);
        }
        normalized.as_str()
    };

    if hostname.parse::<IpAddr>().is_ok() {
        return Err(DomainRuleError::IpLiteral);
    }
    if hostname.len() > 253 || !hostname.contains('.') {
        return Err(DomainRuleError::InvalidLength);
    }
    for label in hostname.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(DomainRuleError::InvalidLength);
        }
        let bytes = label.as_bytes();
        if !bytes[0].is_ascii_alphanumeric()
            || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        {
            return Err(DomainRuleError::InvalidLabel);
        }
    }

    Ok(normalized)
}

pub fn canonicalize_domain_rules(inputs: &[String]) -> Result<Vec<String>, DomainRuleError> {
    let mut seen = HashSet::new();
    let mut rules = Vec::with_capacity(inputs.len());
    for input in inputs {
        let rule = canonicalize_domain_rule(input, true)?;
        if seen.insert(rule.clone()) {
            rules.push(rule);
        }
    }
    Ok(rules)
}

pub fn domain_rule_matches(rule: &str, host: &str) -> bool {
    let Ok(rule) = canonicalize_domain_rule(rule, true) else {
        return false;
    };
    let Ok(host) = canonicalize_domain_rule(host, false) else {
        return false;
    };

    if let Some(base) = rule.strip_prefix("*.") {
        host != base && host.ends_with(&format!(".{base}"))
    } else {
        host == rule || host.ends_with(&format!(".{rule}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{canonicalize_domain_rules, domain_rule_matches};

    #[test]
    fn canonicalizes_case_whitespace_and_final_dot() {
        let rules =
            canonicalize_domain_rules(&["  Roblox.COM.  ".to_string(), "roblox.com".to_string()])
                .expect("valid domains");
        assert_eq!(rules, vec!["roblox.com"]);
    }

    #[test]
    fn rejects_urls_ports_ips_and_unicode() {
        for invalid in [
            "https://roblox.com/path",
            "roblox.com:443",
            "127.0.0.1",
            "[2001:db8::1]",
            "b\u{00fc}cher.de",
        ] {
            assert!(
                canonicalize_domain_rules(&[invalid.to_string()]).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn accepts_explicit_punycode_and_valid_wildcard() {
        let rules = canonicalize_domain_rules(&[
            "xn--bcher-kva.de".to_string(),
            "*.ROBLOX.com.".to_string(),
        ])
        .expect("valid rules");
        assert_eq!(rules, vec!["xn--bcher-kva.de", "*.roblox.com"]);
    }

    #[test]
    fn exact_rule_matches_apex_and_boundary_safe_subdomains() {
        assert!(domain_rule_matches("roblox.com", "roblox.com"));
        assert!(domain_rule_matches("roblox.com", "www.roblox.com"));
        assert!(domain_rule_matches("roblox.com", "api.roblox.com."));
        assert!(!domain_rule_matches("roblox.com", "evilroblox.com"));
        assert!(!domain_rule_matches("roblox.com", "roblox.com.evil.test"));
    }

    #[test]
    fn wildcard_matches_subdomains_but_not_apex() {
        assert!(domain_rule_matches("*.roblox.com", "www.roblox.com"));
        assert!(domain_rule_matches("*.roblox.com", "api.cdn.roblox.com"));
        assert!(!domain_rule_matches("*.roblox.com", "roblox.com"));
    }

    #[test]
    fn does_not_expand_hidden_aliases() {
        let rules = canonicalize_domain_rules(&["roblox.com".to_string()]).expect("valid domain");
        assert_eq!(rules, vec!["roblox.com"]);
        assert!(!rules.iter().any(|rule| rule.contains("rbxcdn")));
    }
}
