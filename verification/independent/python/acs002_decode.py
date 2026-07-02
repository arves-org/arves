"""
ACS-002 / CCP-002 — ARVES Canonical Serialization: CANONICAL DECODER + VALIDATOR.

Independent implementation from the ARVES Standard Kit ONLY
(standard/acs/ACS-002_Canonical_Serialization.md and
standard/conformance/CONFORMANCE.md). No reference-runtime source was consulted;
no Rust and nothing under runtime/ was read. This module proves the Kit is
self-sufficient: every rule below is derived from ACS-002 prose and cited to the
section that fixes it.

A conformant decoder MUST do TWO things (ACS-002 §6.3, §3 "Non-canonical
encoding", CONFORMANCE.md "rejection check"):
  1. decode a canonical body to an ARVES value (§4 value model), and
  2. REJECT any byte string that is not itself the canonical form — otherwise two
     byte strings that "mean the same value" would carry different ContentIds and
     break ORCH-004 idempotency (§6.3).

Decoding therefore validates canonical form INLINE while parsing the bytes — it
never pattern-matches whole test inputs. Each violation raises `Rejected` with a
stable reason code (CONFORMANCE.md "Reason codes (ACS-002 §5)").

Reason codes (verbatim from CONFORMANCE.md line 38-41, sourced ACS-002 §5):
    non-shortest-int      §5.2  argument not in fewest bytes
    non-shortest-len      §5.1/§5.2  length argument not in fewest bytes
    indefinite-length     §5.1  additional-info 31 (streaming) forbidden
    unsorted-map-keys     §5.6  entries not bytewise-ascending by encoded key
    duplicate-map-keys    §5.6  two byte-equal encoded keys
    float-not-float64     §5.3  half (ai 25) / single (ai 26) float forbidden
    negative-zero-float   §5.3  fb8000000000000000 forbidden
    non-finite-float      §5.3  NaN / +Inf / -Inf forbidden
    trailing-data         §5.9  bytes remain after the one top-level item
    reserved-or-unsupported §4  tags (major 6), undefined (0xf7), reserved simples,
                                and any major/ai not in the §4 value model
    truncated             (parse) input ends mid-item; the item is incomplete
    non-nfc-text          §5.4  text octets are valid UTF-8 but not NFC

The value model (§4) is the ONLY thing that may appear in a canonical body:
Null, Bool, Integer, Float, Text, Bytes, Array, Map. Anything else -> rejected.
"""

import struct
import unicodedata

# Reuse our OWN independent encoder's value wrappers and encoder so the decoder
# produces values that round-trip through the encoder we already wrote/verified.
# (Permitted by the independence rules: reuse of our own Python only.)
from acs002_dcbor import AInt, AFloat, encode


class Rejected(Exception):
    """
    A typed rejection carrying the stable reason code (ACS-002 §5 / CONFORMANCE.md).
    `reason` is exactly one of the reason-code strings listed above.
    """
    __slots__ = ("reason", "detail")

    def __init__(self, reason, detail=""):
        self.reason = reason
        self.detail = detail
        super().__init__("%s%s" % (reason, (": " + detail) if detail else ""))


# Additional-information sentinels (RFC 8949 §3, as profiled by ACS-002 §5).
_AI_1BYTE = 24
_AI_2BYTE = 25
_AI_4BYTE = 26
_AI_8BYTE = 27
_AI_RESERVED_28 = 28   # RFC 8949: 28-30 reserved
_AI_RESERVED_29 = 29
_AI_RESERVED_30 = 30
_AI_INDEFINITE = 31    # ACS-002 §5.1: MUST be rejected


class _Reader:
    """A cursor over the input bytes. Raises `truncated` when it runs off the end."""
    __slots__ = ("buf", "pos")

    def __init__(self, buf):
        self.buf = buf
        self.pos = 0

    def take(self, n):
        # (parse rule) reading past the end means the item is incomplete -> truncated.
        if self.pos + n > len(self.buf):
            raise Rejected("truncated",
                           "need %d bytes at offset %d, have %d"
                           % (n, self.pos, len(self.buf) - self.pos))
        chunk = self.buf[self.pos:self.pos + n]
        self.pos += n
        return chunk

    def take1(self):
        return self.take(1)[0]

    def remaining(self):
        return len(self.buf) - self.pos


def _read_argument(r, ai):
    """
    Read the CBOR argument for additional-info `ai` and ENFORCE shortest form.

    ACS-002 §5.2 (integers) and §5.1/§5.2 (lengths): the argument SHALL use the
    fewest bytes (inline 0..23, then 1, 2, 4, or 8 bytes). A longer-than-necessary
    encoding is non-canonical and MUST be rejected. `is_len=False` -> non-shortest-int;
    the caller re-labels as non-shortest-len for lengths (see _shortest_check).

    Returns (value:int, ai:int) so the caller can decide the reason label.
    """
    if ai <= 23:
        return ai, ai
    if ai == _AI_1BYTE:
        v = r.take1()
        return v, ai
    if ai == _AI_2BYTE:
        v = struct.unpack(">H", r.take(2))[0]
        return v, ai
    if ai == _AI_4BYTE:
        v = struct.unpack(">I", r.take(4))[0]
        return v, ai
    if ai == _AI_8BYTE:
        v = struct.unpack(">Q", r.take(8))[0]
        return v, ai
    if ai == _AI_INDEFINITE:
        # §5.1: indefinite-length / streaming forbidden. Handled by callers too,
        # but any use of ai 31 as an argument is an indefinite marker.
        raise Rejected("indefinite-length",
                       "additional-info 31 (indefinite/streaming) is forbidden (§5.1)")
    # ai 28,29,30 are RESERVED by RFC 8949 and are not in the §4 value model.
    raise Rejected("reserved-or-unsupported",
                   "reserved additional-info %d (§4)" % ai)


def _shortest_argument_ai(value):
    """
    The additional-info that a canonical encoder MUST use for `value` (§5.2):
    fewest bytes. Used to detect non-shortest encodings.
    """
    if value <= 23:
        return value            # inline
    if value <= 0xFF:
        return _AI_1BYTE
    if value <= 0xFFFF:
        return _AI_2BYTE
    if value <= 0xFFFFFFFF:
        return _AI_4BYTE
    return _AI_8BYTE            # up to 2^64-1


def _check_shortest(value, ai, int_reason):
    """
    ACS-002 §5.2: reject a longer-than-necessary argument encoding.
    `int_reason` is 'non-shortest-int' for integer values or 'non-shortest-len'
    for length prefixes (§5.1) — the spec distinguishes the two reason codes.
    """
    expected = _shortest_argument_ai(value)
    # For inline values, expected == value (<=23). For multi-byte, expected is a
    # sentinel 24..27. `ai` is the actual sentinel used (>=24) or the inline value.
    if ai <= 23:
        actual_rank = ai              # inline: rank is the value itself, always minimal
        # inline is always shortest; nothing longer can also be inline. OK.
        return
    # ai is a 24..27 sentinel. Compare byte-width against the minimum needed.
    width_of = {_AI_1BYTE: 1, _AI_2BYTE: 2, _AI_4BYTE: 4, _AI_8BYTE: 8}
    min_ai = expected
    if width_of[ai] > width_of.get(min_ai, 0) or (min_ai <= 23 and value <= 23):
        # Either a wider sentinel than needed, OR value fits inline but a sentinel
        # was used (e.g. 0x1800 encodes 0 with a 1-byte arg — non-shortest).
        raise Rejected(int_reason,
                       "value %d encoded with ai %d, shortest is ai %s"
                       % (value, ai, min_ai))


# ---------------------------------------------------------------------------
# Float decoding (§5.3): floats are ALWAYS float64 in a canonical body.
# ---------------------------------------------------------------------------

def _decode_float64_payload(r):
    """
    Decode a major-7 / ai-27 (0xfb) IEEE-754 binary64 and enforce §5.3:
    reject NaN / +-Inf (non-finite-float) and -0.0 (negative-zero-float).
    """
    raw = r.take(8)
    (x,) = struct.unpack(">d", raw)
    # §5.3: NaN, +Inf, -Inf have no canonical form -> reject.
    if x != x:  # NaN
        raise Rejected("non-finite-float", "NaN has no canonical form (§5.3)")
    if x == float("inf") or x == float("-inf"):
        raise Rejected("non-finite-float", "Infinity has no canonical form (§5.3)")
    # §5.3: -0.0 MUST be rejected on decode (canonical zero is fb0000000000000000).
    if raw == b"\x80\x00\x00\x00\x00\x00\x00\x00":
        raise Rejected("negative-zero-float",
                       "-0.0 is non-canonical; canonical zero is fb0000000000000000 (§5.3)")
    return AFloat(x)


# ---------------------------------------------------------------------------
# Core recursive decode of one data item. Returns a (value, key_encoding) where
# key_encoding is the raw canonical key bytes IF this item is used as a map key,
# else None. We recompute key encodings via our own encoder for the sort check.
# ---------------------------------------------------------------------------

def _decode_item(r):
    """Decode exactly one ARVES value (§4) at the cursor, validating §5 inline."""
    ib = r.take1()                       # initial byte
    major = ib >> 5
    ai = ib & 0x1F

    # ---- major 0: unsigned integer (§4 kind 3, §5.2) ----
    if major == 0:
        if ai == _AI_INDEFINITE:
            raise Rejected("indefinite-length", "integers cannot be indefinite (§5.1)")
        value, used_ai = _read_argument(r, ai)
        _check_shortest(value, used_ai, "non-shortest-int")  # §5.2
        return AInt(value)

    # ---- major 1: negative integer (§4 kind 3, §5.2), value = -1 - arg ----
    if major == 1:
        if ai == _AI_INDEFINITE:
            raise Rejected("indefinite-length", "integers cannot be indefinite (§5.1)")
        arg, used_ai = _read_argument(r, ai)
        _check_shortest(arg, used_ai, "non-shortest-int")    # §5.2
        return AInt(-1 - arg)

    # ---- major 2: byte string (§4 kind 6, §5.5) ----
    if major == 2:
        if ai == _AI_INDEFINITE:
            raise Rejected("indefinite-length",
                           "byte strings SHALL be definite-length (§5.1)")
        length, used_ai = _read_argument(r, ai)
        _check_shortest(length, used_ai, "non-shortest-len")  # §5.1
        return r.take(length)                                 # opaque octets (§5.5)

    # ---- major 3: text string (§4 kind 5, §5.4) ----
    if major == 3:
        if ai == _AI_INDEFINITE:
            raise Rejected("indefinite-length",
                           "text strings SHALL be definite-length (§5.1)")
        length, used_ai = _read_argument(r, ai)
        _check_shortest(length, used_ai, "non-shortest-len")  # §5.1
        octets = r.take(length)                               # may raise truncated
        # §5.4: text is UTF-8; a canonical body's text is NFC. Reject non-UTF-8
        # (not in the model) and non-NFC (non-nfc-text).
        try:
            s = octets.decode("utf-8")
        except UnicodeDecodeError:
            raise Rejected("reserved-or-unsupported",
                           "text is not valid UTF-8 (§5.4)")
        # NFC check: Python stdlib HAS unicodedata, so we ENFORCE §5.4 here. A
        # dependency-free implementation MAY defer THIS ONE rule (CONFORMANCE.md
        # nfc-tier) — we do not defer it.
        if unicodedata.normalize("NFC", s) != s:
            raise Rejected("non-nfc-text",
                           "text octets are not NFC-normalized (§5.4)")
        return s

    # ---- major 4: array (§4 kind 7, §5.8) ----
    if major == 4:
        if ai == _AI_INDEFINITE:
            raise Rejected("indefinite-length",
                           "arrays SHALL be definite-length (§5.1)")
        n, used_ai = _read_argument(r, ai)
        _check_shortest(n, used_ai, "non-shortest-len")       # §5.1
        items = []
        for _ in range(n):
            items.append(_decode_item(r))                     # order preserved (§5.8)
        return items

    # ---- major 5: map (§4 kind 8, §5.6) ----
    if major == 5:
        if ai == _AI_INDEFINITE:
            raise Rejected("indefinite-length",
                           "maps SHALL be definite-length (§5.1)")
        n, used_ai = _read_argument(r, ai)
        _check_shortest(n, used_ai, "non-shortest-len")       # §5.1
        return _decode_map_entries(r, n)

    # ---- major 6: tag — NOT in the §4 value model; RESERVED (§4) ----
    if major == 6:
        raise Rejected("reserved-or-unsupported",
                       "CBOR tags (major 6) are not in the ACS-002 value model; "
                       "the tag space is RESERVED (§4)")

    # ---- major 7: simple values / floats (§4 kinds 1,2,4) ----
    if major == 7:
        # Simple values by additional-info:
        if ai == 20:                       # false
            return False                   # §4 kind 2
        if ai == 21:                       # true
            return True                    # §4 kind 2
        if ai == 22:                       # null (0xf6)
            return None                    # §4 kind 1
        if ai == 23:                       # undefined (0xf7)
            raise Rejected("reserved-or-unsupported",
                           "the CBOR 'undefined' simple value (0xf7) is not in the "
                           "ACS-002 value model (§4)")
        if ai == _AI_1BYTE:
            # simple value in one byte (0xf8 nn): values 0..255. None are in §4.
            r.take1()
            raise Rejected("reserved-or-unsupported",
                           "one-byte CBOR simple value is not in the ACS-002 model (§4)")
        if ai == _AI_2BYTE:                # 0xf9 half-precision float
            r.take(2)
            raise Rejected("float-not-float64",
                           "half-precision float (ai 25) forbidden; floats are "
                           "always binary64 (§5.3)")
        if ai == _AI_4BYTE:                # 0xfa single-precision float
            r.take(4)
            raise Rejected("float-not-float64",
                           "single-precision float (ai 26) forbidden; floats are "
                           "always binary64 (§5.3)")
        if ai == _AI_8BYTE:                # 0xfb binary64 float (§5.3)
            return _decode_float64_payload(r)
        if ai == _AI_INDEFINITE:           # 0xff break — only valid inside indefinite
            raise Rejected("indefinite-length",
                           "unexpected 'break' (0xff): indefinite encodings are "
                           "forbidden (§5.1)")
        # ai 24 handled above; ai 0..19 are unassigned/reserved simple values.
        raise Rejected("reserved-or-unsupported",
                       "reserved/unassigned simple value (major 7, ai %d) is not in "
                       "the ACS-002 value model (§4)" % ai)

    # Unreachable: major is 0..7.
    raise Rejected("reserved-or-unsupported", "unknown major type %d" % major)


def _decode_map_entries(r, n):
    """
    Decode `n` map entries and enforce ACS-002 §5.6:
      - keys are Text or Integer (§4 kind 8);
      - entries SHALL be sorted by the BYTEWISE lexicographic order of each key's
        own canonical (dCBOR) encoding;
      - no two encoded keys may be byte-equal (duplicate).
    We recompute each key's canonical encoding with our OWN encoder and compare it
    to the incoming key bytes to detect both ordering and duplication.
    """
    out = {}
    prev_key_enc = None
    seen = set()
    for _ in range(n):
        # A key is itself a full data item; decode it (this also validates the key
        # is canonical, e.g. a non-shortest integer key is rejected here).
        key = _decode_item(r)
        # §4 kind 8: map keys MUST be Text or Integer.
        if isinstance(key, str):
            key_enc = encode(key)
            hkey = ("T", key)
        elif isinstance(key, AInt):
            key_enc = encode(key)
            hkey = ("I", key.v)
        else:
            raise Rejected("reserved-or-unsupported",
                           "map key must be Text or Integer (§4 kind 8)")
        # §5.6: duplicate encoded keys.
        if key_enc in seen:
            raise Rejected("duplicate-map-keys",
                           "duplicate encoded map key %s (§5.6)" % key_enc.hex())
        # §5.6: bytewise-ascending order. Because keys are canonically encoded and
        # each is unique, strict-ascending is the total order.
        if prev_key_enc is not None and key_enc <= prev_key_enc:
            raise Rejected("unsorted-map-keys",
                           "map key %s not bytewise-greater than previous %s (§5.6)"
                           % (key_enc.hex(), prev_key_enc.hex()))
        seen.add(key_enc)
        prev_key_enc = key_enc
        val = _decode_item(r)
        out[hkey] = val
    return _MapValue(out)


class _MapValue(dict):
    """
    Decoded Map. Keys are ('T', text) or ('I', int) tuples so Text 1 and Integer 1
    never collide (they are distinct §4 kinds). Round-trips via `to_encoder_map`.
    """

    def to_encoder_map(self):
        """Rebuild a dict our encoder accepts (str keys, AInt keys)."""
        m = {}
        for (kind, k), v in self.items():
            if kind == "T":
                m[k] = _to_encoder_value(v)
            else:  # "I"
                m[AInt(k)] = _to_encoder_value(v)
        return m


def _to_encoder_value(v):
    """Map a decoded value back to something acs002_dcbor.encode accepts."""
    if isinstance(v, _MapValue):
        return v.to_encoder_map()
    if isinstance(v, list):
        return [_to_encoder_value(x) for x in v]
    # AInt, AFloat, str, bytes, bool, None pass through unchanged.
    return v


def decode(buf):
    """
    Decode a canonical ACS-002/1 body and validate canonical form (§5, §6.3).

    Returns the decoded ARVES value. Integers are AInt, Floats are AFloat, Text is
    str, Bytes is bytes, Bool is Python bool, Null is None, Array is list, Map is a
    _MapValue (dict subclass). Raises `Rejected(reason, detail)` on any
    non-canonical input (§3, §5, §6.3).

    §5.9: after the single top-level item, ANY remaining byte is trailing data.
    """
    if not isinstance(buf, (bytes, bytearray)):
        raise TypeError("decode expects bytes")
    r = _Reader(bytes(buf))
    value = _decode_item(r)
    if r.remaining() != 0:
        # §5.9: exactly one top-level item, no trailing octets.
        raise Rejected("trailing-data",
                       "%d trailing byte(s) after the top-level item (§5.9)"
                       % r.remaining())
    return value


def reencode(value):
    """
    Re-encode a value returned by `decode` using our own canonical encoder, to
    verify the round-trip / idempotency obligation (§6.2): canon(decode(b)) == b.
    """
    return encode(_to_encoder_value(value))
