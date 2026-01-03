use std::ops::{Deref, DerefMut};

/// Vue template is a platform-agnostic superset of HTML (syntax only).
/// More namespaces can be declared by platform specific compilers.
pub type Namespace = u32;

#[derive(Debug, PartialEq, Clone)]
pub enum Namespaces {
    HTML,
    SVG,
    MathML,
}

#[derive(Debug, PartialEq, Clone)]
pub enum NodeTypes {
    Root,
    Element,
    Text,
    Comment,
    SimpleExpression,
    Interpolation,
    Attribute,
    Directive,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ElementTypes {
    Element,
    Component,
    Slot,
    Template,
}

#[derive(Debug)]
pub struct Node<I> {
    pub type_: NodeTypes,
    pub loc: SourceLocation,

    pub inner: I,
}

impl<I> Deref for Node<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<I> DerefMut for Node<I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<I> PartialEq for Node<I>
where
    I: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.type_ == other.type_ && self.loc == other.loc && self.inner == other.inner
    }
}

impl<I> Clone for Node<I>
where
    I: Clone,
{
    fn clone(&self) -> Self {
        Self {
            type_: self.type_.clone(),
            loc: self.loc.clone(),
            inner: self.inner.clone(),
        }
    }
}

/// The node's range. The `start` is inclusive and `end` is exclusive.
/// [start, end)
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub start: Position,
    pub end: Position,
    pub source: String,
}

impl SourceLocation {
    /// Some expressions, e.g. sequence and conditional expressions, are never
    /// associated with template nodes, so their source locations are just a stub.
    /// Container types like CompoundExpression also don't need a real location.
    pub fn loc_stub() -> Self {
        Self {
            start: Position {
                offset: 0,
                line: 1,
                column: 1,
            },
            end: Position {
                offset: 0,
                line: 1,
                column: 1,
            },
            source: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// from start of file
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExpressionNode {
    Simple(SimpleExpressionNode),
    Compound,
}

impl ExpressionNode {
    pub fn new_simple(
        content: String,
        is_static: Option<bool>,
        loc: Option<SourceLocation>,
        const_type: Option<ConstantTypes>,
    ) -> Self {
        Self::Simple(SimpleExpressionNode::new(
            content, is_static, loc, const_type,
        ))
    }
}

#[derive(Debug, PartialEq)]
pub enum TemplateChildNode {
    Element(ElementNode),
    Interpolation(InterpolationNode),
    Text(TextNode),
    Comment(CommentNode),
}

impl TemplateChildNode {
    pub fn new_interpolation(content: ExpressionNode, loc: SourceLocation) -> Self {
        Self::Interpolation(InterpolationNode::new(content, loc))
    }

    pub fn new_text(content: impl Into<String>, loc: SourceLocation) -> Self {
        Self::Text(TextNode::new(content, loc))
    }

    pub fn new_comment(content: impl Into<String>, loc: SourceLocation) -> Self {
        Self::Comment(CommentNode::new(content, loc))
    }
}

#[derive(Debug)]
pub struct Root {
    pub source: String,
    pub children: Vec<TemplateChildNode>,
}

pub type RootNode = Node<Root>;

impl RootNode {
    pub fn new(children: Vec<TemplateChildNode>, source: Option<String>) -> Self {
        Node {
            type_: NodeTypes::Root,
            loc: SourceLocation::loc_stub(),
            inner: Root {
                source: source.unwrap_or_default(),
                children,
            },
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ElementNode {
    PlainElement(PlainElementNode),
    Template(TemplateNode),
}

impl ElementNode {
    pub fn loc(&self) -> &SourceLocation {
        match self {
            Self::PlainElement(el) => &el.loc,
            Self::Template(el) => &el.loc,
        }
    }

    pub fn loc_mut(&mut self) -> &mut SourceLocation {
        match self {
            Self::PlainElement(el) => &mut el.loc,
            Self::Template(el) => &mut el.loc,
        }
    }

    pub fn ns(&self) -> &Namespace {
        match self {
            Self::PlainElement(el) => &el.ns,
            Self::Template(el) => &el.ns,
        }
    }

    pub fn tag(&self) -> &String {
        match self {
            Self::PlainElement(el) => &el.tag,
            Self::Template(el) => &el.tag,
        }
    }

    pub fn tag_type_mut(&mut self) -> &mut ElementTypes {
        match self {
            Self::PlainElement(el) => &mut el.tag_type,
            Self::Template(el) => &mut el.tag_type,
        }
    }

    pub fn props(&self) -> &Vec<BaseElementProps> {
        match self {
            Self::PlainElement(el) => &el.props,
            Self::Template(el) => &el.props,
        }
    }

    pub fn props_mut(&mut self) -> &mut Vec<BaseElementProps> {
        match self {
            Self::PlainElement(el) => &mut el.props,
            Self::Template(el) => &mut el.props,
        }
    }

    pub fn children(&self) -> &Vec<TemplateChildNode> {
        match self {
            Self::PlainElement(el) => &el.children,
            Self::Template(el) => &el.children,
        }
    }

    pub fn children_mut(&mut self) -> &mut Vec<TemplateChildNode> {
        match self {
            Self::PlainElement(el) => &mut el.children,
            Self::Template(el) => &mut el.children,
        }
    }

    pub fn is_self_closing_mut(&mut self) -> &mut Option<bool> {
        match self {
            Self::PlainElement(el) => &mut el.is_self_closing,
            Self::Template(el) => &mut el.is_self_closing,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum BaseElementProps {
    Attribute(AttributeNode),
    Directive(DirectiveNode),
}

impl BaseElementProps {
    pub fn name(&self) -> &String {
        match self {
            Self::Attribute(el) => &el.name,
            Self::Directive(el) => &el.name,
        }
    }

    pub fn loc(&self) -> &SourceLocation {
        match self {
            Self::Attribute(el) => &el.loc,
            Self::Directive(el) => &el.loc,
        }
    }

    pub fn loc_mut(&mut self) -> &mut SourceLocation {
        match self {
            Self::Attribute(el) => &mut el.loc,
            Self::Directive(el) => &mut el.loc,
        }
    }
}

#[derive(Debug)]
pub struct BaseElement<C, S> {
    pub ns: Namespace,
    pub tag: String,
    pub tag_type: ElementTypes,
    pub props: Vec<BaseElementProps>,
    pub children: Vec<TemplateChildNode>,
    pub is_self_closing: Option<bool>,
    pub codegen_node: Option<C>,
    pub ssr_codegen_node: Option<S>,
}

impl<C, S> PartialEq for BaseElement<C, S>
where
    C: PartialEq,
    S: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
            && self.tag_type == other.tag_type
            && self.children == other.children
            && self.is_self_closing == other.is_self_closing
            && self.codegen_node == other.codegen_node
            && self.ssr_codegen_node == other.ssr_codegen_node
    }
}

pub type BaseElementNode<C, S> = Node<BaseElement<C, S>>;

pub type PlainElementNode = BaseElementNode<(), ()>;

// TemplateNode is a container type that always gets compiled away
pub type TemplateNode = BaseElementNode<(), ()>;

#[derive(Debug, PartialEq, Clone)]
pub struct Text {
    pub content: String,
}

pub type TextNode = Node<Text>;

impl TextNode {
    pub fn new(content: impl Into<String>, loc: SourceLocation) -> Self {
        Node {
            type_: NodeTypes::Text,
            loc,

            inner: Text {
                content: content.into(),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Comment {
    pub content: String,
}

pub type CommentNode = Node<Comment>;

impl CommentNode {
    pub fn new(content: impl Into<String>, loc: SourceLocation) -> Self {
        Node {
            type_: NodeTypes::Comment,
            loc,

            inner: Comment {
                content: content.into(),
            },
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Attribute {
    pub name: String,
    pub value: Option<TextNode>,
}

pub type AttributeNode = Node<Attribute>;

#[derive(Debug, PartialEq, Clone)]
pub struct Directive {
    /// the normalized name without prefix or shorthands, e.g. "bind", "on"
    pub name: String,
    /// the raw attribute name, preserving shorthand, and including arg & modifiers
    /// this is only used during parse.
    pub raw_name: Option<String>,
    pub exp: Option<ExpressionNode>,
    pub arg: Option<ExpressionNode>,
}

pub type DirectiveNode = Node<Directive>;

/// Static types have several levels.
/// Higher levels implies lower levels. e.g. a node that can be stringified
/// can always be hoisted and skipped for patch.
#[derive(Debug, PartialEq, Clone)]
pub enum ConstantTypes {
    NotConstant,
    CanSkipPatch,
    CanCache,
    CanStringify,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SimpleExpression {
    content: String,
    is_static: bool,
    const_type: ConstantTypes,

    /// an expression parsed as the params of a function will track
    /// the identifiers declared inside the function body.
    identifiers: Option<Vec<String>>,
    is_handler_key: Option<bool>,
}

impl SimpleExpression {
    pub fn new(
        content: String,
        is_static: Option<bool>,
        const_type: Option<ConstantTypes>,
    ) -> Self {
        Self {
            content,
            is_static: is_static.unwrap_or_default(),
            const_type: const_type.unwrap_or(ConstantTypes::NotConstant),
            identifiers: None,
            is_handler_key: None,
        }
    }
}

pub type SimpleExpressionNode = Node<SimpleExpression>;

impl SimpleExpressionNode {
    pub fn new(
        content: String,
        is_static: Option<bool>,
        loc: Option<SourceLocation>,
        const_type: Option<ConstantTypes>,
    ) -> Self {
        Self {
            type_: NodeTypes::SimpleExpression,
            loc: loc.unwrap_or_else(|| SourceLocation::loc_stub()),
            inner: SimpleExpression::new(content, is_static, const_type),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Interpolation {
    pub content: ExpressionNode,
}

pub type InterpolationNode = Node<Interpolation>;

impl InterpolationNode {
    pub fn new(content: ExpressionNode, loc: SourceLocation) -> Self {
        Self {
            type_: NodeTypes::Interpolation,
            loc,
            inner: Interpolation { content },
        }
    }
}
