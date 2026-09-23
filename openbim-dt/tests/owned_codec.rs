//! Owned-type codec evidence: maximal round trips, schema validity of the
//! written XML, and a drift guard that reads the generated schema tables.

use std::str::FromStr;

use openbim_dt::{
    AnyUri, Base, Base64Binary, Concept, DataTemplate, DataType, DataTypeConstraint, DataTypeName,
    DataValue, DateTime, Decimal, Dimension, Document, Element, GroupOfProperties, Guid, Language,
    MultiLanguageText, NonNegativeInteger, ObjectType, Property, QuantityKind, Rational, Reference,
    ReferenceDocument, Scale, Subject, Unit, ValueList,
};

include!("support/maximal.rs");

/// Validates owned output through the public schema validator. Every owned
/// kind is a declared `Library` child, so wrapping it in a Library checks it
/// against its own type definition, including kinds that are not global roots.
fn assert_schema_valid(element: Element) {
    let library = Element::new(Some(openbim_dt::NAMESPACE), Some("dt"), "Library")
        .with_attribute(openbim_dt::Attribute::new(
            Some("dt"),
            "GUID",
            Some(openbim_dt::NAMESPACE),
            "ffffffff-0000-4000-8000-000000000000",
        ))
        .with_child(element);
    let document = Document::standalone(library);
    let xml = document.to_xml_string().expect("owned output serializes");
    let reparsed = Document::parse(&xml).expect("owned output reparses");
    let report = reparsed.validate_schema();
    assert!(report.is_conforming(), "{:#?}\n{xml}", report.violations());
}

macro_rules! round_trip {
    ($name:ident, $ty:ty, $value:expr) => {
        #[test]
        fn $name() {
            let value: $ty = $value;
            let element = value.to_element();
            let decoded = <$ty>::from_element(&element).expect("decodes its own output");
            assert_eq!(decoded, value);
            assert_schema_valid(element);
        }
    };
}

round_trip!(
    object_type_round_trips_and_validates,
    ObjectType,
    ObjectType::new(maximal_subject(0x800))
);
round_trip!(
    property_round_trips_and_validates,
    Property,
    maximal_property()
);
round_trip!(
    group_round_trips_and_validates,
    GroupOfProperties,
    maximal_group()
);
round_trip!(
    template_round_trips_and_validates,
    DataTemplate,
    maximal_template()
);
round_trip!(
    quantity_kind_round_trips_and_validates,
    QuantityKind,
    maximal_quantity_kind()
);
round_trip!(
    dimension_round_trips_and_validates,
    Dimension,
    maximal_dimension()
);
round_trip!(unit_round_trips_and_validates, Unit, maximal_unit());
round_trip!(
    reference_document_round_trips_and_validates,
    ReferenceDocument,
    maximal_reference_document()
);
