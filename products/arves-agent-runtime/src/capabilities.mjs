// ARVES Agent Runtime — capability registry.
//
// A capability is a named, DETERMINISTIC handler: (args, context) -> outcome. Determinism
// is what lets an agent's execution be replayed to the identical content-addressed truth.
// In production these wrap real Engine/Capability-fabric capabilities via the Kernel
// bridge; here they are deterministic stubs with the real interface (Production-first).

export class CapabilityRegistry {
  #caps = new Map();
  register(cap) { this.#caps.set(cap.name, cap); return this; }
  /** Capability Selection: resolve an intent to a capability. */
  select(intent) {
    const c = this.#caps.get(intent);
    if (!c) throw new Error(`Agent Runtime: no capability registered for intent '${intent}'`);
    return c;
  }
  names() { return [...this.#caps.keys()]; }
}

/** The default capability set for the demo. Each is a pure function of its inputs. */
export function defaultCapabilities() {
  return new CapabilityRegistry()
    .register({
      name: 'notify',
      execute: (args) => ({ notified: args.who, about: args.about, channel: 'email' }),
    })
    .register({
      name: 'schedule',
      execute: (args) => ({ scheduled: args.block, forWhom: args.who, before: args.before }),
    })
    .register({
      name: 'brief',
      // facts count is an Integer -> BigInt (ACS-002 §5.2: exact integer carrier).
      execute: (args, context) => ({ briefed: args.who, facts: BigInt(context.length), topic: args.about }),
    });
}
