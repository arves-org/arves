# @arves/enterprise-os — Enterprise Cognitive OS (P5)

An operating system for an **organization's** cognition: department agents share **one**
content-addressed truth base, governance **policy is enforced as truth**, and every
violation, approval, and conflict is a **replayable compliance event** in the real Kernel.
Built entirely on the **frozen ARVES Runtime v1.0** — the *second* product on the same
unchanged runtime, and thus the proof that ARVES is a platform, not one app.

## Why existing AI can't do it

| Feature | Why a wrapper (ChatGPT/LangGraph/AutoGen/CrewAI) can't | Runtime API | Truth / Evidence |
|---|---|---|---|
| **Multi-agent shared truth** | each agent has its own context; there is no single authoritative, content-addressed truth base | SDK + Bridge | one `uci.fact` set, shared |
| **Policy enforced as truth** | a wrapper cannot *prove* a decision was checked against policy, nor block it | Bridge commit | `uci.policy` truths; decision checked against them |
| **Compliance audit ledger** | no tamper-evident, replayable record of what was blocked/approved and why | Bridge → Kernel | `uci.compliance` truths |
| **Cross-department consistency** | no shared, addressable decision history to detect that ops contradicts finance/legal | Bridge commit | prior `uci.decision` ContentId as evidence |

## Run

```
cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml
node examples/enterprise-day.mjs
```

The demo (exits 0): a $150k spend is **blocked** by policy → **allowed** after legal
approval (committed as truth) → a later ops "cancel" is **blocked as a cross-department
conflict** (cites the approved decision's truth id) → a compliant small spend is **not
falsely blocked**. Every step is truth in the real Kernel: auditable and replayable.

## Platform boundary

Consumes the **frozen Runtime v1.0** API (SDK + Bridge → real Kernel); edits no runtime or
spec file. It needed **no Runtime Change Request** — v1.0 carried a second, structurally
different product unchanged. If a future enterprise feature needs the runtime, that is an
RCR (→ v1.1), never a runtime edit (`runtime/RUNTIME_FREEZE_v1.0.md`).
