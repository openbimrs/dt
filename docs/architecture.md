# Architecture

## Repository role

`openbimrs/dt` is the canonical source repository for ISO 23387 data-template
contracts. `packages/dt` is its superproject integration location: an
`openbimrs/openbim` integration commit records one verified child revision and
provides ecosystem-level tests plus the feature-gated `openbim` facade.

The child repository remains buildable without cloning the integration
workspace. Published crates therefore use explicit metadata and versioned
registry dependencies rather than paths into sibling repositories.

## Dependency direction

```text
openbim-core  <-  openbim-dt  <-  openbim-loin / dictionary clients
                         ^
openbim facade  ----------+
```

- Data-template contracts may use released shared core contracts.
- LOIN and dictionary clients may consume released data-template contracts.
- Data templates must never depend on LOIN.
- The facade may optionally re-export data-template and consumer crates.

This keeps a reusable vocabulary below the document formats that import it.

## Responsibility layers

Future implementation has separate responsibilities:

1. **Domain contracts** — concepts, templates, properties, groups, identifiers,
   multilingual text, units, dimensions, quantity kinds, and references.
2. **Wire representation** — namespace-aware XML syntax, ordering, lexical
   evidence, and unknown-content retention.
3. **Validation** — structural and semantic diagnostics with stable paths and
   source/version evidence.
4. **Governance and mappings** — explicit ISO 23386 lifecycle and ISO 12006-3
   mapping contracts rather than implicit codec policy.
5. **Consumers** — LOIN, dictionary clients, and application adapters depending
   only on released lower-level APIs.

Parsing does not imply validation. A typed model does not imply complete schema
coverage. Reading known fields does not imply lossless writing.

## Current scaffold contracts

The `0.1.1` release commits only:

- the ISO 23387 edition 2 XML namespace;
- a named draft placeholder namespace for future targeted diagnostics.

These constants do not imply a model, parser, writer, validator, governance
workflow, mapping implementation, or consumer integration.

## Standards and fixture boundary

No ISO/DIN/CEN document, XSD, or annex example is vendored. Local references stay
under ignored `references/`. A fixture enters `tests/fixtures/` only when it is
original synthetic material or has explicit redistribution terms compatible
with the repository and crate.

The locally available ISO 23387 Annex F example remains a restricted reference;
it is not a public test fixture.

## Cross-repository delivery

Changes spanning repositories follow dependency order:

1. land and publish `openbim-dt` changes;
2. update consumer crates against the released DT contract;
3. push and publish those consumers;
4. update and verify the OpenBIM.rs submodule pins;
5. publish the integration commit when a facade release is intended.

Each superproject pin is a compatibility declaration and rollback point.
