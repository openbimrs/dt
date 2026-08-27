//! XML Schema validation against the ISO 23387 structural grammar.
//!
//! This validates instance documents against the schema's element/attribute
//! grammar: global roots, child sequences and choices, cardinalities,
//! required and unknown attributes, enumerations, patterns, and the XML
//! Schema datatypes the vocabulary uses.
//!
//! It is not a general-purpose XSD processor: there is no support for
//! `xsi:type` substitution, identity constraints, or importing arbitrary
//! schema documents. The normative schema is never redistributed; only the
//! derived structural tables in [`crate::schema`] are compiled in.

use std::fmt;

use crate::{
    schema::{AttributeRule, Definition, DEFINITIONS, GLOBAL_ROOTS},
    Document, Element, Node, NAMESPACE,
};

/// A stable category for a schema-validation failure.
///
/// Codes are part of the public contract: callers match on them rather than
/// parsing messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SchemaViolationCode {
    /// The root element is not a global root declared by the schema.
    UnknownRoot,
    /// An element appeared where the schema does not declare it.
    UnexpectedElement,
    /// A required child occurred fewer times than `minOccurs`.
    MissingRequiredChild,
    /// A child occurred more times than `maxOccurs`.
    TooManyOccurrences,
    /// Children appeared in an order the declared sequence does not permit.
    OutOfOrderChild,
    /// A required choice branch was absent.
    MissingChoiceBranch,
    /// A required attribute was absent.
    MissingRequiredAttribute,
    /// An attribute is not declared on this element.
    UnknownAttribute,
    /// A value is outside its datatype's lexical space.
    InvalidLexicalValue,
    /// A value is not a member of the declared enumeration.
    InvalidEnumerationValue,
    /// A value does not satisfy the declared pattern facet.
    PatternMismatch,
    /// Character data appeared in element-only content.
    UnexpectedText,
    /// An element is in a namespace the schema does not define.
    ForeignNamespace,
}

/// One schema-validation failure with its location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaViolation {
    code: SchemaViolationCode,
    /// Slash-separated element path, for example `/Library/Property`.
    path: String,
    message: String,
}

impl SchemaViolation {
    #[must_use]
    pub const fn code(&self) -> SchemaViolationCode {
        self.code
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SchemaViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} at {}: {}",
            self.code, self.path, self.message
        )
    }
}

/// The outcome of validating a document against the ISO 23387 grammar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchemaReport {
    violations: Vec<SchemaViolation>,
}

impl SchemaReport {
    /// Whether the document satisfied every checked schema rule.
    #[must_use]
    pub fn is_conforming(&self) -> bool {
        self.violations.is_empty()
    }

    #[must_use]
    pub fn violations(&self) -> &[SchemaViolation] {
        &self.violations
    }

    fn push(&mut self, code: SchemaViolationCode, path: &str, message: impl Into<String>) {
        self.violations.push(SchemaViolation {
            code,
            path: path.to_owned(),
            message: message.into(),
        });
    }
}

impl Document {
    /// Validates this document against the ISO 23387 element grammar.
    ///
    /// This is a stricter, structural check than [`Document::validate`], which
    /// reports advisory diagnostics. A conforming report means every checked
    /// rule held: declared root, child sequence and cardinality, attribute
    /// presence, and datatype facets.
    #[must_use]
    pub fn validate_schema(&self) -> SchemaReport {
        let mut report = SchemaReport::default();
        let root = self.root();

        if root.namespace_uri() != Some(NAMESPACE) {
            report.push(
                SchemaViolationCode::ForeignNamespace,
                &format!("/{}", root.local_name()),
                format!(
                    "root element is in namespace {:?}, not the ISO 23387 namespace",
                    root.namespace_uri().unwrap_or("(none)")
                ),
            );
            return report;
        }

        let Some(definition) = global_root_definition(root.local_name()) else {
            report.push(
                SchemaViolationCode::UnknownRoot,
                &format!("/{}", root.local_name()),
                format!(
                    "`{}` is not a global root element declared by the schema",
                    root.local_name()
                ),
            );
            return report;
        };

        let path = format!("/{}", root.local_name());
        validate_element(root, definition, &path, &mut report);
        report
    }
}

fn global_root_definition(local_name: &str) -> Option<&'static Definition> {
    GLOBAL_ROOTS
        .iter()
        .find(|(name, _)| *name == local_name)
        .map(|(_, index)| &DEFINITIONS[*index])
}

fn validate_element(
    element: &Element,
    definition: &'static Definition,
    path: &str,
    report: &mut SchemaReport,
) {
    validate_attributes(element, definition, path, report);
    validate_content(element, definition, path, report);
}

fn validate_attributes(
    element: &Element,
    definition: &'static Definition,
    path: &str,
    report: &mut SchemaReport,
) {
    for attribute in element.attributes() {
        // Namespace declarations are not schema attributes.
        if attribute.qname() == "xmlns" || attribute.prefix() == Some("xmlns") {
            continue;
        }
        // `xml:*` and `xsi:*` are governed by their own specifications.
        if matches!(
            attribute.namespace_uri(),
            Some("http://www.w3.org/XML/1998/namespace")
                | Some("http://www.w3.org/2001/XMLSchema-instance")
        ) {
            continue;
        }

        // Schema attributes are unqualified or ISO 23387 qualified.
        let applies =
            attribute.namespace_uri().is_none() || attribute.namespace_uri() == Some(NAMESPACE);
        if !applies {
            continue;
        }

        match find_attribute(definition, attribute.local_name()) {
            Some(rule) => validate_simple_value(
                attribute.value(),
                rule.data_type,
                rule.pattern,
                rule.enum_values,
                &format!("{path}/@{}", attribute.local_name()),
                report,
            ),
            None => report.push(
                SchemaViolationCode::UnknownAttribute,
                path,
                format!(
                    "attribute `{}` is not declared on `{}`",
                    attribute.local_name(),
                    definition.handle
                ),
            ),
        }
    }

    for rule in definition.attributes.iter().filter(|rule| rule.required) {
        let present = element.attributes().iter().any(|attribute| {
            attribute.local_name() == rule.name
                && (attribute.namespace_uri().is_none()
                    || attribute.namespace_uri() == Some(NAMESPACE))
        });
        if !present {
            report.push(
                SchemaViolationCode::MissingRequiredAttribute,
                path,
                format!("required attribute `{}` is missing", rule.name),
            );
        }
    }
}

fn find_attribute(definition: &'static Definition, name: &str) -> Option<&'static AttributeRule> {
    definition.attributes.iter().find(|rule| rule.name == name)
}

fn validate_content(
    element: &Element,
    definition: &'static Definition,
    path: &str,
    report: &mut SchemaReport,
) {
    let element_only = !definition.children.is_empty();

    if element_only {
        for node in element.nodes() {
            let has_text = match node {
                Node::Text(text) => !text.chars().all(is_xml_whitespace),
                Node::CData(text) => !text.is_empty(),
                _ => false,
            };
            if has_text {
                report.push(
                    SchemaViolationCode::UnexpectedText,
                    path,
                    format!(
                        "`{}` has element-only content but carries character data",
                        definition.handle
                    ),
                );
                break;
            }
        }
    } else if let Some(data_type) = definition.data_type {
        validate_simple_value(
            &element.direct_text(),
            data_type,
            definition.pattern,
            definition.enum_values,
            path,
            report,
        );
    }

    // Walk children against the declared sequence. `cursor` only moves
    // forward, so a child matching an already-passed rule is out of order
    // rather than silently accepted.
    let mut counts = vec![0_u32; definition.children.len()];
    let mut cursor = 0_usize;

    for child in element.children() {
        if child.namespace_uri() != Some(NAMESPACE) {
            // Foreign-namespace children are retained by the parser but are
            // not part of this schema's content model.
            continue;
        }

        let child_path = format!("{path}/{}", child.local_name());
        let Some(position) = definition
            .children
            .iter()
            .position(|rule| rule.name == child.local_name())
        else {
            report.push(
                SchemaViolationCode::UnexpectedElement,
                &child_path,
                format!(
                    "`{}` is not declared as a child of `{}`",
                    child.local_name(),
                    definition.handle
                ),
            );
            continue;
        };

        let rule = &definition.children[position];

        if position < cursor && !shares_choice_group(definition, position, cursor) {
            report.push(
                SchemaViolationCode::OutOfOrderChild,
                &child_path,
                format!(
                    "`{}` appears after content the schema declares later in the sequence",
                    child.local_name()
                ),
            );
        } else {
            cursor = position;
        }

        counts[position] += 1;
        if let Some(max) = rule.max_occurs {
            if counts[position] == max + 1 {
                report.push(
                    SchemaViolationCode::TooManyOccurrences,
                    &child_path,
                    format!(
                        "`{}` occurs more than the permitted {max} time(s)",
                        child.local_name()
                    ),
                );
            }
        }

        validate_element(child, &DEFINITIONS[rule.definition], &child_path, report);
    }

    for (position, rule) in definition.children.iter().enumerate() {
        // Choice branches carry their own minimum; the group is checked below.
        if rule.choice_group.is_some() {
            continue;
        }
        if counts[position] < rule.min_occurs {
            report.push(
                SchemaViolationCode::MissingRequiredChild,
                path,
                format!(
                    "`{}` requires at least {} `{}` child element(s) but found {}",
                    definition.handle, rule.min_occurs, rule.name, counts[position]
                ),
            );
        }
    }

    for (group_index, group) in definition.choice_groups.iter().enumerate() {
        let total: u32 = definition
            .children
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.choice_group == Some(group_index))
            .map(|(position, _)| counts[position])
            .sum();

        if total < group.min_occurs {
            report.push(
                SchemaViolationCode::MissingChoiceBranch,
                path,
                format!(
                    "`{}` requires at least {} of ({}) but found {total}",
                    definition.handle,
                    group.min_occurs,
                    group.members.join(" | ")
                ),
            );
        }
        if let Some(max) = group.max_occurs {
            if total > max {
                report.push(
                    SchemaViolationCode::TooManyOccurrences,
                    path,
                    format!(
                        "`{}` permits at most {max} of ({}) but found {total}",
                        definition.handle,
                        group.members.join(" | ")
                    ),
                );
            }
        }
    }
}

/// Whether two child positions belong to the same repeating choice, in which
/// case interleaving is permitted and order carries no meaning.
fn shares_choice_group(definition: &'static Definition, left: usize, right: usize) -> bool {
    let left_group = definition.children.get(left).and_then(|r| r.choice_group);
    let right_group = definition.children.get(right).and_then(|r| r.choice_group);
    left_group.is_some() && left_group == right_group
}

/// The four characters XML Schema treats as whitespace. Unicode separators
/// such as NBSP are *not* whitespace here and must not be collapsed away.
const fn is_xml_whitespace(character: char) -> bool {
    matches!(character, '\u{0009}' | '\u{000A}' | '\u{000D}' | '\u{0020}')
}

fn collapse(value: &str) -> String {
    value
        .split(is_xml_whitespace)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn validate_simple_value(
    raw: &str,
    data_type: &str,
    pattern: Option<&str>,
    enum_values: &[&str],
    path: &str,
    report: &mut SchemaReport,
) {
    // Every datatype the ISO 23387 vocabulary uses has `collapse` whitespace
    // processing; `xs:string` itself is `preserve`.
    let value = if data_type == "xs:string" && pattern.is_none() && enum_values.is_empty() {
        raw.to_owned()
    } else {
        collapse(raw)
    };

    if !enum_values.is_empty() && !enum_values.contains(&value.as_str()) {
        report.push(
            SchemaViolationCode::InvalidEnumerationValue,
            path,
            format!(
                "`{value}` is not one of the permitted values ({})",
                enum_values.join(", ")
            ),
        );
        return;
    }

    if let Some(pattern) = pattern {
        if !matches_pattern(pattern, &value) {
            report.push(
                SchemaViolationCode::PatternMismatch,
                path,
                format!("`{value}` does not match the declared pattern"),
            );
            return;
        }
    }

    if !lexical_space_admits(data_type, &value) {
        report.push(
            SchemaViolationCode::InvalidLexicalValue,
            path,
            format!("`{value}` is not a valid `{data_type}` value"),
        );
    }
}

/// Whether `value` is in the lexical space of the named XML Schema datatype.
///
/// Only the datatypes the ISO 23387 vocabulary actually references are
/// modelled. Anything else is accepted rather than guessed at, so an unmapped
/// type can never produce a false rejection.
fn lexical_space_admits(data_type: &str, value: &str) -> bool {
    match data_type {
        "xs:string" | "xs:anySimpleType" | "xs:anyURI" => true,
        "xs:boolean" => matches!(value, "true" | "false" | "1" | "0"),
        "xs:int" => value.parse::<i32>().is_ok(),
        "xs:integer" => is_integer(value),
        "xs:positiveInteger" => is_integer(value) && !is_non_positive(value),
        "xs:decimal" => is_decimal(value),
        "xs:double" | "xs:float" => {
            matches!(value, "INF" | "-INF" | "NaN") || value.parse::<f64>().is_ok()
        }
        "xs:dateTime" => is_date_time(value),
        "xs:language" => is_language(value),
        _ => true,
    }
}

fn is_integer(value: &str) -> bool {
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_non_positive(value: &str) -> bool {
    let (negative, digits) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    negative || digits.bytes().all(|byte| byte == b'0')
}

fn is_decimal(value: &str) -> bool {
    let body = value.strip_prefix(['+', '-']).unwrap_or(value);
    let (whole, fraction) = match body.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (body, None),
    };
    if !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    match fraction {
        // At least one digit must appear on some side of the point.
        Some(fraction) => {
            fraction.bytes().all(|byte| byte.is_ascii_digit())
                && !(whole.is_empty() && fraction.is_empty())
        }
        None => !whole.is_empty(),
    }
}

/// `xs:language` is an RFC 3066 tag: `[a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*`.
fn is_language(value: &str) -> bool {
    let mut parts = value.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    if primary.is_empty() || primary.len() > 8 || !primary.bytes().all(|b| b.is_ascii_alphabetic())
    {
        return false;
    }
    parts.all(|part| {
        !part.is_empty() && part.len() <= 8 && part.bytes().all(|b| b.is_ascii_alphanumeric())
    })
}

/// `xs:dateTime`: `[-]CCYY-MM-DDThh:mm:ss[.sss][Z|(+|-)hh:mm]`.
fn is_date_time(value: &str) -> bool {
    let body = value.strip_prefix('-').unwrap_or(value);
    let Some((date, rest)) = body.split_once('T') else {
        return false;
    };

    let date_parts: Vec<&str> = date.split('-').collect();
    if date_parts.len() != 3 {
        return false;
    }
    let (year, month, day) = (date_parts[0], date_parts[1], date_parts[2]);
    if year.len() < 4 || !year.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // A four-digit year may not be zero-padded beyond four digits.
    if year.len() > 4 && year.starts_with('0') {
        return false;
    }
    if !is_fixed_digits(month, 2) || !is_fixed_digits(day, 2) {
        return false;
    }
    let month_value: u32 = month.parse().unwrap_or(0);
    let day_value: u32 = day.parse().unwrap_or(0);
    if !(1..=12).contains(&month_value) || day_value < 1 {
        return false;
    }
    let year_value: i64 = year.parse().unwrap_or(0);
    if day_value > days_in_month(year_value, month_value) {
        return false;
    }

    // Split the timezone designator off the time.
    let (time, zone) = if let Some(stripped) = rest.strip_suffix('Z') {
        (stripped, None)
    } else if let Some(position) = rest.rfind(['+', '-']) {
        (&rest[..position], Some(&rest[position + 1..]))
    } else {
        (rest, None)
    };

    if let Some(zone) = zone {
        let Some((hours, minutes)) = zone.split_once(':') else {
            return false;
        };
        if !is_fixed_digits(hours, 2) || !is_fixed_digits(minutes, 2) {
            return false;
        }
        let hour_value: u32 = hours.parse().unwrap_or(99);
        let minute_value: u32 = minutes.parse().unwrap_or(99);
        if hour_value > 14 || minute_value > 59 || (hour_value == 14 && minute_value != 0) {
            return false;
        }
    }

    let time_parts: Vec<&str> = time.split(':').collect();
    if time_parts.len() != 3 {
        return false;
    }
    if !is_fixed_digits(time_parts[0], 2) || !is_fixed_digits(time_parts[1], 2) {
        return false;
    }
    let (seconds, fraction) = match time_parts[2].split_once('.') {
        Some((seconds, fraction)) => (seconds, Some(fraction)),
        None => (time_parts[2], None),
    };
    if !is_fixed_digits(seconds, 2) {
        return false;
    }
    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
    }

    let hour_value: u32 = time_parts[0].parse().unwrap_or(99);
    let minute_value: u32 = time_parts[1].parse().unwrap_or(99);
    let second_value: u32 = seconds.parse().unwrap_or(99);
    if minute_value > 59 || second_value > 59 {
        return false;
    }
    // 24:00:00 denotes end of day and is the only permitted hour-24 form.
    if hour_value == 24 {
        return minute_value == 0 && second_value == 0;
    }
    hour_value <= 23
}

fn is_fixed_digits(value: &str, count: usize) -> bool {
    value.len() == count && value.bytes().all(|byte| byte.is_ascii_digit())
}

const fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

const fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Matches `value` against the XML Schema regular-expression subset used by
/// the ISO 23387 schema.
///
/// XSD patterns are implicitly anchored to the whole value. Rather than
/// depending on a full regex engine for two patterns, this supports exactly
/// the constructs the vocabulary uses: character classes with ranges and
/// negation, `{n}` and `{n,m}` quantifiers, `?`, `*`, `+`, groups, and
/// alternation. An unsupported construct returns `false` — a pattern that
/// cannot be evaluated must never silently pass.
fn matches_pattern(pattern: &str, value: &str) -> bool {
    let tokens: Vec<char> = pattern.chars().collect();
    let input: Vec<char> = value.chars().collect();
    match parse_alternation(&tokens, 0) {
        Some((node, next)) if next == tokens.len() => node
            .match_at(&input, 0)
            .into_iter()
            .any(|end| end == input.len()),
        _ => false,
    }
}

#[derive(Debug, Clone)]
enum Pattern {
    /// One position: a literal, a class, or a parenthesised sub-pattern.
    Repeat {
        inner: Box<Pattern>,
        min: usize,
        max: Option<usize>,
    },
    Sequence(Vec<Pattern>),
    Alternation(Vec<Pattern>),
    Class {
        negated: bool,
        items: Vec<ClassItem>,
    },
    Literal(char),
}

#[derive(Debug, Clone, Copy)]
enum ClassItem {
    Single(char),
    Range(char, char),
}

impl Pattern {
    /// All input positions this pattern can reach starting at `start`.
    ///
    /// Returning every reachable end rather than the first keeps alternation
    /// and variable repetition correct under backtracking.
    fn match_at(&self, input: &[char], start: usize) -> Vec<usize> {
        match self {
            Self::Literal(expected) => match input.get(start) {
                Some(actual) if actual == expected => vec![start + 1],
                _ => Vec::new(),
            },
            Self::Class { negated, items } => match input.get(start) {
                Some(actual) => {
                    let hit = items.iter().any(|item| match *item {
                        ClassItem::Single(character) => character == *actual,
                        ClassItem::Range(low, high) => (low..=high).contains(actual),
                    });
                    if hit != *negated {
                        vec![start + 1]
                    } else {
                        Vec::new()
                    }
                }
                None => Vec::new(),
            },
            Self::Sequence(parts) => {
                let mut positions = vec![start];
                for part in parts {
                    let mut next = Vec::new();
                    for position in positions {
                        for end in part.match_at(input, position) {
                            if !next.contains(&end) {
                                next.push(end);
                            }
                        }
                    }
                    if next.is_empty() {
                        return Vec::new();
                    }
                    positions = next;
                }
                positions
            }
            Self::Alternation(branches) => {
                let mut ends = Vec::new();
                for branch in branches {
                    for end in branch.match_at(input, start) {
                        if !ends.contains(&end) {
                            ends.push(end);
                        }
                    }
                }
                ends
            }
            Self::Repeat { inner, min, max } => {
                let mut ends = Vec::new();
                let mut frontier = vec![start];
                let mut count = 0usize;
                if *min == 0 {
                    ends.push(start);
                }
                while !frontier.is_empty() {
                    if let Some(max) = max {
                        if count >= *max {
                            break;
                        }
                    }
                    let mut next = Vec::new();
                    for position in &frontier {
                        for end in inner.match_at(input, *position) {
                            // A zero-width match would loop forever.
                            if end != *position && !next.contains(&end) {
                                next.push(end);
                            }
                        }
                    }
                    count += 1;
                    if next.is_empty() {
                        break;
                    }
                    if count >= *min {
                        for end in &next {
                            if !ends.contains(end) {
                                ends.push(*end);
                            }
                        }
                    }
                    frontier = next;
                }
                ends
            }
        }
    }
}

type Parsed = Option<(Pattern, usize)>;

fn parse_alternation(tokens: &[char], mut position: usize) -> Parsed {
    let mut branches = Vec::new();
    loop {
        let (branch, next) = parse_sequence(tokens, position)?;
        branches.push(branch);
        position = next;
        if tokens.get(position) == Some(&'|') {
            position += 1;
        } else {
            break;
        }
    }
    let node = if branches.len() == 1 {
        branches.remove(0)
    } else {
        Pattern::Alternation(branches)
    };
    Some((node, position))
}

fn parse_sequence(tokens: &[char], mut position: usize) -> Parsed {
    let mut parts = Vec::new();
    while position < tokens.len() && !matches!(tokens[position], '|' | ')') {
        let (atom, next) = parse_atom(tokens, position)?;
        let (atom, next) = parse_quantifier(atom, tokens, next)?;
        parts.push(atom);
        position = next;
    }
    Some((Pattern::Sequence(parts), position))
}

fn parse_atom(tokens: &[char], position: usize) -> Parsed {
    match tokens.get(position)? {
        '(' => {
            let (inner, next) = parse_alternation(tokens, position + 1)?;
            if tokens.get(next) != Some(&')') {
                return None;
            }
            Some((inner, next + 1))
        }
        '[' => parse_class(tokens, position + 1),
        '\\' => {
            // Escapes for regex metacharacters; `\d`-style shorthands are not
            // used by this schema and are rejected rather than approximated.
            let escaped = *tokens.get(position + 1)?;
            if escaped.is_ascii_alphanumeric() {
                return None;
            }
            Some((Pattern::Literal(escaped), position + 2))
        }
        // `.` and other unsupported metacharacters fail closed.
        '.' | '*' | '+' | '?' | '{' | '}' | ']' => None,
        literal => Some((Pattern::Literal(*literal), position + 1)),
    }
}

fn parse_class(tokens: &[char], mut position: usize) -> Parsed {
    let negated = tokens.get(position) == Some(&'^');
    if negated {
        position += 1;
    }
    let mut items = Vec::new();
    while let Some(token) = tokens.get(position) {
        if *token == ']' {
            if items.is_empty() {
                return None;
            }
            return Some((Pattern::Class { negated, items }, position + 1));
        }
        let low = if *token == '\\' {
            position += 1;
            *tokens.get(position)?
        } else {
            *token
        };
        position += 1;

        // A `-` before the closing bracket is a literal, not a range.
        if tokens.get(position) == Some(&'-') && tokens.get(position + 1) != Some(&']') {
            let high = *tokens.get(position + 1)?;
            items.push(ClassItem::Range(low, high));
            position += 2;
        } else {
            items.push(ClassItem::Single(low));
        }
    }
    None
}

fn parse_quantifier(atom: Pattern, tokens: &[char], position: usize) -> Parsed {
    let (min, max, next) = match tokens.get(position) {
        Some('?') => (0, Some(1), position + 1),
        Some('*') => (0, None, position + 1),
        Some('+') => (1, None, position + 1),
        Some('{') => {
            let close = tokens[position..].iter().position(|token| *token == '}')? + position;
            let body: String = tokens[position + 1..close].iter().collect();
            let (min, max) = match body.split_once(',') {
                None => {
                    let exact = body.parse().ok()?;
                    (exact, Some(exact))
                }
                Some((low, "")) => (low.parse().ok()?, None),
                Some((low, high)) => (low.parse().ok()?, Some(high.parse().ok()?)),
            };
            (min, max, close + 1)
        }
        _ => return Some((atom, position)),
    };
    Some((
        Pattern::Repeat {
            inner: Box::new(atom),
            min,
            max,
        },
        next,
    ))
}
