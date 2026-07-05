// ARVES Cloud Preview — assert-based property tests. Plain Node, no deps, exit 0/1.
// Run: node cloud.test.mjs
// Requires the bridge binary:
//   cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml

import assert from 'node:assert/strict';
import { ArvesCloud, fromWire } from './src/cloud.mjs';
import { Arves } from '../arves-sdk-ts/src/arves.mjs';

let n = 0;
const ok = (name, cond) => { assert.ok(cond, name); n++; console.log('  ✓', name); };
const throws = (name, fn, match) => {
  let e = null;
  try { fn(); } catch (x) { e = x; }
  assert.ok(e && (!match || String(e.message).includes(match)), `${name} (expected throw${match ? ` ~ ${match}` : ''})`);
  n++; console.log('  ✓', name);
};

console.log('construction (allowlist rules):');
{
  throws('empty tenant list rejected', () => new ArvesCloud({ tenants: [] }), 'non-empty');
  throws('invalid tenant name rejected', () => new ArvesCloud({ tenants: ['Bad Name!'] }), 'invalid tenant');
  throws('duplicate tenant rejected', () => new ArvesCloud({ tenants: ['acme', 'acme'] }), 'duplicate');
  throws('uppercase tenant rejected (names are lowercase tokens)',
    () => new ArvesCloud({ tenants: ['ACME'] }), 'invalid tenant');
}

console.log('wire convention (BigInt-safe JSON -> ARVES values):');
{
  ok('$int -> BigInt', fromWire({ $int: '42' }) === 42n);
  ok('negative $int -> BigInt', fromWire({ $int: '-7' }) === -7n);
  ok('$int at the 2^64-1 boundary accepted', fromWire({ $int: '18446744073709551615' }) === (1n << 64n) - 1n);
  ok('$float -> ACS Float wrapper', fromWire({ $float: 0.5 }).v === 0.5);
  ok('strings/booleans/null pass through', fromWire('x') === 'x' && fromWire(true) === true && fromWire(null) === null);
  ok('nested structures convert recursively', fromWire({ a: [{ $int: '1' }] }).a[0] === 1n);
  throws('bare number rejected with field path', () => fromWire({ a: { b: 1 } }), "'value.a.b'");
  throws('$int above 2^64-1 rejected', () => fromWire({ $int: '18446744073709551616' }), 'range');
  throws('non-decimal $int rejected', () => fromWire({ $int: '0x2a' }), 'decimal');
  throws('$int given a JSON number rejected', () => fromWire({ $int: 42 }), 'decimal');
  throws('non-finite-capable $float payload rejected', () => fromWire({ $float: '0.5' }), 'finite');
  throws('unknown wrapper rejected', () => fromWire({ $bytes: 'ff' }), 'unknown wrapper');
  throws('reserved "$" key inside a map rejected', () => fromWire({ $int: '1', other: true }), 'reserved');
  // "__proto__" as a wire key would hit the inherited accessor and silently DROP the
  // subtree (two distinct bodies -> one ContentId). It must be rejected, never mangled.
  throws('"__proto__" map key rejected (silent-drop guard)',
    () => fromWire(JSON.parse('{"a":true,"__proto__":{"evil":true}}')), '__proto__');
  ok('Object.prototype untouched by the attempt', !('evil' in {}));
  const deep = (() => { let v = null; for (let i = 0; i < 200; i++) v = [v]; return v; })();
  throws('depth bomb rejected cleanly (MAX_DEPTH)', () => fromWire(deep), 'MAX_DEPTH');
}

console.log('gateway (against the real Kernel via per-tenant bridges):');
const cloud = new ArvesCloud({ tenants: ['acme', 'globex'], maxBodyBytes: 65536 });
try {
  const port = await cloud.listen(0);
  const base = `http://127.0.0.1:${port}`;
  const post = (path, body) => fetch(base + path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: typeof body === 'string' ? body : JSON.stringify(body),
  });

  const wireFact = {
    value: {
      type: 'uci.fact', claim: 'sky-is-blue',
      confidence: { $float: 0.5 }, observed_at: { $int: '1730000000000000000' },
    },
  };
  const arves = new Arves();
  const localId = arves.address({
    type: 'uci.fact', claim: 'sky-is-blue',
    confidence: arves.float(0.5), observed_at: 1730000000000000000n,
  }, 'commit');

  // -- universality + per-tenant isolation ----------------------------------
  const a1 = await (await post('/acme/commit', wireFact)).json();
  const a2 = await (await post('/acme/commit', wireFact)).json();
  const g1 = await (await post('/globex/commit', wireFact)).json();
  ok('same body -> same ContentId for both tenants', a1.contentId === g1.contentId);
  ok('HTTP ContentId equals SDK-local address (one world)', a1.contentId === localId);
  ok('acme first commit is fresh', a1.status === 'committed');
  ok('acme re-commit is already-committed (per-tenant idempotency)', a2.status === 'already-committed');
  ok('globex first commit of the SAME fact is fresh (isolated truth stores)', g1.status === 'committed');
  ok('responses carry the tenant for audit', a1.tenant === 'acme' && g1.tenant === 'globex');
  ok('responses carry the Kernel truth index', typeof a1.index === 'number');

  // -- hosted cognitive chain -------------------------------------------------
  const inv = await (await post('/acme/invoke', { ...wireFact, capability: 'derive.fact' })).json();
  ok('invoke through Capability->Engine->Kernel returns the one-world id', inv.contentId === localId);
  ok('invoke of already-committed truth is idempotent', inv.status === 'already-committed');
  const rRef = await post('/acme/invoke', { ...wireFact, capability: 'not.bound' });
  ok('unbound capability -> 422 invoke-refused', rRef.status === 422 && (await rRef.json()).error.code === 'invoke-refused');
  const rCap = await post('/acme/invoke', { ...wireFact, capability: 'evil cap' });
  ok('whitespace capability -> 400 (protocol injection blocked at the gateway)',
    rCap.status === 400 && (await rCap.json()).error.code === 'bad-capability');

  // -- input hygiene ----------------------------------------------------------
  const r404t = await post('/initech/commit', wireFact);
  const e404t = await r404t.json();
  ok('unknown tenant -> 404 unknown-tenant (field: tenant)',
    r404t.status === 404 && e404t.error.code === 'unknown-tenant' && e404t.error.field === 'tenant');
  const r404r = await post('/acme/nonsense', wireFact);
  ok('unknown action -> 404 unknown-route', r404r.status === 404 && (await r404r.json()).error.code === 'unknown-route');
  const r404s = await fetch(base + '/');
  ok('bare / -> 404 unknown-route', r404s.status === 404 && (await r404s.json()).error.code === 'unknown-route');
  const r405 = await fetch(base + '/acme/commit'); // GET on a POST route
  ok('wrong method -> 405', r405.status === 405 && (await r405.json()).error.code === 'method-not-allowed');
  const r405h = await post('/acme/health', {});
  ok('POST on health -> 405', r405h.status === 405);

  const r413 = await post('/acme/commit', JSON.stringify({ value: { blob: 'x'.repeat(70000) } }));
  ok('oversized body -> 413 body-too-large', r413.status === 413 && (await r413.json()).error.code === 'body-too-large');
  ok('gateway still serves AFTER an oversized request (no crash, no wedge)',
    (await (await fetch(base + '/acme/health')).json()).ok === true);

  const r400j = await post('/acme/commit', '{"value": oops');
  const e400j = await r400j.json();
  ok('malformed JSON -> 400 malformed-json (field: body)',
    r400j.status === 400 && e400j.error.code === 'malformed-json' && e400j.error.field === 'body');
  const r400a = await post('/acme/commit', '[1,2,3]');
  ok('non-object JSON body -> 400 malformed-body', r400a.status === 400 && (await r400a.json()).error.code === 'malformed-body');
  const r400m = await post('/acme/commit', {});
  const e400m = await r400m.json();
  ok("missing 'value' -> 400 missing-field (field: value)",
    r400m.status === 400 && e400m.error.code === 'missing-field' && e400m.error.field === 'value');
  const r400n = await post('/acme/commit', { value: { x: { y: 3 } } });
  const e400n = await r400n.json();
  ok('bare number -> 400 with exact field path value.x.y',
    r400n.status === 400 && e400n.error.code === 'bare-number' && e400n.error.field === 'value.x.y');
  const rDeep = await post('/acme/commit', '{"value": ' + '['.repeat(200) + 'null' + ']'.repeat(200) + '}');
  ok('deep nesting -> clean 400, not a crash', rDeep.status === 400);

  // Regression (silent value mangling): a body containing "__proto__" must NOT collapse
  // onto the ContentId of the body without it — it is rejected outright with a 400.
  const rPlain = await (await post('/acme/commit', '{"value":{"a":true}}')).json();
  const rProto = await post('/acme/commit', '{"value":{"a":true,"__proto__":{"evil":true}}}');
  const eProto = await rProto.json();
  ok('"__proto__" wire key -> 400 reserved-key (never silently dropped)',
    rProto.status === 400 && eProto.error.code === 'reserved-key');
  ok('the two distinct bodies no longer share a ContentId (second has none)',
    typeof rPlain.contentId === 'string' && eProto.contentId === undefined);

  // -- health -------------------------------------------------------------------
  const h1 = await (await fetch(base + '/globex/health')).json();
  const h2 = await (await fetch(base + '/globex/health')).json();
  ok('health round-trips the real Kernel', h1.ok === true && h1.kernel === 'live');
  ok('health probe is deterministic + idempotent (2nd probe already-committed)',
    h1.probe.contentId === h2.probe.contentId && h2.probe.status === 'already-committed');

} finally {
  await cloud.close();
}

console.log('listen() failure path (bind failure must leave NOTHING running):');
{
  // Occupy a port, then make ArvesCloud try to bind it -> EADDRINUSE.
  const http = await import('node:http');
  const blocker = http.createServer(() => {});
  await new Promise((res) => blocker.listen(0, '127.0.0.1', res));
  const busyPort = blocker.address().port;

  const c2 = new ArvesCloud({ tenants: ['acme'] });
  let bindErr = null;
  try { await c2.listen(busyPort); } catch (e) { bindErr = e; }
  ok('listen() on a busy port rejects (EADDRINUSE)', bindErr !== null);
  // Regression: the failed instance must NOT claim it is listening, must not have
  // leaked bridge child processes, and a retry on a free port must succeed.
  const p2 = await c2.listen(0);
  ok('retry listen(0) on the SAME instance succeeds after a failed bind', Number.isInteger(p2) && p2 > 0);
  const h = await (await fetch(`http://127.0.0.1:${p2}/acme/health`)).json();
  ok('retried gateway serves the real Kernel', h.ok === true && h.kernel === 'live');
  await c2.close();
  await new Promise((res) => blocker.close(res));
  // Leaked children would keep the event loop alive; the natural-exit contract at the
  // bottom of this file is itself the leak assertion.
}

console.log('bridge line-cap coupling (maxBodyBytes raised above the bridge MAX_LINE):');
{
  // With maxBodyBytes > ~512 KiB the gateway cap no longer protects the bridge's 1 MiB
  // request line (hex doubles the bytes): the bridge refuses with 'ERR too-large', and
  // the gateway must report 413 body-too-large-for-bridge on /commit — NOT a
  // capability-shaped 'invoke-refused'.
  const c3 = new ArvesCloud({ tenants: ['acme'], maxBodyBytes: 2 * 1024 * 1024 });
  try {
    const p3 = await c3.listen(0);
    const big = JSON.stringify({ value: { blob: 'x'.repeat(600_000) } }); // ~1.2 MiB hex line
    const r = await fetch(`http://127.0.0.1:${p3}/acme/commit`, {
      method: 'POST', headers: { 'content-type': 'application/json' }, body: big,
    });
    const e = await r.json();
    ok('bridge-refused oversized commit -> 413 body-too-large-for-bridge (field: value)',
      r.status === 413 && e.error.code === 'body-too-large-for-bridge' && e.error.field === 'value');
    const h = await (await fetch(`http://127.0.0.1:${p3}/acme/health`)).json();
    ok('gateway still serves after a bridge-refused commit', h.ok === true);
  } finally {
    await c3.close();
  }
}

console.log(`\nALL ${n} CHECKS PASSED`);
// natural exit (0 unless an assert threw) — process.exit() here races libuv
// child-process teardown on Windows.
process.exitCode = 0;
