// Extra self-tests beyond the TSV: ACS-004 validation (§6.5, §8) and the
// spec-pinned §11.4 derived-variant ContentId (not in the golden TSV).

import { encode } from './encode.mjs';
import { contentId, toHex, DOMAIN } from './contentid.mjs';
import { Text, Int, Float, Arr, Map_ } from './value.mjs';
import { acs004Schema, acs004Instance } from './vectors.mjs';
import { validateInstance } from './acs004.mjs';

let pass = 0, fail = 0;
function check(label, cond) {
  if (cond) { pass++; console.log(`  ok   ${label}`); }
  else { fail++; console.log(`  FAIL ${label}`); }
}

const schema = acs004Schema();
const inst = acs004Instance();

// Clause 2: the observed instance validates.
check('observed instance validates against schema', validateInstance(inst, schema).ok);

// §11.4 derived variant: origin=derived + invocation present -> distinct ContentId.
function derivedInstance() {
  return Map_([
    [Text('urn'), Text('urn:arves:uci.core:fact@1.0:f-1730000000')],
    [Text('tenant'), Text('acme')],
    [Text('workspace'), Text('research')],
    [Text('origin'), Text('derived')],
    [Text('source'), Text('sensor-array-7')],
    [Text('invocation'), Text('urn:arves:uci.core:invocation@1.0:inv-9')],
    [Text('confidence'), Float(0.98)],
    [Text('valid_from'), Int(1730000000000000000n)],
    [Text('recorded_at'), Int(1730000000500000000n)],
    [Text('claim'), Text('sky-is-blue')],
    [Text('observed_at'), Int(1730000000000000000n)],
    [Text('evidence'), Arr([Text('urn:arves:uci.core:evidence@1.0:e-42')])],
  ]);
}
const derived = derivedInstance();
check('derived instance validates', validateInstance(derived, schema).ok);
const derivedCid = toHex(contentId(DOMAIN.COMMIT_CONTENT, encode(derived)));
const EXPECT_DERIVED = '12200bc84b15220c19b853116d09314f91ecc9e8249e4f645eca8b236c94bfd96ef1';
check(`derived-variant ContentId == §11.4 (${derivedCid})`, derivedCid === EXPECT_DERIVED);

// Clause 5 negatives: derived without invocation must FAIL; observed WITH invocation must FAIL.
const derivedNoInv = Map_(derived.value.filter(([k]) => k.value !== 'invocation'));
check('derived w/o invocation -> validation FAIL', !validateInstance(derivedNoInv, schema).ok);

const observedWithInv = Map_([...inst.value, [Text('invocation'), Text('urn:arves:x')]]);
check('observed WITH invocation -> validation FAIL', !validateInstance(observedWithInv, schema).ok);

// Unknown-field rejection (§6.5.5).
const withExtra = Map_([...inst.value, [Text('surprise'), Text('x')]]);
check('unknown field -> validation FAIL', !validateInstance(withExtra, schema).ok);

// confidence > 1.0 -> FAIL (conf refinement).
const badConf = Map_(inst.value.map(([k, v]) => k.value === 'confidence' ? [k, Float(1.5)] : [k, v]));
check('confidence>1.0 -> validation FAIL', !validateInstance(badConf, schema).ok);

console.log(`\nself-test: ${pass} ok, ${fail} fail`);
process.exit(fail ? 1 : 0);
