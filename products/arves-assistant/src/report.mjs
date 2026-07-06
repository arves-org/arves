// ARVES Assistant — EXPORT / REPORT MY DAY (completeness).
//
// A structured, deterministic export of everything the assistant currently knows —
// grouped by entity, with decisions, admitted skills, and guardrail policies — built
// PURELY from the committed-truth projection (assistant.truths()/decisions()/skills()/
// guardrails). No clock, no randomness: the fact instants come from the committed `at`
// (nanoseconds), rendered back to their canonical UTC ISO string. So `report json`
// replays byte-identically across a restart, exactly like why().

/** Build the report object from an assistant's committed-truth projection. */
export function reportDay(assistant) {
  const byEntity = new Map();
  for (const t of assistant.truths()) {
    const iso = new Date(Number(t.fact.at / 1_000_000n)).toISOString(); // ns -> ms -> UTC ISO
    if (!byEntity.has(t.fact.entity)) byEntity.set(t.fact.entity, []);
    byEntity.get(t.fact.entity).push({ id: t.id, event: t.fact.event, at: iso, sources: [...t.sources] });
  }
  for (const items of byEntity.values()) {
    items.sort((a, b) => (a.at !== b.at ? (a.at < b.at ? -1 : 1) : (a.event < b.event ? -1 : a.event > b.event ? 1 : 0)));
  }
  const entities = [...byEntity.keys()].sort().map((entity) => ({ entity, items: byEntity.get(entity) }));
  const decisions = assistant.decisions().map((d) => ({ subject: d.subject, action: d.action, because: d.because }));
  const skills = assistant.skills();
  const policies = assistant.guardrails.policies().map((p) => ({ name: p.name, appliesTo: [...p.appliesTo], approverRole: p.approverRole }));
  return {
    generatedFrom: 'committed-truth',
    counts: {
      truths: entities.reduce((n, e) => n + e.items.length, 0),
      entities: entities.length,
      decisions: decisions.length,
      skills: skills.length,
      policies: policies.length,
    },
    entities, decisions, skills, policies,
  };
}

/** Printable rendering of a report — deterministic, id-first so every line is checkable. */
export function renderReport(r) {
  const s = (id) => (id ? `${id.slice(0, 16)}…` : '-');
  const lines = [];
  const c = r.counts;
  lines.push('ARVES Assistant — DAY REPORT (projection of committed truth; replayable byte-for-byte)');
  lines.push(`  ${c.truths} truth(s) across ${c.entities} entit(y/ies) · ${c.decisions} decision(s) · ${c.skills} skill(s) · ${c.policies} polic(y/ies)`);
  if (r.entities.length === 0) lines.push('  (memory empty — observe/import, or run over a --wal-dir that has prior truth)');
  for (const e of r.entities) {
    lines.push(`  ${e.entity}:`);
    for (const it of e.items) lines.push(`    ${s(it.id)} ${it.at}  ${it.event}  [${it.sources.join(', ')}]`);
  }
  if (r.decisions.length > 0) {
    lines.push('  DECISIONS:');
    for (const d of r.decisions) lines.push(`    ${d.subject} -> ${d.action}  (${d.because})`);
  }
  if (r.skills.length > 0) lines.push(`  SKILLS: ${r.skills.join(', ')}`);
  if (r.policies.length > 0) {
    lines.push('  POLICIES:');
    for (const p of r.policies) lines.push(`    ${p.name}: [${p.appliesTo.join(', ')}] require '${p.approverRole}'`);
  }
  return lines.join('\n');
}
