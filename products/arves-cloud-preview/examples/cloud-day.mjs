// ARVES Cloud Preview — a day of hosted, multi-tenant ARVES over plain HTTP.
//
// What this demo PROVES (all against the real Rust reference Kernel, via one bridge
// process per tenant):
//   1. Content addressing is UNIVERSAL: the same fact committed by two different
//      tenants over HTTP gets the SAME ACS-001 ContentId — and it equals the id the
//      SDK computes locally, with no network at all (one world).
//   2. Truth stores are ISOLATED per tenant: acme's re-commit is 'already-committed'
//      for acme, while globex's FIRST commit of the same fact is fresh ('committed')
//      — per-tenant idempotency (ORCH-004), process-isolated tenancy.
//   3. Input hygiene is hard: unknown tenant -> 404, oversized body -> 413,
//      malformed JSON -> 400 — clean JSON errors, never a crash.
//   4. The full cognitive chain is hosted too: POST /:tenant/invoke runs
//      Capability -> Engine -> Kernel; an unbound capability is refused (422).
//
// Requires: cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml
// Run:      node examples/cloud-day.mjs      (exit 0 = every property held)

import { ArvesCloud } from '../src/cloud.mjs';
import { Arves } from '../../arves-sdk-ts/src/arves.mjs';

const checks = [];
const ok = (name, cond) => { checks.push([name, !!cond]); console.log(`  ${cond ? 'PASS' : 'FAIL'}  ${name}`); };

const cloud = new ArvesCloud({ tenants: ['acme', 'globex'] });
let exitCode = 1;
try {
  const port = await cloud.listen(0); // ephemeral local port; 127.0.0.1 only
  const base = `http://127.0.0.1:${port}`;
  const post = (path, body) => fetch(base + path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: typeof body === 'string' ? body : JSON.stringify(body),
  });

  console.log('ARVES Cloud Preview — hosted multi-tenant gateway on the real Kernel\n');

  // The SAME fact, expressed in the documented BigInt-safe wire convention.
  const wireFact = {
    value: {
      type: 'uci.fact',
      claim: 'sky-is-blue',
      confidence: { $float: 0.5 },
      observed_at: { $int: '1730000000000000000' },
    },
  };
  // What the SDK computes locally (no HTTP, no Kernel) for the identical value.
  const arves = new Arves();
  const localId = arves.address({
    type: 'uci.fact', claim: 'sky-is-blue',
    confidence: arves.float(0.5), observed_at: 1730000000000000000n,
  }, 'commit');

  console.log('— one world, many tenants —');
  const a1 = await (await post('/acme/commit', wireFact)).json();
  const a2 = await (await post('/acme/commit', wireFact)).json();
  const g1 = await (await post('/globex/commit', wireFact)).json();
  console.log(`  acme   commit #1 : ${a1.contentId} (${a1.status})`);
  console.log(`  acme   commit #2 : ${a2.contentId} (${a2.status})`);
  console.log(`  globex commit #1 : ${g1.contentId} (${g1.status})`);
  console.log(`  SDK local address: ${localId}\n`);
  ok('same body -> same ContentId across tenants (content addressing is universal)',
    a1.contentId === g1.contentId && a1.contentId === a2.contentId);
  ok('HTTP ContentId equals the SDK-local address (one-world identity, verifiable offline)',
    a1.contentId === localId);
  ok("acme re-commit is 'already-committed' (per-tenant idempotency, ORCH-004)",
    a1.status === 'committed' && a2.status === 'already-committed');
  ok("globex's FIRST commit of the same fact is fresh (isolated truth stores)",
    g1.status === 'committed');
  ok('every outcome is audited with its tenant', a1.tenant === 'acme' && g1.tenant === 'globex');

  console.log('\n— hosted cognitive chain: Capability -> Engine -> Kernel —');
  const inv = await (await post('/acme/invoke', { ...wireFact, capability: 'derive.fact' })).json();
  console.log(`  acme invoke derive.fact : ${inv.contentId} (${inv.status})`);
  ok('invoke returns the same one-world id (already-committed for acme: chain-level idempotency)',
    inv.contentId === localId && inv.status === 'already-committed');
  const bad = await post('/acme/invoke', { ...wireFact, capability: 'not.bound' });
  ok('unbound capability refused with 422 (Capability layer gates execution)', bad.status === 422
    && (await bad.json()).error.code === 'invoke-refused');

  console.log('\n— input hygiene (clean errors, never a crash) —');
  const r404 = await post('/initech/commit', wireFact);
  const e404 = await r404.json();
  console.log(`  unknown tenant  -> ${r404.status} ${e404.error.code}`);
  ok('unknown tenant -> 404 (allowlist fixed at construction)',
    r404.status === 404 && e404.error.code === 'unknown-tenant' && e404.error.field === 'tenant');

  const r413 = await post('/acme/commit', JSON.stringify({ value: { blob: 'x'.repeat(70000) } }));
  const e413 = await r413.json();
  console.log(`  oversized body  -> ${r413.status} ${e413.error.code}`);
  ok('oversized body -> 413 (hard byte cap)', r413.status === 413 && e413.error.code === 'body-too-large');

  const r400 = await post('/acme/commit', '{"value": oops');
  const e400 = await r400.json();
  console.log(`  malformed JSON  -> ${r400.status} ${e400.error.code} (field: ${e400.error.field})`);
  ok('malformed JSON -> 400 naming the field', r400.status === 400
    && e400.error.code === 'malformed-json' && e400.error.field === 'body');

  const rNum = await post('/acme/commit', { value: { type: 'uci.fact', n: 42 } });
  const eNum = await rNum.json();
  console.log(`  bare number     -> ${rNum.status} ${eNum.error.code} (field: ${eNum.error.field})`);
  ok('bare JSON number -> 400 with the exact field path (BigInt-safe convention enforced)',
    rNum.status === 400 && eNum.error.code === 'bare-number' && eNum.error.field === 'value.n');

  console.log('\n— health (a REAL Kernel round-trip per tenant) —');
  const h = await (await fetch(`${base}/globex/health`)).json();
  console.log(`  globex: kernel=${h.kernel} probe=${h.probe.status}`);
  ok('health proves Kernel liveness with a deterministic probe commit', h.ok === true && h.kernel === 'live');

  const failed = checks.filter(([, c]) => !c);
  console.log(failed.length === 0
    ? `\nAll ${checks.length} properties held. Hosted ARVES: many tenants, one identity,\n`
      + 'isolated truth — every ContentId verifiable locally, no trust in the gateway needed.'
    : `\nFAIL: ${failed.length} propert${failed.length === 1 ? 'y' : 'ies'} did not hold.`);
  exitCode = failed.length === 0 ? 0 : 1;
} finally {
  await cloud.close(); // shut down cleanly: HTTP server, then every tenant bridge
}
// exitCode (not process.exit()): lets Node drain child-process/socket handles cleanly
// on Windows instead of racing libuv handle teardown.
process.exitCode = exitCode;
