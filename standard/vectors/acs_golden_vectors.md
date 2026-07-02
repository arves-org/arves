# ACS Golden Vectors — differential-conformance targets (language-neutral)

The byte-exact `ContentId`s any conformant ARVES implementation MUST reproduce
from the frozen spec + ratified ACS alone. `ContentId = 0x12 0x20 ‖ SHA-256(domain_tag ‖ body)`;
bodies are ACS-002/1 deterministic CBOR (except ACS-001/ACS-005 raw-byte bodies).
The Rust reference (`arves-acs`) asserts all of these (tests: `acs_001_cs_1`,
`acs_002_cs_1_*`, `acs_platform`); a second-language runtime (Go, …) must match
this table — that match is the Independent-Runtime proof.

| Standard | Domain | Input (summary) | ContentId |
|----------|--------|-----------------|-----------|
| ACS-001-CS-1 #1 | 0x01 | raw `hello-truth` | `122056e30f71852b0e4c253cf05dab6be2bb5b8470ac878a52f10c5af2a40d69b76e` |
| ACS-001-CS-1 #2 | 0x02 | raw `{"engine":"summarize","version":"1"}` | `12205c631bd808332b0889763100ad7458710c137320381e3b4ea9cce3c0640a4e54` |
| ACS-001-CS-1 #3 | 0x04 | raw `acme/research\|c1\|hello-truth` | `1220ae7a70002ef6dd81018d4715a986dae6dfdc1b7bc85acdd66698875f2fe302bc` |
| ACS-002-CS-1 V1 | 0x01 | dCBOR map `uci.fact` {type,claim,confidence:0.5,observed_at:i64} | `12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e` |
| ACS-002-CS-1 V2 | 0x02 | dCBOR map engine-manifest {seed:null,reads:[..],engine,version,deterministic} | `1220e5aad722341bd0838fb268d73a0a28401457883b9e5e623c05dc0623f57a690d` |
| ACS-002-CS-1 V3 | 0x05 | dCBOR map {n:-1000,label:NFC "Amélie é—中"} | `12207c5367768a3cd0d90b781cac2530335f0310ffc155eac4ac82da80af71e2366a` |
| ACS-003-CS-1 | 0x06 | dCBOR envelope (12 fields, payload_cid=ACS-002 V1) | `1220fc0ef055e4d39de1c3ab7d2597361d24f7a8b6a1a0609a91b872b85ae4896f93` |
| ACS-004-CS-1 schema | 0x07 | dCBOR `uci.fact@1.0` schema document (430 B) | `12206b3f99c64d23029f49b986e4c89152955c649274e5cf60b1b3ad581b19fa4b87` |
| ACS-004-CS-1 instance | 0x01 | dCBOR `uci.fact@1.0` instance (264 B) | `12206fce3fbcfce59860140942f7d1ca9e7b274fd936f2237011ff144552f091f07e` |
| ACS-005-CS-1 term-set | 0x08 | raw `GL-001\n…\nGL-014` | `1220ced393907a4d27eb54ac12acea65e29c7168c2991b3ca9df4b39765e870d2074` |
| ACS-005-CS-1 requirement | 0x09 | raw `ORCH-001-R1: …` | `12207f1a532d2be5061377d6664be065bbb45b6e61741bb70c1195454054e1cf0475` |
| ACS-005-CS-1 term-names | 0x08 | raw `Capability\n…\nTenant` | `12200c1c893c613d0f12976697084f05a76243589ed55a3d2cdae9dbce9d69df4751` |

dCBOR scalar encodings (ACS-002, body only): `true`=f5 · `false`=f4 · `null`=f6 ·
`0`=00 · `24`=1818 · `-1000`=3903e7 · `1.0`=fb3ff0000000000000 · text `hello-truth`=6b68656c6c6f2d7472757468.

**Status:** Rust ✅ (all 12 asserted in `arves-acs`). Go/Java/Python ⬜ (Independent
Runtime Challenge). Differential harness ⬜.
