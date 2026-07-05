// Test fixture — a FAKE bridge that deliberately answers the first two requests in
// REVERSE arrival order (echoing each request's `id=` token, like the real RCR-011
// server). Used by products/robustness.test.mjs to prove the client's id-correlation:
// with the old positional-FIFO matching, caller 1 would silently receive caller 2's
// response. Response index = ARRIVAL order (1-based), so the swap is observable.
// Not part of any product; test-only.
import { createInterface } from 'node:readline';

const CID = 'ab'.repeat(34); // well-formed 34-byte multihash hex (passes client shape checks)
const rl = createInterface({ input: process.stdin });
let n = 0;
const held = [];
rl.on('line', (line) => {
  n += 1;
  const m = /^id=(\S+)\s+/.exec(line);
  const resp = `${m ? m[1] + ' ' : ''}${CID} committed ${n}`;
  if (n <= 2) {
    held.push(resp);
    if (n === 2) process.stdout.write(held[1] + '\n' + held[0] + '\n'); // REVERSED
  } else {
    process.stdout.write(resp + '\n');
  }
});
