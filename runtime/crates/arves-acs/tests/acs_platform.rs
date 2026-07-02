//! ACS Platform — executable golden vectors for ACS-003/004/005 (Vertical Proof
//! bricks 3-5). Completes the Rust reference for all five ARVES Core Standards:
//! each standard's published vector is reproduced byte-for-byte here, so a second
//! (e.g. Go) implementation has an executable target, not just prose.

use arves_acs::cbor::{encode, Value::*};
use arves_acs::{content_id, domain, hex};

/// The ACS-002-CS-1 V1 fact body (reused as the ACS-003 envelope payload).
fn fact_v1() -> Vec<u8> {
    encode(&Map(vec![
        (Text("type".into()), Text("uci.fact".into())),
        (Text("claim".into()), Text("sky-is-blue".into())),
        (Text("confidence".into()), Float(0.5)),
        (Text("observed_at".into()), Int(1730000000000000000)),
    ]))
}

/// ACS-003-CS-1 — reconstruct the canonical envelope and reproduce its ContentId
/// (domain 0x06). payload_cid is the ACS-001 ContentId of the dCBOR payload body.
#[test]
fn acs_003_cs_1_envelope() {
    let payload_cid = content_id(domain::COMMIT_CONTENT, &fact_v1());
    // sanity: the payload address is the published ACS-002-CS-1 V1 CID
    assert_eq!(
        hex(&payload_cid),
        "12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e"
    );
    let envelope = Map(vec![
        (Text("ser_version".into()), Text("ACS-002/1".into())),
        (Text("event_id".into()), Text("urn:arves:evt:01J8ZK9M4Q2N7C3F".into())),
        (Text("event_type".into()), Text("information.fact.committed".into())),
        (Text("tenant_id".into()), Text("acme".into())),
        (Text("workspace_id".into()), Text("research".into())),
        (Text("correlation_id".into()), Text("urn:arves:corr:c1".into())),
        (Text("causation_id".into()), Null),
        (Text("source".into()), Text("urn:arves:svc:information-core".into())),
        (Text("occurred_at".into()), Int(1730000000000000000)),
        (Text("schema_version".into()), Int(1)),
        (Text("payload_domain".into()), Int(1)),
        (Text("payload_cid".into()), Bytes(payload_cid)),
    ]);
    assert_eq!(
        hex(&content_id(domain::CANONICAL_ENVELOPE, &encode(&envelope))),
        "1220fc0ef055e4d39de1c3ab7d2597361d24f7a8b6a1a0609a91b872b85ae4896f93"
    );
}

/// ACS-004-CS-1 — reconstruct the uci.fact@1.0 instance; reproduce its ContentId
/// (domain 0x01, a committed fact).
#[test]
fn acs_004_cs_1_instance() {
    let inst = Map(vec![
        (Text("urn".into()), Text("urn:arves:uci.core:fact@1.0:f-1730000000".into())),
        (Text("tenant".into()), Text("acme".into())),
        (Text("workspace".into()), Text("research".into())),
        (Text("origin".into()), Text("observed".into())),
        (Text("source".into()), Text("sensor-array-7".into())),
        (Text("confidence".into()), Float(0.98)),
        (Text("valid_from".into()), Int(1730000000000000000)),
        (Text("recorded_at".into()), Int(1730000000500000000)),
        (Text("claim".into()), Text("sky-is-blue".into())),
        (Text("observed_at".into()), Int(1730000000000000000)),
        (Text("evidence".into()), Array(vec![Text("urn:arves:uci.core:evidence@1.0:e-42".into())])),
    ]);
    assert_eq!(
        hex(&content_id(domain::COMMIT_CONTENT, &encode(&inst))),
        "12206fce3fbcfce59860140942f7d1ca9e7b274fd936f2237011ff144552f091f07e"
    );
}

/// ACS-005-CS-1 — glossary term-set + requirement clause + term-name list, under
/// the corrected tags 0x08/0x09 (the collision fix). Bodies are raw (not dCBOR).
#[test]
fn acs_005_cs_1_glossary_and_requirement() {
    // V1: Term IDs GL-001..GL-014, LF-joined, no trailing LF, tag 0x08.
    let term_ids: Vec<String> = (1..=14).map(|i| format!("GL-{i:03}")).collect();
    assert_eq!(
        hex(&content_id(domain::GLOSSARY_TERM_SET, term_ids.join("\n").as_bytes())),
        "1220ced393907a4d27eb54ac12acea65e29c7168c2991b3ca9df4b39765e870d2074"
    );

    // V2: requirement clause text, tag 0x09.
    let req = "ORCH-001-R1: The Control Plane MUST NOT own cognitive truth; only the Kernel MAY own cognitive truth.";
    assert_eq!(
        hex(&content_id(domain::REQUIREMENT_CLAUSE, req.as_bytes())),
        "12207f1a532d2be5061377d6664be065bbb45b6e61741bb70c1195454054e1cf0475"
    );

    // V3: term-name list (the §9.1 enforced set), sorted, LF-joined, tag 0x08.
    let names = [
        "Capability", "Cognitive Entity", "Cognitive Truth", "Commit", "Conformance",
        "Content Address", "Control Plane", "Data Plane", "Decision Trace", "Engine",
        "Kernel", "Replay", "Shard", "Tenant",
    ];
    assert_eq!(
        hex(&content_id(domain::GLOSSARY_TERM_SET, names.join("\n").as_bytes())),
        "12200c1c893c613d0f12976697084f05a76243589ed55a3d2cdae9dbce9d69df4751"
    );
}
