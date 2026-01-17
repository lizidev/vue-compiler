use vue_compiler_core::{
    BaseElement, ElementNode, ElementTypes, Namespaces, NodeTypes, PlainElementNode,
    PlainElementNodeCodegenNode, PropsExpression, SourceLocation, VNodeCall, VNodeCallChildren,
};
use vue_compiler_shared::PatchFlags;

pub fn create_element_with_codegen(
    tag: impl Into<String>,
    props: Option<PropsExpression>,
    children: Option<VNodeCallChildren>,
    patch_flag: Option<PatchFlags>,
) -> ElementNode {
    ElementNode::PlainElement(PlainElementNode {
        type_: NodeTypes::Element,
        loc: SourceLocation::loc_stub(),
        inner: BaseElement {
            ns: Namespaces::HTML as u32,
            tag: "div".to_string(),
            tag_type: ElementTypes::Element,
            props: Vec::new(),
            children: Vec::new(),
            is_self_closing: None,
            codegen_node: Some(PlainElementNodeCodegenNode::VNodeCall(VNodeCall {
                tag: tag.into(),
                props,
                children,
                patch_flag,
                is_block: false,
                disable_tracking: false,
                is_component: false,
                loc: SourceLocation::loc_stub(),
            })),
            ssr_codegen_node: None,
        },
    })
}

pub fn gen_flag_text(flag: PatchFlags) -> String {
    format!("{} /* {} */", flag.to_i16(), flag.as_str())
}
