//! Lossless XML codec for the owned ISO 23387 types.
//!
//! Every owned type has `to_element` and `from_element`:
//!
//! * `to_element` builds a `dt:`-qualified element named after the type's
//!   global element (`dt:ObjectType`, `dt:Unit`, ...). It emits no `xmlns`
//!   declarations: the embedding document owns them. Use
//!   [`Document::standalone`](crate::Document::standalone) for a self-contained
//!   document. Children are written in the order the schema declares them
//!   (from the generated tables), so the output passes the order checks in
//!   [`Document::validate_schema`](crate::Document::validate_schema).
//! * `from_element` reads the element's *content* and ignores its local
//!   name, because standards such as ISO 7817-3 embed ISO 23387 content under
//!   their own unqualified element (`<ObjectType>` with `dt:`-qualified
//!   children). The element itself must be ISO 23387 qualified or
//!   unqualified; any other namespace is refused.
//!
//! # Contract
//!
//! `T::from_element(&value.to_element()) == Ok(value)` for every value. The
//! reverse is a semantic projection: comments, processing instructions and
//! child order within a repeating choice are not retained. Use the wire tree
//! ([`Element`]) when those must survive.
//!
//! Decoding fails closed with a structured [`CodecError`] rather than dropping
//! anything. In particular, a child the owned model does not represent,
//! including a foreign-namespace extension, is reported as
//! [`CodecErrorKind::UnknownChild`] instead of being skipped, and an
//! attribute it cannot hold as [`CodecErrorKind::UnknownAttribute`].
//!
//! Values that are well typed but violate a schema minimum (for example a
//! [`Subject`] with neither `HasPartRef` nor `IsSubtypeOfRef`) still encode;
//! run schema validation on the result to detect that.

use std::{error::Error, fmt, str::FromStr};

use crate::{
    schema::DEFINITIONS, AnyUri, Attribute, Base, Base64Binary, Concept, DataTemplate, DataType,
    DataTypeConstraint, DataTypeName, DataValue, DateTime, Decimal, Dimension, Element,
    GroupOfProperties, Guid, Language, MultiLanguageText, Node, NonNegativeInteger, ObjectType,
    Property, QuantityKind, Rational, Reference, ReferenceDocument, Scale, Subject, Unit,
    ValueError, ValueList, NAMESPACE,
};

const PREFIX: &str = "dt";

/// Why decoding an owned value from an element failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CodecErrorKind {
    /// A required attribute is absent.
    MissingAttribute,
    /// A required child element is absent.
    MissingChild,
    /// A child occurs more often than the owned model can hold.
    TooManyChildren,
    /// A child the owned model does not represent (including extensions).
    UnknownChild,
    /// An attribute the owned model does not represent, including a schema
    /// attribute in the wrong namespace (e.g. an unqualified `GUID`).
    UnknownAttribute,
    /// An attribute or text value is outside its lexical space.
    InvalidValue,
    /// Element content where the schema declares simple content, or the reverse.
    UnexpectedContent,
}

/// A structured decoding failure: what went wrong and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecError {
    kind: CodecErrorKind,
    path: String,
    detail: String,
}

impl CodecError {
    fn new(kind: CodecErrorKind, path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            kind,
            path: path.into(),
            detail: detail.into(),
        }
    }

    /// Stable failure category.
    #[must_use]
    pub const fn kind(&self) -> CodecErrorKind {
        self.kind
    }

    /// Location relative to the decoded element: `Name[2]`, `@GUID`,
    /// `DataType/PossibleValues[1]/ValueList[1]`. Empty for the element itself.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Human-readable detail. Not stable; match on [`CodecError::kind`].
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    fn nested(mut self, parent: &str) -> Self {
        self.path = if self.path.is_empty() {
            parent.to_owned()
        } else {
            format!("{parent}/{}", self.path)
        };
        self
    }
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} at `{}`: {}",
            self.kind, self.path, self.detail
        )
    }
}

impl Error for CodecError {}

// ---------------------------------------------------------------------------
// Schema-driven ordering
// ---------------------------------------------------------------------------

/// Declared child order for a type definition, from the generated tables.
fn declared_children(handle: &str) -> &'static [crate::schema::ChildRule] {
    DEFINITIONS
        .iter()
        .find(|definition| definition.handle == handle)
        .map_or(&[], |definition| definition.children)
}

/// Accumulates children by name, then emits them in schema order.
struct ChildWriter {
    handle: &'static str,
    children: Vec<(&'static str, Element)>,
}

impl ChildWriter {
    const fn new(handle: &'static str) -> Self {
        Self {
            handle,
            children: Vec::new(),
        }
    }

    fn push(&mut self, name: &'static str, element: Element) {
        self.children.push((name, element));
    }

    fn texts(&mut self, name: &'static str, values: &[MultiLanguageText]) {
        for value in values {
            self.push(name, multilingual(name, value));
        }
    }

    fn references(&mut self, name: &'static str, values: &[Reference]) {
        for value in values {
            self.push(name, reference(name, value));
        }
    }

    fn simple<'a>(&mut self, name: &'static str, values: impl IntoIterator<Item = &'a str>) {
        for value in values {
            self.push(name, dt_element(name).with_text(value));
        }
    }

    /// Emits children grouped in declared order, preserving insertion order
    /// within each name. Panics in debug builds if a pushed name is not
    /// declared for this handle: that would be a codec bug, and the drift
    /// tests exercise every name.
    fn finish(self, mut element: Element) -> Element {
        let order = declared_children(self.handle);
        debug_assert!(
            self.children
                .iter()
                .all(|(name, _)| order.iter().any(|rule| rule.name == *name)),
            "codec wrote a child not declared on {}",
            self.handle
        );
        let mut children = self.children;
        for rule in order {
            let (matching, rest): (Vec<_>, Vec<_>) = children
                .into_iter()
                .partition(|(name, _)| *name == rule.name);
            children = rest;
            for (_, child) in matching {
                element = element.with_child(child);
            }
        }
        element
    }
}

fn dt_element(local_name: &str) -> Element {
    Element::new(Some(NAMESPACE), Some(PREFIX), local_name)
}

fn dt_attribute(local_name: &str, value: &str) -> Attribute {
    Attribute::new(Some(PREFIX), local_name, Some(NAMESPACE), value)
}

fn plain_attribute(local_name: &str, value: &str) -> Attribute {
    Attribute::new(None, local_name, None, value)
}

fn multilingual(name: &str, value: &MultiLanguageText) -> Element {
    dt_element(name)
        .with_attribute(plain_attribute("language", value.language()))
        .with_text(value.text())
}

fn reference(name: &str, value: &Reference) -> Element {
    let mut element = dt_element(name);
    if let Some(uri) = value.uri() {
        element = element.with_attribute(dt_attribute("referenceURI", uri));
    }
    if let Some(guid) = value.guid() {
        element = element.with_attribute(dt_attribute("GUID", guid.as_str()));
    }
    element
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

fn concept_attributes(element: Element, concept: &Concept) -> Element {
    let mut element = element;
    if let Some(about) = concept.about_value() {
        element = element.with_attribute(dt_attribute("about", about.as_str()));
    }
    element
        .with_attribute(dt_attribute("GUID", concept.guid().as_str()))
        .with_attribute(plain_attribute(
            "dateOfCreation",
            concept.date_of_creation_value().as_str(),
        ))
}

fn write_concept(children: &mut ChildWriter, concept: &Concept) {
    children.texts("Name", concept.names());
    children.texts("Definition", concept.definitions());
    children.references("ReferenceDocumentRef", concept.reference_document_refs());
    children.texts("Description", concept.descriptions());
    children.texts("Example", concept.examples());
    children.references("SimilarToRef", concept.similar_to_refs());
    children.simple(
        "LanguageOfCreator",
        concept.languages_of_creator().iter().map(Language::as_str),
    );
    children.simple(
        "CountryOfOrigin",
        concept.countries_of_origin().iter().map(String::as_str),
    );
    children.simple(
        "VisualRepresentation",
        concept
            .visual_representations()
            .iter()
            .map(Base64Binary::as_str),
    );
    children.simple(
        "MajorVersion",
        concept
            .major_versions()
            .iter()
            .map(NonNegativeInteger::as_str),
    );
    children.simple(
        "MinorVersion",
        concept
            .minor_versions()
            .iter()
            .map(NonNegativeInteger::as_str),
    );
    children.simple("Status", concept.statuses().iter().map(String::as_str));
    children.references("ReplacedObjectsRef", concept.replaced_objects_refs());
    children.simple(
        "DeprecationExplanation",
        concept
            .deprecation_explanations()
            .iter()
            .map(String::as_str),
    );
    children.references("DictionaryRef", concept.dictionary_refs());
}

fn write_subject(children: &mut ChildWriter, subject: &Subject) {
    write_concept(children, subject.concept());
    children.references("HasPartRef", subject.has_part_refs());
    children.references("IsSubtypeOfRef", subject.is_subtype_of_refs());
}

fn encode(
    local_name: &'static str,
    handle: &'static str,
    concept: &Concept,
    fill: impl FnOnce(&mut ChildWriter),
) -> Element {
    let mut children = ChildWriter::new(handle);
    fill(&mut children);
    children.finish(concept_attributes(dt_element(local_name), concept))
}

fn data_type_element(value: &DataType) -> Element {
    let mut element = dt_element("DataType");
    if let Some(name) = value.name() {
        element = element.with_attribute(plain_attribute("name", name.as_str()));
    }
    let mut children = ChildWriter::new("DataTypeType");
    for constraint in value.constraints() {
        let (name, bound) = match constraint {
            DataTypeConstraint::MinInclusive(bound) => ("MinInclusive", bound),
            DataTypeConstraint::MinExclusive(bound) => ("MinExclusive", bound),
            DataTypeConstraint::MaxInclusive(bound) => ("MaxInclusive", bound),
            DataTypeConstraint::MaxExclusive(bound) => ("MaxExclusive", bound),
        };
        children.push(
            name,
            dt_element(name).with_attribute(plain_attribute("value", bound)),
        );
    }
    for format in value.data_formats() {
        children.push(
            "DataFormat",
            dt_element("DataFormat").with_attribute(plain_attribute("value", format)),
        );
    }
    for list in value.possible_values() {
        let mut values = dt_element("ValueList")
            .with_attribute(plain_attribute("language", list.language().as_str()));
        for item in list.values() {
            let mut entry = dt_element("Value");
            if let Some(order) = item.order() {
                entry = entry.with_attribute(plain_attribute("order", &order.to_string()));
            }
            values = values.with_child(entry.with_text(item.value()));
        }
        children.push(
            "PossibleValues",
            dt_element("PossibleValues").with_child(values),
        );
    }
    children.finish(element)
}

impl Concept {
    /// Encodes this concept as a `dt:Subject`-free `ConceptType` element named
    /// `local_name`. Concrete types use their own `to_element`.
    #[must_use]
    pub fn to_element_named(&self, local_name: &'static str) -> Element {
        encode(local_name, "ConceptType", self, |children| {
            write_concept(children, self);
        })
    }
}

impl Subject {
    /// Encodes as a global `dt:Subject` element.
    #[must_use]
    pub fn to_element(&self) -> Element {
        encode("Subject", "SubjectType", self.concept(), |children| {
            write_subject(children, self);
        })
    }
}

impl ObjectType {
    /// Encodes as a global `dt:ObjectType` element.
    #[must_use]
    pub fn to_element(&self) -> Element {
        let subject = self.subject();
        encode(
            "ObjectType",
            "ObjectTypeType",
            subject.concept(),
            |children| {
                write_subject(children, subject);
            },
        )
    }
}

impl GroupOfProperties {
    /// Encodes as a global `dt:GroupOfProperties` element.
    #[must_use]
    pub fn to_element(&self) -> Element {
        let subject = self.subject();
        encode(
            "GroupOfProperties",
            "GroupOfPropertiesType",
            subject.concept(),
            |children| {
                write_subject(children, subject);
                children.references("HasPropertyRef", self.property_refs());
            },
        )
    }
}

impl DataTemplate {
    /// Encodes as a global `dt:DataTemplate` element.
    #[must_use]
    pub fn to_element(&self) -> Element {
        let subject = self.subject();
        encode(
            "DataTemplate",
            "DataTemplateType",
            subject.concept(),
            |children| {
                write_subject(children, subject);
                children.references("HasObjectTypeRef", self.object_type_refs());
                children.references("HasPropertyRef", self.property_refs());
                children.references("HasGroupOfPropertiesRef", self.group_of_properties_refs());
            },
        )
    }
}

impl Property {
    /// Encodes as a global `dt:Property` element.
    #[must_use]
    pub fn to_element(&self) -> Element {
        encode("Property", "PropertyType", self.concept(), |children| {
            write_concept(children, self.concept());
            children.push("DataType", data_type_element(self.data_type()));
            children.simple("Symbol", self.symbols().iter().map(String::as_str));
            children.references("DimensionRef", self.dimension_refs());
            children.references("UnitRef", self.unit_refs());
            children.references("QuantityKindRef", self.quantity_kind_refs());
            children.references("IsDependentOnRef", self.dependency_refs());
            children.references(
                "IsSpecializationOfRef",
                self.specialization_ref().map_or(&[], std::slice::from_ref),
            );
        })
    }
}

impl QuantityKind {
    /// Encodes as a `dt:QuantityKind` element (library member, not a global root).
    #[must_use]
    pub fn to_element(&self) -> Element {
        encode(
            "QuantityKind",
            "QuantityKindType",
            self.concept(),
            |children| {
                write_concept(children, self.concept());
                children.references("DimensionRef", std::slice::from_ref(self.dimension_ref()));
            },
        )
    }
}

/// Exponent element names in the order of [`Dimension::exponents`].
pub(crate) const DIMENSION_EXPONENTS: [&str; 7] = [
    "DimensionExponentForAmountOfSubstance",
    "DimensionExponentForElectricCurrent",
    "DimensionExponentForLength",
    "DimensionExponentForLuminousIntensity",
    "DimensionExponentForMass",
    "DimensionExponentForThermodynamicTemperature",
    "DimensionExponentForTime",
];

impl Dimension {
    /// Encodes as a `dt:Dimension` element (library member, not a global root).
    #[must_use]
    pub fn to_element(&self) -> Element {
        encode("Dimension", "DimensionType", self.concept(), |children| {
            write_concept(children, self.concept());
            for (name, exponent) in DIMENSION_EXPONENTS.into_iter().zip(self.exponents()) {
                children.simple(name, [exponent.as_str()]);
            }
        })
    }
}

impl Unit {
    /// Encodes as a `dt:Unit` element (library member, not a global root).
    #[must_use]
    pub fn to_element(&self) -> Element {
        encode("Unit", "UnitType", self.concept(), |children| {
            write_concept(children, self.concept());
            children.texts("Symbol", self.symbols());
            children.references("DimensionRef", std::slice::from_ref(self.dimension_ref()));
            children.simple("Scale", [self.scale().as_str()]);
            children.simple("Base", [self.base().as_str()]);
            children.simple("Coefficient", [self.coefficient().as_str()]);
            children.simple("Offset", [self.offset().as_str()]);
        })
    }
}

impl ReferenceDocument {
    /// Encodes as a `dt:ReferenceDocument` element (library member).
    #[must_use]
    pub fn to_element(&self) -> Element {
        encode(
            "ReferenceDocument",
            "ReferenceDocumentType",
            self.concept(),
            |children| {
                write_concept(children, self.concept());
                children.simple(
                    "DateOfPublication",
                    self.date_of_publication().map(DateTime::as_str),
                );
                children.simple("Author", self.author());
                children.simple("ISBN", self.isbn());
                children.simple("Language", self.languages().iter().map(Language::as_str));
                children.simple("Publisher", self.publisher());
                children.simple("URI", self.uri());
            },
        )
    }
}

// ---------------------------------------------------------------------------
// Decoding
// ---------------------------------------------------------------------------

/// Children of one element, indexed for positional error paths and checked
/// off as the decoder consumes them. Anything left over is an unknown child.
struct Reader<'a> {
    element: &'a Element,
    consumed: Vec<bool>,
}

impl<'a> Reader<'a> {
    fn new(element: &'a Element) -> Result<Self, CodecError> {
        for node in element.nodes() {
            if let Node::Text(text) | Node::CData(text) = node {
                if !text.chars().all(|c| matches!(c, ' ' | '\t' | '\r' | '\n')) {
                    return Err(CodecError::new(
                        CodecErrorKind::UnexpectedContent,
                        "",
                        "character data in element-only content",
                    ));
                }
            }
        }
        let count = element.children().count();
        Ok(Self {
            element,
            consumed: vec![false; count],
        })
    }

    /// Every dt child named `name`, with its 1-based path index.
    fn take(&mut self, name: &str) -> Vec<(String, &'a Element)> {
        let mut index = 0;
        let mut found = Vec::new();
        for (position, child) in self.element.children().enumerate() {
            if child.namespace_uri() == Some(NAMESPACE) && child.local_name() == name {
                index += 1;
                self.consumed[position] = true;
                found.push((format!("{name}[{index}]"), child));
            }
        }
        found
    }

    fn texts(&mut self, name: &str) -> Result<Vec<MultiLanguageText>, CodecError> {
        self.take(name)
            .into_iter()
            .map(|(path, child)| read_multilingual(child).map_err(|error| error.nested(&path)))
            .collect()
    }

    fn references(&mut self, name: &str) -> Result<Vec<Reference>, CodecError> {
        self.take(name)
            .into_iter()
            .map(|(path, child)| read_reference(child).map_err(|error| error.nested(&path)))
            .collect()
    }

    fn simple<T>(
        &mut self,
        name: &str,
        parse: impl Fn(&str) -> Result<T, ValueError>,
    ) -> Result<Vec<T>, CodecError> {
        self.take(name)
            .into_iter()
            .map(|(path, child)| {
                refuse_unrepresented_attributes(child, &[]).map_err(|error| error.nested(&path))?;
                let text = simple_text(child).map_err(|error| error.nested(&path))?;
                parse(&text).map_err(|error| invalid(&path, &error))
            })
            .collect()
    }

    fn strings(&mut self, name: &str) -> Result<Vec<String>, CodecError> {
        self.simple(name, |text| Ok(text.to_owned()))
    }

    fn at_most_one<T>(name: &str, mut values: Vec<T>) -> Result<Option<T>, CodecError> {
        if values.len() > 1 {
            return Err(CodecError::new(
                CodecErrorKind::TooManyChildren,
                format!("{name}[2]"),
                format!(
                    "`{name}` occurs {} times; at most one is declared",
                    values.len()
                ),
            ));
        }
        Ok(values.pop())
    }

    fn exactly_one<T>(name: &str, values: Vec<T>) -> Result<T, CodecError> {
        Self::at_most_one(name, values)?.ok_or_else(|| {
            CodecError::new(
                CodecErrorKind::MissingChild,
                "",
                format!("required child `{name}` is missing"),
            )
        })
    }

    /// Fails on the first child nothing consumed.
    fn finish(self) -> Result<(), CodecError> {
        let unknown = self
            .element
            .children()
            .zip(&self.consumed)
            .find(|(_, consumed)| !**consumed);
        match unknown {
            None => Ok(()),
            Some((child, _)) => Err(CodecError::new(
                CodecErrorKind::UnknownChild,
                child.qname(),
                format!(
                    "`{}` ({}) is not represented by the owned model",
                    child.local_name(),
                    child.namespace_uri().unwrap_or("no namespace")
                ),
            )),
        }
    }
}

/// An attribute identity: namespace and local name.
type AttributeName = (Option<&'static str>, &'static str);

/// `ConceptType` attributes. `about` and `GUID` are global attribute
/// declarations, hence namespace-qualified; `dateOfCreation` is local and
/// unqualified.
const CONCEPT_ATTRIBUTES: &[AttributeName] = &[
    (Some(NAMESPACE), "about"),
    (Some(NAMESPACE), "GUID"),
    (None, "dateOfCreation"),
];
const REFERENCE_ATTRIBUTES: &[AttributeName] =
    &[(Some(NAMESPACE), "GUID"), (Some(NAMESPACE), "referenceURI")];
const LANGUAGE_ATTRIBUTE: &[AttributeName] = &[(None, "language")];
const VALUE_ATTRIBUTE: &[AttributeName] = &[(None, "value")];

/// Refuses any attribute the owned model cannot hold, so decoding never
/// silently drops content. Namespace declarations are syntax, not content.
fn refuse_unrepresented_attributes(
    element: &Element,
    allowed: &[AttributeName],
) -> Result<(), CodecError> {
    for attribute in element.attributes() {
        if attribute.qname() == "xmlns" || attribute.prefix() == Some("xmlns") {
            continue;
        }
        let namespace = attribute.namespace_uri();
        let local_name = attribute.local_name();
        if !allowed
            .iter()
            .any(|(ns, name)| *ns == namespace && *name == local_name)
        {
            return Err(CodecError::new(
                CodecErrorKind::UnknownAttribute,
                format!("@{}", attribute.qname()),
                format!(
                    "attribute `{local_name}` ({}) is not represented by the owned model",
                    namespace.unwrap_or("no namespace")
                ),
            ));
        }
    }
    Ok(())
}

/// Refuses child elements and non-whitespace text in empty content.
fn empty_content(element: &Element) -> Result<(), CodecError> {
    let text = simple_text(element)?;
    if text.chars().all(|c| matches!(c, ' ' | '\t' | '\r' | '\n')) {
        Ok(())
    } else {
        Err(CodecError::new(
            CodecErrorKind::UnexpectedContent,
            "",
            "character data where empty content is declared",
        ))
    }
}

fn invalid(path: &str, error: &ValueError) -> CodecError {
    CodecError::new(CodecErrorKind::InvalidValue, path, error.to_string())
}

/// Text content of a simple-content element; child elements are refused.
fn simple_text(element: &Element) -> Result<String, CodecError> {
    if element.children().next().is_some() {
        return Err(CodecError::new(
            CodecErrorKind::UnexpectedContent,
            "",
            "element content where simple content is declared",
        ));
    }
    Ok(element.direct_text())
}

fn read_multilingual(element: &Element) -> Result<MultiLanguageText, CodecError> {
    refuse_unrepresented_attributes(element, LANGUAGE_ATTRIBUTE)?;
    let language = element.attribute_ns(None, "language").ok_or_else(|| {
        CodecError::new(
            CodecErrorKind::MissingAttribute,
            "@language",
            "required attribute `language` is missing",
        )
    })?;
    let text = simple_text(element)?;
    MultiLanguageText::new(language, text).map_err(|error| invalid("@language", &error))
}

fn read_reference(element: &Element) -> Result<Reference, CodecError> {
    refuse_unrepresented_attributes(element, REFERENCE_ATTRIBUTES)?;
    empty_content(element)?;
    let guid = element
        .attribute_ns(Some(NAMESPACE), "GUID")
        .map(Guid::from_str)
        .transpose()
        .map_err(|error| invalid("@GUID", &error))?;
    let uri = element
        .attribute_ns(Some(NAMESPACE), "referenceURI")
        .map(AnyUri::from_str)
        .transpose()
        .map_err(|error| invalid("@referenceURI", &error))?;
    Ok(Reference::new(guid, uri))
}

fn required_attribute<'a>(
    element: &'a Element,
    namespace: Option<&str>,
    name: &str,
) -> Result<&'a str, CodecError> {
    element.attribute_ns(namespace, name).ok_or_else(|| {
        CodecError::new(
            CodecErrorKind::MissingAttribute,
            format!("@{name}"),
            format!("required attribute `{name}` is missing"),
        )
    })
}

fn read_concept(element: &Element, children: &mut Reader<'_>) -> Result<Concept, CodecError> {
    refuse_unrepresented_attributes(element, CONCEPT_ATTRIBUTES)?;
    let guid = Guid::from_str(required_attribute(element, Some(NAMESPACE), "GUID")?)
        .map_err(|error| invalid("@GUID", &error))?;
    let date = DateTime::from_str(required_attribute(element, None, "dateOfCreation")?)
        .map_err(|error| invalid("@dateOfCreation", &error))?;
    let mut concept = Concept::from_identity(guid, date);
    if let Some(about) = element.attribute_ns(Some(NAMESPACE), "about") {
        concept.set_about(Some(
            AnyUri::from_str(about).map_err(|error| invalid("@about", &error))?,
        ));
    }
    for value in children.texts("Name")? {
        concept.add_name(value);
    }
    for value in children.texts("Definition")? {
        concept.add_definition(value);
    }
    for value in children.references("ReferenceDocumentRef")? {
        concept.add_reference_document_ref(value);
    }
    for value in children.texts("Description")? {
        concept.add_description(value);
    }
    for value in children.texts("Example")? {
        concept.add_example(value);
    }
    for value in children.references("SimilarToRef")? {
        concept.add_similar_to_ref(value);
    }
    for value in children.simple("LanguageOfCreator", Language::from_str)? {
        concept.add_language_of_creator(value);
    }
    for value in children.strings("CountryOfOrigin")? {
        concept.add_country_of_origin(value);
    }
    for value in children.simple("VisualRepresentation", Base64Binary::from_str)? {
        concept.add_visual_representation(value);
    }
    for value in children.simple("MajorVersion", NonNegativeInteger::from_str)? {
        concept.add_major_version(value);
    }
    for value in children.simple("MinorVersion", NonNegativeInteger::from_str)? {
        concept.add_minor_version(value);
    }
    for value in children.strings("Status")? {
        concept.add_status(value);
    }
    for value in children.references("ReplacedObjectsRef")? {
        concept.add_replaced_objects_ref(value);
    }
    for value in children.strings("DeprecationExplanation")? {
        concept.add_deprecation_explanation(value);
    }
    for value in children.references("DictionaryRef")? {
        concept.add_dictionary_ref(value);
    }
    Ok(concept)
}

fn read_subject(element: &Element, children: &mut Reader<'_>) -> Result<Subject, CodecError> {
    let mut subject = Subject::new(read_concept(element, children)?);
    for value in children.references("HasPartRef")? {
        subject.add_has_part_ref(value);
    }
    for value in children.references("IsSubtypeOfRef")? {
        subject.add_is_subtype_of_ref(value);
    }
    Ok(subject)
}

fn read_data_type(element: &Element) -> Result<DataType, CodecError> {
    refuse_unrepresented_attributes(element, &[(None, "name")])?;
    let mut children = Reader::new(element)?;
    let mut value = DataType::new(element.attribute_ns(None, "name").map(DataTypeName::from));
    // Constraints are one choice group; read them in document order so a
    // mixed Min/Max sequence round-trips in its original order.
    for (position, child) in element.children().enumerate() {
        if child.namespace_uri() != Some(NAMESPACE) {
            continue;
        }
        let make: fn(String) -> DataTypeConstraint = match child.local_name() {
            "MinInclusive" => DataTypeConstraint::MinInclusive,
            "MinExclusive" => DataTypeConstraint::MinExclusive,
            "MaxInclusive" => DataTypeConstraint::MaxInclusive,
            "MaxExclusive" => DataTypeConstraint::MaxExclusive,
            _ => continue,
        };
        children.consumed[position] = true;
        let path = format!("{}[{}]", child.local_name(), position + 1);
        refuse_unrepresented_attributes(child, VALUE_ATTRIBUTE)
            .map_err(|error| error.nested(&path))?;
        let bound = child.attribute_ns(None, "value").ok_or_else(|| {
            CodecError::new(
                CodecErrorKind::MissingAttribute,
                format!("{path}/@value"),
                "a bound without `value` cannot be represented",
            )
        })?;
        simple_text(child).map_err(|error| error.nested(&path))?;
        empty_content(child).map_err(|error| error.nested(&path))?;
        value.add_constraint(make(bound.to_owned()));
    }
    for (path, child) in children.take("DataFormat") {
        refuse_unrepresented_attributes(child, VALUE_ATTRIBUTE)
            .map_err(|error| error.nested(&path))?;
        empty_content(child).map_err(|error| error.nested(&path))?;
        let format = child.attribute_ns(None, "value").ok_or_else(|| {
            CodecError::new(
                CodecErrorKind::MissingAttribute,
                format!("{path}/@value"),
                "a data format without `value` cannot be represented",
            )
        })?;
        value.add_data_format(format);
    }
    for (path, possible) in children.take("PossibleValues") {
        refuse_unrepresented_attributes(possible, &[]).map_err(|error| error.nested(&path))?;
        let mut lists = Reader::new(possible).map_err(|error| error.nested(&path))?;
        for (list_path, list) in lists.take("ValueList") {
            let nested = format!("{path}/{list_path}");
            value
                .add_possible_values(read_value_list(list).map_err(|error| error.nested(&nested))?);
        }
        lists.finish().map_err(|error| error.nested(&path))?;
    }
    children.finish()?;
    Ok(value)
}

fn read_value_list(element: &Element) -> Result<ValueList, CodecError> {
    refuse_unrepresented_attributes(element, LANGUAGE_ATTRIBUTE)?;
    let language = Language::from_str(required_attribute(element, None, "language")?)
        .map_err(|error| invalid("@language", &error))?;
    let mut children = Reader::new(element)?;
    let mut values = Vec::new();
    for (path, child) in children.take("Value") {
        refuse_unrepresented_attributes(child, &[(None, "order")])
            .map_err(|error| error.nested(&path))?;
        let order = child
            .attribute_ns(None, "order")
            .map(|order| {
                order.trim().parse::<i32>().map_err(|_| {
                    CodecError::new(
                        CodecErrorKind::InvalidValue,
                        format!("{path}/@order"),
                        format!("`{order}` is not an xs:int"),
                    )
                })
            })
            .transpose()?;
        let text = simple_text(child).map_err(|error| error.nested(&path))?;
        values.push(DataValue::new(text, order));
    }
    children.finish()?;
    let mut values = values.into_iter();
    let first = values.next().ok_or_else(|| {
        CodecError::new(
            CodecErrorKind::MissingChild,
            "",
            "a value list requires at least one `Value`",
        )
    })?;
    let mut list = ValueList::new(language, first);
    for value in values {
        list.add_value(value);
    }
    Ok(list)
}

/// Decodes one owned value. The element's local name is deliberately not
/// checked: the same type appears under several names (a `Unit` is also a
/// `Library` child, LOIN embeds `ObjectType` content under its own element),
/// so the caller chooses. Its namespace must be ISO 23387, or it has none
/// when a host standard such as LOIN wraps DT content in a local element.
fn decode<T>(
    element: &Element,
    read: impl FnOnce(&mut Reader<'_>) -> Result<T, CodecError>,
) -> Result<T, CodecError> {
    if !matches!(element.namespace_uri(), None | Some(NAMESPACE)) {
        return Err(CodecError::new(
            CodecErrorKind::UnexpectedContent,
            "",
            format!(
                "`{}` is in namespace {}, not ISO 23387",
                element.local_name(),
                element.namespace_uri().unwrap_or_default()
            ),
        ));
    }
    let mut children = Reader::new(element)?;
    let value = read(&mut children)?;
    children.finish()?;
    Ok(value)
}

impl Concept {
    /// Decodes plain `ConceptType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| read_concept(element, children))
    }
}

impl Subject {
    /// Decodes `SubjectType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| read_subject(element, children))
    }
}

impl ObjectType {
    /// Decodes `ObjectTypeType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| {
            read_subject(element, children).map(Self::new)
        })
    }
}

impl GroupOfProperties {
    /// Decodes `GroupOfPropertiesType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| {
            let subject = read_subject(element, children)?;
            let mut refs = children.references("HasPropertyRef")?.into_iter();
            // The owned type requires one property ref, matching the schema's
            // minimum of one for its choice group.
            let first = refs.next().ok_or_else(|| {
                CodecError::new(
                    CodecErrorKind::MissingChild,
                    "",
                    "required child `HasPropertyRef` is missing",
                )
            })?;
            let mut group = Self::new(subject, first);
            for value in refs {
                group.add_property_ref(value);
            }
            Ok(group)
        })
    }
}

impl DataTemplate {
    /// Decodes `DataTemplateType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| {
            let mut template = Self::new(read_subject(element, children)?);
            for value in children.references("HasObjectTypeRef")? {
                template.add_object_type_ref(value);
            }
            for value in children.references("HasPropertyRef")? {
                template.add_property_ref(value);
            }
            for value in children.references("HasGroupOfPropertiesRef")? {
                template.add_group_of_properties_ref(value);
            }
            Ok(template)
        })
    }
}

impl Property {
    /// Decodes `PropertyType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| {
            let concept = read_concept(element, children)?;
            let data_type = Reader::exactly_one("DataType", children.take("DataType"))?;
            let data_type =
                read_data_type(data_type.1).map_err(|error| error.nested(&data_type.0))?;
            let mut property = Self::new(concept, data_type);
            for value in children.strings("Symbol")? {
                property.add_symbol(value);
            }
            for value in children.references("DimensionRef")? {
                property.add_dimension_ref(value);
            }
            for value in children.references("UnitRef")? {
                property.add_unit_ref(value);
            }
            for value in children.references("QuantityKindRef")? {
                property.add_quantity_kind_ref(value);
            }
            for value in children.references("IsDependentOnRef")? {
                property.add_dependency_ref(value);
            }
            property.set_specialization_ref(Reader::at_most_one(
                "IsSpecializationOfRef",
                children.references("IsSpecializationOfRef")?,
            )?);
            Ok(property)
        })
    }
}

impl QuantityKind {
    /// Decodes `QuantityKindType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| {
            let concept = read_concept(element, children)?;
            let dimension =
                Reader::exactly_one("DimensionRef", children.references("DimensionRef")?)?;
            Ok(Self::new(concept, dimension))
        })
    }
}

impl Dimension {
    /// Decodes `DimensionType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| {
            let concept = read_concept(element, children)?;
            let mut exponents = Vec::with_capacity(DIMENSION_EXPONENTS.len());
            for name in DIMENSION_EXPONENTS {
                exponents.push(Reader::exactly_one(
                    name,
                    children.simple(name, Decimal::from_str)?,
                )?);
            }
            let exponents: [Decimal; 7] = exponents
                .try_into()
                .unwrap_or_else(|_| unreachable!("seven names yield seven exponents"));
            Ok(Self::new(concept, exponents))
        })
    }
}

impl Unit {
    /// Decodes `UnitType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| {
            let concept = read_concept(element, children)?;
            let symbols = children.texts("Symbol")?;
            let dimension =
                Reader::exactly_one("DimensionRef", children.references("DimensionRef")?)?;
            let scale = Reader::exactly_one("Scale", children.strings("Scale")?)?;
            let base = Reader::exactly_one("Base", children.strings("Base")?)?;
            let coefficient = Reader::exactly_one(
                "Coefficient",
                children.simple("Coefficient", Rational::from_str)?,
            )?;
            let offset =
                Reader::exactly_one("Offset", children.simple("Offset", Rational::from_str)?)?;
            let mut unit = Self::new(
                concept,
                dimension,
                Scale::from(scale.as_str()),
                Base::from(base.as_str()),
                coefficient,
                offset,
            );
            for symbol in symbols {
                unit.add_symbol(symbol);
            }
            Ok(unit)
        })
    }
}

impl ReferenceDocument {
    /// Decodes `ReferenceDocumentType` content (any element name).
    pub fn from_element(element: &Element) -> Result<Self, CodecError> {
        decode(element, |children| {
            let concept = read_concept(element, children)?;
            let mut languages = children.simple("Language", Language::from_str)?.into_iter();
            let first = languages.next().ok_or_else(|| {
                CodecError::new(
                    CodecErrorKind::MissingChild,
                    "",
                    "required child `Language` is missing",
                )
            })?;
            let mut document = Self::new(concept, first);
            for language in languages {
                document.add_language(language);
            }
            document.set_date_of_publication(Reader::at_most_one(
                "DateOfPublication",
                children.simple("DateOfPublication", DateTime::from_str)?,
            )?);
            document.set_author(Reader::at_most_one("Author", children.strings("Author")?)?);
            document.set_isbn(Reader::at_most_one("ISBN", children.strings("ISBN")?)?);
            document.set_publisher(Reader::at_most_one(
                "Publisher",
                children.strings("Publisher")?,
            )?);
            document.set_uri(Reader::at_most_one(
                "URI",
                children.simple("URI", AnyUri::from_str)?,
            )?);
            Ok(document)
        })
    }
}

#[cfg(test)]
mod drift {
    //! Drift guard: the codec must write every child element the generated
    //! schema tables declare for each owned type, so a schema regeneration
    //! that adds a child fails here instead of being silently dropped.

    use super::*;
    use crate::schema::DEFINITIONS;

    fn declared(handle: &str) -> Vec<&'static str> {
        let definition = DEFINITIONS
            .iter()
            .find(|definition| definition.handle == handle)
            .unwrap_or_else(|| panic!("schema tables no longer define {handle}"));
        definition.children.iter().map(|rule| rule.name).collect()
    }

    fn written(element: &Element) -> Vec<String> {
        let mut names: Vec<String> = element
            .children()
            .map(|child| child.local_name().to_owned())
            .collect();
        names.dedup();
        names
    }

    fn assert_writes_every_declared_child(handle: &str, element: &Element) {
        let written = written(element);
        let missing: Vec<_> = declared(handle)
            .into_iter()
            .filter(|name| !written.iter().any(|w| w == name))
            .collect();
        assert!(
            missing.is_empty(),
            "{handle}: codec never writes {missing:?}"
        );
    }

    // The same maximal fixtures the integration tests round-trip.
    use crate::{
        Base, Base64Binary, DataTypeConstraint, DataTypeName, DataValue, Decimal, Language,
        NonNegativeInteger, Rational, Scale, ValueList,
    };
    include!("../tests/support/maximal.rs");

    #[test]
    fn codec_writes_every_child_the_schema_declares() {
        let cases = [
            (
                "ObjectTypeType",
                ObjectType::new(maximal_subject(1)).to_element(),
            ),
            ("PropertyType", maximal_property().to_element()),
            ("GroupOfPropertiesType", maximal_group().to_element()),
            ("DataTemplateType", maximal_template().to_element()),
            ("QuantityKindType", maximal_quantity_kind().to_element()),
            ("DimensionType", maximal_dimension().to_element()),
            ("UnitType", maximal_unit().to_element()),
            (
                "ReferenceDocumentType",
                maximal_reference_document().to_element(),
            ),
        ];
        for (handle, element) in &cases {
            assert_writes_every_declared_child(handle, element);
        }
    }
}
