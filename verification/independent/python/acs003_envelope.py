"""
ACS-003 / CCP-003 — ARVES Canonical Envelope: REFERENCE SEMANTIC VALIDATOR.

Independent implementation from the ARVES Standard Kit ONLY
(standard/acs/ACS-003_Canonical_Envelope.md, composing over the independent
ACS-001 addresser and the ACS-002 codec+decoder we already wrote/verified).
No reference-runtime source was consulted; nothing under runtime/ was read. This
module proves the Kit is self-sufficient: the ACS-003 §6.3 decoder MUST-reject
rules are implementable from the standard alone (KPI = Evidence Increased), and
it builds an ORACLE for a future negative-vector CCP.

WHERE THIS SITS (two-layer validation, per ACS-003 §6.3):
  Layer 1 — CANONICAL FORM (ACS-002 §6.3): "is not canonical ACS-002 (non-shortest
    int, unsorted/duplicate keys, indefinite length, non-NFC text, non-finite/-0.0
    float, tags, trailing data)". This is ALREADY enforced by acs002_decode.decode()
    while it parses the bytes; a non-canonical envelope never reaches this validator
    because decode() raises `Rejected` first. This module therefore does NOT re-check
    canonical form — it validates the SEMANTICS of an already-decoded value.
  Layer 2 — ENVELOPE SEMANTICS (ACS-003 §6.3, §5, §5.1, §5.2, §10.4): the field-set,
    presence, closed-key-set, per-field type, multihash shape, and SHARD-001
    non-empty rules. THIS is what the present module enforces. Every mutation in the
    self-test is still valid canonical CBOR, so decode() accepts it; this SEMANTIC
    validator is what must reject it — which is precisely the point of the proof.

Rejection reasons are FREE-FORM descriptive strings that cite the governing § of
ACS-003. Stable machine reason codes for ACS-003 are a FUTURE CCP (ACS-001 §4.1 /
negative-vector CCP); they are intentionally NOT invented here.
"""

import sys

# Reuse our OWN independent Python only (permitted by the independence rules):
#   - the ACS-002 value wrappers + canonical encoder,
#   - the ACS-002 canonical decoder + its decoded-Map type + rejection type,
#   - the ACS-001 addresser,
#   - the spec-derived golden value builders.
from acs002_dcbor import AInt, AFloat, encode
from acs002_decode import decode, Rejected, _MapValue
from acs001_address import content_id
import acs_values


# ---------------------------------------------------------------------------
# §5 field table (normative). The Canonical Envelope has EXACTLY these 12 Text
# keys. `causation_id` is the sole OPTIONAL field (Text | Null); the other 11 are
# REQUIRED. Type obligations are taken verbatim from the §5 "ACS-002 type" column.
# ---------------------------------------------------------------------------

# The four Integer fields (§5: occurred_at/schema_version/payload_domain; the
# validator additionally knows nothing else is Integer). Enumerated for §6.3
# "types any field contrary to §5 (notably ... as non-Integer)".
_INTEGER_FIELDS = ("occurred_at", "schema_version", "payload_domain")

# The Text-typed REQUIRED fields (§5). Each MUST be an ACS-002 Text (Python str).
_TEXT_FIELDS = (
    "ser_version",
    "event_id",
    "event_type",
    "tenant_id",
    "workspace_id",
    "correlation_id",
    "source",
)

# payload_cid is the sole Bytes field (§5). causation_id is Text|Null (§5.7).
# The complete closed key set (§5: "No other keys SHALL appear"):
_ALL_FIELDS = frozenset(
    _TEXT_FIELDS + _INTEGER_FIELDS + ("payload_cid", "causation_id")
)
assert len(_ALL_FIELDS) == 12, "the ACS-003 §5 field set is exactly 12 fields"

# The REQUIRED fields: everything except the one OPTIONAL field, causation_id (§5).
_REQUIRED_FIELDS = _ALL_FIELDS - {"causation_id"}
assert len(_REQUIRED_FIELDS) == 11

# ACS-001 §5 SHA-256 multihash shape, reused by ACS-003 §6.3 for payload_cid:
# 34 bytes total = 0x12 (sha2-256 code) 0x20 (digest length 32) || 32-byte digest.
_MULTIHASH_SHA256_CODE = 0x12
_MULTIHASH_SHA256_LEN = 0x20
_MULTIHASH_TOTAL_LEN = 2 + 32   # = 34 (ACS-001 §5 / ACS-003 §6.3)


class EnvelopeInvalid(Exception):
    """
    Raised by validate_envelope() when an ACS-003 §6.3 semantic reject rule fires.
    `reason` is a free-form descriptive string that cites the governing § (stable
    machine reason codes are a future CCP — deliberately not invented here).
    """
    __slots__ = ("reason",)

    def __init__(self, reason):
        self.reason = reason
        super().__init__(reason)


def _get(env, text_key):
    """
    Fetch the value for a Text key from a decoded envelope. The decoder keys Text
    entries as ('T', text) tuples (acs002_decode._MapValue). Returns a sentinel
    (KeyError propagated as absence) via the caller's `in` test; here we assume
    presence has already been checked.
    """
    return env[("T", text_key)]


def validate_envelope(value):
    """
    Validate a DECODED ACS-003 Canonical Envelope against every ACS-003 §6.3
    semantic MUST-reject clause. Raises EnvelopeInvalid(reason) on the first
    violation; returns None on success.

    `value` is the object returned by acs002_decode.decode() for the envelope body
    (a _MapValue whose Text keys are ('T', text) tuples). Canonical-form validation
    (ACS-002 §6.3) has already been performed by decode(); this function performs
    ONLY the ACS-003 envelope-semantics layer.

    Reject rules enforced (each cited to its §):
      R1  §4 / §6.3   — the envelope value SHALL be an ACS-002 Map.
      R2  §6.3 / §5   — every REQUIRED field (11) SHALL be present.
      R3  §5 / §6.3   — NO unknown key (closed 12-field set; "No other keys SHALL appear").
      R4  §5 / §6.3   — each field typed per §5 (Text=str, Integer=AInt, Bytes for
                        payload_cid, Text|Null for causation_id); notably occurred_at/
                        schema_version/payload_domain are Integer, never Float (§5.1).
      R5  §6.3 / ACS-001 §5 — payload_cid is a well-formed 34-byte
                        0x12 0x20 || 32-byte SHA-256 multihash.
      R6  §6.3 / §5.2 (SHARD-001) — tenant_id / workspace_id non-null and non-empty.
    """

    # ---- R1: the envelope value SHALL be an ACS-002 Map (§4, §6.3). ----
    # The decoder represents a Map as _MapValue (a dict subclass). Anything else
    # (Text, Integer, Array, Null, ...) is not an envelope.
    if not isinstance(value, _MapValue):
        raise EnvelopeInvalid(
            "ACS-003 §4/§6.3: envelope value is not an ACS-002 Map (got %s)"
            % type(value).__name__
        )
    env = value

    # Collect the present key names. All envelope keys are Text (§5: "Every field
    # below is a Text key of the envelope Map"). A non-Text key is therefore itself
    # an unknown/illegal key and is caught by R3 below.
    present_text_keys = set()
    for (kind, k) in env.keys():
        if kind == "T":
            present_text_keys.add(k)
        else:
            # A non-Text (Integer) map key cannot be a §5 envelope field.
            raise EnvelopeInvalid(
                "ACS-003 §5/§6.3: envelope carries a non-Text (Integer) map key %r; "
                "every envelope field is a Text key (unknown key rejected)" % (k,)
            )

    # ---- R3: NO unknown key (§5 closed set; §6.3 "carries an unknown key"). ----
    unknown = present_text_keys - _ALL_FIELDS
    if unknown:
        raise EnvelopeInvalid(
            "ACS-003 §5/§6.3: unknown key(s) present, closed field set violated: %s"
            % ", ".join(sorted(unknown))
        )

    # ---- R2: every REQUIRED field present (§6.3 "missing any REQUIRED field"). ----
    missing = _REQUIRED_FIELDS - present_text_keys
    if missing:
        raise EnvelopeInvalid(
            "ACS-003 §6.3: missing REQUIRED field(s): %s"
            % ", ".join(sorted(missing))
        )

    # ---- R4: per-field type checks (§5 type column; §5.1 Integer-not-Float). ----

    # Text-typed REQUIRED fields SHALL be an ACS-002 Text (Python str). A bool
    # (§4 kind 2) or any non-str is a type violation. (bool is excluded explicitly:
    # although Python bools are ints, decode() returns them as bool, and Text is str.)
    for f in _TEXT_FIELDS:
        v = _get(env, f)
        if not isinstance(v, str):
            raise EnvelopeInvalid(
                "ACS-003 §5/§6.3: field %r SHALL be Text (str), got %s"
                % (f, type(v).__name__)
            )

    # Integer-typed REQUIRED fields SHALL be an ACS-002 Integer (AInt), and SHALL
    # NOT be a Float (§5.1 forbids occurred_at as a binary64 float; §5/§10.4 forbid
    # schema_version/payload_domain as Float). Our decoder yields Integer=AInt,
    # Float=AFloat, so a Float shows up as AFloat and is rejected here.
    for f in _INTEGER_FIELDS:
        v = _get(env, f)
        if isinstance(v, AFloat):
            raise EnvelopeInvalid(
                "ACS-003 §5.1/§6.3: field %r SHALL be an ACS-002 Integer, not a "
                "Float (nanosecond/contract values MUST NOT round-trip through "
                "binary64)" % f
            )
        if not isinstance(v, AInt):
            raise EnvelopeInvalid(
                "ACS-003 §5/§6.3: field %r SHALL be an ACS-002 Integer (AInt), got %s"
                % (f, type(v).__name__)
            )

    # causation_id: OPTIONAL, and when present SHALL be Text OR the explicit Null
    # (§5.7 present-with-Null). An AInt/Bytes/Array/Bool here is a type violation.
    if "causation_id" in present_text_keys:
        cv = _get(env, "causation_id")
        if not (cv is None or isinstance(cv, str)):
            raise EnvelopeInvalid(
                "ACS-003 §5/§5.7/§6.3: field 'causation_id' SHALL be Text or Null, "
                "got %s" % type(cv).__name__
            )

    # payload_cid: SHALL be Bytes (§5: "Carried as Bytes ... NOT as a hex Text").
    pcid = _get(env, "payload_cid")
    if not isinstance(pcid, (bytes, bytearray)):
        raise EnvelopeInvalid(
            "ACS-003 §5/§6.3: field 'payload_cid' SHALL be Bytes, got %s"
            % type(pcid).__name__
        )
    pcid = bytes(pcid)

    # ---- R5: payload_cid well-formed 34-byte SHA-256 multihash (§6.3, ACS-001 §5).
    if len(pcid) != _MULTIHASH_TOTAL_LEN:
        raise EnvelopeInvalid(
            "ACS-003 §6.3/ACS-001 §5: payload_cid SHALL be a 34-byte multihash, "
            "got %d byte(s)" % len(pcid)
        )
    if pcid[0] != _MULTIHASH_SHA256_CODE or pcid[1] != _MULTIHASH_SHA256_LEN:
        raise EnvelopeInvalid(
            "ACS-003 §6.3/ACS-001 §5: payload_cid SHALL begin with the SHA-256 "
            "multihash prefix 0x12 0x20, got 0x%02x 0x%02x" % (pcid[0], pcid[1])
        )

    # ---- R6: tenant_id / workspace_id non-null and non-empty (§6.3, §5.2, SHARD-001).
    # (Type as Text was already enforced in R4; here we enforce non-empty. Null would
    #  have failed R4's str check, but §5.2/§6.3 name null AND empty explicitly.)
    for f in ("tenant_id", "workspace_id"):
        v = _get(env, f)
        if v is None:
            raise EnvelopeInvalid(
                "ACS-003 §5.2/§6.3 (SHARD-001): %r SHALL NOT be Null" % f
            )
        if v == "":
            raise EnvelopeInvalid(
                "ACS-003 §5.2/§6.3 (SHARD-001): %r SHALL NOT be empty" % f
            )

    # All ACS-003 §6.3 envelope-semantic clauses satisfied.
    return None


# ---------------------------------------------------------------------------
# SELF-TEST — the proof. One positive; one negative per §6.3 reject rule.
# ---------------------------------------------------------------------------

def _valid_envelope_dict():
    """
    Build the VALID §10.2 envelope value (encoder-side dict), with the CORRECT
    34-byte payload_cid computed as content_id(0x01, encode(acs002_v1_fact())):
    the ACS-001 ContentId of the ACS-002 V1 fact body (ACS-003 §10.1).
    """
    payload_body = encode(acs_values.acs002_v1_fact())     # ACS-002/1 dCBOR body
    payload_cid = content_id(0x01, payload_body)           # ACS-001 §5 multihash
    return acs_values.acs003_envelope(payload_cid)


def _roundtrip_validate(enc_dict):
    """
    encode -> decode -> validate. Returns (accepted: bool, reason: str). A dict that
    cannot even be encoded/decoded is reported distinctly so a test bug is not
    mistaken for a semantic rejection.
    """
    body = encode(enc_dict)          # ACS-002/1 canonical bytes (still canonical!)
    value = decode(body)             # ACS-002 §6.3 canonical-form validation passes
    try:
        validate_envelope(value)
        return True, ""
    except EnvelopeInvalid as e:
        return False, e.reason


def run_selftest():
    """
    Exit 0 iff the positive envelope is ACCEPTED and every negative is REJECTED.
    Prints a per-case PASS/FAIL line and a summary.
    """
    results = []   # (name, expect_accept, ok, detail)

    # ---------- POSITIVE ----------
    valid = _valid_envelope_dict()
    acc, reason = _roundtrip_validate(valid)
    ok = (acc is True)
    results.append(("POSITIVE valid §10.2 envelope", True, ok,
                    "accepted" if acc else ("REJECTED: " + reason)))

    # A small helper to run a negative: start from a FRESH valid dict, mutate, expect
    # rejection.
    def negative(name, mutate):
        d = _valid_envelope_dict()
        mutate(d)
        acc2, reason2 = _roundtrip_validate(d)
        rejected = (acc2 is False)
        results.append((name, False, rejected,
                        ("rejected: " + reason2) if rejected
                        else "ACCEPTED (should have been rejected)"))

    # ---------- NEGATIVES (one per §6.3 reject rule) ----------

    # (a) R2 — missing a REQUIRED field: drop event_type (§6.3).
    negative("NEG (a) missing REQUIRED field event_type  [R2 §6.3]",
             lambda d: d.pop("event_type"))

    # (b) R3 — unknown/extra key (§5 closed set; §6.3).
    negative("NEG (b) unknown extra key 'extra'          [R3 §5/§6.3]",
             lambda d: d.__setitem__("extra", "nope"))

    # (c) R4 — wrong-typed field: occurred_at as Text instead of AInt (§5/§5.1).
    negative("NEG (c) occurred_at as Text not Integer     [R4 §5.1/§6.3]",
             lambda d: d.__setitem__("occurred_at", "1730000000000000000"))

    # (d) R5 — payload_cid wrong length: 33 bytes (§6.3/ACS-001 §5).
    negative("NEG (d) payload_cid wrong length (33 bytes) [R5 §6.3/ACS-001 §5]",
             lambda d: d.__setitem__("payload_cid", bytes(33)))

    # (e) R5 — payload_cid wrong multihash prefix (34 bytes, prefix 0x00 0x20).
    negative("NEG (e) payload_cid wrong multihash prefix  [R5 §6.3/ACS-001 §5]",
             lambda d: d.__setitem__(
                 "payload_cid", bytes([0x00, 0x20]) + bytes(32)))

    # (f) R6 — empty tenant_id (§6.3/§5.2 SHARD-001).
    negative("NEG (f) empty tenant_id                     [R6 §5.2/§6.3 SHARD-001]",
             lambda d: d.__setitem__("tenant_id", ""))

    # (g) R4 — causation_id wrong type: AInt instead of Text|Null (§5/§5.7).
    negative("NEG (g) causation_id as Integer not Text|Null[R4 §5.7/§6.3]",
             lambda d: d.__setitem__("causation_id", AInt(7)))

    # ---------- REPORT ----------
    print("ACS-003 Canonical Envelope — reference validator self-test")
    print("=" * 70)
    all_ok = True
    for name, expect_accept, ok, detail in results:
        status = "PASS" if ok else "FAIL"
        if not ok:
            all_ok = False
        print("  [%s] %-48s -> %s" % (status, name, detail))
    print("=" * 70)
    passed = sum(1 for _, _, ok, _ in results if ok)
    total = len(results)
    pos_ok = sum(1 for _, ea, ok, _ in results if ea and ok)
    neg_ok = sum(1 for _, ea, ok, _ in results if (not ea) and ok)
    print("SUMMARY: %d/%d cases passed (%d positive accepted, %d negatives rejected)"
          % (passed, total, pos_ok, neg_ok))
    print("RESULT: %s" % ("GREEN (exit 0)" if all_ok else "RED (exit 1)"))
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(run_selftest())
