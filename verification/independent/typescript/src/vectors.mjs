// Reconstruction of every positive golden-vector body from the ACS prose.
//
// The conformance procedure (CONFORMANCE.md "Encoder conformance") requires us
// to build the logical value described normatively in each ACS-00x spec and
// encode it — NOT to copy body_hex. For ACS-001/ACS-005 the body is raw bytes
// given directly (no encoding step). For ACS-002/003/004 we build the ARVES
// value and encode it with our dCBOR encoder.

import { NULL, Bool, Int, Float, Text, Bytes, Arr, Map_ } from './value.mjs';
import { encode } from './encode.mjs';
import { fromHex, DOMAIN } from './contentid.mjs';

const enc = (s) => new TextEncoder().encode(s);

// --- ACS-001: raw-byte bodies (§7) ---------------------------------------
export function acs001Bodies() {
  return {
    'hello-truth': enc('hello-truth'),
    'engine-manifest': enc('{"engine":"summarize","version":"1"}'),
    // NOTE: the pipe '|' is a literal char in the invocation idempotency key.
    invocation: enc('acme/research|c1|hello-truth'),
  };
}

// --- ACS-002 structured values (§8.1) -------------------------------------
// V1 uci.fact — author key order deliberately scrambled to prove §5.6 sorting.
export function acs002_V1() {
  return Map_([
    [Text('confidence'), Float(0.5)],
    [Text('observed_at'), Int(1730000000000000000n)],
    [Text('type'), Text('uci.fact')],
    [Text('claim'), Text('sky-is-blue')],
  ]);
}

// V2 engine-manifest — order scrambled; seed:Null present (not absent, §5.7);
// reads is an Array with order preserved (uci.observation before uci.fact).
export function acs002_V2() {
  return Map_([
    [Text('engine'), Text('summarize')],
    [Text('version'), Int(1n)],
    [Text('deterministic'), Bool(true)],
    [Text('reads'), Arr([Text('uci.observation'), Text('uci.fact')])],
    [Text('seed'), NULL],
  ]);
}

// V3 decision-trace — label supplied in NFD (decomposed) form to prove NFC
// normalization: "Amelie" with e+U+0301, space, e+U+0301, em-dash, 中.
// The canonical body MUST carry precomposed é (U+00E9 = c3a9).
export function acs002_V3() {
  const nfd =
    'A' + 'm' + 'e' + '́' + 'l' + 'i' + 'e' + ' ' + 'e' + '́' + '—' + '中';
  return Map_([
    [Text('n'), Int(-1000n)],
    [Text('label'), Text(nfd)],
  ]);
}

// --- ACS-003 canonical envelope (§10.2) -----------------------------------
export function acs003Envelope() {
  const payloadCid = fromHex(
    '12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e'
  );
  return Map_([
    [Text('ser_version'), Text('ACS-002/1')],
    [Text('event_id'), Text('urn:arves:evt:01J8ZK9M4Q2N7C3F')],
    [Text('event_type'), Text('information.fact.committed')],
    [Text('tenant_id'), Text('acme')],
    [Text('workspace_id'), Text('research')],
    [Text('correlation_id'), Text('urn:arves:corr:c1')],
    [Text('causation_id'), NULL], // present with Null — root event (§5.7)
    [Text('source'), Text('urn:arves:svc:information-core')],
    [Text('occurred_at'), Int(1730000000000000000n)],
    [Text('schema_version'), Int(1n)],
    [Text('payload_domain'), Int(1n)],
    [Text('payload_cid'), Bytes(payloadCid)],
  ]);
}

// --- ACS-004 schema document (§11.2) --------------------------------------
function fieldDesc(type, card) {
  return Map_([
    [Text('type'), Text(type)],
    [Text('card'), Text(card)],
  ]);
}

export function acs004Schema() {
  return Map_([
    [Text('urn'), Text('uci.fact')],
    [
      Text('ver'),
      Map_([
        [Text('major'), Int(1n)],
        [Text('minor'), Int(0n)],
      ]),
    ],
    [Text('root'), Text('Fact')],
    [
      Text('aspects'),
      Arr([
        Text('Identity'),
        Text('Provenance'),
        Text('Temporal'),
        Text('Trust'),
        Text('TenantScope'),
      ]),
    ],
    [
      Text('fields'),
      Map_([
        [Text('urn'), fieldDesc('urn', '1')],
        [Text('tenant'), fieldDesc('text', '1')],
        [Text('workspace'), fieldDesc('text', '1')],
        [Text('origin'), fieldDesc('text', '1')],
        [Text('source'), fieldDesc('text', '1')],
        [Text('invocation'), fieldDesc('urn', '0..1')],
        [Text('confidence'), fieldDesc('conf', '1')],
        [Text('valid_from'), fieldDesc('int', '1')],
        [Text('recorded_at'), fieldDesc('int', '1')],
        [Text('claim'), fieldDesc('text', '1')],
        [Text('observed_at'), fieldDesc('int', '1')],
        [Text('evidence'), fieldDesc('urn', '0..*')],
      ]),
    ],
  ]);
}

// --- ACS-004 instance (§11.3) — observed fact, invocation absent -----------
export function acs004Instance() {
  return Map_([
    [Text('urn'), Text('urn:arves:uci.core:fact@1.0:f-1730000000')],
    [Text('tenant'), Text('acme')],
    [Text('workspace'), Text('research')],
    [Text('origin'), Text('observed')],
    [Text('source'), Text('sensor-array-7')],
    [Text('confidence'), Float(0.98)],
    [Text('valid_from'), Int(1730000000000000000n)],
    [Text('recorded_at'), Int(1730000000500000000n)],
    [Text('claim'), Text('sky-is-blue')],
    [Text('observed_at'), Int(1730000000000000000n)],
    [Text('evidence'), Arr([Text('urn:arves:uci.core:evidence@1.0:e-42')])],
  ]);
}

// --- ACS-005: raw-byte bodies (§8, §9) ------------------------------------
export function acs005Bodies() {
  // Term-set: GL-001 .. GL-014, LF-joined, sorted, no trailing LF.
  const termIds = [];
  for (let i = 1; i <= 14; i++) termIds.push('GL-' + String(i).padStart(3, '0'));
  const termSet = termIds.join('\n');

  // Requirement clause (§9.2 authoritative wording, NOT the §6.1 paraphrase).
  const requirement =
    'ORCH-001-R1: The Control Plane MUST NOT own cognitive truth; only the Kernel MAY own cognitive truth.';

  // Term-name list (§9.1), LF-joined, ascending sorted, no trailing LF.
  const termNames = [
    'Capability',
    'Cognitive Entity',
    'Cognitive Truth',
    'Commit',
    'Conformance',
    'Content Address',
    'Control Plane',
    'Data Plane',
    'Decision Trace',
    'Engine',
    'Kernel',
    'Replay',
    'Shard',
    'Tenant',
  ].join('\n');

  return {
    'term-set': enc(termSet),
    requirement: enc(requirement),
    'term-names': enc(termNames),
  };
}
