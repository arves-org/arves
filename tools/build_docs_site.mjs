#!/usr/bin/env node
/*
 * ARVES documentation site generator (Growth-era DX, zero dependencies).
 *
 * Renders the repo's existing Markdown (README, QUICKSTART, product & platform docs, and the
 * spec-markdown/ corpus mirror) into a static, navigable site under `docs-site/` — so a first
 * visitor lands on Getting Started -> Architecture -> Runtime -> Standard -> SDK -> Products ->
 * Certification -> Marketplace -> Foundation and writes a first capability in minutes, instead
 * of facing 50 raw files.
 *
 * No external dependencies, no build step: the output is plain static HTML/CSS deployable to
 * GitHub Pages as-is (a .nojekyll is emitted). Regenerate: node tools/build_docs_site.mjs
 * Single source of truth: the Markdown. Do not hand-edit docs-site/ — regenerate.
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const OUT = path.join(ROOT, 'docs-site');
const SPEC = path.join(ROOT, 'spec-markdown');

// ---------- tiny, self-contained Markdown -> HTML ----------
const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const slugify = (s) => s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 60);

function inline(s, linkMap) {
  s = esc(s);
  s = s.replace(/`([^`]+)`/g, (_m, c) => `<code>${c}</code>`);
  s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_m, t, u) => {
    let href = u.trim();
    const hashIdx = href.indexOf('#');
    const anchor = hashIdx >= 0 ? href.slice(hashIdx) : '';
    const bare = hashIdx >= 0 ? href.slice(0, hashIdx) : href;
    if (/^https?:\/\//.test(bare)) { /* external */ }
    else if (linkMap[bare]) href = linkMap[bare] + anchor;
    else { const base = bare.split('/').pop(); if (linkMap[base]) href = linkMap[base] + anchor; }
    return `<a href="${href}">${t}</a>`;
  });
  s = s.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  s = s.replace(/(^|[^*])\*([^*]+)\*/g, '$1<em>$2</em>');
  return s;
}

function splitRow(line) {
  let s = line.trim();
  if (s.startsWith('|')) s = s.slice(1);
  if (s.endsWith('|')) s = s.slice(0, -1);
  return s.split('|').map((x) => x.replace(/\\\|/g, '|').trim());
}

function mdToHtml(md, linkMap) {
  const lines = md.replace(/\r\n/g, '\n').split('\n');
  const out = [];
  let para = [];
  const flush = () => { if (para.length) { out.push(`<p>${inline(para.join(' '), linkMap)}</p>`); para = []; } };
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (/^\s*```/.test(line)) {
      flush();
      const lang = line.trim().slice(3).trim();
      i++; const buf = [];
      while (i < lines.length && !/^\s*```/.test(lines[i])) { buf.push(lines[i]); i++; }
      i++;
      out.push(`<pre><code${lang ? ` class="lang-${esc(lang)}"` : ''}>${esc(buf.join('\n'))}</code></pre>`);
      continue;
    }
    const h = /^(#{1,6})\s+(.*)$/.exec(line);
    if (h) { flush(); const lvl = h[1].length; const txt = h[2].replace(/#+\s*$/, '').trim(); out.push(`<h${lvl} id="${slugify(txt)}">${inline(txt, linkMap)}</h${lvl}>`); i++; continue; }
    if (/^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) { flush(); out.push('<hr>'); i++; continue; }
    if (/^>\s?/.test(line)) {
      flush(); const buf = [];
      while (i < lines.length && /^>\s?/.test(lines[i])) { buf.push(lines[i].replace(/^>\s?/, '')); i++; }
      out.push(`<blockquote>${mdToHtml(buf.join('\n'), linkMap)}</blockquote>`);
      continue;
    }
    if (/\|/.test(line) && i + 1 < lines.length && /^\s*\|?\s*:?-{2,}/.test(lines[i + 1]) && lines[i + 1].includes('-')) {
      flush();
      const header = splitRow(line); i += 2; const rows = [];
      while (i < lines.length && lines[i].includes('|') && lines[i].trim() !== '') { rows.push(splitRow(lines[i])); i++; }
      let t = '<div class="table-wrap"><table><thead><tr>' + header.map((c) => `<th>${inline(c, linkMap)}</th>`).join('') + '</tr></thead><tbody>';
      for (const r of rows) t += '<tr>' + r.map((c) => `<td>${inline(c, linkMap)}</td>`).join('') + '</tr>';
      out.push(t + '</tbody></table></div>');
      continue;
    }
    // A list starts only at a block boundary (para empty) — so a wrapped paragraph line that
    // happens to begin with +/-/* is not misread as a bullet.
    if (/^\s*([-*+]|\d+\.)\s+/.test(line) && para.length === 0) {
      const ordered = /^\s*\d+\.\s+/.test(line); const items = [];
      while (i < lines.length) {
        const l = lines[i];
        if (/^\s*([-*+]|\d+\.)\s+/.test(l)) { items.push(l.replace(/^\s*([-*+]|\d+\.)\s+/, '')); i++; continue; }
        // lazy continuation: a wrapped line that is not itself a new block joins the current item,
        // so multi-line bullets stay one <li>, ordered lists stay one <ol> (1..N), and **/` pairs balance.
        const isBlock = l.trim() === '' || /^#{1,6}\s+/.test(l) || /^\s*```/.test(l) || /^>\s?/.test(l) || /^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(l);
        if (!isBlock && items.length) { items[items.length - 1] += ' ' + l.trim(); i++; continue; }
        break;
      }
      const tag = ordered ? 'ol' : 'ul';
      const start = ordered ? (parseInt(line.trim(), 10) || 1) : 1; // preserve authored numbering across heading breaks
      const open = start > 1 ? `<ol start="${start}">` : `<${tag}>`;
      out.push(open + items.map((x) => `<li>${inline(x, linkMap)}</li>`).join('') + `</${tag}>`);
      continue;
    }
    if (line.trim() === '') { flush(); i++; continue; }
    para.push(line.trim()); i++;
  }
  flush();
  return out.join('\n');
}

const plainText = (md) => md.replace(/```[\s\S]*?```/g, ' ').replace(/[#>*`_|-]/g, ' ').replace(/\[([^\]]+)\]\([^)]+\)/g, '$1').replace(/\s+/g, ' ').trim();

// ---------- site structure ----------
const PAGES = [
  { section: 'Getting Started', slug: 'index', title: 'Home', landing: true },
  { section: 'Getting Started', slug: 'manifesto', title: 'Why ARVES Exists', src: 'WHY_ARVES.md' },
  { section: 'Getting Started', slug: 'quickstart', title: 'Quickstart (10 min)', src: 'QUICKSTART.md' },
  { section: 'Getting Started', slug: 'why-arves', title: 'Why ARVES', src: 'README.md' },
  { section: 'Getting Started', slug: 'contributing', title: 'Contributing', src: 'CONTRIBUTING.md' },
  { section: 'Platform', slug: 'runtime', title: 'Runtime v1.0 (Frozen)', src: 'runtime/RUNTIME_FREEZE_v1.0.md' },
  { section: 'Platform', slug: 'standard', title: 'The Standard', src: 'standard/README.md' },
  { section: 'Platform', slug: 'certification', title: 'Certification Program', src: 'verification/evidence/CERTIFICATION_PROGRAM.md' },
  { section: 'Platform', slug: 'evidence', title: 'Evidence Ledger', src: 'verification/evidence/EVIDENCE_LEDGER.md' },
  { section: 'SDK & Ecosystem', slug: 'sdk', title: 'Developer SDK', src: 'products/arves-sdk-ts/README.md' },
  { section: 'SDK & Ecosystem', slug: 'authoring-kit', title: 'Authoring Kit (arves CLI)', src: 'products/arves-ecosystem-sdk/README.md' },
  { section: 'SDK & Ecosystem', slug: 'marketplace', title: 'Marketplace', src: 'products/arves-marketplace/README.md' },
  { section: 'Products', slug: 'products', title: 'Products Overview', src: 'products/README.md' },
  { section: 'Products', slug: 'personal-os', title: 'Personal Cognitive OS', src: 'products/arves-personal-os/README.md' },
  { section: 'Products', slug: 'enterprise-os', title: 'Enterprise Cognitive OS', src: 'products/arves-enterprise-os/README.md' },
  { section: 'Products', slug: 'cognitive-memory', title: 'Cognitive Memory', src: 'products/arves-cognitive-memory/README.md' },
  { section: 'Products', slug: 'agent-runtime', title: 'Agent Runtime', src: 'products/arves-agent-runtime/README.md' },
  { section: 'Foundation', slug: 'foundation', title: 'Foundation', src: 'FOUNDATION.md' },
  { section: 'Foundation', slug: 'closure', title: 'Build Program Closure', src: 'ARVES_BUILD_PROGRAM_CLOSURE.md' },
  { section: 'Foundation', slug: 'rcr-001', title: 'RCR-001 (Runtime v1.1)', src: 'runtime/rcr/RCR-001.md' },
  { section: 'Foundation', slug: 'success', title: 'Success — the North Star', src: 'SUCCESS.md' },
  { section: 'Foundation', slug: 'failure', title: 'Why ARVES Could Fail', src: 'FAILURE.md' },
  { section: 'Foundation', slug: 'releasing', title: 'Releasing (Growth Protocol)', src: 'RELEASING.md' },
  { section: 'Foundation', slug: 'dx-baseline', title: 'Developer Journey (DX baseline)', src: 'verification/dx/DEVELOPER_JOURNEY_REPORT.md' },
  { section: 'Getting Started', slug: 'cli-reference', title: 'CLI Reference', src: 'docs/CLI_REFERENCE.md' },
  { section: 'Getting Started', slug: 'deploy', title: 'Deploy (Docker)', src: 'docs/DEPLOY.md' },
  { section: 'Platform', slug: 'runtime-authors', title: 'Add Your Own Runtime (G2)', src: 'standard/RUNTIME_AUTHORS_GUIDE.md' },
  { section: 'Platform', slug: 'spec-starter', title: 'Spec — Read These First', src: 'docs/SPEC_STARTER.md' },
  { section: 'SDK & Ecosystem', slug: 'reasoning', title: 'AI Capability SDK', src: 'products/arves-ecosystem-sdk/REASONING.md' },
  { section: 'SDK & Ecosystem', slug: 'authoring-languages', title: 'Authoring Languages', src: 'docs/AUTHORING_LANGUAGES.md' },
  { section: 'Specification', slug: 'spec', title: 'Specification Corpus', specIndex: true },
].filter((p) => p.landing || p.specIndex || fs.existsSync(path.join(ROOT, p.src)));

// spec corpus pages
const specFiles = fs.existsSync(SPEC)
  ? fs.readdirSync(SPEC).filter((f) => f.endsWith('.md') && f !== 'README.md').sort()
  : [];
const specPages = specFiles.map((f) => ({
  slug: 'spec/' + f.replace(/\.md$/, ''), title: f.replace(/\.md$/, '').replace(/_/g, ' '), src: 'spec-markdown/' + f, spec: true,
}));

// link map: source path & unique basename -> output html
const allPages = [...PAGES.filter((p) => p.src), ...specPages];
const linkMap = {};
for (const p of allPages) linkMap[p.src] = p.slug + '.html';
const baseCount = {};
for (const p of allPages) { const b = p.src.split('/').pop(); baseCount[b] = (baseCount[b] || 0) + 1; }
for (const p of allPages) { const b = p.src.split('/').pop(); if (baseCount[b] === 1) linkMap[b] = p.slug + '.html'; }
// directory links (e.g. [products/arves-ecosystem-sdk/](...)) -> that dir's README page
for (const p of allPages) { if (p.src.endsWith('/README.md')) { const d = p.src.slice(0, -'/README.md'.length); linkMap[d + '/'] = p.slug + '.html'; linkMap[d] = p.slug + '.html'; } }
// spec-markdown/ dir links -> spec index
linkMap['spec-markdown/'] = 'spec.html';
linkMap['spec-markdown'] = 'spec.html';

const sections = [...new Set(PAGES.map((p) => p.section))];
function navHtml(activeSlug, depth) {
  const up = '../'.repeat(depth);
  let h = '';
  for (const sec of sections) {
    h += `<div class="nav-sec">${sec}</div>`;
    for (const p of PAGES.filter((x) => x.section === sec)) {
      const cls = p.slug === activeSlug ? ' class="active"' : '';
      h += `<a href="${up}${p.slug}.html"${cls}>${p.title}</a>`;
    }
  }
  return h;
}

function page(title, contentHtml, activeSlug, depth = 0) {
  const up = '../'.repeat(depth);
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(title)} · ARVES</title>
<link rel="stylesheet" href="${up}style.css">
</head>
<body>
<header class="topbar">
  <a class="brand" href="${up}index.html">ARVES</a>
  <span class="badge">v1.0 · BUILD SEALED</span>
  <input id="q" class="search" type="search" placeholder="Search the docs…" autocomplete="off">
  <div id="results" class="results"></div>
</header>
<div class="layout">
  <nav class="sidebar">${navHtml(activeSlug, depth)}</nav>
  <main class="content">${contentHtml}
    <footer class="foot">ARVES — a cognitive computing platform · Runtime/Spec/Standard FROZEN · changes via RCR only · generated from the repository Markdown (do not hand-edit).</footer>
  </main>
</div>
<script>window.DEPTH=${depth};</script>
<script src="${up}search.js"></script>
</body>
</html>`;
}

const LANDING = `
<section class="hero">
  <h1>ARVES</h1>
  <p class="tagline">A cognitive computing platform: a frozen, certified runtime that turns AI reasoning into <em>truth</em> — deterministic, content-addressed, replayable, auditable — with an ecosystem of products and third-party capabilities on top.</p>
  <div class="cta">
    <a class="btn primary" href="quickstart.html">Get started</a>
    <a class="btn" href="manifesto.html">Why ARVES?</a>
    <a class="btn" href="foundation.html">Foundation</a>
  </div>
</section>

<section class="fivemin">
  <h2>5 minutes to your first capability</h2>
  <p>Author, self-check, and certify a capability with the <code>arves</code> CLI — <strong>Node ≥18, no runtime build required</strong>; the platform does the heavy lifting:</p>
  <pre><code>node products/arves-ecosystem-sdk/bin/arves.mjs init hospital.incident
node products/arves-ecosystem-sdk/bin/arves.mjs doctor hospital.incident.capability.mjs   # HEALTHY, or every fix you need
node products/arves-ecosystem-sdk/bin/arves.mjs certify hospital.incident.capability.mjs  # -> CERTIFIED
node products/arves-ecosystem-sdk/bin/arves.mjs package hospital.incident.capability.mjs  # -> signed artifact id</code></pre>
  <p class="muted">To also run the runtime demos (Personal / Enterprise Cognitive OS), first build the Runtime API — see <a href="quickstart.html">Quickstart</a>: <code>cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml</code>.</p>
</section>

<section class="cards">
  <a class="card" href="quickstart.html"><h3>Getting Started →</h3><p>Build the runtime, run the demos, ship a capability.</p></a>
  <a class="card" href="runtime.html"><h3>Runtime v1.0 →</h3><p>The frozen substrate: Kernel, Persistence, Bridge, ACS. Changes via RCR only.</p></a>
  <a class="card" href="standard.html"><h3>The Standard →</h3><p>ACS-001..005 + conformance vectors — the contract any runtime implements.</p></a>
  <a class="card" href="sdk.html"><h3>SDK &amp; Authoring Kit →</h3><p>Content-addressing in a few lines; author + certify + publish capabilities.</p></a>
  <a class="card" href="products.html"><h3>Products →</h3><p>Personal &amp; Enterprise Cognitive OS — impossible for a chatbot wrapper.</p></a>
  <a class="card" href="certification.html"><h3>Certification →</h3><p>Certify any runtime from <code>standard/</code> alone. Independence is graded (G1→G2).</p></a>
  <a class="card" href="marketplace.html"><h3>Marketplace →</h3><p>Publish once, install anywhere. Certified &amp; signed; the gate is enforced.</p></a>
  <a class="card" href="foundation.html"><h3>Foundation →</h3><p>Governance + survivability: ARVES designed to outlive its makers.</p></a>
</section>
`;

const CSS = `:root{--bg:#0d1117;--panel:#161b22;--ink:#e6edf3;--mut:#9198a1;--acc:#58a6ff;--acc2:#3fb950;--bd:#30363d;--code:#0b0f14}
*{box-sizing:border-box}html{scroll-behavior:smooth}
body{margin:0;font:16px/1.65 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;background:var(--bg);color:var(--ink)}
a{color:var(--acc);text-decoration:none}a:hover{text-decoration:underline}
.topbar{position:sticky;top:0;z-index:20;display:flex;align-items:center;gap:14px;padding:10px 18px;background:rgba(13,17,23,.92);backdrop-filter:blur(6px);border-bottom:1px solid var(--bd)}
.brand{font-weight:800;letter-spacing:.14em;color:var(--ink);font-size:18px}
.badge{font-size:11px;color:var(--acc2);border:1px solid var(--bd);border-radius:999px;padding:2px 9px;white-space:nowrap}
.search{margin-left:auto;width:min(340px,42vw);padding:7px 11px;border-radius:8px;border:1px solid var(--bd);background:var(--panel);color:var(--ink)}
.results{position:absolute;top:52px;right:18px;width:min(460px,80vw);background:var(--panel);border:1px solid var(--bd);border-radius:10px;overflow:hidden;display:none;max-height:70vh;overflow-y:auto}
.results a{display:block;padding:10px 14px;border-bottom:1px solid var(--bd);color:var(--ink)}
.results a:hover{background:#1f2630;text-decoration:none}.results .t{font-weight:600;color:var(--acc)}.results .x{font-size:13px;color:var(--mut)}
.layout{display:flex;max-width:1220px;margin:0 auto}
.sidebar{width:262px;flex:none;padding:22px 12px 60px;border-right:1px solid var(--bd);height:calc(100vh - 53px);position:sticky;top:53px;overflow-y:auto}
.nav-sec{font-size:12px;text-transform:uppercase;letter-spacing:.08em;color:var(--mut);margin:16px 10px 6px;font-weight:700}
.sidebar a{display:block;padding:5px 10px;border-radius:7px;color:var(--ink);font-size:14.5px}
.sidebar a:hover{background:var(--panel);text-decoration:none}.sidebar a.active{background:#1f6feb22;color:var(--acc);font-weight:600}
.content{flex:1;min-width:0;padding:34px 40px 60px;max-width:860px}
.content h1{font-size:2rem;margin:.2em 0 .6em;line-height:1.2}
.content h2{font-size:1.5rem;margin:1.7em 0 .5em;padding-bottom:.25em;border-bottom:1px solid var(--bd)}
.content h3{font-size:1.2rem;margin:1.4em 0 .4em}
.content code{background:#6e768166;padding:.15em .4em;border-radius:5px;font-size:.9em;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.content pre{background:var(--code);border:1px solid var(--bd);border-radius:10px;padding:14px 16px;overflow-x:auto}
.content pre code{background:none;padding:0;font-size:13.5px;line-height:1.55}
.table-wrap{overflow-x:auto}
table{border-collapse:collapse;width:100%;margin:1em 0;font-size:14.5px}
th,td{border:1px solid var(--bd);padding:7px 11px;text-align:left;vertical-align:top}
th{background:var(--panel)}
blockquote{margin:1em 0;padding:.4em 1em;border-left:3px solid var(--acc);background:var(--panel);border-radius:0 8px 8px 0;color:var(--mut)}
hr{border:none;border-top:1px solid var(--bd);margin:2em 0}
.foot{margin-top:50px;padding-top:18px;border-top:1px solid var(--bd);color:var(--mut);font-size:13px}
.hero{padding:26px 0 8px}.hero h1{font-size:3rem;letter-spacing:.06em;margin:0}
.tagline{font-size:1.15rem;color:var(--mut);max-width:70ch}
.cta{display:flex;gap:12px;flex-wrap:wrap;margin:22px 0 8px}
.btn{border:1px solid var(--bd);border-radius:9px;padding:9px 16px;color:var(--ink);font-weight:600}
.btn.primary{background:var(--acc);color:#0d1117;border-color:var(--acc)}.btn:hover{text-decoration:none;border-color:var(--acc)}
.fivemin{margin:34px 0}
.cards{display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:14px;margin-top:18px}
.card{display:block;border:1px solid var(--bd);border-radius:12px;padding:16px 18px;background:var(--panel)}
.card:hover{border-color:var(--acc);text-decoration:none;transform:translateY(-2px);transition:.15s}
.card h3{margin:.1em 0 .3em;font-size:1.05rem;color:var(--acc)}.card p{margin:0;color:var(--mut);font-size:14px}
.muted{color:var(--mut);font-size:14px}
@media(max-width:820px){.sidebar{display:none}.content{padding:24px 18px}}
`;

const SEARCH_JS = `(function(){
  var up='../'.repeat(window.DEPTH||0);
  var q=document.getElementById('q'),box=document.getElementById('results'),idx=[];
  fetch(up+'search-index.json').then(function(r){return r.json()}).then(function(d){idx=d}).catch(function(){});
  function esc(s){return s.replace(/[&<>]/g,function(c){return{'&':'&amp;','<':'&lt;','>':'&gt;'}[c]})}
  q&&q.addEventListener('input',function(){
    var v=q.value.trim().toLowerCase();
    if(v.length<2){box.style.display='none';return}
    var hits=idx.map(function(p){var t=(p.title.toLowerCase().indexOf(v)>=0?3:0)+(p.text.toLowerCase().indexOf(v)>=0?1:0);return{p:p,s:t}}).filter(function(h){return h.s>0}).sort(function(a,b){return b.s-a.s}).slice(0,12);
    if(!hits.length){box.style.display='none';return}
    box.innerHTML=hits.map(function(h){return '<a href="'+up+h.p.url+'"><span class="t">'+esc(h.p.title)+'</span><br><span class="x">'+esc(h.p.text.slice(0,110))+'…</span></a>'}).join('');
    box.style.display='block';
  });
  document.addEventListener('click',function(e){if(!box.contains(e.target)&&e.target!==q)box.style.display='none'});
})();`;

// ---------- generate ----------
fs.rmSync(OUT, { recursive: true, force: true });
fs.mkdirSync(path.join(OUT, 'spec'), { recursive: true });
fs.writeFileSync(path.join(OUT, 'style.css'), CSS);
fs.writeFileSync(path.join(OUT, 'search.js'), SEARCH_JS);
fs.writeFileSync(path.join(OUT, '.nojekyll'), '');
if (fs.existsSync(path.join(ROOT, 'LICENSE'))) fs.copyFileSync(path.join(ROOT, 'LICENSE'), path.join(OUT, 'LICENSE'));

const searchIndex = [];
let count = 0;

for (const p of PAGES) {
  let html;
  if (p.landing) html = LANDING;
  else if (p.specIndex) {
    const items = specPages.map((s) => `<li><a href="${s.slug}.html">${esc(s.title)}</a></li>`).join('');
    html = `<h1>Specification Corpus</h1>\n<blockquote><p>A faithful Markdown mirror of the frozen <code>.docx</code> corpus (authoritative source of record). Format only; content unchanged. See <a href="foundation.html">Foundation</a>.</p></blockquote>\n<p>${specPages.length} documents:</p>\n<ul class="spec-list">${items}</ul>`;
  } else {
    const md = fs.readFileSync(path.join(ROOT, p.src), 'utf8');
    html = mdToHtml(md, linkMap);
    searchIndex.push({ title: p.title, url: p.slug + '.html', text: plainText(md).slice(0, 500) });
  }
  fs.writeFileSync(path.join(OUT, p.slug + '.html'), page(p.title, html, p.slug, 0));
  count++;
}

for (const s of specPages) {
  const md = fs.readFileSync(path.join(ROOT, s.src), 'utf8');
  fs.writeFileSync(path.join(OUT, s.slug + '.html'), page(s.title, mdToHtml(md, linkMap), 'spec', 1));
  searchIndex.push({ title: s.title, url: s.slug + '.html', text: plainText(md).slice(0, 400) });
  count++;
}

fs.writeFileSync(path.join(OUT, 'search-index.json'), JSON.stringify(searchIndex));
console.log(`docs-site: ${count} pages (${PAGES.length} primary + ${specPages.length} spec), ${searchIndex.length} indexed.`);
console.log(`open: docs-site/index.html   ·   deploy: point GitHub Pages at docs-site/`);
