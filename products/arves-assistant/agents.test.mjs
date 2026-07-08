// ARVES Assistant — stage-3 product-local tests (assert-based, exit 0/1).
//
// Covers A5 + A7: agent-tagged proposal truths (attribution IS the committed body) ·
// deterministic first-committed-wins conflict resolution with the loser's committed
// reference to the winner · researcher findings gathering facts into truth ·
// why() decision-path reconstruction (structured trace + printable rendering) with
// honest refusals for unknown ids/subjects. Offline, deterministic, no third-party
// deps; every bridge closed in finally. In-memory Kernel suffices here (durability +
// the restart-identical why() trace are the capstone's proof, examples/jarvis-day.mjs).

import assert from 'node:assert/strict';
import { Assistant } from './src/assistant.mjs';
import {
  AgentCouncil, ResearcherAgent, SchedulerAgent,
  canonicalAgentProposal, canonicalFinding, canonicalResolution,
} from './src/agents.mjs';
import { why, renderWhy } from './src/why.mjs';
import { registerSkill, defineCapability } from './src/skills.mjs';
import { StubReasoner, goalSlug } from './src/reasoner.mjs';
import { Arves } from '../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();
const snap = (v) => JSON.stringify(v, (_, x) => (typeof x === 'bigint' ? `${x}n` : x));

const orderSkill = () => defineCapability({
  name: 'spend.order',
  version: '1.0.0',
  produces: ['uci.assistant.order'],
  actionClass: 'spend', // risk class is bound to the SKILL; the gate keys on this, not the proposal
  execute: (input) => [{
    target: 'uci.assistant.order',
    value: { type: 'uci.assistant.order', request: input.request, state: 'placed' },
  }],
});
const ORDER_INPUTS = [{ type: 'uci.assistant.order-request', request: 'order x' }];

const tests = [];
const test = (name, fn) => tests.push({ name, fn });

test('A5: an agent proposal is a COMMITTED, agent-tagged truth (attribution is in the body, content-addressed)', async () => {
  const a = new Assistant({ tenant: 'agents', workspace: 'w1' });
  try {
    const council = new AgentCouncil(a);
    const researcher = new ResearcherAgent();
    const r = await researcher.propose(council, { subject: 'plan:renew-passport', action: 'research-first' });
    assert.equal(r.won, true);
    // One world: the committed id IS the address of the canonical agent-tagged body.
    assert.equal(r.id, arves.address(canonicalAgentProposal(
      'researcher', '1.0.0', 'plan:renew-passport', 'research-first',
      "researcher rule: subject 'plan:renew-passport' needs research before action",
    ), 'trace'));
    const w = council.winner('plan:renew-passport');
    assert.equal(w.agent, 'researcher');
    assert.equal(w.id, r.id);
    // The attribution tag is MANDATORY — an untagged actor cannot propose.
    await assert.rejects(() => council.propose({}, { subject: 's', action: 'a', because: 'b' }), /attribution tag/);
    await assert.rejects(() => council.propose(researcher, { subject: '', action: 'a', because: 'b' }), /subject/);
  } finally { a.close(); }
});

test('A5: conflicting proposals on ONE subject resolve FIRST-COMMITTED-WINS; the loser records a committed reference to the winner', async () => {
  const a = new Assistant({ tenant: 'agents', workspace: 'w2' });
  try {
    const council = new AgentCouncil(a);
    const researcher = new ResearcherAgent();
    const scheduler = new SchedulerAgent();
    await a.observe('tasks-file', { entity: 'urn:you', event: 'renew-passport', at: 1_751_817_600_000n });

    const first = await researcher.propose(council, { subject: 'plan:renew-passport', action: 'research-first' });
    const [second] = await scheduler.planDay(a, council); // proposes book:renew-passport -> CONFLICT
    assert.equal(first.won, true);
    assert.equal(second.won, false, 'the later, different action LOSES deterministically');
    assert.equal(second.winnerId, first.id, 'the loser is told exactly who won');
    assert.equal(second.winnerAgent, 'researcher');
    // The losing proposal is STILL committed truth (audit), and the resolution truth
    // is the loser's content-addressed reference to the winner.
    assert.match(second.id, /^[0-9a-f]{68}$/);
    assert.equal(second.resolutionId,
      arves.address(canonicalResolution('plan:renew-passport', first.id, second.id), 'trace'));
    // The winner projection is unchanged by the losing proposal.
    assert.equal(council.winner('plan:renew-passport').id, first.id);

    // Determinism: replaying the same script re-commits the same bodies (already-committed).
    const council2 = new AgentCouncil(a);
    const first2 = await researcher.propose(council2, { subject: 'plan:renew-passport', action: 'research-first' });
    const [second2] = await scheduler.planDay(a, council2);
    assert.equal(first2.id, first.id);
    assert.equal(first2.status, 'already-committed');
    assert.equal(second2.resolutionId, second.resolutionId, 'the SAME conflict resolves to the SAME resolution truth');

    // Agreement (same action) corroborates instead of conflicting.
    const agree = await council.propose({ name: 'other-agent', version: '1.0.0' },
      { subject: 'plan:renew-passport', action: 'research-first', because: 'independent agreement' });
    assert.equal(agree.won, true);
    assert.equal(agree.corroborates, first.id);
  } finally { a.close(); }
});

test('A5: the researcher GATHERS FACTS INTO TRUTH — findings cite the supporting fact ContentIds, deterministically', async () => {
  const a = new Assistant({ tenant: 'agents', workspace: 'w3' });
  try {
    await a.observe('notes-file', { entity: 'urn:you', event: 'renew-passport', at: 1_751_817_600_000n });
    await a.observe('tasks-file', { entity: 'urn:you', event: 'passport-photo-needed', at: 1_751_821_200_000n });
    await a.observe('notes-file', { entity: 'urn:you', event: 'dentist-appointment', at: 1_751_792_400_000n });
    const researcher = new ResearcherAgent();
    const f1 = await researcher.research(a, 'passport');
    assert.equal(f1.facts.length, 2, 'the deterministic rule found both passport facts');
    assert.equal(f1.id, arves.address(canonicalFinding('researcher', '1.0.0', 'passport', f1.facts), 'trace'));
    const f2 = await researcher.research(a, 'passport');
    assert.equal(f2.id, f1.id);
    assert.equal(f2.status, 'already-committed', 'same truth base -> same finding truth');
    await assert.rejects(() => researcher.research(a, ''), /topic/);
    assert.throws(() => canonicalFinding('r', '1', 't', ['not-an-id']), /ContentId/);
  } finally { a.close(); }
});

test('A7: why() reconstructs the decision path — observed evidence, proposer, policy, block, approval, committed effect', async () => {
  const a = new Assistant({ tenant: 'agents', workspace: 'w4' });
  try {
    a.useReasoner(new StubReasoner());
    await registerSkill(a, orderSkill(), ORDER_INPUTS);
    await a.observe('notes-file', { entity: 'urn:you', event: 'moms-birthday', at: 1_751_878_800_000n });
    const pol = await a.guardrails.setPolicy({
      name: 'spend-needs-user-approval', appliesTo: ['spend', 'irreversible'], approverRole: 'user',
    });
    const goal = 'order flowers for mom';
    const subject = `spend:${goalSlug(goal)}`;
    const blocked = await a.think(goal);
    const ap = await a.guardrails.approve('user', subject);
    const allowed = await a.think(goal);
    assert.equal(allowed.acted, true);
    const effectId = allowed.invocation.truths[0].id;

    // why by the EFFECT truth id: the whole path is reconstructed and id-checkable.
    const trace = why(a, effectId);
    assert.equal(trace.subject, subject);
    assert.equal(trace.observed.length, 1, 'what was observed (with evidence sources)');
    assert.deepEqual(trace.observed[0].sources, ['notes-file']);
    assert.equal(trace.proposals.length, 1, 'which reasoner proposed');
    assert.equal(trace.proposals[0].by, 'stub-reasoner');
    assert.equal(trace.proposals[0].kind, 'reasoner');
    assert.equal(trace.proposals[0].id, allowed.proposalId);
    assert.deepEqual(trace.policies.map((p) => p.id), [pol.id], 'which policy was checked');
    assert.deepEqual(trace.compliance.map((c) => c.id), [blocked.complianceId], 'what was blocked, as committed truth');
    assert.deepEqual(trace.approvals, [{ id: ap.id, role: 'user' }], 'what approval existed');
    assert.equal(trace.committed.length, 1, 'what committed');
    assert.equal(trace.committed[0].id, effectId);
    assert.equal(trace.committed[0].proposalId, allowed.proposalId, 'the effect cites its causing proposal');
    assert.equal(trace.admissions.length, 1, 'the admitted skill code is part of the answer');
    assert.match(trace.admissions[0].codeHash, /^[0-9a-f]+$/);

    // why by SUBJECT gives the identical trace (minus the asked-via target).
    const bySubject = why(a, subject);
    assert.equal(snap({ ...trace, target: null }), snap(bySubject));

    // The rendering is printable, deterministic, and id-first.
    const text = renderWhy(trace);
    assert.ok(text.includes('WHY —') && text.includes('POLICY CHECKED') && text.includes('APPROVED')
      && text.includes('COMMITTED') && text.includes('recoverFromWal'));
    assert.equal(text, renderWhy(why(a, effectId)), 'same journal -> byte-identical rendering');

    // HONEST RESIDUAL, PROVEN BY EXECUTION (adversarial finding): the disclosed
    // imperfection of read-only reconstruction is that an invoke-EFFECT truth's causal
    // edge to its skill/proposal is PROCESS metadata, absent from the self-describing
    // committed body. Prove it on the EFFECT-bearing subject (not just prose): after a
    // pure WAL recovery, why(subject) rebuilds every self-describing station identically
    // but the COMMITTED effect→skill edge (and the admission derived from it) is GONE.
    const liveSubject = why(a, subject);
    assert.equal(liveSubject.committed.length, 1, 'LIVE: the effect→skill edge is present');
    assert.equal(liveSubject.admissions.length, 1, 'LIVE: the acting skill admission is present');
    const report = await a.recoverFromWal();
    assert.ok(report.recovered >= 1, 'the WAL scan enumerated the committed set');
    assert.ok(a.journal().every((e) => e.status === 'already-committed'),
      'recoverFromWal commits nothing — every entry is recovered truth');
    const recovered = why(a, subject);
    // The self-describing stations survive byte-identically...
    assert.deepEqual(recovered.proposals.map((p) => p.id), liveSubject.proposals.map((p) => p.id),
      'proposals reconstruct from committed truth alone');
    assert.deepEqual(recovered.policies.map((p) => p.id), liveSubject.policies.map((p) => p.id),
      'the checked policy reconstructs');
    assert.deepEqual(recovered.approvals, liveSubject.approvals, 'the approval reconstructs');
    assert.deepEqual(recovered.compliance.map((c) => c.id), liveSubject.compliance.map((c) => c.id),
      'the block reconstructs');
    // ...but the process-metadata edge does NOT — the disclosed residual, asserted:
    assert.equal(recovered.committed.length, 0,
      'RESIDUAL: the effect→skill edge is absent after read-only recovery (differs from the live trace)');
    assert.equal(recovered.admissions.length, 0,
      'RESIDUAL: the acting-skill admission derives from that edge and is likewise absent');
    assert.notEqual(snap(recovered), snap(liveSubject),
      'the effect subject is NOT byte-identical after recovery — the residual is real, not hidden');
  } finally { a.close(); }
});

test('A7 honesty: why() refuses loudly — unknown ids, observation-only ids, unmentioned subjects, no smuggled answers', async () => {
  const a = new Assistant({ tenant: 'agents', workspace: 'w5' });
  try {
    assert.deepEqual(a.journal(), [], 'a fresh process has an EMPTY journal (projection, not truth)');
    assert.throws(() => why(a, 'f'.repeat(68)), /not in this process's decision journal/);
    assert.throws(() => why(a, 'plan:never-discussed'), /nothing in the decision journal mentions/);
    assert.throws(() => why(null, 'x'), /Assistant/);
    assert.throws(() => why(a, ''), /ContentId .* or a subject/);
    // A fact is an observation, not a decision — why() says so instead of inventing a path.
    const obs = await a.observe('notes-file', { entity: 'urn:you', event: 'dentist-appointment', at: 1_751_792_400_000n });
    assert.throws(() => why(a, obs.id), /no decision subject/);
    // The journal now carries exactly the two observation truths (fact + attestation).
    const j = a.journal();
    assert.equal(j.length, 2);
    assert.deepEqual(j.map((e) => e.meta.via), ['observe', 'observe']);
    assert.equal(j[0].id, obs.id);
    // REGRESSION (adversarial finding): journal bodies are IMMUTABLE snapshots, deep-
    // frozen at commit time — a consumer mutating a returned body throws LOUDLY (strict
    // mode) instead of silently corrupting every later why() trace.
    assert.ok(Object.isFrozen(j[0].body), 'journal bodies are deep-frozen at commit time');
    assert.throws(() => { j[0].body.event = 'tampered'; }, TypeError, 'mutating a journaled body throws, never silent');
    assert.equal(a.journal()[0].body.event, 'dentist-appointment', 'the recorded trace is byte-identical after the attempt');
    // And the caller's ORIGINAL object is not the record either (deep-cloned, BigInt-safe):
    const mine = { type: 'uci.assistant.note', text: 'v1', n: 7n };
    const noted = await a.commitTruth(mine, 'trace');
    mine.text = 'mutated-after-commit';
    const rec = a.journal().find((e) => e.id === noted.contentId).body;
    assert.equal(rec.text, 'v1', 'post-commit caller mutation cannot alter the journal');
    assert.equal(rec.n, 7n, 'BigInt survives the snapshot exactly');
  } finally { a.close(); }
});

test('A5+A7: agent conflicts appear in the why() trace — proposals, first-committed-wins resolution, cited finding', async () => {
  const a = new Assistant({ tenant: 'agents', workspace: 'w6' });
  try {
    await a.observe('tasks-file', { entity: 'urn:you', event: 'renew-passport', at: 1_751_817_600_000n });
    const council = new AgentCouncil(a);
    const researcher = new ResearcherAgent();
    const scheduler = new SchedulerAgent();
    const finding = await researcher.research(a, 'passport');
    const first = await researcher.propose(council,
      { subject: 'plan:renew-passport', action: 'research-first', findingId: finding.id });
    const [lost] = await scheduler.planDay(a, council);

    const trace = why(a, 'plan:renew-passport');
    assert.equal(trace.proposals.length, 2, 'BOTH proposals are in the trace (the ledger keeps losers)');
    assert.deepEqual(trace.proposals.map((p) => p.by), ['researcher', 'scheduler']);
    assert.deepEqual(trace.proposals.map((p) => p.kind), ['agent', 'agent']);
    assert.equal(trace.resolutions.length, 1);
    assert.equal(trace.resolutions[0].rule, 'first-committed-wins');
    assert.equal(trace.resolutions[0].winner, first.id);
    assert.equal(trace.resolutions[0].loser, lost.id);
    assert.deepEqual(trace.findings.map((f) => f.id), [finding.id], 'the evidence-linked finding is cited');
    // why by the LOSING proposal id resolves to the same subject and the same story.
    assert.equal(snap(why(a, lost.id)), snap({ ...trace, target: lost.id }));
  } finally { a.close(); }
});

test('A7 (RCR-033): recoverFromWal() rebuilds the journal READ-ONLY from the WAL — why() reconstructs the SAME conflict trace with ZERO re-commits', async () => {
  const a = new Assistant({ tenant: 'agents', workspace: 'w7' });
  try {
    await a.observe('tasks-file', { entity: 'urn:you', event: 'renew-passport', at: 1_751_817_600_000n });
    const council = new AgentCouncil(a);
    const researcher = new ResearcherAgent();
    const scheduler = new SchedulerAgent();
    const finding = await researcher.research(a, 'passport');
    const first = await researcher.propose(council,
      { subject: 'plan:renew-passport', action: 'research-first', findingId: finding.id });
    const [lost] = await scheduler.planDay(a, council);

    // The live trace (journal built by the running process).
    const live = why(a, 'plan:renew-passport');

    // Now DISCARD the in-process journal and rebuild it purely from committed truth via
    // the read-only scan verb — no re-run of the day, no re-commit.
    const report = await a.recoverFromWal();
    assert.ok(report.recovered >= 5, 'the WAL scan enumerated the committed set');
    // Every rebuilt journal entry is already-committed truth (a pure read, nothing fresh).
    assert.ok(a.journal().every((e) => e.status === 'already-committed'),
      'recoverFromWal commits nothing — every entry is recovered truth');

    // why() over the scan-rebuilt journal reconstructs the conflict identically.
    const recovered = why(a, 'plan:renew-passport');
    assert.equal(snap(recovered), snap(live), 'scan-rebuilt trace is byte-identical to the live trace');
    assert.deepEqual(recovered.proposals.map((p) => p.by), ['researcher', 'scheduler']);
    assert.equal(recovered.resolutions[0].winner, first.id);
    assert.equal(recovered.resolutions[0].loser, lost.id);
    assert.deepEqual(recovered.findings.map((f) => f.id), [finding.id]);
    // The observation layer recovered read-only too (fact + its attesting source).
    assert.equal(recovered.observed.length, 1);
    assert.deepEqual(recovered.observed[0].sources, ['tasks-file']);
  } finally { a.close(); }
});

// ---- runner ---------------------------------------------------------------------------
let failed = 0;
for (const { name, fn } of tests) {
  try {
    await fn();
    console.log(`  PASS  ${name}`);
  } catch (e) {
    failed++;
    console.log(`  FAIL  ${name}\n        ${e.message}`);
  }
}
console.log(`\narves-assistant (stage 3 — agents/why): ${tests.length - failed}/${tests.length} tests pass`);
process.exit(failed === 0 ? 0 : 1);
