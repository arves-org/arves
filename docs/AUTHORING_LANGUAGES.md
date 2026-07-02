# Authoring Languages — What You Write Capabilities In

**Short answer: today you author ARVES capabilities in JavaScript (ES modules, `.mjs`),
running on Node >= 18.** That is the only supported authoring surface right now. This page
clears up a genuine point of confusion in the docs.

## The distinction that matters: *authoring* vs *runtime*

ARVES has two very different kinds of "language":

| | What it is | Language today |
|---|---|---|
| **Capability authoring** | The code *you* write to produce truth — a capability's `execute()` and its test inputs. Consumes the [Ecosystem Author SDK](../products/arves-ecosystem-sdk/README.md); never touches the runtime. | **JavaScript / `.mjs` only** |
| **Runtime implementation** | The engine that *hosts* capabilities and commits their effects as truth (Kernel, persistence, engine, bridge). An independent party can build one in any language, as long as it matches the frozen contracts byte-for-byte. | Rust and Python reference runtimes exist; any language is permitted |

When you see **"Python / Go / Java"** in ARVES documentation, it almost always refers to the
**runtime implementation** side — the promise that a conformant ARVES *runtime* can be written
in those languages and certified against the same spec. It does **not** mean you can author a
capability in Python or Java today. Authoring is JavaScript.

## Why authoring is JavaScript today

The authoring toolchain — [`defineCapability`](../products/arves-ecosystem-sdk/src/kit.mjs),
`certifyCapability`, `packageCapability`, and the [`arves` CLI](./CLI_REFERENCE.md) — is
implemented as a Node/ESM package. A capability file **default-exports**
`{ capability, testInputs, source }`, and the whole author → certify → package → publish →
install flow runs offline on Node with no Rust build. This keeps the newcomer on-ramp to
minutes (see [DEVELOPER_JOURNEY_REPORT.md](../verification/dx/DEVELOPER_JOURNEY_REPORT.md)).

What crosses the boundary into a runtime is never your source code — it is the
**content-addressed, ACS-canonical truth** your capability produces (see
[SPEC_STARTER.md](./SPEC_STARTER.md), ACS-001/002). Because the boundary is *bytes*, not a
language binding, the authoring language and the runtime language are independent choices.

## What this means in practice

- To author a capability now: use JavaScript / `.mjs`. Start with
  `arves init <name>` or `arves create <name> --provider reference`
  (see the [CLI reference](./CLI_REFERENCE.md)).
- Do **not** expect a `.py` or `.java` capability to be accepted by the tooling today — there is
  no importer for it.
- If a doc invites you to build in "Python/Go/Java", read the surrounding context: it is
  describing a **runtime** you could implement, not a capability you could author.

## Roadmap — a JVM / Python Authoring Kit

A non-JavaScript **authoring** on-ramp is a tracked roadmap item, not a shipped feature. The
plan is an **Authoring Kit** for at least the JVM (Java/Kotlin) and Python that would let authors
write capabilities in those languages while still producing the identical ACS-canonical truth the
JavaScript kit produces — because the contract is the bytes, not the binding, this is an additive
SDK effort that does not change the frozen runtime or spec.

Until that ships, this page is the honest statement of record: **capability authoring is
JavaScript/`.mjs` today; a JVM/Python Authoring Kit is on the roadmap.** The DX baseline records
this same gap ("no non-JS capability authoring on-ramp") as a backlog item — see
[DEVELOPER_JOURNEY_REPORT.md](../verification/dx/DEVELOPER_JOURNEY_REPORT.md).
