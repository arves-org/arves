// ARVES Assistant — PERSISTENT CLI CONFIG (completeness; the daily-driver ergonomics
// a maintainer hits immediately: not re-typing --tenant/--workspace/--wal-dir every run).
//
// A tiny JSON file (default ~/.jarvisrc.json) holds session defaults. Precedence is
// explicit and honest: a CLI flag beats the config file beats the built-in default.
// std-only, offline, deterministic (no clock, no network). Malformed config fails LOUD
// (never silently ignored) so a broken file can't quietly change which shard you write.
//
// HONEST SCOPE: the only reasoner that ships in-repo is the deterministic StubReasoner
// (NOT AI). `reasoner` is accepted for forward-compat but only 'stub' validates here —
// a real LLM plugs in OUTSIDE the repo (src/llm-reasoner.example.mjs), so this config
// cannot conjure intelligence that isn't present.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

/** The keys a config file (and the resolved session) may carry. */
export const CONFIG_KEYS = ['tenant', 'workspace', 'walDir', 'exe', 'reasoner'];

/** Built-in defaults — the floor of the precedence chain. */
export const DEFAULT_SESSION = Object.freeze({
  tenant: 'you', workspace: 'jarvis', walDir: undefined, exe: undefined, reasoner: 'stub',
});

/** Default config path: $JARVIS_HOME/.jarvisrc.json, else ~/.jarvisrc.json. */
export function defaultConfigPath() {
  const home = process.env.JARVIS_HOME || os.homedir() || '.';
  return path.join(home, '.jarvisrc.json');
}

/** Load a config file. A MISSING file is not an error (returns {}); a malformed one is.
 *  Unknown keys are ignored; every recognized value must be a string. */
export function loadConfig(file) {
  let text;
  try { text = fs.readFileSync(file, 'utf8'); }
  catch (e) { if (e.code === 'ENOENT') return {}; throw new Error(`config: cannot read ${file} (${e.code ?? e.message})`); }
  let obj;
  try { obj = JSON.parse(text); }
  catch (e) { throw new Error(`config: ${file} is not valid JSON (${e.message}) — fix or delete it`); }
  if (obj === null || typeof obj !== 'object' || Array.isArray(obj)) {
    throw new Error(`config: ${file} must contain a JSON object`);
  }
  const out = {};
  for (const k of CONFIG_KEYS) {
    if (obj[k] === undefined) continue;
    if (typeof obj[k] !== 'string' || obj[k] === '') throw new Error(`config: '${k}' in ${file} must be a non-empty string`);
    out[k] = obj[k];
  }
  return out;
}

/** Persist a config object (only recognized, non-empty keys), pretty-printed + trailing newline. */
export function saveConfig(file, cfg) {
  const clean = {};
  for (const k of CONFIG_KEYS) if (cfg[k] !== undefined && cfg[k] !== '') clean[k] = cfg[k];
  fs.writeFileSync(file, `${JSON.stringify(clean, null, 2)}\n`);
  return clean;
}

/** Only the in-repo StubReasoner is selectable; anything else fails loudly (honest scope). */
export function validateReasonerChoice(name) {
  if (name !== undefined && name !== 'stub') {
    throw new Error(`config: reasoner '${name}' is not available in-repo — only 'stub' (the deterministic StubReasoner, NOT AI) ships here; plug a real LLM OUTSIDE the repo (see src/llm-reasoner.example.mjs and docs/JARVIS_QUICKSTART.md)`);
  }
  return name ?? 'stub';
}

/** Resolve the effective session opts: CLI flag > config file > built-in default. */
export function resolveSession(cliOpts = {}, cfg = {}) {
  const pick = (k) => (cliOpts[k] !== undefined ? cliOpts[k] : (cfg[k] !== undefined ? cfg[k] : DEFAULT_SESSION[k]));
  return {
    tenant: pick('tenant'),
    workspace: pick('workspace'),
    walDir: pick('walDir'),
    exe: pick('exe'),
    reasoner: validateReasonerChoice(pick('reasoner')),
  };
}
