export const meta = {
  name: 'ados-council',
  description: 'ARVES Development OS Executive Council: a panel of C-level chief lenses + Independent Challenger + Future Architect assess the whole repo, then the PMO synthesizes ONE prioritized, dependency-traced backlog with the top-3 to do now.',
  whenToUse: 'At a milestone boundary, or when you need the single actionable program plan ("what are the top 3 things to actually do now?"). Reusable and idempotent.',
  phases: [
    { title: 'Council', detail: 'C-level chiefs + Independent Challenger + Future Architect each assess their domain (wave-batched)' },
    { title: 'PMO', detail: 'synthesize all assessments into one prioritized, dependency-traced backlog + top-3' },
  ],
}

const ROOT = 'c:/Users/hkuzudisli/Desktop/Arves-Foundation-Docs'
const REVIEWS = `${ROOT}/runtime/docs/reviews`
const STANDARDS = `${ROOT}/runtime/docs/standards`
const VERIF = `${ROOT}/verification`
const CRATES = `${ROOT}/runtime/crates`
const ORG = `${ROOT}/runtime/docs/organization`
const label = (args && args.label) || 'latest'

const CHIEF_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    chief: { type: 'string' },
    verdict: { type: 'string', description: 'one-line state-of-the-domain' },
    top_risks: { type: 'array', items: { type: 'string' } },
    top_opportunities: { type: 'array', items: { type: 'string' } },
    recommended_next: {
      type: 'array',
      description: '1-4 concrete next actions this chief would fund',
      items: {
        type: 'object',
        additionalProperties: false,
        properties: {
          action: { type: 'string' },
          instrument: { type: 'string', enum: ['IDR', 'CCP-Amendment', 'ACS', 'Runtime', 'Verification', 'Certification', 'Ecosystem', 'Product'] },
          roi: { type: 'integer', description: '1-10' },
          risk: { type: 'string', enum: ['low', 'medium', 'high'] },
          depends_on: { type: 'string' },
        },
        required: ['action', 'instrument', 'roi', 'risk', 'depends_on'],
      },
    },
  },
  required: ['chief', 'verdict', 'top_risks', 'top_opportunities', 'recommended_next'],
}

const PMO_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    backlog_path: { type: 'string' },
    executive_summary: { type: 'string' },
    top_3: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        properties: {
          rank: { type: 'integer' },
          task: { type: 'string' },
          roi: { type: 'integer' },
          risk: { type: 'string', enum: ['low', 'medium', 'high'] },
          depends_on: { type: 'string' },
          traces_to: { type: 'string' },
          done_check: { type: 'string' },
        },
        required: ['rank', 'task', 'roi', 'risk', 'depends_on', 'traces_to', 'done_check'],
      },
    },
    conflicts_resolved: { type: 'array', items: { type: 'string' } },
  },
  required: ['backlog_path', 'executive_summary', 'top_3', 'conflicts_resolved'],
}

const CTX = `You are a C-level chief in the ARVES Development OS Executive Council. Objective: help ARVES become an ISO/IEEE-grade, independently-implementable, vendor-neutral cognitive-infrastructure standard that lasts 20 years. Read the CURRENT repository state relevant to your domain (Read/Grep/Glob):
- Reviews + Global Readiness Report: ${REVIEWS}
- ACS/CCP standards + v1.1 program: ${STANDARDS}
- Evidence (formal/ runtime/ certification/ independent/): ${VERIF}
- Reference runtime crates: ${CRATES}
- Roadmap, doctrine (ED-001..004), organization, PMO backlog: ${ROOT}/runtime/docs/ARVES_Master_Roadmap.md, ${ROOT}/runtime/docs/ENGINEERING_DOCTRINE.md, ${ORG}
RULES: never propose editing the frozen .docx corpus (ED-001); only IDR/CCP/ACS/Runtime/Verification/Certification/Ecosystem/Product instruments. Be concrete and current (reflect what is already done: I1 complete, architecture gate live, ACS-001/002 + CCP-005 drafted, ACS-003/004 pending, L1 not yet attested). Return only your domain's assessment; the PMO will reconcile across chiefs.`

const CHIEFS = [
  { key: 'cto', title: 'CTO', focus: 'Whole-system technical strategy. Read broadly. What is the single highest-ROI work right now, and what should WAIT and why? Name top risks/opportunities/investments/technical-debt.' },
  { key: 'chief-scientist', title: 'Chief Scientist', focus: 'Theory/ontology/mathematics/formal-semantics/invariants/claims. What is publishable? provable? What remains informal or unfalsifiable? Which formalization has the highest scientific ROI?' },
  { key: 'chief-runtime', title: 'Chief Runtime Engineer', focus: 'Runtime only: scaling, latency, memory, storage, replay, consensus, recovery, replication, benchmark/profiling. What runtime work is prerequisite vs premature (esp. re: I2)?' },
  { key: 'chief-standard', title: 'Chief Standard Engineer', focus: 'Every ACS/IDR/CCP/contract/invariant: duplicates, ambiguity, undefined terms, missing MUST/SHOULD, wrong ownership, broken cross-references. What standardization work unblocks independent implementation first?' },
  { key: 'chief-verification', title: 'Chief Verification Engineer', focus: 'TLA+, model checking, property/mutation/differential/fuzz testing, replay, independent runtime. What proof is cheapest+highest-leverage next (note the architecture gate already exists)?' },
  { key: 'chief-security', title: 'Chief Security Engineer', focus: 'Zero-trust threat model: replay, tampering, trust, identity, signatures, envelope, content-addressing, supply chain, secrets. What security work must land before external adoption?' },
  { key: 'chief-dx', title: 'Chief DX Engineer', focus: 'As a brand-new developer: could I understand ARVES? how long? what is confusing, missing (tutorials/samples), inconsistently named, weakly documented? Highest-ROI DX fix?' },
  { key: 'chief-product', title: 'Chief Product Engineer', focus: 'Could a hospital / Siemens / OpenAI / a startup build on ARVES today? What APIs/SDKs/examples are missing? What is the first lighthouse product and what does it need?' },
  { key: 'chief-performance', title: 'Chief Performance Engineer', focus: 'Benchmarks, latency, memory, CPU, storage, snapshot/replay cost, scaling at 10/100/1000/10000 nodes. What perf work is needed vs premature at the current stage?' },
  { key: 'chief-ecosystem', title: 'Chief Ecosystem Engineer', focus: 'What connector/capability/SDK/language/marketplace next? Community + open-source readiness. What is the first ecosystem move that compounds?' },
  { key: 'independent-challenger', title: 'Independent Challenger', focus: 'Your job is NOT to improve ARVES; it is to try to KILL it. Assume ARVES is fundamentally wrong. Destroy assumptions; find contradictions, impossible cases, hidden coupling, undefined behaviour, fatal academic criticism, implementation dead-ends. If you cannot kill it, say precisely why it survives.' },
  { key: 'future-architect', title: 'Future Architect (2030-2050)', focus: 'Ignore today. Will ARVES survive quantum, AGI, embodied AI, regulation? What becomes obsolete, what becomes universal? Which decision made now would age badly, and what to do today to prevent it?' },
]

log(`ADOS Executive Council: ${CHIEFS.length} chiefs -> PMO synthesis. Milestone label: ${label}.`)

const pad2 = (n) => (n < 10 ? '0' + n : '' + n)
const BATCH = 3
const assessments = []
for (let i = 0; i < CHIEFS.length; i += BATCH) {
  const chunk = CHIEFS.slice(i, i + BATCH)
  log(`Council wave ${i / BATCH + 1}/${Math.ceil(CHIEFS.length / BATCH)}: ${chunk.map((c) => c.title).join(', ')}`)
  const part = await parallel(
    chunk.map((c) => () =>
      agent(`${CTX}\n\nYOUR ROLE: ${c.title}\n${c.focus}`, { label: `chief:${c.key}`, phase: 'Council', schema: CHIEF_SCHEMA })
    )
  )
  assessments.push(...part.filter(Boolean))
}
log(`Council complete: ${assessments.length}/${CHIEFS.length} chiefs reported. PMO synthesizing.`)

const backlogPath = `${ORG}/PMO_Backlog_${label}.md`
const pmoPrompt = `You are the ARVES Program Management Office (PMO). ${assessments.length} C-level chiefs (incl. an Independent Challenger and a Future Architect) each returned a domain assessment (JSON below). Also read the existing PMO backlog and Global Readiness Report under ${ORG} and ${REVIEWS} for continuity.

Produce ONE actionable program plan and WRITE it to ${backlogPath}:
1) A single, de-duplicated, dependency-ordered backlog. Each item: rank, task, instrument, ROI (1-10), risk, blocked-by, traces-to (standard/ACS/IDR/invariant + owning office), and a done-check (its conformance/verification criterion). Merge overlapping chief recommendations; resolve conflicts explicitly (state the resolution + rationale).
2) A clear TOP 3 "do now", in order, with dependencies honored.
3) Respect: frozen corpus immutable (ED-001); one property per milestone (ED-002); adversarial hunt mandatory (ED-003); Scientifically-Proven DoD (ED-004); the pre-I2 gate (interop + Lock Review before I2). Prefer executing identified work over expanding the meta-organization.

Then RETURN the schema summary (backlog_path=${backlogPath}, executive_summary, top_3, conflicts_resolved).

Chief assessments (JSON):
${JSON.stringify(assessments, null, 1)}`

const pmo = await agent(pmoPrompt, { label: 'pmo:synthesis', phase: 'PMO', schema: PMO_SCHEMA })
return { council: assessments.length, backlog: backlogPath, pmo }
