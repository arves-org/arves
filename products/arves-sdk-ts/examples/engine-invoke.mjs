// The full cognitive work chain, end to end from a product:
//   Product → SDK → Bridge → Capability (resolve/authorize) → Engine (invoke) → Kernel.
//
// A TypeScript product invokes a *capability*; the real reference runtime resolves the
// capability binding, runs the pure Engine, and commits the engine's proposed effect as
// ACS-001-addressed truth in the real Kernel — the id equal to what the SDK computes
// locally (one world). An unbound capability is refused. Run: node examples/engine-invoke.mjs
// (requires: cargo build -p arves-bridge --bin arves-bridge)

import { Arves } from '../src/arves.mjs';
import { KernelBridge } from '../src/bridge.mjs';

const arves = new Arves();
const bridge = new KernelBridge();

const fact = { type: 'uci.fact', claim: 'sky-is-blue', confidence: arves.float(0.5), observed_at: 1730000000000000000n };
const localId = arves.address(fact, 'commit');

console.log('ARVES full cognitive work chain — Capability → Engine → Kernel\n');

// Invoke the bound reference capability: resolve → engine → commit as truth.
const r1 = await bridge.invoke(fact, 'derive.fact', 'commit');
console.log('  invoke derive.fact:');
console.log('    SDK-local id     :', localId);
console.log('    Kernel truth id  :', r1.contentId, `(${r1.status}, index ${r1.index})`);
console.log('    one world?       :', localId === r1.contentId);

// Re-invoke → idempotent through the whole chain.
const r2 = await bridge.invoke(fact, 'derive.fact', 'commit');
console.log('    re-invoke        :', r2.status, '(ORCH-004 idempotency across Capability→Engine→Kernel)');

// An unbound capability is refused — the Capability layer gates execution.
let refused = false;
try { await bridge.invoke(fact, 'not.bound', 'commit'); } catch { refused = true; }
console.log('    unbound capability refused?:', refused);

bridge.close();

const ok = localId === r1.contentId && r1.status === 'committed' && r2.status === 'already-committed' && refused;
console.log(ok
  ? '\nSDK → Bridge → Capability → Engine → Kernel: the full cognitive runtime, one identity.\nProducts now run on the real cognitive work chain, not just SDK→Kernel.'
  : '\nFAIL: the chain did not behave as required.');
process.exit(ok ? 0 : 1);
