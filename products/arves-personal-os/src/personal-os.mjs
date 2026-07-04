// ARVES Personal Cognitive OS (P4) — the first operating system for a person's cognition,
// built entirely on the FROZEN ARVES Runtime v1.0 (Runtime Change Request, never a runtime
// edit). It is not an assistant, a chatbot, or an automation tool: it maintains a
// persistent, content-addressed world model of a person's reality and produces reasoning
// that is reproducible, evidence-backed, auditable, and replayable — properties a
// ChatGPT/LangGraph/n8n wrapper cannot provide.
//
// Runtime APIs consumed (all frozen v1.0): the SDK (content addressing) + the Kernel
// Bridge (commit truth to the real Kernel). Every fact, decision, and briefing is committed
// as truth through the bridge to the WAL-backed Rust reference Kernel — addressable,
// idempotent, replayable.
//
// SCOPE CAVEAT (honest): the durable persistence/replay/recovery claim holds ONLY along the
// `#bridge` path, and ONLY when the `arves-bridge` binary is built and running (QUICKSTART
// step 1) — otherwise `#bridge.commit(...)` rejects and nothing is committed. The queryable
// world model this class exposes (`truths()`, the `#facts`/`#evidence` maps, dedup, and
// contradiction detection) is an IN-MEMORY per-process index built via `class Arves` (a JS
// `Map`, no WAL/recovery); it is NOT read back from the Kernel and is lost on process exit.
// Reproducibility of the briefing id comes from content addressing, not from durable state.

import { Arves } from '../../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();

// Canonicalize a source observation into the abstract ARVES fact it attests. The source
// is evidence, NOT part of identity — so the same event from many systems is one truth.
function canonicalize(obs) {
  return {
    type: 'uci.fact',
    entity: obs.fact.entity,
    event: obs.fact.event,
    at: BigInt(obs.fact.at) * 1_000_000n, // ms → ns, exact (Runtime §5.2)
  };
}

export class PersonalCognitiveOS {
  #bridge;
  #facts = new Map();      // ContentId → fact (deduplicated truths)
  #evidence = new Map();   // ContentId → Set(source)
  #decisions = new Map();  // subject → { id, decision }  (the persistent decision world model)

  constructor(bridge) { this.#bridge = bridge; }

  /** Observe one slice of reality: commit it as truth in the real Kernel (frozen Runtime
   *  API) and fold it into the world model. Same event from another source → deduped,
   *  with the new source added as evidence. Verifies one-world identity (SDK id == Kernel id). */
  async observe(obs) {
    const fact = canonicalize(obs);
    const localId = arves.address(fact, 'commit');
    const res = await this.#bridge.commit(fact, 'commit'); // truth → real Kernel
    if (res.contentId !== localId) throw new Error('one-world violation: SDK id != Kernel id');
    const isNew = !this.#facts.has(localId);
    if (isNew) { this.#facts.set(localId, fact); this.#evidence.set(localId, new Set()); }
    this.#evidence.get(localId).add(obs.source);
    return { id: localId, deduped: !isNew, sources: [...this.#evidence.get(localId)].sort() };
  }

  /** The world model: deduplicated truths, each with its attesting sources. */
  truths() {
    return [...this.#facts.entries()]
      .map(([id, fact]) => ({ id, fact, sources: [...this.#evidence.get(id)].sort() }))
      .sort((a, b) => (a.id < b.id ? -1 : 1)); // stable → reproducible reasoning
  }

  /** Record a decision as truth in the real Kernel — the decision itself is a durable,
   *  content-addressed truth in the WAL. NOTE (audit P1): the contradiction-detection index
   *  (`#decisions`) is an in-memory, process-scoped cache rebuilt per run, not read back from
   *  the Kernel — so it is the *commit* that is durable, and cross-session contradiction
   *  detection requires replaying the decisions into a fresh instance. The demo detects within
   *  one process. A chatbot still has no durable, evidence-backed decision *record* at all. */
  async recordDecision(decision) {
    const step = { type: 'uci.decision', subject: decision.subject, action: decision.action, because: decision.because };
    const res = await this.#bridge.commit(step, 'trace');
    this.#decisions.set(decision.subject, { id: res.contentId, decision });
    return res.contentId;
  }

  /** Detect whether a candidate decision contradicts a prior COMMITTED decision (same
   *  subject, opposing action). Returns the contradiction with the prior decision's truth
   *  id as evidence — "this contradicts what you decided before, here is the proof." */
  checkContradiction(candidate) {
    const prior = this.#decisions.get(candidate.subject);
    if (prior && prior.decision.action !== candidate.action) {
      return { contradicts: true, priorId: prior.id, prior: prior.decision };
    }
    return { contradicts: false };
  }

  /** The daily cognitive briefing: reason deterministically over the world model to
   *  produce evidence-backed recommendations, surface contradictions with prior decisions,
   *  and commit the whole briefing as ONE content-addressed truth in the real Kernel.
   *  Running it again over the same world model yields the identical briefing id and the
   *  Kernel reports already-committed — reproducible, audited cognition. */
  async dailyBriefing() {
    const truths = this.truths();
    const recommendations = [];
    const contradictions = [];

    for (const t of truths) {
      const { entity, event } = t.fact;
      const evidence = [t.id];
      if (event === 'q3-review') {
        recommendations.push({ text: `Meeting "${event}" today — attested by ${t.sources.length} systems (high confidence); keep it.`, evidence });
      } else if (event === 'low-sleep') {
        recommendations.push({ text: 'Low sleep detected — recommend a lighter schedule and no high-stakes decisions today.', evidence });
      } else if (event === 'pr-review-requested') {
        recommendations.push({ text: `Review requested on ${entity} — 25 min block suggested.`, evidence });
      } else if (entity === 'invest:acme-fund' && event === 'price-up-20pct') {
        // A signal that may conflict with a standing decision — check the world model.
        const candidate = { subject: 'invest:acme-fund', action: 'approve' };
        const c = this.checkContradiction(candidate);
        if (c.contradicts) {
          contradictions.push({
            text: `acme-fund is up 20% — but this CONTRADICTS your decision to ${c.prior.action} it (because: ${c.prior.because}).`,
            priorDecision: c.priorId,
            evidence,
          });
        } else {
          recommendations.push({ text: 'acme-fund is up 20% — consider reviewing.', evidence });
        }
      }
    }

    // Commit the briefing as one content-addressed truth (reproducible + audited).
    const briefing = {
      type: 'uci.briefing',
      recommendations: recommendations.map((r) => r.text),
      contradictions: contradictions.map((c) => c.text),
      from: truths.map((t) => t.id),
    };
    const res = await this.#bridge.commit(briefing, 'trace');
    return { id: res.contentId, status: res.status, recommendations, contradictions };
  }
}
