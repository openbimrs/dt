//! Lossless, namespace-aware XML tree used by ISO 23387 documents.

use std::{borrow::Cow, error::Error, fmt, io::Cursor, sync::Arc};

use quick_xml::{
    events::{
        attributes::Attribute as XmlAttribute, BytesCData, BytesDecl, BytesEnd, BytesPI,
        BytesStart, BytesText, Event,
    },
    name::QName,
    Writer,
};

use crate::parser::{parse_document, ParseError, ParseOptions};

/// XML declaration values retained by the document model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlDeclaration {
    pub version: String,
    pub encoding: Option<String>,
    pub standalone: Option<String>,
}

/// One XML attribute with both lexical and resolved namespace identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    qname: String,
    prefix: Option<String>,
    local_name: String,
    namespace_uri: Option<Arc<str>>,
    value: String,
}

impl Attribute {
    pub(crate) fn parsed(
        qname: String,
        prefix: Option<String>,
        local_name: String,
        namespace_uri: Option<Arc<str>>,
        value: String,
    ) -> Self {
        Self {
            qname,
            prefix,
            local_name,
            namespace_uri,
            value,
        }
    }

    #[must_use]
    pub fn qname(&self) -> &str {
        &self.qname
    }

    #[must_use]
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    #[must_use]
    pub fn local_name(&self) -> &str {
        &self.local_name
    }

    #[must_use]
    pub fn namespace_uri(&self) -> Option<&str> {
        self.namespace_uri.as_deref()
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// An XML element retaining names, namespace resolution, attributes, and order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    qname: String,
    prefix: Option<String>,
    local_name: String,
    namespace_uri: Option<Arc<str>>,
    attributes: Vec<Attribute>,
    nodes: Vec<Node>,
    empty_style: bool,
}

impl Element {
    pub(crate) fn parsed(
        qname: String,
        prefix: Option<String>,
        local_name: String,
        namespace_uri: Option<Arc<str>>,
        attributes: Vec<Attribute>,
        empty_style: bool,
    ) -> Self {
        Self {
            qname,
            prefix,
            local_name,
            namespace_uri,
            attributes,
            nodes: Vec::new(),
            empty_style,
        }
    }

    #[must_use]
    pub fn qname(&self) -> &str {
        &self.qname
    }

    #[must_use]
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    #[must_use]
    pub fn local_name(&self) -> &str {
        &self.local_name
    }

    #[must_use]
    pub fn namespace_uri(&self) -> Option<&str> {
        self.namespace_uri.as_deref()
    }

    #[must_use]
    pub fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }

    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Whether the source used an empty-element tag such as `<dt:Property/>`.
    #[must_use]
    pub const fn was_empty_element(&self) -> bool {
        self.empty_style
    }

    /// Direct child elements in document order.
    pub fn children(&self) -> impl Iterator<Item = &Element> {
        self.nodes.iter().filter_map(Node::as_element)
    }

    /// Finds an attribute by resolved namespace URI and local name.
    #[must_use]
    pub fn attribute_ns(&self, namespace_uri: Option<&str>, local_name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| {
                attribute.namespace_uri() == namespace_uri && attribute.local_name() == local_name
            })
            .map(Attribute::value)
    }

    /// Direct semantic text and CDATA content, preserving order.
    #[must_use]
    pub fn direct_text(&self) -> String {
        let mut result = String::new();
        for node in &self.nodes {
            match node {
                Node::Text(value) | Node::CData(value) => result.push_str(value),
                _ => {}
            }
        }
        result
    }

    pub(crate) fn push(&mut self, node: Node) {
        push_coalescing_text(&mut self.nodes, node);
    }
}

pub(crate) fn push_coalescing_text(nodes: &mut Vec<Node>, node: Node) {
    if let Node::Text(value) = node {
        if let Some(Node::Text(existing)) = nodes.last_mut() {
            existing.push_str(&value);
        } else {
            nodes.push(Node::Text(value));
        }
    } else {
        nodes.push(node);
    }
}

/// XML node kinds retained by the document model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
    CData(String),
    Comment(String),
    /// Complete processing-instruction content: target plus optional data.
    ProcessingInstruction(String),
}

impl Node {
    #[must_use]
    pub fn as_element(&self) -> Option<&Element> {
        match self {
            Self::Element(element) => Some(element),
            _ => None,
        }
    }
}

/// A well-formed XML document with one root element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    declaration: Option<XmlDeclaration>,
    prolog: Vec<Node>,
    root: Element,
    epilog: Vec<Node>,
}

impl Document {
    /// Parses with conservative production defaults.
    pub fn parse(xml: &str) -> Result<Self, ParseError> {
        Self::parse_with_options(xml, ParseOptions::default())
    }

    /// Parses with explicit resource limits.
    pub fn parse_with_options(xml: &str, options: ParseOptions) -> Result<Self, ParseError> {
        parse_document(xml, options)
    }

    pub(crate) fn parsed(
        declaration: Option<XmlDeclaration>,
        prolog: Vec<Node>,
        root: Element,
        epilog: Vec<Node>,
    ) -> Self {
        Self {
            declaration,
            prolog,
            root,
            epilog,
        }
    }

    #[must_use]
    pub const fn declaration(&self) -> Option<&XmlDeclaration> {
        self.declaration.as_ref()
    }

    #[must_use]
    pub fn prolog(&self) -> &[Node] {
        &self.prolog
    }

    #[must_use]
    pub const fn root(&self) -> &Element {
        &self.root
    }

    #[must_use]
    pub fn epilog(&self) -> &[Node] {
        &self.epilog
    }

    /// Serializes the semantic XML tree without dropping retained content.
    pub fn to_xml_string(&self) -> Result<String, WriteError> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        if let Some(declaration) = &self.declaration {
            writer.write_event(Event::Decl(BytesDecl::new(
                &declaration.version,
                declaration.encoding.as_deref(),
                declaration.standalone.as_deref(),
            )))?;
        }
        for node in &self.prolog {
            write_node(&mut writer, node)?;
        }
        write_element(&mut writer, &self.root)?;
        for node in &self.epilog {
            write_node(&mut writer, node)?;
        }
        String::from_utf8(writer.into_inner().into_inner()).map_err(WriteError::Utf8)
    }

    pub(crate) fn take_root(self) -> Element {
        self.root
    }
}

fn write_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    element: &Element,
) -> Result<(), WriteError> {
    let mut start = BytesStart::new(element.qname());
    for attribute in element.attributes() {
        start.push_attribute(XmlAttribute {
            key: QName(attribute.qname().as_bytes()),
            value: Cow::Owned(escape_attribute_value(attribute.value())),
        });
    }
    if element.empty_style && element.nodes.is_empty() {
        writer.write_event(Event::Empty(start))?;
        return Ok(());
    }
    writer.write_event(Event::Start(start))?;
    for node in element.nodes() {
        write_node(writer, node)?;
    }
    writer.write_event(Event::End(BytesEnd::new(element.qname())))?;
    Ok(())
}

fn escape_attribute_value(value: &str) -> Vec<u8> {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '"' => output.push_str("&quot;"),
            '\t' => output.push_str("&#x9;"),
            '\n' => output.push_str("&#xA;"),
            '\r' => output.push_str("&#xD;"),
            _ => output.push(character),
        }
    }
    output.into_bytes()
}

fn escape_text_value(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '\r' => output.push_str("&#13;"),
            _ => output.push(character),
        }
    }
    output
}

fn write_node(writer: &mut Writer<Cursor<Vec<u8>>>, node: &Node) -> Result<(), WriteError> {
    match node {
        Node::Element(element) => write_element(writer, element),
        Node::Text(value) => writer
            .write_event(Event::Text(BytesText::from_escaped(escape_text_value(
                value,
            ))))
            .map_err(Into::into),
        Node::CData(value) => writer
            .write_event(Event::CData(BytesCData::new(value)))
            .map_err(Into::into),
        Node::Comment(value) => writer
            .write_event(Event::Comment(BytesText::new(value)))
            .map_err(Into::into),
        Node::ProcessingInstruction(value) => writer
            .write_event(Event::PI(BytesPI::new(value)))
            .map_err(Into::into),
    }
}

/// XML serialization failure.
#[derive(Debug)]
pub enum WriteError {
    Xml(std::io::Error),
    Utf8(std::string::FromUtf8Error),
}

impl fmt::Display for WriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Xml(error) => write!(formatter, "could not write XML: {error}"),
            Self::Utf8(error) => write!(formatter, "XML writer produced invalid UTF-8: {error}"),
        }
    }
}

impl Error for WriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Xml(error) => Some(error),
            Self::Utf8(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for WriteError {
    fn from(value: std::io::Error) -> Self {
        Self::Xml(value)
    }
}
