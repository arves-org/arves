#!/usr/bin/env node
// `arves` — the Ecosystem Authoring CLI. The developer-experience front door: go from an
// idea to a certified, signed, publishable capability without touching the ARVES runtime.
//
//   arves init <name>        → scaffold a green, certifiable capability file
//   arves doctor <file>      → conformance assistant: explain every violation + its exact fix
//   arves certify <file>     → conformance + certification verdict
//   arves package <file>     → a signed, content-addressed, versioned artifact
//
// A capability file default-exports { capability, testInputs, source }.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { defineCapability, certifyCapability, packageCapability } from '../src/kit.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const KIT = path.resolve(HERE, '..', 'src', 'kit.mjs');
const CMDS = ['init', 'doctor', 'certify', 'package'];

const [, , cmd, arg] = process.argv;
if (!cmd || !arg || !CMDS.includes(cmd)) {
  console.error('usage: arves <init|doctor|certify|package> <name-or-file>');
  process.exit(2);
}

// ---- init: scaffold a working, certifiable capability -----------------------
if (cmd === 'init') {
  const name = arg.replace(/[^A-Za-z0-9._-]/g, '').trim();
  if (!name) { console.error('init: give a capability name, e.g. `arves init hospital.incident`'); process.exit(2); }
  const target = path.resolve(process.cwd(), `${name}.capability.mjs`);
  // A relative import so the scaffold runs immediately inside this repo; external
  // authors installing the package would import from '@arves/ecosystem-sdk' instead.
  let rel = path.relative(path.dirname(target), KIT).split(path.sep).join('/');
  if (!rel.startsWith('.')) rel = './' + rel;
  const tpl = `// ${name} — an ARVES capability. Authored with ONLY the Ecosystem Author SDK;
// the ARVES runtime is never touched. Pure + deterministic so it certifies and replays.
// External authors: import from '@arves/ecosystem-sdk' instead of the relative path below.
import { defineCapability, float } from '${rel}';

export const capability = defineCapability({
  name: '${name}',
  version: '1.0.0',
  produces: ['uci.fact'],
  // Return an array of effects { target, value }. \`value\` is an ARVES value: null · boolean ·
  // BigInt (integer) · float(x) · string (UTF-8) · Uint8Array · Array · plain object (string/BigInt keys).
  // A bare JS number is rejected — use BigInt or float(x). Timestamps: epoch-ms * 1_000_000n (ns).
  execute: (input) => [{
    target: 'uci.fact',
    value: {
      type: 'uci.fact',
      entity: String(input.entity),
      event: String(input.event),
    },
  }],
});

// At least one representative input — certification runs \`execute\` on these.
export const testInputs = [
  { entity: 'example:1', event: 'created' },
];

export const source = '${name}@1.0.0';
export default { capability, testInputs, source };
void float; // available for float values, e.g. value: float(0.5)
`;
  try {
    fs.writeFileSync(target, tpl, { flag: 'wx' });
  } catch (e) {
    if (e.code === 'EEXIST') { console.error(`init: ${name}.capability.mjs already exists (refusing to overwrite)`); process.exit(1); }
    throw e;
  }
  console.log(`created ${name}.capability.mjs`);
  console.log(`next:  arves doctor ${name}.capability.mjs   # then certify, then package`);
  process.exit(0);
}

// ---- everything else loads the capability module ----------------------------
const mod = await import(pathToFileURL(path.resolve(process.cwd(), arg)).href);
const { capability, testInputs, source } = mod.default ?? mod;

// A conformance assistant: turns each failed check into a plain-language remedy.
const REMEDY = {
  'manifest-valid': 'defineCapability needs a non-empty name, version, and produces[] (an array of the effect targets you emit).',
  'has-test-inputs': 'export `testInputs` with >=1 representative input — certification runs execute() on them, so an empty set would pass vacuously.',
  'effects-declared': 'every effect { target } your execute() returns must be listed in the capability\'s produces[]. Add the target to produces[], or fix the target string.',
  'effects-acs-canonical': 'an effect value is not an ARVES value. Use BigInt for integers, float(x) for floats (never a bare JS number), string/Uint8Array/Array/plain-object; object keys must be string or BigInt.',
  'deterministic': 'execute() must be pure: the same input must yield the same effects. Remove Math.random(), Date.now(), counters, or other mutable/ambient state.',
};

if (cmd === 'doctor') {
  const c = certifyCapability(capability, testInputs ?? []);
  const label = `${capability.manifest.name}@${capability.manifest.version}`;
  if (c.certified) {
    console.log(`doctor ${label}: HEALTHY — all conformance checks pass.`);
    for (const chk of c.checks) console.log(`  ✓ ${chk.name}`);
    console.log('next:  arves package ' + arg + '   # produce a signed, publishable artifact');
    process.exit(0);
  }
  console.log(`doctor ${label}: NOT YET CONFORMANT — ${c.checks.filter((x) => !x.ok).length} issue(s) to fix:\n`);
  for (const chk of c.checks) {
    if (chk.ok) { console.log(`  ✓ ${chk.name}`); continue; }
    console.log(`  ✗ ${chk.name}`);
    if (chk.detail) console.log(`      found : ${chk.detail}`);
    console.log(`      fix   : ${REMEDY[chk.name] ?? 'see the ecosystem-sdk README value model.'}`);
  }
  process.exit(1);
}

if (cmd === 'certify') {
  const c = certifyCapability(capability, testInputs ?? []);
  console.log(`certify ${capability.manifest.name}@${capability.manifest.version}: ${c.certified ? 'CERTIFIED' : 'REJECTED'}`);
  for (const chk of c.checks) console.log(`  ${chk.ok ? '✓' : '✗'} ${chk.name}${chk.ok ? '' : `  — ${chk.detail || REMEDY[chk.name] || ''}`}`);
  if (!c.certified) console.log('tip:   run `arves doctor ' + arg + '` for per-issue fixes.');
  process.exit(c.certified ? 0 : 1);
}

// package
void source; // author's human-readable source note; the artifact binds code + test inputs
const p = packageCapability(capability, testInputs ?? []);
console.log(`package ${capability.manifest.name}@${p.version}`);
console.log(`  artifact ${p.id}`);
process.exit(0);
