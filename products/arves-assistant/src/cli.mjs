// ARVES Assistant — the JARVIS CLI / REPL (product maturity, OQ-2).
//
// A local command line the maintainer runs to drive their assistant interactively OR in a
// script, over a REAL KernelBridge on their own shard + WAL directory. Two modes:
//   - one-shot:  `node bin/jarvis.mjs --wal-dir DIR observe email urn:you dentist 2026-07-06T09:00:00Z`
//   - REPL:      `node bin/jarvis.mjs --wal-dir DIR`   then type commands (scriptable via piped stdin)
//
// Every command runs through the SAME frozen Runtime v1.0 surface the rest of the product
// uses (bridge line protocol; IDR-006 — no runtime code is linked). Durability is real: a
// fresh process over the same --wal-dir rebuilds its memory READ-ONLY from the WAL (RCR-033
// scan, via Assistant.recoverFromWal()), so `recall`/`why` in a brand-new process explain a
// decision made by a prior process.
//
// HONESTY (stated, not hidden): the reasoner is the deterministic keyword-table StubReasoner
// (NOT AI); the skills are the offline example library; there is no network anywhere. Plug an
// LLM into the Reasoner slot per docs/JARVIS_QUICKSTART.md — the CLI governance path
// (proposal-as-truth -> guardrail gate -> certified-skill-only execution) is identical. Scope
// is single host, no authN on commit (v2.0 debt #8) — an approval's role is structural, not
// cryptographic. A probe is a probe.

import readline from 'node:readline';
import { Assistant } from './assistant.mjs';
import { StubReasoner } from './reasoner.mjs';
import { registerExampleSkills } from './example-skills.mjs';
import { CONNECTORS, connectorByName } from './connectors.mjs';
import { why, renderWhy } from './why.mjs';
import { reportDay, renderReport } from './report.mjs';
import { CONFIG_KEYS, defaultConfigPath, loadConfig, saveConfig, resolveSession, validateReasonerChoice } from './config.mjs';

const ISO_UTC = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/;
const DEFAULT_POLICY = 'spend-needs-approval';

const short = (id) => (id ? `${id.slice(0, 16)}…` : '-');

/** Parse `--tenant/--workspace/--wal-dir/--exe/--config` flags; everything else is the
 *  command. Flag values are left UNSET (undefined) when not given so config-file and
 *  built-in defaults can fill them in resolveSession() with a clear precedence. The
 *  config-file path is returned separately (not part of the session opts). */
export function parseArgs(argv) {
  const opts = { tenant: undefined, workspace: undefined, walDir: undefined, exe: undefined };
  let configPath;
  const rest = [];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--tenant') opts.tenant = argv[++i];
    else if (a === '--workspace') opts.workspace = argv[++i];
    else if (a === '--wal-dir') opts.walDir = argv[++i];
    else if (a === '--exe') opts.exe = argv[++i];
    else if (a === '--config') configPath = argv[++i];
    else if (a === '-h' || a === '--help') rest.push('help');
    else rest.push(a);
  }
  return { opts, rest, configPath };
}

/** Build an assistant, attach the stub reasoner, recover memory from the WAL (if durable),
 *  register the example skill library, and seed the default spend policy on a VIRGIN shard.
 *  Returns a ready assistant — the caller MUST close() it. Exposed for tests. */
export async function openSession(opts) {
  validateReasonerChoice(opts.reasoner); // honest: only the in-repo StubReasoner is selectable
  const assistant = new Assistant({ tenant: opts.tenant, workspace: opts.workspace, walDir: opts.walDir, exe: opts.exe });
  assistant.useReasoner(new StubReasoner());
  try {
    if (opts.walDir !== undefined) await assistant.recoverFromWal(); // RCR-033: memory from truth
    await registerExampleSkills(assistant);                          // certified + bound (idempotent)
    // Truth-derived seeding: the default policy is committed ONCE, only when the shard's
    // recovered policy set is empty (a virgin WAL, or an in-memory session). We do NOT
    // re-inject it every session — so once policy state is established as truth (including,
    // if a removal command is ever added, a shard the user cleared), openSession respects
    // that committed state rather than resurrecting the default from process code.
    if (assistant.guardrails.policies().length === 0) {
      await assistant.guardrails.setPolicy({ name: DEFAULT_POLICY, appliesTo: ['spend', 'irreversible'], approverRole: 'user' });
    }
  } catch (e) {
    assistant.close();
    throw e;
  }
  return assistant;
}

/** Execute ONE command against a ready assistant. Returns `{ lines, ok }` — never throws
 *  (errors become an `error: ...` line + ok:false) so the REPL survives a bad command. */
export async function runCommand(assistant, tokens, opts = {}) {
  const [cmd, ...rest] = tokens;
  const lines = [];
  const say = (...xs) => lines.push(xs.join(' '));
  try {
    switch (cmd) {
      case undefined:
      case 'help': help(say); break;
      case 'status': statusCmd(assistant, opts, say); break;
      case 'observe': await observeCmd(assistant, rest, say); break;
      case 'import': await importCmd(assistant, rest, say); break;
      case 'recall': recallCmd(assistant, rest, say); break;
      case 'ask': await askCmd(assistant, rest, say); break;
      case 'why': whyCmd(assistant, rest, say); break;
      case 'report': reportCmd(assistant, rest, say); break;
      case 'export': reportCmd(assistant, rest, say); break;
      case 'config': configCmd(rest, opts, say); break;
      case 'approve': await approveCmd(assistant, rest, say); break;
      case 'policy': await policyCmd(assistant, rest, say); break;
      case 'skills': say('skills:', assistant.skills().join(', ') || '(none)'); break;
      case 'decisions': decisionsCmd(assistant, say); break;
      default: say(`unknown command '${cmd}' — try 'help'`); return { lines, ok: false };
    }
    return { lines, ok: true };
  } catch (e) {
    say(`error: ${e.message}`);
    return { lines, ok: false };
  }
}

// ---- individual commands --------------------------------------------------------------

function help(say) {
  const rows = [
    ['observe <source> <entity> <event> <iso-utc>', 'record one fact (source is evidence, not identity)'],
    ['import <connector> [file]', `bulk-observe via a connector: ${Object.keys(CONNECTORS).sort().join('|')}`],
    ['ask <goal>', 'think: reasoner proposal -> guardrail gate -> certified skill -> truth'],
    ['recall [entity]', 'list remembered truths (RCR-033 WAL scan on a fresh process)'],
    ['why <subject|id>', 'reconstruct a decision path from committed truth'],
    ['report [json]', 'export the day from committed truth (text, or JSON with `report json`)'],
    ['config [show|set|unset|path] ...', 'view/edit the persistent config (tenant/workspace/wal-dir/…)'],
    ['approve <role> <subject>', 'commit a separate approval truth (unlocks a gated action)'],
    ['policy <name> <role> <class...>', 'publish a guardrail policy as truth'],
    ['skills', 'list registered (certified + bound) skills'],
    ['decisions', 'list standing decisions'],
    ['status', 'show shard, wal-dir, reasoner, counts'],
    ['help', 'this list'],
    ['exit | quit', '(REPL) leave'],
  ];
  const w = Math.max(...rows.map((r) => r[0].length));
  say("JARVIS commands (honest: deterministic stub reasoner, offline; plug an LLM per the quickstart):");
  for (const [c, d] of rows) say(`  ${c.padEnd(w)}  ${d}`);
  say("");
  say("  note: REPL arguments are whitespace-split — a single argument cannot contain a space.");
  say("        For a space-bearing subject/name, use one-shot argv mode (jarvis <cmd> <args...>),");
  say("        which preserves each argument verbatim.");
}

function statusCmd(a, opts, say) {
  say('JARVIS — ARVES Assistant (G1 preview; stub reasoner = NOT AI; single host, no authN — v1.0 scope)');
  say(`  shard:     ${opts.tenant ?? 'you'}/${opts.workspace ?? 'jarvis'}`);
  say(`  wal-dir:   ${opts.walDir ?? '(in-memory — ephemeral; pass --wal-dir <path> for durable memory)'}`);
  say(`  reasoner:  ${a.reasoner ? `${a.reasoner.name}@${a.reasoner.version}` : '(none)'}`);
  if (opts.configPath !== undefined) say(`  config:    ${opts.configPath}`);
  say(`  truths:    ${a.truths().length}`);
  say(`  decisions: ${a.decisions().length}`);
  say(`  skills:    ${a.skills().join(', ') || '(none)'}`);
  say(`  policies:  ${a.guardrails.policies().map((p) => p.name).join(', ') || '(none)'}`);
}

async function observeCmd(a, rest, say) {
  if (rest.length < 4) throw new Error('usage: observe <source> <entity> <event> <iso-utc>   e.g. observe email urn:you dentist-appointment 2026-07-06T09:00:00Z');
  const [source, entity, event, iso] = rest;
  if (!ISO_UTC.test(iso)) throw new Error(`timestamp must be an ISO-8601 UTC instant ending in Z, got '${iso}'`);
  const ms = Date.parse(iso);
  if (!Number.isFinite(ms)) throw new Error(`unparseable instant '${iso}'`);
  const r = await a.observe(source, { entity, event, at: BigInt(ms) });
  say(`observed ${short(r.id)}  ${entity} :: ${event}  [${r.sources.join(', ')}]  (${r.status}${r.deduped ? ', merged into existing truth' : ''})`);
}

async function importCmd(a, rest, say) {
  if (rest.length < 1) throw new Error(`usage: import <connector> [file]   connectors: ${Object.keys(CONNECTORS).sort().join(', ')}`);
  const [name, file] = rest;
  const conn = connectorByName(name);
  const obs = file === undefined ? conn() : conn(file);
  let fresh = 0; let merged = 0;
  for (const { source, fact } of obs) {
    const r = await a.observe(source, fact);
    if (r.deduped) merged++; else fresh++;
  }
  say(`imported ${obs.length} observation(s) via '${name}'${file ? ` from ${file}` : ' (fixture)'} -> ${fresh} new truth(s), ${merged} merged`);
}

function recallCmd(a, rest, say) {
  const entity = rest[0];
  const truths = a.recall(entity);
  if (truths.length === 0) {
    say(entity ? `(no truths for entity '${entity}')` : '(memory empty — observe/import, or run over a --wal-dir that has prior truth)');
    return;
  }
  say(`recall${entity ? ` [${entity}]` : ''}: ${truths.length} truth(s)  (read projection of committed truth; a fresh process rebuilds this from the RCR-033 WAL scan)`);
  for (const t of truths) say(`  ${short(t.id)}  ${t.fact.entity} :: ${t.fact.event}  [${t.sources.join(', ')}]`);
}

async function askCmd(a, rest, say) {
  const goal = rest.join(' ').trim();
  if (goal === '') throw new Error('usage: ask <goal>   e.g. ask summarize my day');
  const r = await a.think(goal);
  if (r.acted === true) {
    say(`ACTED  '${goal}'`);
    say(`  proposal ${short(r.proposalId)} -> skill '${r.proposal.skill}' (class '${r.proposal.actionClass}')  subject '${r.proposal.subject}'`);
    for (const t of r.invocation.truths) say(`  committed effect ${short(t.id)} '${t.target}'  (${t.status})`);
    if (r.approvals && r.approvals.length > 0) say(`  citing approval(s): ${r.approvals.map(short).join(', ')}`);
    say(`  explain:  why ${r.proposal.subject}`);
    return;
  }
  if (r.blocked === true) {
    const role = /committed '([^']+)' approval/.exec(r.rule ?? '')?.[1] ?? '<role>';
    say(`BLOCKED  '${goal}'`);
    say(`  proposal ${short(r.proposalId)} -> skill '${r.proposal.skill}' (class '${r.proposal.actionClass}')  subject '${r.proposal.subject}'`);
    say(`  policy ${short(r.policyId)} '${r.policy}': ${r.rule}`);
    say(`  block committed as compliance truth ${short(r.complianceId)}`);
    say(`  to unlock:  approve ${role} ${r.proposal.subject}`);
    return;
  }
  if (r.failed === true && r.stage === 'proposal-rejected') {
    // The proposal was rejected BEFORE the gate ran and before any skill executed (typically
    // a real LLM reasoner supplied non-canonical input). Do NOT claim a gate or a skill ran.
    say(`REJECTED  '${goal}'`);
    say(`  reasoner '${r.proposal.reasoner ?? '?'}' proposed skill '${r.proposal.skill}' but its input was invalid: ${r.error}`);
    say(`  rejected before the guardrail; no skill executed`);
    say(`  rejection committed as compliance truth ${short(r.complianceId)}`);
    return;
  }
  if (r.failed === true) {
    // stage 'skill-execution': the gate PASSED and the certified skill RAN, but its execute()
    // threw. Governed, not a crash: committed as truth, no effect committed.
    say(`FAILED  '${goal}'`);
    say(`  proposal ${short(r.proposalId)} -> skill '${r.proposal.skill}'  subject '${r.proposal.subject}'  (guardrail passed)`);
    say(`  skill execution failed: ${r.error}`);
    say(`  failure committed as compliance truth ${short(r.complianceId)}  (explain:  why ${r.proposal.subject})`);
    return;
  }
  // no-action-proposed: the stub reasoner has no rule for this goal (honest, not a guess)
  say(`NO ACTION  '${goal}'`);
  say(`  ${r.proposal.because}`);
  say(`  (proposal ${short(r.proposalId)} committed as truth; plug an LLM reasoner for open-ended goals — see the quickstart)`);
}

function whyCmd(a, rest, say) {
  const s = rest.join(' ').trim();
  if (s === '') throw new Error('usage: why <subject|truth-id>   e.g. why spend:order-flowers');
  for (const l of renderWhy(why(a, s)).split('\n')) say(l);
}

async function approveCmd(a, rest, say) {
  const [role, subject] = rest;
  if (!role || !subject) throw new Error('usage: approve <role> <subject>   e.g. approve user spend:order-flowers');
  const r = await a.guardrails.approve(role, subject);
  say(`approved '${role}' for '${subject}'  (${short(r.id)}, ${r.status}) — a SEPARATE committed approval truth`);
}

async function policyCmd(a, rest, say) {
  const [name, role, ...classes] = rest;
  if (!name || !role || classes.length === 0) throw new Error('usage: policy <name> <approverRole> <class...>   e.g. policy spend-guard user spend irreversible');
  const r = await a.guardrails.setPolicy({ name, appliesTo: classes, approverRole: role });
  say(`policy '${name}' set: [${classes.join(', ')}] require '${role}' approval  (${short(r.id)}, ${r.status})`);
}

function decisionsCmd(a, say) {
  const ds = a.decisions();
  if (ds.length === 0) { say('(no standing decisions)'); return; }
  for (const d of ds) say(`  ${d.subject} -> ${d.action}  (${d.because})`);
}

function reportCmd(a, rest, say) {
  const r = reportDay(a);
  if (rest[0] === 'json') { for (const l of JSON.stringify(r, null, 2).split('\n')) say(l); return; }
  for (const l of renderReport(r).split('\n')) say(l);
}

/** View or edit the persistent config. `config` / `config show` need no file (they show
 *  the effective session); `set`/`unset`/`path` require a config path (--config or the bin). */
function configCmd(rest, opts, say) {
  const file = opts.configPath;
  const [sub, key, ...restVals] = rest;
  if (sub === undefined || sub === 'show') {
    say(`config file: ${file ?? '(none — pass --config <path>, or run via the bin which defaults to ~/.jarvisrc.json)'}`);
    say('effective session (CLI flag > config file > default):');
    for (const k of CONFIG_KEYS) say(`  ${k.padEnd(10)} ${opts[k] ?? '(unset)'}`);
    if (file !== undefined) {
      const cfg = loadConfig(file);
      const keys = Object.keys(cfg);
      say(keys.length > 0 ? `config file has: ${keys.map((k) => `${k}=${cfg[k]}`).join(', ')}` : 'config file is empty (or absent)');
    }
    return;
  }
  if (file === undefined) throw new Error('config set/unset/path needs a config path — pass --config <path> (the bin defaults to ~/.jarvisrc.json)');
  if (sub === 'path') { say(file); return; }
  if (sub === 'set') {
    if (!CONFIG_KEYS.includes(key)) throw new Error(`config: unknown key '${key}' — known: ${CONFIG_KEYS.join(', ')}`);
    const value = restVals.join(' ');
    if (value === '') throw new Error(`usage: config set <key> <value>   keys: ${CONFIG_KEYS.join(', ')}`);
    if (key === 'reasoner') validateReasonerChoice(value); // honest: only 'stub' ships in-repo
    const cfg = loadConfig(file); cfg[key] = value; saveConfig(file, cfg);
    say(`set ${key} = ${value}  (${file}) — takes effect next session`);
    return;
  }
  if (sub === 'unset') {
    if (!CONFIG_KEYS.includes(key)) throw new Error(`config: unknown key '${key}' — known: ${CONFIG_KEYS.join(', ')}`);
    const cfg = loadConfig(file); delete cfg[key]; saveConfig(file, cfg);
    say(`unset ${key}  (${file})`);
    return;
  }
  throw new Error(`config: unknown subcommand '${sub}' — try: config show | config set <key> <value> | config unset <key> | config path`);
}

// ---- entry points ---------------------------------------------------------------------

async function repl(assistant, opts) {
  const interactive = process.stdin.isTTY === true;
  const rl = readline.createInterface({ input: process.stdin, terminal: false });
  const prompt = () => { if (interactive) process.stdout.write('jarvis> '); };
  if (interactive) console.log("JARVIS REPL — type 'help', or 'exit' to quit.");
  prompt();
  for await (const raw of rl) {
    const line = raw.trim();
    if (line === '' || line.startsWith('#')) { prompt(); continue; }
    if (line === 'exit' || line === 'quit') break;
    const { lines } = await runCommand(assistant, line.split(/\s+/), opts);
    for (const l of lines) console.log(l);
    prompt();
  }
  rl.close();
  return 0;
}

/** CLI entry. Returns a process exit code. */
export async function main(argv = []) {
  const { opts, rest, configPath: cliConfigPath } = parseArgs(argv);
  // Config precedence (CLI flag > config file > default). An explicit --config that is
  // malformed fails loud; a malformed DEFAULT config only warns and is ignored, so the
  // bin never becomes unstartable because of a stray file in the home directory.
  const configPath = cliConfigPath ?? defaultConfigPath();
  let cfg;
  try { cfg = loadConfig(configPath); }
  catch (e) {
    if (cliConfigPath !== undefined) { console.error(`jarvis: ${e.message}`); return 1; }
    console.error(`jarvis: ignoring ${configPath} — ${e.message}`);
    cfg = {};
  }
  let session;
  try { session = resolveSession(opts, cfg); }
  catch (e) { console.error(`jarvis: ${e.message}`); return 1; }
  session.configPath = configPath;

  let assistant;
  try {
    assistant = await openSession(session);
  } catch (e) {
    console.error(`jarvis: could not start — ${e.message}`);
    return 1;
  }
  try {
    if (rest.length > 0) {
      const { lines, ok } = await runCommand(assistant, rest, session);
      for (const l of lines) console.log(l);
      return ok ? 0 : 1;
    }
    return await repl(assistant, session);
  } finally {
    assistant.close();
  }
}
