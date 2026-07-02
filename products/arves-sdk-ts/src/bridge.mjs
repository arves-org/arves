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
  #proc; #waiters = []; #buf = '';

  constructor(exe = DEFAULT_EXE) {
    this.#proc = spawn(exe, [], { stdio: ['pipe', 'pipe', 'inherit'] });
    this.#proc.stdout.setEncoding('utf8');
    this.#proc.stdout.on('data', (d) => this.#onData(d));
  }

  #onData(d) {
    this.#buf += d;
    let i;
    while ((i = this.#buf.indexOf('\n')) >= 0) {
      const line = this.#buf.slice(0, i).trim();
      this.#buf = this.#buf.slice(i + 1);
      const w = this.#waiters.shift();
      if (w) w(line);
    }
  }

  #send(reqLine) {
    return new Promise((resolve) => {
      this.#waiters.push(resolve);
      this.#proc.stdin.write(reqLine + '\n');
    });
  }

  /** Commit an ARVES value as truth through the real reference Kernel. Returns the
   *  ACS-001 ContentId the Kernel assigned + whether it was newly committed. */
  async commit(value, domain = 'commit') {
    const tag = DOMAIN[domain];
    if (tag === undefined) throw new Error(`unknown domain '${domain}'`);
    const domHex = tag.toString(16).padStart(2, '0');
    const line = await this.#send(`${domHex} ${hex(encode(value))}`);
    const [contentId, status, index] = line.split(/\s+/);
    if (contentId === 'ERR') throw new Error('bridge error: ' + line);
    return { contentId, status, index: index === undefined ? undefined : Number(index) };
  }

  close() { this.#proc.stdin.end(); }
}
