//! Regression: Library's children declared by `ref=` must be validated
//! against the referenced global element, not an empty local stub.

use openbim_dt::Document;

const LIBRARY: &str = r#"<dt:Library xmlns:dt="https://standards.iso.org/iso/23387/ed-2/en/" dt:GUID="11111111-1111-1111-1111-111111111111">
  <dt:ObjectType dt:GUID="22222222-2222-2222-2222-222222222222" dateOfCreation="2026-01-01T00:00:00Z">
    <dt:Name language="en">Wall</dt:Name>
    <dt:IsSubtypeOfRef dt:GUID="33333333-3333-3333-3333-333333333333"/>
  </dt:ObjectType>
</dt:Library>"#;

#[test]
fn library_ref_children_validate_against_the_global_declaration() {
    let report = Document::parse(LIBRARY).unwrap().validate_schema();
    assert!(report.is_conforming(), "{:#?}", report.violations());
}
