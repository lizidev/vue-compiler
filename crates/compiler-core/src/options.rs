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
