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
  #proc; #waiters = []; #buf = ''; #dead = null; #timeoutMs;

  constructor(exe = DEFAULT_EXE, { timeoutMs = 10000 } = {}) {
    this.#timeoutMs = timeoutMs;
    this.#proc = spawn(exe, [], { stdio: ['pipe', 'pipe', 'inherit'] });
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

  #onData(d) {
    this.#buf += d;
    let i;
    while ((i = this.#buf.indexOf('\n')) >= 0) {
      const line = this.#buf.slice(0, i).trim();
      this.#buf = this.#buf.slice(i + 1);
      const w = this.#waiters.shift();
      if (w) { clearTimeout(w.timer); w.resolve(line); }
    }
  }

  // The child died (or failed to start): reject every pending waiter and mark the
  // bridge dead so all future calls reject immediately instead of enqueuing forever.
  #fail(err) {
    if (this.#dead) return;
    this.#dead = err;
    const pending = this.#waiters.splice(0);
    for (const w of pending) { clearTimeout(w.timer); w.reject(err); }
  }

  #send(reqLine) {
    if (this.#dead) return Promise.reject(this.#dead);
    // A request line MUST be exactly one protocol line: any newline would desync the
    // FIFO (one request → many response lines). Reject rather than corrupt.
    if (reqLine.includes('\n')) return Promise.reject(new Error('ARVES: request contains a newline (protocol injection)'));
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        const idx = this.#waiters.indexOf(waiter);
        if (idx >= 0) this.#waiters.splice(idx, 1);
        reject(new Error(`arves-bridge request timed out after ${this.#timeoutMs}ms`));
      }, this.#timeoutMs);
      const waiter = { resolve, reject, timer };
      this.#waiters.push(waiter);
      this.#proc.stdin.write(reqLine + '\n');
    });
  }

  #parse(line, kind, ctx) {
    const [contentId, status, index] = line.split(/\s+/);
    if (contentId === 'ERR') throw new Error(`bridge ${kind} refused: ${line}${ctx ? ` (${ctx})` : ''}`);
    // Defensive: a conformant response is `<64-hex-id> <status> <index>`. Anything else
    // means desync/corruption — fail loudly rather than return a wrong ContentId.
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

  close() { try { this.#proc.stdin.end(); } catch { /* already gone */ } }
}
