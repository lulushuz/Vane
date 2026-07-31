#[cfg(test)]
mod tests {
    use crate::config::domain::{canonicalize_domain_rules, domain_rule_matches, DomainRuleError};
    use proptest::prelude::*;

    #[test]
    fn a01_simple_domain() {
        let rules = canonicalize_domain_rules(&["example.com".to_string()]).unwrap();
        assert_eq!(rules, vec!["example.com"]);
    }

    #[test]
    fn a02_uppercase_domain() {
        let rules = canonicalize_domain_rules(&["EXAMPLE.COM".to_string()]).unwrap();
        assert_eq!(rules, vec!["example.com"]);
    }

    #[test]
    fn a03_trailing_dot() {
        let rules = canonicalize_domain_rules(&["example.com.".to_string()]).unwrap();
        assert_eq!(rules, vec!["example.com"]);
    }

    #[test]
    fn a04_subdomain() {
        let rules = canonicalize_domain_rules(&["sub.example.com".to_string()]).unwrap();
        assert_eq!(rules, vec!["sub.example.com"]);
    }

    #[test]
    fn a05_url_formats_rejected() {
        let urls = [
            "https://example.com",
            "https://example.com/path",
            "http://example.com",
        ];
        for url in urls {
            let res = canonicalize_domain_rules(&[url.to_string()]);
            assert!(
                matches!(res, Err(DomainRuleError::UrlOrPath)),
                "Failed for {}",
                url
            );
        }
    }

    #[test]
    fn a06_port_in_domain_rejected() {
        let res = canonicalize_domain_rules(&["example.com:443".to_string()]);
        assert!(matches!(res, Err(DomainRuleError::UrlOrPath)));
    }

    #[test]
    fn a07_wildcard_rejected() {
        let res = canonicalize_domain_rules(&["*.example.com".to_string()]);
        assert!(matches!(res, Err(DomainRuleError::InvalidWildcard)));
    }

    #[test]
    fn a08_unicode_and_punycode() {
        // Non-ASCII rejected
        assert!(canonicalize_domain_rules(&["ünalakademi.com.tr".to_string()]).is_err());
        // Explicit Punycode accepted
        let res = canonicalize_domain_rules(&["xn--nalakademi-25a.com.tr".to_string()]).unwrap();
        assert_eq!(res, vec!["xn--nalakademi-25a.com.tr"]);
    }

    #[test]
    fn a09_whitespace_and_control_chars() {
        // Whitespace trimmed around domain
        let res = canonicalize_domain_rules(&["  example.com  ".to_string()]).unwrap();
        assert_eq!(res, vec!["example.com"]);

        // Space inside domain rejected
        assert!(canonicalize_domain_rules(&["example .com".to_string()]).is_err());

        // Newline inside input rejected
        assert!(canonicalize_domain_rules(&["example.com\ninvalid.com".to_string()]).is_err());
    }

    #[test]
    fn a10_duplicate_deduplication() {
        let inputs = vec![
            "example.com".to_string(),
            "EXAMPLE.COM".to_string(),
            "example.com.".to_string(),
            "  example.com  ".to_string(),
        ];
        let res = canonicalize_domain_rules(&inputs).unwrap();
        assert_eq!(res, vec!["example.com"]);
    }

    #[test]
    fn a11_large_domain_list_handling() {
        let large_list: Vec<String> = (0..10_000).map(|i| format!("sub{i}.example.com")).collect();
        let res = canonicalize_domain_rules(&large_list).unwrap();
        assert_eq!(res.len(), 10_000);
    }

    #[test]
    fn a12_domain_matching_semantics() {
        assert!(domain_rule_matches("example.com", "example.com"));
        assert!(domain_rule_matches("example.com", "sub.example.com"));
        assert!(!domain_rule_matches("example.com", "notexample.com"));
    }

    proptest! {
        #[test]
        fn a13_proptest_canonicalization_idempotent_and_clean(s in "\\PC*") {
            if let Ok(rules) = canonicalize_domain_rules(std::slice::from_ref(&s)) {
                for rule in &rules {
                    // Output must not contain newlines or control chars
                    prop_assert!(!rule.contains('\n'));
                    prop_assert!(!rule.contains('\r'));
                    prop_assert!(!rule.contains('\0'));
                    prop_assert!(!rule.is_empty());

                    // Idempotent: re-canonicalizing output yields exact same output
                    let recan = canonicalize_domain_rules(std::slice::from_ref(rule)).unwrap();
                    prop_assert_eq!(recan, vec![rule.clone()]);
                }
            }
        }
    }
}

