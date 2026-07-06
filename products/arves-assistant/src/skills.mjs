// ARVES Assistant — the SKILL LAYER (A3): a skill is a CERTIFIED capability.
//
// This layer is a thin, honest composition of two frozen-platform surfaces:
//   - the Ecosystem SDK trust boundary (products/arves-ecosystem-sdk/src/kit.mjs):
//     defineCapability / certifyCapability — REUSED, not reimplemented, so a skill is
//     the same kind of citizen as any marketplace capability;
//   - the bridge's dynamic `bind` verb (RCR-016) + `invoke` verb: once registered, a
//     skill's effects are committed through the FULL runtime chain
//     Capability (resolve binding) -> Engine (fabric-enforced) -> Kernel (commit truth).
//
// THE GATE IS RE-RUN, NEVER TRUSTED: registerSkill() re-executes certifyCapability()
// against the supplied test inputs AT INSTALL TIME. Any `certified: true` flag a caller
// staples onto a capability object is IGNORED — a forged flag cannot get an uncertified
// skill bound (proven in skills.test.mjs). This inherits the certifier's honest limits:
// the determinism check is a best-effort run-twice PROBE over the supplied inputs, not a
// purity proof (engine-enforced determinism is deferred v1.1 RCR debt).
//
// HONEST EXECUTION MODEL (v1.0): the bridge hosts exactly ONE engine (the reference
// `engine:derive.fact@1.0.0`); `bind` attaches the skill's NAME to that engine identity —
// the runtime does NOT load the skill's JS code. The skill's execute() runs PRODUCT-SIDE
// (here, in this process); each proposed effect VALUE is then carried through the runtime
// chain under the bound name and committed as ACS-addressed truth. Runtime-hosted product
// engine code would be a platform change — recorded as an RCR candidate in README.md,
// never faked here.

import { certifyCapability, codeHash } from '../../arves-ecosystem-sdk/src/kit.mjs';

// Re-export the authoring surface so a skill author needs only this module.
export { defineCapability, certifyCapability, float } from '../../arves-ecosystem-sdk/src/kit.mjs';

/** Register a skill on an assistant: (1) RE-RUN certification — the only gate that
 *  counts; (2) commit a skill-registration truth (name/version/produces/codeHash —
 *  the audit record of WHAT code was admitted); (3) dynamically `bind` the skill name
 *  in the assistant's shard (RCR-016); (4) attach it to the assistant's skill index so
 *  think()/invokeSkill() can execute it through the full chain.
 *
 *  `testInputs`: >=1 representative inputs (certification refuses to pass vacuously).
 *  Skill names must be single whitespace-free tokens (the bridge `bind` grammar). */
export async function registerSkill(assistant, cap, testInputs) {
  if (cap === null || typeof cap !== 'object' || !cap.manifest || typeof cap.execute !== 'function') {
    throw new Error('skill refused: not a capability — author it with defineCapability({ name, version, produces, execute })');
  }
  const name = cap.manifest.name;

  // FORGED-FLAG REFUSAL: certification is re-run RIGHT HERE, on every registration.
  // No property of `cap` (e.g. a stapled-on `certified: true`) is ever consulted.
  const cert = certifyCapability(cap, testInputs);
  if (!cert.certified) {
    const failed = cert.checks.filter((c) => !c.ok).map((c) => c.name).join(', ');
    throw new Error(`skill '${name}' refused: fails certification (${failed}) — a forged 'certified' flag cannot bypass this gate`);
  }

  // The admission itself is auditable committed truth. codeHash binds the ACTUAL execute
  // source (kit.mjs), so the ledger records which code was admitted under this name.
  const registration = {
    type: 'uci.assistant.skill',
    name,
    version: cap.manifest.version,
    produces: [...cap.manifest.produces],
    codeHash: codeHash(cap),
  };
  const reg = await assistant.commitTruth(registration, 'trace');

  // RCR-016: dynamic bind in the assistant's shard — from here on, `invoke <name>` flows
  // Capability -> Engine -> Kernel instead of being refused `ERR unbound`.
  await assistant.bindName(name);

  // attachSkill() is itself a gate (defense in depth): it RE-RUNS certification over
  // these testInputs and ignores any caller-supplied checks — so it needs the inputs.
  assistant.attachSkill(name, { cap, registrationId: reg.contentId, testInputs: [...testInputs] });
  return { name, bound: true, registrationId: reg.contentId, checks: cert.checks };
}
