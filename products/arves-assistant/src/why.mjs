// ARVES Assistant — EXPLAIN YOURSELF (A7): why(assistant, truthIdOrSubject).
//
// Reconstructs the DECISION PATH for a subject (or for the truth id of a proposal /
// effect / approval / decision / resolution) entirely from COMMITTED TRUTHS:
//   what was OBSERVED (facts + their attesting evidence sources)
//   what agents RESEARCHED (findings citing fact ContentIds)
//   who PROPOSED what (reasoner- or agent-attributed proposal truths)
//   how conflicts RESOLVED (first-committed-wins resolution truths)
//   which POLICY was checked (policy truths whose action classes gate the proposals)
//   what was BLOCKED (compliance truths) and what APPROVAL existed (approval truths)
//   what COMMITTED (effect truths through the bound skill, with the admitted codeHash)
//   what DECISIONS stand (decision truths).
// Every station in the trace is a ContentId + its canonical committed body — the trace
// is checkable against the ledger, not a narrative.
//
// HONEST MECHANISM, stated loudly: the input is the assistant's DECISION JOURNAL — a
// read projection of committed truth. Since RCR-033 the bridge exposes a read-only `scan`
// verb (the Kernel replays its WAL through the Query layer), so the journal can be rebuilt
// after a restart WITHOUT re-running the day: assistant.recoverFromWal() enumerates the
// shard's committed truth and rebuilds the journal with ZERO re-commits — TOTAL WAL-backed
// reconstruction, real. why() then explains a fresh process's decision path straight from
// committed truth. Residual (stated, not hidden): an invoke-EFFECT truth's causal edge to
// its skill/proposal is process metadata, absent from the self-describing body, so a
// scan-rebuilt journal reconstructs every station EXCEPT that effect→skill link (the
// COMMITTED station); the deterministic re-run path still supplies it, and a native
// attributed-INVOKE verb is the recorded next-RCR candidate. Determinism: the trace is a
// pure function of the journal — bodies and ContentIds only, no clocks, no process-local
// sequence numbers in the output — so the SAME day replayed after a restart yields a
// byte-identical trace.

const ID_RE = /^[0-9a-f]{68}$/;

/** Journal entries deduplicated by ContentId (first occurrence wins — replays of the
 *  same body add nothing), preserving commit order. */
function dedupe(journal) {
  const seen = new Set();
  const out = [];
  for (const e of journal) {
    if (!seen.has(e.id)) { seen.add(e.id); out.push(e); }
  }
  return out;
}

/** The proposal that caused an invoked effect: the LATEST proposal truth committed
 *  BEFORE the effect whose `skill` names the invoked capability. Product-side causal
 *  reconstruction (the honest form until a runtime attribution verb exists). */
function proposalFor(entries, invokeEntry) {
  let best = null;
  for (const e of entries) {
    if (e.seq >= invokeEntry.seq) break;
    const t = e.body === null || typeof e.body !== 'object' ? undefined : e.body.type;
    if ((t === 'uci.assistant.proposal' || t === 'uci.assistant.agent-proposal')
        && e.body.skill === invokeEntry.meta.capability) best = e;
  }
  return best;
}

/** Resolve a truth id to the decision SUBJECT it belongs to. Throws loudly for ids the
 *  journal has not (re-)proved, and for observation-only truths that carry no subject. */
function subjectOfId(entries, id) {
  const e = entries.find((x) => x.id === id);
  if (e === undefined) {
    throw new Error(`why: truth ${id.slice(0, 16)}… is not in this process's decision journal — `
      + 'the journal is a product-side projection: after a restart, re-run the deterministic day '
      + '(rebuild + re-register + re-think) so every body re-proves as already-committed. '
      + 'A native WAL-scan verb over the bridge is the recorded RCR candidate.');
  }
  const body = e.body;
  if (body !== null && typeof body === 'object' && typeof body.subject === 'string' && body.subject.length > 0) {
    return body.subject;
  }
  if (e.meta.via === 'invoke') {
    const prop = proposalFor(entries, e);
    if (prop !== null && typeof prop.body.subject === 'string') return prop.body.subject;
  }
  const t = body !== null && typeof body === 'object' ? body.type : typeof body;
  throw new Error(`why: truth ${id.slice(0, 16)}… is a '${t}' with no decision subject `
    + '(an observation, not a decision) — pass a subject string or a proposal/effect/approval/decision id');
}

/** why(assistant, truthIdOrSubject) -> the structured decision trace for one subject. */
export function why(assistant, truthIdOrSubject) {
  if (assistant === null || typeof assistant !== 'object' || typeof assistant.journal !== 'function') {
    throw new Error('why: an Assistant (with a decision journal) is required');
  }
  if (typeof truthIdOrSubject !== 'string' || truthIdOrSubject.length === 0) {
    throw new Error('why: pass a truth ContentId (68-hex) or a subject string');
  }
  const entries = dedupe(assistant.journal());
  const isId = ID_RE.test(truthIdOrSubject);
  const subject = isId ? subjectOfId(entries, truthIdOrSubject) : truthIdOrSubject;

  const trace = {
    subject,
    target: isId ? truthIdOrSubject : null,
    observed: [],    // [{ id, entity, event, sources }] — the truth base evidence (reasoner context = the FULL base)
    findings: [],    // [{ id, agent, topic, facts }] — agent research cited by this subject's proposals
    proposals: [],   // [{ id, kind, by, version, skill, action, actionClass, goal, because }]
    resolutions: [], // [{ id, rule, winner, loser }]
    policies: [],    // [{ id, name, appliesTo, approverRole }] — policies gating this subject's action classes
    compliance: [],  // [{ id, outcome, policy, skill }] — what was BLOCKED/refused, as committed truth
    approvals: [],   // [{ id, role }] — the separate approval truths for this subject
    committed: [],   // [{ id, skill, target, proposalId, registrationId }] — the effect truths that landed
    admissions: [],  // [{ id, skill, version, codeHash }] — which code was admitted under the acting skills
    decisions: [],   // [{ id, action, because }]
  };

  // Pass 1: the observation layer (facts + attesting sources) and the finding pool.
  const facts = new Map();      // fact id -> { entity, event }
  const sources = new Map();    // fact id -> sorted source names
  const findingPool = new Map(); // finding id -> body
  for (const e of entries) {
    const b = e.body;
    if (b === null || typeof b !== 'object') continue;
    if (b.type === 'uci.assistant.fact') facts.set(e.id, { entity: b.entity, event: b.event });
    else if (b.type === 'uci.assistant.attestation') {
      if (!sources.has(b.of)) sources.set(b.of, []);
      sources.get(b.of).push(b.source);
    } else if (b.type === 'uci.assistant.finding') findingPool.set(e.id, b);
  }

  // Pass 2: the decision layer, filtered to this subject, in commit order.
  const actionClasses = new Set();
  const actingSkills = new Set();
  for (const e of entries) {
    const b = e.body;
    if (e.meta.via === 'invoke') {
      const prop = proposalFor(entries, e);
      if (prop !== null && prop.body.subject === subject) {
        trace.committed.push({
          id: e.id, skill: e.meta.capability, target: e.meta.target,
          proposalId: prop.id, registrationId: e.meta.registrationId,
        });
        actingSkills.add(e.meta.capability);
      }
      continue;
    }
    if (b === null || typeof b !== 'object') continue;
    switch (b.type) {
      case 'uci.assistant.proposal':
      case 'uci.assistant.agent-proposal':
        if (b.subject === subject) {
          const isReasoner = b.type === 'uci.assistant.proposal';
          trace.proposals.push({
            id: e.id,
            kind: isReasoner ? 'reasoner' : 'agent',
            by: isReasoner ? b.reasoner : b.agent,
            version: isReasoner ? b.reasonerVersion : b.agentVersion,
            skill: b.skill, action: b.action, actionClass: b.actionClass,
            goal: b.goal, because: b.because,
          });
          if (typeof b.actionClass === 'string') actionClasses.add(b.actionClass);
          // Findings cited by content address in the because are part of the evidence.
          for (const m of String(b.because ?? '').matchAll(/[0-9a-f]{68}/g)) {
            const f = findingPool.get(m[0]);
            if (f !== undefined && !trace.findings.some((x) => x.id === m[0])) {
              trace.findings.push({ id: m[0], agent: `${f.agent}@${f.agentVersion}`, topic: f.topic, facts: [...f.facts] });
            }
          }
        }
        break;
      case 'uci.assistant.resolution':
        if (b.subject === subject) trace.resolutions.push({ id: e.id, rule: b.rule, winner: b.winner, loser: b.loser });
        break;
      case 'uci.assistant.compliance':
        if (b.subject === subject) trace.compliance.push({ id: e.id, outcome: b.outcome, policy: b.policy, skill: b.skill });
        break;
      case 'uci.assistant.approval':
        if (b.subject === subject) trace.approvals.push({ id: e.id, role: b.role });
        break;
      case 'uci.assistant.decision':
        if (b.subject === subject) trace.decisions.push({ id: e.id, action: b.action, because: b.because });
        break;
      default: break;
    }
  }

  // Pass 3: policies that gate this subject's action classes; admissions of acting skills.
  for (const e of entries) {
    const b = e.body;
    if (b === null || typeof b !== 'object') continue;
    if (b.type === 'uci.assistant.policy' && b.appliesTo.some((c) => actionClasses.has(c))) {
      trace.policies.push({ id: e.id, name: b.name, appliesTo: [...b.appliesTo], approverRole: b.approverRole });
    } else if (b.type === 'uci.assistant.skill' && actingSkills.has(b.name)) {
      trace.admissions.push({ id: e.id, skill: b.name, version: b.version, codeHash: b.codeHash });
    }
  }

  // The observation layer: every fact the assistant knows, with its evidence sources.
  // (Honest breadth: a reasoner proposal's context is the FULL truth base, so the whole
  // base — each fact with its attesting sources — is the observed evidence.)
  trace.observed = [...facts.entries()]
    .map(([id, f]) => ({ id, entity: f.entity, event: f.event, sources: (sources.get(id) ?? []).slice().sort() }))
    .sort((a, b) => (a.id < b.id ? -1 : 1));

  const decided = trace.proposals.length + trace.resolutions.length + trace.compliance.length
    + trace.approvals.length + trace.committed.length + trace.decisions.length;
  if (decided === 0) {
    throw new Error(`why: nothing in the decision journal mentions subject '${subject}' — `
      + 'either it never happened, or this fresh process has not re-proved its journal yet '
      + '(re-run the deterministic day; every body answers already-committed).');
  }
  return trace;
}

/** Printable rendering of a why() trace — one line per committed truth, id-first so a
 *  human (or a checker) can confront every claim with the ledger. Deterministic. */
export function renderWhy(trace) {
  const s = (id) => `${id.slice(0, 16)}…`;
  const lines = [];
  lines.push(`WHY — subject '${trace.subject}'${trace.target ? ` (asked via truth ${s(trace.target)})` : ''}`);
  lines.push('  OBSERVED (facts + evidence sources; the reasoner context is the full truth base):');
  for (const o of trace.observed) lines.push(`    ${s(o.id)} ${o.entity} :: ${o.event}  [${o.sources.join(', ')}]`);
  if (trace.findings.length > 0) {
    lines.push('  RESEARCHED (agent findings cited by the proposals):');
    for (const f of trace.findings) lines.push(`    ${s(f.id)} ${f.agent} topic '${f.topic}' citing ${f.facts.length} fact(s)`);
  }
  lines.push('  PROPOSED:');
  for (const p of trace.proposals) {
    const what = p.skill !== undefined ? `skill '${p.skill}' (class '${p.actionClass}')` : `action '${p.action}'`;
    lines.push(`    ${s(p.id)} by ${p.by}@${p.version} (${p.kind}) -> ${what}${p.goal ? ` for goal '${p.goal}'` : ''}`);
  }
  for (const r of trace.resolutions) {
    lines.push(`  RESOLVED: ${s(r.id)} rule '${r.rule}' — winner ${s(r.winner)}, loser ${s(r.loser)} (loser's committed reference to the winner)`);
  }
  if (trace.policies.length > 0) {
    lines.push('  POLICY CHECKED:');
    for (const p of trace.policies) lines.push(`    ${s(p.id)} '${p.name}' gates [${p.appliesTo.join(', ')}] (approver role '${p.approverRole}')`);
  }
  for (const c of trace.compliance) {
    lines.push(`  BLOCKED: ${s(c.id)} outcome '${c.outcome}'${c.policy ? ` citing policy ${s(c.policy)}` : ''}`);
  }
  for (const a of trace.approvals) lines.push(`  APPROVED: ${s(a.id)} by role '${a.role}' (a SEPARATE committed approval truth)`);
  if (trace.committed.length > 0) {
    lines.push('  COMMITTED:');
    for (const c of trace.committed) {
      const adm = trace.admissions.find((x) => x.skill === c.skill);
      lines.push(`    ${s(c.id)} effect '${c.target}' via skill '${c.skill}' (proposal ${s(c.proposalId)}${adm ? `, admitted code ${adm.codeHash.slice(0, 16)}…` : ''})`);
    }
  }
  for (const d of trace.decisions) lines.push(`  DECIDED: ${s(d.id)} action '${d.action}' — ${d.because}`);
  lines.push('  (mechanism: read projection of committed truths — rebuildable read-only from the WAL via');
  lines.push('   the RCR-033 bridge scan verb, recoverFromWal(); no re-commit needed for total reconstruction)');
  return lines.join('\n');
}
