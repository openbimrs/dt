//! Typed ISO 23387 views over the lossless XML tree.

use std::{collections::HashMap, error::Error, fmt, str::FromStr};

use crate::{
    AnyUri, DataTypeName, DateTime, Document, Element, Guid, MultiLanguageText, Reference,
    ValueError, DRAFT_PLACEHOLDER_NAMESPACE, NAMESPACE,
};

/// Recognized ISO 23387 element kinds, including local `Library` children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementKind {
    Library,
    Subject,
    DataTemplate,
    ObjectType,
    GroupOfProperties,
    Property,
    Unit,
    Dimension,
    QuantityKind,
    ReferenceDocument,
}

impl ElementKind {
    /// Whether Annex E declares this kind as a concrete global document root.
    #[must_use]
    pub const fn is_global_root(self) -> bool {
        matches!(
            self,
            Self::Library
                | Self::DataTemplate
                | Self::ObjectType
                | Self::GroupOfProperties
                | Self::Property
        )
    }

    #[must_use]
    pub fn from_element(element: &Element) -> Option<Self> {
        if element.namespace_uri() != Some(NAMESPACE) {
            return None;
        }
        match element.local_name() {
            "Library" => Some(Self::Library),
            "Subject" => Some(Self::Subject),
            "DataTemplate" => Some(Self::DataTemplate),
            "ObjectType" => Some(Self::ObjectType),
            "GroupOfProperties" => Some(Self::GroupOfProperties),
            "Property" => Some(Self::Property),
            "Unit" => Some(Self::Unit),
            "Dimension" => Some(Self::Dimension),
            "QuantityKind" => Some(Self::QuantityKind),
            "ReferenceDocument" => Some(Self::ReferenceDocument),
            _ => None,
        }
    }
}

impl Document {
    #[must_use]
    pub fn root_kind(&self) -> Option<ElementKind> {
        ElementKind::from_element(self.root()).filter(|kind| kind.is_global_root())
    }

    #[must_use]
    pub fn library(&self) -> Option<Library<'_>> {
        (self.root_kind() == Some(ElementKind::Library)).then(|| Library {
            element: self.root(),
        })
    }

    /// Separates the root for embedding or conversion into an owned DT type.
    #[must_use]
    pub fn into_root_element(self) -> Element {
        self.take_root()
    }
}

/// Borrowed ISO 23387 library view.
#[derive(Debug, Clone, Copy)]
pub struct Library<'a> {
    element: &'a Element,
}

impl<'a> Library<'a> {
    #[must_use]
    pub const fn element(self) -> &'a Element {
        self.element
    }

    pub fn guid(self) -> Result<Guid, ValueError> {
        required_guid(self.element)
    }

    pub fn items(self) -> impl Iterator<Item = LibraryItem<'a>> {
        self.element.children().map(classify_library_item)
    }
}

/// One child retained by a library, including forward-compatible extensions.
#[derive(Debug, Clone, Copy)]
pub enum LibraryItem<'a> {
    Name(MultilingualTextRef<'a>),
    DataTemplate(DataTemplateRef<'a>),
    ObjectType(ConceptRef<'a>),
    GroupOfProperties(ConceptRef<'a>),
    Property(PropertyRef<'a>),
    Unit(ConceptRef<'a>),
    Dimension(ConceptRef<'a>),
    QuantityKind(ConceptRef<'a>),
    ReferenceDocument(ConceptRef<'a>),
    Extension(&'a Element),
}

fn classify_library_item(element: &Element) -> LibraryItem<'_> {
    if is_dt(element, "Name") {
        return LibraryItem::Name(MultilingualTextRef { element });
    }
    match ElementKind::from_element(element) {
        Some(ElementKind::DataTemplate) => LibraryItem::DataTemplate(DataTemplateRef { element }),
        Some(ElementKind::ObjectType) => LibraryItem::ObjectType(ConceptRef { element }),
        Some(ElementKind::GroupOfProperties) => {
            LibraryItem::GroupOfProperties(ConceptRef { element })
        }
        Some(ElementKind::Property) => LibraryItem::Property(PropertyRef { element }),
        Some(ElementKind::Unit) => LibraryItem::Unit(ConceptRef { element }),
        Some(ElementKind::Dimension) => LibraryItem::Dimension(ConceptRef { element }),
        Some(ElementKind::QuantityKind) => LibraryItem::QuantityKind(ConceptRef { element }),
        Some(ElementKind::ReferenceDocument) => {
            LibraryItem::ReferenceDocument(ConceptRef { element })
        }
        _ => LibraryItem::Extension(element),
    }
}

/// Borrowed multilingual text typed by ISO 23387.
#[derive(Debug, Clone, Copy)]
pub struct MultilingualTextRef<'a> {
    element: &'a Element,
}

impl<'a> MultilingualTextRef<'a> {
    #[must_use]
    pub fn language(self) -> Option<&'a str> {
        self.element.attribute_ns(None, "language")
    }

    #[must_use]
    pub fn text(self) -> String {
        self.element.direct_text()
    }

    pub fn to_owned(self) -> Result<MultiLanguageText, ValueError> {
        MultiLanguageText::new(self.language().unwrap_or_default(), self.text())
    }
}

/// Borrowed reference typed by ISO 23387.
#[derive(Debug, Clone, Copy)]
pub struct ReferenceRef<'a> {
    element: &'a Element,
}

impl<'a> ReferenceRef<'a> {
    pub fn guid(self) -> Option<Result<Guid, ValueError>> {
        self.element
            .attribute_ns(Some(NAMESPACE), "GUID")
            .map(Guid::from_str)
    }

    #[must_use]
    pub fn uri(self) -> Option<&'a str> {
        self.element.attribute_ns(Some(NAMESPACE), "referenceURI")
    }

    pub fn to_owned(self) -> Result<Reference, ValueError> {
        let guid = self.guid().transpose()?;
        let uri = self.uri().map(str::parse).transpose()?;
        Ok(Reference::new(guid, uri))
    }
}

/// Shared borrowed view for every `ConceptType`-derived element.
#[derive(Debug, Clone, Copy)]
pub struct ConceptRef<'a> {
    element: &'a Element,
}

impl<'a> ConceptRef<'a> {
    #[must_use]
    pub const fn element(self) -> &'a Element {
        self.element
    }

    pub fn guid(self) -> Result<Guid, ValueError> {
        required_guid(self.element)
    }

    #[must_use]
    pub fn date_of_creation(self) -> Option<&'a str> {
        self.element.attribute_ns(None, "dateOfCreation")
    }

    pub fn names(self) -> impl Iterator<Item = MultilingualTextRef<'a>> {
        self.element
            .children()
            .filter(|element| is_dt(element, "Name"))
            .map(|element| MultilingualTextRef { element })
    }

    pub fn definitions(self) -> impl Iterator<Item = MultilingualTextRef<'a>> {
        self.element
            .children()
            .filter(|element| is_dt(element, "Definition"))
            .map(|element| MultilingualTextRef { element })
    }

    pub fn references(self, local_name: &'a str) -> impl Iterator<Item = ReferenceRef<'a>> {
        self.element
            .children()
            .filter(move |element| is_dt(element, local_name))
            .map(|element| ReferenceRef { element })
    }
}

/// Borrowed data-template view.
#[derive(Debug, Clone, Copy)]
pub struct DataTemplateRef<'a> {
    element: &'a Element,
}

impl<'a> DataTemplateRef<'a> {
    #[must_use]
    pub const fn element(self) -> &'a Element {
        self.element
    }

    #[must_use]
    pub const fn concept(self) -> ConceptRef<'a> {
        ConceptRef {
            element: self.element,
        }
    }

    pub fn property_references(self) -> impl Iterator<Item = ReferenceRef<'a>> {
        self.concept().references("HasPropertyRef")
    }

    pub fn group_references(self) -> impl Iterator<Item = ReferenceRef<'a>> {
        self.concept().references("HasGroupOfPropertiesRef")
    }

    pub fn object_type_reference(self) -> Option<ReferenceRef<'a>> {
        self.concept().references("HasObjectTypeRef").next()
    }
}

/// Borrowed property view.
#[derive(Debug, Clone, Copy)]
pub struct PropertyRef<'a> {
    element: &'a Element,
}

impl<'a> PropertyRef<'a> {
    #[must_use]
    pub const fn element(self) -> &'a Element {
        self.element
    }

    #[must_use]
    pub const fn concept(self) -> ConceptRef<'a> {
        ConceptRef {
            element: self.element,
        }
    }

    pub fn names(self) -> impl Iterator<Item = MultilingualTextRef<'a>> {
        self.concept().names()
    }

    #[must_use]
    pub fn data_type(self) -> Option<DataTypeRef<'a>> {
        self.element
            .children()
            .find(|element| is_dt(element, "DataType"))
            .map(|element| DataTypeRef { element })
    }
}

/// Borrowed data-type constraint view.
#[derive(Debug, Clone, Copy)]
pub struct DataTypeRef<'a> {
    element: &'a Element,
}

impl<'a> DataTypeRef<'a> {
    #[must_use]
    pub fn name(self) -> Option<&'a str> {
        self.element.attribute_ns(None, "name")
    }

    #[must_use]
    pub fn name_kind(self) -> Option<DataTypeName> {
        self.name().map(DataTypeName::from)
    }
}

/// An owned element proven to have one ISO 23387 global kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedElement<const KIND: u8> {
    element: Element,
}

impl<const KIND: u8> TypedElement<KIND> {
    pub fn try_from_element(element: Element) -> Result<Self, ModelError> {
        let expected = kind_from_discriminant(KIND);
        let actual = ElementKind::from_element(&element);
        if actual != Some(expected) {
            return Err(ModelError { expected, actual });
        }
        Ok(Self { element })
    }

    #[must_use]
    pub const fn element(&self) -> &Element {
        &self.element
    }

    #[must_use]
    pub fn into_element(self) -> Element {
        self.element
    }
}

impl<const KIND: u8> TryFrom<Element> for TypedElement<KIND> {
    type Error = ModelError;

    fn try_from(element: Element) -> Result<Self, Self::Error> {
        Self::try_from_element(element)
    }
}

pub type LibraryElement = TypedElement<0>;
pub type SubjectElement = TypedElement<1>;
pub type DataTemplateElement = TypedElement<2>;
pub type ObjectTypeElement = TypedElement<3>;
pub type GroupOfPropertiesElement = TypedElement<4>;
pub type PropertyElement = TypedElement<5>;
pub type UnitElement = TypedElement<6>;
pub type DimensionElement = TypedElement<7>;
pub type QuantityKindElement = TypedElement<8>;
pub type ReferenceDocumentElement = TypedElement<9>;

const fn kind_from_discriminant(value: u8) -> ElementKind {
    match value {
        0 => ElementKind::Library,
        1 => ElementKind::Subject,
        2 => ElementKind::DataTemplate,
        3 => ElementKind::ObjectType,
        4 => ElementKind::GroupOfProperties,
        5 => ElementKind::Property,
        6 => ElementKind::Unit,
        7 => ElementKind::Dimension,
        8 => ElementKind::QuantityKind,
        9 => ElementKind::ReferenceDocument,
        _ => panic!("invalid private DT element discriminant"),
    }
}

/// Typed-element conversion failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelError {
    pub expected: ElementKind,
    pub actual: Option<ElementKind>,
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "expected DT element {:?}, found {:?}",
            self.expected, self.actual
        )
    }
}

impl Error for ModelError {}

/// Validation severity; parsing itself remains non-normalizing and permissive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Stable validation categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    DraftNamespace,
    WrongRootNamespace,
    UnknownRootElement,
    MissingGuid,
    InvalidGuid,
    InvalidUri,
    DuplicateGuid,
    MissingCreationDate,
    InvalidCreationDate,
    MissingName,
    MissingDefinition,
    DuplicateDefinition,
    EmptyReference,
    MissingLanguage,
    InvalidLanguage,
    MissingDataTypeName,
    UnknownDataType,
}

/// One semantic validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub path: String,
    pub message: String,
}

impl Document {
    /// Runs structural checks that do not require redistributing the ISO schema.
    #[must_use]
    pub fn validate(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if self.root().namespace_uri() == Some(DRAFT_PLACEHOLDER_NAMESPACE) {
            diagnostics.push(diagnostic(
                Severity::Error,
                DiagnosticCode::DraftNamespace,
                "/",
                "root element uses the pre-release placeholder namespace, not ISO 23387 edition 2",
            ));
        } else if self.root().namespace_uri() != Some(NAMESPACE) {
            diagnostics.push(diagnostic(
                Severity::Error,
                DiagnosticCode::WrongRootNamespace,
                "/",
                "root element is not in the ISO 23387 edition 2 namespace",
            ));
        } else if self.root_kind().is_none() {
            diagnostics.push(diagnostic(
                Severity::Error,
                DiagnosticCode::UnknownRootElement,
                "/",
                "root element is not a recognized ISO 23387 global element",
            ));
        }

        let mut guids = HashMap::<String, String>::new();
        validate_element(self.root(), None, None, "", &mut guids, &mut diagnostics);
        diagnostics
    }
}

fn validate_element(
    element: &Element,
    parent_kind: Option<ElementKind>,
    parent_local: Option<&str>,
    parent_path: &str,
    guids: &mut HashMap<String, String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let path = format!("{parent_path}/{}", element.qname());
    let kind = ElementKind::from_element(element);
    let identity = match parent_kind {
        None => kind.is_some_and(ElementKind::is_global_root),
        Some(ElementKind::Library) => kind.is_some(),
        _ => false,
    };
    let requires_guid = identity;
    let concept = identity && kind != Some(ElementKind::Library);

    let guid = element.attribute_ns(Some(NAMESPACE), "GUID");
    if requires_guid && guid.is_none() {
        diagnostics.push(diagnostic(
            Severity::Error,
            DiagnosticCode::MissingGuid,
            &path,
            "required dt:GUID attribute is missing",
        ));
    }
    if let Some(value) = guid {
        if requires_guid {
            if let Some(first_path) = guids.insert(value.to_ascii_lowercase(), path.clone()) {
                diagnostics.push(diagnostic(
                    Severity::Error,
                    DiagnosticCode::DuplicateGuid,
                    &path,
                    format!("GUID duplicates {first_path}"),
                ));
            }
        }
        if Guid::from_str(value).is_err() {
            diagnostics.push(diagnostic(
                Severity::Error,
                DiagnosticCode::InvalidGuid,
                &path,
                "dt:GUID does not match the ISO 23387 lexical contract",
            ));
        }
    }
    for attribute_name in ["referenceURI", "about"] {
        if let Some(uri) = element.attribute_ns(Some(NAMESPACE), attribute_name) {
            if uri.parse::<AnyUri>().is_err() {
                diagnostics.push(diagnostic(
                    Severity::Error,
                    DiagnosticCode::InvalidUri,
                    &path,
                    format!("invalid dt:{attribute_name} value {uri:?}"),
                ));
            }
        }
    }
    if concept {
        match element.attribute_ns(None, "dateOfCreation") {
            None => diagnostics.push(diagnostic(
                Severity::Error,
                DiagnosticCode::MissingCreationDate,
                &path,
                "ConceptType-derived element lacks dateOfCreation",
            )),
            Some(value) if DateTime::from_str(value).is_err() => diagnostics.push(diagnostic(
                Severity::Error,
                DiagnosticCode::InvalidCreationDate,
                &path,
                "dateOfCreation does not match the xs:dateTime lexical contract",
            )),
            Some(_) => {}
        }
        if !element.children().any(|child| is_dt(child, "Name")) {
            diagnostics.push(diagnostic(
                Severity::Error,
                DiagnosticCode::MissingName,
                &path,
                "ConceptType-derived element lacks a Name",
            ));
        }
        match element
            .children()
            .filter(|child| is_dt(child, "Definition"))
            .count()
        {
            0 => diagnostics.push(diagnostic(
                Severity::Warning,
                DiagnosticCode::MissingDefinition,
                &path,
                "ConceptType-derived element lacks its required Definition",
            )),
            1 => {}
            _ => diagnostics.push(diagnostic(
                Severity::Warning,
                DiagnosticCode::DuplicateDefinition,
                &path,
                "ConceptType-derived element has more than one Definition",
            )),
        }
    }
    if is_reference_element(element, parent_local)
        && element.attribute_ns(Some(NAMESPACE), "GUID").is_none()
        && element
            .attribute_ns(Some(NAMESPACE), "referenceURI")
            .is_none()
    {
        diagnostics.push(diagnostic(
            Severity::Warning,
            DiagnosticCode::EmptyReference,
            &path,
            "reference has neither dt:GUID nor dt:referenceURI",
        ));
    }
    if is_multilingual_text_element(element, parent_local) {
        match element.attribute_ns(None, "language") {
            None => diagnostics.push(diagnostic(
                Severity::Error,
                DiagnosticCode::MissingLanguage,
                &path,
                "multi-language text lacks its required language attribute",
            )),
            Some(language) if MultiLanguageText::new(language, element.direct_text()).is_err() => {
                diagnostics.push(diagnostic(
                    Severity::Error,
                    DiagnosticCode::InvalidLanguage,
                    &path,
                    "language does not match the xs:language lexical contract",
                ));
            }
            Some(_) => {}
        }
    }
    if is_dt(element, "DataType") {
        match element.attribute_ns(None, "name") {
            None => diagnostics.push(diagnostic(
                Severity::Warning,
                DiagnosticCode::MissingDataTypeName,
                &path,
                "DataType has no name; Annex E permits it but the value is underspecified",
            )),
            Some(name) if matches!(DataTypeName::from(name), DataTypeName::Other(_)) => {
                diagnostics.push(diagnostic(
                    Severity::Warning,
                    DiagnosticCode::UnknownDataType,
                    &path,
                    format!("unknown data-type name {name:?} retained"),
                ));
            }
            Some(_) => {}
        }
    }

    for child in element.children() {
        validate_element(
            child,
            kind,
            Some(element.local_name()),
            &path,
            guids,
            diagnostics,
        );
    }
}

fn is_reference_element(element: &Element, parent_local: Option<&str>) -> bool {
    element.namespace_uri() == Some(NAMESPACE)
        && is_known_reference_name(element.local_name())
        && reference_allowed(parent_local, element.local_name())
}

fn reference_allowed(parent: Option<&str>, child: &str) -> bool {
    let concept_parent = parent.is_some_and(is_concept_parent);
    (concept_parent
        && matches!(
            child,
            "ReferenceDocumentRef" | "DictionaryRef" | "SimilarToRef" | "ReplacedObjectsRef"
        ))
        || (parent.is_some_and(is_subject_parent)
            && matches!(child, "HasPartRef" | "IsSubtypeOfRef"))
        || matches!(
            (parent, child),
            (
                Some("DataTemplate"),
                "HasObjectTypeRef" | "HasPropertyRef" | "HasGroupOfPropertiesRef"
            ) | (Some("GroupOfProperties"), "HasPropertyRef")
                | (
                    Some("Property"),
                    "UnitRef"
                        | "QuantityKindRef"
                        | "DimensionRef"
                        | "IsDependentOnRef"
                        | "IsSpecializationOfRef"
                )
                | (Some("QuantityKind"), "DimensionRef")
                | (Some("Unit"), "DimensionRef")
        )
}

fn is_known_reference_name(child: &str) -> bool {
    matches!(
        child,
        "ReferenceDocumentRef"
            | "DictionaryRef"
            | "SimilarToRef"
            | "ReplacedObjectsRef"
            | "HasPartRef"
            | "IsSubtypeOfRef"
            | "HasObjectTypeRef"
            | "HasPropertyRef"
            | "HasGroupOfPropertiesRef"
            | "IsSpecializationOfRef"
            | "IsDependentOnRef"
            | "UnitRef"
            | "QuantityKindRef"
            | "DimensionRef"
    )
}

fn is_multilingual_text_element(element: &Element, parent_local: Option<&str>) -> bool {
    element.namespace_uri() == Some(NAMESPACE)
        && is_known_multilingual_name(element.local_name())
        && multilingual_allowed(parent_local, element.local_name())
}

fn multilingual_allowed(parent: Option<&str>, child: &str) -> bool {
    (parent.is_some_and(is_concept_parent)
        && matches!(child, "Name" | "Definition" | "Description" | "Example"))
        || matches!(
            (parent, child),
            (Some("Unit"), "Symbol") | (Some("PossibleValues"), "ValueList")
        )
}

fn is_known_multilingual_name(child: &str) -> bool {
    matches!(
        child,
        "Name" | "Definition" | "Description" | "Example" | "Symbol" | "ValueList"
    )
}

fn is_concept_parent(parent: &str) -> bool {
    matches!(
        parent,
        "DataTemplate"
            | "ObjectType"
            | "GroupOfProperties"
            | "Property"
            | "ReferenceDocument"
            | "QuantityKind"
            | "Dimension"
            | "Unit"
    )
}

fn is_subject_parent(parent: &str) -> bool {
    matches!(parent, "DataTemplate" | "ObjectType" | "GroupOfProperties")
}

fn diagnostic(
    severity: Severity,
    code: DiagnosticCode,
    path: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        severity,
        code,
        path: path.into(),
        message: message.into(),
    }
}

fn required_guid(element: &Element) -> Result<Guid, ValueError> {
    Guid::from_str(
        element
            .attribute_ns(Some(NAMESPACE), "GUID")
            .unwrap_or_default(),
    )
}

fn is_dt(element: &Element, local_name: &str) -> bool {
    element.namespace_uri() == Some(NAMESPACE) && element.local_name() == local_name
}
