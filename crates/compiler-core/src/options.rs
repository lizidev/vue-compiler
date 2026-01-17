use crate::{
    ast::{ElementNode, Namespace, Namespaces},
    errors::{CompilerError, DefaultErrorHandlingOptions},
    tokenizer::ParseMode,
    utils::GlobalCompileTimeConstants,
};

pub trait ErrorHandlingOptions: std::fmt::Debug {
    fn on_warn(&mut self, warning: CompilerError) {
        // __DEV__
        println!("[Vue warn] {:?}", warning);
    }
    fn on_error(&mut self, error: CompilerError) {
        println!("{:?}", error);
    }
}

#[derive(Debug, PartialEq)]
pub enum Whitespace {
    Preserve,
    Condense,
}

pub struct ParserOptions {
    /// Base mode is platform agnostic and only parses HTML-like template syntax,
    /// treating all tags the same way. Specific tag parsing behavior can be
    /// configured by higher-level compilers.
    ///
    /// HTML mode adds additional logic for handling special parsing behavior in
    /// `<script>`, `<style>`,`<title>` and `<textarea>`.
    /// The logic is handled inside compiler-core for efficiency.
    ///
    /// SFC mode treats content of all root-level tags except `<template>` as plain
    /// text.
    pub parse_mode: ParseMode,
    /// Specify the root namespace to use when parsing a template.
    /// Defaults to `Namespaces.HTML` (0).
    pub ns: Namespaces,
    /// e.g. platform native elements, e.g. `<div>` for browsers
    pub is_native_tag: Option<Box<dyn Fn(&String) -> bool>>,
    /// e.g. native elements that can self-close, e.g. `<img>`, `<br>`, `<hr>`
    pub is_void_tag: Box<dyn Fn(&String) -> bool>,
    /// e.g. elements that should preserve whitespace inside, e.g. `<pre>`
    pub is_pre_tag: Box<dyn Fn(&String) -> bool>,
    /// Platform-specific built-in components e.g. `<Transition>`
    pub is_built_in_component: Option<Box<dyn Fn(&String) -> Option<()>>>,
    /// Separate option for end users to extend the native elements list
    pub is_custom_element: Option<Box<dyn Fn(&String) -> Option<bool>>>,
    /// Transform expressions like {{ foo }} to `_ctx.foo`.
    /// If this option is false, the generated code will be wrapped in a
    /// `with (this) { ... }` block.
    /// - This is force-enabled in module mode, since modules are by default strict
    /// and cannot use `with`
    /// @default mode === 'module'
    pub prefix_identifiers: Option<bool>,
    /// Get tag namespace
    /// (tag: string, parent: ElementNode | undefined, rootNamespace: Namespace) => Namespace
    pub get_namespace: Box<dyn Fn(&String, Option<&ElementNode>, Namespace) -> Namespace>,
    /// Whitespace handling strategy
    /// @default 'condense'
    pub whitespace: Option<Whitespace>,
    /// Whether to keep comments in the templates AST.
    /// This defaults to `true` in development and `false` in production builds.
    pub comments: Option<bool>,

    pub error_handling_options: Box<dyn ErrorHandlingOptions>,

    /// Global compile-time constants
    pub global_compile_time_constants: GlobalCompileTimeConstants,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            parse_mode: ParseMode::BASE,
            ns: Namespaces::HTML,
            is_native_tag: None,
            is_void_tag: Box::new(|_| false),
            is_pre_tag: Box::new(|_| false),
            is_built_in_component: None,
            is_custom_element: None,
            prefix_identifiers: Some(false),
            get_namespace: Box::new(|_, _, _| Namespaces::HTML as u32),
            whitespace: None,
            comments: None,

            error_handling_options: Box::new(DefaultErrorHandlingOptions),

            global_compile_time_constants: GlobalCompileTimeConstants::default(),
        }
    }
}

impl std::fmt::Debug for ParserOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("ParserOptions");
        debug_struct
            .field("parse_mode", &self.parse_mode)
            .field("ns", &self.ns)
            .field(
                "is_native_tag",
                &"Option<Box<dyn Fn(&String) -> Option<bool>>>",
            )
            .field("is_void_tag", &"<Fn(&String) -> bool>")
            .field("is_pre_tag", &"<Fn(&String) -> bool>")
            .field(
                "is_custom_element",
                &"Option<Box<dyn Fn(&String) -> Option<bool>>>",
            )
            .field("error_handling_options", &self.error_handling_options)
            .field(
                "global_compile_time_constants",
                &self.global_compile_time_constants,
            );

        debug_struct.finish()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CodegenMode {
    Module,
    Function,
}

#[derive(Debug)]
pub struct BindingMetadata;

#[derive(Debug)]
pub struct CodegenOptions {
    // SharedTransformCodegenOptions
    /// Transform expressions like {{ foo }} to `_ctx.foo`.
    /// If this option is false, the generated code will be wrapped in a
    /// `with (this) { ... }` block.
    /// - This is force-enabled in module mode, since modules are by default strict
    /// and cannot use `with`
    /// @default mode === 'module'
    pub prefix_identifiers: Option<bool>,
    /// Control whether generate SSR-optimized render functions instead.
    /// The resulting function must be attached to the component via the
    /// `ssrRender` option instead of `render`.
    ///
    /// When compiler generates code for SSR's fallback branch, we need to set it to false:
    /// - context.ssr = false
    ///
    /// see `subTransform` in `ssrTransformComponent.ts`
    pub ssr: Option<bool>,
    /// Indicates whether the compiler generates code for SSR,
    /// it is always true when generating code for SSR,
    /// regardless of whether we are generating code for SSR's fallback branch,
    /// this means that when the compiler generates code for SSR's fallback branch:
    ///  - context.ssr = false
    ///  - context.inSSR = true
    pub in_ssr: Option<bool>,
    /// Optional binding metadata analyzed from script - used to optimize
    /// binding access when `prefixIdentifiers` is enabled.
    pub binding_metadata: Option<BindingMetadata>,
    /// Compile the function for inlining inside setup().
    /// This allows the function to directly access setup() local bindings.
    pub inline: Option<bool>,
    /// Indicates that transforms and codegen should try to output valid TS code
    pub is_ts: Option<bool>,

    /// - `module` mode will generate ES module import statements for helpers
    /// and export the render function as the default export.
    /// - `function` mode will generate a single `const { helpers... } = Vue`
    /// statement and return the render function. It expects `Vue` to be globally
    /// available (or passed by wrapping the code with an IIFE). It is meant to be
    /// used with `new Function(code)()` to generate a render function at runtime.
    /// @default 'function'
    pub mode: Option<CodegenMode>,
    /// SFC scoped styles ID
    pub scope_id: Option<String>,
    /// Option to optimize helper import bindings via variable assignment
    /// (only used for webpack code-split)
    /// @default false
    pub optimize_imports: Option<bool>,
    /// Customize where to import runtime helpers from.
    /// @default 'vue'
    pub runtime_module_name: Option<String>,
    /// Customize the global variable name of `Vue` to get helpers from
    /// in function mode
    /// @default 'Vue'
    pub runtime_global_name: Option<String>,

    /// Global compile-time constants
    pub global_compile_time_constants: GlobalCompileTimeConstants,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            prefix_identifiers: None,
            ssr: None,
            in_ssr: None,
            binding_metadata: None,
            inline: None,
            is_ts: None,
            mode: None,
            scope_id: None,
            optimize_imports: None,
            runtime_module_name: None,
            runtime_global_name: None,
            global_compile_time_constants: GlobalCompileTimeConstants::default(),
        }
    }
}
