//! Public construction API: a built tree must equal its own reparse.
use openbim_dt::{Attribute, Document, Element, NAMESPACE};

fn built() -> Element {
    let guid = Attribute::new(
        Some("dt"),
        "GUID",
        Some(NAMESPACE),
        "10000000-0000-0000-0000-000000000000",
    );
    let name = Element::new(Some(NAMESPACE), Some("dt"), "Name")
        .with_attribute(Attribute::new(None, "language", None, "en"))
        .with_text("Wall");
    Element::new(Some(NAMESPACE), Some("dt"), "ObjectType")
        .with_attribute(guid)
        .with_child(name)
}

#[test]
fn built_tree_writes_and_reparses_to_itself() {
    let document = Document::standalone(built());
    let xml = document.to_xml_string().expect("built trees serialize");
    let reparsed = Document::parse(&xml).expect("output reparses");
    assert_eq!(reparsed.root(), document.root(), "{xml}");
}

#[test]
fn built_tree_refuses_the_exact_writer() {
    let document = Document::standalone(built());
    assert!(
        document.to_xml_string_exact().is_err(),
        "no source to slice"
    );
}
