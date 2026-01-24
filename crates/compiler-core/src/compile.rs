use std::collections::HashMap;

use crate::{
    ast::RootNode,
    codegen::{CodegenResult, generate},
    options::{CodegenOptions, CompilerOptions},
    parser::base_parse,
    transform::{DirectiveTransform, NodeTransform, transform},
    transforms::{
        transform_element::TransformElement, transform_v_bind_shorthand::TransformVBindShorthand,
        v_bind::TransformBind, v_if::TransformIf,
    },
};

pub type TransformPreset = (
    Vec<Box<dyn NodeTransform>>,
    HashMap<String, Box<dyn DirectiveTransform>>,
);

pub fn get_base_transform_preset() -> TransformPreset {
    (
        vec![
            Box::new(TransformVBindShorthand),
            Box::new(TransformIf::default()),
            Box::new(TransformElement),
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
    let mut ast = match source {
        BaseCompileSource::String(source) => base_parse(&source, None),
        BaseCompileSource::RootNode(node) => node,
    };

    let (node_transforms, directive_transforms) = get_base_transform_preset();

    let (mut transform_options,) = options.into();
    transform_options.node_transforms = Some(node_transforms);
    transform_options.directive_transforms = Some(directive_transforms);

    transform(&mut ast, transform_options);

    generate(ast, CodegenOptions::default())
}
