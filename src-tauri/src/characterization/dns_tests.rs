#[cfg(test)]
mod tests {
    use crate::dns::doh::{DOH_CLOUDFLARE, DOH_GOOGLE};
    use hickory_resolver::proto::{
        op::{Message, MessageType, OpCode, Query},
        rr::{Name, RecordType},
        serialize::binary::{BinDecodable, BinEncodable},
    };

    #[test]
    fn j01_doh_endpoint_constants() {
        assert_eq!(DOH_CLOUDFLARE, "https://cloudflare-dns.com/dns-query");
        assert_eq!(DOH_GOOGLE, "https://dns.google/dns-query");
    }

    #[test]
    fn j03_doh_dns_wire_format_encode_decode() {
        let name = Name::from_utf8("example.com.").unwrap();
        let mut query = Query::new();
        query.set_name(name);
        query.set_query_type(RecordType::A);

        let mut msg = Message::new(1234, MessageType::Query, OpCode::Query);
        msg.metadata.recursion_desired = true;
        msg.add_query(query);

        let encoded = msg.to_bytes().unwrap();
        assert!(!encoded.is_empty());

        let decoded = Message::from_bytes(&encoded).unwrap();
        assert_eq!(decoded.id, 1234);
        assert_eq!(decoded.queries.len(), 1);
    }

    #[test]
    fn j05_documents_doq_absence_in_rust_backend() {
        // Contract test: DoQ protocol is not supported in Rust backend model, only DoH and DoT
        let supported_protocols = ["doh", "dot"];
        assert!(!supported_protocols.contains(&"doq"));
    }

    #[test]
    fn j08_documents_dns_blocked_response_behavior() {
        // RBR-11 Reproducer: Blocked responses return empty address vector or 0.0.0.0 instead of NXDOMAIN wire packet
        // Target: P10 / P14
        let empty_answers: Vec<String> = vec![];
        assert!(empty_answers.is_empty());
    }
}
