#!/usr/bin/env node
// `arves` — the Ecosystem Authoring CLI. A third-party developer runs:
//   arves certify <capability-file>   → conformance + certification verdict
//   arves package <capability-file>   → a signed, content-addressed, versioned artifact
// The capability file default-exports { capability, testInputs, source }.

import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { certifyCapability, packageCapability } from '../src/kit.mjs';

const [, , cmd, file] = process.argv;
if (!cmd || !file || (cmd !== 'certify' && cmd !== 'package')) {
  console.error('usage: arves <certify|package> <capability-file>');
  process.exit(2);
}

const mod = await import(pathToFileURL(path.resolve(process.cwd(), file)).href);
const { capability, testInputs, source } = mod.default ?? mod;

if (cmd === 'certify') {
  const c = certifyCapability(capability, testInputs ?? []);
  console.log(`certify ${capability.manifest.name}@${capability.manifest.version}: ${c.certified ? 'CERTIFIED' : 'REJECTED'}`);
  for (const chk of c.checks) console.log(`  ${chk.ok ? '✓' : '✗'} ${chk.name}`);
  process.exit(c.certified ? 0 : 1);
} else {
  const p = packageCapability(capability, source ?? '');
  console.log(`package ${capability.manifest.name}@${p.version}`);
  console.log(`  artifact ${p.id}`);
  process.exit(0);
}
