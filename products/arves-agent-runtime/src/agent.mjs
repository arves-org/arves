// ARVES Agent Runtime (P3) — a cognitive agent whose ENTIRE decision trace is
// committed as content-addressed truth through the P2 bridge (`bridge.mjs` → the
// `arves-bridge` server → the WAL-backed Rust reference Kernel).
//
// The loop the platform prescribes:
//   Memory → Reasoning → Planning → Capability Selection → Execution → Truth Update.
// Every step is committed through the Kernel bridge, so the agent's reasoning is
// deterministic, auditable, and replayable at the truth layer — not just in app memory.
//
// SCOPE CAVEAT (honest): durable persistence/replay/recovery hold ONLY along the bridge
// path, and ONLY when the `arves-bridge` binary is built and running (QUICKSTART step 1) —
// if it is not, `bridge.commit(...)` rejects and nothing is committed. This module's own
// addressing helper is `class Arves` from the SDK, which is an IN-MEMORY reference substrate
// (a JS `Map`, no WAL/recovery); it is used here only to compute the local ContentId, not to
// store truth. So "audited/replayable in the real Kernel" is a claim about the bridge path,
// not about `class Arves`, which persists nothing on its own.
//
// IDR-006: this consumes the SDK (P0), Cognitive Memory (P1), and the bridge (P2). It
// modifies no platform file.

import { Arves } from '../../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();

// --- deterministic reasoning + planning over truths (product-layer agent logic) ---

function relevantTo(goal, truths) {
  return truths.filter((t) => goal.includes(t.fact.event)); // simple, deterministic match
}

function planFor(relevant) {
  // A fixed, deterministic plan template for a "prepare for <event>" goal.
  const t = relevant[0];
  const who = t ? t.fact.entity : 'unknown';
  const about = t ? t.fact.event : 'unknown';
  return [
    { intent: 'brief', args: { who, about } },
    { intent: 'schedule', args: { who, block: 'prep-block', before: about } },
    { intent: 'notify', args: { who, about } },
  ];
}

// Product-boundary guard: a capability outcome must be ACS-encodable. A bare JS number
// is ambiguous (int vs float) and lossy beyond 2^53 — reject it here so a buggy capability
// produces a recorded refusal, not a codec crash mid-trace. (Integers must be BigInt,
// floats arves.float(x).)
function normalizeOutcome(v) {
  if (typeof v === 'number') throw new Error('capability returned a bare number; use BigInt or a float wrapper');
  if (Array.isArray(v)) { v.forEach(normalizeOutcome); return v; }
  // Recurse only into PLAIN objects; class instances (Flt float wrapper, Uint8Array) are
  // opaque ACS values and pass through untouched.
  if (v && typeof v === 'object' && v.constructor === Object) {
    for (const x of Object.values(v)) normalizeOutcome(x);
  }
  return v;
}

export class Agent {
  #caps;
  constructor(capabilities) { this.#caps = capabilities; }

  /** Run the full agent loop for `goal` over `truths`, committing every step as truth in
   *  the real Kernel via `bridge`. Returns the decision trace (each step content-addressed). */
  async run(goal, truths, bridge) {
    const trace = [];
    // Truth Update: commit one decision step as truth in the REAL Kernel; verify the
    // Kernel's assigned id equals the id the agent computes locally (one world).
    const commit = async (kind, payload) => {
      const step = { type: `uci.${kind}`, ...payload };
      const localId = arves.address(step, 'trace');
      const res = await bridge.commit(step, 'trace');
      const entry = { kind, step, id: res.contentId, status: res.status, oneWorld: localId === res.contentId };
      trace.push(entry);
      return entry;
    };

    // 1. Memory + 2. Reasoning. Order-INDEPENDENCE: sort relevant truths by their content
    // address so the same knowledge in any incidental order yields the identical trace
    // (the replay/one-world claim would be false otherwise).
    const relevant = relevantTo(goal, truths).slice().sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));

    // Honest refusal: if nothing in memory is relevant, do NOT fabricate an authoritative
    // plan — record a refusal as truth and stop (misleading truth is worse than none).
    if (relevant.length === 0) {
      const reasoning = await commit('reasoning', { goal, from: [], conclusion: 'no relevant truth; refusing to plan' });
      const root = await commit('decision-trace', { goal, steps: [reasoning.id], outcome: 'refused' });
      return { goal, reasoning, planning: null, actions: [], root, trace, refused: true };
    }

    const reasoning = await commit('reasoning', {
      goal,
      from: relevant.map((t) => t.id).sort(),
      conclusion: `${relevant.length} relevant truth(s) for goal`,
    });

    // 3. Planning.
    const plan = planFor(relevant);
    const planning = await commit('plan', { goal, steps: plan.map((s) => s.intent) });

    // 4. Capability Selection + 5. Execution. Each step is guarded: a missing capability
    // or a bad outcome becomes a recorded REFUSAL step (truth), never a mid-loop crash
    // that leaves a half-committed trace.
    const actions = [];
    for (const step of plan) {
      let action;
      try {
        const cap = this.#caps.select(step.intent);               // Capability Selection
        const outcome = normalizeOutcome(cap.execute(step.args, relevant.map((t) => t.fact))); // Execution
        action = await commit('action', { intent: step.intent, capability: cap.name, outcome });
      } catch (err) {
        action = await commit('action', { intent: step.intent, capability: 'none', outcome: { refused: String(err.message).slice(0, 80) } });
      }
      actions.push(action);
    }

    // 6. Truth Update — commit the decision-trace root over all prior steps.
    const priorIds = [reasoning.id, planning.id, ...actions.map((a) => a.id)];
    const root = await commit('decision-trace', { goal, steps: priorIds });

    return { goal, reasoning, planning, actions, root, trace };
  }
}
