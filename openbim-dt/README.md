# openbim-dt

Pure-Rust ISO 23387 edition 2 data-template contracts and a bounded,
namespace-aware XML codec.

`openbim-dt` is the lower-level vocabulary consumed by LOIN and independent
data-template tooling. It does not depend on LOIN.

## Implemented

- validated GUID, date-time, language/decimal, any-URI/reference,
  multilingual-text, rational, unit, data-type/value-list, and reusable
  `ConceptType` value contracts;
- owned subject, object-type, property, group, quantity-kind,
  reference-document, dimension, unit, and data-template contracts;
- a bounded strict XML 1.0 parser that rejects malformed namespace/content
  constructs, DTDs, and undeclared entities;
- semantic XML round trips retaining namespaces, ordering, comments, processing
  instructions, CDATA, and unknown attributes/elements;
- typed borrowed views and owned wrappers for the five concrete global roots and
  every local `Library` child family represented by the edition 2 XSD;
- structured built-in diagnostics separate from parsing;
- `inspect`, `validate`, and `rewrite` CLI commands.

This is not an XSD validator, complete clause-level conformance engine, ISO 23386
governance workflow, or ISO 12006-3 mapper. Byte-identical XML output is not
claimed.

```rust
use openbim_dt::{Document, ElementKind};

let xml = r#"<dt:Library
  xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/"
  dt:GUID="11111111-1111-1111-1111-111111111111"/>"#;
let document = Document::parse(xml)?;
assert_eq!(document.root_kind(), Some(ElementKind::Library));
let output = document.to_xml_string()?;
assert_eq!(Document::parse(&output)?, document);
# Ok::<(), Box<dyn std::error::Error>>(())
```

See the [repository README](https://github.com/openbimrs/dt#readme),
[architecture](https://openbimrs.github.io/dt/architecture/), and
[API documentation](https://docs.rs/openbim-dt) for exact capability boundaries.

No ISO/DIN/CEN document, XSD, or annex example is distributed in this package.
The included XML fixture is original MIT-licensed synthetic test material.

## License

MIT — see [LICENSE](https://github.com/openbimrs/dt/blob/main/LICENSE).
