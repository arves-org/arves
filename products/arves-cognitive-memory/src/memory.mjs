// ARVES Cognitive Memory (P2) — the common core of every ARVES product.
//
// It consumes the P1 SDK (which consumes the platform). It modifies no platform file
// (IDR-006). It proves six ARVES capabilities on real data:
//   Identity · Deduplication · Evidence · Truth · Audit (tamper-evident) · Replay,
// plus provable reasoning. The insight throughout: identity IS the content address, so
// facts from unrelated systems that mean the same thing collapse to one truth for free.
//
// SCOPE CAVEAT (honest): this module is an IN-MEMORY reference substrate built directly on
// `class Arves` (a JS `Map`, plus a co-located hash chain `#log`/`#head`). It does NOT use
// the Kernel bridge and has NO WAL, NO durable persistence, and NO crash recovery — all
// state is lost on process exit and every claim below is per-process, not durable.
//   - "Replay" means: re-ingesting the same observations recomputes the same content
//     address. It is deterministic recomputation, not replay from a durable log.
//   - "Audit (tamper-evident)" is a hash chain whose integrity is only as strong as its HEAD.
//     `verifyChain()` detects tampering of any PAST entry ONLY relative to a head the verifier
//     already trusts; an attacker who can rewrite the whole log AND the head produces a chain
//     that verifies clean. Real tamper-evidence therefore requires an EXTERNALLY-TRUSTED head
//     (anchored outside this process — e.g. committed to the real Kernel via `bridge.mjs`, or
//     to another append-only authority). Co-located head + log is integrity, not attestation.
// For durable, cross-process, WAL-backed truth, commit through `bridge.mjs` to the real Kernel.

import { Arves } from '../../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();

// --- canonicalization: converge each source's native shape onto ONE canonical fact ---

// Identity resolution: source-specific handles -> a canonical entity URN. (In production
// this is an entity-resolution service; here it is a deterministic table.)
const IDENTITY = new Map([
  ['ada@analytical.example', 'urn:arves:person:ada'],
  ['Ada Lovelace', 'urn:arves:person:ada'],
  ['003AL', 'urn:arves:person:ada'],
]);
function resolveEntity(key) {
  const urn = IDENTITY.get(key);
  if (!urn) throw new Error(`Cognitive Memory: unresolved identity '${key}'`);
  return urn;
}

// Event resolution: different source labels for the SAME event resolve to one key via
// an explicit alias table (like entity resolution) — NOT by stripping words, which would
// falsely merge genuinely different events (e.g. "Board Meeting" vs "Board"). Unknown
// labels fall back to a lossless slug, so distinct events stay distinct.
const EVENT_ALIASES = new Map([
  ['Q3 Review', 'q3-review'],
  ['Q3 Review Meeting', 'q3-review'],
]);
function eventKey(text) {
  return EVENT_ALIASES.get(text) || text.toLowerCase().trim().replace(/\s+/g, '-');
}

/** Map any source observation to the single canonical ARVES fact it attests. Three
 *  different schemas produce a byte-identical fact — hence a single ContentId. */
export function canonicalize(obs) {
  const r = obs.raw;
  let entityKey, label, epochMs;
  switch (obs.source) {
    case 'email': entityKey = r.attendee; label = r.subject; epochMs = r.epochMs; break;
    case 'calendar': entityKey = r.attendeeEmail; label = r.title; epochMs = r.epochMs; break;
    case 'crm': entityKey = r.contactId; label = r.activity; epochMs = r.epochMs; break;
    default: throw new Error(`Cognitive Memory: unknown source '${obs.source}'`);
  }
  return {
    type: 'uci.fact',
    entity: resolveEntity(entityKey),
    event: eventKey(label),
    at: BigInt(epochMs) * 1_000_000n, // ms -> ns, held EXACTLY (BigInt) per ACS-002 §5.2
  };
}

export class CognitiveMemory {
  #facts = new Map();      // ContentId -> canonical fact  (the deduplicated truths)
  #evidence = new Map();   // ContentId -> Set(source)     (who attested each truth)
  #log = [];               // append-only audit entries (each content-addressed)
  #head = '00';            // tamper-evident audit-chain head (genesis = '00')

  /** Observe one source observation. Returns the truth's ContentId and whether it was a
   *  duplicate of an already-known truth. Records evidence and extends the audit chain. */
  ingest(obs) {
    const fact = canonicalize(obs);
    const id = arves.commit(fact);                 // identity = content address (ACS-001)
    const isNew = !this.#facts.has(id);
    if (isNew) { this.#facts.set(id, fact); this.#evidence.set(id, new Set()); }
    this.#evidence.get(id).add(obs.source);        // evidence / provenance

    // Tamper-evident audit chain: each entry commits to the previous head, so altering
    // any past entry changes every later address and the head — a Merkle-style ledger.
    const entry = {
      type: 'uci.audit',
      seq: BigInt(this.#log.length),
      op: 'observe',
      fact: id,
      source: obs.source,
      at: fact.at,
      prev: this.#head,
    };
    const entryId = arves.address(entry, 'trace');
    this.#log.push({ ...entry, id: entryId });
    this.#head = entryId;
    return { id, deduped: !isNew, source: obs.source };
  }

  /** The deduplicated truths, each with its evidence set. */
  truths() {
    return [...this.#facts.entries()].map(([id, fact]) => ({
      id, fact, sources: [...this.#evidence.get(id)].sort(),
    }));
  }

  evidenceOf(id) { return [...(this.#evidence.get(id) || [])].sort(); }
  /** The audit trail as an immutable deep copy — callers cannot mutate history. */
  auditTrail() { return this.#log.map((e) => Object.freeze({ ...e })); }
  head() { return this.#head; }

  /** Verify the audit chain is intact — this is what makes it *tamper-evident*, not just
   *  a chain that was built once. Walks from genesis, re-derives each entry's id from its
   *  own fields and the running head, and checks it matches the stored id and links to the
   *  prior head; finally checks the recomputed head equals the current head. Altering,
   *  reordering, or dropping ANY past entry makes this fail at that point.
   *  Returns `{ ok, brokenAt, reason }`. */
  verifyChain(log = this.#log, head = this.#head) {
    let running = '00';
    for (let i = 0; i < log.length; i++) {
      const e = log[i];
      if (e.prev !== running) return { ok: false, brokenAt: i, reason: 'broken link (prev != running head)' };
      const reId = arves.address(
        { type: e.type, seq: e.seq, op: e.op, fact: e.fact, source: e.source, at: e.at, prev: e.prev },
        'trace',
      );
      if (reId !== e.id) return { ok: false, brokenAt: i, reason: 'entry tampered (id mismatch)' };
      running = e.id;
    }
    if (running !== head) return { ok: false, brokenAt: log.length, reason: 'head mismatch (entry dropped/appended)' };
    return { ok: true, brokenAt: null, reason: 'intact' };
  }

  /** A deterministic content address of the whole memory STATE (order-independent):
   *  the sorted set of (truth, evidence-count). Same truths + evidence => same root. */
  root() {
    const state = [...this.#facts.keys()].sort().map((id) => ({
      f: id, n: BigInt(this.#evidence.get(id).size),
    }));
    return arves.address(state, 'trace');
  }

  /** Provable reasoning: a conclusion content-addressed over its supporting truths, with
   *  the evidence that backs it. Recomputing the same conclusion yields the same id. */
  reason(claim, supportIds) {
    const supports = [...supportIds].sort();
    const conclusion = { type: 'uci.conclusion', claim, supports };
    const id = arves.address(conclusion, 'trace');
    const evidence = supports.flatMap((fid) => this.evidenceOf(fid));
    return { id, claim, supports, evidence: [...new Set(evidence)].sort() };
  }
}

/** Rebuild a memory from an observation log — used to prove deterministic replay. */
export function replay(observations) {
  const m = new CognitiveMemory();
  for (const o of observations) m.ingest(o);
  return m;
}
