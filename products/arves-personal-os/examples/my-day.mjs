// ARVES Personal Cognitive OS — "my day" flagship demo.
//
// A person's multi-domain reality → one persistent, content-addressed world model in the
// REAL Kernel → a daily briefing whose reasoning is reproducible, evidence-backed,
// auditable, and aware of your prior decisions. Every one of these properties is
// IMPOSSIBLE for a ChatGPT/LangGraph/n8n/AutoGen wrapper — that is the point.
//
// Runs entirely on the FROZEN Runtime v1.0 API (SDK + Bridge). It edits no runtime file.
// Run: node examples/my-day.mjs   (requires: cargo build -p arves-bridge --bin arves-bridge)

import { KernelBridge } from '../../arves-sdk-ts/src/bridge.mjs';
import { PersonalCognitiveOS } from '../src/personal-os.mjs';
import { personalReality } from '../src/connectors.mjs';

const bridge = new KernelBridge();
const os = new PersonalCognitiveOS(bridge);

console.log('ARVES Personal Cognitive OS — your day, as reproducible cognition\n');

// A standing decision made "yesterday", committed as truth in the world model.
await os.recordDecision({ subject: 'invest:acme-fund', action: 'decline', because: 'high risk' });

// Ingest today's reality from six systems.
const obs = personalReality();
const ingested = [];
for (const o of obs) ingested.push({ src: o.source, ...(await os.observe(o)) });

const meeting = ingested.filter((r) => r.sources.includes('calendar'));
console.log('[Reality] ingested', obs.length, 'observations from', new Set(obs.map((o) => o.source)).size, 'systems');
console.log('[Truth]   deduplicated to', os.truths().length, 'truths (the q3-review meeting from calendar+email+slack collapsed to ONE, 3 attestations)');

// The daily briefing — reasoned over the world model, committed as truth.
const b1 = await os.dailyBriefing();
console.log('\nToday:');
for (const r of b1.recommendations) console.log('  •', r.text);
for (const c of b1.contradictions) console.log('  ⚠', c.text, `[prior decision ${c.priorDecision.slice(0, 14)}…]`);
console.log('\n[Briefing] committed as truth', b1.id.slice(0, 18) + '…', `(${b1.status})`);

// Reproducibility: the same world model → the identical briefing id; the Kernel reports
// already-committed. A chatbot answers differently every time and cannot prove anything.
const b2 = await os.dailyBriefing();
bridge.close();

const oneMeetingTruth = os.truths().filter((t) => t.fact.event === 'q3-review').length === 1;
const threeAttest = meeting.length === 3 && ingested.find((r) => r.deduped);
const reproducible = b1.id === b2.id && b2.status === 'already-committed';
const caughtContradiction = b1.contradictions.length === 1;
const evidenceBacked = b1.recommendations.every((r) => r.evidence.length > 0);

console.log('\nWhy this is impossible without ARVES:');
console.log('  three systems → one truth (cross-source identity)   :', oneMeetingTruth && !!threeAttest);
console.log('  briefing is byte-reproducible + idempotent in Kernel:', reproducible);
console.log('  it caught a contradiction with a PRIOR decision      :', caughtContradiction);
console.log('  every recommendation cites its evidence (defensible) :', evidenceBacked);

const ok = oneMeetingTruth && threeAttest && reproducible && caughtContradiction && evidenceBacked;
console.log(ok
  ? '\nA persistent world model with reproducible, audited, evidence-backed, decision-aware cognition.\nNot a chatbot — an operating system for cognition, on the frozen ARVES Runtime v1.0.'
  : '\nFAIL: a Personal-OS property did not hold.');
process.exit(ok ? 0 : 1);
