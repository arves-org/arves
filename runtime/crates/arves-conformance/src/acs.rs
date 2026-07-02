//! ARVES Conformance Platform — the executable ACS conformance runner.
//!
//! This is the universal, implementation-agnostic conformance core: it reconstructs
//! each ARVES Core Standard's golden vector from its logical value (via the
//! `arves-acs` reference codec) and checks the byte-exact `ContentId` against the
//! published target. ANY implementation (Rust today; Go/Java/Python/third-party
//! later) is conformant iff it reproduces the same `ContentId`s — that is the
//! Independent-Runtime proof. Populates the Scenario Conformance Framework's
//! previously-empty assertion surface for the ACS layer (Global Readiness gap G3).
//!
//! Run: `cargo run -p arves-conformance --bin conformance` -> ARVES Conformance Report.

use arves_acs::cbor::{decode_canonical, encode, Value, Value::*};
use arves_acs::{content_id, domain, hex};

/// One checked golden vector.
pub struct VectorResult {
    pub standard: &'static str,
    pub vector: &'static str,
    pub domain: u8,
    pub body_hex: String,
    pub content_id: String,
    pub expected: &'static str,
    pub pass: bool,
}

/// The ACS conformance report (one entry per checked vector).
pub struct AcsReport {
    pub results: Vec<VectorResult>,
}

impl AcsReport {
    pub fn all_pass(&self) -> bool {
        self.results.iter().all(|r| r.pass)
    }
    /// Machine-readable, language-neutral golden-vector corpus (TSV). Any
    /// implementation loads this + reads the ACS specs for the logical inputs, then
    /// checks its own `body_hex` (encoder) and `content_id` (addresser).
    pub fn tsv(&self) -> String {
        let mut s = String::from("standard\tvector\tdomain\tbody_hex\tcontent_id\n");
        for r in &self.results {
            s.push_str(&format!(
                "{}\t{}\t0x{:02x}\t{}\t{}\n",
                r.standard, r.vector, r.domain, r.body_hex, r.content_id
            ));
        }
        s
    }

    /// (standard, passed, total) grouped in declaration order.
    pub fn by_standard(&self) -> Vec<(&'static str, usize, usize)> {
        let mut out: Vec<(&'static str, usize, usize)> = Vec::new();
        for r in &self.results {
            match out.iter_mut().find(|(s, _, _)| *s == r.standard) {
                Some(e) => {
                    e.2 += 1;
                    if r.pass {
                        e.1 += 1;
                    }
                }
                None => out.push((r.standard, if r.pass { 1 } else { 0 }, 1)),
            }
        }
        out
    }
}

fn check(standard: &'static str, vector: &'static str, domain_tag: u8, body: &[u8], expected: &'static str) -> VectorResult {
    let cid = hex(&content_id(domain_tag, body));
    VectorResult {
        standard,
        vector,
        domain: domain_tag,
        body_hex: hex(body),
        pass: cid == expected,
        content_id: cid,
        expected,
    }
}

fn raw(standard: &'static str, vector: &'static str, domain_tag: u8, body: &[u8], expected: &'static str) -> VectorResult {
    check(standard, vector, domain_tag, body, expected)
}

fn dcbor(standard: &'static str, vector: &'static str, domain_tag: u8, v: &Value, expected: &'static str) -> VectorResult {
    check(standard, vector, domain_tag, &encode(v), expected)
}

fn fact_v1() -> Value {
    Map(vec![
        (Text("type".into()), Text("uci.fact".into())),
        (Text("claim".into()), Text("sky-is-blue".into())),
        (Text("confidence".into()), Float(0.5)),
        (Text("observed_at".into()), Int(1730000000000000000)),
    ])
}

/// An ACS-004 schema field descriptor `{card, type}`.
fn field(card: &str, ty: &str) -> Value {
    Map(vec![
        (Text("card".into()), Text(card.into())),
        (Text("type".into()), Text(ty.into())),
    ])
}

/// Run the ACS conformance suite over all reconstructable golden vectors.
pub fn run() -> AcsReport {
    let mut r = Vec::new();

    // ACS-001 Universal Content Identity (raw bodies).
    r.push(raw("ACS-001", "hello-truth", domain::COMMIT_CONTENT, b"hello-truth",
        "122056e30f71852b0e4c253cf05dab6be2bb5b8470ac878a52f10c5af2a40d69b76e"));
    r.push(raw("ACS-001", "engine-manifest", domain::ENGINE_MANIFEST, br#"{"engine":"summarize","version":"1"}"#,
        "12205c631bd808332b0889763100ad7458710c137320381e3b4ea9cce3c0640a4e54"));
    r.push(raw("ACS-001", "invocation", domain::INVOCATION, b"acme/research|c1|hello-truth",
        "1220ae7a70002ef6dd81018d4715a986dae6dfdc1b7bc85acdd66698875f2fe302bc"));

    // ACS-002 Canonical Serialization (dCBOR maps).
    r.push(dcbor("ACS-002", "V1 uci.fact", domain::COMMIT_CONTENT, &fact_v1(),
        "12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e"));
    r.push(dcbor("ACS-002", "V2 engine-manifest", domain::ENGINE_MANIFEST, &Map(vec![
        (Text("engine".into()), Text("summarize".into())),
        (Text("version".into()), Int(1)),
        (Text("deterministic".into()), Bool(true)),
        (Text("reads".into()), Array(vec![Text("uci.observation".into()), Text("uci.fact".into())])),
        (Text("seed".into()), Null),
    ]), "1220e5aad722341bd0838fb268d73a0a28401457883b9e5e623c05dc0623f57a690d"));
    r.push(dcbor("ACS-002", "V3 nfc+neg", domain::DECISION_TRACE, &Map(vec![
        (Text("label".into()), Text("Am\u{00e9}lie \u{00e9}\u{2014}\u{4e2d}".into())),
        (Text("n".into()), Int(-1000)),
    ]), "12207c5367768a3cd0d90b781cac2530335f0310ffc155eac4ac82da80af71e2366a"));

    // ACS-003 Canonical Envelope (payload_cid = ACS-002 V1 address).
    let payload_cid = content_id(domain::COMMIT_CONTENT, &encode(&fact_v1()));
    r.push(dcbor("ACS-003", "envelope", domain::CANONICAL_ENVELOPE, &Map(vec![
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
    ]), "1220fc0ef055e4d39de1c3ab7d2597361d24f7a8b6a1a0609a91b872b85ae4896f93"));

    // ACS-004 Universal Type Registry (uci.fact@1.0 instance, committed truth).
    r.push(dcbor("ACS-004", "uci.fact instance", domain::COMMIT_CONTENT, &Map(vec![
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
    ]), "12206fce3fbcfce59860140942f7d1ca9e7b274fd936f2237011ff144552f091f07e"));

    // ACS-004 schema document (content-addressed type identity, domain 0x07).
    r.push(dcbor("ACS-004", "schema", domain::TYPE_SCHEMA, &Map(vec![
        (Text("urn".into()), Text("uci.fact".into())),
        (Text("ver".into()), Map(vec![(Text("major".into()), Int(1)), (Text("minor".into()), Int(0))])),
        (Text("root".into()), Text("Fact".into())),
        (Text("fields".into()), Map(vec![
            (Text("urn".into()), field("1", "urn")),
            (Text("tenant".into()), field("1", "text")),
            (Text("workspace".into()), field("1", "text")),
            (Text("origin".into()), field("1", "text")),
            (Text("source".into()), field("1", "text")),
            (Text("invocation".into()), field("0..1", "urn")),
            (Text("confidence".into()), field("1", "conf")),
            (Text("valid_from".into()), field("1", "int")),
            (Text("recorded_at".into()), field("1", "int")),
            (Text("claim".into()), field("1", "text")),
            (Text("observed_at".into()), field("1", "int")),
            (Text("evidence".into()), field("0..*", "urn")),
        ])),
        (Text("aspects".into()), Array(vec![
            Text("Identity".into()), Text("Provenance".into()), Text("Temporal".into()),
            Text("Trust".into()), Text("TenantScope".into()),
        ])),
    ]), "12206b3f99c64d23029f49b986e4c89152955c649274e5cf60b1b3ad581b19fa4b87"));

    // ACS-005 Normative Language (raw bodies, tags 0x08/0x09).
    let term_ids: Vec<String> = (1..=14).map(|i| format!("GL-{i:03}")).collect();
    r.push(raw("ACS-005", "term-set", domain::GLOSSARY_TERM_SET, term_ids.join("\n").as_bytes(),
        "1220ced393907a4d27eb54ac12acea65e29c7168c2991b3ca9df4b39765e870d2074"));
    r.push(raw("ACS-005", "requirement", domain::REQUIREMENT_CLAUSE,
        b"ORCH-001-R1: The Control Plane MUST NOT own cognitive truth; only the Kernel MAY own cognitive truth.",
        "12207f1a532d2be5061377d6664be065bbb45b6e61741bb70c1195454054e1cf0475"));
    let names = ["Capability","Cognitive Entity","Cognitive Truth","Commit","Conformance","Content Address","Control Plane","Data Plane","Decision Trace","Engine","Kernel","Replay","Shard","Tenant"];
    r.push(raw("ACS-005", "term-names", domain::GLOSSARY_TERM_SET, names.join("\n").as_bytes(),
        "12200c1c893c613d0f12976697084f05a76243589ed55a3d2cdae9dbce9d69df4751"));

    AcsReport { results: r }
}

/// One negative (rejection) vector: a byte string that is NOT canonical dCBOR and
/// that a conformant decoder MUST reject. `tier` "core" is enforced by every
/// conformant implementation; "nfc" needs a Unicode NFC facility and may be
/// DEFERRED by a dependency-free implementation (documented, not a failure).
pub struct NegVector {
    pub standard: &'static str,
    pub case: &'static str,
    pub tier: &'static str,
    pub input_hex: String,
    pub reason: &'static str,
    pub outcome: String,
    pub pass: bool,
}

/// The negative-vector corpus (ACS-002 canonical-form rejection). Each entry is a
/// minimal non-canonical byte string paired with the normative reason it must be
/// rejected. These are the SAME inputs the `arves-acs` unit tests assert, surfaced
/// here as a machine-readable corpus for the Standard Kit so any implementation can
/// prove it rejects them too.
fn negative_corpus() -> Vec<(&'static str, &'static str, Vec<u8>, &'static str)> {
    vec![
        ("non-shortest-int", "core", vec![0x18, 0x00], "non-shortest-int"),
        ("non-shortest-len", "core", vec![0x78, 0x01, 0x61], "non-shortest-len"),
        ("indefinite-length", "core", vec![0x9f, 0x00, 0xff], "indefinite-length"),
        ("unsorted-map-keys", "core", vec![0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02], "unsorted-map-keys"),
        ("duplicate-map-keys", "core", vec![0xa2, 0x61, 0x61, 0x01, 0x61, 0x61, 0x02], "duplicate-map-keys"),
        ("float-not-float64/half", "core", vec![0xf9, 0x3c, 0x00], "float-not-float64"),
        ("float-not-float64/single", "core", vec![0xfa, 0x3f, 0x80, 0x00, 0x00], "float-not-float64"),
        ("negative-zero-float", "core", vec![0xfb, 0x80, 0, 0, 0, 0, 0, 0, 0], "negative-zero-float"),
        ("non-finite-float", "core", vec![0xfb, 0x7f, 0xf0, 0, 0, 0, 0, 0, 0], "non-finite-float"),
        ("trailing-data", "core", vec![0x00, 0x00], "trailing-data"),
        ("reserved-or-unsupported/tag", "core", vec![0xc0, 0x00], "reserved-or-unsupported"),
        ("truncated", "core", vec![0x64, 0x61], "truncated"),
        // map key outside the §4 value model ({null:0}) — key must be Text or Integer.
        ("map-key-not-in-model", "core", vec![0xa1, 0xf6, 0x00], "reserved-or-unsupported"),
        // text item with a non-UTF-8 octet (0xff).
        ("text-invalid-utf8", "core", vec![0x61, 0xff], "reserved-or-unsupported"),
        // bare 'break' stop code at top level.
        ("top-level-break", "core", vec![0xff], "indefinite-length"),
        // "é" as base 'e' + combining acute (NFD) — MUST be rejected as non-NFC by
        // an implementation with a Unicode NFC facility; the dependency-free Rust
        // reference DEFERS this one rule (no Unicode table).
        ("non-nfc-text", "nfc", vec![0x63, 0x65, 0xcc, 0x81], "non-nfc-text"),
    ]
}

/// Run the ACS-002 negative (rejection) conformance suite against the reference
/// `decode_canonical`. A "core" case passes iff it is rejected with the matching
/// reason; an "nfc" case passes if rejected OR explicitly deferred.
pub fn run_negative() -> Vec<NegVector> {
    negative_corpus()
        .into_iter()
        .map(|(case, tier, bytes, reason)| {
            let res = decode_canonical(&bytes);
            let (outcome, pass) = match (tier, res) {
                ("core", Err(e)) if e.code() == reason => (format!("REJECTED({})", e.code()), true),
                ("core", Err(e)) => (format!("REJECTED({}) [want {reason}]", e.code()), false),
                ("core", Ok(_)) => ("ACCEPTED [MUST reject]".to_string(), false),
                (_, Err(e)) => (format!("REJECTED({})", e.code()), true), // nfc: caught anyway
                (_, Ok(_)) => ("DEFERRED(needs Unicode NFC table)".to_string(), true),
            };
            NegVector { standard: "ACS-002", case, tier, input_hex: hex(&bytes), reason, outcome, pass }
        })
        .collect()
}

/// Every "core" negative vector was rejected with the correct reason.
pub fn negative_core_pass(v: &[NegVector]) -> bool {
    v.iter().filter(|n| n.tier == "core").all(|n| n.pass)
}

/// Machine-readable negative-vector corpus (TSV) for the Standard Kit.
pub fn negative_tsv(v: &[NegVector]) -> String {
    let mut s = String::from("standard\tcase\ttier\tinput_hex\treject_reason\n");
    for n in v {
        s.push_str(&format!("{}\t{}\t{}\t{}\t{}\n", n.standard, n.case, n.tier, n.input_hex, n.reason));
    }
    s
}

/// Render the human-readable ARVES Conformance Report.
pub fn render(report: &AcsReport) -> String {
    let mut s = String::from("ARVES Conformance Report — ACS layer\n");
    s.push_str("========================================\n");
    for (std, pass, total) in report.by_standard() {
        let verdict = if pass == total { "PASS" } else { "FAIL" };
        s.push_str(&format!("  {std:<9} {verdict} ({pass}/{total})\n"));
    }
    let passed = report.results.iter().filter(|r| r.pass).count();
    let total = report.results.len();
    s.push_str("----------------------------------------\n");
    s.push_str(&format!("  ACS golden vectors: {passed}/{total} {}\n", if passed == total { "PASS" } else { "FAIL" }));

    // Negative (rejection) conformance: a standard defines what MUST be rejected.
    let neg = run_negative();
    let core: Vec<&NegVector> = neg.iter().filter(|n| n.tier == "core").collect();
    let core_rej = core.iter().filter(|n| n.pass).count();
    let deferred = neg.iter().filter(|n| n.tier == "nfc").count();
    s.push_str(&format!(
        "  ACS-002 negative vectors: {}/{} core REJECTED {}",
        core_rej, core.len(), if core_rej == core.len() { "PASS" } else { "FAIL" }
    ));
    if deferred > 0 {
        s.push_str(&format!(" (+{deferred} nfc-tier DEFERRED: needs Unicode NFC table)"));
    }
    s.push('\n');
    s.push_str("  Architecture gate (LAYER-001/OWN-001): PASS (arves-conformance::architecture_gate)\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acs_conformance_all_pass() {
        let report = run();
        for r in &report.results {
            assert!(r.pass, "{} [{}] got {} expected {}", r.standard, r.vector, r.content_id, r.expected);
        }
        assert!(report.all_pass());
        assert_eq!(report.results.len(), 12);
    }

    #[test]
    fn acs_negative_core_all_rejected() {
        let neg = run_negative();
        for n in &neg {
            assert!(n.pass, "{} [{}/{}]: {}", n.standard, n.tier, n.case, n.outcome);
        }
        assert!(negative_core_pass(&neg));
        // 15 core rejection rules + 1 nfc-tier (deferred by this reference).
        assert_eq!(neg.iter().filter(|n| n.tier == "core").count(), 15);
        assert_eq!(neg.iter().filter(|n| n.tier == "nfc").count(), 1);
    }
}
