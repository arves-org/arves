"""
ARVES Conformance runner — ACS layer (independent Python implementation).

Follows standard/conformance/CONFORMANCE.md:
  For each row (standard, vector, domain, body_hex, content_id):
    1. Encoder conformance: from the logical value described in the ACS spec,
       produce the canonical body; assert hex(body) == body_hex.
       (ACS-001/005 bodies are raw bytes, given directly; no encoding step.)
    2. Addresser conformance: ContentId = 0x12 0x20 || SHA256(domain || body);
       assert hex(ContentId) == content_id.
An implementation is ACS-conformant iff every row passes BOTH checks.

The logical values come from acs_values.py, each cited to an ACS section, and are
encoded by our own acs002_dcbor / addressed by our own acs001_address — nothing is
derived from the target hex.
"""

import os
import sys

from acs001_address import content_id
from acs002_dcbor import encode
import acs_values as V


HERE = os.path.dirname(os.path.abspath(__file__))
# standard/vectors/acs_golden_vectors.tsv relative to this file.
TSV = os.path.normpath(os.path.join(
    HERE, "..", "..", "..", "standard", "vectors", "acs_golden_vectors.tsv"))


class KitGap(Exception):
    """Raised when the Kit does not let us determine the logical value/body."""


def build_body(standard, vector, domain):
    """
    Return the canonical `body` bytes for a TSV row, built from the ACS spec prose.
    ACS-001 and ACS-005 bodies are raw bytes; ACS-002/003/004 bodies are dCBOR.
    """
    key = (standard, vector)

    # ---- ACS-001: raw-byte bodies (§7) ----
    if key == ("ACS-001", "hello-truth"):
        return V.acs001_hello_truth_body()
    if key == ("ACS-001", "engine-manifest"):
        return V.acs001_engine_manifest_body()
    if key == ("ACS-001", "invocation"):
        return V.acs001_invocation_body()

    # ---- ACS-002: dCBOR bodies (§8.1) ----
    if key == ("ACS-002", "V1 uci.fact"):
        return encode(V.acs002_v1_fact())
    if key == ("ACS-002", "V2 engine-manifest"):
        return encode(V.acs002_v2_engine_manifest())
    if key == ("ACS-002", "V3 nfc+neg"):
        return encode(V.acs002_v3_nfc_neg())

    # ---- ACS-003: envelope (§10.2). payload_cid = ACS-001 addr of V1 fact under 0x01 ----
    if key == ("ACS-003", "envelope"):
        payload_body = encode(V.acs002_v1_fact())          # ACS-002 V1 body (§10.1)
        payload_cid = content_id(0x01, payload_body)       # domain 0x01 commit-content (§10.1)
        return encode(V.acs003_envelope(payload_cid))

    # ---- ACS-004: uci.fact@1.0 instance (§11.3) ----
    if key == ("ACS-004", "uci.fact instance"):
        return encode(V.acs004_instance())
    if key == ("ACS-004", "schema"):
        # ACS-004 §11.2 schema document (domain 0x07); present in the .md table but
        # NOT a row in the TSV. Kept so the runner covers it if the TSV adds it.
        return encode(V.acs004_schema_document())

    # ---- ACS-005: raw-byte bodies (§8/§9.2) ----
    if key == ("ACS-005", "term-set"):
        return V.acs005_term_set_body()
    if key == ("ACS-005", "requirement"):
        return V.acs005_requirement_body()
    if key == ("ACS-005", "term-names"):
        return V.acs005_term_names_body()

    raise KitGap("no spec-derived builder for row (%s, %s)" % (standard, vector))


def load_rows():
    rows = []
    with open(TSV, "r", encoding="utf-8") as f:
        header = f.readline().rstrip("\n").split("\t")
        assert header == ["standard", "vector", "domain", "body_hex", "content_id"], \
            "unexpected TSV header: %r" % (header,)
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            parts = line.split("\t")
            standard, vector, domain_s, body_hex, cid = parts
            rows.append({
                "standard": standard,
                "vector": vector,
                "domain": int(domain_s, 16),   # e.g. "0x01" -> 1
                "body_hex": body_hex.lower(),
                "content_id": cid.lower(),
            })
    return rows


def run():
    rows = load_rows()
    # Group ordering for the report: preserve first-seen standard order.
    order = []
    grouped = {}
    for r in rows:
        s = r["standard"]
        if s not in grouped:
            grouped[s] = []
            order.append(s)
        grouped[s].append(r)

    results = {}       # standard -> list of (row, ok, detail)
    total_pass = 0
    total = 0

    for s in order:
        results[s] = []
        for r in grouped[s]:
            total += 1
            ok = True
            detail = ""
            try:
                body = build_body(r["standard"], r["vector"], r["domain"])
                got_body_hex = body.hex()
                cid = content_id(r["domain"], body)
                got_cid_hex = cid.hex()

                if got_body_hex != r["body_hex"]:
                    ok = False
                    detail = ("body mismatch\n      expected=%s\n      got     =%s"
                              % (r["body_hex"], got_body_hex))
                elif got_cid_hex != r["content_id"]:
                    ok = False
                    detail = ("ContentId mismatch\n      expected=%s\n      got     =%s"
                              % (r["content_id"], got_cid_hex))
            except KitGap as e:
                ok = False
                detail = "KIT GAP: %s" % e
            except Exception as e:  # pragma: no cover - surfaced as FAIL detail
                ok = False
                detail = "ERROR: %s: %s" % (type(e).__name__, e)

            if ok:
                total_pass += 1
            results[s].append((r, ok, detail))

    # ---- Emit the report exactly as CONFORMANCE.md prescribes ----
    print("ARVES Conformance Report — ACS layer")
    all_pass = True
    for s in order:
        rs = results[s]
        n_ok = sum(1 for (_, ok, _) in rs if ok)
        n = len(rs)
        verdict = "PASS" if n_ok == n else "FAIL"
        if n_ok != n:
            all_pass = False
        print("  %s %s (%d/%d)" % (s, verdict, n_ok, n))
        # Show detail lines for any failures.
        for (r, ok, detail) in rs:
            if not ok:
                print("    - %s / %s  FAIL: %s" % (r["standard"], r["vector"], detail))
    print("  ACS golden vectors: %d/%d PASS" % (total_pass, total))
    verdict = "CONFORMANT" if all_pass and total_pass == total else "NON-CONFORMANT"
    print("VERDICT: %s (ACS layer)" % verdict)

    return 0 if (all_pass and total_pass == total) else 1


if __name__ == "__main__":
    sys.exit(run())
