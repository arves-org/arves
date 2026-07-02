// ARVES Marketplace — the distribution loop between DIFFERENT parties.
//
// A publisher (Acme Support Inc.) authors + certifies + packages a capability and publishes
// it. A different consumer discovers it, installs it into ITS OWN host, and runs it on the
// frozen Runtime v1.0 — producing truth. The two never coordinate; the marketplace only
// accepts certified, signed artifacts and refuses uncertified/tampered/duplicate ones. This
// is the loop that makes ARVES a platform others ship on.
//
// Run: node examples/publish-install.mjs   (requires: cargo build -p arves-bridge --bin arves-bridge)

import { defineCapability, certifyCapability, packageCapability, CapabilityHost } from '../../arves-ecosystem-sdk/src/kit.mjs';
import { KernelBridge } from '../../arves-sdk-ts/src/bridge.mjs';
import { Marketplace } from '../src/marketplace.mjs';

console.log('ARVES Marketplace — publish once, install anywhere\n');

// ---- PUBLISHER: a third party ships a capability ----
const cap = defineCapability({
  name: 'ticket.triage', version: '1.0.0', produces: ['uci.fact'],
  execute: (t) => [{ target: 'uci.fact', value: { type: 'uci.fact', entity: `ticket:${t.id}`, event: `priority-${t.priority}` } }],
});
const testInputs = [{ id: 'T1', priority: 'high' }, { id: 'T2', priority: 'low' }];
const cert = certifyCapability(cap, testInputs); // the author certifies locally...
console.log('[author]    certified locally:', cert.certified);
const pkg = packageCapability(cap, testInputs); // ...and the test inputs travel in the signed artifact

const market = new Marketplace();
market.publish({ pkg, cap, publisher: 'Acme Support Inc.' }); // ...the marketplace RE-certifies
console.log('[publisher] published:', market.list().map((x) => `${x.id} by ${x.publisher}`).join(', '));

// ---- CONSUMER: a DIFFERENT org installs from the marketplace and runs it ----
const bridge = new KernelBridge();
const host = new CapabilityHost(bridge);
market.install('ticket.triage', '1.0.0', host);
const r = await host.invoke('ticket.triage', { id: 'T1', priority: 'high' });
bridge.close();
console.log('[consumer]  installed + invoked → truth', r.truths[0].id.slice(0, 18) + '…', `(${r.truths[0].status})`);

// ---- The marketplace refuses bad publishes (the gate is ENFORCED, not attested) ----
let rejUncertified = false;
let rejTampered = false;
let rejDuplicate = false;
// A non-conformant capability (undeclared effect target) is refused even though nobody can
// hand the marketplace a "certified:true" flag anymore — it re-runs certification itself.
const badCap = defineCapability({ name: 'evil.triage', version: '1.0.0', produces: ['uci.fact'],
  execute: () => [{ target: 'uci.UNDECLARED', value: { type: 'uci.fact' } }] });
const badPkg = packageCapability(badCap, [{ id: 'X' }]);
try { market.publish({ pkg: badPkg, cap: badCap, publisher: 'x' }); } catch { rejUncertified = true; }
const tampered = { ...pkg, artifact: { ...pkg.artifact, codeHash: 'deadbeef' } };
try { market.publish({ pkg: tampered, cap, publisher: 'x' }); } catch { rejTampered = true; }
try { market.publish({ pkg, cap, publisher: 'x' }); } catch { rejDuplicate = true; }

console.log('\nMarketplace integrity:');
console.log('  refused uncertified publish :', rejUncertified);
console.log('  refused tampered artifact   :', rejTampered);
console.log('  refused duplicate version   :', rejDuplicate);

const ok = market.list().length === 1 && r.truths.length === 1 && r.truths[0].status === 'committed'
  && rejUncertified && rejTampered && rejDuplicate;
console.log(ok
  ? '\nOne party published; a different party installed and ran it on the frozen runtime.\nCertified, signed, distributed — that is a platform ecosystem.'
  : '\nFAIL: a marketplace property did not hold.');
process.exit(ok ? 0 : 1);
