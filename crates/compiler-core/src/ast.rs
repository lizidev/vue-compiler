use crate::{
    OpenBlock,
    codegen::CodegenNode,
    runtime_helpers::{CreateBlock, CreateElementBlock, CreateElementVNode, CreateVNode},
    transform::TransformContext,
    utils::{find_dir, find_prop},
};
use vue_compiler_shared::PatchFlags;

pub use crate::transforms::transform_element::PropsExpression;

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
    // containers
    CompoundExpression,
    If,
    IfBranch,
    For,
    // codegen
    VNodeCall,
    JSCallExpression,
    JSObjectExpression,
    JSProperty,
    JSArrayExpression,
    JSCacheExpression,

    // ssr codegen
    JSTemplateLiteral,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ElementTypes {
    Element,
    Component,
    Slot,
    Template,
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

#[derive(Debug)]
pub enum ParentNode<'a> {
    Root(&'a mut RootNode),
    Element(&'a mut ElementNode),
}

impl<'a> ParentNode<'a> {
    pub fn children_mut(&mut self) -> &mut Vec<TemplateChildNode> {
        match self {
            Self::Root(node) => node.children.as_mut(),
            Self::Element(node) => node.children_mut(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExpressionNode {
    Simple(SimpleExpressionNode),
    Compound(CompoundExpressionNode),
}

impl ExpressionNode {
    pub fn new_simple(
        content: impl Into<String>,
        is_static: Option<bool>,
        loc: Option<SourceLocation>,
        const_type: Option<ConstantTypes>,
    ) -> Self {
        Self::Simple(SimpleExpressionNode::new(
            content, is_static, loc, const_type,
        ))
    }

    pub fn new_compound(
        children: Vec<CompoundExpressionNodeChild>,
        loc: Option<SourceLocation>,
    ) -> Self {
        Self::Compound(CompoundExpressionNode::new(children, loc))
    }

    #[inline]
    pub fn type_(&self) -> NodeTypes {
        match self {
            Self::Simple(node) => node.type_(),
            Self::Compound(node) => node.type_(),
        }
    }

    pub fn loc(&self) -> &SourceLocation {
        match self {
            Self::Simple(node) => &node.loc,
            Self::Compound(node) => &node.loc,
        }
    }

    pub fn is_handler_key(&self) -> Option<bool> {
        match self {
            Self::Simple(node) => node.is_handler_key,
            Self::Compound(node) => node.is_handler_key,
        }
    }

    pub fn is_static_exp(&self) -> bool {
        if let Self::Simple(node) = self
            && node.is_static
        {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TemplateChildNode {
    Element(ElementNode),
    Interpolation(InterpolationNode),
    Compound(CompoundExpressionNode),
    Text(TextNode),
    Comment(CommentNode),
    If(IfNode),
    IfBranch(IfBranchNode),
    For(ForNode),
}

impl TemplateChildNode {
    pub fn new_interpolation(content: ExpressionNode, loc: SourceLocation) -> Self {
        Self::Interpolation(InterpolationNode::new(content, loc))
    }

    pub fn new_compound(
        children: Vec<CompoundExpressionNodeChild>,
        loc: Option<SourceLocation>,
    ) -> Self {
        Self::Compound(CompoundExpressionNode::new(children, loc))
    }

    pub fn new_text(content: impl Into<String>, loc: SourceLocation) -> Self {
        Self::Text(TextNode::new(content, loc))
    }

    pub fn new_comment(content: impl Into<String>, loc: SourceLocation) -> Self {
        Self::Comment(CommentNode::new(content, loc))
    }

    pub fn type_(&self) -> NodeTypes {
        match self {
            Self::Element(node) => node.type_().clone(),
            Self::Interpolation(node) => node.type_(),
            Self::Compound(node) => node.type_(),
            Self::Text(node) => node.type_(),
            Self::Comment(node) => node.type_(),
            Self::If(_) => NodeTypes::If,
            Self::IfBranch(_) => NodeTypes::IfBranch,
            Self::For(node) => node.type_(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RootCodegenNode {
    TemplateChild(TemplateChildNode),
    JSChild(JSChildNode),
}

#[derive(Debug)]
pub struct RootNode {
    pub source: String,
    pub children: Vec<TemplateChildNode>,
    pub helpers: ::indexmap::IndexSet<String>,
    pub components: Vec<String>,
    pub directives: Vec<String>,
    pub hoists: Vec<Option<JSChildNode>>,
    pub cached: Vec<Option<CacheExpression>>,
    pub temps: usize,
    pub codegen_node: Option<RootCodegenNode>,
    pub transformed: Option<bool>,
    pub loc: SourceLocation,
}

impl RootNode {
    pub fn new(children: Vec<TemplateChildNode>, source: Option<String>) -> Self {
        Self {
            source: source.unwrap_or_default(),
            children,
            helpers: Default::default(),
            components: Vec::new(),
            directives: Vec::new(),
            hoists: Vec::new(),
            cached: Vec::new(),
            temps: 0,
            codegen_node: None,
            transformed: None,
            loc: SourceLocation::loc_stub(),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::Root
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ElementNode {
    PlainElement(PlainElementNode),
    Component(ComponentNode),
    SlotOutlet(SlotOutletNode),
    Template(TemplateNode),
}

impl ElementNode {
    #[inline]
    pub fn type_(&self) -> NodeTypes {
        NodeTypes::Element
    }

    pub fn loc(&self) -> &SourceLocation {
        match self {
            Self::PlainElement(el) => &el.loc,
            Self::Component(node) => &node.loc,
            Self::SlotOutlet(node) => &node.loc,
            Self::Template(el) => &el.loc,
        }
    }

    pub fn loc_mut(&mut self) -> &mut SourceLocation {
        match self {
            Self::PlainElement(el) => &mut el.loc,
            Self::Component(node) => &mut node.loc,
            Self::SlotOutlet(node) => &mut node.loc,
            Self::Template(el) => &mut el.loc,
        }
    }

    pub fn ns(&self) -> &Namespace {
        match self {
            Self::PlainElement(el) => &el.ns,
            Self::Component(node) => &node.ns,
            Self::SlotOutlet(node) => &node.ns,
            Self::Template(el) => &el.ns,
        }
    }

    pub fn tag(&self) -> &String {
        match self {
            Self::PlainElement(el) => &el.tag,
            Self::Component(node) => &node.tag,
            Self::SlotOutlet(node) => &node.tag,
            Self::Template(el) => &el.tag,
        }
    }

    pub fn tag_type(&self) -> ElementTypes {
        match self {
            Self::PlainElement(node) => node.tag_type(),
            Self::Component(node) => node.tag_type(),
            Self::SlotOutlet(node) => node.tag_type(),
            Self::Template(node) => node.tag_type(),
        }
    }

    pub fn props(&self) -> &Vec<BaseElementProps> {
        match self {
            Self::PlainElement(el) => &el.props,
            Self::Component(node) => &node.props,
            Self::SlotOutlet(node) => &node.props,
            Self::Template(el) => &el.props,
        }
    }

    pub fn props_mut(&mut self) -> &mut Vec<BaseElementProps> {
        match self {
            Self::PlainElement(el) => &mut el.props,
            Self::Component(node) => &mut node.props,
            Self::SlotOutlet(node) => &mut node.props,
            Self::Template(el) => &mut el.props,
        }
    }

    pub fn children(&self) -> &Vec<TemplateChildNode> {
        match self {
            Self::PlainElement(el) => &el.children,
            Self::Component(node) => &node.children,
            Self::SlotOutlet(node) => &node.children,
            Self::Template(el) => &el.children,
        }
    }

    pub fn children_mut(&mut self) -> &mut Vec<TemplateChildNode> {
        match self {
            Self::PlainElement(el) => &mut el.children,
            Self::Component(node) => &mut node.children,
            Self::SlotOutlet(node) => &mut node.children,
            Self::Template(el) => &mut el.children,
        }
    }

    pub fn is_self_closing_mut(&mut self) -> &mut Option<bool> {
        match self {
            Self::PlainElement(el) => &mut el.is_self_closing,
            Self::Component(node) => &mut node.is_self_closing,
            Self::SlotOutlet(node) => &mut node.is_self_closing,
            Self::Template(el) => &mut el.is_self_closing,
        }
    }

    pub fn to_component(&self) -> Self {
        match &self {
            Self::PlainElement(node) => Self::Component(ComponentNode {
                ns: node.ns.clone(),
                tag: node.tag.clone(),
                props: node.props.clone(),
                children: node.children.clone(),
                is_self_closing: node.is_self_closing.clone(),
                codegen_node: None,
                ssr_codegen_node: None,
                loc: node.loc.clone(),
            }),
            Self::Component(node) => Self::Component(node.clone()),
            Self::SlotOutlet(node) => Self::Component(ComponentNode {
                ns: node.ns.clone(),
                tag: node.tag.clone(),
                props: node.props.clone(),
                children: node.children.clone(),
                is_self_closing: node.is_self_closing.clone(),
                codegen_node: None,
                ssr_codegen_node: None,
                loc: node.loc.clone(),
            }),
            Self::Template(node) => Self::Component(ComponentNode {
                ns: node.ns.clone(),
                tag: node.tag.clone(),
                props: node.props.clone(),
                children: node.children.clone(),
                is_self_closing: node.is_self_closing.clone(),
                codegen_node: None,
                ssr_codegen_node: None,
                loc: node.loc.clone(),
            }),
        }
    }

    pub fn to_slot_outlet(&self) -> Self {
        match &self {
            Self::PlainElement(node) => Self::SlotOutlet(SlotOutletNode {
                ns: node.ns.clone(),
                tag: node.tag.clone(),
                props: node.props.clone(),
                children: node.children.clone(),
                is_self_closing: node.is_self_closing.clone(),
                codegen_node: None,
                ssr_codegen_node: None,
                loc: node.loc.clone(),
            }),
            Self::Component(node) => Self::SlotOutlet(SlotOutletNode {
                ns: node.ns.clone(),
                tag: node.tag.clone(),
                props: node.props.clone(),
                children: node.children.clone(),
                is_self_closing: node.is_self_closing.clone(),
                codegen_node: None,
                ssr_codegen_node: None,
                loc: node.loc.clone(),
            }),
            Self::SlotOutlet(node) => Self::SlotOutlet(node.clone()),
            Self::Template(node) => Self::SlotOutlet(SlotOutletNode {
                ns: node.ns.clone(),
                tag: node.tag.clone(),
                props: node.props.clone(),
                children: node.children.clone(),
                is_self_closing: node.is_self_closing.clone(),
                codegen_node: None,
                ssr_codegen_node: None,
                loc: node.loc.clone(),
            }),
        }
    }

    pub fn to_template(&self) -> Self {
        match &self {
            Self::PlainElement(node) => Self::Template(TemplateNode {
                ns: node.ns.clone(),
                tag: node.tag.clone(),
                props: node.props.clone(),
                children: node.children.clone(),
                is_self_closing: node.is_self_closing.clone(),
                codegen_node: None,
                ssr_codegen_node: None,
                loc: node.loc.clone(),
            }),
            Self::Component(node) => Self::Template(TemplateNode {
                ns: node.ns.clone(),
                tag: node.tag.clone(),
                props: node.props.clone(),
                children: node.children.clone(),
                is_self_closing: node.is_self_closing.clone(),
                codegen_node: None,
                ssr_codegen_node: None,
                loc: node.loc.clone(),
            }),
            Self::SlotOutlet(node) => Self::Template(TemplateNode {
                ns: node.ns.clone(),
                tag: node.tag.clone(),
                props: node.props.clone(),
                children: node.children.clone(),
                is_self_closing: node.is_self_closing.clone(),
                codegen_node: None,
                ssr_codegen_node: None,
                loc: node.loc.clone(),
            }),
            Self::Template(node) => Self::Template(node.clone()),
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
pub struct BaseElementNode<C, S> {
    pub ns: Namespace,
    pub tag: String,
    pub props: Vec<BaseElementProps>,
    pub children: Vec<TemplateChildNode>,
    pub is_self_closing: Option<bool>,
    pub codegen_node: Option<C>,
    pub ssr_codegen_node: Option<S>,
    pub loc: SourceLocation,
}

impl<C, S> PartialEq for BaseElementNode<C, S>
where
    C: PartialEq,
    S: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.ns == other.ns
            && self.tag == other.tag
            && self.props == other.props
            && self.children == other.children
            && self.is_self_closing == other.is_self_closing
            && self.codegen_node == other.codegen_node
            && self.ssr_codegen_node == other.ssr_codegen_node
            && self.loc == other.loc
    }
}

impl<C, S> Clone for BaseElementNode<C, S>
where
    C: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            ns: self.ns.clone(),
            tag: self.tag.clone(),
            props: self.props.clone(),
            children: self.children.clone(),
            is_self_closing: self.is_self_closing.clone(),
            codegen_node: self.codegen_node.clone(),
            ssr_codegen_node: self.ssr_codegen_node.clone(),
            loc: self.loc.clone(),
        }
    }
}

impl<C, S> BaseElementNode<C, S> {
    #[inline]
    pub fn type_(&self) -> NodeTypes {
        NodeTypes::Element
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum PlainElementNodeCodegenNode {
    VNodeCall(VNodeCall),
}

pub type PlainElementNode = BaseElementNode<PlainElementNodeCodegenNode, ()>;

impl PlainElementNode {
    #[inline]
    pub fn tag_type(&self) -> ElementTypes {
        ElementTypes::Element
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ComponentNodeCodegenNode {
    VNodeCall(VNodeCall),
}

pub type ComponentNode = BaseElementNode<ComponentNodeCodegenNode, ()>;

impl ComponentNode {
    #[inline]
    pub fn tag_type(&self) -> ElementTypes {
        ElementTypes::Component
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum SlotOutletNodeCodegenNode {}

pub type SlotOutletNode = BaseElementNode<SlotOutletNodeCodegenNode, ()>;

impl SlotOutletNode {
    #[inline]
    pub fn tag_type(&self) -> ElementTypes {
        ElementTypes::Slot
    }
}

// TemplateNode is a container type that always gets compiled away
pub type TemplateNode = BaseElementNode<(), ()>;

impl TemplateNode {
    #[inline]
    pub fn tag_type(&self) -> ElementTypes {
        ElementTypes::Template
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TextNode {
    pub content: String,
    pub loc: SourceLocation,
}

impl TextNode {
    pub fn new(content: impl Into<String>, loc: SourceLocation) -> Self {
        Self {
            content: content.into(),
            loc,
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::Text
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CommentNode {
    pub content: String,
    pub loc: SourceLocation,
}

impl CommentNode {
    pub fn new(content: impl Into<String>, loc: SourceLocation) -> Self {
        Self {
            content: content.into(),
            loc,
        }
    }

    #[inline]
    pub fn type_(&self) -> NodeTypes {
        NodeTypes::Comment
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct AttributeNode {
    pub name: String,
    pub name_loc: SourceLocation,
    pub value: Option<TextNode>,
    pub loc: SourceLocation,
}

impl AttributeNode {
    pub fn type_(&self) -> NodeTypes {
        NodeTypes::Attribute
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct DirectiveNode {
    /// the normalized name without prefix or shorthands, e.g. "bind", "on"
    pub name: String,
    /// the raw attribute name, preserving shorthand, and including arg & modifiers
    /// this is only used during parse.
    pub raw_name: Option<String>,
    pub exp: Option<ExpressionNode>,
    pub arg: Option<ExpressionNode>,
    pub modifiers: Vec<SimpleExpressionNode>,
    /// optional property to cache the expression parse result for v-for
    pub for_parse_result: Option<ForParseResult>,
    pub loc: SourceLocation,
}

impl DirectiveNode {
    pub fn type_(&self) -> NodeTypes {
        NodeTypes::Directive
    }
}

/// Static types have several levels.
/// Higher levels implies lower levels. e.g. a node that can be stringified
/// can always be hoisted and skipped for patch.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ConstantTypes {
    NotConstant,
    CanSkipPatch,
    CanCache,
    CanStringify,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SimpleExpressionNode {
    pub content: String,
    pub is_static: bool,
    pub const_type: ConstantTypes,

    /// an expression parsed as the params of a function will track
    /// the identifiers declared inside the function body.
    identifiers: Option<Vec<String>>,
    is_handler_key: Option<bool>,
    pub loc: SourceLocation,
}

impl SimpleExpressionNode {
    pub fn new(
        content: impl Into<String>,
        is_static: Option<bool>,
        loc: Option<SourceLocation>,
        mut const_type: Option<ConstantTypes>,
    ) -> Self {
        if is_static == Some(true) {
            const_type = Some(ConstantTypes::CanStringify);
        }
        Self {
            content: content.into(),
            is_static: is_static.unwrap_or_default(),
            const_type: const_type.unwrap_or(ConstantTypes::NotConstant),
            identifiers: None,
            is_handler_key: None,
            loc: loc.unwrap_or_else(|| SourceLocation::loc_stub()),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::SimpleExpression
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct InterpolationNode {
    pub content: ExpressionNode,
    pub loc: SourceLocation,
}

impl InterpolationNode {
    pub fn new(content: ExpressionNode, loc: SourceLocation) -> Self {
        Self { content, loc }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::Interpolation
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CompoundExpressionNodeChild {
    Simple(SimpleExpressionNode),
    Compound(CompoundExpressionNode),
    Interpolation(InterpolationNode),
    Text(TextNode),
    String(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct CompoundExpressionNode {
    pub children: Vec<CompoundExpressionNodeChild>,

    is_handler_key: Option<bool>,
    pub loc: SourceLocation,
}

impl CompoundExpressionNode {
    pub fn new(children: Vec<CompoundExpressionNodeChild>, loc: Option<SourceLocation>) -> Self {
        Self {
            children,
            is_handler_key: None,
            loc: loc.unwrap_or_else(SourceLocation::loc_stub),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::CompoundExpression
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum IfCodegenNode {
    IfConditional(IfConditionalExpression),
    // CacheExpression
}

impl IfCodegenNode {
    pub fn get_parent_condition(&mut self) -> &mut IfConditionalExpression {
        match self {
            Self::IfConditional(node) => node.get_parent_condition(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfNode {
    pub branches: Vec<IfBranchNode>,
    pub codegen_node: Option<IfCodegenNode>,
    pub loc: SourceLocation,
}

impl IfNode {
    pub fn type_(&self) -> NodeTypes {
        NodeTypes::If
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfBranchNode {
    // else
    pub condition: Option<ExpressionNode>,
    pub children: Vec<TemplateChildNode>,
    user_key: Option<BaseElementProps>,
    is_template_if: Option<bool>,
    pub loc: SourceLocation,
}

impl IfBranchNode {
    pub fn new(node: &ElementNode, dir: DirectiveNode) -> Self {
        let is_template_if = node.tag_type() == ElementTypes::Template;
        Self {
            condition: if dir.name == "else" {
                None
            } else {
                dir.exp.clone()
            },
            children: if is_template_if && !find_dir(node, "for", None).is_some() {
                node.children().clone()
            } else {
                vec![TemplateChildNode::Element(node.clone())]
            },
            user_key: find_prop(node, "key", None, None),
            is_template_if: Some(is_template_if),
            loc: node.loc().clone(),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::IfBranch
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ForParseResult {
    pub source: ExpressionNode,
    pub value: Option<ExpressionNode>,
    pub key: Option<ExpressionNode>,
    pub index: Option<ExpressionNode>,
    pub finalized: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ForNode {
    pub source: ExpressionNode,
    pub value_alias: Option<ExpressionNode>,
    pub key_alias: Option<ExpressionNode>,
    pub object_index_alias: Option<ExpressionNode>,
    pub parse_result: ForParseResult,
    pub children: Vec<TemplateChildNode>,
    pub codegen_node: Option<ForCodegenNode>,
    pub loc: SourceLocation,
}

impl ForNode {
    pub fn type_(&self) -> NodeTypes {
        NodeTypes::For
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TemplateTextChildNode {
    Text(TextNode),
    Interpolation(InterpolationNode),
    Compound(CompoundExpressionNode),
}

impl From<TemplateChildNode> for TemplateTextChildNode {
    fn from(value: TemplateChildNode) -> Self {
        match value {
            TemplateChildNode::Text(node) => Self::Text(node),
            TemplateChildNode::Interpolation(node) => Self::Interpolation(node),
            TemplateChildNode::Compound(node) => Self::Compound(node),
            _ => {
                unreachable!();
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum VNodeCallChildren {
    //. multiple children
    TemplateChildNodeList(Vec<TemplateChildNode>),
    /// single text child
    TemplateTextChildNode(TemplateTextChildNode),
    /// v-for fragment call
    ForRenderListExpression(ForRenderListExpression),
}

#[derive(Debug, PartialEq, Clone)]
pub struct VNodeCall {
    pub tag: String,
    pub props: Option<PropsExpression>,
    pub children: Option<VNodeCallChildren>,
    pub patch_flag: Option<PatchFlags>,
    pub is_block: bool,
    pub disable_tracking: bool,
    pub is_component: bool,
    pub loc: SourceLocation,
}

impl VNodeCall {
    pub fn new(
        context: Option<&mut TransformContext>,
        tag: impl Into<String>,
        props: Option<PropsExpression>,
        children: Option<VNodeCallChildren>,
        patch_flag: Option<PatchFlags>,
        is_block: Option<bool>,
        disable_tracking: Option<bool>,
        is_component: Option<bool>,
        loc: Option<SourceLocation>,
    ) -> Self {
        let is_block = is_block.unwrap_or_default();
        let is_component = is_component.unwrap_or_default();

        if let Some(context) = context {
            if is_block {
                context.helper(OpenBlock.to_string());
                context.helper(get_vnode_block_helper(context.in_ssr, is_component));
            } else {
                context.helper(get_vnode_helper(context.in_ssr, is_component));
            }
        }

        Self {
            tag: tag.into(),
            props,
            children,
            patch_flag,
            is_block,
            disable_tracking: disable_tracking.unwrap_or_default(),
            is_component,
            loc: loc.unwrap_or_else(|| SourceLocation::loc_stub()),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::VNodeCall
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ForCodegenNode {
    pub tag: String,
    pub children: ForRenderListExpression,
    pub patch_flag: PatchFlags,
    pub disable_tracking: bool,
    pub is_component: bool,
    pub loc: SourceLocation,
}

impl Into<VNodeCall> for ForCodegenNode {
    fn into(self) -> VNodeCall {
        VNodeCall {
            tag: self.tag,
            props: None,
            children: None,
            patch_flag: Some(self.patch_flag),
            is_block: true,
            disable_tracking: self.disable_tracking,
            is_component: false,
            loc: self.loc,
        }
    }
}

impl ForCodegenNode {
    #[inline]
    pub fn type_(&self) -> NodeTypes {
        NodeTypes::VNodeCall
    }

    #[inline]
    pub fn is_block(&self) -> bool {
        true
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ForRenderListArgument {
    Expression(ExpressionNode),
    ForIterator(ForIteratorExpression),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ForRenderListExpression {
    pub callee: CallCallee,
    pub arguments: Vec<ForRenderListArgument>,
    pub loc: SourceLocation,
}

impl ForRenderListExpression {
    pub fn new(
        callee: impl Into<CallCallee>,
        arguments: Option<Vec<ForRenderListArgument>>,
        loc: Option<SourceLocation>,
    ) -> Self {
        Self {
            callee: callee.into(),
            arguments: arguments.unwrap_or_default(),
            loc: loc.unwrap_or_else(|| SourceLocation::loc_stub()),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::JSCallExpression
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FunctionParams {
    Expression(ExpressionNode),
    String(String),
    ExpressionList(Vec<ExpressionNode>),
    StringList(Vec<String>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ForIteratorExpression {
    pub params: Option<FunctionParams>,
    pub returns: Option<BlockCodegenNode>,
    pub newline: bool,
}

// JS Node Types ---------------------------------------------------------------

// We also include a number of JavaScript AST nodes for code generation.
// The AST is an intentionally minimal subset just to meet the exact needs of
// Vue render function generation.

#[derive(Debug, PartialEq, Clone)]
pub enum JSChildNode {
    VNodeCall(VNodeCall),
    Call(CallExpression),
    Object(ObjectExpression),
    Array(ArrayExpression),
    Simple(SimpleExpressionNode),
    Compound(CompoundExpressionNode),
    IfConditional(Box<IfConditionalExpression>),
    Cache(Box<CacheExpression>),
}

impl JSChildNode {
    pub fn is_static_exp(&self) -> bool {
        if let Self::Simple(node) = self
            && node.is_static
        {
            true
        } else {
            false
        }
    }
}

impl From<ExpressionNode> for JSChildNode {
    fn from(value: ExpressionNode) -> Self {
        match value {
            ExpressionNode::Simple(node) => Self::Simple(node),
            ExpressionNode::Compound(node) => Self::Compound(node),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CallArgument {
    String(String),
    JSChild(JSChildNode),
    SSRCodegen(SSRCodegenNode),
    TemplateChild(TemplateChildNode),
    TemplateChildren(Vec<TemplateChildNode>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum CallCallee {
    String(String),
    Symbol(String),
}

impl From<&str> for CallCallee {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for CallCallee {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CallExpression {
    pub callee: CallCallee,
    pub arguments: Vec<CallArgument>,
    pub loc: SourceLocation,
}

impl CallExpression {
    pub fn new(
        callee: impl Into<CallCallee>,
        arguments: Option<Vec<CallArgument>>,
        loc: Option<SourceLocation>,
    ) -> Self {
        Self {
            callee: callee.into(),
            arguments: arguments.unwrap_or_default(),
            loc: loc.unwrap_or_else(|| SourceLocation::loc_stub()),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::JSCallExpression
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ObjectExpression {
    pub properties: Vec<Property>,
    pub loc: SourceLocation,
}

impl ObjectExpression {
    pub fn new(properties: Vec<Property>, loc: Option<SourceLocation>) -> Self {
        Self {
            properties,
            loc: loc.unwrap_or_else(|| SourceLocation::loc_stub()),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::JSObjectExpression
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Property {
    pub key: ExpressionNode,
    pub value: JSChildNode,
    pub loc: SourceLocation,
}

impl Property {
    pub fn new(key: ExpressionNode, value: JSChildNode) -> Self {
        Self {
            key,
            value,
            loc: SourceLocation::loc_stub(),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::JSProperty
    }
}

pub type ArrayExpressionElement = CodegenNode;

#[derive(Debug, PartialEq, Clone)]
pub struct ArrayExpression {
    pub elements: Vec<ArrayExpressionElement>,
    pub loc: SourceLocation,
}

impl ArrayExpression {
    pub fn new(elements: Vec<ArrayExpressionElement>, loc: Option<SourceLocation>) -> Self {
        Self {
            elements,
            loc: loc.unwrap_or_else(|| SourceLocation::loc_stub()),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::JSArrayExpression
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CacheExpression {
    pub index: usize,
    pub value: JSChildNode,
    pub need_pause_tracking: bool,
    pub in_v_once: bool,
    pub need_array_spread: bool,
    pub loc: SourceLocation,
}

impl CacheExpression {
    pub fn new(
        index: usize,
        value: JSChildNode,
        need_pause_tracking: Option<bool>,
        in_v_once: Option<bool>,
    ) -> Self {
        Self {
            index,
            value,
            need_pause_tracking: need_pause_tracking.unwrap_or_default(),
            in_v_once: in_v_once.unwrap_or_default(),
            need_array_spread: false,
            loc: SourceLocation::loc_stub(),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::JSCacheExpression
    }
}

// SSR-specific Node Types -----------------------------------------------------

#[derive(Debug, PartialEq, Clone)]
pub enum SSRCodegenNode {
    TemplateLiteral(TemplateLiteral),
}

#[derive(Debug, PartialEq, Clone)]
pub enum TemplateLiteralElement {
    String(String),
    JSChild(JSChildNode),
}

#[derive(Debug, PartialEq, Clone)]
pub struct TemplateLiteral {
    pub elements: Vec<TemplateLiteralElement>,
    pub loc: SourceLocation,
}

impl TemplateLiteral {
    pub fn new(elements: Vec<TemplateLiteralElement>) -> Self {
        Self {
            elements,
            loc: SourceLocation::loc_stub(),
        }
    }

    pub fn type_(&self) -> NodeTypes {
        NodeTypes::JSTemplateLiteral
    }
}

// Codegen Node Types ----------------------------------------------------------
#[derive(Debug, PartialEq, Clone)]
pub enum BlockCodegenNode {
    VNodeCall(VNodeCall),
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfConditionalExpression {
    pub test: JSChildNode,
    pub consequent: JSChildNode,
    pub alternate: JSChildNode,
    pub newline: bool,
}

impl IfConditionalExpression {
    fn get_parent_condition(&mut self) -> &mut Self {
        if !matches!(self.alternate, JSChildNode::IfConditional(_)) {
            return self;
        }
        if let JSChildNode::IfConditional(alternate) = &mut self.alternate {
            return alternate.get_parent_condition();
        }
        unreachable!()
    }
}

pub fn get_vnode_helper(ssr: bool, is_component: bool) -> String {
    if ssr || is_component {
        CreateVNode.to_string()
    } else {
        CreateElementVNode.to_string()
    }
}

pub fn get_vnode_block_helper(ssr: bool, is_component: bool) -> String {
    if ssr || is_component {
        CreateBlock.to_string()
    } else {
        CreateElementBlock.to_string()
    }
}

pub fn convert_to_block(node: &mut VNodeCall, context: &mut TransformContext) {
    if !node.is_block {
        node.is_block = true;
        context.remove_helper(&get_vnode_helper(context.in_ssr, node.is_component));
        context.helper(OpenBlock.to_string());
        context.helper(get_vnode_block_helper(context.in_ssr, node.is_component));
    }
}
