# ARVES Reference Runtime (UCI) - Rust

The **Runtime layer** of the ARVES Engineering Operating System (AEOS).
Reference implementation (Runtime A) of the frozen UCS/UCI v1.0 specification.

**Rules:** implementation proves the specification and never changes it. The frozen Baseline milestone set governs: I1 Distributed Runtime -> I2 Cluster Kernel -> I3 Distributed Query -> I4 Capability Scheduling -> I5 Multi-Agent Runtime -> I6 Reference Products.

One crate per architecture component; each crate's module header cites its governing invariants.

## Layout (frozen-architecture mapping)

- `crates/arves-ontology` - uci.* cognitive type registry + mandatory aspects (Identity/Provenance/Trust/Temporal/TenantScope).
- `crates/arves-invariants` - Machine-checkable invariant markers + property-test scaffolding.
- `crates/arves-kernel` - Owner of cognitive TRUTH and the sole commit gateway.
- `crates/arves-persistence` - Durable append-only WAL + snapshots of Kernel-committed state.
- `crates/arves-lcw` - Living Cognitive World: single owner of Working Memory / live state.
- `crates/arves-query` - Strictly read-only projections over Kernel/LCW/Persistence (read tiers).
- `crates/arves-information-platform` - Connectors + canonicalization; emits proposed writes to the Kernel.
- `crates/arves-capability-fabric` - Capability registry + bindings (owns bindings, never truth/plans).
- `crates/arves-engine-fabric` - Pure, stateless engines behind the Engine ABI; produce inference, not truth.
- `crates/arves-control-plane` - Orchestrator: owns the Plan/Engine Graph; owns no truth, no persistent state.
- `crates/arves-execution` - Execution layer: performs actions; routes outcomes as proposed writes to the Kernel.
- `crates/arves-consensus` - Per-shard Raft: CP truth replication (leader/followers, joint consensus).
- `crates/arves-conformance` - Scenario Conformance harness: 12 axes -> reference scenarios -> node probes -> verdict.
- `crates/arves-runtime` - Runnable binary (single-node I1 entry point): a single-node commit gateway over a durable append-only WAL with cross-process recovery. Subcommands `write` / `recover` / `checkpoint` drive the cross-process restart proofs; with no subcommand it runs the in-memory demo. Run via `cargo run -p arves-runtime`.

Status: **I1 runtime core IMPLEMENTED and runnable** - single-node commit gateway + durable WAL + deterministic cross-process recovery (per `RUNTIME_FREEZE_v1.0.md`; frozen at tag `runtime-v1.0`). Build: `cd runtime && cargo check`.
