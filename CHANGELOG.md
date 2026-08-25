# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

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

[Unreleased]: https://github.com/openbimrs/dt/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/openbimrs/dt/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/openbimrs/dt/releases/tag/v0.1.0
