// ARVES Agent Runtime (P3) — a cognitive agent whose ENTIRE decision trace is
// committed as content-addressed truth in the REAL reference Kernel (via the P2 bridge).
//
// The loop the platform prescribes:
//   Memory → Reasoning → Planning → Capability Selection → Execution → Truth Update.
// Every step is committed through the Kernel bridge, so the agent's reasoning is
// deterministic, auditable, and replayable at the truth layer — not just in app memory.
// This product needs ARVES: without content-addressed truth + a real Kernel, you cannot
// get idempotent, byte-reproducible, audited agent reasoning for free.
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

    // 1. Memory + 2. Reasoning.
    const relevant = relevantTo(goal, truths);
    const reasoning = await commit('reasoning', {
      goal,
      from: relevant.map((t) => t.id).sort(),
      conclusion: `${relevant.length} relevant truth(s) for goal`,
    });

    // 3. Planning.
    const plan = planFor(relevant);
    const planning = await commit('plan', { goal, steps: plan.map((s) => s.intent) });

    // 4. Capability Selection + 5. Execution.
    const actions = [];
    for (const step of plan) {
      const cap = this.#caps.select(step.intent);               // Capability Selection
      const outcome = cap.execute(step.args, relevant.map((t) => t.fact)); // Execution (deterministic)
      const action = await commit('action', { intent: step.intent, capability: cap.name, outcome });
      actions.push(action);
    }

    // 6. Truth Update — commit the decision-trace root over all prior steps.
    const priorIds = [reasoning.id, planning.id, ...actions.map((a) => a.id)];
    const root = await commit('decision-trace', { goal, steps: priorIds });

    return { goal, reasoning, planning, actions, root, trace };
  }
}
