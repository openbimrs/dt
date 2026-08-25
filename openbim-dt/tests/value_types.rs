use std::str::FromStr;

use openbim_dt::{
    AnyUri, Base, Concept, DataTypeName, DateTime, Decimal, Guid, Language, MultiLanguageText,
    PositiveInteger, Rational, Reference, Scale,
};

#[test]
fn guid_preserves_lexical_form_after_validation() {
    let upper = "ABCDEF12-3456-7890-ABCD-EF1234567890";
    let guid = Guid::from_str(upper).expect("valid GUID");
    assert_eq!(guid.as_str(), upper);
    assert_eq!(guid.to_string(), upper);

    for invalid in [
        "",
        "abcdef12-3456-7890-abcd-ef123456789",
        "abcdef123456-7890-abcd-ef1234567890",
        "zbcdef12-3456-7890-abcd-ef1234567890",
    ] {
        assert!(Guid::from_str(invalid).is_err(), "accepted {invalid:?}");
    }
}

#[test]
fn standard_value_types_are_forward_compatible() {
    assert_eq!(DataTypeName::from("STRING"), DataTypeName::String);
    assert_eq!(
        DataTypeName::from("FUTURE"),
        DataTypeName::Other("FUTURE".into())
    );
    assert_eq!(DataTypeName::String.as_str(), "STRING");

    assert_eq!(Scale::from("LINEAR"), Scale::Linear);
    assert_eq!(Scale::from("CURVED"), Scale::Other("CURVED".into()));
    assert_eq!(Base::from("PI"), Base::Pi);
    assert_eq!(Base::from("TAU"), Base::Other("TAU".into()));
}

#[test]
fn rational_validation_keeps_source_lexeme() {
    for valid in ["0", "-3", "+12", "2/3", "-10/7"] {
        let value = Rational::from_str(valid).expect("valid rational");
        assert_eq!(value.as_str(), valid);
    }
    for invalid in ["", "1/0", "1/02", "1.0", "--1", "1/"] {
        assert!(Rational::from_str(invalid).is_err(), "accepted {invalid:?}");
    }
}

#[test]
fn decimal_and_language_contracts_match_xml_schema_lexical_spaces() {
    for (source, normalized) in [
        ("0", "0"),
        ("-3", "-3"),
        ("+12.50", "+12.50"),
        (".5", ".5"),
        ("5.", "5."),
        (" \t 1.0\n", "1.0"),
    ] {
        assert_eq!(Decimal::from_str(source).unwrap().as_str(), normalized);
    }
    for invalid in ["", ".", "1e3", "--1", "1.2.3"] {
        assert!(Decimal::from_str(invalid).is_err(), "accepted {invalid:?}");
    }
    assert_eq!(Language::from_str(" en-US\n").unwrap().as_str(), "en-US");
    assert!(Language::from_str("de CH").is_err());

    assert_eq!(
        PositiveInteger::from_str(" +004 ").unwrap().as_str(),
        "+004"
    );
    assert!(PositiveInteger::from_str("0").is_err());
    assert!(PositiveInteger::from_str("-1").is_err());
}

#[test]
fn multilingual_text_and_references_are_dt_owned_contracts() {
    let text = MultiLanguageText::new("de", "Feuerwiderstand").unwrap();
    assert_eq!(text.language(), "de");
    assert_eq!(text.text(), "Feuerwiderstand");

    let guid = Guid::from_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
    let reference = Reference::new(
        Some(guid.clone()),
        Some(AnyUri::from_str(" urn:test:property ").unwrap()),
    );
    assert_eq!(reference.guid(), Some(&guid));
    assert_eq!(reference.uri(), Some("urn:test:property"));
    assert!(Reference::new(None, None).is_empty());
    assert!(Reference::identified(None, None).is_err());
    assert_eq!(
        AnyUri::from_str(" https://example.com/café path ")
            .unwrap()
            .as_str(),
        "https://example.com/café path"
    );
    assert_eq!(AnyUri::from_str("a[b]").unwrap().as_str(), "a[b]");
    assert!(AnyUri::from_str("a\0b").is_err());
}

#[test]
fn concept_contract_is_reusable_by_standards_that_extend_concept_type() {
    let guid = Guid::from_str("44444444-4444-4444-4444-444444444444").unwrap();
    let mut concept = Concept::new(
        guid.clone(),
        DateTime::from_str(" 2026-08-25T00:00:00Z ").unwrap(),
        MultiLanguageText::new("en", "Fire safety purpose").unwrap(),
        MultiLanguageText::new("en", "Synthetic definition").unwrap(),
    );
    concept.add_reference(Reference::new(Some(guid), None));

    assert_eq!(
        concept.guid().as_str(),
        "44444444-4444-4444-4444-444444444444"
    );
    assert_eq!(concept.names()[0].text(), "Fire safety purpose");
    assert_eq!(concept.definition().text(), "Synthetic definition");
    assert_eq!(concept.references().len(), 1);
    assert_eq!(concept.date_of_creation(), "2026-08-25T00:00:00Z");
    assert!(DateTime::from_str("not-a-date").is_err());
    assert!(DateTime::from_str("02024-01-01T00:00:00Z").is_err());
    assert!(DateTime::from_str("2024-01-01T24:00:00.0Z").is_ok());
    assert!(DateTime::from_str("2024-01-01T24:00:00.001Z").is_err());
}
