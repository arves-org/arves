#!/usr/bin/env node
// `arves` — the Ecosystem Authoring CLI. The developer-experience front door: go from an
// idea to a certified, signed, publishable capability without touching the ARVES runtime.
//
//   arves init <name>        → scaffold a green, certifiable capability file
//   arves create <name> --provider <p> → scaffold a REASONING capability (LLM-backed, auditable)
//   arves doctor <file>      → conformance assistant: explain every violation + its exact fix
//   arves certify <file>     → conformance + certification verdict
//   arves package <file>     → a signed, content-addressed, versioned artifact
//   arves publish <file>     → certify + package, then store in the persistent local registry
//   arves install <name@ver> → fetch from the local registry, re-verify + re-certify, print id
//
// A capability file default-exports { capability, testInputs, source }.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { defineCapability, certifyCapability, packageCapability } from '../src/kit.mjs';
import { publish as registryPublish, install as registryInstall } from '../src/registry.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const KIT = path.resolve(HERE, '..', 'src', 'kit.mjs');
const REASONING = path.resolve(HERE, '..', 'src', 'reasoning.mjs');
const CMDS = ['init', 'create', 'doctor', 'certify', 'package', 'publish', 'install'];

const HELP = `arves — the ARVES Ecosystem Authoring CLI

usage: arves <command> <name-or-file> [options]

commands:
  init <name>                 scaffold a green, certifiable capability file (<name>.capability.mjs)
  create <name> --provider <p> scaffold a REASONING capability. <p> = reference | local | claude | gpt | gemini
                              (default: reference). reference/local are deterministic → certify+replay
                              offline; claude/gpt/gemini scaffold an adapter STUB needing integration.
  doctor <file>               conformance assistant: report every violation and its exact fix
  certify <file>              run certification and print the PASS/FAIL verdict + per-check status
  package <file>              produce a signed, content-addressed, versioned artifact
  publish <file>              certify + package, then store the artifact in the local registry
  install <name@version>      fetch from the local registry, re-verify signature + re-run
                              certification, then print the artifact id

A capability file default-exports { capability, testInputs, source }.
Authoring needs only Node >=18 — no Rust build required.
Docs: the documentation site (docs-site/) or products/arves-ecosystem-sdk/README.md`;

const [, , cmd, arg, ...rest] = process.argv;
if (!cmd || cmd === '--help' || cmd === '-h' || cmd === 'help') { console.log(HELP); process.exit(cmd ? 0 : 2); }
if (!CMDS.includes(cmd)) { console.error(`arves: unknown command '${cmd}'\n`); console.log(HELP); process.exit(2); }
if (arg === '--help' || arg === '-h') { console.log(HELP); process.exit(0); }
const NAME_ARG = { init: 'name', create: 'name', install: 'name@version' };
if (!arg) { console.error(`usage: arves ${cmd} <${NAME_ARG[cmd] ?? 'file'}>   (try: arves --help)`); process.exit(2); }

// A tiny flag reader for `--provider <p>` (and `--provider=<p>`), used by `create`.
function readFlag(argv, flag) {
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === flag) return argv[i + 1];
    if (argv[i].startsWith(flag + '=')) return argv[i].slice(flag.length + 1);
  }
  return undefined;
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

// ---- create: scaffold a REASONING (LLM-backed) capability -------------------
// A reasoning capability wraps a provider whose output is committed ONCE as content-addressed
// truth; replay reads the recorded trace and NEVER re-calls the provider (ORCH-003; ACS-005
// GL-012). That doctrine is what turns a swappable, possibly non-deterministic LLM into a
// deterministic, auditable step inside ARVES — the moat vs. a plain wrapper.
if (cmd === 'create') {
  const name = arg.replace(/[^A-Za-z0-9._-]/g, '').trim();
  if (!name) { console.error('create: give a capability name, e.g. `arves create triage.summary --provider reference`'); process.exit(2); }
  const KNOWN = ['reference', 'local', 'claude', 'gpt', 'gemini'];
  const provider = (readFlag([arg, ...rest], '--provider') ?? 'reference').trim();
  if (!KNOWN.includes(provider)) {
    console.error(`create: unknown provider '${provider}' — choose one of: ${KNOWN.join(', ')}`);
    process.exit(2);
  }
  const deterministic = provider === 'reference' || provider === 'local';
  const target = path.resolve(process.cwd(), `${name}.capability.mjs`);
  // Same relative-path computation as `init`, but to the reasoning module (the shared contract).
  let rel = path.relative(path.dirname(target), REASONING).split(path.sep).join('/');
  if (!rel.startsWith('.')) rel = './' + rel;

  const stubNote = deterministic ? '' : `
// NOTE: provider '${provider}' is an ADAPTER STUB. Providers.${provider}.reason(input) THROWS until
// you supply an API adapter + key at the integration point (no network / no keys live in-repo).
// Until then this capability will NOT certify (execute throws) — that is intentional: the
// deterministic-truth doctrine below is what makes it auditable once the adapter is wired in.`;

  const reasonBlock = deterministic
    ? `  // Deterministic reasoning: a pure function of the input, so this capability CERTIFIES and
  // REPLAYS offline. Swap in a live provider later; the doctrine below keeps replay honest.
  reason: (input) => 'reasoned(' + String(input.prompt ?? '') + ')',`
    : `  // Live provider: Providers.${provider}.reason(input) is the integration point. It throws until
  // an adapter is supplied — wire it up, then the FIRST call's output is committed as truth.
  provider: Providers['${provider}'],`;

  const tpl = `// ${name} — an ARVES REASONING capability (provider: ${provider}).
// Authored with ONLY the Ecosystem Author SDK; the FROZEN runtime is never touched.
//
// DOCTRINE (ORCH-003; ACS-005 GL-012): a provider's output is committed ONCE as
// content-addressed truth. REPLAY reads the recorded trace — it NEVER re-calls the provider.
// That is what makes a swappable, possibly non-deterministic LLM behave deterministically and
// remain auditable inside ARVES: the moat vs. a plain API wrapper.${stubNote}
import { defineReasoningCapability, Providers } from '${rel}';

export const capability = defineReasoningCapability({
  name: '${name}',
  version: '1.0.0',
  produces: ['uci.reasoning'],
${reasonBlock}
});

// At least one representative input — certification runs execute() on these.
export const testInputs = [
  { prompt: 'example: summarize the incident' },
];

export const source = '${name}@1.0.0 (provider=${provider})';
export default { capability, testInputs, source };
void Providers;
`;
  try {
    fs.writeFileSync(target, tpl, { flag: 'wx' });
  } catch (e) {
    if (e.code === 'EEXIST') { console.error(`create: ${name}.capability.mjs already exists (refusing to overwrite)`); process.exit(1); }
    throw e;
  }
  console.log(`created ${name}.capability.mjs   (reasoning capability, provider=${provider})`);
  if (deterministic) {
    console.log(`next:  arves doctor ${name}.capability.mjs   # deterministic → should certify offline`);
  } else {
    console.log(`next:  wire an adapter for provider '${provider}' (it throws until integrated), then certify`);
  }
  process.exit(0);
}

// ---- install: fetch from the local registry, re-verify + re-certify ---------
// Does NOT load a local file — `arg` is a name@version key resolved against the registry.
if (cmd === 'install') {
  try {
    const res = await registryInstall(arg);
    console.log(`install ${res.key}: VERIFIED + RE-CERTIFIED`);
    console.log(`  artifact ${res.id}`);
    process.exit(0);
  } catch (e) {
    console.error(`install ${arg}: REFUSED — ${e.message}`);
    process.exit(1);
  }
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

if (cmd === 'package') {
  void source; // author's human-readable source note; the artifact binds code + test inputs
  const p = packageCapability(capability, testInputs ?? []);
  console.log(`package ${capability.manifest.name}@${p.version}`);
  console.log(`  artifact ${p.id}`);
  process.exit(0);
}

// publish: certify + package, then store the artifact in the persistent local registry.
// The source FILE path travels with the record so `install` can re-import the real code and
// re-run the whole trust boundary (signature + tamper + certification) on read-back.
if (cmd === 'publish') {
  void source;
  const sourceFile = path.resolve(process.cwd(), arg);
  try {
    const res = registryPublish(capability, testInputs ?? [], sourceFile);
    console.log(`publish ${res.key}: STORED in local registry`);
    console.log(`  artifact ${res.id}`);
    console.log(`  record   ${res.file}`);
    console.log(`next:  arves install ${res.key}   # fetch back, re-verify + re-certify`);
    process.exit(0);
  } catch (e) {
    console.error(`publish ${capability.manifest.name}@${capability.manifest.version}: REFUSED — ${e.message}`);
    process.exit(1);
  }
}
