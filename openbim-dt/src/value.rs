//! ISO 23387 lexical value contracts.

use std::{error::Error, fmt, str::FromStr};

/// A value rejected by an ISO 23387 lexical contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueError {
    kind: ValueErrorKind,
    value: String,
}

impl ValueError {
    fn new(kind: ValueErrorKind, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
        }
    }

    /// The failed lexical contract.
    #[must_use]
    pub const fn kind(&self) -> ValueErrorKind {
        self.kind
    }
}

impl fmt::Display for ValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid {:?} value {:?}", self.kind, self.value)
    }
}

impl Error for ValueError {}

/// Lexical contracts checked by [`ValueError`].
///
/// Non-exhaustive: new ISO 23387 lexical contracts are added as the owned
/// model grows, and matching code must keep a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueErrorKind {
    /// ISO 23387 `GuidType`.
    Guid,
    /// XML Schema `language`.
    Language,
    /// ISO 23387 `RationalType`.
    Rational,
    /// XML Schema `decimal`.
    Decimal,
    /// XML Schema `positiveInteger`.
    PositiveInteger,
    /// A semantically identified reference with neither GUID nor URI.
    EmptyReference,
    /// XML Schema `dateTime`.
    CreationDate,
    /// XML Schema `anyURI`.
    Uri,
    /// XML Schema `nonNegativeInteger`.
    NonNegativeInteger,
    /// XML Schema `base64Binary`.
    Base64Binary,
}

/// An ISO 23387 GUID, preserving its validated source spelling.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Guid(String);

impl Guid {
    /// Returns the original validated lexical value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Guid {
    type Err = ValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        const HYPHENS: [usize; 4] = [8, 13, 18, 23];
        let valid = value.len() == 36
            && value.bytes().enumerate().all(|(index, byte)| {
                if HYPHENS.contains(&index) {
                    byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            });
        valid
            .then(|| Self(value.to_owned()))
            .ok_or_else(|| ValueError::new(ValueErrorKind::Guid, value))
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// XML Schema `language` used by DT and importing standards.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Language(String);

impl Language {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Language {
    type Err = ValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = collapse_whitespace(value);
        is_language(&value)
            .then(|| Self(value.clone()))
            .ok_or_else(|| ValueError::new(ValueErrorKind::Language, value))
    }
}

/// XML Schema `dateTime` value after whitespace collapsing and lexical validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DateTime(String);

impl DateTime {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for DateTime {
    type Err = ValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = collapse_whitespace(value);
        is_xs_datetime(&value)
            .then(|| Self(value.clone()))
            .ok_or_else(|| ValueError::new(ValueErrorKind::CreationDate, value))
    }
}

/// XML Schema 1.0 `anyURI` after whitespace collapsing.
///
/// Its lexical space is broader than an ASCII URI-reference: XML Schema's
/// escaping procedure admits Unicode and spaces that become percent-encoded in
/// the corresponding URI. The stored value retains that pre-escaped spelling.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnyUri(String);

impl AnyUri {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for AnyUri {
    type Err = ValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = collapse_whitespace(value);
        value
            .chars()
            .all(is_xml_10_character)
            .then(|| Self(value.clone()))
            .ok_or_else(|| ValueError::new(ValueErrorKind::Uri, value))
    }
}

/// ISO 23387 multilingual text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MultiLanguageText {
    language: Language,
    text: String,
}

impl MultiLanguageText {
    /// Creates text after validating the XML Schema `language` lexeme.
    pub fn new(language: impl Into<String>, text: impl Into<String>) -> Result<Self, ValueError> {
        let language = language.into().parse()?;
        Ok(Self {
            language,
            text: text.into(),
        })
    }

    /// Language tag exactly as supplied.
    #[must_use]
    pub fn language(&self) -> &str {
        self.language.as_str()
    }

    /// Text value exactly as supplied.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// An ISO 23387 reference by GUID, URI, or both.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reference {
    guid: Option<Guid>,
    uri: Option<AnyUri>,
}

impl Reference {
    /// Creates the exact XSD contract. Annex E permits both attributes to be absent.
    #[must_use]
    pub const fn new(guid: Option<Guid>, uri: Option<AnyUri>) -> Self {
        Self { guid, uri }
    }

    /// Creates a semantically identified reference, rejecting the XSD-valid empty state.
    pub fn identified(guid: Option<Guid>, uri: Option<AnyUri>) -> Result<Self, ValueError> {
        if guid.is_none() && uri.is_none() {
            return Err(ValueError::new(ValueErrorKind::EmptyReference, ""));
        }
        Ok(Self::new(guid, uri))
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.guid.is_none() && self.uri.is_none()
    }

    /// Referenced GUID, when present.
    #[must_use]
    pub const fn guid(&self) -> Option<&Guid> {
        self.guid.as_ref()
    }

    /// Referenced URI, when present.
    #[must_use]
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(AnyUri::as_str)
    }
}

/// Generates a slice getter and an appending adder for a repeatable field.
macro_rules! list_accessors {
    ($($(#[$doc:meta])* $field:ident: $ty:ty => $getter:ident, $adder:ident;)*) => {
        $(
            $(#[$doc])*
            #[must_use]
            pub fn $getter(&self) -> &[$ty] {
                &self.$field
            }
            $(#[$doc])*
            pub fn $adder(&mut self, value: $ty) {
                self.$field.push(value);
            }
        )*
    };
}
pub(crate) use list_accessors;

/// Owned ISO 23387 `ConceptType`: every attribute and child it declares.
///
/// Each field corresponds to exactly one declared child element (or
/// attribute), and every repeatable child is a `Vec`, so a value decoded by
/// [`Concept`]-bearing `from_element` codecs holds everything the schema
/// permits. `ConceptType` content is a repeating choice, so child order
/// carries no meaning and is not stored.
///
/// The schema requires at least one child from the choice, and `new` always
/// supplies a name. `from_identity` builds an empty concept for decoders and
/// callers that add content afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concept {
    guid: Guid,
    date_of_creation: DateTime,
    about: Option<AnyUri>,
    names: Vec<MultiLanguageText>,
    definitions: Vec<MultiLanguageText>,
    descriptions: Vec<MultiLanguageText>,
    examples: Vec<MultiLanguageText>,
    reference_document_refs: Vec<Reference>,
    similar_to_refs: Vec<Reference>,
    replaced_objects_refs: Vec<Reference>,
    dictionary_refs: Vec<Reference>,
    languages_of_creator: Vec<Language>,
    countries_of_origin: Vec<String>,
    visual_representations: Vec<Base64Binary>,
    major_versions: Vec<NonNegativeInteger>,
    minor_versions: Vec<NonNegativeInteger>,
    statuses: Vec<String>,
    deprecation_explanations: Vec<String>,
}

impl Concept {
    /// Creates a concept with one name and one definition.
    #[must_use]
    pub fn new(
        guid: Guid,
        date_of_creation: DateTime,
        first_name: MultiLanguageText,
        definition: MultiLanguageText,
    ) -> Self {
        let mut concept = Self::from_identity(guid, date_of_creation);
        concept.names.push(first_name);
        concept.definitions.push(definition);
        concept
    }

    /// Creates a concept carrying only its required attributes.
    ///
    /// It is not schema-valid until at least one child is added.
    #[must_use]
    pub const fn from_identity(guid: Guid, date_of_creation: DateTime) -> Self {
        Self {
            guid,
            date_of_creation,
            about: None,
            names: Vec::new(),
            definitions: Vec::new(),
            descriptions: Vec::new(),
            examples: Vec::new(),
            reference_document_refs: Vec::new(),
            similar_to_refs: Vec::new(),
            replaced_objects_refs: Vec::new(),
            dictionary_refs: Vec::new(),
            languages_of_creator: Vec::new(),
            countries_of_origin: Vec::new(),
            visual_representations: Vec::new(),
            major_versions: Vec::new(),
            minor_versions: Vec::new(),
            statuses: Vec::new(),
            deprecation_explanations: Vec::new(),
        }
    }

    #[must_use]
    pub const fn guid(&self) -> &Guid {
        &self.guid
    }

    #[must_use]
    pub fn date_of_creation(&self) -> &str {
        self.date_of_creation.as_str()
    }

    /// The `dt:about` attribute, when present.
    #[must_use]
    pub fn about(&self) -> Option<&str> {
        self.about.as_ref().map(AnyUri::as_str)
    }

    pub fn set_about(&mut self, value: Option<AnyUri>) {
        self.about = value;
    }

    /// The first `Definition`, when present.
    ///
    /// `ConceptType` permits any number of definitions (typically one per
    /// language); use [`Concept::definitions`] to see all of them.
    #[must_use]
    pub fn definition(&self) -> Option<&MultiLanguageText> {
        self.definitions.first()
    }

    /// Replaces every definition with `definition`.
    pub fn set_definition(&mut self, definition: MultiLanguageText) {
        self.definitions.clear();
        self.definitions.push(definition);
    }

    /// `ReferenceDocumentRef` children.
    #[deprecated(since = "0.3.0", note = "use `reference_document_refs`")]
    #[must_use]
    pub fn references(&self) -> &[Reference] {
        &self.reference_document_refs
    }

    /// Appends a `ReferenceDocumentRef`.
    #[deprecated(since = "0.3.0", note = "use `add_reference_document_ref`")]
    pub fn add_reference(&mut self, reference: Reference) {
        self.reference_document_refs.push(reference);
    }

    list_accessors! {
        /// `Name` children.
        names: MultiLanguageText => names, add_name;
        /// `Definition` children.
        definitions: MultiLanguageText => definitions, add_definition;
        /// `Description` children.
        descriptions: MultiLanguageText => descriptions, add_description;
        /// `Example` children.
        examples: MultiLanguageText => examples, add_example;
        /// `ReferenceDocumentRef` children.
        reference_document_refs: Reference => reference_document_refs, add_reference_document_ref;
        /// `SimilarToRef` children.
        similar_to_refs: Reference => similar_to_refs, add_similar_to_ref;
        /// `ReplacedObjectsRef` children.
        replaced_objects_refs: Reference => replaced_objects_refs, add_replaced_objects_ref;
        /// `DictionaryRef` children.
        dictionary_refs: Reference => dictionary_refs, add_dictionary_ref;
        /// `LanguageOfCreator` children.
        languages_of_creator: Language => languages_of_creator, add_language_of_creator;
        /// `CountryOfOrigin` children.
        countries_of_origin: String => countries_of_origin, add_country_of_origin;
        /// `VisualRepresentation` children.
        visual_representations: Base64Binary => visual_representations, add_visual_representation;
        /// `MajorVersion` children.
        major_versions: NonNegativeInteger => major_versions, add_major_version;
        /// `MinorVersion` children.
        minor_versions: NonNegativeInteger => minor_versions, add_minor_version;
        /// `Status` children.
        statuses: String => statuses, add_status;
        /// `DeprecationExplanation` children.
        deprecation_explanations: String => deprecation_explanations, add_deprecation_explanation;
    }

    pub(crate) const fn date_of_creation_value(&self) -> &DateTime {
        &self.date_of_creation
    }

    pub(crate) const fn about_value(&self) -> Option<&AnyUri> {
        self.about.as_ref()
    }
}

/// ISO 23387 property data-type names with forward-compatible retention.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataTypeName {
    Boolean,
    Integer,
    Rational,
    Real,
    Complex,
    String,
    DateTime,
    /// A future or extension value retained verbatim.
    Other(String),
}

impl DataTypeName {
    /// Wire spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Boolean => "BOOLEAN",
            Self::Integer => "INTEGER",
            Self::Rational => "RATIONAL",
            Self::Real => "REAL",
            Self::Complex => "COMPLEX",
            Self::String => "STRING",
            Self::DateTime => "DATETIME",
            Self::Other(value) => value,
        }
    }
}

impl From<&str> for DataTypeName {
    fn from(value: &str) -> Self {
        match value {
            "BOOLEAN" => Self::Boolean,
            "INTEGER" => Self::Integer,
            "RATIONAL" => Self::Rational,
            "REAL" => Self::Real,
            "COMPLEX" => Self::Complex,
            "STRING" => Self::String,
            "DATETIME" => Self::DateTime,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// Unit scale with forward-compatible retention.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scale {
    Linear,
    Logarithmic,
    Other(String),
}

impl From<&str> for Scale {
    fn from(value: &str) -> Self {
        match value {
            "LINEAR" => Self::Linear,
            "LOGARITHMIC" => Self::Logarithmic,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl Scale {
    /// Wire spelling; the inverse of `From<&str>`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Linear => "LINEAR",
            Self::Logarithmic => "LOGARITHMIC",
            Self::Other(value) => value,
        }
    }
}

/// Unit logarithm base with forward-compatible retention.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Base {
    One,
    Two,
    E,
    Pi,
    Ten,
    Other(String),
}

impl From<&str> for Base {
    fn from(value: &str) -> Self {
        match value {
            "ONE" => Self::One,
            "TWO" => Self::Two,
            "E" => Self::E,
            "PI" => Self::Pi,
            "TEN" => Self::Ten,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl Base {
    /// Wire spelling; the inverse of `From<&str>`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::One => "ONE",
            Self::Two => "TWO",
            Self::E => "E",
            Self::Pi => "PI",
            Self::Ten => "TEN",
            Self::Other(value) => value,
        }
    }
}

/// XML Schema decimal preserving its whitespace-collapsed validated lexeme.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decimal(String);

impl Decimal {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Decimal {
    type Err = ValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = collapse_whitespace(value);
        let unsigned = value.strip_prefix(['+', '-']).unwrap_or(&value);
        let mut parts = unsigned.split('.');
        let integer = parts.next().unwrap_or_default();
        let fraction = parts.next();
        let valid_integer = integer.bytes().all(|byte| byte.is_ascii_digit());
        let valid_fraction =
            fraction.is_none_or(|part| part.bytes().all(|byte| byte.is_ascii_digit()));
        let has_digit = !integer.is_empty() || fraction.is_some_and(|part| !part.is_empty());
        let valid = valid_integer && valid_fraction && has_digit && parts.next().is_none();
        valid
            .then(|| Self(value.clone()))
            .ok_or_else(|| ValueError::new(ValueErrorKind::Decimal, value))
    }
}

/// XML Schema `positiveInteger`, preserved after whitespace collapse.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PositiveInteger(String);

impl PositiveInteger {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for PositiveInteger {
    type Err = ValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = collapse_whitespace(value);
        let digits = value.strip_prefix('+').unwrap_or(&value);
        let valid = !digits.is_empty()
            && digits.bytes().all(|byte| byte.is_ascii_digit())
            && digits.bytes().any(|byte| byte != b'0');
        valid
            .then(|| Self(value.clone()))
            .ok_or_else(|| ValueError::new(ValueErrorKind::PositiveInteger, value))
    }
}

/// XML Schema `nonNegativeInteger`, preserving the validated source lexeme.
///
/// The lexeme is kept verbatim after whitespace collapse (`007` stays `007`),
/// so the type is unbounded and round trips are exact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NonNegativeInteger(String);

impl NonNegativeInteger {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for NonNegativeInteger {
    type Err = ValueError;

    /// Accepts `[+]?[0-9]+`, and `-0` style zero, per the XSD lexical space.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = collapse_whitespace(value);
        let (negative, digits) = match value.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, value.strip_prefix('+').unwrap_or(&value)),
        };
        let valid = !digits.is_empty()
            && digits.bytes().all(|byte| byte.is_ascii_digit())
            && (!negative || digits.bytes().all(|byte| byte == b'0'));
        valid
            .then(|| Self(value.clone()))
            .ok_or_else(|| ValueError::new(ValueErrorKind::NonNegativeInteger, value))
    }
}

/// XML Schema `base64Binary`, preserving the validated source lexeme.
///
/// Validation follows the XSD 1.1 canonical-lexical grammar: whitespace-
/// separated base64 characters in groups of four, with at most two `=` pad
/// characters at the end whose preceding character is restricted so the
/// padding bits are zero. The empty value is valid (zero octets).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Base64Binary(String);

impl Base64Binary {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Base64Binary {
    type Err = ValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let collapsed = collapse_whitespace(value);
        is_base64_lexeme(&collapsed)
            .then(|| Self(collapsed.clone()))
            .ok_or_else(|| ValueError::new(ValueErrorKind::Base64Binary, collapsed))
    }
}

fn is_base64_lexeme(value: &str) -> bool {
    let symbols: Vec<u8> = value.bytes().filter(|byte| *byte != b' ').collect();
    if symbols.len() % 4 != 0 {
        return false;
    }
    let padding = symbols
        .iter()
        .rev()
        .take_while(|byte| **byte == b'=')
        .count();
    if padding > 2 {
        return false;
    }
    let body = &symbols[..symbols.len() - padding];
    if !body
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/'))
    {
        return false;
    }
    // The last data character before padding may only carry zero padding bits:
    // one pad => B04 alphabet (value % 4 == 0), two pads => B16 (value % 16 == 0).
    match (padding, body.last()) {
        (0, _) | (_, None) => padding == 0,
        (1, Some(&last)) => base64_value(last) % 4 == 0,
        (2, Some(&last)) => base64_value(last) % 16 == 0,
        _ => false,
    }
}

const fn base64_value(byte: u8) -> u8 {
    match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' => 62,
        _ => 63,
    }
}

/// ISO 23387 rational value preserving the validated source lexeme.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rational(String);

impl Rational {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Rational {
    type Err = ValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
        let mut parts = unsigned.split('/');
        let numerator = parts.next().unwrap_or_default();
        let denominator = parts.next();
        let valid_numerator =
            !numerator.is_empty() && numerator.bytes().all(|b| b.is_ascii_digit());
        let valid_denominator = denominator.is_none_or(|part| {
            part.bytes()
                .next()
                .is_some_and(|first| matches!(first, b'1'..=b'9'))
                && part.bytes().all(|b| b.is_ascii_digit())
        });
        let valid = valid_numerator && valid_denominator && parts.next().is_none();
        valid
            .then(|| Self(value.to_owned()))
            .ok_or_else(|| ValueError::new(ValueErrorKind::Rational, value))
    }
}

fn collapse_whitespace(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut pending_space = false;
    for character in value.chars() {
        if matches!(character, ' ' | '\t' | '\r' | '\n') {
            pending_space = !output.is_empty();
        } else {
            if pending_space {
                output.push(' ');
                pending_space = false;
            }
            output.push(character);
        }
    }
    output
}

fn is_xml_10_character(value: char) -> bool {
    matches!(value, '\u{9}' | '\u{A}' | '\u{D}')
        || ('\u{20}'..='\u{D7FF}').contains(&value)
        || ('\u{E000}'..='\u{FFFD}').contains(&value)
        || ('\u{10000}'..='\u{10FFFF}').contains(&value)
}

fn is_language(value: &str) -> bool {
    let mut parts = value.split('-');
    let Some(first) = parts.next() else {
        return false;
    };
    let first_valid =
        (1..=8).contains(&first.len()) && first.bytes().all(|b| b.is_ascii_alphabetic());
    first_valid
        && parts.all(|part| {
            (1..=8).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}

fn is_xs_datetime(value: &str) -> bool {
    let Some((date, time_and_zone)) = value.split_once('T') else {
        return false;
    };
    if time_and_zone.contains('T') || !valid_xs_date(date) {
        return false;
    }
    let (time, zone) = split_timezone(time_and_zone);
    valid_xs_time(time) && zone.is_none_or(valid_timezone)
}

fn valid_xs_date(value: &str) -> bool {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let mut parts = unsigned.split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if parts.next().is_some()
        || year.len() < 4
        || (year.len() > 4 && year.starts_with('0'))
        || !year.bytes().all(|b| b.is_ascii_digit())
        || year.bytes().all(|b| b == b'0')
        || month.len() != 2
        || day.len() != 2
    {
        return false;
    }
    let (Ok(month), Ok(day)) = (month.parse::<u8>(), day.parse::<u8>()) else {
        return false;
    };
    let year_mod_400 = year.bytes().fold(0_u16, |value, digit| {
        (value * 10 + u16::from(digit - b'0')) % 400
    });
    let leap = year_mod_400 % 4 == 0 && (year_mod_400 % 100 != 0 || year_mod_400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn valid_xs_time(value: &str) -> bool {
    let mut parts = value.split(':');
    let (Some(hour), Some(minute), Some(second)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if parts.next().is_some() || hour.len() != 2 || minute.len() != 2 {
        return false;
    }
    let (Ok(hour), Ok(minute)) = (hour.parse::<u8>(), minute.parse::<u8>()) else {
        return false;
    };
    let mut second_parts = second.split('.');
    let whole = second_parts.next().unwrap_or_default();
    let fraction = second_parts.next();
    let valid_second = whole.len() == 2
        && whole.bytes().all(|b| b.is_ascii_digit())
        && whole.parse::<u8>().is_ok_and(|v| v <= 59)
        && fraction.is_none_or(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        && second_parts.next().is_none();
    valid_second
        && minute <= 59
        && (hour <= 23
            || (hour == 24
                && minute == 0
                && whole == "00"
                && fraction.is_none_or(|value| value.bytes().all(|byte| byte == b'0'))))
}

fn split_timezone(value: &str) -> (&str, Option<&str>) {
    if let Some(time) = value.strip_suffix('Z') {
        return (time, Some("Z"));
    }
    if value.len() >= 6 {
        let boundary = value.len() - 6;
        if matches!(value.as_bytes()[boundary], b'+' | b'-') {
            return (&value[..boundary], Some(&value[boundary..]));
        }
    }
    (value, None)
}

fn valid_timezone(value: &str) -> bool {
    if value == "Z" {
        return true;
    }
    let bytes = value.as_bytes();
    if bytes.len() != 6 || !matches!(bytes[0], b'+' | b'-') || bytes[3] != b':' {
        return false;
    }
    let (Ok(hour), Ok(minute)) = (value[1..3].parse::<u8>(), value[4..6].parse::<u8>()) else {
        return false;
    };
    hour <= 14 && minute <= 59 && (hour != 14 || minute == 0)
}
