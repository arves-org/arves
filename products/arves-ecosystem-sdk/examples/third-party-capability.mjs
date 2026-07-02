// The ecosystem flow, end to end: a THIRD PARTY's capability is authored, certified,
// packaged (signed), installed, and invoked — its output becoming truth in the FROZEN
// Runtime v1.0. The ARVES runtime is never modified and never sees the capability's code.
//
//   Author → Certify → Package(sign) → Install → Invoke → Truth (frozen Kernel)
//
// This is the proof of platform-hood: "how much code did someone else write?"
// Run: node examples/third-party-capability.mjs   (requires: cargo build -p arves-bridge --bin arves-bridge)

import thirdParty from './invoice-ocr.capability.mjs';
import { certifyCapability, packageCapability, verifyArtifact, CapabilityHost } from '../src/kit.mjs';
import { KernelBridge } from '../../arves-sdk-ts/src/bridge.mjs';

const { capability, testInputs, source } = thirdParty;

console.log('ARVES Ecosystem — publishing a third-party capability\n');

// 1. Certify (conformance).
const cert = certifyCapability(capability, testInputs);
console.log('[certify]', capability.manifest.name, '→', cert.certified ? 'CERTIFIED' : 'REJECTED');
for (const c of cert.checks) console.log('           ', c.ok ? '✓' : '✗', c.name);

// 2. Package (content-addressed signing).
const pkg = packageCapability(capability, source);
console.log('[package] signed artifact', pkg.id.slice(0, 20) + '…', `v${pkg.version}`);
console.log('[verify]  signature verifies:', verifyArtifact(pkg));

// Tamper-evidence: mutate the packaged artifact → signature must fail.
const tampered = { ...pkg, artifact: { ...pkg.artifact, manifest: { ...pkg.artifact.manifest, name: 'evil.ocr' } } };
console.log('[tamper]  altered artifact detected:', !verifyArtifact(tampered));

// 3. Install (certification-gated) + 4. Invoke through the frozen runtime.
const bridge = new KernelBridge();
const host = new CapabilityHost(bridge);
host.install(pkg, capability, cert);
const r = await host.invoke('invoice.ocr', { vendor: 'acme', amountUsd: 1234n, date: 1751468400000 });
// Idempotent: invoking again commits the same truth (already-committed).
const r2 = await host.invoke('invoice.ocr', { vendor: 'acme', amountUsd: 1234n, date: 1751468400000 });

// An UNCERTIFIED capability must be refused installation.
let refusedUncertified = false;
try { host.install(pkg, capability, { certified: false, checks: [] }); } catch { refusedUncertified = true; }
bridge.close();

console.log('\n[invoke]  invoice.ocr → truth', r.truths[0].id.slice(0, 20) + '…', `(${r.truths[0].status})`);
console.log('[install] uncertified capability refused:', refusedUncertified);

const ok = cert.certified
  && verifyArtifact(pkg) && !verifyArtifact(tampered)
  && r.truths.length === 1 && r.truths[0].status === 'committed'
  && r2.truths[0].status === 'already-committed'
  && refusedUncertified;

console.log('\nWhat just happened:');
console.log('  • a third party wrote invoice.ocr using ONLY the Author SDK (no runtime source)');
console.log('  • it was certified, signed (content-addressed), and installed');
console.log('  • its output is truth in the frozen Kernel — the runtime did not change');
console.log(ok
  ? '\nSomeone else\'s code now runs on ARVES and produces auditable truth. That is an ecosystem.'
  : '\nFAIL: an ecosystem property did not hold.');
process.exit(ok ? 0 : 1);
