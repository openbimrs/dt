# openbim-dt crate instructions

`openbim-dt` is the canonical ISO 23387 data-template crate.

Current implementation: ISO 23387 edition 2 namespace identity and a named draft
placeholder namespace only. No model, parser, writer, validator, governance
workflow, or mapping is implemented.

## Rules

- Keep the crate independent of the OpenBIM.rs parent workspace.
- Keep `#![forbid(unsafe_code)]`.
- Preserve dependency direction: this crate may use `openbim-core`; it must not
  depend on LOIN or higher-level consumers.
- Public capability claims require tests and redistributable evidence.
- Do not copy standards prose, XSDs, PDFs, or annex examples into crate sources.
- Keep `AGENTS.md` and `PLAN.md` excluded from crates.io archives.

Run `../scripts/gate.sh` from this directory or `./scripts/gate.sh` from the
repository root before committing.
