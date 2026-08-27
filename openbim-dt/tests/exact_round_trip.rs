//! Byte-identical round trips via `Document::to_xml_string_exact`.
//!
//! These tests pin the distinction the README draws between the two writers:
//! `to_xml_string` is semantic and may normalize equivalent syntax, while
//! `to_xml_string_exact` reproduces the original bytes or fails closed.

use openbim_dt::Document;

/// Inputs whose syntax a semantic writer is free to normalize. Each one is a
/// concrete way two XML documents can be equivalent but not byte-equal.
const NORMALIZATION_TRAPS: &[(&str, &str)] = &[
    (
        "single-quoted attributes",
        "<dt:Library xmlns:dt='https://standards.iso.org/iso/23387/ed-2/en/' dt:GUID='11111111-1111-1111-1111-111111111111'/>",
    ),
    (
        "redundant whitespace inside tags",
        "<dt:Library    xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"\n\t dt:GUID=\"11111111-1111-1111-1111-111111111111\"   />",
    ),
    (
        "numeric character references that need not stay escaped",
        "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"><dt:Name language=\"en\">&#65;&#x42;C</dt:Name></dt:Library>",
    ),
    (
        "entity references for characters that are legal raw",
        "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"><dt:Name language=\"en\">a &gt; b</dt:Name></dt:Library>",
    ),
    (
        "CDATA holding text that needs no CDATA",
        "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"><dt:Name language=\"en\"><![CDATA[plain]]></dt:Name></dt:Library>",
    ),
    (
        "empty-element vs explicit close tag",
        "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"><dt:Name language=\"en\"></dt:Name></dt:Library>",
    ),
    (
        "comments and processing instructions in prolog and epilog",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- lead -->\n<?render mode=\"strict\"?>\n<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"/>\n<!-- trail -->\n",
    ),
    (
        "comment containing characters a text writer would escape",
        "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"><!-- a & b < c > d --></dt:Library>",
    ),
    (
        "significant whitespace and mixed content",
        "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\">\n  <dt:Name language=\"en\">  spaced  </dt:Name>\n</dt:Library>\n",
    ),
    (
        "attribute order that alphabetical sorting would disturb",
        "<dt:Library dt:GUID=\"11111111-1111-1111-1111-111111111111\" xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\" about=\"urn:z\"/>",
    ),
    (
        "default namespace plus prefixed namespace on one element",
        "<Library xmlns=\"https://standards.iso.org/iso/23387/ed-2/en/\" xmlns:other=\"urn:other\" other:tag=\"1\"/>",
    ),
    (
        "tab and newline character references inside an attribute",
        "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\" about=\"a&#9;b&#10;c\"/>",
    ),
    (
        "standalone declaration",
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"/>",
    ),
    (
        "deeply nested unknown content",
        "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"><x:a xmlns:x=\"urn:x\"><x:b><x:c>v</x:c></x:b></x:a></dt:Library>",
    ),
];

#[test]
fn exact_writer_reproduces_source_bytes_for_every_normalization_trap() {
    for (label, xml) in NORMALIZATION_TRAPS {
        let document = Document::parse(xml).unwrap_or_else(|error| {
            panic!("{label}: input should parse, got {error}");
        });
        let exact = document
            .to_xml_string_exact()
            .unwrap_or_else(|error| panic!("{label}: exact write failed: {error}"));
        assert_eq!(
            exact, *xml,
            "{label}: exact writer must reproduce the source byte for byte"
        );
    }
}

#[test]
fn exact_round_trip_is_stable_across_repeated_cycles() {
    for (label, xml) in NORMALIZATION_TRAPS {
        let mut current = (*xml).to_owned();
        for cycle in 0..3 {
            let document = Document::parse(&current).unwrap();
            let next = document.to_xml_string_exact().unwrap();
            assert_eq!(
                next, current,
                "{label}: cycle {cycle} drifted from the previous output"
            );
            current = next;
        }
    }
}

/// The claim is byte-identity, not "identity for the cases we happened to
/// list". A semantic re-parse of the exact output must also be equivalent.
#[test]
fn exact_output_reparses_to_an_equivalent_document() {
    for (label, xml) in NORMALIZATION_TRAPS {
        let first = Document::parse(xml).unwrap();
        let exact = first.to_xml_string_exact().unwrap();
        let second = Document::parse(&exact).unwrap();
        assert_eq!(
            first.to_xml_string().unwrap(),
            second.to_xml_string().unwrap(),
            "{label}: exact output must be semantically identical too"
        );
    }
}

/// Documents that were built rather than parsed have no source, so the exact
/// writer must refuse rather than invent bytes.
#[test]
fn exact_writer_fails_closed_without_retained_source() {
    let xml = "<dt:Library xmlns:dt=\"https://standards.iso.org/iso/23387/ed-2/en/\"/>";
    let document = Document::parse(xml).unwrap();
    assert!(document.source().is_some());
    assert!(document.to_xml_string_exact().is_ok());

    let detached = document.without_source();
    assert!(detached.source().is_none());
    assert!(
        detached.to_xml_string_exact().is_err(),
        "a document with no retained source must not fabricate exact bytes"
    );
    // The semantic writer still works; only the exact claim is withdrawn.
    assert!(detached.to_xml_string().is_ok());
}

/// Guards the difference between the two writers actually existing. If the
/// semantic writer ever became byte-exact this test would need rewriting, but
/// silently claiming exactness from `to_xml_string` would be worse.
#[test]
fn semantic_writer_is_allowed_to_normalize_where_exact_writer_is_not() {
    let single_quoted =
        "<dt:Library xmlns:dt='https://standards.iso.org/iso/23387/ed-2/en/' about='urn:a'/>";
    let document = Document::parse(single_quoted).unwrap();

    assert_eq!(document.to_xml_string_exact().unwrap(), single_quoted);
    assert_ne!(
        document.to_xml_string().unwrap(),
        single_quoted,
        "this input exists to prove the semantic writer normalizes it"
    );
}
