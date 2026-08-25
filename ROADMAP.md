# Roadmap

`openbim-dt` is an honest ISO 23387 namespace scaffold. This roadmap separates
the two implemented namespace constants from future modeling, codecs,
validation, governance, and integrations.

## Current baseline

Implemented and released:

- ISO 23387 edition 2 namespace identity;
- the known pre-release placeholder namespace for future targeted diagnostics;
- standalone Rust gates, package leakage checks, API documentation, and
  OpenBIM facade integration.

Not implemented:

- a complete ISO 23387 domain model;
- XML parsing, writing, schema validation, or lossless round trips;
- semantic validation and structured diagnostics;
- ISO 23386 governance workflows;
- ISO 12006-3 mapping;
- LOIN, bSDD, or other consumer integration.

## Delivery principles

1. **Domain and wire layers stay separate.** XML names and ordering must not
   become the only representation of ISO 23387 concepts.
2. **Lossless before convenient.** Unknown attributes, elements, ordering, and
   namespace evidence must survive before a codec claims lossless round trips.
3. **Parsing is not validation.** Structural decoding and semantic diagnostics
   are separate APIs and capability claims.
4. **Explicit editions and drafts.** Readers retain source namespace/version
   evidence; writers require an explicit supported target.
5. **Evidence-backed coverage.** Claims require redistributable or original
   fixtures, positive/negative tests, and mutation probes.
6. **Dependency direction stays downward.** Consumers may depend on data
   templates; data templates never depend on LOIN.

## Milestones

### 1. Format-neutral domain contracts

- Model concepts, properties, groups, templates, object types, dimensions,
  units, quantity kinds, multilingual text, and reference identities.
- Represent required/optional boundaries and stable identifiers explicitly.
- Preserve extension points and provenance for concepts not yet modeled.

**Exit evidence:** typed contract tests cover cardinality and identity rules
without importing XML-specific policy into domain types.

### 2. Lossless ISO 23387 XML representation

- Implement namespace-aware reading with explicit edition evidence.
- Retain unknown elements, attributes, ordering, and lexical data where required.
- Require an explicit supported namespace when writing.

**Exit evidence:** public or original fixtures round-trip byte-equivalently where
claimed, plus mutation tests for unknown-data retention and namespace handling.

### 3. Structured validation

- Separate XSD structural checks from ISO 23387 semantic diagnostics.
- Return stable paths, identifiers, severities, and source evidence.
- Never silently normalize invalid draft namespaces into edition 2.

**Exit evidence:** every diagnostic has positive, negative, and mutation-tested
coverage.

### 4. ISO 23386 and ISO 12006-3 boundaries

- Model governance and dictionary mapping as explicit companion contracts.
- Keep lifecycle policy and mapping provenance out of the core wire codec.
- Document which clauses and editions each capability covers.

### 5. Consumers

- Add LOIN integration only when LOIN uses released DT types.
- Evaluate dictionary/bSDD adapters independently from the ISO 23387 codec.
- Maintain acyclic dependencies and registry-versioned release order.

## Standards material boundary

Purchased or otherwise restricted standards files remain local under
`references/`. The documentation and release pipelines must never copy that
directory into Git, crates, fixtures, build artifacts, or GitHub Pages.
