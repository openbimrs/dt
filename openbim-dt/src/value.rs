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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Owned core of ISO 23387 `ConceptType` for reuse by dependent standards.
///
/// Format codecs retain the complete XML tree separately; this value is the
/// stable, application-facing subset shared by DT and standards such as LOIN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concept {
    guid: Guid,
    date_of_creation: DateTime,
    names: Vec<MultiLanguageText>,
    definition: MultiLanguageText,
    references: Vec<Reference>,
}

impl Concept {
    /// Creates an Annex E-valid required `ConceptType` core.
    #[must_use]
    pub fn new(
        guid: Guid,
        date_of_creation: DateTime,
        first_name: MultiLanguageText,
        definition: MultiLanguageText,
    ) -> Self {
        Self {
            guid,
            date_of_creation,
            names: vec![first_name],
            definition,
            references: Vec::new(),
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

    #[must_use]
    pub fn names(&self) -> &[MultiLanguageText] {
        &self.names
    }

    #[must_use]
    pub const fn definition(&self) -> &MultiLanguageText {
        &self.definition
    }

    #[must_use]
    pub fn references(&self) -> &[Reference] {
        &self.references
    }

    pub fn add_name(&mut self, name: MultiLanguageText) {
        self.names.push(name);
    }

    pub fn set_definition(&mut self, definition: MultiLanguageText) {
        self.definition = definition;
    }

    pub fn add_reference(&mut self, reference: Reference) {
        self.references.push(reference);
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
