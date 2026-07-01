//! I1.5 Persistent WAL - B11: real cross-process restart proof.
//!
//! This is the milestone's headline behaviour and the genuine upgrade over I1.4:
//! process A (`arves-runtime write <dir>`) commits + fsyncs + EXITS; a distinct
//! process B (`arves-runtime recover <dir>`) starts fresh, replays the on-disk
//! WAL, and computes the SAME truth_hash. No shared memory - only the directory.

use std::path::PathBuf;
use std::process::Command;

fn tmp(sub: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    p.push(sub);
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("create tmp dir");
    p
}

fn field(stdout: &[u8], key: &str) -> String {
    let s = String::from_utf8_lossy(stdout);
    s.lines()
        .find_map(|l| l.strip_prefix(key).map(|v| v.trim().to_string()))
        .unwrap_or_else(|| panic!("missing {key} in output:\n{s}"))
}

#[test]
fn behaviour_11_real_cross_process_restart() {
    let dir = tmp("b11_real_restart");
    let bin = env!("CARGO_BIN_EXE_arves-runtime");

    // Process A: commit to a file WAL, fsync, print truth, exit.
    let out_w = Command::new(bin)
        .arg("write")
        .arg(&dir)
        .output()
        .expect("spawn write process");
    assert!(
        out_w.status.success(),
        "write process failed: status={:?} stderr={}",
        out_w.status,
        String::from_utf8_lossy(&out_w.stderr)
    );
    let hash_w = field(&out_w.stdout, "TRUTH_HASH=");
    let count_w = field(&out_w.stdout, "COUNT=");

    // Process B: a genuinely separate OS process recovers from the directory.
    let out_r = Command::new(bin)
        .arg("recover")
        .arg(&dir)
        .output()
        .expect("spawn recover process");
    assert!(
        out_r.status.success(),
        "recover process failed: status={:?} stderr={}",
        out_r.status,
        String::from_utf8_lossy(&out_r.stderr)
    );
    let hash_r = field(&out_r.stdout, "TRUTH_HASH=");
    let count_r = field(&out_r.stdout, "COUNT=");

    assert_eq!(count_w, "3", "process A committed 3 truths");
    assert_eq!(count_r, "3", "process B recovered 3 truths");
    assert_ne!(hash_w, "0x0000000000000000", "non-trivial truth hash");
    assert_eq!(
        hash_w, hash_r,
        "truth hash is identical across a real process restart (disk durability)"
    );
}

/// Recovery is idempotent across repeated fresh processes: running `recover`
/// again yields the same truth and never grows the log.
#[test]
fn behaviour_11b_repeated_recover_is_stable() {
    let dir = tmp("b11b_repeat_recover");
    let bin = env!("CARGO_BIN_EXE_arves-runtime");

    let w = Command::new(bin).arg("write").arg(&dir).output().unwrap();
    assert!(w.status.success());
    let hash0 = field(&w.stdout, "TRUTH_HASH=");

    for _ in 0..3 {
        let r = Command::new(bin).arg("recover").arg(&dir).output().unwrap();
        assert!(r.status.success());
        assert_eq!(field(&r.stdout, "TRUTH_HASH="), hash0);
        assert_eq!(field(&r.stdout, "COUNT="), "3");
    }
}
