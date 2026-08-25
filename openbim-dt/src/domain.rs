//! Owned ISO 23387 type contracts used across standard boundaries.

use crate::{
    AnyUri, Base, Concept, DataTypeName, DateTime, Decimal, Language, MultiLanguageText, Rational,
    Reference, Scale,
};

/// Owned `SubjectType` core shared by object types, groups, and data templates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    concept: Concept,
    has_part_refs: Vec<Reference>,
    is_subtype_of_ref: Option<Reference>,
}

impl Subject {
    #[must_use]
    pub const fn new(concept: Concept) -> Self {
        Self {
            concept,
            has_part_refs: Vec::new(),
            is_subtype_of_ref: None,
        }
    }
    #[must_use]
    pub const fn concept(&self) -> &Concept {
        &self.concept
    }
    #[must_use]
    pub fn has_part_refs(&self) -> &[Reference] {
        &self.has_part_refs
    }
    #[must_use]
    pub const fn is_subtype_of_ref(&self) -> Option<&Reference> {
        self.is_subtype_of_ref.as_ref()
    }
    pub fn add_has_part_ref(&mut self, value: Reference) {
        self.has_part_refs.push(value);
    }
    pub fn set_is_subtype_of_ref(&mut self, value: Option<Reference>) {
        self.is_subtype_of_ref = value;
    }
}

/// ISO 23387 `ObjectTypeType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectType(Subject);
impl ObjectType {
    #[must_use]
    pub const fn new(subject: Subject) -> Self {
        Self(subject)
    }
    #[must_use]
    pub const fn subject(&self) -> &Subject {
        &self.0
    }
    #[must_use]
    pub fn into_subject(self) -> Subject {
        self.0
    }
}

/// One ordered simple value in ISO 23387 `ValueListType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataValue {
    value: String,
    order: Option<i32>,
}
impl DataValue {
    #[must_use]
    pub fn new(value: impl Into<String>, order: Option<i32>) -> Self {
        Self {
            value: value.into(),
            order,
        }
    }
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
    #[must_use]
    pub const fn order(&self) -> Option<i32> {
        self.order
    }
}

/// One language-specific ISO 23387 `ValueListType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueList {
    language: Language,
    values: Vec<DataValue>,
}
impl ValueList {
    #[must_use]
    pub fn new(language: Language, first_value: DataValue) -> Self {
        Self {
            language,
            values: vec![first_value],
        }
    }
    #[must_use]
    pub const fn language(&self) -> &Language {
        &self.language
    }
    #[must_use]
    pub fn values(&self) -> &[DataValue] {
        &self.values
    }
    pub fn add_value(&mut self, value: DataValue) {
        self.values.push(value);
    }
}

/// One constraint carried by `DataTypeType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataTypeConstraint {
    MinInclusive(String),
    MinExclusive(String),
    MaxInclusive(String),
    MaxExclusive(String),
}

/// ISO 23387 `DataTypeType`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataType {
    name: Option<DataTypeName>,
    constraints: Vec<DataTypeConstraint>,
    data_formats: Vec<String>,
    possible_values: Vec<ValueList>,
}
impl DataType {
    #[must_use]
    pub fn new(name: Option<DataTypeName>) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }
    #[must_use]
    pub const fn name(&self) -> Option<&DataTypeName> {
        self.name.as_ref()
    }
    #[must_use]
    pub fn constraints(&self) -> &[DataTypeConstraint] {
        &self.constraints
    }
    #[must_use]
    pub fn data_formats(&self) -> &[String] {
        &self.data_formats
    }
    #[must_use]
    pub fn possible_values(&self) -> &[ValueList] {
        &self.possible_values
    }
    pub fn add_constraint(&mut self, value: DataTypeConstraint) {
        self.constraints.push(value);
    }
    pub fn add_data_format(&mut self, value: impl Into<String>) {
        self.data_formats.push(value.into());
    }
    pub fn add_possible_values(&mut self, value: ValueList) {
        self.possible_values.push(value);
    }
}

/// ISO 23387 `PropertyType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    concept: Concept,
    data_type: DataType,
    symbols: Vec<String>,
    dimension_ref: Option<Reference>,
    unit_refs: Vec<Reference>,
    quantity_kind_refs: Vec<Reference>,
    dependency_refs: Vec<Reference>,
    specialization_ref: Option<Reference>,
}
impl Property {
    #[must_use]
    pub const fn new(concept: Concept, data_type: DataType) -> Self {
        Self {
            concept,
            data_type,
            symbols: Vec::new(),
            dimension_ref: None,
            unit_refs: Vec::new(),
            quantity_kind_refs: Vec::new(),
            dependency_refs: Vec::new(),
            specialization_ref: None,
        }
    }
    #[must_use]
    pub const fn concept(&self) -> &Concept {
        &self.concept
    }

    #[must_use]
    pub const fn data_type(&self) -> &DataType {
        &self.data_type
    }
    #[must_use]
    pub fn symbols(&self) -> &[String] {
        &self.symbols
    }
    #[must_use]
    pub const fn dimension_ref(&self) -> Option<&Reference> {
        self.dimension_ref.as_ref()
    }
    #[must_use]
    pub fn unit_refs(&self) -> &[Reference] {
        &self.unit_refs
    }
    #[must_use]
    pub fn quantity_kind_refs(&self) -> &[Reference] {
        &self.quantity_kind_refs
    }
    #[must_use]
    pub fn dependency_refs(&self) -> &[Reference] {
        &self.dependency_refs
    }
    #[must_use]
    pub const fn specialization_ref(&self) -> Option<&Reference> {
        self.specialization_ref.as_ref()
    }
    pub fn add_symbol(&mut self, value: impl Into<String>) {
        self.symbols.push(value.into());
    }
    pub fn set_dimension_ref(&mut self, value: Option<Reference>) {
        self.dimension_ref = value;
    }
    pub fn add_unit_ref(&mut self, value: Reference) {
        self.unit_refs.push(value);
    }
    pub fn add_quantity_kind_ref(&mut self, value: Reference) {
        self.quantity_kind_refs.push(value);
    }
    pub fn add_dependency_ref(&mut self, value: Reference) {
        self.dependency_refs.push(value);
    }
    pub fn set_specialization_ref(&mut self, value: Option<Reference>) {
        self.specialization_ref = value;
    }
}

/// ISO 23387 `QuantityKindType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantityKind {
    concept: Concept,
    dimension_ref: Reference,
}
impl QuantityKind {
    #[must_use]
    pub const fn new(concept: Concept, dimension_ref: Reference) -> Self {
        Self {
            concept,
            dimension_ref,
        }
    }
    #[must_use]
    pub const fn concept(&self) -> &Concept {
        &self.concept
    }
    #[must_use]
    pub const fn dimension_ref(&self) -> &Reference {
        &self.dimension_ref
    }
}

/// ISO 23387 `GroupOfPropertiesType`; the constructor enforces its non-empty property-reference sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupOfProperties {
    subject: Subject,
    property_refs: Vec<Reference>,
}
impl GroupOfProperties {
    #[must_use]
    pub fn new(subject: Subject, first_property_ref: Reference) -> Self {
        Self {
            subject,
            property_refs: vec![first_property_ref],
        }
    }
    #[must_use]
    pub const fn subject(&self) -> &Subject {
        &self.subject
    }
    #[must_use]
    pub fn property_refs(&self) -> &[Reference] {
        &self.property_refs
    }
    pub fn add_property_ref(&mut self, value: Reference) {
        self.property_refs.push(value);
    }
}

/// ISO 23387 `ReferenceDocumentType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceDocument {
    concept: Concept,
    languages: Vec<Language>,
    date_of_publication: Option<DateTime>,
    author: Option<String>,
    isbn: Option<String>,
    publisher: Option<String>,
    uri: Option<AnyUri>,
}
impl ReferenceDocument {
    pub fn new(concept: Concept, first_language: Language) -> Self {
        Self {
            concept,
            languages: vec![first_language],
            date_of_publication: None,
            author: None,
            isbn: None,
            publisher: None,
            uri: None,
        }
    }
    #[must_use]
    pub const fn concept(&self) -> &Concept {
        &self.concept
    }
    #[must_use]
    pub fn languages(&self) -> &[Language] {
        &self.languages
    }
    pub fn add_language(&mut self, value: Language) {
        self.languages.push(value);
    }
    #[must_use]
    pub const fn date_of_publication(&self) -> Option<&DateTime> {
        self.date_of_publication.as_ref()
    }
    pub fn set_date_of_publication(&mut self, value: Option<DateTime>) {
        self.date_of_publication = value;
    }
    #[must_use]
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }
    pub fn set_author(&mut self, value: Option<String>) {
        self.author = value;
    }
    #[must_use]
    pub fn isbn(&self) -> Option<&str> {
        self.isbn.as_deref()
    }
    pub fn set_isbn(&mut self, value: Option<String>) {
        self.isbn = value;
    }
    #[must_use]
    pub fn publisher(&self) -> Option<&str> {
        self.publisher.as_deref()
    }
    pub fn set_publisher(&mut self, value: Option<String>) {
        self.publisher = value;
    }
    #[must_use]
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_ref().map(AnyUri::as_str)
    }
    pub fn set_uri(&mut self, value: Option<AnyUri>) {
        self.uri = value;
    }
}

/// Seven SI base-dimension exponents in Annex E declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dimension {
    concept: Concept,
    exponents: [Decimal; 7],
}
impl Dimension {
    #[must_use]
    pub const fn new(concept: Concept, exponents: [Decimal; 7]) -> Self {
        Self { concept, exponents }
    }
    #[must_use]
    pub const fn concept(&self) -> &Concept {
        &self.concept
    }
    #[must_use]
    pub const fn exponents(&self) -> &[Decimal; 7] {
        &self.exponents
    }
}

/// ISO 23387 `UnitType`, with all required scalar/reference fields present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    concept: Concept,
    symbols: Vec<MultiLanguageText>,
    dimension_ref: Reference,
    scale: Scale,
    base: Base,
    coefficient: Rational,
    offset: Rational,
}
impl Unit {
    #[must_use]
    pub const fn new(
        concept: Concept,
        dimension_ref: Reference,
        scale: Scale,
        base: Base,
        coefficient: Rational,
        offset: Rational,
    ) -> Self {
        Self {
            concept,
            symbols: Vec::new(),
            dimension_ref,
            scale,
            base,
            coefficient,
            offset,
        }
    }
    #[must_use]
    pub const fn concept(&self) -> &Concept {
        &self.concept
    }
    #[must_use]
    pub fn symbols(&self) -> &[MultiLanguageText] {
        &self.symbols
    }
    pub fn add_symbol(&mut self, value: MultiLanguageText) {
        self.symbols.push(value);
    }
    #[must_use]
    pub const fn dimension_ref(&self) -> &Reference {
        &self.dimension_ref
    }
    #[must_use]
    pub const fn scale(&self) -> &Scale {
        &self.scale
    }
    #[must_use]
    pub const fn base(&self) -> &Base {
        &self.base
    }
    #[must_use]
    pub const fn coefficient(&self) -> &Rational {
        &self.coefficient
    }
    #[must_use]
    pub const fn offset(&self) -> &Rational {
        &self.offset
    }
}

/// ISO 23387 `DataTemplateType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataTemplate {
    subject: Subject,
    object_type_ref: Option<Reference>,
    property_refs: Vec<Reference>,
    group_of_properties_refs: Vec<Reference>,
}
impl DataTemplate {
    #[must_use]
    pub const fn new(subject: Subject) -> Self {
        Self {
            subject,
            object_type_ref: None,
            property_refs: Vec::new(),
            group_of_properties_refs: Vec::new(),
        }
    }
    #[must_use]
    pub const fn subject(&self) -> &Subject {
        &self.subject
    }
    #[must_use]
    pub const fn object_type_ref(&self) -> Option<&Reference> {
        self.object_type_ref.as_ref()
    }
    #[must_use]
    pub fn property_refs(&self) -> &[Reference] {
        &self.property_refs
    }
    pub fn add_property_ref(&mut self, value: Reference) {
        self.property_refs.push(value);
    }
    #[must_use]
    pub fn group_of_properties_refs(&self) -> &[Reference] {
        &self.group_of_properties_refs
    }
    pub fn add_group_of_properties_ref(&mut self, value: Reference) {
        self.group_of_properties_refs.push(value);
    }
    pub fn set_object_type_ref(&mut self, value: Option<Reference>) {
        self.object_type_ref = value;
    }
}
