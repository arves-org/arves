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
//!
//! `decode_canonical` (below) is the matching REJECTING decoder: it accepts a byte
//! string only if it is in the exact canonical form this encoder produces, and
//! returns a specific `DecodeError` for every non-canonical input. This is the
//! other half of a real serialization standard — a conformant implementation must
//! agree not only on what to ACCEPT but on what to REJECT, or two implementations
//! could assign different addresses to "the same" value. (NFC-rejection is the one
//! rule this dependency-free reference defers — it needs a Unicode table — so it is
//! carried as an `nfc`-tier negative vector, enforced by implementations that have
//! a Unicode facility. All other canonical-form rules are enforced here.)

/// A canonical ARVES value (the ACS-002 value model).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// A signed integer in the ACS-002 §4 model range `[-2^64, 2^64-1]` (the full
    /// CBOR major-0/major-1 space with an 8-byte argument). `i128` is used because
    /// that range does not fit in `i64`; the ontology's `Timestamp` (i64) is a
    /// strict subset. Constructing an `Int` outside the range is a programmer error.
    Int(i128),
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
            debug_assert!(
                *i >= -(1i128 << 64) && *i <= (1i128 << 64) - 1,
                "ACS-002 §4: Integer range is [-2^64, 2^64-1]"
            );
            if *i >= 0 {
                // major 0; value fits u64 (max 2^64-1).
                head(o, 0, *i as u64);
            } else {
                // major 1 encodes -1 - n; for i<0, n = -1 - i (in [0, 2^64-1]).
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
            debug_assert!(
                m.iter().all(|(k, _)| matches!(k, Value::Text(_) | Value::Int(_))),
                "ACS-002 §4 kind 8: map keys must be Text or Integer"
            );
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

// --- Canonical decoder (the rejecting half of the standard) -------------------

/// Why a byte string is NOT canonical dCBOR (ACS-002 §5). A conformant decoder
/// MUST reject every one of these. Each maps to a stable machine reason `code()`
/// used in the negative-vector corpus and the conformance report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Integer not in shortest form (RFC 8949 §4.2.1 / ACS-002).
    NonShortestInt,
    /// Length/count not in shortest form.
    NonShortestLen,
    /// Indefinite-length item or stray break — definite lengths only.
    IndefiniteLength,
    /// Additional-info 28/29/30 (reserved), a tag, or a simple/undefined value
    /// not in the ACS-002 value model.
    ReservedOrUnsupported,
    /// Map keys not in ascending bytewise order of their encoded key bytes.
    UnsortedMapKeys,
    /// Two map keys with identical encoded bytes.
    DuplicateMapKeys,
    /// A float not encoded as 64-bit (half 0xf9 / single 0xfa forbidden).
    FloatNotFloat64,
    /// Negative zero (-0.0): must normalize to +0.0, so it is non-canonical on the wire.
    NegativeZeroFloat,
    /// NaN or ±Infinity — forbidden by ACS-002.
    NonFiniteFloat,
    /// Input ended mid-item.
    Truncated,
    /// Bytes remain after a complete top-level item.
    TrailingData,
}

impl DecodeError {
    /// Stable machine reason code (shared by the Kit corpus + every runner).
    pub fn code(self) -> &'static str {
        match self {
            DecodeError::NonShortestInt => "non-shortest-int",
            DecodeError::NonShortestLen => "non-shortest-len",
            DecodeError::IndefiniteLength => "indefinite-length",
            DecodeError::ReservedOrUnsupported => "reserved-or-unsupported",
            DecodeError::UnsortedMapKeys => "unsorted-map-keys",
            DecodeError::DuplicateMapKeys => "duplicate-map-keys",
            DecodeError::FloatNotFloat64 => "float-not-float64",
            DecodeError::NegativeZeroFloat => "negative-zero-float",
            DecodeError::NonFiniteFloat => "non-finite-float",
            DecodeError::Truncated => "truncated",
            DecodeError::TrailingData => "trailing-data",
        }
    }
}

struct Cursor<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Cursor<'a> {
    fn next_u8(&mut self) -> Result<u8, DecodeError> {
        let x = *self.b.get(self.i).ok_or(DecodeError::Truncated)?;
        self.i += 1;
        Ok(x)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.i.checked_add(n).ok_or(DecodeError::Truncated)?;
        if end > self.b.len() {
            return Err(DecodeError::Truncated);
        }
        let s = &self.b[self.i..end];
        self.i = end;
        Ok(s)
    }
}

/// Decode a byte string as a single canonical dCBOR value, enforcing every ACS-002
/// canonical-form rule (except NFC — see the module note). Returns the value, or
/// the specific reason the bytes are non-canonical. Whole input must be one item.
pub fn decode_canonical(bytes: &[u8]) -> Result<Value, DecodeError> {
    let mut c = Cursor { b: bytes, i: 0 };
    let v = read_value(&mut c)?;
    if c.i != bytes.len() {
        return Err(DecodeError::TrailingData);
    }
    Ok(v)
}

/// Read the unsigned argument for majors 0..=5 with shortest-form enforcement.
fn read_arg(c: &mut Cursor, ai: u8, as_len: bool) -> Result<u64, DecodeError> {
    let short = if as_len { DecodeError::NonShortestLen } else { DecodeError::NonShortestInt };
    match ai {
        0..=23 => Ok(ai as u64),
        24 => {
            let v = c.next_u8()? as u64;
            if v < 24 {
                return Err(short);
            }
            Ok(v)
        }
        25 => {
            let b = c.take(2)?;
            let v = u16::from_be_bytes([b[0], b[1]]) as u64;
            if v < 0x100 {
                return Err(short);
            }
            Ok(v)
        }
        26 => {
            let b = c.take(4)?;
            let v = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64;
            if v < 0x1_0000 {
                return Err(short);
            }
            Ok(v)
        }
        27 => {
            let b = c.take(8)?;
            let v = u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
            if v < 0x1_0000_0000 {
                return Err(short);
            }
            Ok(v)
        }
        28 | 29 | 30 => Err(DecodeError::ReservedOrUnsupported),
        _ => Err(DecodeError::IndefiniteLength), // ai == 31
    }
}

fn read_value(c: &mut Cursor) -> Result<Value, DecodeError> {
    let ib = c.next_u8()?;
    let major = ib >> 5;
    let ai = ib & 0x1f;

    if major == 7 {
        return read_simple_or_float(c, ai);
    }
    // major 6 = CBOR tags: not in the §4 value model. Reject before parsing the
    // argument so a tag reads as reserved-or-unsupported (not a stray arg-shape error).
    if major == 6 {
        return Err(DecodeError::ReservedOrUnsupported);
    }

    let as_len = matches!(major, 2 | 3 | 4 | 5);
    let arg = read_arg(c, ai, as_len)?;

    match major {
        // Every 8-byte-argument integer is in the §4 model range [-2^64, 2^64-1],
        // so there is no in-range rejection: major 0 -> [0, 2^64-1], major 1 ->
        // [-2^64, -1]. The i128 value type holds the whole range exactly.
        0 => Ok(Value::Int(arg as i128)),
        1 => Ok(Value::Int(-1 - arg as i128)),
        2 => Ok(Value::Bytes(c.take(arg as usize)?.to_vec())),
        3 => {
            let s = c.take(arg as usize)?;
            // Non-UTF-8 octets are not a Text value in the §4 model -> reserved-or-
            // unsupported (the same reason a conformant peer emits; NFC is deferred).
            let txt = core::str::from_utf8(s).map_err(|_| DecodeError::ReservedOrUnsupported)?;
            Ok(Value::Text(txt.to_string()))
        }
        4 => {
            let n = arg as usize;
            let mut items = Vec::with_capacity(n.min(64));
            for _ in 0..n {
                items.push(read_value(c)?);
            }
            Ok(Value::Array(items))
        }
        5 => {
            let n = arg as usize;
            let mut entries = Vec::with_capacity(n.min(64));
            let mut prev_key: Option<Vec<u8>> = None;
            for _ in 0..n {
                let key_start = c.i;
                let k = read_value(c)?;
                // §4 kind 8: a map key MUST be a Text or an Integer. A Null/Bool/
                // Float/Bytes/Array/Map key is not a valid ARVES map, so the body is
                // non-canonical and MUST be rejected.
                if !matches!(k, Value::Text(_) | Value::Int(_)) {
                    return Err(DecodeError::ReservedOrUnsupported);
                }
                let key_bytes = c.b[key_start..c.i].to_vec();
                let val = read_value(c)?;
                if let Some(prev) = &prev_key {
                    match key_bytes.as_slice().cmp(prev.as_slice()) {
                        core::cmp::Ordering::Less => return Err(DecodeError::UnsortedMapKeys),
                        core::cmp::Ordering::Equal => return Err(DecodeError::DuplicateMapKeys),
                        core::cmp::Ordering::Greater => {}
                    }
                }
                prev_key = Some(key_bytes);
                entries.push((k, val));
            }
            Ok(Value::Map(entries))
        }
        _ => Err(DecodeError::ReservedOrUnsupported), // major 6 (tags): not in the model
    }
}

fn read_simple_or_float(c: &mut Cursor, ai: u8) -> Result<Value, DecodeError> {
    match ai {
        20 => Ok(Value::Bool(false)), // 0xf4
        21 => Ok(Value::Bool(true)),  // 0xf5
        22 => Ok(Value::Null),        // 0xf6
        25 | 26 => Err(DecodeError::FloatNotFloat64), // half / single float forbidden
        27 => {
            let b = c.take(8)?;
            let bits = u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
            if bits == 0x8000_0000_0000_0000 {
                return Err(DecodeError::NegativeZeroFloat);
            }
            if (bits >> 52) & 0x7ff == 0x7ff {
                return Err(DecodeError::NonFiniteFloat); // NaN or ±Inf
            }
            Ok(Value::Float(f64::from_bits(bits)))
        }
        // 31 is the 'break' stop code — only meaningful inside an indefinite item,
        // which is forbidden; label it indefinite-length (matching a conformant peer).
        31 => Err(DecodeError::IndefiniteLength),
        // 23 undefined, 24 simple-in-next-byte, 28..30 reserved.
        _ => Err(DecodeError::ReservedOrUnsupported),
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

    // The decoder ACCEPTS every canonical body and round-trips it exactly:
    // decode(encode(v)) == v  AND  encode(decode(body)) == body.
    #[test]
    fn decode_round_trips_canonical() {
        let cases = vec![
            Null,
            Bool(true),
            Bool(false),
            Int(0),
            Int(23),
            Int(24),
            Int(-1000),
            Int(1730000000000000000),
            Int(i64::MIN as i128),
            Int(i64::MAX as i128),
            // The §4 range extends beyond i64 to the full CBOR major-0/1 space.
            Int((1i128 << 64) - 1), // 2^64 - 1 (major 0, 8-byte arg all-ones)
            Int(-(1i128 << 64)),    // -2^64   (major 1, 8-byte arg all-ones)
            Int(1i128 << 63),       // 2^63    (i64::MAX + 1)
            Int(-(1i128 << 63) - 1), // -(2^63)-1 (i64::MIN - 1)
            Float(0.0),
            Float(0.5),
            Float(-1.25),
            Text("hello-truth".into()),
            Text("Am\u{00e9}lie \u{00e9}\u{2014}\u{4e2d}".into()),
            Bytes(vec![0x12, 0x20, 0xff, 0x00]),
            Array(vec![Text("uci.observation".into()), Text("uci.fact".into())]),
            // The ACS-002 V1 golden fact, authored out of order on purpose.
            Map(vec![
                (Text("observed_at".into()), Int(1730000000000000000)),
                (Text("type".into()), Text("uci.fact".into())),
                (Text("confidence".into()), Float(0.5)),
                (Text("claim".into()), Text("sky-is-blue".into())),
            ]),
        ];
        for v in cases {
            let body = encode(&v);
            let decoded = decode_canonical(&body).expect("canonical body must decode");
            // Re-encoding the decoded value reproduces the exact canonical bytes.
            assert_eq!(encode(&decoded), body, "round-trip bytes differ for {v:?}");
        }
    }

    // The decoder REJECTS each non-canonical input with the specific reason.
    #[test]
    fn decode_rejects_noncanonical() {
        let cases: Vec<(&str, Vec<u8>, DecodeError)> = vec![
            // 0 encoded with a 1-byte argument instead of the 1-byte head 0x00.
            ("non-shortest int", vec![0x18, 0x00], DecodeError::NonShortestInt),
            // text "a" with a 1-byte length argument (0x78 0x01) instead of 0x61.
            ("non-shortest len", vec![0x78, 0x01, 0x61], DecodeError::NonShortestLen),
            // indefinite-length array [0] then break.
            ("indefinite array", vec![0x9f, 0x00, 0xff], DecodeError::IndefiniteLength),
            // map {"b":1,"a":2} — keys out of bytewise order.
            ("unsorted keys", vec![0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02], DecodeError::UnsortedMapKeys),
            // map {"a":1,"a":2} — duplicate key.
            ("duplicate keys", vec![0xa2, 0x61, 0x61, 0x01, 0x61, 0x61, 0x02], DecodeError::DuplicateMapKeys),
            // half-precision float 1.0.
            ("half float", vec![0xf9, 0x3c, 0x00], DecodeError::FloatNotFloat64),
            // single-precision float 1.0.
            ("single float", vec![0xfa, 0x3f, 0x80, 0x00, 0x00], DecodeError::FloatNotFloat64),
            // -0.0 as float64.
            ("negative zero", vec![0xfb, 0x80, 0, 0, 0, 0, 0, 0, 0], DecodeError::NegativeZeroFloat),
            // +Infinity as float64.
            ("infinity", vec![0xfb, 0x7f, 0xf0, 0, 0, 0, 0, 0, 0], DecodeError::NonFiniteFloat),
            // canonical 0 (0x00) followed by a trailing byte.
            ("trailing data", vec![0x00, 0x00], DecodeError::TrailingData),
            // a CBOR tag (major 6) — not in the ACS-002 value model.
            ("tag", vec![0xc0, 0x00], DecodeError::ReservedOrUnsupported),
            // truncated: claims a 4-byte text, gives 1.
            ("truncated", vec![0x64, 0x61], DecodeError::Truncated),
            // map with a Null key {null:0} — §4 kind 8 keys must be Text or Integer.
            ("map key not in model", vec![0xa1, 0xf6, 0x00], DecodeError::ReservedOrUnsupported),
            // text item whose octet 0xff is not valid UTF-8.
            ("text invalid utf8", vec![0x61, 0xff], DecodeError::ReservedOrUnsupported),
            // a bare 'break' (0xff) at top level — only valid inside an indefinite item.
            ("top-level break", vec![0xff], DecodeError::IndefiniteLength),
        ];
        for (name, bytes, want) in cases {
            match decode_canonical(&bytes) {
                Err(got) => assert_eq!(got, want, "{name}: wrong reason"),
                Ok(v) => panic!("{name}: expected rejection {want:?}, decoded {v:?}"),
            }
        }
    }
}
