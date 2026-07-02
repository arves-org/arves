// ACS-004 instance validation (§6.5) against a schema document (§6).
//
// A conformant validator SHALL accept an instance iff every §6.5 clause holds
// and reject it otherwise. Validation is a pure function of (instance, schema).
// This is not exercised by the TSV golden vectors (which pin bytes+ContentId),
// but it is required by ACS-004-CS-1 clauses 2 and 5, so it is implemented and
// self-tested here.

// type codes -> validator (§6.3). Carrier + refinement constraint.
function checkType(code, v) {
  switch (code) {
    case 'null':
      return v.kind === 'null';
    case 'bool':
      return v.kind === 'bool';
    case 'int':
      return v.kind === 'int' && v.value >= -(2n ** 63n) && v.value <= 2n ** 63n - 1n;
    case 'u32':
      return v.kind === 'int' && v.value >= 0n && v.value <= 2n ** 32n - 1n;
    case 'float':
      return v.kind === 'float' && Number.isFinite(v.value);
    case 'conf':
      return v.kind === 'float' && Number.isFinite(v.value) && v.value >= 0.0 && v.value <= 1.0;
    case 'text':
      return v.kind === 'text';
    case 'bytes':
      return v.kind === 'bytes';
    case 'urn':
      return v.kind === 'text' && v.value.startsWith('urn:arves:');
    default:
      return false;
  }
}

// Extract a plain JS map { fieldName: valueNode } from an ARVES Map value.
function mapEntries(mapVal) {
  const m = new Map();
  for (const [k, v] of mapVal.value) {
    if (k.kind !== 'text') throw new Error('instance keys must be Text (§6.5.1)');
    m.set(k.value, v);
  }
  return m;
}

// schema is the ARVES Map from vectors.acs004Schema(); instance an ARVES Map.
// Returns { ok: boolean, reason?: string }.
export function validateInstance(instance, schema) {
  if (instance.kind !== 'map') return { ok: false, reason: 'instance not a Map' };

  const inst = mapEntries(instance);
  const schemaMap = mapEntries(schema);
  const fieldsVal = schemaMap.get('fields');
  const fields = mapEntries(fieldsVal);

  // §6.5.5: no key absent from schema fields (closed schema).
  for (const key of inst.keys()) {
    if (!fields.has(key)) return { ok: false, reason: `unknown field '${key}'` };
  }

  for (const [fname, descNode] of fields) {
    const desc = mapEntries(descNode);
    const type = desc.get('type').value;
    const card = desc.get('card').value;
    const present = inst.has(fname);

    if ((card === '1' || card === '1..*') && !present) {
      return { ok: false, reason: `required field '${fname}' absent` };
    }
    if (!present) continue; // optional & absent -> fine

    const val = inst.get(fname);
    if (card === '1' || card === '0..1') {
      if (!checkType(type, val)) return { ok: false, reason: `field '${fname}' fails type ${type}` };
    } else {
      // 1..* or 0..*
      if (val.kind !== 'array') return { ok: false, reason: `field '${fname}' must be Array` };
      if (card === '1..*' && val.value.length < 1) {
        return { ok: false, reason: `field '${fname}' must be non-empty` };
      }
      for (const el of val.value) {
        if (!checkType(type, el)) return { ok: false, reason: `element of '${fname}' fails ${type}` };
      }
    }
  }

  // §8 provenance state machine: invocation present iff origin == "derived".
  const origin = inst.get('origin');
  const hasInvocation = inst.has('invocation');
  if (origin && origin.kind === 'text') {
    if (origin.value === 'derived' && !hasInvocation) {
      return { ok: false, reason: 'origin==derived requires invocation (§8)' };
    }
    if (origin.value !== 'derived' && hasInvocation) {
      return { ok: false, reason: 'invocation present but origin!=derived (§8)' };
    }
  }

  return { ok: true };
}
