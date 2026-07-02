// ARVES Enterprise Cognitive OS — flagship demo.
//
// Multiple department agents share ONE content-addressed truth base; governance policy is
// enforced as truth; every violation, approval, and conflict is a replayable compliance
// event in the real Kernel. None of this is achievable with an AI wrapper — a wrapper has
// no shared enforced truth, no compliance ledger, and no cross-agent consistency.
//
// Runs on the FROZEN Runtime v1.0 (SDK + Bridge). Edits no runtime file.
// Run: node examples/enterprise-day.mjs   (requires: cargo build -p arves-bridge --bin arves-bridge)

import { KernelBridge } from '../../arves-sdk-ts/src/bridge.mjs';
import { EnterpriseCognitiveOS } from '../src/enterprise-os.mjs';

const bridge = new KernelBridge();
const org = new EnterpriseCognitiveOS(bridge);

console.log('ARVES Enterprise Cognitive OS — governed multi-agent cognition\n');

// Governance: large spend requires legal approval (committed as an addressable policy truth).
await org.setPolicy({ domain: 'spend', rule: 'spend>100k requires legal approval', thresholdUsd: 100000n });

// Finance agent proposes a $150k spend with no legal approval → BLOCKED by policy.
const d1 = await org.proposeDecision({ agent: 'finance', subject: 'spend:vendor-x', action: 'approve', amountUsd: 150000n, approvals: [] });
console.log('[finance] approve $150k spend:vendor-x  →', d1.committed ? 'committed' : `BLOCKED (${d1.reason})`);

// Legal approves; finance re-proposes WITH legal approval → allowed, committed as truth.
const d2 = await org.proposeDecision({ agent: 'finance', subject: 'spend:vendor-x', action: 'approve', amountUsd: 150000n, approvals: ['legal'] });
console.log('[finance] re-propose with legal approval →', d2.committed ? `committed ${d2.id.slice(0, 16)}…` : `BLOCKED (${d2.reason})`);

// Ops agent later tries to cancel the approved spend → cross-department CONFLICT, blocked.
const d3 = await org.proposeDecision({ agent: 'ops', subject: 'spend:vendor-x', action: 'cancel' });
console.log('[ops]     cancel spend:vendor-x         →', d3.committed ? 'committed' : `BLOCKED (${d3.reason}, conflicts with ${String(d3.prior).slice(0, 16)}…)`);

bridge.close();

// A different, compliant decision must NOT be falsely blocked.
const bridge2 = new KernelBridge();
const org2 = new EnterpriseCognitiveOS(bridge2);
await org2.setPolicy({ domain: 'spend', rule: 'spend>100k requires legal approval', thresholdUsd: 100000n });
const small = await org2.proposeDecision({ agent: 'finance', subject: 'spend:coffee', action: 'approve', amountUsd: 500n });
bridge2.close();

console.log('\nWhy this is impossible without ARVES:');
const policyBlocks = d1.committed === false && d1.reason.includes('legal');
const allowedAfterApproval = d2.committed === true;
const conflictCaught = d3.committed === false && d3.reason === 'cross-department-conflict';
const noFalseBlock = small.committed === true;
console.log('  policy enforced as truth (violation blocked + audited):', policyBlocks);
console.log('  allowed once compliant (legal approval)               :', allowedAfterApproval);
console.log('  cross-department conflict detected (shared truth)      :', conflictCaught);
console.log('  compliant decision not falsely blocked                 :', noFalseBlock);

const ok = policyBlocks && allowedAfterApproval && conflictCaught && noFalseBlock;
console.log(ok
  ? '\nGoverned, multi-agent, compliant, auditable cognition on the frozen ARVES Runtime v1.0.\nA second, different product on the SAME unchanged runtime — that is a platform.'
  : '\nFAIL: an Enterprise-OS property did not hold.');
process.exit(ok ? 0 : 1);
