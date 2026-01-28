use vue_compiler_shared::PatchFlags;

use crate::{
    SlotOutletNode,
    ast::{
        ArrayExpression, CacheExpression, CallArgument, CallCallee, CallExpression, CommentNode,
        ComponentNode, ComponentNodeCodegenNode, CompoundExpressionNode,
        CompoundExpressionNodeChild, ElementNode, ExpressionNode, ForNode, IfBranchNode,
        IfCodegenNode, IfConditionalExpression, IfNode, InterpolationNode, JSChildNode,
        ObjectExpression, PlainElementNode, PlainElementNodeCodegenNode, Property, PropsExpression,
        RootCodegenNode, RootNode, SSRCodegenNode, SimpleExpressionNode, TemplateChildNode,
        TemplateLiteral, TemplateLiteralElement, TemplateTextChildNode, TextNode, VNodeCall,
        VNodeCallChildren, get_vnode_helper,
    },
    get_vnode_block_helper,
    options::{CodegenMode, CodegenOptions},
    runtime_helpers::{
        CreateComment, CreateElementVNode, CreateStatic, CreateText, CreateVNode, OpenBlock,
        ResolveComponent, ResolveDirective, SetBlockTracking, ToDisplayString,
    },
    utils::{GlobalCompileTimeConstants, is_simple_identifier, to_valid_asset_id},
};

/// The `SourceMapGenerator` type from `source-map-js` is a bit incomplete as it
/// misses `toJSON()`. We also need to add types for internal properties which we
/// need to access for better performance.
///
/// Since TS 5.3, dts generation starts to strangely include broken triple slash
/// references for source-map-js, so we are inlining all source map related types
/// here to to workaround that.
pub trait CodegenSourceMapGenerator: std::fmt::Debug {
    // setSourceContent(sourceFile: string, sourceContent: string): void
    // // SourceMapGenerator has this method but the types do not include it
    // toJSON(): RawSourceMap
    // _sources: Set<string>
    // _names: Set<string>
    // _mappings: {
    //   add(mapping: MappingItem): void
    // }
}

const PURE_ANNOTATION: &'static str = "/*@__PURE__*/";

fn alias_helper(s: String) -> String {
    format!("{s}: _{s}")
}

#[derive(Debug, PartialEq, Clone)]
pub enum CodegenNode {
    // TemplateChildNode
    Element(ElementNode),
    Interpolation(InterpolationNode),
    Compound(CompoundExpressionNode),
    Text(TextNode),
    Comment(CommentNode),
    If(IfNode),
    IfBranch(IfBranchNode),
    For(ForNode),
    // JSChildNode
    VNodeCall(VNodeCall),
    Call(CallExpression),
    Object(ObjectExpression),
    Array(ArrayExpression),
    Simple(SimpleExpressionNode),
    IfConditional(IfConditionalExpression),
    Cache(CacheExpression),
    // SSRCodegenNode,
    TemplateLiteral(TemplateLiteral),
}

impl From<TemplateChildNode> for CodegenNode {
    fn from(node: TemplateChildNode) -> Self {
        match node {
            TemplateChildNode::Element(node) => Self::Element(node),
            TemplateChildNode::Interpolation(node) => Self::Interpolation(node),
            TemplateChildNode::Compound(node) => Self::Compound(node),
            TemplateChildNode::Text(node) => Self::Text(node),
            TemplateChildNode::Comment(node) => Self::Comment(node),
            TemplateChildNode::If(node) => Self::If(node),
            TemplateChildNode::IfBranch(node) => Self::IfBranch(node),
            TemplateChildNode::For(node) => Self::For(node),
        }
    }
}

impl From<JSChildNode> for CodegenNode {
    fn from(node: JSChildNode) -> Self {
        match node {
            JSChildNode::VNodeCall(node) => Self::VNodeCall(node),
            JSChildNode::Call(node) => Self::Call(node),
            JSChildNode::Object(node) => Self::Object(node),
            JSChildNode::Array(node) => Self::Array(node),
            JSChildNode::Simple(node) => Self::Simple(node),
            JSChildNode::Compound(node) => Self::Compound(node),
            JSChildNode::IfConditional(node) => Self::IfConditional(*node),
            JSChildNode::Cache(node) => Self::Cache(*node),
        }
    }
}

impl From<SSRCodegenNode> for CodegenNode {
    fn from(node: SSRCodegenNode) -> Self {
        match node {
            SSRCodegenNode::TemplateLiteral(node) => Self::TemplateLiteral(node),
        }
    }
}

impl From<RootCodegenNode> for CodegenNode {
    fn from(node: RootCodegenNode) -> Self {
        match node {
            RootCodegenNode::TemplateChild(node) => Self::from(node),
            RootCodegenNode::JSChild(node) => Self::from(node),
        }
    }
}

impl From<PlainElementNodeCodegenNode> for CodegenNode {
    fn from(node: PlainElementNodeCodegenNode) -> Self {
        match node {
            PlainElementNodeCodegenNode::VNodeCall(node) => Self::VNodeCall(node),
        }
    }
}

impl From<ComponentNodeCodegenNode> for CodegenNode {
    fn from(node: ComponentNodeCodegenNode) -> Self {
        match node {
            ComponentNodeCodegenNode::VNodeCall(node) => Self::VNodeCall(node),
        }
    }
}

impl From<ExpressionNode> for CodegenNode {
    fn from(node: ExpressionNode) -> Self {
        match node {
            ExpressionNode::Simple(node) => Self::Simple(node),
            ExpressionNode::Compound(node) => Self::Compound(node),
        }
    }
}

impl From<IfCodegenNode> for CodegenNode {
    fn from(node: IfCodegenNode) -> Self {
        match node {
            IfCodegenNode::IfConditional(node) => Self::IfConditional(node),
        }
    }
}

#[derive(Debug)]
pub struct CodegenResult {
    pub code: String,
    pub preamble: String,
    pub ast: RootNode,
}

enum NewlineType {
    Start = 0,
    End = -1,
    None = -2,
    Unknown = -3,
}

#[derive(Debug)]
struct CodegenContext {
    // SharedTransformCodegenOptions
    prefix_identifiers: bool,
    ssr: bool,
    in_ssr: bool,
    is_ts: bool,

    mode: CodegenMode,
    scope_id: Option<String>,
    optimize_imports: bool,
    runtime_module_name: String,
    runtime_global_name: String,

    code: String,
    indent_level: usize,
    pure: bool,
    map: Option<Box<dyn CodegenSourceMapGenerator>>,

    global_compile_time_constants: GlobalCompileTimeConstants,
}

impl CodegenContext {
    fn new(options: &CodegenOptions) -> Self {
        Self {
            prefix_identifiers: options
                .prefix_identifiers
                .unwrap_or(options.mode == Some(CodegenMode::Module)),
            ssr: options.ssr.unwrap_or_default(),
            in_ssr: options.in_ssr.unwrap_or_default(),
            is_ts: options.is_ts.unwrap_or_default(),

            mode: options.mode.clone().unwrap_or(CodegenMode::Function),
            scope_id: options.scope_id.clone(),
            optimize_imports: options.optimize_imports.unwrap_or_default(),
            runtime_module_name: options
                .runtime_module_name
                .clone()
                .unwrap_or_else(|| "vue".to_string()),
            runtime_global_name: options
                .runtime_global_name
                .clone()
                .unwrap_or_else(|| "Vue".to_string()),

            code: String::new(),
            indent_level: 0,
            pure: false,
            map: None,

            global_compile_time_constants: options.global_compile_time_constants,
        }
    }

    fn helper(&self, key: String) -> String {
        format!("_{}", key)
    }

    fn push(&mut self, code: &str, newline_index: Option<NewlineType>, node: Option<CodegenNode>) {
        let newline_index = newline_index.unwrap_or(NewlineType::None);

        self.code.push_str(code);
        if !self.global_compile_time_constants.__browser__ && self.map.is_some() {
            let _ = newline_index;
            let _ = node;
            todo!();
        }
    }

    fn indent(&mut self) {
        self.indent_level += 1;
        newline(self, self.indent_level);
    }

    fn deindent(&mut self, without_new_line: Option<bool>) {
        let without_new_line = without_new_line.unwrap_or_default();
        if without_new_line {
            self.indent_level -= 1;
        } else {
            self.indent_level -= 1;
            newline(self, self.indent_level);
        }
    }

    fn newline(&mut self) {
        newline(self, self.indent_level);
    }
}

fn newline(context: &mut CodegenContext, n: usize) {
    context.push(
        &format!("\n{}", "  ".repeat(n)),
        Some(NewlineType::Start),
        None,
    );
}

pub fn generate(ast: RootNode, options: CodegenOptions) -> CodegenResult {
    let mut context = CodegenContext::new(&options);
    let mode = context.mode.clone();
    let prefix_identifiers = context.prefix_identifiers;
    let ssr = context.ssr;
    let scope_id = context.scope_id.clone();

    let helpers = ast.helpers.clone();
    let has_helpers = !helpers.is_empty();
    let use_with_block = !prefix_identifiers && mode != CodegenMode::Module;
    let gen_scope_id = !options.global_compile_time_constants.__browser__
        && scope_id.is_some()
        && mode == CodegenMode::Module;
    let is_setup_inlined =
        !options.global_compile_time_constants.__browser__ && options.inline.unwrap_or_default();

    if !options.global_compile_time_constants.__browser__ && mode == CodegenMode::Module {
        gen_module_preamble(&ast, &mut context, gen_scope_id, Some(is_setup_inlined));
    } else {
        gen_function_preamble(&ast, &mut context);
    }
    // enter render function
    let function_name = if ssr { "ssrRender" } else { "render" };
    let mut args = if ssr {
        vec!["_ctx", "_push", "_parent", "_attrs"]
    } else {
        vec!["_ctx", "_cache"]
    };
    if !options.global_compile_time_constants.__browser__
        && options.binding_metadata.is_some()
        && !options.inline.unwrap_or_default()
    {
        // binding optimization args
        args.extend(vec!["$props", "$setup", "$data", "$options"]);
    }

    let signature = if !options.global_compile_time_constants.__browser__
        && options.is_ts.unwrap_or_default()
    {
        args.iter()
            .map(|arg| format!("{arg}: any"))
            .collect::<Vec<String>>()
            .join(",")
    } else {
        args.join(", ")
    };

    if is_setup_inlined {
        context.push(&format!("({signature}) => {{"), None, None);
    } else {
        context.push(
            &format!("function {function_name}({signature}) {{"),
            None,
            None,
        );
    }
    context.indent();

    if use_with_block {
        context.push("with (_ctx) {", None, None);
        context.indent();
        // function mode const declarations should be inside with block
        // also they should be renamed to avoid collision with user properties
        if has_helpers {
            let helpers = ast
                .helpers
                .iter()
                .cloned()
                .map(alias_helper)
                .collect::<Vec<String>>()
                .join(", ");
            context.push(
                &format!("const {{ {helpers} }} = _Vue\n"),
                Some(NewlineType::End),
                None,
            );
            context.newline();
        }
    }

    // generate asset resolution statements
    if ast.components.len() > 0 {
        gen_assets(&ast.components, AssetType::Component, &mut context);
        if ast.directives.len() > 0 || ast.temps > 0 {
            context.newline();
        }
    }
    if ast.directives.len() > 0 {
        gen_assets(&ast.directives, AssetType::Directive, &mut context);
        if ast.temps > 0 {
            context.newline();
        }
    }

    if ast.temps > 0 {
        context.push("let ", None, None);
        for i in 0..ast.temps {
            context.push(
                &format!("{}_temp{i}", if i > 0 { ", " } else { "" }),
                None,
                None,
            );
        }
    }
    if ast.components.len() > 0 || ast.directives.len() > 0 || ast.temps > 0 {
        context.push("\n", Some(NewlineType::Start), None);
        context.newline();
    }

    // generate the VNode tree expression
    if !ssr {
        context.push("return ", None, None);
    }
    if let Some(codegen_node) = ast.codegen_node.clone() {
        gen_node(CodegenNode::from(codegen_node), &mut context);
    } else {
        context.push("null", None, None);
    }

    if use_with_block {
        context.deindent(None);
        context.push("}", None, None);
    }

    context.deindent(None);
    context.push("}", None, None);

    CodegenResult {
        code: context.code,
        preamble: String::new(),
        ast,
    }
}

fn gen_function_preamble(ast: &RootNode, context: &mut CodegenContext) {
    let ssr = context.ssr;
    let prefix_identifiers = context.prefix_identifiers;
    let runtime_module_name = context.runtime_module_name.clone();
    let runtime_global_name = context.runtime_global_name.clone();
    let vue_binding = if !context.global_compile_time_constants.__browser__ && ssr {
        let runtime_module_name =
            ::serde_json::to_string(&runtime_module_name).unwrap_or(runtime_module_name);
        format!("require({runtime_module_name})")
    } else {
        runtime_global_name
    };

    // Generate const declaration for helpers
    // In prefix mode, we place the const declaration at top so it's done
    // only once; But if we not prefixing, we place the declaration inside the
    // with block so it doesn't incur the `in` check cost for every helper access.
    if ast.helpers.len() > 0 {
        if !context.global_compile_time_constants.__browser__ && prefix_identifiers {
        } else {
            // "with" mode.
            // save Vue in a separate variable to avoid collision
            context.push(
                &format!("const _Vue = {vue_binding}\n"),
                Some(NewlineType::End),
                None,
            );
            // in "with" mode, helpers are declared inside the with block to avoid
            // has check cost, but hoists are lifted out of the function - we need
            // to provide the helper here.
            if ast.hoists.len() != 0 {
                let static_helpers = vec![
                    CreateVNode.to_string(),
                    CreateElementVNode.to_string(),
                    CreateComment.to_string(),
                    CreateText.to_string(),
                    CreateStatic.to_string(),
                ]
                .into_iter()
                .filter(|helper| ast.helpers.contains(helper))
                .map(alias_helper)
                .collect::<Vec<String>>()
                .join(", ");
                context.push(
                    &format!("const {{ {} }} = _Vue\n", static_helpers),
                    Some(NewlineType::End),
                    None,
                );
            }
        }
    }
    gen_hoists(&ast.hoists, context);
    context.newline();
    context.push("return ", None, None);
}

fn gen_module_preamble(
    ast: &RootNode,
    context: &mut CodegenContext,
    _gen_scope_id: bool,
    inline: Option<bool>,
) {
    let runtime_module_name = context.runtime_module_name.clone();

    if ast.helpers.len() != 0 {
        if context.optimize_imports {
            // when bundled with webpack with code-split, calling an import binding
            // as a function leads to it being wrapped with `Object(a.b)` or `(0,a.b)`,
            // incurring both payload size increase and potential perf overhead.
            // therefore we assign the imports to variables (which is a constant ~50b
            // cost per-component instead of scaling with template size)
            let helpers = ast
                .helpers
                .iter()
                .cloned()
                .collect::<Vec<String>>()
                .join(", ");
            let runtime_module_name =
                ::serde_json::to_string(&runtime_module_name).unwrap_or(runtime_module_name);
            let code = format!("import {{ {helpers} }} from {runtime_module_name}\n");
            context.push(&code, Some(NewlineType::End), None);

            let helpers = ast
                .helpers
                .iter()
                .cloned()
                .map(|s| format!("_{s} = {s}"))
                .collect::<Vec<String>>()
                .join(", ");
            let code =
                format!("\n// Binding optimization for webpack code-split\nconst {helpers}\n");
            context.push(&code, Some(NewlineType::End), None);
        } else {
            let helpers = ast
                .helpers
                .iter()
                .cloned()
                .map(|s| format!("{s} as _{s}"))
                .collect::<Vec<String>>()
                .join(", ");
            let runtime_module_name =
                ::serde_json::to_string(&runtime_module_name).unwrap_or_default();
            let code = format!("import {{ {helpers} }} from {runtime_module_name}\n");
            context.push(&code, Some(NewlineType::End), None);
        }
    }

    gen_hoists(&ast.hoists, context);
    context.newline();

    if !inline.unwrap_or_default() {
        context.push("export ", None, None);
    }
}

#[derive(Debug, PartialEq)]
pub enum AssetType {
    Component,
    Directive,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Component => "component",
            Self::Directive => "directive",
        };
        write!(f, "{}", name)
    }
}

fn gen_assets(assets: &Vec<String>, type_: AssetType, context: &mut CodegenContext) {
    let is_ts = context.is_ts;
    let resolver = context.helper(if type_ == AssetType::Component {
        ResolveComponent.to_string()
    } else {
        ResolveDirective.to_string()
    });
    for (i, mut id) in assets.clone().into_iter().enumerate() {
        // potential component implicit self-reference inferred from SFC filename
        let maybe_self_reference = id.ends_with("__self");
        if maybe_self_reference {
            id = id.split_at(id.len() - 6).0.to_string();
        }
        context.push(
            &format!(
                "const {} = {resolver}({}{}){}",
                to_valid_asset_id(&id, &type_),
                serde_json::to_string(&id).unwrap_or_else(|_| id.to_string()),
                if maybe_self_reference { ", true" } else { "" },
                if is_ts { "!" } else { "" },
            ),
            None,
            None,
        );
        if i < assets.len() - 1 {
            context.newline();
        }
    }
}

fn gen_hoists(hoists: &Vec<Option<JSChildNode>>, context: &mut CodegenContext) {
    if hoists.is_empty() {
        return;
    }
    context.pure = true;
    context.newline();

    for (i, exp) in hoists.clone().into_iter().enumerate() {
        if let Some(exp) = exp {
            context.push(&format!("const _hoisted_{} = ", i + 1), None, None);
            gen_node(CodegenNode::from(exp), context);
            context.newline();
        }
    }

    context.pure = false;
}

fn gen_node_list_as_array(nodes: Vec<GenNodeListNode>, context: &mut CodegenContext) {
    let multilines = if nodes.len() > 3 {
        true
    } else if (!context.global_compile_time_constants.__browser__
        || context.global_compile_time_constants.__dev__)
        && nodes
            .iter()
            .any(|n| matches!(n, GenNodeListNode::CodegenNode(_)))
    {
        true
    } else {
        false
    };
    context.push("[", None, None);
    if multilines {
        context.indent();
    }
    gen_node_list(nodes, context, Some(multilines), None);
    if multilines {
        context.deindent(None);
    }
    context.push("]", None, None);
}

#[derive(Debug)]
enum GenNodeListNode {
    String(String),
    CodegenNode(CodegenNode),
    TemplateChildNodeList(Vec<TemplateChildNode>),
}

impl From<VNodeCallChildren> for GenNodeListNode {
    fn from(value: VNodeCallChildren) -> Self {
        match value {
            VNodeCallChildren::TemplateChildNodeList(list) => Self::TemplateChildNodeList(list),
            VNodeCallChildren::TemplateTextChildNode(node) => match node {
                TemplateTextChildNode::Text(node) => Self::CodegenNode(CodegenNode::Text(node)),
                TemplateTextChildNode::Interpolation(node) => {
                    Self::CodegenNode(CodegenNode::Interpolation(node))
                }
                TemplateTextChildNode::Compound(node) => {
                    Self::CodegenNode(CodegenNode::Compound(node))
                }
            },
        }
    }
}

impl From<PropsExpression> for GenNodeListNode {
    fn from(value: PropsExpression) -> Self {
        match value {
            PropsExpression::Object(node) => Self::CodegenNode(CodegenNode::Object(node)),
        }
    }
}

impl From<CallArgument> for GenNodeListNode {
    fn from(value: CallArgument) -> Self {
        match value {
            CallArgument::String(node) => Self::String(node),
            CallArgument::JSChild(node) => Self::CodegenNode(CodegenNode::from(node)),
            CallArgument::SSRCodegen(node) => Self::CodegenNode(CodegenNode::from(node)),
            CallArgument::TemplateChild(node) => Self::CodegenNode(CodegenNode::from(node)),
            CallArgument::TemplateChildren(node) => Self::TemplateChildNodeList(node),
        }
    }
}

fn gen_node_list(
    nodes: Vec<GenNodeListNode>,
    context: &mut CodegenContext,
    multilines: Option<bool>,
    comma: Option<bool>,
) {
    let multilines = multilines.unwrap_or_default();
    let comma = comma.unwrap_or(true);
    let nodes_len = nodes.len();
    for (i, node) in nodes.into_iter().enumerate() {
        match node {
            GenNodeListNode::String(node) => {
                context.push(&node, Some(NewlineType::Unknown), None);
            }
            GenNodeListNode::TemplateChildNodeList(node) => {
                gen_node_list_as_array(
                    node.into_iter()
                        .map(CodegenNode::from)
                        .map(|n| GenNodeListNode::CodegenNode(n))
                        .collect(),
                    context,
                );
            }
            GenNodeListNode::CodegenNode(node) => {
                gen_node(node, context);
            }
        }
        if i < nodes_len - 1 {
            if multilines {
                if comma {
                    context.push(",", None, None);
                }
                context.newline();
            } else {
                if comma {
                    context.push(", ", None, None);
                }
            }
        }
    }
}

fn gen_node(node: CodegenNode, context: &mut CodegenContext) {
    match node {
        CodegenNode::Element(node) => match node {
            ElementNode::PlainElement(node) => {
                if context.global_compile_time_constants.__dev__ && node.codegen_node.is_none() {
                    println!(
                        "Codegen node is missing for element/if/for node. ` +
                          `Apply appropriate transforms first."
                    );
                }

                let PlainElementNode { codegen_node, .. } = node;
                if let Some(codegen_node) = codegen_node {
                    gen_node(CodegenNode::from(codegen_node), context);
                }
            }
            ElementNode::Component(node) => {
                if context.global_compile_time_constants.__dev__ && node.codegen_node.is_none() {
                    println!(
                        "Codegen node is missing for element/if/for node. ` +
                          `Apply appropriate transforms first."
                    );
                }

                let ComponentNode { codegen_node, .. } = node;
                if let Some(codegen_node) = codegen_node {
                    gen_node(CodegenNode::from(codegen_node), context);
                }
            }
            ElementNode::SlotOutlet(node) => {
                if context.global_compile_time_constants.__dev__ && node.codegen_node.is_none() {
                    println!(
                        "Codegen node is missing for element/if/for node. ` +
                          `Apply appropriate transforms first."
                    );
                }

                let SlotOutletNode { codegen_node, .. } = node;
                if let Some(_codegen_node) = codegen_node {
                    todo!()
                    // gen_node(CodegenNode::from(codegen_node), context);
                }
            }
            ElementNode::Template(node) => {
                if context.global_compile_time_constants.__dev__ && node.codegen_node.is_none() {
                    println!(
                        "Codegen node is missing for element/if/for node. ` +
                          `Apply appropriate transforms first."
                    );
                }
            }
        },
        CodegenNode::If(node) => {
            if context.global_compile_time_constants.__dev__ && node.codegen_node.is_none() {
                println!(
                    "Codegen node is missing for element/if/for node. ` +
                      `Apply appropriate transforms first."
                );
            }

            let IfNode { codegen_node, .. } = node;
            if let Some(codegen_node) = codegen_node {
                gen_node(CodegenNode::from(codegen_node), context);
            }
        }
        CodegenNode::For(node) => {
            if context.global_compile_time_constants.__dev__ && node.codegen_node.is_none() {
                println!(
                    "Codegen node is missing for element/if/for node. ` +
                      `Apply appropriate transforms first."
                );
            }

            let ForNode { codegen_node, .. } = node;
            if let Some(codegen_node) = codegen_node {
                gen_node(CodegenNode::VNodeCall(codegen_node.into()), context);
            }
        }
        CodegenNode::Text(text) => {
            gen_text(text, context);
        }
        CodegenNode::Simple(node) => {
            gen_expression(node, context);
        }
        CodegenNode::Interpolation(node) => {
            gen_interpolation(node, context);
        }
        CodegenNode::Compound(node) => {
            gen_compound_expression(node, context);
        }
        CodegenNode::Comment(node) => {
            gen_comment(node, context);
        }
        CodegenNode::VNodeCall(node) => {
            gen_vnode_call(node, context);
        }
        CodegenNode::Call(node) => {
            gen_call_expression(node, context);
        }
        CodegenNode::Object(node) => {
            gen_object_expression(node, context);
        }
        CodegenNode::Array(node) => {
            gen_array_expression(node, context);
        }
        // NodeTypes.JS_CONDITIONAL_EXPRESSION
        CodegenNode::IfConditional(node) => {
            gen_if_conditional_expression(node, context);
        }
        CodegenNode::Cache(node) => {
            gen_cache_expression(node, context);
        }
        // SSR only types
        CodegenNode::TemplateLiteral(node) => {
            if !context.global_compile_time_constants.__browser__ {
                gen_template_literal(node, context);
            }
        }
        /* v8 ignore start */
        CodegenNode::IfBranch(_) => {
            // noop
        } // _ => {
          //     println!("node {:#?}", node);
          //     unreachable!()
          // }
    }
}

fn gen_text(node: TextNode, context: &mut CodegenContext) {
    let code = serde_json::to_string(&node.content).unwrap_or_else(|_| node.content.clone());
    context.push(
        &code,
        Some(NewlineType::Unknown),
        Some(CodegenNode::Text(node.clone())),
    );
}

fn gen_expression(node: SimpleExpressionNode, context: &mut CodegenContext) {
    if node.is_static {
        context.push(
            &serde_json::to_string(&node.content).unwrap_or_else(|_| node.content.clone()),
            Some(NewlineType::Unknown),
            Some(CodegenNode::Simple(node.clone())),
        )
    } else {
        context.push(
            &node.content,
            Some(NewlineType::Unknown),
            Some(CodegenNode::Simple(node.clone())),
        )
    }
}

fn gen_interpolation(node: InterpolationNode, context: &mut CodegenContext) {
    if context.pure {
        context.push(PURE_ANNOTATION, None, None);
    }
    context.push(
        &format!("{}(", context.helper(ToDisplayString.to_string())),
        None,
        None,
    );
    gen_node(CodegenNode::from(node.content.clone()), context);
    context.push(")", None, None);
}

fn gen_compound_expression(node: CompoundExpressionNode, context: &mut CodegenContext) {
    let CompoundExpressionNode { children, .. } = node;
    for child in children {
        let node = match child {
            CompoundExpressionNodeChild::Simple(node) => CodegenNode::Simple(node),
            CompoundExpressionNodeChild::Compound(node) => CodegenNode::Compound(node),
            CompoundExpressionNodeChild::Interpolation(node) => CodegenNode::Interpolation(node),
            CompoundExpressionNodeChild::Text(node) => CodegenNode::Text(node),
            CompoundExpressionNodeChild::String(str) => {
                context.push(&str, Some(NewlineType::Unknown), None);
                continue;
            }
        };

        gen_node(node, context);
    }
}

fn gen_expression_as_property_key(node: ExpressionNode, context: &mut CodegenContext) {
    match node {
        ExpressionNode::Compound(node) => {
            context.push("[", None, None);
            gen_compound_expression(node, context);
            context.push("]", None, None);
        }
        ExpressionNode::Simple(node) if node.is_static => {
            // only quote keys if necessary
            if is_simple_identifier(&node.content) {
                context.push(
                    &node.content,
                    Some(NewlineType::None),
                    Some(CodegenNode::Simple(node.clone())),
                );
            } else {
                let text =
                    serde_json::to_string(&node.content).unwrap_or_else(|_| node.content.clone());
                context.push(
                    &text,
                    Some(NewlineType::None),
                    Some(CodegenNode::Simple(node.clone())),
                );
            }
        }
        ExpressionNode::Simple(node) => {
            context.push(
                &format!("[{}]", node.content),
                Some(NewlineType::Unknown),
                Some(CodegenNode::Simple(node.clone())),
            );
        }
    }
}

fn gen_comment(node: CommentNode, context: &mut CodegenContext) {
    if context.pure {
        context.push(PURE_ANNOTATION, None, None);
    }
    context.push(
        &format!(
            "{}({})",
            context.helper(CreateComment.to_string()),
            serde_json::to_string(&node.content).unwrap_or_else(|_| node.content.clone())
        ),
        None,
        None,
    );
}

fn gen_vnode_call(node: VNodeCall, context: &mut CodegenContext) {
    // add dev annotations to patch flags
    let patch_flag_string = if let Some(patch_flag) = node.patch_flag {
        let patch_flag_string = if context.global_compile_time_constants.__dev__ {
            if patch_flag < 0 {
                // special flags (negative and mutually exclusive)
                format!("{} /* {} */", patch_flag, patch_flag.as_str())
            } else {
                // bitwise flags
                let flag_names = PatchFlags::keys()
                    .into_iter()
                    .filter_map(|n| {
                        if n > 0 && (patch_flag & n) != 0 {
                            Some(n.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} /* {flag_names} */", patch_flag)
            }
        } else {
            patch_flag.to_string()
        };
        Some(patch_flag_string)
    } else {
        None
    };

    if node.is_block {
        context.push(
            &format!(
                "({}({}), ",
                context.helper(OpenBlock.to_string()),
                if node.disable_tracking { "true" } else { "" }
            ),
            None,
            None,
        );
    }
    if context.pure {
        context.push(PURE_ANNOTATION, None, None);
    }
    let call_helper = if node.is_block {
        get_vnode_block_helper(context.in_ssr, node.is_component)
    } else {
        get_vnode_helper(context.in_ssr, node.is_component)
    };
    context.push(
        &format!("{}(", context.helper(call_helper)),
        Some(NewlineType::None),
        Some(CodegenNode::VNodeCall(node.clone())),
    );
    let nodes = {
        let mut nodes = Vec::new();
        if let Some(patch_flag) = patch_flag_string {
            nodes.push(GenNodeListNode::String(patch_flag));
        }
        if let Some(children) = node.children.clone() {
            nodes.push(GenNodeListNode::from(children));
        } else if nodes.len() != 0 {
            nodes.push(GenNodeListNode::String("null".to_string()));
        }
        if let Some(props) = node.props.clone() {
            nodes.push(GenNodeListNode::from(props));
        } else if nodes.len() != 0 {
            nodes.push(GenNodeListNode::String("null".to_string()));
        }
        nodes.push(GenNodeListNode::String(node.tag.clone()));
        nodes.reverse();
        nodes
    };
    gen_node_list(nodes, context, None, None);
    context.push(")", None, None);
    if node.is_block {
        context.push(")", None, None);
    }
}

// JavaScript
fn gen_call_expression(node: CallExpression, context: &mut CodegenContext) {
    let callee = match node.callee.clone() {
        CallCallee::String(callee) => callee,
        CallCallee::Symbol(callee) => context.helper(callee),
    };
    if context.pure {
        context.push(PURE_ANNOTATION, None, None);
    }
    context.push(
        &format!("{callee}("),
        Some(NewlineType::None),
        Some(CodegenNode::Call(node.clone())),
    );
    gen_node_list(
        node.arguments
            .into_iter()
            .map(GenNodeListNode::from)
            .collect(),
        context,
        None,
        None,
    );
    context.push(")", None, None);
}

fn gen_object_expression(node: ObjectExpression, context: &mut CodegenContext) {
    let ObjectExpression { properties, .. } = node.clone();
    if properties.is_empty() {
        context.push(
            "{}",
            Some(NewlineType::None),
            Some(CodegenNode::Object(node)),
        );
        return;
    }
    let multilines = properties.len() > 1
        || ((!context.global_compile_time_constants.__browser__
            || context.global_compile_time_constants.__dev__)
            && properties
                .iter()
                .any(|p| !matches!(p.value, JSChildNode::Simple(_))));
    context.push(if multilines { "{" } else { "{ " }, None, None);
    if multilines {
        context.indent();
    }
    let properties_len = properties.len();
    for (i, Property { key, value, .. }) in properties.into_iter().map(|p| p).enumerate() {
        // key
        gen_expression_as_property_key(key, context);
        context.push(": ", None, None);
        // value
        gen_node(CodegenNode::from(value), context);
        if i < properties_len - 1 {
            // will only reach this if it's multilines
            context.push(",", None, None);
            context.newline();
        }
    }
    if multilines {
        context.deindent(None);
    }
    context.push(if multilines { "}" } else { " }" }, None, None);
}

fn gen_array_expression(node: ArrayExpression, context: &mut CodegenContext) {
    gen_node_list_as_array(
        node.elements
            .into_iter()
            .map(|e| GenNodeListNode::CodegenNode(e))
            .collect(),
        context,
    );
}

fn gen_if_conditional_expression(node: IfConditionalExpression, context: &mut CodegenContext) {
    let IfConditionalExpression {
        test,
        consequent,
        alternate,
        newline: need_new_line,
    } = node;
    if let JSChildNode::Simple(test) = test {
        let needs_parens = !is_simple_identifier(&test.content);
        if needs_parens {
            context.push("(", None, None);
        }
        gen_expression(test, context);
        if needs_parens {
            context.push(")", None, None);
        }
    } else {
        context.push("(", None, None);
        gen_node(CodegenNode::from(test), context);
        context.push(")", None, None);
    }
    if need_new_line {
        context.indent();
    }
    context.indent_level += 1;
    if !need_new_line {
        context.push(" ", None, None);
    }
    context.push("? ", None, None);
    gen_node(CodegenNode::from(consequent), context);
    context.indent_level -= 1;
    if need_new_line {
        context.newline();
    } else {
        context.push(" ", None, None);
    }
    context.push(": ", None, None);
    let is_nested = matches!(alternate, JSChildNode::IfConditional(_));
    if !is_nested {
        context.indent_level += 1;
    }
    gen_node(CodegenNode::from(alternate), context);
    if !is_nested {
        context.indent_level -= 1;
    }
    if need_new_line {
        context.deindent(Some(true) /* without newline */);
    }
}

fn gen_cache_expression(node: CacheExpression, context: &mut CodegenContext) {
    let CacheExpression {
        index,
        value,
        need_pause_tracking,
        in_v_once,
        need_array_spread,
        ..
    } = node;
    if need_array_spread {
        context.push("[...(", None, None);
    }
    context.push(&format!("_cache[{}] || (", index), None, None);
    if need_pause_tracking {
        context.indent();
        context.push(
            &format!("{}(-1", context.helper(SetBlockTracking.to_string())),
            None,
            None,
        );
        if in_v_once {
            context.push(", true", None, None);
        }
        context.push("),", None, None);
        context.newline();
        context.push("(", None, None);
    }
    context.push(&format!("_cache[{}] = ", index), None, None);
    gen_node(CodegenNode::from(value), context);
    if need_pause_tracking {
        context.push(&format!(").cacheIndex = {},", index), None, None);
        context.newline();
        context.push(
            &format!("{}(1),", context.helper(SetBlockTracking.to_string())),
            None,
            None,
        );
        context.newline();
        context.push(&format!("_cache[{}]", index), None, None);
        context.deindent(None);
    }
    context.push(")", None, None);
    if need_array_spread {
        context.push(")]", None, None);
    }
}

fn gen_template_literal(node: TemplateLiteral, context: &mut CodegenContext) {
    context.push("`", None, None);
    let l = node.elements.len();
    let multilines = l > 3;
    for e in node.elements {
        match e {
            TemplateLiteralElement::String(node) => {
                context.push(
                    &node
                        .replace('\\', r"\\")
                        .replace('$', r"\$")
                        .replace('`', r"\`"),
                    Some(NewlineType::Unknown),
                    None,
                );
            }
            TemplateLiteralElement::JSChild(node) => {
                context.push("${", None, None);
                if multilines {
                    context.indent();
                }
                gen_node(CodegenNode::from(node), context);
                if multilines {
                    context.deindent(None);
                }
                context.push("}", None, None);
            }
        }
    }
    context.push("`", None, None);
}
