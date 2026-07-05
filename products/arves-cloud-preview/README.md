# @arves/cloud-preview — ARVES Cloud Platform (P8, PREVIEW)

**Pins:** ARVES Runtime v1.0 (FROZEN, tag `runtime-v1.0`) + `arves-standard-kit 0.3.1`.
**Modifies no platform file (IDR-006).** Products are customers of the frozen runtime.

A **local multi-tenant HTTP gateway** in front of the real reference runtime — the
smallest honest slice of "hosted ARVES". Any HTTP client, in any language, commits truth
to the real Rust Kernel and receives the ACS-001 ContentId back, which it can recompute
locally and verify **without trusting the gateway**.

> **This is a PREVIEW, not a deployed SaaS.** It runs on `127.0.0.1`, with no TLS, no
> authentication, and no durability across restarts. Every limitation is listed below —
> honesty is the product.

## Run

```
cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml
node examples/cloud-day.mjs     # demo (exit 0 = every property held)
node cloud.test.mjs             # property tests (exit 0 = all pass)
```

## What the preview PROVES

| Property | How it is shown |
|---|---|
| **Hosted access to the real Kernel** | `POST /:tenant/commit` reaches the frozen Rust reference Kernel through the `arves-bridge` binary; the response carries the Kernel's ACS-001 ContentId + truth index |
| **One-world identity over HTTP** | the ContentId returned over HTTP equals the address the SDK computes locally, offline — the caller can verify every response without trusting the gateway |
| **Content addressing is universal** | the same fact committed by two different tenants gets the **same** ContentId |
| **Per-tenant idempotency + isolated truth** | acme's re-commit is `already-committed` for acme while globex's *first* commit of the same fact is fresh (`committed`) — same identity, isolated stores (ORCH-004 per tenant) |
| **Hosted cognitive chain** | `POST /:tenant/invoke` runs Capability → Engine → Kernel; an unbound capability is refused (422) |
| **Hard input hygiene** | unknown tenant → 404 · oversized body → 413 (byte cap enforced while streaming) · malformed JSON → 400 naming the field · bare JSON number → 400 with the exact field path · the gateway never crashes on hostile input |

## HTTP API

| Route | Body | Success response |
|---|---|---|
| `POST /:tenant/commit` | `{"value": <wire-value>}` | `{tenant, contentId, status, index}` |
| `POST /:tenant/invoke` | `{"value": <wire-value>, "capability": "derive.fact"}` | `{tenant, capability, contentId, status, index}` |
| `GET /:tenant/health` | — | `{tenant, ok, kernel:"live", probe:{contentId, status}}` |

`status` is `committed` \| `already-committed`. Errors are always
`{tenant?, error:{code, field?, message}}` — never a stack trace, never a crash.
Tenants are an **allowlist fixed at construction** (`new ArvesCloud({tenants:[...]})`);
names must match `/^[a-z][a-z0-9-]{0,31}$/`; anything else on the wire is a 404.
Body size: requests over `maxBodyBytes` (default 64 KiB) are 413 `body-too-large` at the
gateway. Independently, the bridge's line protocol caps a request line at 1 MiB (hex
encoding doubles the value bytes), so if an operator raises `maxBodyBytes` above
~512 KiB, oversized values are refused by the **bridge** instead and surface as 413
`body-too-large-for-bridge`. A refused plain commit is reported as `commit-refused`
(422) — never mislabeled as a capability (`invoke-refused`) failure.
The commit domain is fixed to `commit` in this preview. `GET /:tenant/health` performs a
**real** Kernel round-trip by committing one fixed, deterministic probe fact
(`{type:"cloud.health-probe"}`) — so the first probe adds one truth to that tenant's
store and every later probe is `already-committed`.

### Wire convention (BigInt-safe JSON)

ACS-002 forbids int/float ambiguity and JSON numbers are unsafe beyond 2^53, so:

- **Integer** → `{"$int": "42"}` (decimal string; range `[-2^64, 2^64-1]`, enforced → 400)
- **Float** → `{"$float": 0.5}` (finite JSON number, binary64)
- **Bare JSON numbers are rejected** with a 400 naming the exact field path (e.g. `value.confidence`)
- strings / booleans / null / arrays / objects pass through; `$`-prefixed map keys are
  reserved for wrappers (any other use → 400); the map key `__proto__` is **rejected**
  with a 400 (`reserved-key`) — JS property assignment would silently drop the subtree,
  collapsing two distinct wire bodies onto one ContentId, so it is not expressible rather
  than silently mangled; ACS Bytes values are **not expressible** over this wire format.

## What the preview does NOT prove (read this)

- **No authN/authZ.** Runtime v1.0 has no principal on commit (RUNTIME_FREEZE v2.0 debt
  #8), so the gateway *cannot* attribute truth to an authenticated caller. The tenant
  path segment is an address, not an identity claim: anyone who can reach the port can
  write as any allowlisted tenant.
- **No TLS.** Plain HTTP on `127.0.0.1` only.
- **No persistence across restart.** The shipped `arves-bridge` builds its Kernel on
  `MemWalStore` (an in-memory WAL). Truth is real Kernel truth — ACS-addressed,
  idempotent, ordered — but it lives only as long as the tenant's bridge process. When
  the gateway stops, the stores are gone.
- **Tenancy is process isolation, not Kernel-shard isolation.** The runtime Kernel
  natively supports tenant isolation (SHARD-001: `ShardKey{tenant, workspace}`), but the
  shipped `arves-bridge` binary pins ONE hard-coded shard (`t1`/`w1`) per process. This
  product therefore spawns one bridge process per tenant — a **product-layer workaround**
  that gives real isolation (separate OS processes, separate memory), but does not
  exercise the Kernel's own multi-shard tenancy.
- **Not a deployed cloud.** No horizontal scaling, no rate limiting, no quotas, no
  billing, no operator plane. "Cloud" here means: *the runtime's guarantees survive
  being put behind a network API* — that is the property being previewed.

## RCR candidates found (recorded, NOT made — the runtime stays frozen)

1. **[bridge: shard selection per request]** — the `arves-bridge` bin hard-codes
   `ShardKey{tenant:"t1", workspace:"w1"}`. A protocol extension (e.g.
   `commit <tenant> <workspace> <domain_hex> <body_hex>`) would let ONE bridge process
   serve many tenants using the Kernel's native SHARD-001 isolation, replacing this
   product's process-per-tenant workaround. → RCR for Runtime v1.1.
2. **[bridge: durable WAL store option]** — the bin constructs `MemKernel::new(MemWalStore::new())`;
   a flag selecting a file-backed WAL store would let hosted deployments make an honest
   persistence-across-restart claim. → RCR for Runtime v1.1.
3. **(pre-existing, re-confirmed)** commit carries no principal — hosted multi-user
   attribution is impossible until RUNTIME_FREEZE v2.0 debt #8 lands.

## Layout

```
src/cloud.mjs           ArvesCloud gateway (node:http only, zero third-party deps)
examples/cloud-day.mjs  two-tenant demo: universality + isolation + hygiene (exit 0/1)
cloud.test.mjs          assert-based property tests (exit 0/1)
```
