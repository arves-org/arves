"""
ACS-002 / CCP-002 — ARVES Canonical Serialization (deterministic CBOR profile).

Independent implementation from the ARVES Standard Kit ONLY
(standard/acs/ACS-002_Canonical_Serialization.md). No reference-runtime source
was consulted.

Value model (ACS-002 §4): Null, Bool, Integer, Float, Text, Bytes, Array, Map.
Integer and Float are DISTINCT kinds and MUST NOT be conflated (§4).

Because Python's bool is a subclass of int and its int/float are ambiguous, this
module uses explicit wrapper types so the caller (and the ACS specs' prose) can
state the value-model kind unambiguously, exactly as the spec demands:
    - AInt(n)   -> ACS-002 Integer  (§5.2 shortest form)
    - AFloat(x) -> ACS-002 Float    (§5.3 fixed binary64, finite, +0.0 only)
Text is a Python str (NFC-normalized here, §5.4); Bytes is a Python bytes/bytearray
(§5.5); Bool is a Python bool; Null is Python None; Array is a list; Map is a dict.
"""

import struct
import unicodedata


class AInt:
    """ACS-002 Integer value (value-model kind 3)."""
    __slots__ = ("v",)

    def __init__(self, v):
        if isinstance(v, bool) or not isinstance(v, int):
            raise TypeError("AInt requires a Python int (not bool)")
        # §4: signed integer in [-2^64, 2^64 - 1] (CBOR majors 0/1).
        if not (-(2 ** 64) <= v <= (2 ** 64) - 1):
            raise ValueError("Integer out of ACS-002 §4 range")
        self.v = v


class AFloat:
    """ACS-002 Float value (value-model kind 4): IEEE-754 binary64, finite."""
    __slots__ = ("v",)

    def __init__(self, v):
        self.v = float(v)


def _encode_head(major, arg):
    """
    Encode a CBOR head (major type + argument) in SHORTEST additional-info form
    (ACS-002 §5.2 for integers; reused for all length-prefixed items §5.1).
    """
    mt = major << 5
    if arg < 0:
        raise ValueError("argument must be non-negative")
    if arg <= 23:
        return bytes([mt | arg])
    elif arg <= 0xFF:
        return bytes([mt | 24, arg])
    elif arg <= 0xFFFF:
        return bytes([mt | 25]) + struct.pack(">H", arg)
    elif arg <= 0xFFFFFFFF:
        return bytes([mt | 26]) + struct.pack(">I", arg)
    elif arg <= 0xFFFFFFFFFFFFFFFF:
        return bytes([mt | 27]) + struct.pack(">Q", arg)
    else:
        raise ValueError("argument exceeds 64 bits (out of ACS-002 model)")


def _encode_int(n):
    """ACS-002 §5.2: non-negative -> major 0; negative -> major 1 (arg = -1 - n)."""
    if n >= 0:
        return _encode_head(0, n)
    else:
        return _encode_head(1, -1 - n)


def _encode_float(x):
    """
    ACS-002 §5.3: 64-bit IEEE-754 double, network byte order, major 7 / ai 27
    (initial byte 0xfb). NaN/+Inf/-Inf rejected. -0.0 normalized to +0.0.
    """
    # Reject non-finite (§5.3). float('inf')/nan check without importing math.
    if x != x:  # NaN
        raise ValueError("ACS-002 §5.3: NaN has no canonical form; rejected")
    if x in (float("inf"), float("-inf")):
        raise ValueError("ACS-002 §5.3: Infinity rejected")
    # Normalize negative zero to positive zero (§5.3).
    if x == 0.0:
        x = 0.0  # this yields +0.0 in Python for the '== 0.0' branch below
        # struct.pack of 0.0 -> fb0000000000000000 ; ensure not -0.0:
        if struct.pack(">d", x) == b"\x80\x00\x00\x00\x00\x00\x00\x00":
            x = 0.0
    packed = struct.pack(">d", x)
    if packed == b"\x80\x00\x00\x00\x00\x00\x00\x00":
        # -0.0 slipped through -> normalize
        packed = b"\x00\x00\x00\x00\x00\x00\x00\x00"
    return b"\xfb" + packed


def _encode_text(s):
    """ACS-002 §5.4: NFC-normalize, then UTF-8 octets, major 3, definite length."""
    nfc = unicodedata.normalize("NFC", s)
    octets = nfc.encode("utf-8")
    return _encode_head(3, len(octets)) + octets


def _encode_bytes(b):
    """ACS-002 §5.5: major 2, definite length, octets verbatim (NOT normalized)."""
    b = bytes(b)
    return _encode_head(2, len(b)) + b


def _encode_array(items):
    """ACS-002 §5.8: major 4, definite length, element order preserved verbatim."""
    out = _encode_head(4, len(items))
    for it in items:
        out += encode(it)
    return out


def _encode_map(d):
    """
    ACS-002 §5.6: major 5, definite length; entries sorted by the BYTEWISE
    lexicographic order of each key's own canonical dCBOR encoding; duplicate
    encoded keys rejected. Keys must be Text or Integer (§4 kind 8).
    """
    encoded_entries = []
    seen = set()
    for k, v in d.items():
        # Keys are Text (str) or Integer (AInt). §4: "every key is a Text or an Integer".
        if isinstance(k, AInt):
            ekey = _encode_int(k.v)
        elif isinstance(k, str):
            ekey = _encode_text(k)
        elif isinstance(k, int) and not isinstance(k, bool):
            # Convenience: a bare python int key is treated as an Integer key.
            ekey = _encode_int(k)
        else:
            raise TypeError("Map key must be Text or Integer (ACS-002 §4)")
        if ekey in seen:
            raise ValueError("ACS-002 §5.6: duplicate map key rejected")
        seen.add(ekey)
        encoded_entries.append((ekey, encode(v)))
    # Bytewise sort by encoded key.
    encoded_entries.sort(key=lambda p: p[0])
    out = _encode_head(5, len(encoded_entries))
    for ekey, eval_ in encoded_entries:
        out += ekey + eval_
    return out


def encode(value):
    """
    Canonicalize an ARVES value to its ACS-002/1 canonical body (§5).
    Dispatch is by value-model kind. Integer and Float are DISTINCT (§4): the
    caller MUST wrap them in AInt / AFloat so the kind is explicit and can never
    be inferred by accident.
    """
    if value is None:
        return b"\xf6"                      # §4 kind 1: Null
    if isinstance(value, bool):
        return b"\xf5" if value else b"\xf4"  # §4 kind 2: Bool
    if isinstance(value, AInt):
        return _encode_int(value.v)         # §4 kind 3: Integer
    if isinstance(value, AFloat):
        return _encode_float(value.v)       # §4 kind 4: Float
    if isinstance(value, str):
        return _encode_text(value)          # §4 kind 5: Text
    if isinstance(value, (bytes, bytearray)):
        return _encode_bytes(value)         # §4 kind 6: Bytes
    if isinstance(value, list):
        return _encode_array(value)         # §4 kind 7: Array
    if isinstance(value, dict):
        return _encode_map(value)           # §4 kind 8: Map
    # A bare Python int/float is deliberately NOT accepted, to enforce the
    # Integer/Float distinction of §4 (no silent inference).
    raise TypeError(
        "Value is not an ACS-002 value-model kind; wrap ints as AInt and "
        "floats as AFloat (ACS-002 §4)"
    )
