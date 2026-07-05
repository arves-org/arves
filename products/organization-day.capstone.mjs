// ARVES CAPSTONE — a full "organization day" on the frozen Runtime v1.0, proven reproducible.
//
// This ties the whole program into one runnable story and proves the properties no
// ChatGPT/LangGraph/AutoGen wrapper can:
//
//   1. ONE TRUTH        — the same fact from many systems collapses to one content address.
//   2. REPRODUCIBLE     — run the ENTIRE day twice on two fresh Kernels → byte-identical truth
//                          ids, briefing id, decision ids (content-addressing determinism, not luck).
//   3. POLICY AS TRUTH  — a large spend is blocked; a proposer SELF-DECLARING approval does NOT
//                          clear the gate (E1); a SEPARATE committed legal-approval truth does.
//   4. CONSISTENCY      — a cross-department cancel of an approved spend is caught as a conflict.
//   5. DECISION MEMORY  — the CEO's briefing catches that a market signal contradicts a standing
//                          decision, citing the prior decision's truth id as evidence.
//   6. AUDIT / REPLAY   — the committed truth is reconstructable by REPLAYING the Kernel WAL (not
//                          re-reading an in-memory Map): proven by the runtime's own live
//                          conformance node (conformance_live, RCR-010 Query/WAL-replay).
//
// Everything commits through the real bridge → the WAL-backed Rust reference Kernel. This file
// edits no runtime/standard byte; products are customers of the frozen Runtime API (IDR-006).
//
// Run: node products/organization-day.capstone.mjs
//   (requires: cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml;
//    step 6 additionally uses runtime/target/debug/conformance_live — built with the workspace.)

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { KernelBridge } from './arves-sdk-ts/src/bridge.mjs';
import { PersonalCognitiveOS } from './arves-personal-os/src/personal-os.mjs';
import { personalReality } from './arves-personal-os/src/connectors.mjs';
import { EnterpriseCognitiveOS } from './arves-enterprise-os/src/enterprise-os.mjs';

// The org's reality for the day (ERP/CRM/tickets), with one event attested by TWO systems so the
// dedup/one-truth property is exercised at the enterprise layer too.
function enterpriseReality() {
  return [
    { source: 'erp', fact: { entity: 'vendor-x', event: 'invoice-received', at: 1751600000000 } },
    { source: 'crm', fact: { entity: 'vendor-x', event: 'invoice-received', at: 1751600000000 } }, // same → dedup
    { source: 'tickets', fact: { entity: 'incident-42', event: 'sev2-opened', at: 1751603600000 } },
  ];
}

// Run the WHOLE organization day against one Kernel and return a deterministic fingerprint plus
// the property checks. A pure function of its inputs (no clock/RNG), so two fresh Kernels must
// produce identical fingerprints — that equality IS the reproducibility proof.
async function runDay(bridge) {
  // --- The CEO's Personal OS: six systems → one audited, decision-aware briefing ---
  const ceo = new PersonalCognitiveOS(bridge);
  await ceo.recordDecision({ subject: 'invest:acme-fund', action: 'decline', because: 'risk' });
  for (const obs of personalReality()) await ceo.observe(obs);
  const briefing = await ceo.dailyBriefing();
  const personalTruthIds = ceo.truths().map((t) => t.id).sort();
  const q3Truths = ceo.truths().filter((t) => t.fact.event === 'q3-review');

  // --- The Enterprise OS: departments share truth; policy is enforced as truth ---
  const org = new EnterpriseCognitiveOS(bridge);
  const obsResults = [];
  for (const obs of enterpriseReality()) obsResults.push(await org.observe(obs));
  await org.setPolicy({ domain: 'spend', rule: 'spend>100k requires legal approval', thresholdUsd: 100000n });

  // Finance self-declares approval — must NOT clear the gate (E1).
  const selfAttested = await org.proposeDecision({ agent: 'finance', subject: 'spend:vendor-x', action: 'approve', amountUsd: 150000n, approvals: ['legal'] });
  // Legal — a SEPARATE actor — commits an approval truth; finance re-proposes → committed.
  const approvalId = await org.approve({ role: 'legal', subject: 'spend:vendor-x', by: 'legal-counsel' });
  const approved = await org.proposeDecision({ agent: 'finance', subject: 'spend:vendor-x', action: 'approve', amountUsd: 150000n });
  // Ops tries to cancel the approved spend → cross-department conflict.
  const conflict = await org.proposeDecision({ agent: 'ops', subject: 'spend:vendor-x', action: 'cancel' });
  // A compliant small spend must NOT be falsely blocked.
  const small = await org.proposeDecision({ agent: 'finance', subject: 'spend:coffee', action: 'approve', amountUsd: 500n });

  const checks = {
    'enterprise dedup: invoice from 2 systems → one truth': obsResults[1].deduped === true && org.truths().length === 2,
    'personal dedup: q3-review attested by 3 systems → one truth': q3Truths.length === 1 && q3Truths[0].sources.length === 3,
    'personal briefing catches the contradiction with a standing decision': briefing.contradictions.length === 1,
    'policy blocks a >100k spend with self-attested approval (E1)': selfAttested.committed === false && String(selfAttested.reason).includes('legal'),
    'a SEPARATE legal approval truth is committed': typeof approvalId === 'string' && approvalId.length > 0,
    'the spend is allowed once a separate legal approval truth exists': approved.committed === true,
    'cross-department cancel is caught as a conflict': conflict.committed === false && conflict.reason === 'cross-department-conflict',
    'a compliant small spend is NOT falsely blocked': small.committed === true,
  };

  // Deterministic fingerprint — every value is a content address or a policy outcome, so it is a
  // pure function of the scripted day. Two fresh Kernels ⇒ identical fingerprint.
  const fingerprint = {
    briefingId: briefing.id,
    personalTruthIds,
    enterpriseObserve: obsResults.map((r) => r.id),
    selfAttested: { committed: selfAttested.committed, ev: selfAttested.complianceEvent ?? null },
    approvalId,
    approvedId: approved.id ?? null,
    conflict: { committed: conflict.committed, ev: conflict.complianceEvent ?? null },
    smallId: small.id ?? null,
  };
  return { fingerprint, checks };
}

async function main() {
  console.log('ARVES CAPSTONE — a full organization day on the frozen Runtime v1.0\n');

  // Run the entire day TWICE on two independent Kernels.
  const b1 = new KernelBridge();
  const day1 = await runDay(b1);
  b1.close();

  const b2 = new KernelBridge();
  const day2 = await runDay(b2);
  b2.close();

  // 1–5: the day's properties held.
  let allChecks = true;
  for (const [name, passed] of Object.entries(day1.checks)) {
    console.log(`  ${passed ? '✓' : '✗'} ${name}`);
    allChecks = allChecks && passed;
  }

  // 2: reproducibility — the two independent runs are byte-identical.
  const fp1 = JSON.stringify(day1.fingerprint);
  const fp2 = JSON.stringify(day2.fingerprint);
  const reproducible = fp1 === fp2;
  console.log(`  ${reproducible ? '✓' : '✗'} REPRODUCIBLE — the entire day, re-run on a fresh Kernel, is byte-identical`);
  if (!reproducible) {
    console.log('    run1:', fp1);
    console.log('    run2:', fp2);
  }

  // 6: audit/replay — the runtime's own live conformance proves committed truth is
  // reconstructable by replaying the Kernel WAL (RCR-010 Query node), not re-reading a Map.
  let replayProven = false;
  let replayNote = '';
  const liveBin = process.platform === 'win32'
    ? 'runtime/target/debug/conformance_live.exe'
    : 'runtime/target/debug/conformance_live';
  if (existsSync(liveBin)) {
    try {
      const out = execFileSync(liveBin, { encoding: 'utf8' });
      replayProven = /LIVE-L1:\s*PASS/.test(out);
      replayNote = replayProven ? '' : ` (marker not found in output)`;
    } catch (e) {
      replayNote = ` (conformance_live failed: ${e.message})`;
    }
  } else {
    replayNote = ' (conformance_live not built — cargo build the workspace to prove this arm)';
  }
  console.log(`  ${replayProven ? '✓' : '○'} AUDIT/REPLAY — committed truth reconstructable by WAL replay (conformance_live, RCR-010)${replayNote}`);

  // Verdict: the JS-provable properties (1–5) and reproducibility (2) are REQUIRED; the WAL-replay
  // arm (6) is REQUIRED when the bin is present and is an honest ○ (not a ✗) if the workspace was
  // not built — the capstone must not fail a user who only built the bridge.
  const replayOk = replayProven || !existsSync(liveBin);
  const ok = allChecks && reproducible && replayOk;

  console.log('\nWhat a wrapper cannot do — proven, not asserted:');
  console.log('  one truth across systems · reproducible cognition · policy enforced as truth ·');
  console.log('  cross-department consistency · decision-aware memory · replayable audit trail.');
  console.log(ok
    ? '\nCAPSTONE PASS — a governed organization ran a full day of cognition as reproducible, '
      + 'auditable truth on the SAME frozen ARVES Runtime v1.0 that carries every other product.'
    : '\nCAPSTONE FAIL — a load-bearing property did not hold (see above).');

  // Assert so a CI/probe run exits non-zero on regression.
  assert.ok(allChecks, 'day properties');
  assert.ok(reproducible, 'reproducibility');
  assert.ok(replayOk, 'WAL-replay conformance (when built)');
  process.exit(ok ? 0 : 1);
}

main().catch((e) => { console.error('CAPSTONE ERROR:', e); process.exit(1); });
