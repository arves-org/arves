#!/usr/bin/env node
// JARVIS CLI launcher — thin wrapper over src/cli.mjs (the frozen Runtime v1.0 is reached
// only over the bridge; IDR-006, no runtime code linked). Usage:
//   node bin/jarvis.mjs --wal-dir ./my-wal status
//   node bin/jarvis.mjs --wal-dir ./my-wal            # interactive REPL
import { main } from '../src/cli.mjs';

main(process.argv.slice(2)).then((code) => process.exit(code)).catch((e) => {
  console.error(`jarvis: ${e.message}`);
  process.exit(1);
});
