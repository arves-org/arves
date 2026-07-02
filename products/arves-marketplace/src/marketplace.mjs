// ARVES Marketplace (P7) — the distribution layer for the ecosystem. One party publishes
// a certified, signed capability; ANY other party discovers and installs it and runs it on
// the frozen Runtime v1.0. The marketplace itself is pure distribution: it holds no truth,
// runs nothing, and never touches the runtime — it only accepts artifacts that are
// certified (conformance) and whose content-addressed signature verifies (integrity).
//
// This is what turns "we built products" into "others ship products others install."

import { verifyArtifact } from '../../arves-ecosystem-sdk/src/kit.mjs';

export class Marketplace {
  #catalog = new Map(); // "name@version" → { pkg, cap, cert, publisher }

  /** Publish a packaged capability. Refused unless it is certified and its signature
   *  verifies; a version, once published, is immutable (supersede with a new version). */
  publish({ pkg, cap, cert, publisher }) {
    if (!cert || !cert.certified) throw new Error('marketplace: refuse uncertified capability');
    if (!verifyArtifact(pkg)) throw new Error('marketplace: refuse tampered/unsigned artifact');
    const key = `${cap.manifest.name}@${cap.manifest.version}`;
    if (this.#catalog.has(key)) throw new Error(`marketplace: ${key} already published (versions are immutable)`);
    this.#catalog.set(key, { pkg, cap, cert, publisher: publisher ?? 'unknown' });
    return pkg.id;
  }

  /** Discover published capabilities. */
  list() {
    return [...this.#catalog.entries()]
      .map(([id, v]) => ({ id, artifact: v.pkg.id, publisher: v.publisher }))
      .sort((a, b) => (a.id < b.id ? -1 : 1));
  }

  fetch(name, version) { return this.#catalog.get(`${name}@${version}`) ?? null; }

  /** A consumer installs a published capability into ITS OWN host (cert-gated +
   *  signature-verified by the host). The publisher and the consumer never coordinate. */
  install(name, version, host) {
    const e = this.fetch(name, version);
    if (!e) throw new Error(`marketplace: ${name}@${version} not found`);
    return host.install(e.pkg, e.cap, e.cert);
  }
}
