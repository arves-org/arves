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
//! (ORCH-001/002) are **pending**: their owning crates are contract-only (I2+), so no runtime
//! proof exists yet, and the catalog says so rather than faking coverage.
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
                           (adversarial_dup_reorder_storm_commits_each_entry_exactly_once)",
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
                           (adversarial_full_cluster_replay_from_wal_rebuilds_identical_truth)",
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
                           arves-consensus::tests::shard_map (RCR-020 blast-radius isolation)",
            },
        },
        // ORCH-001 / ORCH-002 — Control-Plane truth/state ownership. Owning crate is
        // contract-only until I2; no runtime proof yet (honest gap).
        PropertyCheck {
            invariant: Orch001ControlPlaneOwnsNoTruth,
            proof: ProofKind::Pending {
                reason: "arves-control-plane is contract-only (I2+); no executable runtime proof yet",
            },
        },
        PropertyCheck {
            invariant: Orch002NoPersistentStateInControlPlane,
            proof: ProofKind::Pending {
                reason: "arves-control-plane is contract-only (I2+); no executable runtime proof yet",
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
        // 7 registered implemented invariants; 5 proven (2 in-process + 3 cited),
        // 2 pending (ORCH-001/002, Control-Plane, contract-only).
        assert_eq!(total, 7, "catalog covers the 7 registered invariants");
        assert_eq!(proven, 5, "5 proven (LAYER/OWN in-process + ORCH-003/004 + SHARD-001 cited)");

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
