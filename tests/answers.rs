use lab_name_server::{Config, ZoneStore, parse_name};
use hickory_proto::op::{Message, Query, ResponseCode};
use hickory_proto::rr::{Record, RData, RecordType};

fn store_from_yaml(yaml: &str) -> ZoneStore {
    let cfg: Config = serde_yaml::from_str(yaml).expect("config yaml parse");
    ZoneStore::from_config(&cfg).expect("build store")
}

fn make_query_msg(name: &str, rtype: RecordType) -> Message {
    let qname = parse_name(name);
    let mut q = Query::new();
    q.set_name(qname);
    q.set_query_type(rtype);
    let mut m = Message::new();
    m.add_query(q);
    m
}

fn has_record_type(records: &[Record<RData>], rt: RecordType) -> bool {
    records.iter().any(|r| r.record_type() == rt)
}

#[test]
fn exact_a_answer() {
    let yaml = r#"
default_ttl: 300
zones:
  - origin: example.test.
    ttl: 300
    soa: { mname: ns1.example.test., rname: hostmaster.example.test., serial: 1, refresh: 3600, retry: 600, expire: 86400, minimum: 300 }
    ns: [ ns1.example.test. ]
    records:
      - { name: "@", type: A, value: 10.0.0.1 }
"#;
    let store = store_from_yaml(yaml);
    let req = make_query_msg("example.test.", RecordType::A);
    let resp = store.answer(&req).expect("should answer");
    assert!(resp.header().authoritative());
    assert_eq!(resp.response_code(), ResponseCode::NoError);
    assert!(has_record_type(resp.answers(), RecordType::A));
    assert!(has_record_type(resp.name_servers(), RecordType::SOA));
    assert!(has_record_type(resp.name_servers(), RecordType::NS));
}

#[test]
fn wildcard_a_answer() {
    let yaml = r#"
default_ttl: 300
zones:
  - origin: example.test.
    ttl: 300
    soa: { mname: ns1.example.test., rname: hostmaster.example.test., serial: 1, refresh: 3600, retry: 600, expire: 86400, minimum: 300 }
    ns: [ ns1.example.test. ]
    records:
      - { name: "*.wild", type: A, value: 10.0.0.99 }
"#;
    let store = store_from_yaml(yaml);
    let req = make_query_msg("foo.wild.example.test.", RecordType::A);
    let resp = store.answer(&req).expect("should answer via wildcard");
    assert!(resp.header().authoritative());
    assert_eq!(resp.response_code(), ResponseCode::NoError);
    assert!(has_record_type(resp.answers(), RecordType::A));
}

#[test]
fn cname_one_hop_answer() {
    let yaml = r#"
default_ttl: 300
zones:
  - origin: example.test.
    ttl: 300
    soa: { mname: ns1.example.test., rname: hostmaster.example.test., serial: 1, refresh: 3600, retry: 600, expire: 86400, minimum: 300 }
    ns: [ ns1.example.test. ]
    records:
      - { name: "@", type: A, value: 10.0.0.1 }
      - { name: www, type: CNAME, value: "@" }
"#;
    let store = store_from_yaml(yaml);
    let req = make_query_msg("www.example.test.", RecordType::A);
    let resp = store.answer(&req).expect("should cname and answer A");
    assert!(resp.header().authoritative());
    assert_eq!(resp.response_code(), ResponseCode::NoError);
    assert!(has_record_type(resp.answers(), RecordType::CNAME));
    assert!(has_record_type(resp.answers(), RecordType::A));
}

#[test]
fn nodata_still_soans() {
    let yaml = r#"
default_ttl: 300
zones:
  - origin: example.test.
    ttl: 300
    soa: { mname: ns1.example.test., rname: hostmaster.example.test., serial: 1, refresh: 3600, retry: 600, expire: 86400, minimum: 300 }
    ns: [ ns1.example.test. ]
    records:
      - { name: onlytxt, type: TXT, value: "hello" }
"#;
    let store = store_from_yaml(yaml);
    let req = make_query_msg("onlytxt.example.test.", RecordType::A);
    let resp = store.answer(&req).expect("should return NODATA NoError");
    assert!(resp.header().authoritative());
    assert_eq!(resp.response_code(), ResponseCode::NoError);
    assert_eq!(resp.answer_count(), 0);
    assert!(has_record_type(resp.name_servers(), RecordType::SOA));
    assert!(has_record_type(resp.name_servers(), RecordType::NS));
}

#[test]
fn txt_resolution() {
    let yaml = r#"zones: [{ origin: example.test., ttl: 300, soa: { mname: ns1.example.test., rname: hostmaster.example.test., serial: 1, refresh: 3600, retry: 600, expire: 86400, minimum: 300 }, ns: [ ns1.example.test. ], records: [ { name: text, type: TXT, value: "hello" } ] }]"#;
    let store = store_from_yaml(yaml);
    let req = make_query_msg("text.example.test.", RecordType::TXT);
    let resp = store.answer(&req).expect("should answer TXT");
    assert!(resp.header().authoritative());
    assert_eq!(resp.response_code(), ResponseCode::NoError);
    assert!(has_record_type(resp.answers(), RecordType::TXT));
}

#[test]
fn mx_resolution() {
    let yaml = r#"zones: [{ origin: example.test., ttl: 300, soa: { mname: ns1.example.test., rname: hostmaster.example.test., serial: 1, refresh: 3600, retry: 600, expire: 86400, minimum: 300 }, ns: [ ns1.example.test. ], records: [ { name: mail, type: MX, preference: 10, value: mailhost.example.test. } ] }]"#;
    let store = store_from_yaml(yaml);
    let req = make_query_msg("mail.example.test.", RecordType::MX);
    let resp = store.answer(&req).expect("should answer MX");
    assert!(resp.header().authoritative());
    assert_eq!(resp.response_code(), ResponseCode::NoError);
    assert!(has_record_type(resp.answers(), RecordType::MX));
}

#[test]
fn wildcard_cname_only_cname() {
    let yaml = r#"zones: [{ origin: example.test., ttl: 300, soa: { mname: ns1.example.test., rname: hostmaster.example.test., serial: 1, refresh: 3600, retry: 600, expire: 86400, minimum: 300 }, ns: [ ns1.example.test. ], records: [ { name: "@", type: A, value: 10.0.0.1 }, { name: "*.app", type: CNAME, value: "@" } ] }]"#;
    let store = store_from_yaml(yaml);
    let req = make_query_msg("foo.app.example.test.", RecordType::A);
    let resp = store.answer(&req).expect("should answer with CNAME from wildcard");
    assert!(resp.header().authoritative());
    assert_eq!(resp.response_code(), ResponseCode::NoError);
    assert!(has_record_type(resp.answers(), RecordType::CNAME));
    assert!(!has_record_type(resp.answers(), RecordType::A));
}
