// ARVES SDK — the ergonomic developer API (P1 Developer Platform).
//
// The point of this class: a developer builds a deterministic, content-addressed,
// replayable cognitive app WITHOUT knowing anything about dCBOR bytes or multihashes.
// Everything below is a thin, safe facade over the conformant ACS codec in codec.mjs.
//
// SCOPE CAVEAT (honest): `class Arves` and `FactStore` below are an IN-MEMORY reference
// substrate — the identity/idempotency/integrity/replay guarantees they demonstrate are
// computed from the ACS content address (that part is real and byte-conformant), but the
// store itself is a plain JS `Map`. It has NO WAL, NO durable persistence, NO crash
// recovery, and NO cross-process audit ledger. It is NOT the frozen Rust reference Kernel.
// The real Kernel path exists and is a peer file — `bridge.mjs` (`KernelBridge`) talks to
// the `arves-bridge` server, which commits to the WAL-backed Rust Kernel — but that path is
// NOT the default and this class does not use it. So when a product built on `class Arves`
// says "persistence / replay / audit", read it as: content-addressed IN MEMORY; durable
// persistence/replay/recovery require routing through `bridge.mjs` to the real Kernel.

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
 *  is automatically deduplicated, because identity is the content address.
 *  IN-MEMORY ONLY: `#facts` is a `Map`; it is not durable and is lost on process exit.
 *  For WAL-backed, replayable, recoverable truth, commit through `bridge.mjs` to the real
 *  Kernel instead (see the SCOPE CAVEAT at the top of this file). */
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
