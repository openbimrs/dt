//! `openbim-dt` — ISO 23387 data templates.
//!
//! # What this is
//!
//! The concept vocabulary that describes *properties themselves*: property
//! definitions, groups of properties, quantity kinds, dimensions, units,
//! object types, and the reference machinery binding them to external
//! dictionaries such as bSDD.
//!
//! # Why it is a crate and not part of `openbim-loin`
//!
//! LOIN does not own ISO 23387. The ISO 7817-3 schema imports the ISO 23387
//! namespace and uses its types. Data-template tooling and dictionary clients
//! also need the vocabulary without acquiring a LOIN dependency.
//!
//! The dependency direction is therefore `openbim-loin` to `openbim-dt`, never
//! the reverse. The current LOIN scaffold does not yet consume these contracts.
//!
//! # Status
//!
//! **Reserved namespace scaffold.** This crate does not yet implement the ISO
//! 23387 data model, XML parsing/writing, semantic validation, ISO 23386
//! governance workflows, or ISO 12006-3 mapping.
//!
//! The ISO XSD is **not vendored** here. Redistribution rights do not follow
//! from possessing a standards document or annex, so restricted references
//! remain local and outside Git, crate packages, and documentation artifacts.

#![forbid(unsafe_code)]

/// The XML namespace declared by ISO 23387 edition 2.
///
/// Namespace identity is the only released wire-level contract. It does not
/// imply that this crate can parse or validate ISO 23387 documents.
pub const NAMESPACE: &str = "https://standards.iso.org/iso/23387/ed-2/en/";

/// A placeholder namespace found in pre-release ISO 23387 schema drafts.
///
/// It is named so future readers can produce a specific non-conformance
/// diagnostic instead of silently treating draft documents as edition 2.
pub const DRAFT_PLACEHOLDER_NAMESPACE: &str = "http://tempuri.org/XMLSchema.xsd";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_identities_match_the_documented_contracts() {
        assert_eq!(NAMESPACE, "https://standards.iso.org/iso/23387/ed-2/en/");
        assert_eq!(
            DRAFT_PLACEHOLDER_NAMESPACE,
            "http://tempuri.org/XMLSchema.xsd"
        );
        assert_ne!(NAMESPACE, DRAFT_PLACEHOLDER_NAMESPACE);
    }
}
