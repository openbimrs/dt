# Roadmap

`openbim-dt` now provides an ISO 23387 edition 2 contract layer, lossless semantic
XML codec, typed views, built-in diagnostics, and CLI. This roadmap distinguishes
that executable scope from schema-complete conformance and governance work.

## Current baseline

Implemented in the `0.2` source line:

- edition 2 and draft-placeholder namespace identities;
- reusable GUID, multilingual text, reference, rational, data-type, unit/base,
  and concept contracts;
- bounded XML parsing and semantic writing with unknown-content retention;
- global DT family views and owned typed wrappers;
- structured built-in validation diagnostics;
- CLI inspection, validation, and rewrite operations;
- synthetic fixture provenance, mutation probes, packaging leakage checks, API
  documentation, and OpenBIM facade integration.

Not implemented:

- full XSD validation or clause-by-clause ISO 23387 conformance;
- complete format-neutral owned representations for every optional ConceptType
  field;
- byte-identical XML output;
- ISO 23386 governance workflows;
- ISO 12006-3 mapping;
- a bSDD adapter.

## Delivery principles

1. **Domain and wire layers stay separate.** XML ordering is retained by the wire
   tree while reusable standard contracts remain application-facing values.
2. **Retain before interpreting.** Unknown attributes, elements, order, and
   namespace evidence survive even when no typed accessor exists.
3. **Parsing is not validation.** Structural decoding and semantic diagnostics
   are separate APIs and capability claims.
4. **Explicit editions and drafts.** Namespace evidence is retained and draft
   inputs are never silently normalized into edition 2.
5. **Evidence-backed coverage.** Claims require original or redistributable
   fixtures, positive/negative tests, and mutation probes.
6. **Dependency direction stays downward.** Consumers may depend on DT; DT never
   depends on LOIN.

## Milestones

### 1. Contract and codec baseline — implemented

- Standard lexical values and reusable concept core.
- Namespace-aware bounded parser and semantic writer.
- Typed views/wrappers for all global edition 2 element families.
- Unknown-content, comments, PI, CDATA, and order retention.

### 2. Built-in diagnostics — implemented subset

Current diagnostics cover namespace/root identity, required/duplicate/invalid
GUIDs, ConceptType creation date/name requirements, reference identity,
multilingual language tags, and data-type names.

Future work should add only diagnostics justified by public evidence and should
keep warning/error policy stable once released.

### 3. Schema-complete model and validation — future

- Measure XSD declaration/field coverage with a machine-readable inventory.
- Complete owned contracts for optional lifecycle and provenance fields.
- Evaluate an XSD validator without vendoring restricted schema material.
- Publish exact edition and clause coverage; never equate parsing with
  conformance.

### 4. ISO 23386 and ISO 12006-3 boundaries — future

- Model governance and dictionary mapping as explicit companion contracts.
- Keep lifecycle policy and mapping provenance out of the wire codec.

### 5. Consumers — active

- Use released DT contracts in LOIN only at ISO 7817-3 fields whose XSD types are
  imported from ISO 23387.
- Evaluate dictionary/bSDD adapters independently.
- Maintain acyclic dependencies and registry-versioned release order.

## Standards material boundary

Purchased or otherwise restricted standards files remain local under
`references/`. Documentation and release pipelines never copy that directory
into Git, crates, fixtures, build artifacts, or GitHub Pages.
