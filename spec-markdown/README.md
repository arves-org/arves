# ARVES Specification Corpus — Markdown Mirror (v1.0.1)

This directory is a **faithful, regenerable Markdown rendering** of the frozen ARVES
specification corpus — the ~50 `.docx` files in [`corpus/`](../corpus/). It exists so the corpus
can be **searched, `git diff`ed, reviewed line-by-line in pull requests, version-controlled,
and read by tools/AI agents** — the capabilities the binary `.docx` format denied (the
replaceability gap surfaced by the Build Program closure audit, pillar 16).

## Authoritative source of record

> **The `.docx` files remain the AUTHORITATIVE frozen corpus (ED-001).** This Markdown mirror
> is a **derived, non-authoritative rendering**. It must **not** be hand-edited. Any
> byte-affecting change to the specification goes through the **CCP / Amendment** process on
> the corpus itself — never here. If the mirror and a `.docx` ever disagree, the `.docx` wins.

## What this is / is not

- **Is:** a format conversion only — content preserved, not changed. A `v1.0.1` *Foundation
  Improvement*: it changes no specification content, no runtime, and no `standard/` vector.
- **Is not:** a reopening of the sealed Build Program, and not a new source of truth.

## Regenerate

```bash
python tools/docx_to_markdown.py          # rewrite this mirror from the .docx
python tools/docx_to_markdown.py --check   # verify parity only (CI); exit 1 on drift
```

The renderer (`tools/docx_to_markdown.py`, dependency: `python-docx`) preserves document
order (paragraphs and tables interleaved), maps Word heading styles to Markdown headings,
list paragraphs to bullets, tables to GitHub-flavored Markdown tables, and bold/italic runs
to Markdown emphasis. Each file carries a banner naming its authoritative `.docx`.

## Fidelity

All **50/50** corpus files render with **0 parity-LOW** (every mirror retains ≥95% of its
source words; the surplus is Markdown markup). Complex Word features (footnotes, tracked
changes, embedded images, exotic numbering) are rendered best-effort — for anything that
matters legally or normatively, consult the authoritative `.docx`.

*v1.0.1 · Foundation Improvement · format-only · the frozen corpus content is unchanged.*
