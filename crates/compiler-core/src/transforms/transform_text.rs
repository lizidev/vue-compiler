use crate::{
    ast::{
        BaseElementProps, CallArgument, CallCallee, CallExpression, CompoundExpressionNode,
        CompoundExpressionNodeChild, ConstantTypes, ElementNode, NodeTypes, TemplateChildNode,
        TextCallCodegenNode, TextCallContent, TextCallNode,
    },
    runtime_helpers::CreateText,
    transform::{NodeTransformState, TransformContext, TransformNode},
    transforms::cache_static::get_constant_type,
    utils::is_text,
};
use vue_compiler_shared::PatchFlags;

/// Merge adjacent text nodes and expressions into a single expression
/// e.g. <div>abc {{ d }} {{ e }}</div> should have a single expression node as child.
pub fn transform_text(
    node: &TransformNode,
    _context: &mut TransformContext,
) -> Option<Box<dyn NodeTransformState>> {
    if matches!(
        node.type_(),
        NodeTypes::Root | NodeTypes::Element | NodeTypes::For | NodeTypes::IfBranch
    ) {
        Some(Box::new(TransformText))
    } else {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransformText;

impl NodeTransformState for TransformText {
    /// perform the transform on node exit so that all expressions have already
    /// been processed.
    fn exit(&mut self, node: &mut TransformNode, context: &mut TransformContext) {
        let mut has_text = false;
        let children_len = {
            let Some(children) = node.children_mut() else {
                unreachable!();
            };

            let mut i = 0;
            loop {
                if i == children.len() {
                    break;
                }
                if !is_text(children[i].type_()) {
                    i += 1;
                    continue;
                };
                has_text = true;
                let j = i + 1;
                loop {
                    if j == children.len() {
                        break;
                    }

                    if !is_text(children[j].type_()) {
                        break;
                    }

                    if is_text(children[i].type_()) {
                        let (child, loc) = match children[i].clone() {
                            TemplateChildNode::Text(child) => {
                                let loc = child.loc.clone();
                                (CompoundExpressionNodeChild::Text(child), loc)
                            }
                            TemplateChildNode::Interpolation(child) => {
                                let loc = child.loc.clone();
                                (CompoundExpressionNodeChild::Interpolation(child), loc)
                            }
                            _ => {
                                unreachable!();
                            }
                        };
                        children[i] = TemplateChildNode::Compound(CompoundExpressionNode::new(
                            vec![child],
                            Some(loc),
                        ));
                    }

                    // merge adjacent text node into current
                    let next = children.remove(j);
                    let next = match next {
                        TemplateChildNode::Text(next) => CompoundExpressionNodeChild::Text(next),
                        TemplateChildNode::Interpolation(next) => {
                            CompoundExpressionNodeChild::Interpolation(next)
                        }
                        _ => {
                            unreachable!();
                        }
                    };
                    if let TemplateChildNode::Compound(node) = &mut children[i] {
                        node.children
                            .push(CompoundExpressionNodeChild::String(" + ".to_string()));
                        node.children.push(next);
                    }
                }
                i += 1;
            }
            children.len()
        };

        if !has_text {
            return;
        }
        // if this is a plain element with a single text child, leave it
        // as-is since the runtime has dedicated fast path for this by directly
        // setting textContent of the element.
        // for component root it's always normalized anyway.
        if children_len == 1 {
            if node.type_() == NodeTypes::Root {
                return;
            } else if let TransformNode::TemplateChild(TemplateChildNode::Element(node)) = &node
                && let ElementNode::PlainElement(node) = node &&
                // #3756
                // custom directives can potentially add DOM elements arbitrarily,
                // we need to avoid setting textContent of the element at runtime
                // to avoid accidentally overwriting the DOM elements added
                // by the user through custom directives.
                !node.props.iter().any(|p| {
                    if let BaseElementProps::Directive(p) = p {
                        !context.directive_transforms.contains_key(&p.name)
                    } else {
                        false
                    }
                })
            {
                return;
            }
        }
        let Some(children) = node.children_mut() else {
            unreachable!();
        };
        // pre-convert text nodes into createTextVNode(text) calls to avoid
        // runtime normalization.
        let mut i = 0;
        loop {
            if i == children.len() {
                break;
            }
            if !(is_text(children[i].type_())
                || children[i].type_() == NodeTypes::CompoundExpression)
            {
                i += 1;
                continue;
            };
            let child = children[i].clone();
            let mut call_args = vec![];
            // createTextVNode defaults to single whitespace, so if it is a
            // single space the code could be an empty call to save bytes.
            if let TemplateChildNode::Text(child) = &child {
                if child.content != " " {
                    call_args.push(CallArgument::TemplateChild(TemplateChildNode::Text(
                        child.clone(),
                    )));
                }
            } else {
                call_args.push(CallArgument::TemplateChild(child.clone()));
            }
            // mark dynamic text with flag so it gets patched inside a block
            if !context.ssr && get_constant_type(&child, context) == ConstantTypes::NotConstant {
                let comment = if context.global_compile_time_constants.__dev__ {
                    " /* ${PatchFlagNames[PatchFlags.TEXT]} */"
                } else {
                    ""
                };
                call_args.push(CallArgument::String(format!(
                    "{}{}",
                    PatchFlags::Text,
                    comment
                )));
            }

            let (loc, content) = match child {
                TemplateChildNode::Text(node) => (node.loc.clone(), TextCallContent::Text(node)),
                TemplateChildNode::Interpolation(node) => {
                    (node.loc.clone(), TextCallContent::Interpolation(node))
                }
                TemplateChildNode::Compound(node) => {
                    (node.loc.clone(), TextCallContent::Compound(node))
                }
                _ => {
                    unreachable!();
                }
            };
            children[i] = TemplateChildNode::TextCall(TextCallNode {
                content,
                codegen_node: TextCallCodegenNode::Call(CallExpression::new(
                    CallCallee::Symbol(context.helper(CreateText.to_string())),
                    Some(call_args),
                    None,
                )),
                loc,
            });
        }
    }
}
