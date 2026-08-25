# openbim-dt crate instructions

`openbim-dt` is the canonical ISO 23387 edition 2 data-template crate.

Implemented scope: value contracts, reusable ConceptType and owned complex-type
contracts, bounded strict namespace-aware parser, semantic writer with
unknown-content retention, typed views/wrappers for DT element families,
structured built-in diagnostics, and CLI.
It is not an XSD validator, complete conformance engine, ISO 23386 governance
workflow, or ISO 12006-3 mapper.

## Rules

- Keep the crate independent of the OpenBIM.rs parent workspace.
- Keep `#![forbid(unsafe_code)]`.
- Preserve dependency direction: LOIN and other consumers may depend on DT; DT
  must not depend on them.
- Keep format-specific XML policy here over the direct maintained parser; do not
  recreate a project-wide XML codec abstraction.
- Parsing and validation remain separate APIs.
- Unknown XML content must survive semantic parse/write/parse round trips.
- Public capability claims require tests, redistributable evidence, and mutation
  probes where a silent regression would invalidate the claim.
- Do not copy standards prose, XSDs, PDFs, or annex examples into crate sources.
- Keep fixture provenance beside every committed fixture.
- Keep `AGENTS.md` and `PLAN.md` excluded from crates.io archives.

Run `../scripts/gate.sh` from this directory or `./scripts/gate.sh` from the
repository root before committing.
