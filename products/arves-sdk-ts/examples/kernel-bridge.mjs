// One world, not two: a TypeScript product commits truth through the REAL Rust
// reference Kernel, and the identity the SDK computes locally is byte-identical to the
// identity the Kernel assigns — because both address by ACS-001.
//
// Products → SDK → Bridge → Kernel. Run: node examples/kernel-bridge.mjs
// (requires the platform bridge built once: cargo build -p arves-bridge --bin arves-bridge)

import { Arves } from '../src/arves.mjs';
import { KernelBridge } from '../src/bridge.mjs';

const arves = new Arves();
const bridge = new KernelBridge();

const fact = {
  type: 'uci.fact',
  claim: 'sky-is-blue',
  confidence: arves.float(0.5),
  observed_at: 1730000000000000000n,
};

console.log('ARVES SDK ↔ Kernel bridge — one identity across two languages\n');

const localId = arves.commit(fact);          // identity computed in TypeScript (SDK)
const r1 = await bridge.commit(fact);        // committed by the real Rust Kernel
const r2 = await bridge.commit(fact);        // committed again -> idempotent

console.log('  SDK local ContentId  :', localId);
console.log('  Kernel committed id  :', r1.contentId, `(${r1.status}, index ${r1.index})`);
console.log('  identical? (one world):', localId === r1.contentId);
console.log('  re-commit            :', r2.contentId, `(${r2.status})  ← ORCH-004 idempotency, keyed on the ACS address`);

bridge.close();

const ok = localId === r1.contentId && r1.status === 'committed' && r2.status === 'already-committed';
console.log(ok
  ? '\nProducts → SDK → Bridge → real Kernel: ONE identity. The SDK world and the runtime world are the same world.'
  : '\nFAIL: identities diverged or idempotency did not hold.');
process.exit(ok ? 0 : 1);
