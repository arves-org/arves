// ARVES AI Capability SDK — the AI-Operating-System layer of the Ecosystem Kit.
//
// THE THESIS: an LLM is not the system. It is a swappable Capability/Provider. The ARVES
// runtime never changes when you switch models; the model's output becomes RECORDED,
// content-addressed, REPLAYABLE truth. This is the difference between an "AI operating
// system" and a chatbot wrapper.
//
// A reasoning capability is an ordinary ARVES capability (same { manifest, execute } shape
// as defineCapability() in ./kit.mjs) whose effects happen to be produced by a reasoning
// provider. Because it is an ordinary capability, it flows through the UNCHANGED trust
// boundary: certify → package → install → invoke → commit. The runtime commits the bytes;
// it never had to know a language model produced them.
//
// ─────────────────────────────────────────────────────────────────────────────
// THE MOAT — RECORDED TRUTH, NOT RE-CALLING (ORCH-003 · ACS-005 GL-012)
// ─────────────────────────────────────────────────────────────────────────────
// A provider's output is committed EXACTLY ONCE as content-addressed truth. From that
// moment the output IS an ACS ContentId in the ledger. REPLAY reads the recorded trace by
// its ContentId; it NEVER re-invokes the provider. A wrapper re-calls the model on every
// run and hopes the answer is stable — it is not, so a wrapper can neither replay nor audit.
// ARVES turns a possibly non-deterministic LLM into a deterministic, auditable fact by
// recording it once: identity is the content address, and the address never moves.
//
//   author time  : provider.reason(input)  ->  value  ->  commit  ->  ContentId  (ONCE)
//   replay time  : ContentId               ->  recorded value                    (NEVER re-calls)
//
// The deterministic providers below (reference, local) let a reasoning capability CERTIFY
// and REPLAY fully OFFLINE — no network, no keys — which is exactly the property the
// certifier (certifyCapability) enforces: same input → same effect address. The remote
// providers (claude, gpt, gemini) are documented integration STUBS: they name the single
// place an operator supplies an API adapter + key. They throw until integrated, on purpose:
// nothing in this repo makes a network call or needs a secret.

import { defineCapability, float } from './kit.mjs';
import { sha256, hex } from '../../arves-sdk-ts/src/codec.mjs';

export { float };

// ---- Providers -------------------------------------------------------------
//
// A provider is { name, reason(input) }. reason(input) returns an ARVES value (or an array
// of effects — see defineReasoningCapability). reference/local are PURE functions of input,
// so any capability built on them is deterministic and certifies green. claude/gpt/gemini
// are adapter stubs marking the documented integration point.

/** Throw the canonical integration error for a not-yet-wired remote provider. Kept in one
 *  place so the message is identical across claude/gpt/gemini and easy to assert against. */
function requireIntegration(name) {
  throw new Error(`provider "${name}" requires integration: supply an API adapter + key`);
}

/** A stable, deterministic token signature of an input — a pure fold over the ARVES value.
 *  Used by the offline providers so their output is a reproducible function of the input
 *  (never a clock, RNG, or ambient state). Not a hash; just a stable, human-legible digest. */
function digest(input) {
  const norm = (v) => {
    if (v === null || v === undefined) return '';
    if (typeof v === 'bigint') return String(v);
    if (typeof v === 'string') return v;
    if (typeof v === 'number') return String(v);
    if (typeof v === 'boolean') return v ? '1' : '0';
    if (Array.isArray(v)) return v.map(norm).join(' ');
    if (typeof v === 'object') return Object.keys(v).sort().map((k) => `${k}:${norm(v[k])}`).join(' ');
    return String(v);
  };
  return norm(input).toLowerCase();
}

/** reference — the DETERMINISTIC reference reasoner. A pure, rule-based classifier that
 *  scores an input's text against a small fixed lexicon and returns a structured verdict.
 *  It stands in for "a model" while being byte-stable: the SAME input always yields the SAME
 *  ARVES value, so a capability built on it certifies and replays with no network or key.
 *  This is the reasoner the conformance/certification path uses. */
const referenceProvider = {
  name: 'reference',
  reason(input) {
    const text = digest(input);
    // Fixed lexicon → sentiment score. Deterministic, order-independent word counting.
    const POS = ['great', 'good', 'love', 'excellent', 'happy', 'wonderful', 'best', 'thanks', 'awesome', 'perfect'];
    const NEG = ['bad', 'terrible', 'hate', 'awful', 'angry', 'broken', 'worst', 'slow', 'crash', 'refund'];
    const words = text.split(/[^a-z0-9]+/).filter(Boolean);
    let pos = 0n;
    let neg = 0n;
    for (const w of words) {
      if (POS.includes(w)) pos += 1n;
      if (NEG.includes(w)) neg += 1n;
    }
    let label = 'neutral';
    if (pos > neg) label = 'positive';
    else if (neg > pos) label = 'negative';
    return {
      type: 'uci.reasoning.verdict',
      provider: 'reference',
      label,
      positive: pos,
      negative: neg,
    };
  },
};

/** local — a second DETERMINISTIC provider, standing in for an on-device / self-hosted
 *  model. Distinct name (so a capability can pin "which model produced this truth") but the
 *  same offline determinism guarantee: pure function of input, certifiable, replayable. */
const localProvider = {
  name: 'local',
  reason(input) {
    const v = referenceProvider.reason(input);
    return { ...v, provider: 'local' };
  },
};

/** claude / gpt / gemini — remote-model adapter STUBS. Each is the DOCUMENTED integration
 *  point: an operator wires an API adapter + key here, out-of-repo. They throw until then —
 *  by design nothing in this repo performs a network call or holds a secret. When wired, the
 *  provider's output is still committed ONCE as truth and replayed from the record (below);
 *  the model being non-deterministic does not make the ARVES fact non-deterministic. */
const claudeProvider = { name: 'claude', reason: () => requireIntegration('claude') };
const gptProvider = { name: 'gpt', reason: () => requireIntegration('gpt') };
const geminiProvider = { name: 'gemini', reason: () => requireIntegration('gemini') };

export const Providers = {
  reference: referenceProvider,
  local: localProvider,
  claude: claudeProvider,
  gpt: gptProvider,
  gemini: geminiProvider,
};

// ---- defineReasoningCapability ---------------------------------------------

/** Author a REASONING capability. It has the SAME shape as defineCapability()'s result
 *  ({ manifest, execute }) and flows through the identical certify/package/install/invoke
 *  path — a reasoning capability is just a capability whose effects come from a provider.
 *
 *  @param name      capability name
 *  @param version   semver
 *  @param produces  effect targets (defaults to ['uci.reasoning.verdict'])
 *  @param provider  a Provider ({ name, reason }); its .reason is used unless `reason` given
 *  @param reason    optional explicit reason(input) that overrides provider.reason
 *
 *  execute(input) calls (reason || provider.reason)(input) and NORMALIZES the result into an
 *  array of effects [{ target, value }]. The reasoner may return either:
 *    - a bare ARVES value                → wrapped as one effect on produces[0]
 *    - an array of ARVES values          → one effect each on produces[0]
 *    - an effect { target, value }       → passed through
 *    - an array of effects               → passed through
 *
 *  DOCTRINE: whatever value a provider returns is committed ONCE as content-addressed truth
 *  (ORCH-003 · ACS-005 GL-012). Replay reads the recorded ContentId; it never re-calls the
 *  provider. With a deterministic provider (reference/local) the capability CERTIFIES via the
 *  existing certifyCapability() and replays fully offline. */
export function defineReasoningCapability({ name, version, produces, provider, reason }) {
  const reasoner = reason ?? (provider && provider.reason);
  if (typeof reasoner !== 'function') {
    throw new Error('reasoning capability: supply `reason(input)` or a `provider` with a reason() function');
  }
  const targets = Array.isArray(produces) && produces.length ? produces : ['uci.reasoning.verdict'];
  const primary = targets[0];

  const isEffect = (x) => x && typeof x === 'object' && !Array.isArray(x)
    && typeof x.target === 'string' && 'value' in x;

  const toEffects = (out) => {
    if (Array.isArray(out)) {
      // Array of effects, or array of bare values.
      if (out.every(isEffect)) return out;
      return out.map((value) => ({ target: primary, value }));
    }
    if (isEffect(out)) return [out];
    return [{ target: primary, value: out }];
  };

  // Reuse defineCapability so the manifest, validation, and shape are IDENTICAL to any other
  // capability — a reasoning capability is not a special citizen at the trust boundary.
  const cap = defineCapability({
    name,
    version,
    produces: targets,
    determinism: 'deterministic',
    execute: (input) => toEffects(reasoner(input)),
  });
  // SECURITY (closure-audit finding): every reasoning capability's `execute` stringifies to the
  // SAME wrapper `(input) => toEffects(reasoner(input))`, so codeHash (which hashes execute source)
  // cannot bind the actual logic — the real logic is the closed-over `reasoner`. Bind it explicitly:
  // hash the reasoner source into the manifest (so packageCapability signs it into the artifact id)
  // AND expose the source so a host/registry can re-verify on install. Tampering the reasoner then
  // either mismatches the signed reasonerHash or breaks the artifact signature.
  const reasonerSource = reasoner.toString();
  cap.manifest.reasonerHash = hex(sha256(new TextEncoder().encode(reasonerSource)));
  cap.reasonerSource = reasonerSource;
  return cap;
}
