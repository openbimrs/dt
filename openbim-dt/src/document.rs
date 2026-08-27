//! Lossless, namespace-aware XML tree used by ISO 23387 documents.
//!
//! Two serializers are provided and they answer different questions:
//!
//! * [`Document::to_xml_string`] rebuilds the document from the *semantic*
//!   tree. Equivalent syntax is normalized (quote style, character-reference
//!   spelling, whitespace inside tags), so the output is well-formed and
//!   information-preserving but not necessarily byte-for-byte identical.
//! * [`Document::to_xml_string_exact`] replays the retained source spans and
//!   reproduces the parsed bytes exactly. It fails closed rather than silently
//!   normalizing when span provenance is unavailable.

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

/// Half-open byte range into the document's retained source text.
///
/// Positions are `u64` to match `quick-xml`'s reader positions directly; the
/// parser never has to narrow, so a document larger than `usize` on a 32-bit
/// host fails the byte budget rather than silently truncating a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) start: u64,
    pub(crate) end: u64,
}

impl Span {
    pub(crate) const fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    /// Smallest span covering both inputs. Used when adjacent text events are
    /// coalesced into a single [`Node::Text`]; those events are contiguous, so
    /// the union is exactly the original byte run.
    pub(crate) const fn union(self, other: Self) -> Self {
        Self {
            start: if self.start < other.start {
                self.start
            } else {
                other.start
            },
            end: if self.end > other.end {
                self.end
            } else {
                other.end
            },
        }
    }

    fn slice(self, source: &str) -> Option<&str> {
        let start = usize::try_from(self.start).ok()?;
        let end = usize::try_from(self.end).ok()?;
        source.get(start..end)
    }
}

/// An ordered list of child nodes plus the source span each node came from.
///
/// The two vectors are always the same length; every mutation goes through
/// [`NodeList::push`], which is the single place that invariant is maintained.
///
/// Equality deliberately ignores spans: they are provenance, not content. Two
/// lists holding the same nodes are equal even if one came from a document
/// that spelled the same content differently.
#[derive(Debug, Clone, Default, Eq)]
pub(crate) struct NodeList {
    nodes: Vec<Node>,
    spans: Vec<Option<Span>>,
}

impl PartialEq for NodeList {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes
    }
}

impl NodeList {
    pub(crate) const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            spans: Vec::new(),
        }
    }

    pub(crate) fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    fn spans(&self) -> &[Option<Span>] {
        &self.spans
    }

    fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Appends a node, coalescing runs of adjacent text into one node.
    ///
    /// Coalescing keeps `direct_text()` faithful for inputs such as
    /// `A&#38;B`, which the reader reports as three separate events. The
    /// merged span is the union of the merged events' spans, which is exact
    /// because those events are contiguous in the source.
    pub(crate) fn push(&mut self, node: Node, span: Option<Span>) {
        if let Node::Text(value) = node {
            if let Some(Node::Text(existing)) = self.nodes.last_mut() {
                existing.push_str(&value);
                let last = self.spans.len() - 1;
                self.spans[last] = match (self.spans[last], span) {
                    (Some(previous), Some(next)) => Some(previous.union(next)),
                    _ => None,
                };
                return;
            }
            self.nodes.push(Node::Text(value));
        } else {
            self.nodes.push(node);
        }
        self.spans.push(span);
    }
}

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
///
/// Equality compares semantic identity and content only. Source spans are
/// provenance for the exact writer, so two elements parsed from differently
/// spelled but equivalent syntax remain equal.
#[derive(Debug, Clone, Eq)]
pub struct Element {
    qname: String,
    prefix: Option<String>,
    local_name: String,
    namespace_uri: Option<Arc<str>>,
    attributes: Vec<Attribute>,
    nodes: NodeList,
    empty_style: bool,
    /// Source bytes of `<name …>` or, for empty-element syntax, `<name …/>`.
    open_span: Option<Span>,
    /// Source bytes of `</name>`; `None` for empty-element syntax.
    close_span: Option<Span>,
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        // `empty_style` is excluded on purpose: `<a/>` and `<a></a>` are the
        // same element. The exact writer distinguishes them via spans; the
        // semantic model must not.
        self.qname == other.qname
            && self.prefix == other.prefix
            && self.local_name == other.local_name
            && self.namespace_uri == other.namespace_uri
            && self.attributes == other.attributes
            && self.nodes == other.nodes
    }
}

impl Element {
    pub(crate) fn parsed(
        qname: String,
        prefix: Option<String>,
        local_name: String,
        namespace_uri: Option<Arc<str>>,
        attributes: Vec<Attribute>,
        empty_style: bool,
        open_span: Option<Span>,
    ) -> Self {
        Self {
            qname,
            prefix,
            local_name,
            namespace_uri,
            attributes,
            nodes: NodeList::new(),
            empty_style,
            open_span,
            close_span: None,
        }
    }

    pub(crate) fn set_close_span(&mut self, span: Option<Span>) {
        self.close_span = span;
    }

    /// The element's full source extent: `<a …>` through `</a>`, or the single
    /// `<a …/>` tag. `None` when either end was not recorded, which forces the
    /// exact writer to fail closed rather than emit a partial slice.
    pub(crate) fn full_span(&self) -> Option<Span> {
        let open = self.open_span?;
        match self.close_span {
            Some(close) => Some(open.union(close)),
            // Empty-element syntax has no close tag; the open span is complete.
            None if self.empty_style => Some(open),
            None => None,
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
        self.nodes.nodes()
    }

    /// Whether the source used an empty-element tag such as `<dt:Property/>`.
    #[must_use]
    pub const fn was_empty_element(&self) -> bool {
        self.empty_style
    }

    /// Direct child elements in document order.
    pub fn children(&self) -> impl Iterator<Item = &Element> {
        self.nodes.nodes().iter().filter_map(Node::as_element)
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
        for node in self.nodes.nodes() {
            match node {
                Node::Text(value) | Node::CData(value) => result.push_str(value),
                _ => {}
            }
        }
        result
    }

    pub(crate) fn push(&mut self, node: Node, span: Option<Span>) {
        self.nodes.push(node, span);
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
///
/// Equality is semantic: two documents are equal when their declaration,
/// prolog, root, and epilog match. Retained source text and spans are
/// provenance and do not participate, so a document compares equal to itself
/// after [`Document::without_source`].
#[derive(Debug, Clone, Eq)]
pub struct Document {
    declaration: Option<XmlDeclaration>,
    prolog: NodeList,
    root: Element,
    epilog: NodeList,
    /// Retained parse input, enabling byte-exact reserialization.
    source: Option<Arc<str>>,
    declaration_span: Option<Span>,
}

impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.declaration == other.declaration
            && self.prolog == other.prolog
            && self.root == other.root
            && self.epilog == other.epilog
    }
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
        prolog: NodeList,
        root: Element,
        epilog: NodeList,
        source: Option<Arc<str>>,
        declaration_span: Option<Span>,
    ) -> Self {
        Self {
            declaration,
            prolog,
            root,
            epilog,
            source,
            declaration_span,
        }
    }

    #[must_use]
    pub const fn declaration(&self) -> Option<&XmlDeclaration> {
        self.declaration.as_ref()
    }

    #[must_use]
    pub fn prolog(&self) -> &[Node] {
        self.prolog.nodes()
    }

    #[must_use]
    pub const fn root(&self) -> &Element {
        &self.root
    }

    #[must_use]
    pub fn epilog(&self) -> &[Node] {
        self.epilog.nodes()
    }

    /// The exact text this document was parsed from, when it is still retained.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Drops the retained source text, keeping the semantic tree.
    ///
    /// Useful when a caller wants to hold many documents in memory and does not
    /// need byte-exact output. Afterwards [`Document::to_xml_string_exact`]
    /// fails closed instead of reconstructing approximate bytes.
    #[must_use]
    pub fn without_source(mut self) -> Self {
        self.source = None;
        self
    }

    /// Serializes the semantic XML tree without dropping retained content.
    ///
    /// Equivalent syntax may be normalized; use [`Document::to_xml_string_exact`]
    /// when the output must match the parsed bytes.
    pub fn to_xml_string(&self) -> Result<String, WriteError> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        if let Some(declaration) = &self.declaration {
            writer.write_event(Event::Decl(BytesDecl::new(
                &declaration.version,
                declaration.encoding.as_deref(),
                declaration.standalone.as_deref(),
            )))?;
        }
        for node in self.prolog.nodes() {
            write_node(&mut writer, node)?;
        }
        write_element(&mut writer, &self.root)?;
        for node in self.epilog.nodes() {
            write_node(&mut writer, node)?;
        }
        String::from_utf8(writer.into_inner().into_inner()).map_err(WriteError::Utf8)
    }

    /// Reproduces the parsed document byte-for-byte from retained source spans.
    ///
    /// Every construct is emitted from the exact bytes the reader consumed, so
    /// attribute quoting, character-reference spelling, intra-tag whitespace,
    /// CDATA versus escaped text, and empty-element syntax all survive
    /// unchanged. The call fails rather than falling back to normalization when
    /// span provenance is missing.
    pub fn to_xml_string_exact(&self) -> Result<String, ExactWriteError> {
        let source = self.source.as_deref().ok_or(ExactWriteError::NoSource)?;
        let mut output = String::with_capacity(source.len());
        if self.declaration.is_some() {
            push_span(&mut output, source, self.declaration_span)?;
        }
        push_nodes(&mut output, source, &self.prolog)?;
        push_element(&mut output, source, &self.root)?;
        push_nodes(&mut output, source, &self.epilog)?;
        Ok(output)
    }

    pub(crate) fn take_root(self) -> Element {
        self.root
    }
}

fn push_span(output: &mut String, source: &str, span: Option<Span>) -> Result<(), ExactWriteError> {
    let span = span.ok_or(ExactWriteError::MissingSpan)?;
    output.push_str(span.slice(source).ok_or(ExactWriteError::SpanOutOfRange)?);
    Ok(())
}

fn push_nodes(output: &mut String, source: &str, list: &NodeList) -> Result<(), ExactWriteError> {
    for (node, span) in list.nodes().iter().zip(list.spans()) {
        match node {
            Node::Element(element) => push_element(output, source, element)?,
            _ => push_span(output, source, *span)?,
        }
    }
    Ok(())
}

fn push_element(
    output: &mut String,
    source: &str,
    element: &Element,
) -> Result<(), ExactWriteError> {
    push_span(output, source, element.open_span)?;
    if element.close_span.is_none() {
        // Empty-element syntax: the open span already covers `<name …/>`.
        if element.nodes.is_empty() {
            return Ok(());
        }
        return Err(ExactWriteError::MissingSpan);
    }
    push_nodes(output, source, &element.nodes)?;
    push_span(output, source, element.close_span)
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
        // Comments carry literal character data, not entity references; writing
        // them escaped would corrupt any comment containing `&`, `<`, or `>`.
        Node::Comment(value) => writer
            .write_event(Event::Comment(BytesText::from_escaped(value.as_str())))
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

/// Why a byte-exact reserialization could not be produced.
///
/// Every variant means the same thing to a caller: the exact bytes are not
/// recoverable, so no output is returned rather than a normalized guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactWriteError {
    /// The document does not retain its parse input.
    NoSource,
    /// A retained construct has no recorded source span.
    MissingSpan,
    /// A recorded span does not address a character boundary of the source.
    SpanOutOfRange,
}

impl fmt::Display for ExactWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NoSource => "document does not retain its parse input",
            Self::MissingSpan => "a retained node has no recorded source span",
            Self::SpanOutOfRange => "a recorded source span is not a character boundary",
        };
        formatter.write_str(message)
    }
}

impl Error for ExactWriteError {}
