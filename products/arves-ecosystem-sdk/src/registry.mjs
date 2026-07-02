// ARVES Ecosystem SDK — file-backed LOCAL capability registry (P6.5).
//
// This closes the "publishing my own artifact is demo-only" gap: `arves publish` now writes a
// persistent, content-addressed artifact to a real registry on disk, and `arves install` fetches
// it back and RE-ENFORCES the full trust boundary (signature + tamper + certification) before it
// will hand the artifact id back. Nothing here touches the FROZEN runtime (IDR-006): the registry
// is pure product code, consuming kit.mjs (which itself consumes the frozen ACS codec).
//
// Trust model (identical to CapabilityHost.install, but across a persistence boundary):
//   publish : refuse to store an UNCERTIFIED capability, or a DUPLICATE (same name@version).
//   install : refuse a TAMPERED artifact (verifyArtifact), refuse code that no longer matches the
//             signed codeHash, and refuse a capability that fails a RE-RUN of certification.
// The gate is ENFORCED on read-back, never merely attested by a stored flag.
//
// Storage layout, under <repoRoot>/.arves-registry/ (created on demand; gitignored elsewhere):
//   index.json                         — { "<name>@<version>": { id, file, codeHash } }
//   artifacts/<name>@<version>.json    — the full record (manifest, artifact body, testInputs, ...)
// A record is a JSON document; ARVES values inside testInputs (BigInt) are encoded with a tagged
// wrapper so the round-trip is lossless and the re-derived testInputsHash still matches.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import {
  defineCapability, certifyCapability, packageCapability,
  verifyArtifact, codeHash,
} from './kit.mjs';
// The ACS codec primitives (same source kit.mjs consumes) — used only to derive a content-based
// cache-buster for re-import. No runtime file is touched (IDR-006): this is the product SDK codec.
import { sha256, hex } from '../../arves-sdk-ts/src/codec.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
// repoRoot = .../products/arves-ecosystem-sdk/src -> up three -> repo root.
export const REPO_ROOT = path.resolve(HERE, '..', '..', '..');
export const REGISTRY_DIR = path.resolve(REPO_ROOT, '.arves-registry');

function paths(dir = REGISTRY_DIR) {
  return {
    dir,
    index: path.join(dir, 'index.json'),
    artifacts: path.join(dir, 'artifacts'),
  };
}

// ---- BigInt-safe JSON (so testInputs round-trip losslessly) ----------------
// A BigInt becomes {"$bigint":"<decimal>"}; nothing else in the ARVES input model needs special
// handling for the registry's purposes (Uint8Array etc. are not used as capability *inputs* here,
// and the tamper-evident testInputsHash would catch any lossy round-trip regardless).
function reviveBig(_k, v) {
  if (v && typeof v === 'object' && typeof v.$bigint === 'string') return BigInt(v.$bigint);
  return v;
}
function replaceBig(_k, v) {
  if (typeof v === 'bigint') return { $bigint: v.toString() };
  return v;
}
export function stringifyRecord(obj) { return JSON.stringify(obj, replaceBig, 2); }
export function parseRecord(text) { return JSON.parse(text, reviveBig); }

// A scaffolded capability imports the SDK (kit.mjs / reasoning.mjs) via a path RELATIVE to its own
// on-disk location. Once stored in the registry it will be re-imported from a DIFFERENT directory
// (.arves-registry/.reimport/), so those relative specifiers would no longer resolve. We rewrite
// each relative `from '...'` specifier to an ABSOLUTE file:// URL — resolved against the source's
// ORIGINAL directory — so the stored record is self-contained and re-importable from anywhere.
// This touches only the import LINE, never `execute`'s body, so the signed codeHash is unchanged.
export function absolutizeImports(source, originalDir) {
  return source.replace(
    /(\bfrom\s*)(['"])(\.[^'"]*)\2/g,
    (_m, kw, q, spec) => {
      const abs = path.resolve(originalDir, spec);
      return `${kw}${q}${pathToFileURL(abs).href}${q}`;
    },
  );
}

function readIndex(p) {
  if (!fs.existsSync(p.index)) return {};
  return JSON.parse(fs.readFileSync(p.index, 'utf8'));
}
function writeIndex(p, idx) {
  fs.mkdirSync(p.dir, { recursive: true });
  fs.writeFileSync(p.index, JSON.stringify(idx, null, 2) + '\n');
}

/** List every stored artifact key ("name@version"), sorted. */
export function list(dir = REGISTRY_DIR) {
  return Object.keys(readIndex(paths(dir))).sort();
}

/**
 * Publish a capability + its representative test inputs to the local registry.
 * `sourceFile` is the ABSOLUTE path of the .capability.mjs that default-exported this capability;
 * it is stored so `install` can re-import the real code and re-run certification. Refuses to store
 * an uncertified capability, and refuses to overwrite an existing name@version (immutability).
 * Returns { id, key, file }.
 */
export function publish(cap, testInputs, sourceFile, dir = REGISTRY_DIR) {
  const cert = certifyCapability(cap, testInputs ?? []);
  if (!cert.certified) {
    const failed = cert.checks.filter((c) => !c.ok).map((c) => c.name).join(', ');
    throw new Error(`publish refused: capability fails certification (${failed})`);
  }
  const pkg = packageCapability(cap, testInputs); // throws if <1 test input
  const key = `${cap.manifest.name}@${cap.manifest.version}`;
  const p = paths(dir);
  const idx = readIndex(p);
  if (idx[key]) {
    throw new Error(`publish refused: ${key} already published (registry artifacts are immutable — bump the version)`);
  }
  if (typeof sourceFile !== 'string' || !sourceFile) {
    throw new Error('publish refused: a source file path is required so install can re-verify the real code');
  }
  const absSource = path.resolve(sourceFile);
  if (!fs.existsSync(absSource)) throw new Error(`publish refused: source file not found: ${absSource}`);

  fs.mkdirSync(p.artifacts, { recursive: true });
  const record = {
    type: 'uci.registry-record',
    key,
    id: pkg.id,
    manifest: pkg.artifact.manifest,
    artifact: pkg.artifact,
    testInputs: pkg.testInputs,
    // Store the code so a fetch is self-contained AND record where it came from. The stored code
    // is re-hashed on install and MUST match the signed artifact codeHash. Its SDK import is
    // absolutized so the record re-imports correctly from the registry's own directory.
    codeHash: pkg.artifact.codeHash,
    source: absolutizeImports(fs.readFileSync(absSource, 'utf8'), path.dirname(absSource)),
    sourceFile: absSource,
    publishedAt: 'n/a (deterministic registry; no wall-clock recorded)',
  };
  fs.writeFileSync(path.join(p.artifacts, `${key}.json`), stringifyRecord(record) + '\n');
  idx[key] = { id: pkg.id, file: `artifacts/${key}.json`, codeHash: pkg.artifact.codeHash };
  writeIndex(p, idx);
  return { id: pkg.id, key, file: idx[key].file };
}

/** Read a stored record (parsed, BigInt-revived) or throw if not found. */
export function fetchRecord(key, dir = REGISTRY_DIR) {
  const p = paths(dir);
  const idx = readIndex(p);
  const entry = idx[key];
  if (!entry) throw new Error(`not found: ${key} (have: ${Object.keys(idx).sort().join(', ') || 'none'})`);
  const file = path.join(p.dir, entry.file);
  return parseRecord(fs.readFileSync(file, 'utf8'));
}

/**
 * Install a capability from the local registry BY re-enforcing the whole trust boundary:
 *   1. re-import the stored source and rebuild the live capability,
 *   2. verify the rebuilt code hashes to the signed artifact codeHash (no swapped code),
 *   3. verifyArtifact — the content-addressed signature + travelling test-input hash,
 *   4. RE-RUN certification against the artifact's own test inputs (gate enforced, not attested).
 * Returns { id, key, capability } on success; throws (refusing) otherwise.
 *
 * Re-importing runs the stored module. We reconstruct the capability from its default export so
 * codeHash/certification operate on the SAME live object a host would install.
 */
export async function install(key, dir = REGISTRY_DIR) {
  const record = fetchRecord(key, dir);

  // (1) Re-import the STORED source (never the original on-disk file, which could have changed or
  // vanished) so the registry is self-contained and tamper-evident: if a stored record's code was
  // mutated, the live `execute` reflects the mutation and step (2) catches it via codeHash. The
  // stored source's SDK import was absolutized at publish time, so it resolves from anywhere. For
  // reasoning capabilities (identical execute wrapper), step (2b) binds the closed-over reasoner via
  // a signed reasonerHash — codeHash alone cannot see a reasoner swap.
  const p = paths(dir);
  const reimportDir = path.join(p.dir, '.reimport');
  fs.mkdirSync(reimportDir, { recursive: true });
  // Cache-bust the ESM loader with a hash of the ACTUAL stored source, not the record id: a
  // mutated record must re-import (Node caches modules by resolved URL, so a fixed name/id would
  // return the stale first import and hide the tamper). Hashing the bytes makes each distinct
  // source a distinct URL.
  const srcHash = hex(sha256(new TextEncoder().encode(record.source)));
  const tmp = path.join(reimportDir, `${key}.${srcHash.slice(0, 16)}.mjs`);
  fs.writeFileSync(tmp, record.source);
  const importUrl = pathToFileURL(tmp).href + `?v=${srcHash}`;
  const mod = await import(importUrl);
  const exported = mod.default ?? mod;
  const cap = exported.capability ?? exported;
  const testInputs = record.testInputs;

  // (2) Code integrity: the live code MUST match the signed artifact.
  if (codeHash(cap) !== record.artifact.codeHash) {
    throw new Error('install refused: stored capability code does not match the signed artifact (tampered)');
  }
  // (2b) Reasoning-logic integrity (closure-audit finding): a reasoning capability's execute wrapper
  // is byte-identical across ALL reasoners, so codeHash alone is vacuous for them — the logic is a
  // closed-over reasoner. Its source hash is signed into the artifact manifest (reasonerHash); re-derive
  // it from the rebuilt capability and compare against the SIGNED value. Mutating the stored reasoner
  // changes the live source (mismatch here); mutating the signed reasonerHash breaks verifyArtifact (3).
  const signedReasonerHash = record.artifact.manifest && record.artifact.manifest.reasonerHash;
  if (signedReasonerHash) {
    const liveReasonerHash = hex(sha256(new TextEncoder().encode(cap.reasonerSource ?? '')));
    if (liveReasonerHash !== signedReasonerHash) {
      throw new Error('install refused: reasoning logic does not match the signed artifact (tampered reasoner)');
    }
  }
  // (3) Signature + test-input integrity.
  const pkg = { id: record.id, artifact: record.artifact, testInputs };
  if (!verifyArtifact(pkg)) {
    throw new Error('install refused: artifact signature / test-inputs do not verify (tampered)');
  }
  // (4) Re-run certification — never trust a stored flag.
  const cert = certifyCapability(cap, testInputs);
  if (!cert.certified) {
    const failed = cert.checks.filter((c) => !c.ok).map((c) => c.name).join(', ');
    throw new Error(`install refused: capability fails re-certification (${failed})`);
  }
  return { id: record.id, key, capability: cap };
}

// Re-export so callers (the CLI) get one import surface.
export { defineCapability };
