use openbim_dt::{
    DiagnosticCode, Document, ElementKind, LibraryItem, ParseErrorKind, ParseOptions,
};

const FIXTURE: &str = include_str!("fixtures/synthetic-library.xml");

#[test]
fn document_round_trip_preserves_all_xml_semantics() {
    let first = Document::parse(FIXTURE).expect("parse synthetic fixture");
    let xml = first.to_xml_string().expect("serialize fixture");
    let second = Document::parse(&xml).expect("reparse serialized fixture");

    assert_eq!(first, second);
    assert!(xml.contains("<!-- ordering and comments must survive -->"));
    assert!(xml.contains("<?fixture mode=\"roundtrip\"?>"));
    assert!(xml.contains("<![CDATA[Required <duration>]]>"));
    assert!(xml.contains("vendor:revision=\"kept\""));
    assert!(xml.contains("<vendor:Extension"));
}

#[test]
fn text_entities_are_decoded_for_callers_and_escaped_on_write() {
    let xml = r#"<dt:Library xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/" dt:GUID="11111111-1111-1111-1111-111111111111"><dt:Name language="en">A &amp; B &lt; C</dt:Name></dt:Library>"#;
    let document = Document::parse(xml).unwrap();
    let name = match document.library().unwrap().items().next().unwrap() {
        LibraryItem::Name(value) => value,
        _ => panic!("expected name"),
    };
    assert_eq!(name.text(), "A & B < C");

    let output = document.to_xml_string().unwrap();
    assert!(output.contains("A &amp; B &lt; C"));
    assert_eq!(Document::parse(&output).unwrap(), document);
}

#[test]
fn control_character_references_survive_repeated_round_trips() {
    let xml = r#"<dt:Library xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/" dt:GUID="11111111-1111-1111-1111-111111111111" dt:about="a&#x9;b&#xA;c&#xD;d">x&#xD;y</dt:Library>"#;
    let first = Document::parse(xml).unwrap();
    let value = first
        .root()
        .attributes()
        .iter()
        .find(|attribute| attribute.local_name() == "about")
        .unwrap()
        .value();
    assert_eq!(value, "a\tb\nc\rd");

    let first_output = first.to_xml_string().unwrap();
    assert!(first_output.contains("a&#x9;b&#xA;c&#xD;d"));
    assert!(first_output.contains(">x&#13;y</dt:Library>"));
    let second = Document::parse(&first_output).unwrap();
    assert_eq!(second, first);
    assert_eq!(second.to_xml_string().unwrap(), first_output);

    let literal_whitespace = Document::parse(
        "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\" dt:about=\"a\tb\nc\rd\"/>",
    )
    .unwrap();
    assert_eq!(
        literal_whitespace
            .root()
            .attributes()
            .iter()
            .find(|attribute| attribute.local_name() == "about")
            .unwrap()
            .value(),
        "a b c d"
    );
}

#[test]
fn library_view_exposes_typed_items_without_discarding_extensions() {
    let document = Document::parse(FIXTURE).unwrap();
    assert_eq!(document.root_kind(), Some(ElementKind::Library));
    let library = document.library().expect("library root");
    assert_eq!(
        library.guid().unwrap().as_str(),
        "11111111-1111-1111-1111-111111111111"
    );

    let items: Vec<_> = library.items().collect();
    assert!(matches!(items[0], LibraryItem::Name(_)));
    let property = items
        .iter()
        .find_map(|item| match item {
            LibraryItem::Property(value) => Some(*value),
            _ => None,
        })
        .expect("property");
    assert_eq!(property.names().next().unwrap().text(), "Fire rating");
    assert_eq!(property.data_type().unwrap().name(), Some("STRING"));
    assert_eq!(
        property.data_type().unwrap().name_kind(),
        Some(openbim_dt::DataTypeName::String)
    );
    assert!(property
        .element()
        .children()
        .any(|child| child.namespace_uri() == Some("urn:openbim-dt:test:vendor")));

    let template = items
        .iter()
        .find_map(|item| match item {
            LibraryItem::DataTemplate(value) => Some(*value),
            _ => None,
        })
        .expect("data template");
    assert_eq!(template.property_references().count(), 1);
}

#[test]
fn validation_is_structured_and_separate_from_parsing() {
    let malformed_semantics = r#"<dt:Library xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/" dt:GUID="bad">
      <dt:Property dt:GUID="bad" dateOfCreation="2026-08-25T00:00:00Z">
        <dt:Name language="bad_tag">broken language tag</dt:Name>
        <dt:DataType name="FUTURE"/>
        <dt:DataType/>
        <dt:UnitRef/>
        <dt:UnitRef dt:GUID="also-bad"/>
      </dt:Property>
      <dt:Property dt:GUID="bad" dateOfCreation="2026-08-25T00:00:00Z"/>
    </dt:Library>"#;
    let document = Document::parse(malformed_semantics).expect("well-formed XML still parses");
    let diagnostics = document.validate();
    assert!(diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::InvalidGuid));
    assert!(diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::DuplicateGuid));
    assert!(diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::MissingName));
    assert!(diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::UnknownDataType));
    assert!(diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::MissingDataTypeName));
    assert!(diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::InvalidLanguage));
    assert!(diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::EmptyReference));
}

#[test]
fn parser_rejects_dtds_and_enforces_resource_limits() {
    let simple_dtd = r#"<!DOCTYPE Library><dt:Library xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/" dt:GUID="11111111-1111-1111-1111-111111111111"/>"#;
    assert_eq!(
        Document::parse(simple_dtd).unwrap_err().kind(),
        ParseErrorKind::DoctypeForbidden
    );

    let dtd = r#"<!DOCTYPE Library [<!ENTITY x "boom">]><Library>&x;</Library>"#;
    let error = Document::parse(dtd).unwrap_err();
    assert_eq!(error.kind(), ParseErrorKind::DoctypeForbidden);

    let options = ParseOptions {
        max_bytes: 64,
        ..ParseOptions::default()
    };
    assert_eq!(
        Document::parse_with_options(FIXTURE, options)
            .unwrap_err()
            .kind(),
        ParseErrorKind::InputTooLarge
    );

    let options = ParseOptions {
        max_depth: 2,
        ..ParseOptions::default()
    };
    let nested = r#"<dt:Library xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/" dt:GUID="11111111-1111-1111-1111-111111111111"><dt:DataTemplate dt:GUID="22222222-2222-2222-2222-222222222222" dateOfCreation="x"><dt:Name language="en">x</dt:Name></dt:DataTemplate></dt:Library>"#;
    assert_eq!(
        Document::parse_with_options(nested, options)
            .unwrap_err()
            .kind(),
        ParseErrorKind::DepthLimit
    );

    let options = ParseOptions {
        max_nodes: 1,
        ..ParseOptions::default()
    };
    assert_eq!(
        Document::parse_with_options(FIXTURE, options)
            .unwrap_err()
            .kind(),
        ParseErrorKind::NodeLimit
    );

    let options = ParseOptions {
        max_attributes_per_element: 1,
        ..ParseOptions::default()
    };
    assert_eq!(
        Document::parse_with_options(FIXTURE, options)
            .unwrap_err()
            .kind(),
        ParseErrorKind::AttributeLimit
    );
}

#[test]
fn wrong_namespace_is_parseable_but_not_a_dt_root() {
    let document = Document::parse("<Library xmlns=\"urn:not-dt\"/>").unwrap();
    assert_eq!(document.root_kind(), None);
    assert!(document
        .validate()
        .iter()
        .any(|d| d.code == DiagnosticCode::WrongRootNamespace));
}

#[test]
fn undeclared_entities_are_rejected_without_entity_expansion() {
    let xml = r#"<dt:Library xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/" dt:GUID="11111111-1111-1111-1111-111111111111">&private;</dt:Library>"#;
    assert_eq!(
        Document::parse(xml).unwrap_err().kind(),
        ParseErrorKind::UnknownEntity
    );
}

#[test]
fn undeclared_namespace_prefixes_are_rejected() {
    let error = Document::parse("<dt:Library/>").unwrap_err();
    assert_eq!(error.kind(), ParseErrorKind::UndeclaredPrefix);

    let malformed = format!("<dt:bad:name xmlns:dt=\"{}\"/>", openbim_dt::NAMESPACE);
    assert_eq!(
        Document::parse(&malformed).unwrap_err().kind(),
        ParseErrorKind::MalformedQName
    );
}

#[test]
fn draft_namespace_gets_a_specific_diagnostic() {
    let xml = r#"<dt:Library xmlns:dt="http://tempuri.org/XMLSchema.xsd" dt:GUID="11111111-1111-1111-1111-111111111111"/>"#;
    assert!(Document::parse(xml)
        .unwrap()
        .validate()
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::DraftNamespace));
}
