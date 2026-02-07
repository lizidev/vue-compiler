use crate::{
    ast::{
        BlockCodegenNode, CallCallee, ConstantTypes, ElementNode, ExpressionNode, ForCodegenNode,
        ForIteratorExpression, ForNode, ForParseResult, ForRenderListArgument,
        ForRenderListExpression, FunctionParams, PlainElementNodeCodegenNode, TemplateChildNode,
        VNodeCall, VNodeCallTag,
    },
    runtime_helpers::{Fragment, RenderList},
    transform::{
        NodeTransformState, StructuralDirectiveTransform, TransformContext, TransformNode,
    },
    utils::find_prop,
};
use vue_compiler_shared::PatchFlags;

pub fn transform_for(
    node: &TransformNode,
    _context: &mut TransformContext,
) -> Option<Box<dyn NodeTransformState>> {
    if node.children().is_some() {
        Some(Box::new(TransformFor::default()))
    } else {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransformFor(Vec<usize>);

impl StructuralDirectiveTransform for TransformFor {
    fn matches(&self, name: &String) -> bool {
        matches!(name.as_str(), "for")
    }
}

impl NodeTransformState for TransformFor {
    fn pre_transform(&mut self, parent: &mut TransformNode, context: &mut TransformContext) {
        let mut i = 0;
        let Some(children) = parent.children_mut() else {
            unreachable!();
        };
        loop {
            if i >= children.len() {
                break;
            }

            let dirs = if let TemplateChildNode::Element(node) = &mut children[i]
                && let Some(dirs) = StructuralDirectiveTransform::transform(self, node)
            {
                dirs
            } else {
                i += 1;
                continue;
            };

            for dir in dirs {
                let Some(parse_result) = dir.for_parse_result else {
                    unreachable!();
                };

                let ForParseResult {
                    source,
                    value,
                    key,
                    index,
                    ..
                } = parse_result.clone();

                let for_node = ForNode {
                    source,
                    value_alias: value,
                    key_alias: key,
                    object_index_alias: index,
                    parse_result: parse_result,
                    children: vec![],
                    codegen_node: None,
                    loc: dir.loc.clone(),
                };

                children.push(TemplateChildNode::For(for_node));
                let node = children.swap_remove(i);
                self.0.push(i);
                if let TemplateChildNode::For(for_node) = &mut children[i] {
                    let TemplateChildNode::Element(node) = node else {
                        unreachable!();
                    };
                    process_codegen(for_node, &node, context);
                    for_node.children = if let ElementNode::Template(node) = node {
                        node.children
                    } else {
                        vec![TemplateChildNode::Element(node)]
                    };
                } else {
                    unreachable!();
                }
            }
        }
    }

    fn pre_exit(&mut self, node: &mut TransformNode, context: &mut TransformContext) {
        let Some(children) = node.children_mut() else {
            unreachable!();
        };
        for (key, index) in self.0.drain(..).enumerate() {
            let TemplateChildNode::For(for_node) = &mut children[index] else {
                unreachable!();
            };

            let is_stable_fragment = matches!(&for_node.source, ExpressionNode::Simple(node) if node.const_type > ConstantTypes::NotConstant);

            let child_block = {
                // Normal element v-for. Directly use the child's codegenNode
                // but mark it as a block.
                let TemplateChildNode::Element(ElementNode::PlainElement(node)) =
                    &for_node.children[0]
                else {
                    unreachable!();
                };

                let Some(PlainElementNodeCodegenNode::VNodeCall(mut child_block)) =
                    node.codegen_node.clone()
                else {
                    unreachable!();
                };

                // TODO
                child_block.is_block = !is_stable_fragment;

                BlockCodegenNode::VNodeCall(child_block)
            };

            let Some(codegen_node) = &mut for_node.codegen_node else {
                unreachable!();
            };
            let arguments = &mut codegen_node.children.arguments;

            arguments.push(ForRenderListArgument::ForIterator(ForIteratorExpression {
                params: Some(FunctionParams::ExpressionList(create_for_loop_params(
                    &for_node.parse_result,
                    Default::default(),
                ))),
                returns: Some(child_block),
                /* force newline */
                newline: true,
            }));
        }
    }
}

fn process_codegen(for_node: &mut ForNode, node: &ElementNode, context: &mut TransformContext) {
    // create the loop render function expression now, and add the
    // iterator on exit after all children have been traversed
    context.helper(RenderList.to_string());
    let render_exp = ForRenderListExpression::new(
        CallCallee::Symbol(context.helper(RenderList.to_string())),
        Some(vec![ForRenderListArgument::Expression(
            for_node.source.clone(),
        )]),
        None,
    );
    let key_prop = find_prop(node, "key", Some(false), Some(true));

    let is_stable_fragment = matches!(&for_node.source, ExpressionNode::Simple(node) if node.const_type > ConstantTypes::NotConstant);
    let fragment_flag = if is_stable_fragment {
        PatchFlags::StableFragment
    } else if key_prop.is_some() {
        PatchFlags::KeyedFragment
    } else {
        PatchFlags::UnkeyedFragment
    };

    let tag = context.helper(Fragment.to_string());
    let codegen_node = VNodeCall::new(
        Some(context),
        VNodeCallTag::Symbol(tag),
        None,
        None,
        Some(fragment_flag),
        /* isBlock */
        Some(true),
        /* disableTracking */
        Some(!is_stable_fragment),
        /* isComponent */
        Some(false),
        Some(for_node.loc.clone()),
    );
    let Some(patch_flag) = codegen_node.patch_flag else {
        unreachable!();
    };
    let VNodeCallTag::Symbol(tag) = codegen_node.tag else {
        unreachable!();
    };
    let codegen_node = ForCodegenNode {
        tag,
        children: render_exp,
        patch_flag,
        disable_tracking: codegen_node.disable_tracking,
        is_component: false,
        loc: codegen_node.loc,
    };

    for_node.codegen_node = Some(codegen_node);
}

fn create_for_loop_params(
    for_parse_result: &ForParseResult,
    memo_args: Vec<ExpressionNode>,
) -> Vec<ExpressionNode> {
    let mut args = vec![
        for_parse_result.value.clone(),
        for_parse_result.key.clone(),
        for_parse_result.index.clone(),
    ];
    args.extend(memo_args.into_iter().map(|arg| Some(arg)));
    let index = args
        .iter()
        .rposition(|arg| arg.is_some())
        .map_or(args.len(), |i| i + 1);

    args.drain(0..index)
        .into_iter()
        .enumerate()
        .map(|(i, arg)| {
            if let Some(arg) = arg {
                arg
            } else {
                ExpressionNode::new_simple("_".repeat(i + 1), Some(false), None, None)
            }
        })
        .collect()
}
