//! ISO 23387 XML Schema validation.
//!
//! Each test pins one schema rule and asserts the specific violation code, so
//! a regression names the rule that broke rather than a count.

use openbim_dt::{Document, SchemaViolationCode};

const NS: &str = "https://standards.iso.org/iso/23387/ed-2/en/";
const GUID: &str = "11111111-1111-1111-1111-111111111111";

fn codes(xml: &str) -> Vec<SchemaViolationCode> {
    let document = Document::parse(xml).expect("fixture must be well-formed XML");
    document
        .validate_schema()
        .violations()
        .iter()
        .map(openbim_dt::SchemaViolation::code)
        .collect()
}

fn assert_conforms(xml: &str) {
    let document = Document::parse(xml).expect("fixture must be well-formed XML");
    let report = document.validate_schema();
    assert!(
        report.is_conforming(),
        "expected a conforming document, got {:?}",
        report.violations()
    );
}

/// A minimally conforming `Property`: both required attributes, one branch
/// from each required choice, and the required `DataType` child.
fn property(body: &str) -> String {
    format!(
        r#"<dt:Property xmlns:dt="{NS}" dt:GUID="{GUID}" dt:dateOfCreation="2026-01-01T00:00:00Z">{body}</dt:Property>"#
    )
}

const VALID_BODY: &str = concat!(
    r#"<dt:Name dt:language="en">Length</dt:Name>"#,
    r#"<dt:DataType name="REAL"><dt:MinInclusive value="0"/></dt:DataType>"#,
    r#"<dt:Symbol>L</dt:Symbol>"#,
);

#[test]
fn baseline_property_conforms() {
    assert_conforms(&property(VALID_BODY));
}

#[test]
fn rejects_a_root_the_schema_does_not_declare_as_global() {
    let xml = format!(r#"<dt:Unit xmlns:dt="{NS}" dt:GUID="{GUID}"/>"#);
    assert!(codes(&xml).contains(&SchemaViolationCode::UnknownRoot));
}

#[test]
fn rejects_a_root_outside_the_iso_23387_namespace() {
    let xml = r#"<Property xmlns="http://example.invalid/other"/>"#;
    assert!(codes(xml).contains(&SchemaViolationCode::ForeignNamespace));
}

#[test]
fn requires_declared_mandatory_attributes() {
    let xml = format!(r#"<dt:Property xmlns:dt="{NS}">{VALID_BODY}</dt:Property>"#);
    let found = codes(&xml);
    assert_eq!(
        found
            .iter()
            .filter(|code| **code == SchemaViolationCode::MissingRequiredAttribute)
            .count(),
        2,
        "both GUID and dateOfCreation are required: {found:?}"
    );
}

#[test]
fn rejects_attributes_the_schema_does_not_declare() {
    let xml = format!(
        r#"<dt:Property xmlns:dt="{NS}" dt:GUID="{GUID}" dt:dateOfCreation="2026-01-01T00:00:00Z" dt:invented="x">{VALID_BODY}</dt:Property>"#
    );
    assert!(codes(&xml).contains(&SchemaViolationCode::UnknownAttribute));
}

#[test]
fn rejects_elements_the_schema_does_not_declare() {
    let body = format!("{VALID_BODY}<dt:Invented/>");
    assert!(codes(&property(&body)).contains(&SchemaViolationCode::UnexpectedElement));
}

#[test]
fn requires_a_branch_from_every_mandatory_choice() {
    // `DataType` alone satisfies neither the descriptive choice nor the
    // Symbol/Dimension/Unit choice.
    let body = r#"<dt:DataType name="REAL"><dt:MinInclusive value="0"/></dt:DataType>"#;
    let found = codes(&property(body));
    assert_eq!(
        found
            .iter()
            .filter(|code| **code == SchemaViolationCode::MissingChoiceBranch)
            .count(),
        2,
        "expected both choice groups to report: {found:?}"
    );
}

#[test]
fn requires_children_whose_min_occurs_is_positive() {
    // `DataType` is minOccurs=1 and absent here.
    let body = concat!(
        r#"<dt:Name dt:language="en">Length</dt:Name>"#,
        r#"<dt:Symbol>L</dt:Symbol>"#,
    );
    assert!(codes(&property(body)).contains(&SchemaViolationCode::MissingRequiredChild));
}

#[test]
fn rejects_more_occurrences_than_max_occurs_allows() {
    // `DataType` is maxOccurs=1.
    let body = concat!(
        r#"<dt:Name dt:language="en">Length</dt:Name>"#,
        r#"<dt:DataType name="REAL"><dt:MinInclusive value="0"/></dt:DataType>"#,
        r#"<dt:DataType name="REAL"><dt:MinInclusive value="0"/></dt:DataType>"#,
        r#"<dt:Symbol>L</dt:Symbol>"#,
    );
    assert!(codes(&property(body)).contains(&SchemaViolationCode::TooManyOccurrences));
}

#[test]
fn rejects_children_that_violate_declared_sequence_order() {
    // `Symbol` is declared after `DataType`, not before it.
    let body = concat!(
        r#"<dt:Name dt:language="en">Length</dt:Name>"#,
        r#"<dt:Symbol>L</dt:Symbol>"#,
        r#"<dt:DataType name="REAL"><dt:MinInclusive value="0"/></dt:DataType>"#,
    );
    assert!(codes(&property(body)).contains(&SchemaViolationCode::OutOfOrderChild));
}

#[test]
fn enforces_the_guid_pattern_facet() {
    let xml = format!(
        r#"<dt:Property xmlns:dt="{NS}" dt:GUID="not-a-guid" dt:dateOfCreation="2026-01-01T00:00:00Z">{VALID_BODY}</dt:Property>"#
    );
    assert!(codes(&xml).contains(&SchemaViolationCode::PatternMismatch));
}

#[test]
fn accepts_every_hex_case_the_guid_pattern_permits() {
    let xml = format!(
        r#"<dt:Property xmlns:dt="{NS}" dt:GUID="AbCdEf01-1111-2222-3333-444455556666" dt:dateOfCreation="2026-01-01T00:00:00Z">{VALID_BODY}</dt:Property>"#
    );
    assert_conforms(&xml);
}

#[test]
fn enforces_enumeration_facets() {
    let body = r#"<dt:Name dt:language="en">L</dt:Name><dt:DataType name="NOT_A_TYPE"><dt:MinInclusive value="0"/></dt:DataType><dt:Symbol>L</dt:Symbol>"#;
    assert!(codes(&property(body)).contains(&SchemaViolationCode::InvalidEnumerationValue));
}

#[test]
fn enforces_datetime_lexical_space() {
    let xml = format!(
        r#"<dt:Property xmlns:dt="{NS}" dt:GUID="{GUID}" dt:dateOfCreation="2026-13-45">{VALID_BODY}</dt:Property>"#
    );
    assert!(codes(&xml).contains(&SchemaViolationCode::InvalidLexicalValue));
}

#[test]
fn rejects_impossible_calendar_dates() {
    // 2026 is not a leap year, so 29 February does not exist.
    let xml = format!(
        r#"<dt:Property xmlns:dt="{NS}" dt:GUID="{GUID}" dt:dateOfCreation="2026-02-29T00:00:00Z">{VALID_BODY}</dt:Property>"#
    );
    assert!(codes(&xml).contains(&SchemaViolationCode::InvalidLexicalValue));
}

#[test]
fn collapses_only_the_four_xml_schema_whitespace_characters() {
    // Tab/newline around an enumerated value collapse away, so it is valid.
    let collapsing = "<dt:Name dt:language=\"en\">L</dt:Name><dt:DataType name=\"\t REAL\n\"><dt:MinInclusive value=\"0\"/></dt:DataType><dt:Symbol>L</dt:Symbol>";
    assert_conforms(&property(collapsing));

    // NBSP is not XML Schema whitespace and must not be collapsed, so the
    // value stays outside the enumeration.
    let nbsp = "<dt:Name dt:language=\"en\">L</dt:Name><dt:DataType name=\"\u{00a0}REAL\"><dt:MinInclusive value=\"0\"/></dt:DataType><dt:Symbol>L</dt:Symbol>";
    assert!(codes(&property(nbsp)).contains(&SchemaViolationCode::InvalidEnumerationValue));
}

#[test]
fn rejects_character_data_in_element_only_content() {
    let body = format!("{VALID_BODY}stray text");
    assert!(codes(&property(&body)).contains(&SchemaViolationCode::UnexpectedText));
}

#[test]
fn ignores_foreign_namespaced_attributes_it_does_not_govern() {
    let xml = format!(
        r#"<dt:Property xmlns:dt="{NS}" xmlns:x="http://example.invalid/x" dt:GUID="{GUID}" dt:dateOfCreation="2026-01-01T00:00:00Z" x:note="ok" xml:lang="en">{VALID_BODY}</dt:Property>"#
    );
    assert_conforms(&xml);
}
