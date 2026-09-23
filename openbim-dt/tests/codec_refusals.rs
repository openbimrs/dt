//! Decoding refuses what the owned model cannot hold instead of
//! dropping it: every `CodecErrorKind` has a triggering input here.

use openbim_dt::{CodecErrorKind, Document, ObjectType, Property};

const NS: &str = "https://standards.iso.org/iso/23387/ed-2/en/";
const GUID: &str = "10000000-0000-0000-0000-000000000000";

/// Wraps `body` in an `ObjectType` with valid identity and one subtype ref.
fn object_type(attributes: &str, body: &str) -> String {
    format!(
        concat!(
            "<dt:ObjectType xmlns:dt='{ns}' dt:GUID='{guid}' ",
            "dateOfCreation='2026-09-23T00:00:00Z' {attributes}>",
            "<dt:Name language='en'>Wall</dt:Name>",
            "<dt:IsSubtypeOfRef dt:GUID='{guid}'/>{body}</dt:ObjectType>"
        ),
        ns = NS,
        guid = GUID,
        attributes = attributes,
        body = body,
    )
}

fn decode_object(xml: &str) -> Result<ObjectType, openbim_dt::CodecError> {
    let document = Document::parse(xml).expect("well-formed test input");
    ObjectType::from_element(document.root())
}

fn kind_of(xml: &str) -> CodecErrorKind {
    decode_object(xml).expect_err(xml).kind()
}

#[test]
fn baseline_input_decodes() {
    decode_object(&object_type("", "")).expect("the unmodified input is representable");
}

#[test]
fn unknown_and_misqualified_attributes_are_refused() {
    // Foreign-namespace extension attribute: valid XML, not representable.
    let foreign = object_type("xmlns:v='urn:vendor' v:flag='x'", "");
    assert_eq!(kind_of(&foreign), CodecErrorKind::UnknownAttribute);
    // `about` is a global, qualified attribute; unqualified it is not the same.
    let unqualified = object_type("about='urn:x'", "");
    assert_eq!(kind_of(&unqualified), CodecErrorKind::UnknownAttribute);
    // On a nested reference, too.
    let nested = object_type("", &format!("<dt:HasPartRef dt:GUID='{GUID}' extra='1'/>"));
    assert_eq!(kind_of(&nested), CodecErrorKind::UnknownAttribute);
}

#[test]
fn unknown_children_and_stray_content_are_refused() {
    let extension = object_type("xmlns:v='urn:vendor'", "<v:Extension/>");
    assert_eq!(kind_of(&extension), CodecErrorKind::UnknownChild);
    let text = object_type("", "stray text");
    assert_eq!(kind_of(&text), CodecErrorKind::UnexpectedContent);
    let in_ref = object_type("", "<dt:HasPartRef>text</dt:HasPartRef>");
    assert_eq!(kind_of(&in_ref), CodecErrorKind::UnexpectedContent);
    let nested = object_type(
        "",
        "<dt:Status><dt:Name language='en'>x</dt:Name></dt:Status>",
    );
    assert_eq!(kind_of(&nested), CodecErrorKind::UnexpectedContent);
}

#[test]
fn missing_and_invalid_values_are_refused() {
    let no_date = format!("<dt:ObjectType xmlns:dt='{NS}' dt:GUID='{GUID}'/>");
    assert_eq!(kind_of(&no_date), CodecErrorKind::MissingAttribute);
    let bad_version = object_type("", "<dt:MajorVersion>-1</dt:MajorVersion>");
    assert_eq!(kind_of(&bad_version), CodecErrorKind::InvalidValue);
    let bad_guid = object_type("", "<dt:HasPartRef dt:GUID='nope'/>");
    assert_eq!(kind_of(&bad_guid), CodecErrorKind::InvalidValue);
}

#[test]
fn property_cardinality_and_required_children_are_enforced() {
    let head = format!(
        "<dt:Property xmlns:dt='{NS}' dt:GUID='{GUID}' dateOfCreation='2026-09-23T00:00:00Z'>"
    );
    let decode = |body: &str| {
        let xml = format!("{head}{body}</dt:Property>");
        let document = Document::parse(&xml).expect("well-formed");
        Property::from_element(document.root())
            .expect_err(&xml)
            .kind()
    };
    assert_eq!(
        decode("<dt:Symbol>x</dt:Symbol>"),
        CodecErrorKind::MissingChild
    );
    let data_type = "<dt:DataType><dt:DataFormat value='x'/></dt:DataType>";
    let twice = format!("{data_type}{data_type}");
    assert_eq!(decode(&twice), CodecErrorKind::TooManyChildren);
}
