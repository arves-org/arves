// ARVES SDK — the ergonomic developer API (P1 Developer Platform).
//
// The point of this class: a developer builds a deterministic, content-addressed,
// replayable cognitive app WITHOUT knowing anything about dCBOR bytes or multihashes.
// Everything below is a thin, safe facade over the conformant ACS codec in codec.mjs.

import { encode, contentId, hex, float, DOMAIN } from './codec.mjs';

export class Arves {
  /** Wrap a floating-point number as an ACS Float (distinct from a BigInt Integer). */
  float(v) { return float(v); }

  /** The content address (ContentId, hex) of any ARVES value under a domain.
   *  Identical values — regardless of map key order, platform, or language — get the
   *  identical address. This is the one guarantee everything else is built on. */
  address(value, domain = 'commit') {
    const tag = DOMAIN[domain];
    if (tag === undefined) throw new Error(`ARVES: unknown domain '${domain}'`);
    return hex(contentId(tag, encode(value)));
  }

  /** Commit a fact and get its stable ContentId (idempotency key). */
  commit(fact) { return this.address(fact, 'commit'); }

  /** True iff `value` still hashes to `expectedId` — tamper / integrity check. */
  verify(value, expectedId, domain = 'commit') { return this.address(value, domain) === expectedId; }

  /** Address a decision trace (an ordered array of ContentIds or values). Recomputing
   *  the same trace yields the same root address — deterministic replay. */
  traceRoot(steps) { return this.address(steps, 'trace'); }
}

/** A minimal content-addressed, idempotent fact store — the smallest thing that shows
 *  why ARVES matters: committing the "same" fact twice (even with reordered fields)
 *  is automatically deduplicated, because identity is the content address. */
export class FactStore {
  #arves = new Arves();
  #facts = new Map();

  commit(fact) {
    const id = this.#arves.commit(fact);
    if (!this.#facts.has(id)) this.#facts.set(id, fact);
    return id;
  }
  get(id) { return this.#facts.get(id); }
  get size() { return this.#facts.size; }
  ids() { return [...this.#facts.keys()]; }
}
