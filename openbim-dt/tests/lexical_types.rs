//! Lexical contracts added for the owned-type codec (ROADMAP milestone 3).
//!
//! Expected values are derived from the XML Schema 1.1 Part 2 definitions of
//! `nonNegativeInteger` and `base64Binary`, not from the implementation.

use std::str::FromStr;

use openbim_dt::{Base64Binary, NonNegativeInteger, ValueErrorKind};

#[test]
fn non_negative_integer_accepts_the_xsd_lexical_space_and_keeps_the_lexeme() {
    for accepted in [
        "0",
        "007",
        "+5",
        "-0",
        "-000",
        " 42 ",
        "18446744073709551616000",
    ] {
        let parsed = NonNegativeInteger::from_str(accepted)
            .unwrap_or_else(|error| panic!("{accepted:?} must be accepted: {error}"));
        assert_eq!(parsed.as_str(), accepted.trim(), "lexeme is preserved");
    }
}

#[test]
fn non_negative_integer_rejects_values_outside_the_lexical_space() {
    for rejected in ["", "-1", "-01", "1.0", "1e3", "0x10", "+", "-", "1 2"] {
        let error = NonNegativeInteger::from_str(rejected)
            .expect_err(&format!("{rejected:?} must be rejected"));
        assert_eq!(error.kind(), ValueErrorKind::NonNegativeInteger);
    }
}

#[test]
fn base64_accepts_canonical_and_whitespace_separated_lexemes() {
    // "", "f", "fo", "foo", "foob" in base64, plus XSD-permitted spacing.
    for accepted in [
        "",
        "Zg==",
        "Zm8=",
        "Zm9v",
        "Zm9vYg==",
        "Zm9v YmFy",
        "Z m 9 v",
    ] {
        Base64Binary::from_str(accepted)
            .unwrap_or_else(|error| panic!("{accepted:?} must be accepted: {error}"));
    }
}

#[test]
fn base64_rejects_bad_length_alphabet_and_nonzero_padding_bits() {
    for rejected in [
        "Zg",       // length not a multiple of four
        "Zg=",      // wrong pad count for length
        "Z===",     // three pad characters
        "Zh==",     // `h` leaves non-zero bits before two pads (must be in B16)
        "Zm9=",     // `9` leaves non-zero bits before one pad (must be in B04)
        "Zm9v!A==", // character outside the alphabet
        "=Zm9",     // padding not at the end
    ] {
        let error =
            Base64Binary::from_str(rejected).expect_err(&format!("{rejected:?} must be rejected"));
        assert_eq!(error.kind(), ValueErrorKind::Base64Binary);
    }
}
