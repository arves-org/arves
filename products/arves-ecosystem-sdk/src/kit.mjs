// ARVES Ecosystem SDK & Authoring Kit (P6.5) — the layer that lets a THIRD PARTY build on
// ARVES without ever touching the runtime. A developer authors a capability, certifies it
// (conformance), packages it (a signed, content-addressed artifact), and a host installs +
// invokes it — its effects becoming truth in the FROZEN Runtime v1.0. The runtime never
// changes: a third-party capability's CODE is unknown to it; only the ACS truth it produces
// crosses the boundary. This is how "someone else's code" runs on ARVES.
//
// The success test: a stranger, using only this Kit, publishes a working capability in
// minutes — no runtime source, no Runtime Change Request.

import { Arves } from '../../arves-sdk-ts/src/arves.mjs';
import { sha256, hex } from '../../arves-sdk-ts/src/codec.mjs';

const arves = new Arves();

// Re-exported so a capability author has the full ARVES value model without reaching into
// runtime internals. An effect `value` MUST be one of: null · boolean · BigInt (integer in
// [-2^64, 2^64-1]) · float(x) (a float wrapper) · string (UTF-8, NFC) · Uint8Array (bytes) ·
// Array · plain object (map; keys are strings or BigInt). A BARE JS number is rejected
// (ambiguous int/float, lossy beyond 2^53) — use BigInt or float().
export { float } from '../../arves-sdk-ts/src/codec.mjs';

// ---- Capability Author SDK -------------------------------------------------

/** Author a capability. `execute(input)` returns an array of effects
 *  `{ target, value }`, where `target` is a declared produce and `value` is any ARVES
 *  value. The runtime commits these effects as truth; the capability is pure product code. */
export function defineCapability({ name, version, produces, execute, determinism = 'deterministic' }) {
  if (typeof name !== 'string' || !name) throw new Error('capability: name required');
  if (typeof version !== 'string' || !version) throw new Error('capability: version required');
  if (!Array.isArray(produces) || produces.length === 0) throw new Error('capability: produces[] required');
  if (typeof execute !== 'function') throw new Error('capability: execute(input) required');
  return {
    manifest: { type: 'uci.capability-manifest', name, version, produces: [...produces].sort(), determinism },
    execute,
  };
}

// ---- Certification (conformance) -------------------------------------------

/** Certify a capability against the ARVES contract, using representative test inputs.
 *  Checks: manifest validity · every effect targets a declared produce · every effect is
 *  ACS-canonical (encodable/addressable) · determinism (same input → same effect
 *  addresses). Returns `{ certified, checks }`. An uncertified capability MUST NOT be
 *  installed. */
export function certifyCapability(cap, testInputs) {
  const checks = [];
  const add = (name, ok, detail = '') => checks.push({ name, ok, detail });

  add('manifest-valid',
    !!cap.manifest.name && !!cap.manifest.version && cap.manifest.produces.length > 0,
    'name, version, and a non-empty produces[] are required');

  // Certification MUST NOT pass vacuously: a capability with no representative inputs
  // exercises no effect, so its checks would pass trivially. Require ≥1 input.
  if (!Array.isArray(testInputs) || testInputs.length === 0) {
    add('has-test-inputs', false, 'certification requires >=1 representative test input (else checks pass vacuously)');
    return { certified: false, checks };
  }
  add('has-test-inputs', true);

  let targetsDeclared = { ok: true, detail: '' };
  let acsCanonical = { ok: true, detail: '' };
  let deterministic = { ok: true, detail: '' };
  for (const input of testInputs) {
    let e1;
    let e2;
    try {
      e1 = cap.execute(input);
      e2 = cap.execute(input);
    } catch (err) {
      acsCanonical = { ok: false, detail: `execute threw on input ${JSON.stringify(input)}: ${err.message}` };
      continue;
    }
    const addr = (effs) => effs.map((x) => {
      if (!cap.manifest.produces.includes(x.target)) {
        targetsDeclared = { ok: false, detail: `effect target '${x.target}' not in produces ${JSON.stringify(cap.manifest.produces)}` };
      }
      try { return arves.address(x.value, 'commit'); } catch (err) {
        acsCanonical = { ok: false, detail: `effect value is not ACS-canonical (${err.message}) — see the value model in the README` };
        return 'ERR';
      }
    }).join(',');
    if (addr(e1) !== addr(e2)) deterministic = { ok: false, detail: `execute is non-deterministic for input ${JSON.stringify(input)}` };
  }
  add('effects-declared', targetsDeclared.ok, targetsDeclared.detail);
  add('effects-acs-canonical', acsCanonical.ok, acsCanonical.detail);
  add('deterministic', deterministic.ok, deterministic.detail);

  return { certified: checks.every((c) => c.ok), checks };
}

// ---- Packaging (content-addressed signing) ---------------------------------

/** The content hash of a capability's ACTUAL executable code. This is what the artifact
 *  signature covers — real code integrity, not an author-claimed string. */
export function codeHash(cap) {
  return hex(sha256(new TextEncoder().encode(cap.execute.toString())));
}

/** Package a capability into a signed, versioned artifact. The "signature" is the ACS
 *  content address of `{ manifest, codeHash }`, where `codeHash` is over the REAL execute
 *  code — content-addressed integrity: any tamper with the manifest OR the code changes the
 *  artifact id (self-verifying; no PKI). */
export function packageCapability(cap) {
  const body = { type: 'uci.capability-artifact', manifest: cap.manifest, codeHash: codeHash(cap) };
  const id = arves.address(body, 'engine'); // domain: engine-manifest
  return { id, artifact: body, version: cap.manifest.version };
}

/** Verify an artifact's content-addressed signature (re-derive its id from its bytes). */
export function verifyArtifact(pkg) {
  return arves.address(pkg.artifact, 'engine') === pkg.id;
}

// ---- Host (install + invoke against the frozen runtime) --------------------

/** A capability host: installs CERTIFIED capabilities and invokes them, committing their
 *  effects as truth via the frozen Runtime v1.0 bridge. The runtime is never modified. */
export class CapabilityHost {
  #bridge;
  #installed = new Map();

  constructor(bridge) { this.#bridge = bridge; }

  /** Install a packaged capability — refused unless it is certified and its artifact
   *  signature verifies (tamper-evident). */
  install(pkg, cap, certification) {
    if (!certification || !certification.certified) throw new Error('install refused: capability is not certified');
    if (!verifyArtifact(pkg)) throw new Error('install refused: artifact signature does not verify (tampered)');
    // Real code integrity: the capability's actual code MUST match the signed artifact —
    // a swapped implementation under a valid artifact is refused.
    if (codeHash(cap) !== pkg.artifact.codeHash) throw new Error('install refused: capability code does not match the signed artifact');
    this.#installed.set(cap.manifest.name, { pkg, cap });
    return pkg.id;
  }

  installed() { return [...this.#installed.keys()].sort(); }

  /** Invoke an installed capability: run its (product-layer) code, commit each effect as
   *  ACS truth in the real Kernel. The runtime commits bytes it never had to know the
   *  shape of — third-party code, first-party truth. */
  async invoke(name, input) {
    const entry = this.#installed.get(name);
    if (!entry) throw new Error(`invoke refused: '${name}' is not installed`);
    const effects = entry.cap.execute(input);
    const truths = [];
    for (const eff of effects) {
      const res = await this.#bridge.commit(eff.value, 'commit');
      truths.push({ target: eff.target, id: res.contentId, status: res.status });
    }
    return { capability: name, artifact: entry.pkg.id, truths };
  }
}
