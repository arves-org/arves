// ARVES Cloud Platform — P8 PREVIEW: a LOCAL multi-tenant HTTP gateway in front of the
// real reference runtime. This is NOT a deployed SaaS; it is the smallest honest slice
// of "hosted ARVES": any HTTP client, in any language, commits truth to the real Rust
// Kernel and gets back the ACS-001 ContentId it can verify locally (one world).
//
// PLATFORM BOUNDARY (IDR-006): this file consumes the frozen Runtime v1.0 API through
// the SDK's KernelBridge client + the `arves-bridge` binary. It modifies NO platform file.
//
// TENANCY — HONEST SCOPE:
//   The runtime Kernel natively supports tenant isolation (SHARD-001: `ShardKey
//   { tenant, workspace }`), but the SHIPPED `arves-bridge` binary pins ONE hard-coded
//   shard ("t1"/"w1") per process. This product therefore achieves tenancy by PROCESS
//   ISOLATION: each tenant gets its OWN bridge process (own Kernel, own truth store).
//   That is a product-layer workaround, recorded as an RCR candidate (see README):
//   [bridge: shard selection per request]. We do NOT edit the runtime.
//
// DURABILITY — HONEST SCOPE:
//   The shipped bridge builds its Kernel on `MemWalStore` (an in-memory WAL). Truth is
//   real Kernel truth (ACS-addressed, idempotent, ordered) but lives only as long as the
//   tenant's bridge process. NO persistence-across-restart is claimed by this preview.
//
// SECURITY — HONEST SCOPE:
//   No authN/authZ (Runtime v1.0 has no principal on commit — RUNTIME_FREEZE v2.0 debt
//   #8), no TLS, no rate limiting. The tenant path segment is an ADDRESS, not an
//   identity claim. Bind to 127.0.0.1 only.
//
// WIRE CONVENTION (BigInt-safe JSON — values cross HTTP as JSON, but ACS-002 forbids
// int/float ambiguity, and JSON numbers are unsafe beyond 2^53):
//   - ACS Integer : {"$int": "42"}     — decimal string, range [-2^64, 2^64-1]
//   - ACS Float   : {"$float": 0.5}    — JSON number (binary64)
//   - bare JSON numbers are REJECTED with a 400 naming the offending field path
//   - strings / booleans / null / arrays / objects pass through unchanged
//   - "$"-prefixed map keys are RESERVED for wrappers; any other use is a 400
//   - the map key "__proto__" is REJECTED (400) — JS assignment would silently drop it
//   (Bytes values and "__proto__" keys are not expressible over this preview's wire format.)

import http from 'node:http';
import { KernelBridge } from '../../arves-sdk-ts/src/bridge.mjs';
import { float } from '../../arves-sdk-ts/src/codec.mjs';

const TENANT_RE = /^[a-z][a-z0-9-]{0,31}$/;
const INT_RE = /^-?(0|[1-9][0-9]*)$/;
const INT_MAX = (1n << 64n) - 1n;
const INT_MIN = -(1n << 64n);
const MAX_WIRE_DEPTH = 128; // mirrors the ACS-002 §5.10 codec bound

/** Typed HTTP failure: every hygiene violation becomes a clean JSON 4xx/5xx, never a crash. */
class HttpError extends Error {
  constructor(status, code, field, message) {
    super(message);
    this.status = status;
    this.code = code;
    this.field = field;
  }
}

/** JSON wire value -> ARVES value (BigInt integers, Flt floats). Throws HttpError(400)
 *  with the exact field path on any ambiguity — explicit over implicit. */
export function fromWire(v, path = 'value', depth = 0) {
  if (depth > MAX_WIRE_DEPTH) {
    throw new HttpError(400, 'too-deep', path, `nesting exceeds MAX_DEPTH=${MAX_WIRE_DEPTH} at '${path}'`);
  }
  if (v === null || typeof v === 'boolean' || typeof v === 'string') return v;
  if (typeof v === 'number') {
    throw new HttpError(400, 'bare-number', path,
      `bare JSON number at '${path}' is ambiguous (int vs float) and unsafe beyond 2^53 — `
      + `use {"$int":"..."} or {"$float": x}`);
  }
  if (Array.isArray(v)) return v.map((x, i) => fromWire(x, `${path}[${i}]`, depth + 1));
  if (typeof v === 'object') {
    const keys = Object.keys(v);
    if (keys.length === 1 && keys[0].startsWith('$')) {
      if (keys[0] === '$int') {
        const s = v.$int;
        if (typeof s !== 'string' || !INT_RE.test(s)) {
          throw new HttpError(400, 'bad-int', path, `'${path}.$int' must be a decimal integer string`);
        }
        const b = BigInt(s);
        if (b > INT_MAX || b < INT_MIN) {
          throw new HttpError(400, 'int-out-of-range', path,
            `'${path}.$int' outside ACS-002 §4 range [-2^64, 2^64-1]`);
        }
        return b;
      }
      if (keys[0] === '$float') {
        const x = v.$float;
        if (typeof x !== 'number' || !Number.isFinite(x)) {
          throw new HttpError(400, 'bad-float', path, `'${path}.$float' must be a finite JSON number`);
        }
        return float(x);
      }
      throw new HttpError(400, 'unknown-wrapper', path,
        `unknown wrapper '${keys[0]}' at '${path}' — only $int and $float are defined`);
    }
    const out = {};
    for (const k of keys) {
      if (k.startsWith('$')) {
        throw new HttpError(400, 'reserved-key', path,
          `map key '${k}' at '${path}' — "$"-prefixed keys are reserved for wire wrappers`);
      }
      if (k === '__proto__') {
        // JSON.parse creates "__proto__" as an OWN key, but plain assignment here would
        // hit the inherited accessor and silently DROP the subtree before encoding —
        // two distinct wire bodies would collapse to one ContentId. Explicit over
        // implicit: reject rather than mangle.
        throw new HttpError(400, 'reserved-key', path,
          `map key '__proto__' at '${path}' is not expressible over this wire format`);
      }
      out[k] = fromWire(v[k], `${path}.${k}`, depth + 1);
    }
    return out;
  }
  throw new HttpError(400, 'bad-value', path, `unsupported JSON value at '${path}'`);
}

export class ArvesCloud {
  #tenants = new Map(); // name -> { bridge }
  #tenantNames;
  #server = null;
  #maxBodyBytes;
  #bridgeExe;
  #bridgeTimeoutMs;

  /**
   * @param {object} opts
   * @param {string[]} opts.tenants   allowlist, fixed at construction; names must match
   *                                  /^[a-z][a-z0-9-]{0,31}$/ — anything else on the wire is 404.
   * @param {number}  [opts.maxBodyBytes=65536]  hard JSON body cap (413 above it).
   *   COUPLING NOTE: the bridge's line protocol caps a request line at 1 MiB and hex
   *   encoding doubles the value bytes, so values whose encoding exceeds ~512 KiB are
   *   refused BY THE BRIDGE ('ERR too-large' → 413 body-too-large-for-bridge). Raising
   *   maxBodyBytes above ~512 KiB therefore moves the effective cap to the bridge.
   * @param {string}  [opts.bridgeExe]  path to arves-bridge (defaults to the repo debug build).
   * @param {number}  [opts.bridgeTimeoutMs=10000]
   */
  constructor({ tenants, maxBodyBytes = 65536, bridgeExe, bridgeTimeoutMs = 10000 } = {}) {
    if (!Array.isArray(tenants) || tenants.length === 0) {
      throw new Error('ArvesCloud: a non-empty tenant allowlist is required at construction');
    }
    for (const t of tenants) {
      if (typeof t !== 'string' || !TENANT_RE.test(t)) {
        throw new Error(`ArvesCloud: invalid tenant name '${t}' (must match ${TENANT_RE})`);
      }
      if (this.#tenants.has(t)) throw new Error(`ArvesCloud: duplicate tenant '${t}'`);
      this.#tenants.set(t, null); // bridge spawned at listen()
    }
    this.#tenantNames = [...this.#tenants.keys()];
    if (!Number.isInteger(maxBodyBytes) || maxBodyBytes <= 0) {
      throw new Error('ArvesCloud: maxBodyBytes must be a positive integer');
    }
    this.#maxBodyBytes = maxBodyBytes;
    this.#bridgeExe = bridgeExe;
    this.#bridgeTimeoutMs = bridgeTimeoutMs;
  }

  get tenants() { return [...this.#tenantNames]; }

  /** Start the gateway. Spawns ONE bridge process PER TENANT (process isolation — see
   *  the tenancy caveat at the top). Returns the bound port. Binds 127.0.0.1 only. */
  async listen(port = 0) {
    if (this.#server) throw new Error('ArvesCloud: already listening');
    const server = http.createServer((req, res) => {
      req.on('error', () => {}); // client abort must never crash the gateway
      res.on('error', () => {});
      this.#handle(req, res).catch((e) => this.#respondError(res, null, e));
    });
    // Bind FIRST, spawn bridges only after success: a failed bind (e.g. EADDRINUSE)
    // must leave NOTHING running — no leaked child processes, no 'already listening'
    // lie — so the caller can simply retry listen() on the same instance.
    await new Promise((resolve, reject) => {
      server.once('error', reject);
      server.listen(port, '127.0.0.1', resolve);
    });
    for (const name of this.#tenantNames) {
      const bridge = this.#bridgeExe
        ? new KernelBridge(this.#bridgeExe, { timeoutMs: this.#bridgeTimeoutMs })
        : new KernelBridge(undefined, { timeoutMs: this.#bridgeTimeoutMs });
      this.#tenants.set(name, { bridge });
    }
    this.#server = server;
    return server.address().port;
  }

  /** Shut down: HTTP server first, then every tenant bridge. Idempotent. */
  async close() {
    if (this.#server) {
      const s = this.#server;
      this.#server = null;
      await new Promise((resolve) => {
        s.close(() => resolve());
        s.closeAllConnections?.();
      });
    }
    for (const [name, entry] of this.#tenants) {
      if (entry?.bridge) entry.bridge.close();
      this.#tenants.set(name, null);
    }
  }

  // ---- HTTP plumbing -------------------------------------------------------

  #sendJson(res, status, obj, close = false) {
    if (res.writableEnded) return;
    const body = JSON.stringify(obj);
    res.writeHead(status, {
      'content-type': 'application/json; charset=utf-8',
      'content-length': Buffer.byteLength(body),
      ...(close ? { connection: 'close' } : {}),
    });
    res.end(body);
  }

  #respondError(res, tenant, e) {
    if (e instanceof HttpError) {
      this.#sendJson(res, e.status, {
        ...(tenant ? { tenant } : {}),
        error: { code: e.code, ...(e.field ? { field: e.field } : {}), message: e.message },
      }, e.status === 413);
      return;
    }
    // Unknown defect: clean 500, never a crash, never a stack trace on the wire.
    this.#sendJson(res, 500, {
      ...(tenant ? { tenant } : {}),
      error: { code: 'internal', message: 'internal gateway error' },
    });
  }

  /** Read the request body, enforcing the byte cap BEFORE buffering unbounded input. */
  #readBody(req) {
    const cap = this.#maxBodyBytes;
    const declared = Number(req.headers['content-length']);
    if (Number.isFinite(declared) && declared > cap) {
      req.resume(); // drain politely; response goes out with connection: close
      return Promise.reject(new HttpError(413, 'body-too-large', 'body',
        `request body ${declared} bytes exceeds the ${cap}-byte cap`));
    }
    return new Promise((resolve, reject) => {
      const chunks = [];
      let size = 0;
      let done = false;
      req.on('data', (c) => {
        if (done) return; // already over cap: discard the rest
        size += c.length;
        if (size > cap) {
          done = true;
          chunks.length = 0;
          reject(new HttpError(413, 'body-too-large', 'body',
            `request body exceeds the ${cap}-byte cap`));
          return;
        }
        chunks.push(c);
      });
      req.on('end', () => { if (!done) { done = true; resolve(Buffer.concat(chunks)); } });
      req.on('error', () => { if (!done) { done = true; reject(new HttpError(400, 'body-aborted', 'body', 'request body aborted')); } });
    });
  }

  #parseJsonObject(buf) {
    let parsed;
    try {
      parsed = JSON.parse(buf.toString('utf8'));
    } catch {
      // covers syntax errors AND pathological nesting (V8 RangeError) — clean 400 either way
      throw new HttpError(400, 'malformed-json', 'body', "request body is not valid JSON — expected an object like {\"value\": ...}");
    }
    if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new HttpError(400, 'malformed-body', 'body', 'request body must be a JSON object');
    }
    return parsed;
  }

  /** Classify a bridge/codec failure into an honest HTTP status. `action` is the route
   *  that triggered the bridge call ('commit' | 'invoke' | 'health') so a refused
   *  COMMIT is never mislabeled as a capability refusal. */
  #mapBridgeError(e, action) {
    const m = String(e && e.message || e);
    if (m.includes('refused')) {
      if (m.includes('too-large')) {
        // The bridge pre-parse rejects any request line over its 1 MiB MAX_LINE
        // ('ERR too-large'). Hex encoding doubles the body, so this is reachable only
        // when maxBodyBytes is raised above ~512 KiB (the 64 KiB default cannot hit it).
        // It is a client-sized-input fault, not a capability refusal.
        return new HttpError(413, 'body-too-large-for-bridge', 'value', m);
      }
      if (action === 'invoke') {
        // Capability-layer refusal (e.g. ERR unbound) — the request was well-formed but
        // the runtime's Capability fabric would not authorize execution.
        return new HttpError(422, 'invoke-refused', 'capability', m);
      }
      // A refused plain commit (no capability on this route) — report it as such.
      return new HttpError(422, 'commit-refused', 'value', m);
    }
    if (m.includes('ARVES:')) {
      // The SDK codec rejected the value (depth bomb, unsupported kind, ...) — client fault.
      return new HttpError(400, 'invalid-value', 'value', m);
    }
    // Bridge process dead / timed out / malformed response — gateway fault, not client fault.
    return new HttpError(503, 'bridge-unavailable', null, m);
  }

  // ---- routing -------------------------------------------------------------

  async #handle(req, res) {
    let tenant = null;
    try {
      const url = new URL(req.url, 'http://localhost');
      const seg = url.pathname.split('/').filter((s) => s.length > 0);
      if (seg.length !== 2) {
        throw new HttpError(404, 'unknown-route', null,
          'routes: POST /:tenant/commit · POST /:tenant/invoke · GET /:tenant/health');
      }
      const [name, action] = seg;
      if (!this.#tenants.has(name)) {
        // Allowlist rule: a tenant exists ONLY if it was named at construction.
        throw new HttpError(404, 'unknown-tenant', 'tenant', `unknown tenant '${name}'`);
      }
      tenant = name;
      const entry = this.#tenants.get(name);
      if (!entry) throw new HttpError(503, 'not-listening', null, 'gateway is not started');
      const { bridge } = entry;

      if (action === 'health') {
        if (req.method !== 'GET') throw new HttpError(405, 'method-not-allowed', null, 'use GET');
        // A REAL liveness probe: round-trip a fixed, deterministic fact through the
        // tenant's Kernel. Honest note: this COMMITS one probe truth into the tenant's
        // store (idempotent — every probe after the first is 'already-committed').
        let r;
        try {
          r = await bridge.commit({ type: 'cloud.health-probe' });
        } catch (e) {
          throw this.#mapBridgeError(e, 'health');
        }
        this.#sendJson(res, 200, {
          tenant, ok: true, kernel: 'live',
          probe: { contentId: r.contentId, status: r.status },
        });
        return;
      }

      if (action === 'commit' || action === 'invoke') {
        if (req.method !== 'POST') throw new HttpError(405, 'method-not-allowed', null, 'use POST');
        const body = this.#parseJsonObject(await this.#readBody(req));
        if (!('value' in body)) {
          throw new HttpError(400, 'missing-field', 'value', "missing required field 'value'");
        }
        const value = fromWire(body.value, 'value');

        if (action === 'commit') {
          let r;
          try {
            r = await bridge.commit(value);
          } catch (e) {
            throw this.#mapBridgeError(e, 'commit');
          }
          // Audit contract: tenant + ACS ContentId come back on EVERY success so the
          // caller can recompute the address locally and verify one-world identity.
          this.#sendJson(res, 200, { tenant, contentId: r.contentId, status: r.status, index: r.index });
          return;
        }

        // invoke
        const cap = body.capability;
        if (typeof cap !== 'string' || cap.length === 0 || /\s/.test(cap)) {
          throw new HttpError(400, 'bad-capability', 'capability',
            "field 'capability' must be a non-empty whitespace-free token");
        }
        let r;
        try {
          r = await bridge.invoke(value, cap);
        } catch (e) {
          throw this.#mapBridgeError(e, 'invoke');
        }
        this.#sendJson(res, 200, { tenant, capability: cap, contentId: r.contentId, status: r.status, index: r.index });
        return;
      }

      throw new HttpError(404, 'unknown-route', null,
        `unknown action '${action}' — routes: commit · invoke · health`);
    } catch (e) {
      this.#respondError(res, tenant, e);
    }
  }
}
