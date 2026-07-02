"""
Logical ARVES values for each golden vector, DERIVED FROM THE ACS SPEC PROSE
(not reverse-engineered from the target hex).

Each builder cites the exact ACS section that fixes the value it constructs.
The conformance runner encodes these with the independent ACS-002 serializer and
addresses them with the independent ACS-001 addresser, then compares to the TSV.
"""

from acs002_dcbor import AInt, AFloat


# ---------------------------------------------------------------------------
# ACS-001 — raw-byte bodies (ACS-001 §7; no dCBOR encoding step, CONFORMANCE.md).
# For ACS-001 the "body" is raw bytes given by the spec's §7 table (utf-8 col).
# ---------------------------------------------------------------------------

def acs001_hello_truth_body():
    # §7 vector 1, domain 0x01: body (utf-8) = "hello-truth"
    return b"hello-truth"


def acs001_engine_manifest_body():
    # §7 vector 2, domain 0x02: body (utf-8) = {"engine":"summarize","version":"1"}
    return b'{"engine":"summarize","version":"1"}'


def acs001_invocation_body():
    # §7 vector 3, domain 0x04: body (utf-8) = acme/research|c1|hello-truth
    return b"acme/research|c1|hello-truth"


# ---------------------------------------------------------------------------
# ACS-002 — structured dCBOR bodies (ACS-002 §8.1). Values from the prose.
# ---------------------------------------------------------------------------

def acs002_v1_fact():
    # §8.1 V1, domain 0x01: Map { "type": "uci.fact", "claim": "sky-is-blue",
    #   "confidence": Float 0.5, "observed_at": Integer 1730000000000000000 }
    # (author key order irrelevant; encoder sorts by encoded-key bytes §5.6).
    return {
        "type": "uci.fact",
        "claim": "sky-is-blue",
        "confidence": AFloat(0.5),
        "observed_at": AInt(1730000000000000000),
    }


def acs002_v2_engine_manifest():
    # §8.1 V2, domain 0x02: Map { "engine": "summarize", "version": Integer 1,
    #   "deterministic": true, "reads": [ "uci.observation", "uci.fact" ],
    #   "seed": Null }
    return {
        "engine": "summarize",
        "version": AInt(1),
        "deterministic": True,
        "reads": ["uci.observation", "uci.fact"],
        "seed": None,
    }


def acs002_v3_nfc_neg():
    # §8.1 V3, domain 0x05: Map { "label": "Amélie é—中", "n": Integer -1000 }
    # label supplied in NFD form to prove NFC normalization (§5.4).
    # "Amélie é—中": we deliberately author the NFD (decomposed) form so the
    # encoder's NFC step is exercised (spec: NFD and NFC inputs MUST both yield
    # the identical body).
    import unicodedata
    label_precomposed = "Amélie é—中"   # é = U+00E9, — = U+2014, 中 = U+4E2D
    label_nfd = unicodedata.normalize("NFD", label_precomposed)
    assert label_nfd != label_precomposed, "expected NFD to differ, to exercise §5.4"
    return {
        "label": label_nfd,
        "n": AInt(-1000),
    }


# ---------------------------------------------------------------------------
# ACS-003 — canonical envelope (ACS-003 §10.2). 12 fields.
# ---------------------------------------------------------------------------

def acs003_envelope(payload_cid_bytes):
    # §10.2 field values, verbatim from the spec.
    # payload_cid is carried as Bytes (the 34-byte multihash), §5 / §10.2.
    return {
        "ser_version": "ACS-002/1",                          # Text §5
        "event_id": "urn:arves:evt:01J8ZK9M4Q2N7C3F",        # Text §5
        "event_type": "information.fact.committed",          # Text §5
        "tenant_id": "acme",                                 # Text §5
        "workspace_id": "research",                          # Text §5
        "correlation_id": "urn:arves:corr:c1",               # Text §5
        "causation_id": None,                                # Null present (§5.7 / §10.2)
        "source": "urn:arves:svc:information-core",          # Text §5
        "occurred_at": AInt(1730000000000000000),            # Integer i64 ns §5/§5.1
        "schema_version": AInt(1),                           # Integer §5
        "payload_domain": AInt(1),                           # Integer (ACS-001 domain) §5
        "payload_cid": bytes(payload_cid_bytes),             # Bytes(34) §5 / §10.2
    }


# ---------------------------------------------------------------------------
# ACS-004 — uci.fact@1.0 instance (ACS-004 §11.3) and schema doc (§11.2).
# ---------------------------------------------------------------------------

def acs004_instance():
    # §11.3 observed uci.fact@1.0 instance, domain 0x01 (commit-content, §10).
    # origin == "observed" => invocation ABSENT (§8); evidence is Array(0..*).
    return {
        "urn": "urn:arves:uci.core:fact@1.0:f-1730000000",   # urn §11.3
        "tenant": "acme",                                    # text
        "workspace": "research",                             # text
        "origin": "observed",                                # text (Origin variant §8)
        "source": "sensor-array-7",                          # text
        "confidence": AFloat(0.98),                          # conf (float64) §7
        "valid_from": AInt(1730000000000000000),             # int i64 ns §7
        "recorded_at": AInt(1730000000500000000),            # int i64 ns §7
        "claim": "sky-is-blue",                              # text
        "observed_at": AInt(1730000000000000000),            # int i64 ns §7
        "evidence": ["urn:arves:uci.core:evidence@1.0:e-42"],  # Array of urn (0..*) §6.4
        # invocation ABSENT (origin == "observed", §8).
    }


def acs004_schema_document():
    # §11.2 uci.fact@1.0 schema document (authoritative form, §3/§6).
    # Present for completeness/verification; NOT one of the 11 TSV rows.
    def fd(type_code, card):
        return {"type": type_code, "card": card}   # field descriptor §6.2

    return {
        "urn": "uci.fact",                                   # Text §6
        "ver": {"major": AInt(1), "minor": AInt(0)},         # Map{major:Int,minor:Int} §6
        "root": "Fact",                                      # Text (RootType §6.1)
        "aspects": ["Identity", "Provenance", "Temporal", "Trust", "TenantScope"],  # Array<Text> §6/§11.2
        "fields": {                                          # Map<Text, field descriptor> §6
            "urn": fd("urn", "1"),
            "tenant": fd("text", "1"),
            "workspace": fd("text", "1"),
            "origin": fd("text", "1"),
            "source": fd("text", "1"),
            "invocation": fd("urn", "0..1"),
            "confidence": fd("conf", "1"),
            "valid_from": fd("int", "1"),
            "recorded_at": fd("int", "1"),
            "claim": fd("text", "1"),
            "observed_at": fd("int", "1"),
            "evidence": fd("urn", "0..*"),
        },
    }


# ---------------------------------------------------------------------------
# ACS-005 — raw-byte bodies (ACS-005 §8, §9.2). No dCBOR (raw text bodies).
# ---------------------------------------------------------------------------

def acs005_term_set_body():
    # §8: body = Term IDs GL-001..GL-014, sorted ascending, LF-joined, no trailing LF.
    ids = ["GL-%03d" % i for i in range(1, 15)]   # GL-001 .. GL-014
    return "\n".join(ids).encode("utf-8")


def acs005_requirement_body():
    # §9.2 vector 2: the EXACT clause text pinned by the vector table (the clause
    # form used for the byte-exact pre-image; lower-case "cognitive truth", no GL refs).
    text = ("ORCH-001-R1: The Control Plane MUST NOT own cognitive truth; "
            "only the Kernel MAY own cognitive truth.")
    return text.encode("utf-8")


def acs005_term_names_body():
    # §9.1 the 14 capitalized normative terms, LF-joined, SORTED, no trailing LF (§9.2 v3).
    terms = [
        "Capability", "Cognitive Entity", "Cognitive Truth", "Commit",
        "Conformance", "Content Address", "Control Plane", "Data Plane",
        "Decision Trace", "Engine", "Kernel", "Replay", "Shard", "Tenant",
    ]
    # §9.2 v3 says "the §9.1 list, LF-joined, sorted". Sort to be faithful to the
    # stated procedure rather than relying on §9.1's authored order.
    terms_sorted = sorted(terms)
    return "\n".join(terms_sorted).encode("utf-8")
