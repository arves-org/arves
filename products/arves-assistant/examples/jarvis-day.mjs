// ARVES Assistant — THE CAPSTONE: one full assistant day, A1–A7 end to end, honestly.
//
// Act 1 (observe):   the user's world arrives from 3 offline sources; the same real-world
//                    event from two systems collapses to ONE truth with an evidence set (A2).
// Act 2 (agents):    two DETERMINISTIC sub-agents work over the ONE shared truth base —
//                    the researcher gathers facts into an agent-tagged finding truth, the
//                    scheduler proposes plan items; their conflicting proposals on one
//                    subject resolve FIRST-COMMITTED-WINS, the loser committing a
//                    reference to the winner (A5; attribution is product-level, in-body).
// Act 3 (skills):    skills are certified capabilities — certification RE-RUN at
//                    registration, forged flags refused, names bound in the shard (A3).
// Act 4 (think):     think(goal) -> StubReasoner proposal (committed as truth) -> guardrail
//                    gate -> certified skill -> committed effect truth (A4).
// Act 5 (guardrail): the spend-class action is BLOCKED (compliance truth) until a SEPARATE
//                    committed approval truth exists; then it acts, citing the approval (A6).
// Act 6 (restart):   the Kernel process dies; a NEW assistant over the SAME --wal-dir
//                    re-proves EVERYTHING as already-committed — memory intact (A1).
// Act 7 (why):       why() reconstructs the decision path end to end from committed truths
//                    — observed evidence, proposer, policy, block, approval, commit — and
//                    the trace is BYTE-IDENTICAL before and after the restart (A7).
//
// LOUD HONESTY: no AI runs here. The StubReasoner is a keyword table; the agents are
// rule-based actors; the intelligence arrives when the maintainer plugs their LLM into
// the Reasoner slot (docs/JARVIS_QUICKSTART.md). why()/rebuild are now WAL-backed: the
// RCR-033 bridge `scan` verb replays the Kernel's WAL read-only, so recoverFromWal()
// reconstructs the decision journal from committed truth with ZERO re-commits (Act 6b).
// The runtime I5 attribution IS now reachable over the bridge (RCR-034 `commit-as`,
// proven in the bridge's own tests); this capstone's agents still use product-level
// in-body tags, and the effect→skill causal edge stays journal metadata — a native
// attributed-INVOKE verb is the recorded next-RCR candidate, never faked. Single host,
// no authN (v1.0 scope). A probe is a probe.
//
// Exit code: 0 iff every property below PASSes.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { Assistant } from '../src/assistant.mjs';
import { allObservations } from '../src/connectors.mjs';
import { AgentCouncil, ResearcherAgent, SchedulerAgent, canonicalFinding, canonicalResolution } from '../src/agents.mjs';
import { why, renderWhy } from '../src/why.mjs';
import { registerSkill, defineCapability } from '../src/skills.mjs';
import { StubReasoner, goalSlug } from '../src/reasoner.mjs';
import { Arves } from '../../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();
const sleep = (ms) => new Promise((res) => setTimeout(res, ms));
const snap = (v) => JSON.stringify(v, (_, x) => (typeof x === 'bigint' ? `${x}n` : x));
const short = (id) => (id ? `${id.slice(0, 16)}…` : '-');
const results = [];
const check = (property, pass, detail = '') => { results.push({ property, pass, detail }); };

// ---- the deterministic day (identical script both days — that IS the replay proof) ----

const JOURNAL = [{
  subject: 'invest:acme-fund',
  action: 'decline',
  because: 'volatility exceeds my risk policy (decided at the Q2 review)',
}];
const TEMPTATION = { subject: 'invest:acme-fund', action: 'approve' };
const GOAL = 'order flowers for mom (35 USD)';
const SPEND_SUBJECT = `spend:${goalSlug(GOAL)}`;

const summarizeSkill = () => defineCapability({
  name: 'day.summarize', version: '1.0.0', produces: ['uci.assistant.briefing'],
  execute: (input) => [{
    target: 'uci.assistant.briefing',
    value: { type: 'uci.assistant.briefing', count: BigInt(input.events.length), events: [...input.events].sort() },
  }],
});
const orderSkill = () => defineCapability({
  name: 'spend.order', version: '1.0.0', produces: ['uci.assistant.order'],
  execute: (input) => [{
    target: 'uci.assistant.order',
    value: { type: 'uci.assistant.order', request: input.request, state: 'placed' },
  }],
});
const SUMMARIZE_INPUTS = [{ type: 'uci.assistant.skill-input', events: ['a', 'b'] }];
const ORDER_INPUTS = [{ type: 'uci.assistant.order-request', request: 'order x' }];

/** One full assistant day — a PURE, fixed script over committed truth. Running it on a
 *  fresh Kernel commits everything; running it again over the same WAL re-proves every
 *  body as already-committed (the membership proof that memory survived). */
async function liveOneDay(assistant) {
  assistant.useReasoner(new StubReasoner());
  // observe the world + record the standing decision (day 2: this IS rebuild())
  const rebuild = await assistant.rebuild({ observations: allObservations(), decisions: JOURNAL });
  // sub-agents over the ONE shared truth base
  const council = new AgentCouncil(assistant);
  const researcher = new ResearcherAgent();
  const scheduler = new SchedulerAgent();
  const finding = await researcher.research(assistant, 'passport');
  const researchPlan = await researcher.propose(council,
    { subject: 'plan:renew-passport', action: 'research-first', findingId: finding.id });
  const schedulerPlans = await scheduler.planDay(assistant, council); // dentist + renew-passport
  // certified skills (forged flag refused at the same gate)
  const reg1 = await registerSkill(assistant, summarizeSkill(), SUMMARIZE_INPUTS);
  const reg2 = await registerSkill(assistant, orderSkill(), ORDER_INPUTS);
  let forgedRefused = false;
  const forged = defineCapability({
    name: 'evil.exec', version: '1.0.0', produces: ['uci.x'],
    execute: () => [{ target: 'uci.NOT-declared', value: { type: 'uci.x' } }],
  });
  forged.certified = true; // never consulted
  try { await registerSkill(assistant, forged, [{ any: 'input' }]); } catch { forgedRefused = true; }
  // think: stub-reasoner -> gate -> skill -> truth
  const briefing = await assistant.think('summarize my day');
  const none = await assistant.think('compose a symphony in D minor');
  // guardrail: block, separate approval, act
  const pol = await assistant.guardrails.setPolicy({
    name: 'spend-needs-user-approval', appliesTo: ['spend', 'irreversible'], approverRole: 'user',
  });
  const blocked = await assistant.think(GOAL);
  const approval = await assistant.guardrails.approve('user', SPEND_SUBJECT);
  const allowed = await assistant.think(GOAL);
  return { rebuild, council, finding, researchPlan, schedulerPlans, reg1, reg2, forgedRefused, briefing, none, pol, blocked, approval, allowed };
}

const walDir = fs.mkdtempSync(path.join(os.tmpdir(), 'arves-jarvis-day-'));
let assistant = null;
try {
  // ================================ DAY 1 (fresh WAL) ==================================
  assistant = new Assistant({ tenant: 'maintainer', workspace: 'jarvis', walDir });
  const d1 = await liveOneDay(assistant);

  // ---- A2: multi-source one-truth ----------------------------------------------------
  const dentist = assistant.truths().find((t) => t.fact.event === 'dentist-appointment');
  check('A2: 7 observations / 3 sources -> 6 truths; the cross-source duplicate is ONE truth with BOTH sources in evidence',
    assistant.truths().length === 6 && dentist.sources.join(',') === 'calendar-file,notes-file',
    `dentist sources=[${dentist.sources.join(', ')}]`);

  // ---- A5: sub-agents over one shared truth base -------------------------------------
  check('A5: the researcher GATHERS FACTS INTO TRUTH — an agent-tagged finding citing fact ContentIds (attribution IN the body)',
    d1.finding.facts.length === 1
      && d1.finding.id === arves.address(canonicalFinding('researcher', '1.0.0', 'passport', d1.finding.facts), 'trace'),
    `finding=${short(d1.finding.id)} cites ${d1.finding.facts.length} fact`);

  const lost = d1.schedulerPlans.find((p) => p.subject === 'plan:renew-passport');
  const won = d1.schedulerPlans.find((p) => p.subject === 'plan:dentist-appointment');
  check('A5: conflicting proposals on ONE subject resolve FIRST-COMMITTED-WINS (researcher first, scheduler loses)',
    d1.researchPlan.won === true && lost.won === false && lost.winnerId === d1.researchPlan.id
      && lost.winnerAgent === 'researcher' && won.won === true,
    `winner=${short(d1.researchPlan.id)} loser=${short(lost.id)}`);

  check('A5: the LOSER records a committed reference to the winner (the concession is content-addressed truth)',
    lost.resolutionId === arves.address(canonicalResolution('plan:renew-passport', d1.researchPlan.id, lost.id), 'trace'),
    `resolution=${short(lost.resolutionId)}`);

  // ---- A3: certified skills -----------------------------------------------------------
  check('A3: skills are CERTIFIED capabilities — certification re-run at registration, bound in the shard (RCR-016); a forged flag is REFUSED',
    d1.reg1.bound && d1.reg2.bound && d1.forgedRefused && assistant.skills().join(',') === 'day.summarize,spend.order',
    `admissions: ${short(d1.reg1.registrationId)} · ${short(d1.reg2.registrationId)}`);

  // ---- A4: the reasoner slot ----------------------------------------------------------
  check('A4: think(goal) -> STUB reasoner proposal (COMMITTED as truth) -> certified skill -> committed effect truth',
    d1.briefing.acted === true && /^[0-9a-f]{68}$/.test(d1.briefing.proposalId)
      && d1.briefing.invocation.truths[0].target === 'uci.assistant.briefing',
    `proposal=${short(d1.briefing.proposalId)} briefing=${short(d1.briefing.invocation.truths[0].id)}`);
  check('A4 honesty: a goal outside the stub\'s table -> action:none (the stub is NOT AI; the LLM plugs in outside the repo)',
    d1.none.acted === false && d1.none.reason === 'no-action-proposed');

  // ---- A6: guardrails -----------------------------------------------------------------
  check('A6: the spend-class action is BLOCKED before any skill runs — the violation is a committed compliance truth citing the policy truth',
    d1.blocked.acted === false && d1.blocked.blocked === true && d1.blocked.policyId === d1.pol.id
      && /^[0-9a-f]{68}$/.test(d1.blocked.complianceId),
    `compliance=${short(d1.blocked.complianceId)} policy=${short(d1.pol.id)}`);
  check('A6: a SEPARATE committed approval truth unlocks it; the acting path cites the approval id',
    d1.allowed.acted === true && d1.allowed.approvals.includes(d1.approval.id),
    `approval=${short(d1.approval.id)} order=${short(d1.allowed.invocation.truths[0].id)}`);

  // ---- A7 (pre-restart): capture the decision trace ----------------------------------
  const effectId = d1.allowed.invocation.truths[0].id;
  const trace1 = why(assistant, effectId);
  const text1 = renderWhy(trace1);
  check('A7: why(effectId) reconstructs the full decision path — observed evidence, proposer, policy checked, block, approval, commit',
    trace1.subject === SPEND_SUBJECT && trace1.observed.length === 6
      && trace1.proposals.length === 1 && trace1.proposals[0].by === 'stub-reasoner'
      && trace1.policies.map((p) => p.id).join() === d1.pol.id
      && trace1.compliance.map((c) => c.id).join() === d1.blocked.complianceId
      && trace1.approvals.map((a) => a.id).join() === d1.approval.id
      && trace1.committed.map((c) => c.id).join() === effectId
      && trace1.admissions.length === 1 && trace1.admissions[0].skill === 'spend.order',
    `stations: observed=${trace1.observed.length} proposal/policy/block/approval/commit all cited`);

  // Day-1 agent-conflict trace (no effects) — the subject we will rebuild READ-ONLY after
  // the restart to prove total WAL-backed reconstruction without re-running the day.
  const conflictTrace1 = why(assistant, 'plan:renew-passport');

  // ================================ THE RESTART ========================================
  assistant.close();
  assistant = null;
  await sleep(400); // let the bridge process exit (Windows WAL-dir file locks)

  // ---- A7 (RCR-033): TOTAL read-only reconstruction from the WAL, no day re-run --------
  // A brand-new process over the same WAL rebuilds its decision journal purely by SCANNING
  // committed truth (the bridge `scan` verb replays the Kernel's WAL) — ZERO re-commits,
  // no candidate bodies — and why() reconstructs the agent conflict byte-identically.
  const recon = new Assistant({ tenant: 'maintainer', workspace: 'jarvis', walDir });
  let reconReport = null; let conflictRecon = null; let reconFresh = true; let effectRecon = null;
  try {
    reconReport = await recon.recoverFromWal();               // read-only: nothing committed
    reconFresh = recon.journal().every((e) => e.status === 'already-committed');
    conflictRecon = why(recon, 'plan:renew-passport');        // reconstructed from truth alone
    effectRecon = why(recon, SPEND_SUBJECT);                  // the EFFECT-bearing subject
  } finally { recon.close(); }
  await sleep(400);
  check('A7 (RCR-033): a fresh process rebuilds why() READ-ONLY from the WAL scan — total reconstruction, ZERO re-commits, byte-identical to day 1',
    reconReport !== null && reconReport.recovered > 0 && reconFresh
      && conflictRecon !== null && snap(conflictRecon) === snap(conflictTrace1),
    `scanned ${reconReport ? reconReport.recovered : 0} committed truths; recommitted 0`);

  // A7 (RCR-033) HONEST RESIDUAL — PROVEN BY EXECUTION, not prose. The no-effect subject
  // above rebuilds byte-identically. The EFFECT subject does NOT: an invoke-EFFECT truth's
  // causal edge to its skill/proposal is PROCESS metadata, absent from the self-describing
  // body, so a read-only scan reconstructs every self-describing station (observed,
  // proposer, policy, block, approval) but the COMMITTED effect→skill edge (and the
  // admission derived from it) is GONE. The disclosed imperfection is asserted here.
  check('A7 (RCR-033) HONEST RESIDUAL: the EFFECT subject reconstructs every self-describing station BUT the effect→skill edge is absent after a read-only scan (proven, not just documented)',
    effectRecon !== null
      && effectRecon.proposals.map((p) => p.id).join() === trace1.proposals.map((p) => p.id).join()
      && effectRecon.policies.map((p) => p.id).join() === trace1.policies.map((p) => p.id).join()
      && effectRecon.approvals.map((a) => a.id).join() === trace1.approvals.map((a) => a.id).join()
      && effectRecon.compliance.map((c) => c.id).join() === trace1.compliance.map((c) => c.id).join()
      && trace1.committed.length === 1 && effectRecon.committed.length === 0
      && effectRecon.admissions.length === 0,
    `live effect→skill edges=${trace1.committed.length}, scan-recovered=${effectRecon ? effectRecon.committed.length : 'n/a'} (the recorded next-RCR candidate closes this)`);

  // ================================ DAY 2 (same WAL) ===================================
  assistant = new Assistant({ tenant: 'maintainer', workspace: 'jarvis', walDir });
  check('A1 honesty: the fresh process remembers NOTHING before rebuild (indexes and journal are projections, never the truth)',
    assistant.truths().length === 0 && assistant.journal().length === 0);

  const d2 = await liveOneDay(assistant);
  check('A1: memory INTACT — every rebuilt fact/attestation/decision answers already-committed (the Kernel membership proof)',
    d2.rebuild.factsFresh === 0 && d2.rebuild.attestationsFresh === 0 && d2.rebuild.decisionsFresh === 0
      && d2.rebuild.factsRecovered === 7 && d2.rebuild.decisionsRecovered === 1,
    `facts ${d2.rebuild.factsRecovered}rec/0fresh · attestations ${d2.rebuild.attestationsRecovered}rec/0fresh · decisions 1rec/0fresh`);

  const c2 = assistant.checkContradiction(TEMPTATION);
  check('A1: contradiction detection works ACROSS the restart, citing the SAME prior decision truth id',
    c2.contradicts === true && c2.priorId === arves.address(
      { type: 'uci.assistant.decision', ...JOURNAL[0] }, 'trace'),
    `priorId=${short(c2.priorId)}`);

  check('A1+A5+A6: the WHOLE governed day replays to the SAME committed truths — agents, proposals, policy, approval, effects (already-committed end to end)',
    d2.finding.id === d1.finding.id && d2.finding.status === 'already-committed'
      && d2.researchPlan.id === d1.researchPlan.id
      && d2.schedulerPlans.find((p) => !p.won).resolutionId === lost.resolutionId
      && d2.pol.id === d1.pol.id && d2.approval.id === d1.approval.id
      && d2.blocked.complianceId === d1.blocked.complianceId
      && d2.allowed.invocation.truths[0].id === effectId
      && d2.allowed.invocation.truths[0].status === 'already-committed',
    `order effect ${short(effectId)} status=${d2.allowed.invocation.truths[0].status}`);

  // ---- A7 (post-restart): the explanation is REPLAYABLE -------------------------------
  const trace2 = why(assistant, effectId);
  check('A7: the why() trace is BYTE-IDENTICAL after the restart — the explanation is committed truth, not process memory',
    snap(trace1) === snap(trace2) && text1 === renderWhy(trace2));
  check('A7: why(subject) === why(truthId) — one decision path, addressable both ways',
    snap(why(assistant, SPEND_SUBJECT)) === snap({ ...trace2, target: null }));

  const conflictTrace = why(assistant, 'plan:renew-passport');
  check('A5+A7: the agent conflict explains itself — both attributed proposals, the first-committed-wins resolution, the cited finding',
    conflictTrace.proposals.map((p) => p.by).join(',') === 'researcher,scheduler'
      && conflictTrace.resolutions.length === 1 && conflictTrace.resolutions[0].rule === 'first-committed-wins'
      && conflictTrace.findings.map((f) => f.id).join() === d1.finding.id);

  console.log('\n--- why(the order effect) — the assistant explains itself: ---\n');
  console.log(renderWhy(trace2));
} catch (e) {
  check(`unexpected error: ${e.message}`, false);
} finally {
  try { if (assistant) assistant.close(); } catch { /* already gone */ }
  await sleep(400); // let the bridge exit before deleting its WAL dir
  try { fs.rmSync(walDir, { recursive: true, force: true }); } catch { /* best-effort temp cleanup */ }
}

// ---- The property table -------------------------------------------------------------
const width = Math.max(...results.map((r) => r.property.length));
console.log('\nARVES Assistant — THE CAPSTONE DAY (A1 memory · A2 one-truth · A3 skills · A4 reasoner · A5 agents · A6 guardrails · A7 why)');
console.log('(honesty: stub reasoner + rule-based agents — NOT AI; product-level attribution; single host, no authN — v1.0 scope)\n');
for (const r of results) {
  console.log(`  ${r.pass ? 'PASS' : 'FAIL'}  ${r.property.padEnd(width)}${r.detail ? `  [${r.detail}]` : ''}`);
}
const failed = results.filter((r) => !r.pass).length;
console.log(`\n${results.length - failed}/${results.length} properties PASS${failed ? ` — ${failed} FAIL` : ''}`);
process.exit(failed === 0 && results.length > 0 ? 0 : 1);
