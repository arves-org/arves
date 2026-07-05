// ARVES SDK — Kernel bridge client.
//
// This is the product-side of the SDK<->Kernel seam. It talks to the `arves-bridge`
// server (the platform's real reference Kernel) over the line protocol, sending an
// ACS-002 canonical body and receiving the ACS-001 ContentId the Kernel committed it
// under. Because both sides address by ACS-001, the id the SDK computes locally and the
// id the Kernel returns are identical — one world. This client modifies no platform
// file (IDR-006); it only *invokes* the platform bridge.

import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { encode, DOMAIN, hex } from './codec.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_EXE = path.resolve(
  HERE, '../../../runtime/target/debug/arves-bridge' + (process.platform === 'win32' ? '.exe' : ''),
);

export class KernelBridge {
  #proc; #waiters = []; #byId = new Map(); #buf = ''; #dead = null; #timeoutMs; #nextId = 1; #shard = '';

  // RCR-014: `{ tenant, workspace }` (both together) scope EVERY request from this
  // client to that shard via the additive `shard=<tenant>/<workspace>` protocol token —
  // one bridge process can serve many tenants, each with its own truth (SHARD-001).
  // Absent (the default), no token is sent and the bridge uses its default shard,
  // byte-identical to before.
  // RCR-015: `{ walDir }` appends `--wal-dir <walDir>` to the bridge args, so the
  // spawned bridge hosts the DURABLE file-backed Kernel over that directory: truth
  // survives the process, and a new client over the same walDir sees the same
  // ContentIds as already-committed. HONEST SCOPE: single-host durability (CRC32 +
  // RCR-002 hash-chain integrity; no authN — v2.0 debt #8). Absent → in-memory
  // Kernel, exactly as before.
  constructor(exe = DEFAULT_EXE, { timeoutMs = 10000, args = [], tenant, workspace, walDir } = {}) {
    this.#timeoutMs = timeoutMs;
    if (walDir !== undefined) {
      if (typeof walDir !== 'string' || walDir.length === 0) {
        throw new Error(`ARVES: invalid walDir ${JSON.stringify(walDir)} (must be a non-empty path string)`);
      }
      args = [...args, '--wal-dir', walDir];
    }
    if ((tenant === undefined) !== (workspace === undefined)) {
      throw new Error('ARVES: tenant and workspace must be provided together');
    }
    if (tenant !== undefined) {
      for (const [name, v] of [['tenant', tenant], ['workspace', workspace]]) {
        // Each part is interpolated into a protocol token: it must be a non-empty,
        // whitespace- and '/'-free string of at most 64 bytes (the bridge's grammar).
        if (typeof v !== 'string' || v.length === 0 || /[\s/]/.test(v) || Buffer.byteLength(v) > 64) {
          throw new Error(`ARVES: invalid ${name} ${JSON.stringify(v)} (non-empty, no whitespace or '/', <= 64 bytes)`);
        }
      }
      this.#shard = `shard=${tenant}/${workspace} `;
    }
    this.#proc = spawn(exe, args, { stdio: ['pipe', 'pipe', 'inherit'] });
    this.#proc.stdout.setEncoding('utf8');
    this.#proc.stdout.on('data', (d) => this.#onData(d));
    // Any way the child can die must settle pending requests, never hang them.
    // (A missing/unbuilt exe emits 'error'; a crash emits 'exit'/'close'; a broken
    // pipe emits stdin 'error' — an unhandled 'error' would otherwise crash the process.)
    const die = (why) => this.#fail(new Error(`arves-bridge unavailable: ${why}`));
    this.#proc.on('error', (e) => die(`spawn/child error: ${e.message}`));
    this.#proc.on('exit', (code, sig) => die(`process exited (code=${code}, signal=${sig})`));
    this.#proc.stdin.on('error', (e) => die(`stdin error: ${e.message}`));
  }

  // RCR-011: responses are matched BY REQUEST ID, not by position. Every request is
  // sent with an `id=r<N>` prefix the bridge echoes back as the response's first token,
  // so a dropped, injected, or reordered line can no longer shift every later response
  // onto the wrong caller. Responses WITHOUT a known id (e.g. the bridge's pre-parse
  // `ERR too-large`, or an older bridge binary that predates the extension) fall back
  // to the oldest pending waiter — exactly the pre-RCR-011 behaviour.
  #onData(d) {
    this.#buf += d;
    let i;
    while ((i = this.#buf.indexOf('\n')) >= 0) {
      const line = this.#buf.slice(0, i).trim();
      this.#buf = this.#buf.slice(i + 1);
      const sp = line.indexOf(' ');
      const first = sp >= 0 ? line.slice(0, sp) : line;
      let w;
      let payload = line;
      if (this.#byId.has(first)) {
        w = this.#byId.get(first);
        this.#byId.delete(first);
        this.#waiters.splice(this.#waiters.indexOf(w), 1);
        payload = sp >= 0 ? line.slice(sp + 1) : '';
      } else {
        w = this.#waiters.shift();
        if (w) this.#byId.delete(w.id);
      }
      if (w) { clearTimeout(w.timer); w.resolve(payload); }
    }
  }

  // The child died (or failed to start): reject every pending waiter and mark the
  // bridge dead so all future calls reject immediately instead of enqueuing forever.
  #fail(err) {
    if (this.#dead) return;
    this.#dead = err;
    const pending = this.#waiters.splice(0);
    this.#byId.clear();
    for (const w of pending) { clearTimeout(w.timer); w.reject(err); }
  }

  #send(reqLine) {
    if (this.#dead) return Promise.reject(this.#dead);
    // A request line MUST be exactly one protocol line: any newline would desync the
    // stream (one request → many response lines). Reject rather than corrupt.
    if (reqLine.includes('\n')) return Promise.reject(new Error('ARVES: request contains a newline (protocol injection)'));
    return new Promise((resolve, reject) => {
      const id = `r${this.#nextId++}`;
      const timer = setTimeout(() => {
        const idx = this.#waiters.indexOf(waiter);
        if (idx >= 0) this.#waiters.splice(idx, 1);
        this.#byId.delete(id);
        reject(new Error(`arves-bridge request timed out after ${this.#timeoutMs}ms`));
      }, this.#timeoutMs);
      const waiter = { resolve, reject, timer, id };
      this.#waiters.push(waiter);
      this.#byId.set(id, waiter);
      this.#proc.stdin.write(`id=${id} ${this.#shard}${reqLine}\n`);
    });
  }

  #parse(line, kind, ctx) {
    const [contentId, status, index] = line.split(/\s+/);
    if (contentId === 'ERR') throw new Error(`bridge ${kind} refused: ${line}${ctx ? ` (${ctx})` : ''}`);
    // Defensive: a conformant response is `<68-hex-id> <status> <index>`. The id is 68 hex
    // chars = 34 bytes: the 2-byte ACS-001 multihash prefix (code + length) + a 32-byte
    // SHA-256 digest. Anything else means desync/corruption — fail loudly rather than return
    // a wrong ContentId.
    if (!/^[0-9a-f]{68}$/.test(contentId) || (status !== 'committed' && status !== 'already-committed')) {
      throw new Error(`arves-bridge malformed response: ${JSON.stringify(line)}`);
    }
    return { contentId, status, index: index === undefined ? undefined : Number(index) };
  }

  /** Commit an ARVES value as truth through the real reference Kernel. */
  async commit(value, domain = 'commit') {
    const tag = DOMAIN[domain];
    if (tag === undefined) throw new Error(`unknown domain '${domain}'`);
    const domHex = tag.toString(16).padStart(2, '0');
    return this.#parse(await this.#send(`${domHex} ${hex(encode(value))}`), 'commit');
  }

  /** Run the FULL cognitive work chain through a capability: Capability (resolve/
   *  authorize) → Engine (invoke) → Kernel (commit). Throws if the capability is
   *  unbound (execution refused). */
  async invoke(value, capability, domain = 'commit') {
    const tag = DOMAIN[domain];
    if (tag === undefined) throw new Error(`unknown domain '${domain}'`);
    // The capability is interpolated into a protocol line — it MUST be a single bare
    // token, or it could inject extra requests and desync the FIFO.
    if (typeof capability !== 'string' || /\s/.test(capability)) {
      throw new Error(`ARVES: invalid capability '${capability}' (must be a whitespace-free token)`);
    }
    const domHex = tag.toString(16).padStart(2, '0');
    return this.#parse(await this.#send(`invoke ${capability} ${domHex} ${hex(encode(value))}`), 'invoke', `capability '${capability}'`);
  }

  /** Bind a capability name in this client's shard (RCR-016), so `invoke` can run it.
   *  Idempotent: rebinding an already-bound name resolves normally. HONEST SCOPE: the
   *  bridge hosts exactly ONE engine (the reference `engine:derive.fact@1.0.0`); bind
   *  attaches NAMES to that engine identity — it does not load arbitrary engine code. */
  async bind(capability) {
    // Same interpolation rule as invoke(): one bare whitespace-free token.
    if (typeof capability !== 'string' || capability.length === 0 || /\s/.test(capability)) {
      throw new Error(`ARVES: invalid capability '${capability}' (must be a whitespace-free token)`);
    }
    const line = await this.#send(`bind ${capability}`);
    if (line.startsWith('ERR')) throw new Error(`bridge bind refused: ${line} (capability '${capability}')`);
    const [verb, name] = line.split(/\s+/);
    if (verb !== 'bound' || name !== capability) {
      throw new Error(`arves-bridge malformed bind response: ${JSON.stringify(line)}`);
    }
    return { bound: capability };
  }

  close() { try { this.#proc.stdin.end(); } catch { /* already gone */ } }
}
