// ARVES Conformance runner (independent TypeScript/Node runtime).
//
// Implements CONFORMANCE.md exactly:
//   (a) positive: rebuild each body from the ACS prose, assert hex(body) and
//       ContentId against acs_golden_vectors.tsv;
//   (b) negative: decode each acs_negative_vectors.tsv input and assert it is
//       REJECTED with the matching reject_reason.
// Emits the ARVES Conformance Report and a PASS/FAIL total.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

import { encode } from './encode.mjs';
import { decode, RejectError } from './decode.mjs';
import { contentId, toHex, fromHex } from './contentid.mjs';
import * as V from './vectors.mjs';
import { validateInstance } from './acs004.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const VEC_DIR = join(__dirname, '..', '..', '..', '..', 'standard', 'vectors');

function parseTsv(path) {
  const text = readFileSync(path, 'utf-8').replace(/\r\n/g, '\n');
  const lines = text.split('\n').filter((l) => l.length > 0);
  const header = lines[0].split('\t');
  return lines.slice(1).map((line) => {
    const cols = line.split('\t');
    const row = {};
    header.forEach((h, i) => (row[h] = cols[i]));
    return row;
  });
}

// Map (standard, vector) -> logical body bytes reconstructed from prose.
function bodyForVector(standard, vector) {
  if (standard === 'ACS-001') {
    return V.acs001Bodies()[vector];
  }
  if (standard === 'ACS-002') {
    if (vector.startsWith('V1')) return encode(V.acs002_V1());
    if (vector.startsWith('V2')) return encode(V.acs002_V2());
    if (vector.startsWith('V3')) return encode(V.acs002_V3());
  }
  if (standard === 'ACS-003') {
    return encode(V.acs003Envelope());
  }
  if (standard === 'ACS-004') {
    if (vector.includes('instance')) return encode(V.acs004Instance());
    if (vector.includes('schema')) return encode(V.acs004Schema());
  }
  if (standard === 'ACS-005') {
    const b = V.acs005Bodies();
    if (vector === 'term-set') return b['term-set'];
    if (vector === 'requirement') return b['requirement'];
    if (vector === 'term-names') return b['term-names'];
  }
  return undefined;
}

function runPositive() {
  const rows = parseTsv(join(VEC_DIR, 'acs_golden_vectors.tsv'));
  const byStd = {};
  const failures = [];

  for (const row of rows) {
    const { standard, vector, domain, body_hex, content_id } = row;
    byStd[standard] = byStd[standard] || { pass: 0, total: 0 };
    byStd[standard].total++;

    const domainTag = parseInt(domain, 16);
    const body = bodyForVector(standard, vector);

    let ok = true;
    let detail = '';
    if (!body) {
      ok = false;
      detail = 'no reconstruction available';
    } else {
      const gotBodyHex = toHex(body);
      if (gotBodyHex !== body_hex) {
        ok = false;
        detail = `body mismatch\n    expected ${body_hex}\n    got      ${gotBodyHex}`;
      } else {
        const cid = toHex(contentId(domainTag, body));
        if (cid !== content_id) {
          ok = false;
          detail = `ContentId mismatch\n    expected ${content_id}\n    got      ${cid}`;
        }
      }
    }

    if (ok) byStd[standard].pass++;
    else failures.push(`  [${standard} ${vector}] ${detail}`);
  }

  return { byStd, failures, total: rows.length };
}

function runNegative() {
  const rows = parseTsv(join(VEC_DIR, 'acs_negative_vectors.tsv'));
  const failures = [];
  let corePass = 0;
  let coreTotal = 0;
  let nfcPass = 0;
  let nfcTotal = 0;

  for (const row of rows) {
    const { case: caseName, tier, input_hex, reject_reason } = row;
    const input = fromHex(input_hex);
    // Core tier defers NFC; nfc tier enforces it (full conformance check).
    const enforceNfc = tier === 'nfc';

    let gotReason = null;
    try {
      decode(input, { enforceNfc });
      gotReason = '(ACCEPTED — should have been rejected)';
    } catch (e) {
      if (e instanceof RejectError) gotReason = e.reason;
      else gotReason = `(threw non-Reject error: ${e.message})`;
    }

    const ok = gotReason === reject_reason;
    if (tier === 'core') {
      coreTotal++;
      if (ok) corePass++;
    } else {
      nfcTotal++;
      if (ok) nfcPass++;
    }
    if (!ok) {
      failures.push(
        `  [${row.standard} ${caseName} (${tier})] expected reject_reason='${reject_reason}', got '${gotReason}'`
      );
    }
  }

  return { corePass, coreTotal, nfcPass, nfcTotal, failures };
}

// ACS-004-CS-1 clauses 2 & 5: instance validation against the schema document.
function runValidation() {
  const schema = V.acs004Schema();
  const inst = V.acs004Instance();
  const checks = [];
  checks.push(['observed instance validates (clause 2)', validateInstance(inst, schema).ok]);
  return checks;
}

function main() {
  const pos = runPositive();
  const neg = runNegative();
  const val = runValidation();

  const order = ['ACS-001', 'ACS-002', 'ACS-003', 'ACS-004', 'ACS-005'];

  console.log('ARVES Conformance Report — ACS layer');
  console.log('  (independent TypeScript/Node runtime; Kit-only)');
  console.log('');
  let allPositivePass = true;
  let posPass = 0;
  for (const std of order) {
    const s = pos.byStd[std];
    if (!s) continue;
    const verdict = s.pass === s.total ? 'PASS' : 'FAIL';
    if (s.pass !== s.total) allPositivePass = false;
    posPass += s.pass;
    console.log(`  ${std} ${verdict} (${s.pass}/${s.total})`);
  }
  console.log(`  ACS golden vectors: ${posPass}/${pos.total} PASS`);
  console.log('');

  // Negative (rejection) results.
  console.log('  Negative (rejection) vectors:');
  console.log(
    `  ACS-002 negative vectors: ${neg.corePass}/${neg.coreTotal} core REJECTED` +
      (neg.nfcTotal > 0 ? ` (+ ${neg.nfcPass}/${neg.nfcTotal} nfc)` : '')
  );
  console.log('');

  // ACS-004 instance validation (§6.5).
  const valOk = val.every(([, ok]) => ok);
  console.log('  ACS-004 instance validation (§6.5):');
  for (const [label, ok] of val) console.log(`    ${ok ? 'PASS' : 'FAIL'} ${label}`);
  console.log('');

  if (pos.failures.length) {
    console.log('POSITIVE FAILURES:');
    for (const f of pos.failures) console.log(f);
    console.log('');
  }
  if (neg.failures.length) {
    console.log('NEGATIVE FAILURES:');
    for (const f of neg.failures) console.log(f);
    console.log('');
  }

  const corePass = allPositivePass && neg.corePass === neg.coreTotal && valOk;
  const nfcPass = neg.nfcPass === neg.nfcTotal;

  console.log('----------------------------------------------------------------');
  console.log(`  positive: ${posPass}/${pos.total}`);
  console.log(
    `  negative: ${neg.corePass}/${neg.coreTotal} core` +
      ` + ${neg.nfcPass}/${neg.nfcTotal} nfc`
  );
  if (corePass && nfcPass) {
    console.log('VERDICT: CONFORMANT (ACS layer — full: core + nfc tier ENFORCED)');
  } else if (corePass) {
    console.log('VERDICT: CONFORMANT (ACS core; nfc-tier NOT fully passed)');
  } else {
    console.log('VERDICT: NON-CONFORMANT');
  }

  process.exit(corePass && nfcPass ? 0 : 1);
}

main();
