// ARVES Assistant — REAL OpenAI reasoner (the client the example adapter was missing).
//
// ============================================================================
//  NO API KEY LIVES IN THIS FILE. IT IS READ ONLY FROM process.env.OPENAI_API_KEY.
// ============================================================================
// This is the one committed piece that turns the deterministic StubReasoner into a REAL
// model, WITHOUT changing anything about governance. It supplies the `client.complete`
// that `LlmReasonerAdapter` (llm-reasoner.example.mjs) delegates the actual call to —
// so the governed pipeline is byte-for-byte the same: the model's proposal is committed
// ONCE as attributed truth, guardrail policies are checked BEFORE any skill runs, only
// certified+bound skills can act, refusals are committed compliance truths, and a
// hallucinated skill name is refused by `parseProposal` before it can reach the runtime.
//
// DOCTRINE (why this is safe to commit):
//   * Zero dependencies — uses the Node >=18 global `fetch`; no SDK, no framework.
//   * No secret — the key is read from the ENVIRONMENT at call time, never from a file,
//     never logged, never echoed in an error. If it is absent, the call FAILS LOUD.
//   * Offline-hermetic tests — nothing here runs in the default test gate; the live test
//     (openai-reasoner.test.mjs) SKIPS cleanly with no key, so CI stays network-free.
//   * Non-determinism is expected and bounded — temperature 0 minimizes drift, and the
//     recorded-truth doctrine means replay reads the committed proposal, never re-calls
//     the model (products/arves-ecosystem-sdk/REASONING.md).
//
// USE (nothing in the repo changes — only YOUR environment):
//   OPENAI_API_KEY=sk-...  node products/arves-assistant/ui/server.mjs --wal-dir ./wal
//   OPENAI_API_KEY=sk-...  OPENAI_MODEL=gpt-4o  node .../bin/jarvis.mjs --wal-dir ./wal ask ...
// or in code:
//   import OpenAiReasoner from './src/openai-reasoner.mjs';
//   assistant.useReasoner(new OpenAiReasoner());        // the ONLY line that changes

import { LlmReasonerAdapter } from './llm-reasoner.example.mjs';

const DEFAULT_MODEL = 'gpt-4o-mini';
const DEFAULT_BASE = 'https://api.openai.com/v1';
const DEFAULT_TIMEOUT_MS = 30_000;

/** A zero-dependency OpenAI Chat Completions client exposing `complete(prompt) -> string`.
 *  The API key is read from process.env.OPENAI_API_KEY AT CALL TIME — never captured into
 *  this object, never written anywhere, never logged. `response_format: json_object` makes
 *  the model return strict JSON, which pairs with the adapter's governed parseProposal. */
export function openAiClient({
  model = process.env.OPENAI_MODEL || DEFAULT_MODEL,
  baseUrl = process.env.OPENAI_BASE_URL || DEFAULT_BASE,
  timeoutMs = Number(process.env.OPENAI_TIMEOUT_MS) || DEFAULT_TIMEOUT_MS,
} = {}) {
  return {
    model,
    async complete(prompt) {
      const key = process.env.OPENAI_API_KEY;
      if (typeof key !== 'string' || key.length === 0) {
        throw new Error('openai-reasoner: OPENAI_API_KEY is not set in the environment — the key is read ONLY from env, never from a file or the repo');
      }
      let res;
      try {
        res = await fetch(`${baseUrl}/chat/completions`, {
          method: 'POST',
          headers: { 'content-type': 'application/json', authorization: `Bearer ${key}` },
          body: JSON.stringify({
            model,
            temperature: 0,
            max_tokens: 400,
            response_format: { type: 'json_object' },
            messages: [
              { role: 'system', content: 'You are the reasoner for a governed assistant. Reply with EXACTLY ONE JSON object and nothing else.' },
              { role: 'user', content: prompt },
            ],
          }),
          signal: AbortSignal.timeout(timeoutMs),
        });
      } catch (e) {
        // Network / timeout — surface loud (the assistant has committed NOTHING yet).
        throw new Error(`openai-reasoner: request failed (${e.name === 'TimeoutError' ? `timeout after ${timeoutMs}ms` : e.message})`);
      }
      if (!res.ok) {
        // OpenAI error bodies describe the problem WITHOUT echoing your key; cap the size.
        const body = await res.text().catch(() => '');
        throw new Error(`openai-reasoner: OpenAI API ${res.status} ${res.statusText}${body ? ` — ${body.slice(0, 240)}` : ''}`);
      }
      const data = await res.json().catch(() => null);
      const text = data && data.choices && data.choices[0] && data.choices[0].message && data.choices[0].message.content;
      if (typeof text !== 'string' || text.length === 0) {
        throw new Error('openai-reasoner: OpenAI response had no message content');
      }
      return text;
    },
  };
}

/** The drop-in real reasoner: a fully-wired LlmReasonerAdapter over the OpenAI client.
 *  `new OpenAiReasoner()` reads model + key from the environment. Every proposal it makes
 *  is attributed to `openai:<model>` in the committed truth. This is the default export so
 *  the UI server / any loader can `new (await import(...)).default()` with no arguments. */
export default class OpenAiReasoner extends LlmReasonerAdapter {
  constructor(opts = {}) {
    const client = openAiClient(opts);
    super({
      client,
      name: `openai:${client.model}`,
      version: '1.0.0',
      ...opts,
    });
  }
}
