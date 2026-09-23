// Maximal owned-type fixtures: every declared child kind populated, most
// twice. Shared by tests/owned_codec.rs and the codec drift unit test via
// include!, so both check the same values. Public API only.

fn guid(seed: u32) -> Guid {
    Guid::from_str(&format!("{seed:08x}-0000-4000-8000-000000000000")).unwrap()
}

fn text(value: &str) -> MultiLanguageText {
    MultiLanguageText::new("en", value).unwrap()
}

fn reference(seed: u32) -> Reference {
    Reference::new(
        Some(guid(seed)),
        Some(AnyUri::from_str(&format!("urn:x:{seed}")).unwrap()),
    )
}

fn created() -> DateTime {
    DateTime::from_str("2026-09-23T00:00:00Z").unwrap()
}

/// A concept with every `ConceptType` child kind populated twice.
fn maximal_concept(seed: u32) -> Concept {
    let mut concept = Concept::new(guid(seed), created(), text("Name A"), text("Def A"));
    concept.set_about(Some(AnyUri::from_str("https://example.org/about").unwrap()));
    concept.add_name(text("Name B"));
    concept.add_definition(text("Def B"));
    for n in 0..2 {
        concept.add_description(text(&format!("Description {n}")));
        concept.add_example(text(&format!("Example {n}")));
        concept.add_reference_document_ref(reference(seed + 10 + n));
        concept.add_similar_to_ref(reference(seed + 20 + n));
        concept.add_replaced_objects_ref(reference(seed + 30 + n));
        concept.add_dictionary_ref(reference(seed + 40 + n));
        concept.add_language_of_creator(Language::from_str(["en", "de-CH"][n as usize]).unwrap());
        concept.add_country_of_origin(["CH", "DE"][n as usize].to_owned());
        concept.add_visual_representation(
            Base64Binary::from_str(["Zm9v", "YmFy"][n as usize]).unwrap(),
        );
        concept.add_major_version(NonNegativeInteger::from_str(["1", "007"][n as usize]).unwrap());
        concept.add_minor_version(NonNegativeInteger::from_str(["0", "12"][n as usize]).unwrap());
        concept.add_status(["draft", "active"][n as usize].to_owned());
        concept.add_deprecation_explanation(format!("Reason {n}"));
    }
    concept
}

fn maximal_subject(seed: u32) -> Subject {
    let mut subject = Subject::new(maximal_concept(seed));
    subject.add_has_part_ref(reference(seed + 50));
    subject.add_has_part_ref(reference(seed + 51));
    subject.add_is_subtype_of_ref(reference(seed + 60));
    subject.add_is_subtype_of_ref(reference(seed + 61));
    subject
}

fn maximal_data_type() -> DataType {
    let mut data_type = DataType::new(Some(DataTypeName::Real));
    data_type.add_constraint(DataTypeConstraint::MinInclusive("0".into()));
    data_type.add_constraint(DataTypeConstraint::MinExclusive("-1".into()));
    data_type.add_constraint(DataTypeConstraint::MaxInclusive("100".into()));
    data_type.add_constraint(DataTypeConstraint::MaxExclusive("101".into()));
    data_type.add_data_format("[0-9]+");
    data_type.add_data_format("[a-z]+");
    for language in ["en", "de"] {
        let mut list = ValueList::new(
            Language::from_str(language).unwrap(),
            DataValue::new("a", Some(1)),
        );
        list.add_value(DataValue::new("b", None));
        data_type.add_possible_values(list);
    }
    data_type
}

fn maximal_property() -> Property {
    let mut property = Property::new(maximal_concept(0x100), maximal_data_type());
    for n in 0..2 {
        property.add_symbol(format!("s{n}"));
        property.add_dimension_ref(reference(0x110 + n));
        property.add_unit_ref(reference(0x120 + n));
        property.add_quantity_kind_ref(reference(0x130 + n));
        property.add_dependency_ref(reference(0x140 + n));
    }
    property.set_specialization_ref(Some(reference(0x150)));
    property
}

fn maximal_group() -> GroupOfProperties {
    let mut group = GroupOfProperties::new(maximal_subject(0x200), reference(0x210));
    group.add_property_ref(reference(0x211));
    group
}

fn maximal_template() -> DataTemplate {
    let mut template = DataTemplate::new(maximal_subject(0x300));
    for n in 0..2 {
        template.add_object_type_ref(reference(0x310 + n));
        template.add_property_ref(reference(0x320 + n));
        template.add_group_of_properties_ref(reference(0x330 + n));
    }
    template
}

fn maximal_quantity_kind() -> QuantityKind {
    QuantityKind::new(maximal_concept(0x400), reference(0x410))
}

fn maximal_dimension() -> Dimension {
    let exponents =
        ["1", "0", "-2", "0.5", "0", "0", "1"].map(|value| Decimal::from_str(value).unwrap());
    Dimension::new(maximal_concept(0x500), exponents)
}

fn maximal_unit() -> Unit {
    let mut unit = Unit::new(
        maximal_concept(0x600),
        reference(0x610),
        Scale::Logarithmic,
        Base::Ten,
        Rational::from_str("-3/4").unwrap(),
        Rational::from_str("+5").unwrap(),
    );
    unit.add_symbol(text("dB"));
    unit.add_symbol(MultiLanguageText::new("de", "dB").unwrap());
    unit
}

fn maximal_reference_document() -> ReferenceDocument {
    let mut document =
        ReferenceDocument::new(maximal_concept(0x700), Language::from_str("en").unwrap());
    document.add_language(Language::from_str("fr").unwrap());
    document.set_date_of_publication(Some(DateTime::from_str("2020-01-02T03:04:05Z").unwrap()));
    document.set_author(Some("A. Author".into()));
    document.set_isbn(Some("978-3-16-148410-0".into()));
    document.set_publisher(Some("Publisher".into()));
    document.set_uri(Some(AnyUri::from_str("https://example.org/doc").unwrap()));
    document
}
