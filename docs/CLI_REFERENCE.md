# ARVES CLI Reference — `arves`

The `arves` command is the **Ecosystem Authoring CLI**: the developer front door for going
from an idea to a certified, signed, publishable capability **without ever touching the frozen
ARVES runtime**. Everything on this page runs **offline** with only **Node >= 18** — no Rust
build, no network, no API keys.

- Binary: [`products/arves-ecosystem-sdk/bin/arves.mjs`](../products/arves-ecosystem-sdk/bin/arves.mjs)
- Run it as: `node products/arves-ecosystem-sdk/bin/arves.mjs <command> ...`
  (examples below use `arves <command>` as shorthand for that invocation)
- A capability file **default-exports** `{ capability, testInputs, source }`.

> **Which commands are real today?** All seven documented here (`init`, `create`, `doctor`,
> `certify`, `package`, `publish`, `install`) are implemented and were run to produce the
> outputs shown on this page. `publish`/`install` operate on a **local, file-backed registry**
> (`.arves-registry/` at the repo root) — a *hosted* registry is still on the DX backlog.

## At a glance

| Command | Purpose | Needs a file? |
|---|---|---|
| [`init`](#arves-init-name) | Scaffold a green, certifiable **data/effect** capability | writes one |
| [`create`](#arves-create-name---provider-p) | Scaffold a **reasoning** (LLM-backed) capability | writes one |
| [`doctor`](#arves-doctor-file) | Conformance assistant: every violation + its exact fix | yes |
| [`certify`](#arves-certify-file) | Certification verdict (PASS/FAIL + per-check status) | yes |
| [`package`](#arves-package-file) | Produce a signed, content-addressed artifact | yes |
| [`publish`](#arves-publish-file) | Certify + package, then store in the local registry | yes |
| [`install`](#arves-install-nameversion) | Fetch from the registry, re-verify + re-certify | no (a `name@version` key) |

The happy path is: **`init` → `doctor` → `certify` → `package` → `publish` → `install`**.

## Help

```
arves --help          # or -h, or `help`, or no arguments
arves <command> --help # same top-level help, useful as a reminder
```

Unknown commands print an error and the help text and exit non-zero.

---

## `arves init <name>`

Scaffold a working, immediately-certifiable capability file `<name>.capability.mjs` in the
current directory. The scaffold is pure and deterministic, so it certifies and replays as-is;
it emits a `uci.fact` effect from an `{ entity, event }` input. The name is sanitized to
`[A-Za-z0-9._-]`. Refuses to overwrite an existing file.

**Usage**

```
arves init <name>
```

**Example**

```
$ arves init hospital.incident
created hospital.incident.capability.mjs
next:  arves doctor hospital.incident.capability.mjs   # then certify, then package
```

---

## `arves create <name> --provider <p>`

Scaffold a **reasoning** capability — one whose output comes from a (swappable) provider.
`--provider` (or `--provider=<p>`) selects the backend; default is `reference`.

| Provider | Behaviour |
|---|---|
| `reference` (default) | Deterministic pure function — **certifies and replays offline**. |
| `local` | Deterministic — certifies and replays offline. |
| `claude` / `gpt` / `gemini` | Scaffolds an **adapter STUB** that throws until you wire an API adapter + key. It will **not** certify until integrated (no network/keys live in-repo — this is intentional). |

The doctrine baked into the scaffold (ORCH-003; ACS-005 GL-012): a provider's output is
committed **once** as content-addressed truth, and **replay reads the recorded trace — it never
re-calls the provider**. That is what turns a possibly non-deterministic LLM into a
deterministic, auditable step. It emits `uci.reasoning`.

**Usage**

```
arves create <name> --provider <reference|local|claude|gpt|gemini>
```

**Example** (deterministic, certifies offline)

```
$ arves create triage.summary --provider reference
created triage.summary.capability.mjs   (reasoning capability, provider=reference)
next:  arves doctor triage.summary.capability.mjs   # deterministic → should certify offline
```

An unknown provider is rejected: `create: unknown provider '<p>' — choose one of: reference, local, claude, gpt, gemini`.

---

## `arves doctor <file>`

The **conformance assistant**. Runs certification and, for every failed check, prints what was
found and the exact remedy in plain language. On a healthy capability it lists all passing
checks and points you at `package`.

**Usage**

```
arves doctor <file>
```

**Example** (healthy)

```
$ arves doctor examples/csv-source.capability.mjs
doctor csv.source@1.0.0: HEALTHY — all conformance checks pass.
  ✓ manifest-valid
  ✓ has-test-inputs
  ✓ effects-declared
  ✓ effects-acs-canonical
  ✓ deterministic
next:  arves package examples/csv-source.capability.mjs   # produce a signed, publishable artifact
```

When something is wrong, each `✗` line carries a `found:` and a `fix:`. Exit code is `1` while
non-conformant, `0` when healthy.

---

## `arves certify <file>`

Run certification and print the **PASS/FAIL verdict** plus the per-check status. This is the
gate: an uncertified capability must not be packaged, published, or installed. Exit code is `0`
when `CERTIFIED`, `1` when `REJECTED` (with a tip to run `doctor`).

**Usage**

```
arves certify <file>
```

**Example**

```
$ arves certify examples/csv-source.capability.mjs
certify csv.source@1.0.0: CERTIFIED
  ✓ manifest-valid
  ✓ has-test-inputs
  ✓ effects-declared
  ✓ effects-acs-canonical
  ✓ deterministic
```

The five checks: manifest validity · at least one representative test input (no vacuous pass) ·
every effect targets a declared `produces[]` · every effect value is ACS-canonical · determinism
(same input → same effect addresses).

---

## `arves package <file>`

Produce a **signed, content-addressed, versioned artifact**. The "signature" is the ACS content
address of `{ manifest, codeHash, testInputsHash }` — over the real `execute` code **and** the
test inputs — so any tamper with the manifest, code, or inputs changes the artifact id
(self-verifying; no PKI).

**Usage**

```
arves package <file>
```

**Example**

```
$ arves package examples/csv-source.capability.mjs
package csv.source@1.0.0
  artifact 122049b251859cebe57c78c93fa2f6d114e83d79dc32e067e26648d0061cf927a890
```

The artifact id is a deterministic function of the code + inputs; the same capability always
packages to the same id.

---

## `arves publish <file>`

Certify **and** package the capability, then store the artifact in the **local, file-backed
registry** at `.arves-registry/` in the repo root. The source file path travels with the record
so `install` can re-import the real code and re-run the whole trust boundary. Refuses to store
an uncertified capability, and refuses to overwrite an existing `name@version` (artifacts are
immutable — bump the version).

**Usage**

```
arves publish <file>
```

**Example**

```
$ arves publish examples/csv-source.capability.mjs
publish csv.source@1.0.0: STORED in local registry
  artifact 122049b251859cebe57c78c93fa2f6d114e83d79dc32e067e26648d0061cf927a890
  record   artifacts/csv.source@1.0.0.json
next:  arves install csv.source@1.0.0   # fetch back, re-verify + re-certify
```

> **Scope:** the registry is **local** (on-disk) today. A hosted/shared registry is a
> documented DX backlog item — see [DEVELOPER_JOURNEY_REPORT.md](../verification/dx/DEVELOPER_JOURNEY_REPORT.md).

---

## `arves install <name@version>`

Fetch an artifact **from the local registry** by its `name@version` key (not a file path) and
**re-enforce the full trust boundary on read-back**: re-import the stored source, verify the
live code hashes to the signed `codeHash`, verify the artifact signature and travelling
test-input hash, and **re-run certification**. The gate is enforced, never merely attested by a
stored flag. Prints the verified artifact id.

**Usage**

```
arves install <name@version>
```

**Example**

```
$ arves install csv.source@1.0.0
install csv.source@1.0.0: VERIFIED + RE-CERTIFIED
  artifact 122049b251859cebe57c78c93fa2f6d114e83d79dc32e067e26648d0061cf927a890
```

A tampered artifact, mismatched code, or a capability that fails re-certification is **REFUSED**
(exit code `1`) with the reason. A missing key lists the keys that are available.

---

## See also

- [Ecosystem SDK README](../products/arves-ecosystem-sdk/README.md) — the value model and the
  full author → certify → package → install → invoke story.
- [AUTHORING_LANGUAGES.md](./AUTHORING_LANGUAGES.md) — what language you author capabilities in
  (today: JavaScript/`.mjs`).
- [SPEC_STARTER.md](./SPEC_STARTER.md) — the three specification documents to read first.
- Worked examples:
  [`examples/csv-source.capability.mjs`](../products/arves-ecosystem-sdk/examples/csv-source.capability.mjs)
  (real CSV → `uci.fact`),
  [`examples/invoice-ocr.capability.mjs`](../products/arves-ecosystem-sdk/examples/invoice-ocr.capability.mjs),
  [`examples/third-party-capability.mjs`](../products/arves-ecosystem-sdk/examples/third-party-capability.mjs).
