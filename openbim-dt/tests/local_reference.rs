use std::{env, fs};

use openbim_dt::{Document, Element, Node, Severity};

/// Optional local-reference verification. The public repository never contains
/// or downloads the restricted Annex example.
#[test]
fn configured_local_annex_example_round_trips() {
    let Ok(path) = env::var("OPENBIM_DT_REFERENCE_EXAMPLE") else {
        eprintln!("OPENBIM_DT_REFERENCE_EXAMPLE is not configured; skipping local corpus probe");
        return;
    };
    let source = fs::read_to_string(path).expect("read configured local DT example");
    let first = Document::parse(&source).expect("parse configured local DT example");
    let output = first
        .to_xml_string()
        .expect("write configured local DT example");
    let second = Document::parse(&output).expect("reparse configured local DT example");
    assert_document_eq(&second, &first);
    assert!(
        first
            .validate()
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error),
        "configured local example produced validation errors: {:?}",
        first.validate()
    );
}

fn assert_document_eq(actual: &Document, expected: &Document) {
    assert_eq!(actual.declaration(), expected.declaration(), "declaration");
    assert_nodes("prolog", actual.prolog(), expected.prolog());
    assert_element("root", actual.root(), expected.root());
    assert_nodes("epilog", actual.epilog(), expected.epilog());
}

fn assert_element(path: &str, actual: &Element, expected: &Element) {
    assert_eq!(actual.qname(), expected.qname(), "{path} qname");
    assert_eq!(
        actual.namespace_uri(),
        expected.namespace_uri(),
        "{path} namespace"
    );
    assert_eq!(
        actual.attributes(),
        expected.attributes(),
        "{path} attributes"
    );
    assert_eq!(
        actual.was_empty_element(),
        expected.was_empty_element(),
        "{path} empty-element style"
    );
    assert_nodes(path, actual.nodes(), expected.nodes());
}

fn assert_nodes(path: &str, actual: &[Node], expected: &[Node]) {
    assert_eq!(actual.len(), expected.len(), "{path} node count");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        let child_path = format!("{path}/{index}");
        match (actual, expected) {
            (Node::Element(actual), Node::Element(expected)) => {
                assert_element(&child_path, actual, expected);
            }
            _ => assert_eq!(actual, expected, "{child_path}"),
        }
    }
}
