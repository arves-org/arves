# ARVES Cognitive Studio — P6 preview (graph-as-data authoring)

> **Platform pin:** consumes `arves-standard-kit 0.3.1` semantics via the P0 SDK
> (`products/arves-sdk-ts`) + the P6.5 Ecosystem Kit (`products/arves-ecosystem-sdk`) +
> the **FROZEN ARVES Runtime v1.0** bridge (`runtime/target/debug/arves-bridge`).
> **This product modifies no platform file (IDR-006).** Runtime gaps found here are
> recorded below as RCR candidates — never worked around by editing `runtime/`.
> **IDR-006 evidence (re-captured post-review, 2026-07-05):** with the parallel
> runtime-team RCR-012 work committed (`b10849e`), `git status --porcelain` shows only
> untracked `products/` directories — zero tracked-file modifications anywhere in the
> frozen tree (`runtime/`, `standard/`, `spec-markdown/`, `corpus/`).

The P6 ladder entry is "Visual Cognitive Studio — visual authoring of cognitive graphs".
This preview delivers the honest core of that: a **cognitive graph authored as data**,
validated by refusal, executed as committed truth on the real Kernel, and rendered to a
**static** HTML visualization. It is *not* a GUI application.

## What it is

```
defineGraph ──> validateGraph ──> runGraph(bridge) ──> renderGraph
 (author)        (refuse bad)      (truth on the        (static HTML,
                                    real Kernel)         self-contained)
```

- **`defineGraph({ name, nodes, edges })`** — a declarative graph spec. Each **node**
  binds a capability authored with the Ecosystem Kit (`defineCapability`) plus the
  representative `testInputs` its certification is re-run against. Each **edge** wires
  one node's *committed-truth output* — the ACS ContentIds together with the values that
  produced them — into the next node's input (`{ from, to, output?, as? }`). The graph's
  own `id` is the ACS content address of its spec (node manifests + real code hashes +
  wiring): change anything, the id changes.
- **`validateGraph(graph)`** — refusal, not attestation: unknown-node edges, edge
  outputs not declared in the from-node's `produces`, two incoming edges of one node
  sharing the same effective input key (`as ?? from` — the later wire would silently
  shadow the earlier one, letting a node impersonate another's evidence input),
  cycles/self-loops, and — enforced
  — **every node's capability must CERTIFY**. Certification is *re-run* here via the
  Kit's `certifyCapability`; no `certified: true` flag on the node, the capability, or
  anywhere else is ever read, so forging one changes nothing.
- **`runGraph(bridge, graph, input)`** — re-validates, then executes nodes in a
  deterministic topological order (lexicographic tie-break), committing every node
  effect as ACS truth through the **real `KernelBridge`** and finally committing the
  whole run (graph id + ordered per-node truth ids) as a `trace`-domain run root.
  Same graph + same input ⇒ **byte-identical ContentIds on a fresh Kernel** (on a warm
  one the ids are identical and the status flips to `already-committed` — ORCH-004
  idempotency). A node effect targeting an undeclared produce is refused at run time
  too, because certification only *sampled* the testInputs.
- **`renderGraph(graph, results?)`** — a **self-contained static HTML** string (inline
  SVG + inline CSS, zero scripts, zero external assets) showing the graph topology and
  each node's committed truth ids. Node names/ids are HTML-escaped. Deterministic: same
  inputs, byte-identical HTML. The caller writes it to a file
  (see `examples/studio-day.html` after running the demo).

## What the preview PROVES

- **Graph authoring as data** with content-addressed graph identity.
- **Certified nodes, enforced** — validation re-runs certification; forged flags are dead.
- **Truth-committed execution** — every node effect and the run root are ACS truths in
  the frozen Runtime v1.0 reference Kernel, committed through the real bridge protocol.
- **Reproducibility** — two runs on two fresh Kernels (two fresh bridge spawns) return
  byte-identical ContentIds for every node and for the run root; a warm re-run
  demonstrates idempotency instead.
- **Evidence chaining** — a downstream node receives the upstream ContentIds and can
  cite them (`evidence` / `basedOn`) inside its own committed truth.

## What it does NOT prove (honest scope)

- **No interactive GUI editor.** "Visual" here is a static render. No drag-and-drop, no
  live editing, no served app. The full P6 vision (a visual editor that round-trips to
  this graph-as-data form) is future work; this preview is its execution + render core.
- **Graph state is in-memory.** The spec lives in JS data; nothing persists it. Only
  the *effects* become Kernel truth.
- **Node code runs in this process (product layer).** Effects become truth in the real
  Kernel, but the node's `execute` is not run inside the runtime's Engine fabric — the
  bridge cannot dynamically bind product capabilities (see RCR candidates). This is the
  same posture as the P6.5 `CapabilityHost`.
- **Bridge truth is process-lifetime.** The bridge server's Kernel is `MemKernel` over
  `MemWalStore` (in-memory WAL): ids are real, reproducible ACS-001 addresses, but the
  committed state is not durable on disk across bridge restarts.
- **Certification is the Kit's best-effort probe.** The determinism check runs each
  testInput twice; it refuses *observed* non-determinism but cannot prove purity
  (input-scoped or delayed non-determinism can pass — engine-enforced determinism is
  recorded v1.1 debt). The studio adds a run-time undeclared-target guard on the real
  input, but a certified-then-misbehaving *value* is still committed as truth.
- **A refused or failed run can leave partial truth.** If `runGraph` refuses
  mid-execution (e.g. the run-time undeclared-target guard fires on node 3 of 5),
  the effects of earlier nodes are already committed with **no run root**. On the
  in-memory bridge these vanish with the process, and a corrected re-run re-derives
  identical ids idempotently (ORCH-004) — but there is no rollback.
- Byte-identical ContentIds prove **content equality of committed values**, not that
  the two processes took identical execution paths.

## RCR candidates (runtime gaps recorded, not worked around)

1. **Dynamic capability binding over the bridge protocol.** The bridge ships one
   pre-bound reference capability (`derive.fact`); there is no `bind`/`register` verb,
   so graph nodes cannot execute inside the Capability→Engine chain — the studio commits
   their effects with direct `commit` lines instead. A v1.1 bridge extension (register a
   capability binding per session) would let `runGraph` use `invoke` end-to-end.
2. **Batch commit.** A graph run of N effects costs N+1 bridge round-trips (plus the run
   root). This is the already-recorded "Kernel batch-commit" v1.1 debt
   (`runtime/RUNTIME_FREEZE_v1.0.md`); graph execution is a concrete customer for it.
3. **Engine-enforced determinism** (existing v1.1 debt) — would upgrade node
   certification from a best-effort probe to a runtime guarantee.
4. **Durable bridge Kernel option.** A bridge flag to back the Kernel with a WAL file
   would make a rendered page's truth ids re-verifiable against a persistent Kernel
   across bridge restarts.

## Run it

```
# once: cargo build -p arves-bridge --bin arves-bridge   (from runtime/)
node products/arves-cognitive-studio/examples/studio-day.mjs   # demo (writes studio-day.html)
node products/arves-cognitive-studio/studio.test.mjs           # 34 regression checks
```

Both exit 0 on success. Plain Node >= 18, zero third-party dependencies; integers are
BigInt, floats are `float(x)` — bare JS numbers are refused by the codec.
