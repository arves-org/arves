// ARVES Marketplace (P7) — the distribution layer for the ecosystem. One party publishes
// a certified, signed capability; ANY other party discovers and installs it and runs it on
// the frozen Runtime v1.0. The marketplace itself is pure distribution: it holds no truth,
// runs nothing, and never touches the runtime — it only accepts artifacts that are
// certified (conformance) and whose content-addressed signature verifies (integrity).
//
// This is what turns "we built products" into "others ship products others install."

import { verifyArtifact, certifyCapability, codeHash } from '../../arves-ecosystem-sdk/src/kit.mjs';

export class Marketplace {
  #catalog = new Map(); // "name@version" → { pkg, cap, publisher }

  /** Publish a packaged capability. The certification gate is ENFORCED, not attested: the
   *  marketplace re-runs certification itself against the artifact's own tamper-evident test
   *  inputs (a forged `certified:true` is worthless here). It also refuses a tampered/unsigned
   *  artifact and a capability whose code does not match the signed artifact; a version, once
   *  published, is immutable (supersede with a new version). */
  publish({ pkg, cap, publisher }) {
    if (!verifyArtifact(pkg)) throw new Error('marketplace: refuse tampered/unsigned artifact');
    if (codeHash(cap) !== pkg.artifact.codeHash) throw new Error('marketplace: capability code does not match the signed artifact');
    const cert = certifyCapability(cap, pkg.testInputs);
    if (!cert.certified) throw new Error('marketplace: refuse uncertified capability (' + cert.checks.filter((c) => !c.ok).map((c) => c.name).join(', ') + ')');
    const key = `${cap.manifest.name}@${cap.manifest.version}`;
    if (this.#catalog.has(key)) throw new Error(`marketplace: ${key} already published (versions are immutable)`);
    this.#catalog.set(key, { pkg, cap, publisher: publisher ?? 'unknown' });
    return pkg.id;
  }

  /** Discover published capabilities. */
  list() {
    return [...this.#catalog.entries()]
      .map(([id, v]) => ({ id, artifact: v.pkg.id, publisher: v.publisher }))
      .sort((a, b) => (a.id < b.id ? -1 : 1));
  }

  fetch(name, version) { return this.#catalog.get(`${name}@${version}`) ?? null; }

  /** A consumer installs a published capability into ITS OWN host. The host independently
   *  re-verifies the signature AND re-runs certification (enforced, not attested). The
   *  publisher and the consumer never coordinate. */
  install(name, version, host) {
    const e = this.fetch(name, version);
    if (!e) throw new Error(`marketplace: ${name}@${version} not found`);
    return host.install(e.pkg, e.cap);
  }
}
