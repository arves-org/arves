// csv.source — a REAL-data connector capability, authored with ONLY the ARVES Ecosystem
// Author SDK (the runtime is never touched). It closes the "all connectors are mocked" DX
// gap: instead of a hand-built object, it ingests an actual CSV *string* (the shape a file
// read, an HTTP body, or a Kafka record would deliver) and emits one `uci.fact` effect per
// data row. Pure + deterministic: the same CSV text yields the same effects, so it certifies
// and replays byte-for-byte.
//
//   Run: node examples/csv-source.capability.mjs   (self-checks; no runtime/cargo needed)
//   Certify: node bin/arves.mjs certify examples/csv-source.capability.mjs

import { defineCapability } from '../src/kit.mjs';

// A minimal, deterministic RFC-4180-style CSV parser: handles quoted fields, escaped quotes
// (""), and commas/newlines inside quotes. Enough to ingest real spreadsheet exports without
// any dependency. Returns an array of row objects keyed by the header row.
function parseCsv(text) {
  const rows = [];
  let field = '';
  let record = [];
  let inQuotes = false;
  let i = 0;
  const s = String(text).replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const pushField = () => { record.push(field); field = ''; };
  const pushRecord = () => { pushField(); rows.push(record); record = []; };
  while (i < s.length) {
    const ch = s[i];
    if (inQuotes) {
      if (ch === '"') {
        if (s[i + 1] === '"') { field += '"'; i += 2; continue; }
        inQuotes = false; i += 1; continue;
      }
      field += ch; i += 1; continue;
    }
    if (ch === '"') { inQuotes = true; i += 1; continue; }
    if (ch === ',') { pushField(); i += 1; continue; }
    if (ch === '\n') { pushRecord(); i += 1; continue; }
    field += ch; i += 1;
  }
  // Flush the trailing field/record if the text does not end in a newline.
  if (field.length > 0 || record.length > 0) pushRecord();
  if (rows.length === 0) return [];
  const header = rows[0].map((h) => h.trim());
  return rows.slice(1)
    // Skip fully-empty trailing lines (a common export artifact).
    .filter((r) => !(r.length === 1 && r[0] === ''))
    .map((r) => {
      const obj = {};
      for (let c = 0; c < header.length; c += 1) obj[header[c]] = (r[c] ?? '').trim();
      return obj;
    });
}

export const capability = defineCapability({
  name: 'csv.source',
  version: '1.0.0',
  produces: ['uci.fact'],
  // input.csv is the raw CSV text (a real payload). Expected columns: entity,event,amount.
  // Emits one uci.fact per row. All values are ARVES values: strings for entity/event, a
  // BigInt for the integer amount (a bare JS number would be rejected by ACS-002). The row
  // index is bound into the fact so distinct rows produce distinct, content-addressed facts.
  execute: (input) => {
    const rows = parseCsv(input.csv ?? '');
    return rows.map((row, idx) => ({
      target: 'uci.fact',
      value: {
        type: 'uci.fact',
        entity: String(row.entity ?? ''),
        event: String(row.event ?? ''),
        amount: BigInt(row.amount ?? '0'),
        row: BigInt(idx),
      },
    }));
  },
});

// Representative inputs: real CSV text, including a quoted field containing a comma and an
// escaped quote, to exercise the parser. Certification runs execute() on these.
export const testInputs = [
  {
    csv: 'entity,event,amount\n' +
      'invoice:acme,created,1234\n' +
      'invoice:globex,created,99\n',
  },
  {
    // Quoted field with an embedded comma + escaped quotes — a real spreadsheet export shape.
    csv: 'entity,event,amount\n' +
      '"customer:""Big, Co""",signup,0\n' +
      'customer:small,signup,5\n',
  },
];

// The author's human-readable source note; the artifact signature content-addresses the
// actual execute() bytes + the test inputs, not this string.
export const source = 'csv.source@1.0.0 :: fact(entity, event, amount:BigInt, row:BigInt) per CSV row';

export default { capability, testInputs, source };

// ---- Self-check when run directly (no runtime, no cargo) --------------------
if (import.meta.url === (await import('node:url')).pathToFileURL(process.argv[1]).href) {
  const effects = capability.execute(testInputs[0]);
  const expectRows = 2;
  const ok = effects.length === expectRows
    && effects[0].value.entity === 'invoice:acme'
    && effects[0].value.amount === 1234n
    && effects[1].value.row === 1n;
  // The quoted/escaped case must parse the embedded comma and quote correctly.
  const quoted = capability.execute(testInputs[1]);
  const okQuoted = quoted.length === 2 && quoted[0].value.entity === 'customer:"Big, Co"';
  // Determinism: same CSV text → identical effects.
  const again = capability.execute(testInputs[0]);
  const deterministic = JSON.stringify(effects, (_k, v) => (typeof v === 'bigint' ? `B(${v})` : v))
    === JSON.stringify(again, (_k, v) => (typeof v === 'bigint' ? `B(${v})` : v));
  console.log('csv.source self-check:');
  console.log('  rows parsed        :', effects.length, ok ? 'OK' : 'FAIL');
  console.log('  quoted/comma field :', quoted[0]?.value.entity, okQuoted ? 'OK' : 'FAIL');
  console.log('  deterministic      :', deterministic ? 'OK' : 'FAIL');
  const pass = ok && okQuoted && deterministic;
  console.log(pass ? '\nPASS — real CSV ingested into uci.fact effects.' : '\nFAIL');
  process.exit(pass ? 0 : 1);
}
