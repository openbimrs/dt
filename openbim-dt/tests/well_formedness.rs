use openbim_dt::{DiagnosticCode, Document, ElementKind};

const NS: &str = "https://standards.iso.org/iso/23387/ed-2/en/";

#[test]
fn parser_rejects_namespace_and_character_well_formedness_violations() {
    let malformed = [
        format!(r#"<dt:Library xmlns:dt="{NS}" xmlns:x="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111" x:GUID="bad"/>"#),
        format!(r#"<dt:Library xmlns:dt="{NS}" xmlns:xml="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111"/>"#),
        format!(r#"<dt:Library xmlns:dt="{NS}" xmlns:xmlns="urn:bad" dt:GUID="11111111-1111-1111-1111-111111111111"/>"#),
        format!(r#"outside<dt:Library xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111"/>"#),
        format!(r#"<dt:Library xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111"><!-- bad -- comment --></dt:Library>"#),
        format!(r#"<dt:Library xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111"><?1bad value?></dt:Library>"#),
        format!(r#"<dt:Library xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111">&#1;</dt:Library>"#),
        format!(r#"<dt:Library xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111">&#0;</dt:Library>"#),
        format!("<dt:Library xmlns:dt=\"{NS}\" dt:GUID=\"11111111-1111-1111-1111-111111111111\">\u{1}</dt:Library>"),
        format!(r#"<?xml version="1.1"?><dt:Library xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111"/>"#),
        format!(r#"<?xml version="1.0" standalone="maybe"?><dt:Library xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111"/>"#),
        format!(r#"<?xml version="1.0" encoding="UTF-16"?><dt:Library xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111"/>"#),
    ];

    for xml in malformed {
        assert!(
            Document::parse(&xml).is_err(),
            "accepted malformed XML: {xml:?}"
        );
    }
}

#[test]
fn root_and_context_sensitive_types_match_annex_e_declarations() {
    let property = format!(
        r#"<dt:Property xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111" dateOfCreation="2026-08-25T00:00:00Z"><dt:Name language="en">P</dt:Name><dt:DataType name="STRING"/><dt:Symbol>F</dt:Symbol><dt:ReferenceDocumentRef/></dt:Property>"#
    );
    let property = Document::parse(&property).unwrap();
    assert_eq!(property.root_kind(), Some(ElementKind::Property));
    let diagnostics = property.validate();
    assert!(!diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::MissingLanguage));
    assert!(diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::EmptyReference));

    let unit = format!(
        r#"<dt:Unit xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111" dateOfCreation="2026-08-25T00:00:00Z"><dt:Name language="en">U</dt:Name><dt:Symbol language="en">m</dt:Symbol><dt:DimensionRef dt:GUID="22222222-2222-2222-2222-222222222222"/><dt:Scale>LINEAR</dt:Scale><dt:Base>TEN</dt:Base><dt:Coefficient>1</dt:Coefficient><dt:Offset>0</dt:Offset></dt:Unit>"#
    );
    let unit = Document::parse(&unit).unwrap();
    assert_eq!(unit.root_kind(), None);
    assert!(unit
        .validate()
        .iter()
        .any(|d| d.code == DiagnosticCode::UnknownRootElement));
}

#[test]
fn reference_and_multilingual_diagnostics_follow_parent_declarations() {
    let xml = format!(
        r#"<dt:Property xmlns:dt="{NS}" dt:GUID="11111111-1111-1111-1111-111111111111" dateOfCreation="2026-08-25T00:00:00Z">
  <dt:Name language=" en ">Synthetic</dt:Name>
  <dt:Definition language="en">Synthetic definition</dt:Definition>
  <dt:IsPartOfGroupOfPropertiesRef/>
  <dt:HasPropertyRef/>
  <dt:DataType><dt:Name>STRING</dt:Name></dt:DataType>
</dt:Property>"#
    );
    let diagnostics = Document::parse(&xml).unwrap().validate();
    assert!(!diagnostics.iter().any(|item| {
        matches!(
            item.code,
            DiagnosticCode::EmptyReference | DiagnosticCode::MissingLanguage
        )
    }));

    let positive = format!(
        r#"<dt:GroupOfProperties xmlns:dt="{NS}" dt:GUID="22222222-2222-2222-2222-222222222222" dateOfCreation="2026-08-25T00:00:00Z">
  <dt:Name language="en">Synthetic</dt:Name>
  <dt:Definition language="en">Synthetic definition</dt:Definition>
  <dt:HasPropertyRef/>
</dt:GroupOfProperties>"#
    );
    assert!(Document::parse(&positive)
        .unwrap()
        .validate()
        .iter()
        .any(|item| item.code == DiagnosticCode::EmptyReference));
}

#[test]
fn inherited_namespace_storage_is_shared_and_scopes_restore_without_cloning() {
    let namespace = format!("urn:{}", "a".repeat(1_000_000));
    let xml = format!(
        "<p:root xmlns:p=\"{namespace}\" xmlns=\"urn:default\"><p:first><p:second/></p:first><plain xmlns=\"\"/><after/><p:shadow xmlns:p=\"urn:inner\"><p:leaf/></p:shadow><p:restored/></p:root>"
    );
    let document = Document::parse(&xml).unwrap();
    let root = document.root();
    let first = root.children().next().unwrap();
    let second = first.children().next().unwrap();

    assert!(std::ptr::eq(
        root.namespace_uri().unwrap(),
        first.namespace_uri().unwrap()
    ));
    assert!(std::ptr::eq(
        first.namespace_uri().unwrap(),
        second.namespace_uri().unwrap()
    ));
    assert_eq!(root.children().nth(1).unwrap().namespace_uri(), None);
    assert_eq!(
        root.children().nth(2).unwrap().namespace_uri(),
        Some("urn:default")
    );
    let shadow = root.children().nth(3).unwrap();
    assert_eq!(shadow.namespace_uri(), Some("urn:inner"));
    assert!(std::ptr::eq(
        shadow.namespace_uri().unwrap(),
        shadow.children().next().unwrap().namespace_uri().unwrap()
    ));
    assert!(std::ptr::eq(
        root.namespace_uri().unwrap(),
        root.children().nth(4).unwrap().namespace_uri().unwrap()
    ));
}

#[test]
fn value_list_requires_language_and_contains_repeating_ordered_values() {
    let valid = format!(
        r#"<dt:PossibleValues xmlns:dt="{NS}"><dt:ValueList language="en"><dt:Value order="-2147483648">A</dt:Value><dt:Value order="2147483647">B</dt:Value></dt:ValueList></dt:PossibleValues>"#
    );
    assert!(!Document::parse(&valid)
        .unwrap()
        .validate()
        .iter()
        .any(|item| item.code == DiagnosticCode::MissingLanguage));

    let missing_language = valid.replace(" language=\"en\"", "");
    assert!(Document::parse(&missing_language)
        .unwrap()
        .validate()
        .iter()
        .any(|item| item.code == DiagnosticCode::MissingLanguage));
}
