# OpenBIM.rs Data Templates

[![CI](https://github.com/openbimrs/dt/actions/workflows/ci.yml/badge.svg)](https://github.com/openbimrs/dt/actions/workflows/ci.yml)
[![Documentation](https://github.com/openbimrs/dt/actions/workflows/pages.yml/badge.svg)](https://openbimrs.github.io/dt/)
[![crates.io](https://img.shields.io/crates/v/openbim-dt.svg)](https://crates.io/crates/openbim-dt)
[![docs.rs](https://docs.rs/openbim-dt/badge.svg)](https://docs.rs/openbim-dt)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue)](https://www.rust-lang.org)

Pure-Rust ISO 23387 edition 2 data-template contracts and a bounded,
namespace-aware XML codec. The crate models identifiers, multilingual text,
references, data-type lexemes, concept cores, and the standard's global element
families while retaining XML content it does not yet understand.

This repository is the canonical DT family repository in
[OpenBIM.rs](https://github.com/openbimrs/openbim). The superproject records an
exact child revision at `packages/dt`.

## Documentation

- [Documentation home](https://openbimrs.github.io/dt/)
- [Rust API reference](https://openbimrs.github.io/dt/api/openbim_dt/)
- [Architecture](https://openbimrs.github.io/dt/architecture/)
- [Roadmap](https://openbimrs.github.io/dt/roadmap/)
- [Changelog](https://openbimrs.github.io/dt/changelog/)

## Capability status

The `0.2` source line is an implemented codec and contract layer, not a claim of
complete ISO conformance.

| Capability | Status |
| --- | --- |
| ISO 23387 edition 2 and known draft namespace identities | Implemented and unit-tested |
| GUID, date-time, language/decimal, any-URI/reference, multilingual text, rational, unit-scale/base, data-type/value-list, and reusable concept contracts | Implemented |
| Owned complex-type contracts | Subject, object type, property, group, quantity kind, reference document, dimension, unit, data template, and ordered value-list cores |
| Bounded namespace-aware XML 1.0 parser | Implemented; strict well-formedness and namespace checks precede event indexing |
| XML writer | Implemented; semantic round trips retain element/attribute order, namespaces, comments, PI, CDATA, and unknown content |
| Typed global-root views | Implemented for Library, DataTemplate, ObjectType, GroupOfProperties, and Property |
| Typed local Library-child views | Implemented for Unit, Dimension, QuantityKind, and ReferenceDocument; these are not accepted as document roots |
| Structured built-in diagnostics | Implemented subset; parsing remains separate from validation |
| XML Schema validation of the ISO 23387 element grammar | Implemented; `Document::validate_schema` checks global roots, sequence order, cardinality, choices, attribute presence, and datatype facets |
| Full clause-level ISO conformance | Not implemented; cross-document reference resolution and prose clauses are out of scope |
| Byte-identical XML round trips | Implemented; `Document::to_xml_string_exact` reproduces the parsed bytes exactly and fails closed without retained source |
| Semantic XML round trips | Implemented; `Document::to_xml_string` preserves content but may normalize equivalent syntax |
| ISO 23386 governance workflow | Not implemented |
| ISO 12006-3 mapping | Not implemented |
| bSDD adapter | Not implemented |

The parser has explicit byte, depth, node, and per-element attribute budgets.
`roxmltree` first rejects malformed XML 1.0, duplicate expanded attributes,
illegal namespace bindings, DTD declarations, invalid characters, undeclared
entities and prefixes, and malformed qualified names. `quick-xml` then builds
the retention tree. Unknown but well-formed XML is retained rather than silently
discarded. Validation reports stable categories and paths but does not load or
redistribute the ISO XSD.

## Why DT is separate from LOIN

ISO 7817-3 LOIN imports ISO 23387 types, but LOIN does not own them.
Data-template authoring, dictionary tooling, and other consumers need the same
lower-level contracts without depending on LOIN.

```text
roxmltree + quick-xml  <-  openbim-dt  <-  openbim-loin / dictionary clients
```

The dependency never points from DT back to LOIN.

## Install

```bash
cargo add openbim-dt
```

### Parse, inspect, and validate

```rust
use openbim_dt::{Document, ElementKind, Severity};

let xml = r#"<dt:Library
  xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/"
  dt:GUID="11111111-1111-1111-1111-111111111111"/>"#;
let document = Document::parse(xml)?;
assert_eq!(document.root_kind(), Some(ElementKind::Library));
assert!(document
    .validate()
    .iter()
    .all(|diagnostic| diagnostic.severity != Severity::Error));
let rewritten = document.to_xml_string()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

### CLI

```bash
openbim-dt inspect data-template.xml
openbim-dt validate data-template.xml
openbim-dt validate-schema data-template.xml
openbim-dt rewrite input.xml output.xml
openbim-dt rewrite-exact input.xml output.xml
```

`validate` returns exit code `2` when semantic errors are reported, and
`validate-schema` does the same for schema violations. Parsing and I/O failures
return exit code `1`. `rewrite` emits the semantic serialization;
`rewrite-exact` reproduces the input bytes exactly. Both use an OS-random,
exclusively created same-directory temporary file followed by an atomic rename.

## Standards and fixtures

No ISO, DIN, or CEN document, XSD, or annex example is distributed by this
repository, its crate, or its documentation site. Legally accessed references
belong under the ignored local `references/` directory.

Public tests use an original synthetic fixture whose provenance and AGPL-3.0-or-later
redistribution terms are recorded beside it. Restricted Annex examples are
never copied into `tests/fixtures/`.

## Development

Requires Rust `1.85` or newer.

```bash
git clone https://github.com/openbimrs/dt.git
cd dt
./scripts/gate.sh
python -m pip install -r docs/requirements.txt
./scripts/build-docs.sh
```

The gate builds every target, runs tests and Clippy, generates rustdoc, executes
mutation probes, packages the crate, and fails closed on restricted-material
leakage.

## Contributing

See [`CONTRIBUTING.md`](https://github.com/openbimrs/dt/blob/main/CONTRIBUTING.md).
Capability claims require executable evidence and must distinguish structural
checks from full schema or standard conformance.

## License

AGPL-3.0-or-later — see [`LICENSE`](https://github.com/openbimrs/dt/blob/main/LICENSE).
