# openbim-dt plan

Updated: 2026-08-25

## Active milestone: `DT-MODEL-CODEC`

Implement the ISO 23387 edition 2 exchange vocabulary as a real, independently
usable crate before wiring it into LOIN.

### Scope

- [x] Validated XML Schema value contracts for GUIDs, date-times, languages,
  decimals, any-URIs, multilingual text, references, concepts, subjects,
  properties, units, dimensions, quantity kinds, reference documents, object
  types, groups, data templates, and libraries.
- [x] Namespace-aware XML reader/writer that retains unknown elements,
  attributes, comments, processing instructions, CDATA, and document order.
- [x] Configurable input/depth/node limits and explicit rejection of DTDs.
- [x] Structured diagnostics kept separate from parsing; validation must not
  silently normalize source values.
- [x] Original synthetic positive/negative fixtures and mutation-verified
  namespace, identity, reference, and limit gates.
- [x] Capability documentation.
- [ ] Publish `openbim-dt 0.2.0` for direct LOIN use after immutable review.

### Out of scope

- ISO 23386 governance workflows.
- ISO 12006-3 RDF/ontology mapping.
- Bundled ISO/DIN/CEN schemas or copied Annex examples.
- Claims of full standard conformance without independently redistributable
  conformance material.

## Follow-up milestones

1. Integrate ISO 23386 governance concepts behind explicit contracts.
2. Add dictionary and ontology adapters without making them core dependencies.
3. Expand conformance evidence when redistributable fixtures become available.

Every milestone requires public or original redistributable fixtures, positive
and negative tests, and an updated capability table.
