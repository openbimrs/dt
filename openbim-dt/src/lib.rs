//! `openbim-dt` — ISO 23387 data templates.
//!
//! # What this is
//!
//! The concept vocabulary that describes *properties themselves*: property
//! definitions, groups of properties, quantity kinds, dimensions, units,
//! object types, and the reference machinery binding them to external
//! dictionaries such as bSDD.
//!
//! # Why it is a crate and not part of `openbim-loin`
//!
//! LOIN does not own ISO 23387. The ISO 7817-3 schema imports the ISO 23387
//! namespace and uses its types. Data-template tooling and dictionary clients
//! also need the vocabulary without acquiring a LOIN dependency.
//!
//! The dependency direction is therefore `openbim-loin` to `openbim-dt`, never
//! the reverse.
//!
//! # Status
//!
//! The crate implements validated lexical contracts, a bounded namespace-aware
//! parser, semantic XML round trips with unknown-content retention, owned type
//! contracts and typed global/local DT element views, diagnostics, and a CLI.
//!
//! It does not claim XML Schema or clause-level ISO conformance, byte-identical
//! output, ISO 23386 governance workflows, or ISO 12006-3 mapping. Parsing and
//! validation are deliberately separate operations.
//!
//! The ISO XSD is **not vendored** here. Redistribution rights do not follow
//! from possessing a standards document or annex, so restricted references
//! remain local and outside Git, crate packages, and documentation artifacts.

#![forbid(unsafe_code)]

mod document;
mod domain;
mod model;
mod parser;
mod value;

pub use document::{Attribute, Document, Element, Node, WriteError, XmlDeclaration};
pub use domain::{
    DataTemplate, DataType, DataTypeConstraint, DataValue, Dimension, GroupOfProperties,
    ObjectType, Property, QuantityKind, ReferenceDocument, Subject, Unit, ValueList,
};
pub use model::{
    ConceptRef, DataTemplateElement, DataTemplateRef, DataTypeRef, Diagnostic, DiagnosticCode,
    DimensionElement, ElementKind, GroupOfPropertiesElement, Library, LibraryElement, LibraryItem,
    ModelError, MultilingualTextRef, ObjectTypeElement, PropertyElement, PropertyRef,
    QuantityKindElement, ReferenceDocumentElement, ReferenceRef, Severity, SubjectElement,
    UnitElement,
};
pub use parser::{ParseError, ParseErrorKind, ParseOptions};
pub use value::{
    AnyUri, Base, Concept, DataTypeName, DateTime, Decimal, Guid, Language, MultiLanguageText,
    PositiveInteger, Rational, Reference, Scale, ValueError, ValueErrorKind,
};

/// The XML namespace declared by ISO 23387 edition 2.
///
/// Readers retain this identity rather than treating draft namespaces as the
/// edition 2 standard.
pub const NAMESPACE: &str = "https://standards.iso.org/iso/23387/ed-2/en/";

/// A placeholder namespace found in pre-release ISO 23387 schema drafts.
///
/// It is named so future readers can produce a specific non-conformance
/// diagnostic instead of silently treating draft documents as edition 2.
pub const DRAFT_PLACEHOLDER_NAMESPACE: &str = "http://tempuri.org/XMLSchema.xsd";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_identities_match_the_documented_contracts() {
        assert_eq!(NAMESPACE, "https://standards.iso.org/iso/23387/ed-2/en/");
        assert_eq!(
            DRAFT_PLACEHOLDER_NAMESPACE,
            "http://tempuri.org/XMLSchema.xsd"
        );
        assert_ne!(NAMESPACE, DRAFT_PLACEHOLDER_NAMESPACE);
    }
}
