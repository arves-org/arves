//! PropertyCheck / Suite (RCR-006): the invariant-id → executable-proof catalog.
//!
//! SYSTEM_GAP_ANALYSIS #18: the registered invariants were proven ad-hoc (LAYER-001/OWN-001
//! by the architecture gate; ORCH-003/004 by Kernel tests) with no single artifact binding
//! each `InvariantId` to *its* proof and reporting coverage. This module is that binding: a
//! [`catalog`] of [`PropertyCheck`]s, each a registered [`Invariant`] paired with a
//! [`ProofKind`]. The two *structural* invariants LAYER-001 and OWN-001 are proven **here,
//! in-process**, over the real Cargo workspace graph (the pure checkers moved out of the
//! architecture-gate test into this lib so both use one implementation). The runtime-behaviour
//! invariants (ORCH-003/004, SHARD-001) are **cited** to their biting tests in the owning
//! crate — the catalog records where each executable proof lives. The Control-Plane invariants
//! (ORCH-001/002) were **pending** while their owning crate was contract-only; since RCR-027
//! (I4 Stage 2 — the cluster capability scheduler, the first Control-Plane behaviour) they are
//! **cited** to the scheduler's crash-rebuild / commit-provenance proofs (scoped to that
//! surface; the I5 Orchestrator adds its own obligations when it lands). RCR-028 (I4 Stage 3)
//! widened the cited proofs across the ADVERSARIAL scheduling surface (schedule storms,
//! node/scheduler death, poison storms, leadership change) and the live
//! `capability-scheduling-distributed` conformance artifact. RCR-031 (I5 Stage 3) widened
//! them again across the MULTI-AGENT surface (RCR-029 identity/attribution + RCR-030
//! decision/compliance flow + the RCR-031 adversarial proofs: seeded storm permutations,
//! attribution-forging refusals, partition honesty, full-cluster replay INCLUDING the
//! attribution trail) and the live `multi-agent-coordination-distributed` artifact —
//! honest scope: the frozen `Orchestrator` plan-graph contract itself remains
//! contract-only (delegation/plan-graph execution is NOT pre-certified by these rows).
//!
//! `run_suite()` executes every in-process proof; the accompanying test asserts they all hold
//! and that the coverage map matches reality (no silent "pending → proven" drift).

use crate::Invariant;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// LAYER-001 / OWN-001 structural checkers (over the real Cargo workspace graph).
// Moved here from tests/architecture_gate.rs so the gate test AND the PropertyCheck
// catalog share one implementation (no duplication, no drift).
// ---------------------------------------------------------------------------

/// Architectural layer rank per crate (higher = higher layer). An internal dependency
/// edge `A -> B` is legal iff `rank(A) > rank(B)` (downward-only, LAYER-001). An unranked
/// crate is a gate failure (it must be classified before it can ship).
pub fn layer_rank(name: &str) -> Option<i32> {
    Some(match name {
        "arves-ontology" | "arves-invariants" => 10, // foundation (pure defs)
        "arves-acs" => 15,                           // content-addressing codec (ACS)
        "arves-persistence" => 20,                   // durable substrate (WAL)
        "arves-consensus" => 30,                     // replication over persistence
        "arves-kernel" => 40,                        // sole truth owner
        "arves-lcw" => 50,                           // working memory (non-truth)
        "arves-query" => 60,                         // read-only projection
        "arves-engine-fabric" => 60,
        "arves-capability-fabric" => 70,
        "arves-execution" => 70,
        "arves-information-platform" => 80,
        "arves-control-plane" => 90, // decides, owns no truth
        "arves-runtime" => 100,      // top orchestrator (bin)
        "arves-bridge" => 105,       // SDK<->Kernel seam (kernel+acs consumer)
        "arves-conformance" => 110,  // test harness (may read all)
        _ => return None,
    })
}

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .to_path_buf()
}

/// Parse the internal (`arves-*`) `[dependencies]` names from a Cargo.toml (dev/build
/// deps are excluded on purpose — a test-only edge is not an architectural edge).
pub fn parse_internal_deps(toml: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    for raw in toml.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]";
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let key: String = line
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '=' && *c != '.')
            .collect();
        if key.starts_with("arves-") {
            deps.push(key);
        }
    }
    deps
}

/// The actual workspace edge set: crate -> [internal deps].
pub fn workspace_edges() -> BTreeMap<String, Vec<String>> {
    let mut edges = BTreeMap::new();
    for entry in fs::read_dir(crates_dir()).expect("read crates dir").flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("arves-") {
            continue;
        }
        let toml = match fs::read_to_string(entry.path().join("Cargo.toml")) {
            Ok(t) => t,
            Err(_) => continue,
        };
        edges.insert(name, parse_internal_deps(&toml));
    }
    edges
}

/// Pure LAYER-001 checker: human-readable violations for an edge set (empty = clean).
pub fn layering_violations(edges: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut v = Vec::new();
    for (crate_name, deps) in edges {
        let cr = match layer_rank(crate_name) {
            Some(r) => r,
            None => {
                v.push(format!("unranked crate '{crate_name}' (classify it in layer_rank)"));
                continue;
            }
        };
        for dep in deps {
            match layer_rank(dep) {
                None => v.push(format!("edge {crate_name} -> {dep}: dependency is unranked")),
                Some(dr) if dr >= cr => v.push(format!(
                    "LAYER-001 violation: {crate_name}(rank {cr}) -> {dep}(rank {dr}) is not downward-only"
                )),
                Some(_) => {}
            }
        }
    }
    v
}

/// Crates whose source DEFINES the truth-commit gateway (`pub trait Kernel`). OWN-001
/// requires exactly one (`arves-kernel`).
pub fn truth_gateway_owners() -> Vec<String> {
    let mut owners = Vec::new();
    for entry in fs::read_dir(crates_dir()).expect("read crates dir").flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("arves-") {
            continue;
        }
        let src = entry.path().join("src");
        let mut defines = false;
        for f in ["lib.rs", "main.rs"] {
            if let Ok(text) = fs::read_to_string(src.join(f)) {
                if text.contains("pub trait Kernel") {
                    defines = true;
                }
            }
        }
        if defines {
            owners.push(name);
        }
    }
    owners.sort();
    owners
}

// ---------------------------------------------------------------------------
// The catalog.
// ---------------------------------------------------------------------------

/// How a registered invariant's executable proof is discharged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofKind {
    /// Proven in-process by this module, over the real workspace/runtime. `pass` is the
    /// live result; `detail` explains it.
    InProcess { pass: bool, detail: String },
    /// Proven by a biting test in the owning crate; `location` is where the executable
    /// proof lives (the catalog cites it rather than re-running a cross-crate test).
    CitedTest { location: &'static str },
    /// No runtime proof exists yet — the owning component is contract-only (I2+). Honest
    /// coverage gap, not a faked pass.
    Pending { reason: &'static str },
}

/// One entry of the invariant → proof catalog.
#[derive(Debug, Clone)]
pub struct PropertyCheck {
    /// The registered invariant this entry proves.
    pub invariant: Invariant,
    /// How its executable proof is discharged.
    pub proof: ProofKind,
}

impl PropertyCheck {
    /// A check "holds" iff it is proven (in-process passing, or a cited test) — a Pending
    /// entry does NOT hold (it is an honest, visible coverage gap).
    pub fn holds(&self) -> bool {
        match &self.proof {
            ProofKind::InProcess { pass, .. } => *pass,
            ProofKind::CitedTest { .. } => true,
            ProofKind::Pending { .. } => false,
        }
    }
}

/// Build the full catalog, executing the in-process (structural) proofs live.
pub fn catalog() -> Vec<PropertyCheck> {
    use Invariant::*;

    // LAYER-001 — downward-only edges over the real Cargo graph (executed now).
    let layer_v = layering_violations(&workspace_edges());
    let layer = PropertyCheck {
        invariant: Layer001DownwardOnly,
        proof: ProofKind::InProcess {
            pass: layer_v.is_empty(),
            detail: if layer_v.is_empty() {
                "all internal crate edges are downward-only".into()
            } else {
                layer_v.join("; ")
            },
        },
    };

    // OWN-001 — exactly one truth-commit gateway (`pub trait Kernel`) (executed now).
    let owners = truth_gateway_owners();
    let own = PropertyCheck {
        invariant: Own001OneOwnerPerState,
        proof: ProofKind::InProcess {
            pass: owners == ["arves-kernel"],
            detail: format!("truth-commit gateway owners: {owners:?}"),
        },
    };

    vec![
        layer,
        own,
        // ORCH-004 — idempotent + content-addressable commit; a re-proposal under the same
        // address resolves to existing truth (behaviour_2) and a same-address/different-payload
        // fork is rejected (behaviour_7 / RCR-005). Under distribution (RCR-022): truth stays
        // exactly-once through duplicate/reordered consensus-message storms and client retries.
        PropertyCheck {
            invariant: Orch004IdempotentAddressable,
            proof: ProofKind::CitedTest {
                location: "arves-kernel::tests::walking_skeleton \
                           (behaviour_2_commit_twice_already_committed, \
                           behaviour_7_content_integrity_same_address_different_payload); \
                           cluster scope (RCR-022): arves-kernel::tests::cluster_adversarial \
                           (adversarial_duplicate_reordered_delivery_truth_exactly_once); \
                           arves-consensus::tests::raft_adversarial \
                           (adversarial_dup_reorder_storm_commits_each_entry_exactly_once); \
                           READ-path idempotency (I3, RCR-023/025): arves-query::tests::query_core \
                           (read_only_reads_change_no_state_and_are_idempotent); \
                           arves-query::tests::distributed_query \
                           (cluster_wide_isolation_on_every_replica_and_tier_and_reads_write_nothing); \
                           SCHEDULING layer (I4, RCR-027/028): arves-control-plane::tests::cluster_scheduling \
                           (duplicate_submission_and_racing_schedulers_never_fork_truth_orch004); \
                           arves-control-plane::tests::adversarial_scheduling \
                           (storm_duplicate_and_reordered_schedules_never_double_execute_orch004, \
                           node_death_mid_invocation_replaces_and_never_duplicates_commit, \
                           leadership_change_mid_schedule_lands_each_invocation_exactly_once); \
                           arves-conformance::live (live_capability_scheduling_scenario_passes); \
                           MULTI-AGENT surface (I5, RCR-029/030/031): arves-control-plane::tests::agent_identity \
                           (registration_commits_content_addressed_truth_and_is_idempotent_orch004, \
                           duplicate_attributed_effects_converge_to_one_truth_orch004); \
                           arves-control-plane::tests::multi_agent_orchestration \
                           (duplicate_and_agreeing_proposals_converge_to_one_truth_orch004_across_agents, \
                           seeded_interleaving_permutations_yield_one_derived_truth_and_no_forks); \
                           arves-control-plane::tests::multi_agent_adversarial \
                           (agent_storm_truth_set_identical_across_all_seeded_schedule_permutations — \
                           the order-independence proof: one truth set across ALL permutations; \
                           replay_rebind_and_rewrap_cannot_forge_attribution_and_floods_never_double_commit); \
                           arves-conformance::live (live_multi_agent_coordination_scenario_passes)",
            },
        },
        // ORCH-003 — replay from the recorded trace reproduces identical truth. Under
        // distribution (RCR-022): every node of the cluster rebuilt from its own WAL
        // reproduces the identical truth_hash / state bytes.
        PropertyCheck {
            invariant: Orch003ReplayableFromTrace,
            proof: ProofKind::CitedTest {
                location: "arves-kernel::tests::walking_skeleton \
                           (behaviour_3_replay_same_truth, behaviour_4_crash_restart_replay_identical); \
                           cluster scope (RCR-022): arves-kernel::tests::cluster_adversarial \
                           (adversarial_full_cluster_replay_from_wal_rebuilds_identical_truth); \
                           READ path (I3, RCR-023/024/025): arves-query::tests::query_core \
                           (orch003_fold_digest_equals_kernel_truth_hash_basis, \
                           orch003_snapshot_at_index_deterministic_and_suffix_equivalent); \
                           arves-query::tests::adversarial_reads \
                           (replay_equivalence_rebuilt_from_wal_equals_live_projection_on_every_replica, \
                           torn_read_impossibility_batches_all_or_none_on_every_reachable_observation, \
                           query_results_deterministic_and_replicas_converge_under_message_storms); \
                           arves-conformance::live (live_distributed_query_scenario_passes); \
                           SCHEDULING layer (I4, RCR-027/028): arves-control-plane::tests::cluster_scheduling \
                           (leader_loss_mid_dispatch_replays_from_record_and_commits_exactly_once — \
                           the retry replays from the RECORDED inference, engine never re-invoked); \
                           arves-control-plane::tests::adversarial_scheduling \
                           (node_death_mid_invocation_replaces_and_never_duplicates_commit); \
                           MULTI-AGENT surface incl. ATTRIBUTION (I5, RCR-029/030/031): \
                           arves-lcw::tests::shared_world \
                           (world_view_at_commit_index_n_is_identical_on_every_replica); \
                           arves-control-plane::tests::multi_agent_orchestration \
                           (conflicting_decisions_resolve_first_committed_wins_with_loser_receiving_winner — \
                           same scripted schedule ⇒ byte-identical shard state); \
                           arves-control-plane::tests::multi_agent_adversarial \
                           (full_cluster_replay_from_wal_rebuilds_identical_truth_and_attribution_trail — \
                           every node rebuilt from its own WAL reproduces identical truth AND an \
                           identical attribution trail / decision derivation / compliance ledger); \
                           arves-conformance::live (live_multi_agent_coordination_scenario_passes)",
            },
        },
        // SHARD-001 — a shard MUST NOT contain cross-tenant data. Proven by a two-tenant
        // isolation test at the truth gateway (RCR-007) + the persistence wrong-shard
        // rejection test (no structural-only citation). Under distribution (RCR-022, per the
        // I2 design §4 SHARD-001 row): two tenants on two independent replicated Raft groups,
        // zero cross-tenant leakage on every replica across a failover, per-shard leadership.
        PropertyCheck {
            invariant: Shard001TenantWorkspacePartition,
            proof: ProofKind::CitedTest {
                location: "arves-kernel::tests::walking_skeleton \
                           (behaviour_8_two_tenant_isolation); \
                           arves-persistence::tests::file_wal (wrong_shard_append_rejected, \
                           multi_shard_isolation_survives_disk); \
                           cluster scope (RCR-022): arves-conformance::live \
                           (live_cluster_distributed_scenario_passes — replicated two-tenant \
                           isolation + per-shard leadership); \
                           arves-consensus::tests::shard_map (RCR-020 blast-radius isolation); \
                           distributed READ isolation (I3, RCR-023/024/025): \
                           arves-query::tests::query_core (shard001_tenant_a_never_sees_tenant_b_on_any_tier); \
                           arves-query::tests::distributed_query \
                           (cluster_wide_isolation_on_every_replica_and_tier_and_reads_write_nothing, \
                           gather_routes_on_typed_shard_identity_never_reparsed_text); \
                           arves-conformance::live (live_distributed_query_scenario_passes); \
                           SCHEDULING layer (I4, RCR-027/028): arves-control-plane::tests::cluster_scheduling \
                           (backpressure_bounds_one_shard_and_the_other_tenants_transcript_is_untouched, \
                           same_capability_provider_and_input_in_two_shards_both_commit_their_own_truth); \
                           arves-control-plane::tests::adversarial_scheduling \
                           (poison_capability_storm_cannot_block_shard_or_cluster, \
                           overcapacity_refusals_are_explicit_accounted_and_retriable_never_silent); \
                           MULTI-AGENT surface (I5, RCR-029/030/031): arves-control-plane::tests::agent_identity \
                           (identity_is_shard_bound_for_life_shard001, \
                           smuggled_foreign_shard_definition_is_refused_shard001); \
                           arves-lcw::tests::shared_world \
                           (hydration_into_a_foreign_shard_memory_is_refused_shard001); \
                           arves-control-plane::tests::multi_agent_adversarial \
                           (full_cluster_replay_from_wal_rebuilds_identical_truth_and_attribution_trail — \
                           two tenants' attribution trails never blur, before or after replay); \
                           arves-conformance::live (live_multi_agent_coordination_scenario_passes — \
                           cross-tenant attribution refused, per-shard worlds isolated)",
            },
        },
        // ORCH-001 / ORCH-002 — Control-Plane truth/state ownership. Pending until RCR-027
        // (I4 Stage 2), which landed the first Control-Plane BEHAVIOUR (the cluster
        // capability scheduler in arves-control-plane) together with the design's §4
        // executable proofs: the scheduler's only path to truth is routing ProposedWrites
        // through the shard leader's frozen Kernel gateway (no other write surface exists;
        // ORCH-001), and every queue/ledger/decision it holds is discardable mid-run and
        // reconstructible from the plan with Kernel-deduped convergence (ORCH-002).
        // HONEST SCOPE: proven at the I4 SCHEDULING surface; the I5 Orchestrator
        // (plan-graph execution) remains contract-only and adds its own obligations when
        // it lands — this row does not pre-certify it.
        PropertyCheck {
            invariant: Orch001ControlPlaneOwnsNoTruth,
            proof: ProofKind::CitedTest {
                location: "arves-control-plane::tests::cluster_scheduling (RCR-027): \
                           scheduler_crash_rebuild_from_plan_converges_to_identical_truth_orch001_orch002 \
                           (dropping the scheduler loses zero committed truth; all truth carries \
                           Kernel commit provenance via the shard leader), \
                           full_distributed_chain_gate_engine_leader_commit_replicas_converge \
                           (effects leave the gate as PROPOSALS; only ClusterKernel::commit \
                           promotes them; Pure work commits nothing); \
                           adversarial scope (I4 Stage 3, RCR-028): \
                           arves-control-plane::tests::adversarial_scheduling \
                           (poison_capability_storm_cannot_block_shard_or_cluster — a poisoned \
                           capability contributes ZERO truth); \
                           arves-conformance::live (live_capability_scheduling_scenario_passes — \
                           dropping the scheduler moves zero committed truth, derived live); \
                           MULTI-AGENT surface (I5, RCR-029/030/031 — the flow decides, only the \
                           Kernel commits; refusals commit nothing): \
                           arves-control-plane::tests::multi_agent_orchestration \
                           (agent_proposals_flow_through_the_scheduler_and_only_the_kernel_commits); \
                           arves-control-plane::tests::multi_agent_adversarial \
                           (partition_minority_proposals_fail_honestly_and_heal_loses_no_attributed_truth — \
                           a minority-side proposal commits NOTHING, never a fork); \
                           arves-conformance::live (live_multi_agent_coordination_scenario_passes)",
            },
        },
        PropertyCheck {
            invariant: Orch002NoPersistentStateInControlPlane,
            proof: ProofKind::CitedTest {
                location: "arves-control-plane::tests::cluster_scheduling (RCR-027): \
                           scheduler_crash_rebuild_from_plan_converges_to_identical_truth_orch001_orch002 \
                           (kill mid-run, rebuild from plan, identical committed truth set — \
                           schedules/placements are discardable plan artifacts); \
                           leader_loss_mid_dispatch_replays_from_record_and_commits_exactly_once \
                           (in-flight work discarded and re-queued under the same key, IDR-004); \
                           adversarial scope (I4 Stage 3, RCR-028): \
                           arves-control-plane::tests::adversarial_scheduling \
                           (node_death_mid_invocation_replaces_and_never_duplicates_commit — the \
                           content-addressed key carries across scheduler death; a fresh scheduler \
                           rebuilt from the plan re-commits idempotently, zero duplicates); \
                           arves-conformance::live (live_capability_scheduling_scenario_passes); \
                           MULTI-AGENT surface (I5, RCR-029/030/031 — identities/policies/approvals/ \
                           decisions live ONLY in the truth base; the flow holds no state): \
                           arves-control-plane::tests::agent_identity \
                           (registered_identity_is_addressable_from_the_shared_world_on_every_replica — \
                           the registry recovers with the truth, never from orchestrator memory); \
                           arves-lcw::tests::shared_world \
                           (hydrated_working_memory_is_derived_disposable_and_never_writes_back); \
                           arves-conformance::live (live_multi_agent_coordination_scenario_passes — \
                           scheduler drop/rebuild converges with zero fresh commits)",
            },
        },
    ]
}

/// Run the suite: `(all_in_process_proofs_pass, proven_count, total)`. A caller/test uses
/// this to gate CI and to report coverage (proven vs pending) without faking a pass.
pub fn run_suite() -> (bool, usize, usize) {
    let cat = catalog();
    let in_process_ok = cat.iter().all(|c| match &c.proof {
        ProofKind::InProcess { pass, .. } => *pass,
        _ => true,
    });
    let proven = cat.iter().filter(|c| c.holds()).count();
    (in_process_ok, proven, cat.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The in-process structural proofs (LAYER-001, OWN-001) hold over the real graph, and
    /// the catalog's coverage map is exactly what we expect — no invariant silently drifts
    /// from Pending to Proven (or vice-versa).
    #[test]
    fn property_check_suite_holds() {
        let (in_process_ok, proven, total) = run_suite();
        assert!(in_process_ok, "an in-process structural proof failed");
        // 7 registered implemented invariants; 7 proven (2 in-process + 5 cited).
        // ORCH-001/002 flipped Pending → CitedTest by RCR-027 (I4 Stage 2, the first
        // Control-Plane behaviour + its §4 executable proofs) — an EXPLICIT, recorded
        // flip (runtime/rcr/RCR-027.md), not silent drift; this assertion is the guard.
        assert_eq!(total, 7, "catalog covers the 7 registered invariants");
        assert_eq!(
            proven, 7,
            "7 proven (LAYER/OWN in-process + ORCH-001/002/003/004 + SHARD-001 cited)"
        );

        // Every in-process entry must carry a live detail string (not empty).
        for c in catalog() {
            if let ProofKind::InProcess { detail, .. } = &c.proof {
                assert!(!detail.is_empty(), "{:?} has no proof detail", c.invariant);
            }
        }
    }

    /// LAYER-001 checker still BITES (an upward edge is flagged) — the property behind the
    /// in-process proof, re-asserted here so the catalog can't pass with a dead checker.
    #[test]
    fn layer_checker_bites() {
        let mut bad = BTreeMap::new();
        bad.insert("arves-persistence".to_string(), vec!["arves-kernel".to_string()]);
        assert!(layering_violations(&bad).iter().any(|s| s.contains("LAYER-001 violation")));
    }
}
