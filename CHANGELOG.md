# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-08-25

### Added

- Added validated GUID, XML Schema date-time/language/decimal/any-URI,
  multilingual text, reference, rational, data-type/value-list,
  unit-scale/base, and reusable ConceptType contracts.
- Added a reusable arbitrary-precision XML Schema `positiveInteger` contract.
- Modelled the four Annex E `DataTypeType` boundary variants and its repeating,
  implementation-defined regular-expression `DataFormat` values.
- Added owned contracts for the ISO 23387 complex types imported by ISO 7817-3:
  subject, object type, property, group, quantity kind, reference document,
  dimension, unit, and data template.
- Added a bounded, namespace-aware XML 1.0 parser and semantic writer retaining
  namespaces, ordering, comments, processing instructions, CDATA, and unknown
  elements and attributes.
- Added typed views and owned wrappers for all five concrete global roots and
  the local `Library` child families represented by the Annex E XSD inventory.
- Added structured built-in diagnostics with stable severity, category, and path
  fields while keeping parsing separate from validation.
- Added `inspect`, `validate`, and `rewrite` CLI commands.
- Added original synthetic XML fixtures with committed provenance and mutation
  probes for strict XML parsing, root classification, unknown-content retention,
  and safe CLI output replacement.

### Changed

- Replaced the unused `openbim-core` scaffold dependency with direct maintained
  `roxmltree` well-formedness checking and `quick-xml` event integration.
- Updated documentation from namespace-scaffold status to the exact implemented
  `0.2` capability boundary without claiming XSD or clause-level conformance.

### Security

- Reject malformed XML 1.0, duplicate expanded attributes, illegal namespace
  bindings, DTD declarations, undeclared entities and prefixes, invalid
  characters, and malformed qualified names; enforce configurable byte, depth,
  node, and per-element attribute limits.
- Write CLI rewrites through an OS-random, exclusively created same-directory
  temporary file so pre-created symlinks cannot redirect the write.

### Fixed

- Corrected global-root classification to the five concrete Annex E declarations
  while retaining typed local `Library` child views.
- Made multilingual-text and reference diagnostics context-sensitive to the
  actual Annex E `Symbol` and `*Ref` declarations.
- Made the owned Concept core require validated creation-date, name, and
  definition state and applied XML Schema whitespace collapsing to typed scalar
  values.
- Reworked inherited namespace handling to use delta scopes and shared URI
  storage, preventing depth-amplified copies of ancestor namespace declarations.
- Corrected `DateTime` extended-year and end-of-day fractional-second edges and
  aligned `AnyUri` with XML Schema 1.0's broad escaped lexical space.
- Preserved tab, newline, and carriage-return character references in attribute
  values, plus carriage-return references in text, across repeated parse/write/
  parse cycles.
- Corrected `ValueListType` to require a validated language and one-or-more
  `DataValue` children whose optional order is bounded to XML Schema `int`, and
  restored missing-language diagnostics.
- Connected the complete pre-standalone DT lineage across
  `packages/openbim/openbim-dt`, `packages/openbim-dt`, and `packages/dt`
  without rewriting the published `v0.1.0` or `v0.1.1` release commits.
- Made the GitHub Pages upload include hidden files so the deployed artifact
  matches the tree validated by the documentation gate, including `.nojekyll`.

## [0.1.1] - 2026-08-25

### Changed

- Established `openbimrs/dt` as the canonical standalone repository while
  preserving the data-template subtree history.
- Corrected repository, homepage, documentation, and package metadata.
- Added standalone CI, packaging gates, architecture documentation, roadmap,
  contributor guidance, and a verified GitHub Pages documentation pipeline.
- Clarified that the released API is a namespace scaffold, not a complete ISO
  23387 model, parser, writer, or validator.

### Security

- Added fail-closed package and documentation checks preventing ISO/DIN/CEN
  references, schemas, PDFs, and annex examples from entering release artifacts.

## [0.1.0] - 2026-08-24

### Added

- Reserved `openbim-dt` with ISO 23387 edition 2 and draft-placeholder namespace
  constants.

[Unreleased]: https://github.com/openbimrs/dt/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/dt/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/openbimrs/dt/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/openbimrs/dt/releases/tag/v0.1.0
