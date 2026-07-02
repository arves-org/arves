//! ARVES build-time Architecture Gate (Verification Office / verification-runtime).
//!
//! Turns two registered invariants from documented rules into COMPILER/CI-enforced
//! rules over the actual Cargo workspace graph:
//!   * LAYER-001 — dependencies are downward-only: every internal crate edge must
//!     point to a strictly-lower architectural layer rank (which also forbids
//!     cycles). Ranks encode the frozen ARVES layer stack.
//!   * OWN-001 — one owner of truth: exactly ONE crate (arves-kernel) may define
//!     the truth-commit gateway (`pub trait Kernel`).
//!
//! The gate reads Cargo.toml / src at test time (no external crates), so a
//! violating edge or a second truth-owner FAILS `cargo test`. Synthetic tests at
//! the bottom prove the checker actually bites.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Architectural layer rank per crate (higher = higher layer). An internal
/// dependency edge A -> B is legal iff rank(A) > rank(B) (downward-only,
/// LAYER-001). Ranks are spaced to allow future insertion and derive from the
/// frozen layer stack: foundation < persistence < consensus < kernel < lcw <
/// query/engine < capability/execution < information-platform < control-plane <
/// runtime < conformance(harness). An unranked crate is a gate failure (it must
/// be classified before it can ship).
fn layer_rank(name: &str) -> Option<i32> {
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
        "arves-control-plane" => 90,                 // decides, owns no truth
        "arves-runtime" => 100,                      // top orchestrator (bin)
        "arves-conformance" => 110,                  // test harness (may read all)
        _ => return None,
    })
}

fn crates_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR = .../runtime/crates/arves-conformance ; parent = crates/
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .to_path_buf()
}

/// Parse the internal (arves-*) dependency names from a Cargo.toml's
/// `[dependencies]` section only (dev/build deps are excluded on purpose).
fn parse_internal_deps(toml: &str) -> Vec<String> {
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
        // key = token before the first whitespace, '=' or '.' (covers both
        // `arves-x = { path = ".." }` and `arves-x.workspace = true`).
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

/// Read the actual workspace edge set: crate -> [internal deps].
fn workspace_edges() -> BTreeMap<String, Vec<String>> {
    let mut edges = BTreeMap::new();
    for entry in fs::read_dir(crates_dir()).expect("read crates dir").flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("arves-") {
            continue;
        }
        let cargo = entry.path().join("Cargo.toml");
        let toml = match fs::read_to_string(&cargo) {
            Ok(t) => t,
            Err(_) => continue,
        };
        edges.insert(name, parse_internal_deps(&toml));
    }
    edges
}

/// Pure LAYER-001 checker: returns a list of human-readable violations for the
/// given edge set. Empty = clean.
fn layering_violations(edges: &BTreeMap<String, Vec<String>>) -> Vec<String> {
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

/// Crates whose source DEFINES the truth-commit gateway (`pub trait Kernel`).
fn truth_gateway_owners() -> Vec<String> {
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
