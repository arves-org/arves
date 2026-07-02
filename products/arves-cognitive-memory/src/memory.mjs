// ARVES Cognitive Memory (P2) — the common core of every ARVES product.
//
// It consumes the P1 SDK (which consumes the platform). It modifies no platform file
// (IDR-006). It proves six ARVES capabilities on real data:
//   Identity · Deduplication · Evidence · Truth · Audit (tamper-evident) · Replay,
// plus provable reasoning. The insight throughout: identity IS the content address, so
// facts from unrelated systems that mean the same thing collapse to one truth for free.

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

// Event normalization: different labels -> one stable event key.
function eventKey(text) {
  return text.toLowerCase().replace(/\bmeeting\b/g, '').trim().replace(/\s+/g, '-');
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
  auditTrail() { return this.#log; }
  head() { return this.#head; }

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
