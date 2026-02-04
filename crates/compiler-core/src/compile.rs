use crate::{
    ast::RootNode,
    codegen::{CodegenResult, generate},
    options::CompilerOptions,
    parser::base_parse,
    transform::{DirectiveTransform, NodeTransform, transform},
    transforms::{
        transform_element::transform_element,
        // transform_v_bind_shorthand::TransformVBindShorthand,
        v_bind::TransformBind,
        v_for::transform_for,
        v_if::transform_if,
    },
};
use std::collections::HashMap;

pub type TransformPreset = (
    Vec<NodeTransform>,
    HashMap<String, Box<dyn DirectiveTransform>>,
);

pub fn get_base_transform_preset() -> TransformPreset {
    (
        vec![
            // Box::new(TransformVBindShorthand),
            transform_if,
            transform_for,
            transform_element,
        ],
        HashMap::from([(
            "bind".to_string(),
            Box::new(TransformBind) as Box<dyn DirectiveTransform>,
        )]),
    )
}

pub enum BaseCompileSource {
    String(String),
    RootNode(RootNode),
}

// we name it `baseCompile` so that higher order compilers like
// @vue/compiler-dom can export `compile` while re-exporting everything else.
pub fn base_compile(source: BaseCompileSource, options: CompilerOptions) -> CodegenResult {
    let (parser_options, mut transform_options, codegen_options) = options.into();

    let mut ast = match source {
        BaseCompileSource::String(source) => base_parse(&source, Some(parser_options)),
        BaseCompileSource::RootNode(node) => node,
    };

    let (node_transforms, directive_transforms) = get_base_transform_preset();

    transform_options.node_transforms = Some(node_transforms);
    transform_options.directive_transforms = Some(directive_transforms);

    transform(&mut ast, transform_options);

    generate(ast, codegen_options)
}
