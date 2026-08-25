# Architecture

## Repository role

`openbimrs/dt` is the canonical source repository for ISO 23387 data-template
contracts. `openbimrs/openbim` integrates a verified revision as the
`packages/dt` submodule and exposes it through a feature-gated facade.

The child repository builds independently. Published crates therefore use
registry dependencies rather than paths into sibling repositories.

## Dependency direction

```text
roxmltree + quick-xml  <-  openbim-dt  <-  openbim-loin / dictionary clients
                                           ^
openbim facade  ---------------+
```

- DT owns ISO 23387 contracts and format policy.
- LOIN and dictionary clients consume released DT contracts.
- DT never depends on LOIN or another higher-level document format.
- XML plumbing is format-specific code: `roxmltree` enforces strict XML 1.0
  well-formedness and namespaces before `quick-xml` builds the retention tree.
  There is no project-wide codec abstraction.

## Implemented layers

### Lexical contracts

`Guid`, `DateTime`, `Language`, `Decimal`, `AnyUri`, `MultiLanguageText`,
`Reference`, `Rational`, `DataTypeName`, `Scale`, `Base`, and the reusable owned
`Concept` core represent contracts imported by other standards. XML Schema
whitespace collapsing is applied by the value types that require it.
`AnyUri` follows the broad XML Schema 1.0 lexical space, retaining Unicode and
spaces that the schema-defined escaping procedure maps to a URI.
Forward-compatible enums retain unknown values instead of normalizing or
rejecting future vocabulary.

### Lossless semantic XML tree

`Document`, `Element`, `Attribute`, and `Node` retain:

- resolved namespace identity and original qualified names;
- attribute and child ordering;
- comments, processing instructions, CDATA, and text;
- unknown elements and attributes;
- XML declaration evidence and prolog/epilog nodes.

"Lossless" means that represented XML semantics survive parse/write/parse.
Output is not promised to be byte-identical: entity spelling, quote style, and
other equivalent syntax may be normalized.

The parser enforces configurable byte, depth, node, and attribute budgets.
Inherited namespace bindings use delta scopes and shared URI storage, so a
large ancestor URI is not copied once per nesting level or resolved node.
Malformed XML, duplicate expanded attributes, illegal namespace bindings, DTDs,
invalid characters, and undeclared entities are rejected; the codec never
resolves external entities. XML 1.1 is rejected explicitly rather than being
accepted with incomplete lexical checks.

### Typed DT views

Borrowed views expose standard concepts without copying or discarding the
underlying tree. The five concrete global roots are `Library`, `DataTemplate`,
`ObjectType`, `GroupOfProperties`, and `Property`. `LibraryItem` additionally
recognizes every local edition 2 library-child family:
Subject, DataTemplate, ObjectType, GroupOfProperties, Property, Unit, Dimension,
QuantityKind, and ReferenceDocument. Unknown top-level content remains available
as an extension element.

Owned typed-element wrappers prove the family of embedded DT elements at
tree-integration boundaries. Standards such as LOIN that declare local elements
using DT-owned XSD types consume the owned DT value/domain contracts instead;
they do not pretend those local elements are global DT XML elements.

### Owned complex-type contracts

`Subject`, `ObjectType`, `Property`, `GroupOfProperties`, `QuantityKind`,
`ReferenceDocument`, `Dimension`, `Unit`, `DataTemplate`, and `ValueList` provide
nominal, owned ISO 23387 type identity at cross-standard in-memory boundaries.
`ValueList` requires a validated XML Schema `language` and at least one
`DataValue`; further values repeat, and each optional order is bounded by the
XML Schema `int` range. These contracts use the lexical types above and remain
separate from namespace-specific XML element wrappers.
`DataType` represents the four Annex E inclusive/exclusive boundary variants,
repeating arbitrary regular-expression `DataFormat` strings, and possible value
lists.

The owned `Concept` constructor requires a validated creation date, a first
name, and one definition so dependent standards cannot construct the incomplete
semantic core identified by the Annex E declarations. The Annex E XSD places
these declarations inside a repeating `choice`, which weakens their effective
occurrence constraints. The syntax tree therefore retains such XSD-shaped
documents; built-in diagnostics expose the semantic mismatch without pretending
to be an XSD engine.

### Validation

Parsing and validation are separate operations. `Document::validate` reports
structured severity, category, path, and message values for built-in identity,
reference, multilingual-text, data-type, and concept checks.

This validator does **not** load the restricted XSD and does not claim complete
XML Schema or clause-level ISO conformance.

### CLI

The small CLI layer provides:

- `inspect` for root/item/diagnostic summaries;
- `validate` with exit code `2` for semantic findings;
- `rewrite` with OS-random exclusive same-directory temporary output and atomic
  rename.

## Standards and fixture boundary

No ISO/DIN/CEN document, XSD, or annex example is vendored. Local references
stay under ignored `references/`. Public test fixtures must be original or have
explicit compatible redistribution terms; fixture provenance is committed beside
the fixture.

A locally available Annex F example is exercised during private verification but
is never copied into Git, crates.io, rustdoc, or GitHub Pages.

## Verification

The release gate covers formatting, all-target builds, tests, Clippy, rustdoc,
metadata, package contents, and restricted-file leakage. Mutation probes prove
that tests reject removal of strict XML preflight, concrete-root classification,
context-sensitive reference/multilingual mappings, XML Schema whitespace,
date-time edge cases, broad `anyURI` values, shared namespace storage, required
concept state, unknown nested writer output, and safe temporary-file creation.

## Cross-repository delivery

Changes spanning repositories follow dependency order:

1. land and publish `openbim-dt`;
2. update and verify `openbim-loin` against that released version;
3. publish the consumer;
4. update exact superproject submodule pins and facade dependencies;
5. verify recursive public clones before migrating a shared checkout.

Each pin remains a compatibility declaration and rollback point.
