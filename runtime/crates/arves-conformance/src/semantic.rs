//! ACS-003 / ACS-004 / ACS-005 **semantic** validators (RCR-004).
//!
//! The reference runtime's NATIVE reject surface for the CCP-006
//! `envelope` / `instance` / `language` negative tiers. Each of those frozen negative
//! vectors decodes CLEANLY as canonical dCBOR (the ACS-002 byte layer accepts it) — the
//! defect is purely semantic, one layer up. Before RCR-004 the Rust reference had no such
//! validators and DEFERRED the tiers (like `nfc`); this module retires that deferral so a
//! G2 party can diff against a Rust reference for the full ACS surface, not just ACS-001/002.
//!
//! These mirror the from-scratch Python reference validators
//! (`verification/independent/python/acs003_envelope.py`, `acs004_instance.py`,
//! `acs005_checker.py`), RE-DERIVED here from the ACS-003/004/005 spec text, and — unlike
//! the Python validators, which emit descriptive prose — they emit the **registered CCP-006
//! reason code** directly (`standard/conformance/CONFORMANCE.md`, ACS-001 §4.1). The
//! `semantic_rejects_frozen_vectors` test proves every one of the 19 frozen semantic
//! vectors is rejected with its exact registered code.

use arves_acs::cbor::Value;

/// A semantic reject, carrying its registered CCP-006 reason code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticReject {
    MissingRequiredField,
    UnknownField,
    FieldTypeMismatch,
    ValueOutOfRange,
    MalformedContentId,
    EmptyShardScope,
    CardinalityViolation,
    ProvenanceInvariant,
    TermsNotSorted,
    DuplicateTerm,
    MalformedTermList,
}

impl SemanticReject {
    /// The stable registered reason code (CCP-006 / CONFORMANCE.md / ACS-001 §4.1).
    pub fn code(self) -> &'static str {
        use SemanticReject::*;
        match self {
            MissingRequiredField => "missing-required-field",
            UnknownField => "unknown-field",
            FieldTypeMismatch => "field-type-mismatch",
            ValueOutOfRange => "value-out-of-range",
            MalformedContentId => "malformed-content-id",
            EmptyShardScope => "empty-shard-scope",
            CardinalityViolation => "cardinality-violation",
            ProvenanceInvariant => "provenance-invariant",
            TermsNotSorted => "terms-not-sorted",
            DuplicateTerm => "duplicate-term",
            MalformedTermList => "malformed-term-list",
        }
    }
}

/// Look up a Text-keyed field in a decoded ACS-002 Map.
fn field<'a>(entries: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    entries.iter().find_map(|(k, v)| match k {
        Value::Text(s) if s == key => Some(v),
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// ACS-003 Canonical Envelope — §5 / §6.3 semantic reject rules.
// ---------------------------------------------------------------------------

const ENV_TEXT_FIELDS: &[&str] = &[
    "ser_version", "event_id", "event_type", "tenant_id", "workspace_id",
    "correlation_id", "source",
];
const ENV_INT_FIELDS: &[&str] = &["occurred_at", "schema_version", "payload_domain"];
/// The closed 12-field key set (§5: "No other keys SHALL appear").
const ENV_ALL_FIELDS: &[&str] = &[
    "ser_version", "event_id", "event_type", "tenant_id", "workspace_id",
    "correlation_id", "source", "occurred_at", "schema_version", "payload_domain",
    "payload_cid", "causation_id",
];

/// Validate a DECODED ACS-003 Canonical Envelope. `Ok(())` if every §6.3 clause holds,
/// else the registered reject code. Canonical-form (ACS-002) has already been enforced by
/// `decode_canonical`; this checks only the envelope-semantics layer.
pub fn validate_envelope(value: &Value) -> Result<(), SemanticReject> {
    use SemanticReject::*;
    // R1 §4/§6.3: the envelope value SHALL be an ACS-002 Map.
    let m = match value {
        Value::Map(m) => m,
        _ => return Err(FieldTypeMismatch),
    };
    // Collect Text keys; a non-Text (Integer) map key is not a §5 field -> unknown key.
    let mut present: Vec<&str> = Vec::new();
    for (k, _) in m {
        match k {
            Value::Text(s) => present.push(s.as_str()),
            _ => return Err(UnknownField),
        }
    }
    // R3 §5/§6.3: no unknown key (closed field set).
    for k in &present {
        if !ENV_ALL_FIELDS.contains(k) {
            return Err(UnknownField);
        }
    }
    // R2 §6.3: every REQUIRED field present (all 12 except the optional causation_id).
    for f in ENV_ALL_FIELDS {
        if *f != "causation_id" && !present.contains(f) {
            return Err(MissingRequiredField);
        }
    }
    // R4 §5/§5.1: Text-typed fields SHALL be Text.
    for f in ENV_TEXT_FIELDS {
        if !matches!(field(m, f), Some(Value::Text(_))) {
            return Err(FieldTypeMismatch);
        }
    }
    // R4 §5.1: Integer-typed fields SHALL be Integer (never Float / other).
    for f in ENV_INT_FIELDS {
        if !matches!(field(m, f), Some(Value::Int(_))) {
            return Err(FieldTypeMismatch);
        }
    }
    // R4 §5.7: causation_id (optional) SHALL be Text or Null when present.
    if present.contains(&"causation_id")
        && !matches!(field(m, "causation_id"), Some(Value::Text(_)) | Some(Value::Null))
    {
        return Err(FieldTypeMismatch);
    }
    // R4 §5: payload_cid SHALL be Bytes ...
    let pcid = match field(m, "payload_cid") {
        Some(Value::Bytes(b)) => b,
        _ => return Err(FieldTypeMismatch),
    };
    // R5 §6.3/ACS-001 §5: ... a well-formed 34-byte 0x12 0x20 || SHA-256 multihash.
    if pcid.len() != 34 || pcid[0] != 0x12 || pcid[1] != 0x20 {
        return Err(MalformedContentId);
    }
    // R6 §5.2/§6.3 (SHARD-001): tenant_id / workspace_id non-empty (type ensured by R4).
    for f in ["tenant_id", "workspace_id"] {
        if let Some(Value::Text(s)) = field(m, f) {
            if s.is_empty() {
                return Err(EmptyShardScope);
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ACS-004 Universal Type Registry — §6.5 / §7 / §8 instance validation.
// ---------------------------------------------------------------------------

const I64_MIN: i128 = -(1i128 << 63);
const I64_MAX: i128 = (1i128 << 63) - 1;
const U32_MAX: i128 = (1i128 << 32) - 1;

/// ACS-004 §6.3 type-code check for a single (already-decoded) value.
fn check_type(code: &str, v: &Value) -> Result<(), SemanticReject> {
    use SemanticReject::*;
    match code {
        "null" => if matches!(v, Value::Null) { Ok(()) } else { Err(FieldTypeMismatch) },
        "bool" => if matches!(v, Value::Bool(_)) { Ok(()) } else { Err(FieldTypeMismatch) },
        "int" => match v {
            Value::Int(i) if *i >= I64_MIN && *i <= I64_MAX => Ok(()),
            Value::Int(_) => Err(ValueOutOfRange),
            _ => Err(FieldTypeMismatch),
        },
        "u32" => match v {
            Value::Int(i) if *i >= 0 && *i <= U32_MAX => Ok(()),
            Value::Int(_) => Err(ValueOutOfRange),
            _ => Err(FieldTypeMismatch),
        },
        "float" => match v {
            Value::Float(f) if f.is_finite() => Ok(()),
            _ => Err(FieldTypeMismatch),
        },
        "conf" => match v {
            Value::Float(f) if f.is_finite() => {
                if *f >= 0.0 && *f <= 1.0 { Ok(()) } else { Err(ValueOutOfRange) }
            }
            _ => Err(FieldTypeMismatch),
        },
        "text" => if matches!(v, Value::Text(_)) { Ok(()) } else { Err(FieldTypeMismatch) },
        "bytes" => if matches!(v, Value::Bytes(_)) { Ok(()) } else { Err(FieldTypeMismatch) },
        "urn" => match v {
            Value::Text(s) if s.starts_with("urn:arves:") => Ok(()),
            _ => Err(FieldTypeMismatch),
        },
        // Unknown type code -> the schema itself is malformed (not exercised by the vectors).
        _ => Err(FieldTypeMismatch),
    }
}

/// Read a §6.2 field descriptor Map -> (type_code, card_code).
fn descriptor(d: &Value) -> Result<(String, String), SemanticReject> {
    let m = match d {
        Value::Map(m) => m,
        _ => return Err(SemanticReject::FieldTypeMismatch),
    };
    let ty = match field(m, "type") {
        Some(Value::Text(s)) => s.clone(),
        _ => return Err(SemanticReject::FieldTypeMismatch),
    };
    let card = match field(m, "card") {
        Some(Value::Text(s)) => s.clone(),
        _ => return Err(SemanticReject::FieldTypeMismatch),
    };
    Ok((ty, card))
}

/// Validate a DECODED instance Map against a DECODED ACS-004 schema document (§6.5/§7/§8).
pub fn validate_instance(instance: &Value, schema: &Value) -> Result<(), SemanticReject> {
    use SemanticReject::*;
    let inst = match instance {
        Value::Map(m) => m,
        _ => return Err(FieldTypeMismatch),
    };
    // §6.5.1: instance keys must be Text.
    for (k, _) in inst {
        if !matches!(k, Value::Text(_)) {
            return Err(UnknownField);
        }
    }
    // Schema fields map.
    let sfields = match schema {
        Value::Map(sm) => match field(sm, "fields") {
            Some(Value::Map(f)) => f,
            _ => return Err(FieldTypeMismatch),
        },
        _ => return Err(FieldTypeMismatch),
    };
    // §6.5.5: closed schema — reject any instance key not in S.fields.
    for (k, _) in inst {
        if let Value::Text(kn) = k {
            if field(sfields, kn).is_none() {
                return Err(UnknownField);
            }
        }
    }
    // Per-field: presence (§6.5.2), type (§6.5.4/§6.3/§7), cardinality (§6.4).
    for (fk, fdesc) in sfields {
        let fname = match fk {
            Value::Text(s) => s.as_str(),
            _ => continue,
        };
        let (type_code, card_code) = descriptor(fdesc)?;
        let val = field(inst, fname);
        if (card_code == "1" || card_code == "1..*") && val.is_none() {
            return Err(MissingRequiredField);
        }
        let val = match val {
            Some(v) => v,
            None => continue, // optional & absent
        };
        if card_code == "1" || card_code == "0..1" {
            if matches!(val, Value::Array(_)) {
                return Err(CardinalityViolation); // scalar cardinality carrying an Array
            }
            check_type(&type_code, val)?;
        } else if card_code == "1..*" || card_code == "0..*" {
            let arr = match val {
                Value::Array(a) => a,
                _ => return Err(CardinalityViolation), // multi cardinality not an Array
            };
            if card_code == "1..*" && arr.is_empty() {
                return Err(CardinalityViolation);
            }
            for el in arr {
                if matches!(el, Value::Array(_)) {
                    return Err(CardinalityViolation);
                }
                check_type(&type_code, el)?;
            }
        } else {
            return Err(FieldTypeMismatch); // unknown cardinality code -> malformed schema
        }
    }
    // §8 provenance state machine: invocation present IFF origin == "derived".
    if let Some(Value::Text(origin)) = field(inst, "origin") {
        let has_invocation = field(inst, "invocation").is_some();
        if origin != "observed" && origin != "derived" && origin != "asserted" {
            return Err(ProvenanceInvariant); // closed Origin variant set (§8)
        }
        if (origin == "derived") != has_invocation {
            return Err(ProvenanceInvariant);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ACS-005 Normative Language — §8/§9.2/§11 term-set structural + grammar rules.
// ---------------------------------------------------------------------------

/// A term-set entry matches the §9.2 v1 Term ID grammar: `GL-` + exactly 3 ASCII digits.
fn is_term_id(e: &str) -> bool {
    let b = e.as_bytes();
    b.len() == 6 && &b[..3] == b"GL-" && b[3].is_ascii_digit() && b[4].is_ascii_digit() && b[5].is_ascii_digit()
}

/// Validate a §8 tag-0x08 glossary term-SET body (Term IDs). `Ok(())` or the registered
/// reject code. Mirrors the Python `_structural_checks` + `_grammar_check` ordering; NFC is
/// deferred here exactly as the ACS-002 byte layer defers it (no Unicode table in the
/// dependency-free reference — the `nfc` tier carries that rule).
pub fn check_term_set(body: &[u8]) -> Result<(), SemanticReject> {
    use SemanticReject::*;
    // R-UTF8 (§9.2/§11): body MUST be valid UTF-8.
    let text = match core::str::from_utf8(body) {
        Ok(t) => t,
        Err(_) => return Err(MalformedTermList),
    };
    // R-NOTRAIL / R-NOLEAD (§8/§11): no leading/trailing LF (single-\n join).
    if text.ends_with('\n') || text.starts_with('\n') {
        return Err(MalformedTermList);
    }
    let entries: Vec<&str> = text.split('\n').collect();
    // R-NOBLANK (§8/§11): no blank line / empty entry.
    if entries.iter().any(|e| e.is_empty()) {
        return Err(MalformedTermList);
    }
    // R-NODUP (§8/§11/§5): entries unique. (Checked before sort, mirroring the reference.)
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            if entries[i] == entries[j] {
                return Err(DuplicateTerm);
            }
        }
    }
    // R-SORT (§8/§11): strictly ascending.
    let mut sorted = entries.clone();
    sorted.sort_unstable();
    if sorted != entries {
        return Err(TermsNotSorted);
    }
    // R-GRAM (§9.2 v1/§7): each entry matches the Term ID grammar.
    for e in &entries {
        if !is_term_id(e) {
            return Err(MalformedTermList);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arves_acs::cbor::{decode_canonical, encode, Value, Value::*};

    fn hex_to_bytes(h: &str) -> Vec<u8> {
        (0..h.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&h[i..i + 2], 16).expect("valid hex"))
            .collect()
    }

    /// An ACS-004 field descriptor `{card, type}` (mirrors the schema golden vector).
    fn fdesc(card: &str, ty: &str) -> Value {
        Map(vec![
            (Text("card".into()), Text(card.into())),
            (Text("type".into()), Text(ty.into())),
        ])
    }

    /// The frozen ACS-004 `uci.fact@1.0` schema document (its `fields` are what
    /// `validate_instance` reads) — byte-identical field set to the ACS-004 golden schema.
    fn fact_schema() -> Value {
        Map(vec![
            (Text("urn".into()), Text("uci.fact".into())),
            (Text("ver".into()), Map(vec![(Text("major".into()), Int(1)), (Text("minor".into()), Int(0))])),
            (Text("root".into()), Text("Fact".into())),
            (Text("fields".into()), Map(vec![
                (Text("urn".into()), fdesc("1", "urn")),
                (Text("tenant".into()), fdesc("1", "text")),
                (Text("workspace".into()), fdesc("1", "text")),
                (Text("origin".into()), fdesc("1", "text")),
                (Text("source".into()), fdesc("1", "text")),
                (Text("invocation".into()), fdesc("0..1", "urn")),
                (Text("confidence".into()), fdesc("1", "conf")),
                (Text("valid_from".into()), fdesc("1", "int")),
                (Text("recorded_at".into()), fdesc("1", "int")),
                (Text("claim".into()), fdesc("1", "text")),
                (Text("observed_at".into()), fdesc("1", "int")),
                (Text("evidence".into()), fdesc("0..*", "urn")),
            ])),
        ])
    }

    /// RCR-004 acceptance proof: the native Rust validators REJECT every frozen semantic
    /// negative vector (envelope 7 + instance 8 + language 4) with the EXACT registered
    /// reason code — retiring the CCP-006 "Rust defers the semantic tiers" deferral.
    #[test]
    fn semantic_rejects_frozen_vectors() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../standard/vectors/acs_negative_vectors.tsv");
        let tsv = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

        // The schema must round-trip through the codec so the instance validator sees the
        // same decoded shape a real body produces.
        let schema = decode_canonical(&encode(&fact_schema())).expect("schema decodes");

        let (mut env, mut inst, mut lang) = (0u32, 0u32, 0u32);
        for line in tsv.lines().skip(1) {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 5 {
                continue;
            }
            let (case, tier, input_hex, reason) = (cols[1], cols[2], cols[3], cols[4]);
            let got = match tier {
                "language" => {
                    lang += 1;
                    check_term_set(&hex_to_bytes(input_hex)).expect_err(case)
                }
                "envelope" | "instance" => {
                    let v = decode_canonical(&hex_to_bytes(input_hex))
                        .unwrap_or_else(|e| panic!("{case}: semantic vector must decode clean: {e:?}"));
                    if tier == "envelope" {
                        env += 1;
                        validate_envelope(&v).expect_err(case)
                    } else {
                        inst += 1;
                        validate_instance(&v, &schema).expect_err(case)
                    }
                }
                _ => continue, // core / nfc — the ACS-002 byte layer, not this module
            };
            assert_eq!(got.code(), reason, "{case}: wrong reject code");
        }
        assert_eq!((env, inst, lang), (7, 8, 4), "expected 7 envelope + 8 instance + 4 language");
    }

    /// The positive golden envelope/instance MUST be ACCEPTED (no false rejects).
    #[test]
    fn semantic_accepts_golden_positives() {
        // A minimal valid instance against the schema (mirrors the ACS-004 golden instance).
        let inst = Map(vec![
            (Text("urn".into()), Text("urn:arves:uci.core:fact@1.0:f-1".into())),
            (Text("tenant".into()), Text("acme".into())),
            (Text("workspace".into()), Text("research".into())),
            (Text("origin".into()), Text("observed".into())),
            (Text("source".into()), Text("sensor-7".into())),
            (Text("confidence".into()), Float(0.98)),
            (Text("valid_from".into()), Int(1730000000000000000)),
            (Text("recorded_at".into()), Int(1730000000500000000)),
            (Text("claim".into()), Text("sky-is-blue".into())),
            (Text("observed_at".into()), Int(1730000000000000000)),
            (Text("evidence".into()), Array(vec![Text("urn:arves:uci.core:evidence@1.0:e-42".into())])),
        ]);
        let schema = decode_canonical(&encode(&fact_schema())).unwrap();
        let inst = decode_canonical(&encode(&inst)).unwrap();
        assert_eq!(validate_instance(&inst, &schema), Ok(()));

        // A valid GL term-set body (GL-001..GL-014) MUST be accepted.
        let body: String = (1..=14).map(|i| format!("GL-{i:03}")).collect::<Vec<_>>().join("\n");
        assert_eq!(check_term_set(body.as_bytes()), Ok(()));
    }
}
