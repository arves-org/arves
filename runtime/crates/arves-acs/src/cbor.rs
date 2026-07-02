//! ACS-002 Canonical Serialization — deterministic CBOR (RFC 8949 §4.2, dCBOR
//! profile) reference encoder. Produces the byte-exact "body" that ACS-001
//! content-addresses. Dependency-free.
//!
//! Profile (normative, ACS-002): shortest-form integers; floats ALWAYS 64-bit
//! (major 7, 0xfb), -0.0 normalized to +0.0, NaN/Inf forbidden; definite lengths
//! only; map keys sorted bytewise by their own encoded bytes, no duplicates;
//! Integer and Float are distinct kinds. Text is UTF-8 and MUST be NFC — this
//! reference ENCODER assumes callers pass NFC text (NFC normalization + non-NFC
//! rejection is a decoder/validation obligation and needs a Unicode table; it is
//! tracked, not yet implemented here). The golden vectors below use NFC inputs.

/// A canonical ARVES value (the ACS-002 value model).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(Vec<(Value, Value)>),
}

/// Encode a value to canonical dCBOR bytes (ACS-002/1).
pub fn encode(v: &Value) -> Vec<u8> {
    let mut o = Vec::new();
    enc(&mut o, v);
    o
}

/// Shortest-form major-type head (RFC 8949 §4.2.1).
fn head(o: &mut Vec<u8>, major: u8, u: u64) {
    let m = major << 5;
    if u < 24 {
        o.push(m | (u as u8));
    } else if u < 0x100 {
        o.push(m | 24);
        o.push(u as u8);
    } else if u < 0x1_0000 {
        o.push(m | 25);
        o.extend_from_slice(&(u as u16).to_be_bytes());
    } else if u < 0x1_0000_0000 {
        o.push(m | 26);
        o.extend_from_slice(&(u as u32).to_be_bytes());
    } else {
        o.push(m | 27);
        o.extend_from_slice(&u.to_be_bytes());
    }
}

fn enc(o: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Null => o.push(0xf6),
        Value::Bool(b) => o.push(if *b { 0xf5 } else { 0xf4 }),
        Value::Int(i) => {
            if *i >= 0 {
                head(o, 0, *i as u64);
            } else {
                // major 1 encodes -1 - n; for i<0, n = -1 - i.
                head(o, 1, (-1 - *i) as u64);
            }
        }
        Value::Float(f) => {
            debug_assert!(f.is_finite(), "ACS-002 forbids NaN/Inf floats");
            let mut bits = f.to_bits();
            if bits == 0x8000_0000_0000_0000 {
                bits = 0; // -0.0 -> +0.0
            }
            o.push(0xfb);
            o.extend_from_slice(&bits.to_be_bytes());
        }
        Value::Text(s) => {
            head(o, 3, s.len() as u64);
            o.extend_from_slice(s.as_bytes());
        }
        Value::Bytes(b) => {
            head(o, 2, b.len() as u64);
            o.extend_from_slice(b);
        }
        Value::Array(a) => {
            head(o, 4, a.len() as u64);
            for it in a {
                enc(o, it);
            }
        }
        Value::Map(m) => {
            head(o, 5, m.len() as u64);
            // Canonical: sort entries bytewise by each key's own encoded bytes.
            let mut entries: Vec<(Vec<u8>, Vec<u8>)> =
                m.iter().map(|(k, val)| (encode(k), encode(val))).collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, val) in entries {
                o.extend_from_slice(&k);
                o.extend_from_slice(&val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value::*;
    use super::*;
    use crate::{content_id, domain, hex};

    #[test]
    fn dcbor_scalars() {
        assert_eq!(hex(&encode(&Bool(true))), "f5");
        assert_eq!(hex(&encode(&Bool(false))), "f4");
        assert_eq!(hex(&encode(&Null)), "f6");
        assert_eq!(hex(&encode(&Int(0))), "00");
        assert_eq!(hex(&encode(&Int(24))), "1818");
        assert_eq!(hex(&encode(&Int(-1000))), "3903e7");
        assert_eq!(hex(&encode(&Float(1.0))), "fb3ff0000000000000");
        assert_eq!(hex(&encode(&Text("hello-truth".into()))), "6b68656c6c6f2d7472757468");
    }

    // ACS-002-CS-1 V1: a uci.fact map. Fed in NON-sorted field order to prove
    // canonicalization (map-key sort) yields the byte-exact standard body + CID.
    #[test]
    fn acs_002_cs_1_v1_fact() {
        let v = Map(vec![
            (Text("observed_at".into()), Int(1730000000000000000)),
            (Text("type".into()), Text("uci.fact".into())),
            (Text("confidence".into()), Float(0.5)),
            (Text("claim".into()), Text("sky-is-blue".into())),
        ]);
        let body = encode(&v);
        assert_eq!(
            hex(&body),
            "a46474797065687563692e6661637465636c61696d6b736b792d69732d626c75656a636f6e666964656e6365fb3fe00000000000006b6f627365727665645f61741b180231d5856d0000"
        );
        assert_eq!(
            hex(&content_id(domain::COMMIT_CONTENT, &body)),
            "12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e"
        );
    }

    // ACS-002-CS-1 V2: engine manifest (null, array, bool, int).
    #[test]
    fn acs_002_cs_1_v2_engine() {
        let v = Map(vec![
            (Text("engine".into()), Text("summarize".into())),
            (Text("version".into()), Int(1)),
            (Text("deterministic".into()), Bool(true)),
            (Text("reads".into()), Array(vec![Text("uci.observation".into()), Text("uci.fact".into())])),
            (Text("seed".into()), Null),
        ]);
        let body = encode(&v);
        assert_eq!(
            hex(&body),
            "a56473656564f6657265616473826f7563692e6f62736572766174696f6e687563692e6661637466656e67696e656973756d6d6172697a656776657273696f6e016d64657465726d696e6973746963f5"
        );
        assert_eq!(
            hex(&content_id(domain::ENGINE_MANIFEST, &body)),
            "1220e5aad722341bd0838fb268d73a0a28401457883b9e5e623c05dc0623f57a690d"
        );
    }

    // ACS-002-CS-1 V3: NFC text (é=U+00E9, —=U+2014, 中=U+4E2D) + negative int.
    #[test]
    fn acs_002_cs_1_v3_nfc_text() {
        let v = Map(vec![
            (Text("label".into()), Text("Am\u{00e9}lie \u{00e9}\u{2014}\u{4e2d}".into())),
            (Text("n".into()), Int(-1000)),
        ]);
        let body = encode(&v);
        assert_eq!(hex(&body), "a2616e3903e7656c6162656c70416dc3a96c696520c3a9e28094e4b8ad");
        assert_eq!(
            hex(&content_id(domain::DECISION_TRACE, &body)),
            "12207c5367768a3cd0d90b781cac2530335f0310ffc155eac4ac82da80af71e2366a"
        );
    }

    // Canonicalization is order-independent: two author orderings -> same bytes.
    #[test]
    fn map_key_order_is_canonical() {
        let a = Map(vec![(Text("b".into()), Int(2)), (Text("a".into()), Int(1))]);
        let b = Map(vec![(Text("a".into()), Int(1)), (Text("b".into()), Int(2))]);
        assert_eq!(encode(&a), encode(&b));
    }
}
