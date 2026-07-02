# ARVES Specification Corpus — frozen source

These ~50 `.docx` files are the **authoritative, frozen** ARVES specification corpus
(Volumes, Ontology, Certification Manual, Invariant Registry, Freeze Record, IDR/CCP batches,
catalogs). They are the source of record (ED-001): their content does **not** change here —
a byte-affecting specification change goes through the **CCP / Amendment** process.

- **Readable mirror:** a faithful, grep-/diff-/PR-able Markdown rendering of every file is in
  [`../spec-markdown/`](../spec-markdown/). Read/search there; if the mirror and a `.docx` ever
  disagree, **the `.docx` wins**.
- **Regenerate the mirror:** `python tools/docx_to_markdown.py`.

*Relocated from the repository root into `corpus/` during a Growth-era organization pass —
content byte-unchanged; only the folder changed.*
