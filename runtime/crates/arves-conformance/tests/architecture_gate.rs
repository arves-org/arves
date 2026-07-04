//! ARVES build-time Architecture Gate (Verification Office / verification-runtime).
//!
//! Turns two registered invariants from documented rules into COMPILER/CI-enforced
//! rules over the actual Cargo workspace graph:
//!   * LAYER-001 — dependencies are downward-only: every internal crate edge must
//!     point to a strictly-lower architectural layer rank (which also forbids cycles).
//!   * OWN-001 — one owner of truth: exactly ONE crate (arves-kernel) may define the
//!     truth-commit gateway (`pub trait Kernel`).
//!
//! The pure checkers now live in `arves_conformance::property_check` (RCR-006) so the gate
//! and the PropertyCheck invariant→proof catalog share ONE implementation. This file keeps
//! the gate tests over the real workspace + the synthetic tests proving the checker bites.

use arves_conformance::property_check::{layering_violations, parse_internal_deps, truth_gateway_owners, workspace_edges};
use std::collections::BTreeMap;

// --- gate tests over the real workspace --------------------------------------

#[test]
fn layer_001_dependencies_are_downward_only() {
    let edges = workspace_edges();
    assert!(!edges.is_empty(), "no arves-* crates found");
    let violations = layering_violations(&edges);
    assert!(
        violations.is_empty(),
        "LAYER-001 architecture gate failed:\n{}",
        violations.join("\n")
    );
}

#[test]
fn own_001_single_truth_commit_gateway() {
    let owners = truth_gateway_owners();
    assert_eq!(
        owners,
        vec!["arves-kernel".to_string()],
        "OWN-001 architecture gate: the truth-commit gateway (`pub trait Kernel`) \
         must be defined by exactly one crate (arves-kernel); found: {owners:?}"
    );
}

// --- synthetic tests proving the gate BITES ----------------------------------

#[test]
fn gate_detects_upward_edge() {
    // persistence(20) -> kernel(40) is an illegal upward edge.
    let mut bad = BTreeMap::new();
    bad.insert("arves-persistence".to_string(), vec!["arves-kernel".to_string()]);
    let v = layering_violations(&bad);
    assert!(
        v.iter().any(|s| s.contains("LAYER-001 violation")),
        "checker failed to flag an upward edge: {v:?}"
    );
}

#[test]
fn gate_detects_unranked_crate() {
    let mut bad = BTreeMap::new();
    bad.insert("arves-mystery".to_string(), vec![]);
    let v = layering_violations(&bad);
    assert!(v.iter().any(|s| s.contains("unranked")), "checker failed to flag an unranked crate");
}

#[test]
fn parser_handles_both_dependency_forms() {
    let toml = "\
[package]\nname = \"x\"\n\n[dependencies]\narves-persistence = { path = \"../arves-persistence\" }\narves-consensus.workspace = true\nserde = \"1\"\n\n[dev-dependencies]\narves-conformance = { path = \"../arves-conformance\" }\n";
    let deps = parse_internal_deps(toml);
    assert_eq!(deps, vec!["arves-persistence".to_string(), "arves-consensus".to_string()]);
    assert!(!deps.iter().any(|d| d == "arves-conformance"), "dev-deps must be excluded");
}
