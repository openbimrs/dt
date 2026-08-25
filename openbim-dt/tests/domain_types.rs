use std::str::FromStr;

use openbim_dt::{
    Base, Concept, DataTemplate, DataType, DataTypeConstraint, DataTypeName, DataValue, DateTime,
    Decimal, Dimension, GroupOfProperties, Language, MultiLanguageText, ObjectType, Property,
    QuantityKind, Rational, Reference, ReferenceDocument, Scale, Subject, Unit, ValueList,
};

fn concept(id: &str) -> Concept {
    Concept::new(
        id.parse().unwrap(),
        DateTime::from_str("2026-08-25T00:00:00Z").unwrap(),
        MultiLanguageText::new("en", "Synthetic").unwrap(),
        MultiLanguageText::new("en", "Synthetic definition").unwrap(),
    )
}

fn reference(id: &str) -> Reference {
    Reference::new(Some(id.parse().unwrap()), None)
}

#[test]
fn owned_type_contracts_enforce_annex_e_required_content() {
    let dimension_ref = reference("10000000-0000-0000-0000-000000000000");
    let property_ref = reference("20000000-0000-0000-0000-000000000000");
    let subject = Subject::new(concept("30000000-0000-0000-0000-000000000000"));
    let object = ObjectType::new(subject.clone());
    assert_eq!(object.subject(), &subject);

    let mut data_type = DataType::new(Some(DataTypeName::String));
    let mut values = ValueList::new(
        Language::from_str("en").unwrap(),
        DataValue::new("30 min", Some(i32::MIN)),
    );
    values.add_value(DataValue::new("60 min", Some(i32::MAX)));
    data_type.add_possible_values(values);
    data_type.add_data_format("[A-Z]{3}");
    data_type.add_data_format("[0-9]+");
    data_type.add_constraint(DataTypeConstraint::MinInclusive("1".to_owned()));
    let mut property = Property::new(concept("40000000-0000-0000-0000-000000000000"), data_type);
    property.add_unit_ref(dimension_ref.clone());
    assert_eq!(property.unit_refs(), &[dimension_ref.clone()]);
    let possible_values = &property.data_type().possible_values()[0];
    assert_eq!(possible_values.language().as_str(), "en");
    assert_eq!(possible_values.values().len(), 2);
    assert_eq!(possible_values.values()[0].order(), Some(i32::MIN));
    assert_eq!(possible_values.values()[1].order(), Some(i32::MAX));
    assert_eq!(property.data_type().data_formats().len(), 2);
    assert_eq!(property.data_type().data_formats()[0], "[A-Z]{3}");
    assert_eq!(property.data_type().data_formats()[1], "[0-9]+");

    let quantity = QuantityKind::new(
        concept("50000000-0000-0000-0000-000000000000"),
        dimension_ref.clone(),
    );
    assert_eq!(quantity.dimension_ref(), &dimension_ref);

    let group = GroupOfProperties::new(subject.clone(), property_ref.clone());
    assert_eq!(group.property_refs(), &[property_ref]);

    let mut template = DataTemplate::new(subject);
    template.set_object_type_ref(Some(dimension_ref));
    assert!(template.object_type_ref().is_some());
}

#[test]
fn scalar_heavy_types_retain_validated_dt_values() {
    let zero = Decimal::from_str("0.0").unwrap();
    let dimension = Dimension::new(
        concept("60000000-0000-0000-0000-000000000000"),
        [
            zero.clone(),
            zero.clone(),
            zero.clone(),
            zero.clone(),
            zero.clone(),
            zero.clone(),
            zero,
        ],
    );
    assert_eq!(dimension.exponents()[0].as_str(), "0.0");
    assert!(Decimal::from_str("1e3").is_err());

    let mut unit = Unit::new(
        concept("70000000-0000-0000-0000-000000000000"),
        reference("80000000-0000-0000-0000-000000000000"),
        Scale::Linear,
        Base::Ten,
        Rational::from_str("1").unwrap(),
        Rational::from_str("0").unwrap(),
    );
    unit.add_symbol(MultiLanguageText::new("en", "m").unwrap());
    assert_eq!(unit.symbols()[0].text(), "m");

    let mut document = ReferenceDocument::new(
        concept("90000000-0000-0000-0000-000000000000"),
        Language::from_str(" en ").unwrap(),
    );
    document.add_language(Language::from_str("de").unwrap());
    assert_eq!(document.languages()[0].as_str(), "en");
    assert_eq!(document.languages()[1].as_str(), "de");
}
