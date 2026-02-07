use crate::{
    ast::{
        CallArgument, CallExpression, ComponentNodeCodegenNode, ElementNode, ExpressionNode,
        IfBranchNode, IfCodegenNode, IfConditionalExpression, IfNode, JSChildNode, NodeTypes,
        ObjectExpression, PlainElementNodeCodegenNode, Property, PropsExpression,
        SimpleExpressionNode, SourceLocation, TemplateChildNode, VNodeCall, VNodeCallChildren,
        VNodeCallTag, convert_to_block,
    },
    runtime_helpers::{CreateComment, Fragment},
    transform::{
        NodeTransformState, StructuralDirectiveTransform, TransformContext, TransformNode,
    },
    utils::inject_prop,
};
use vue_compiler_shared::PatchFlags;

pub fn transform_if(
    node: &TransformNode,
    _context: &mut TransformContext,
) -> Option<Box<dyn NodeTransformState>> {
    if node.children().is_some() {
        Some(Box::new(TransformIf::default()))
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct TransformIfState {
    index: usize,
}

fn exit(context: &mut TransformContext, if_node: &mut IfNode, key: usize) {
    let branch = if_node.branches[0].clone();

    let Some(condition) = branch.condition.clone() else {
        unreachable!();
    };

    let mut codegen_node = IfCodegenNode::IfConditional(IfConditionalExpression {
        test: JSChildNode::from(condition),
        consequent: create_children_codegen_node(branch, key, context),
        alternate: JSChildNode::Call(CallExpression::new(
            context.helper(CreateComment.to_string()),
            Some(vec![
                if context.global_compile_time_constants.__dev__ {
                    CallArgument::String("\"v-if\"".to_string())
                } else {
                    CallArgument::String("\"\"".to_string())
                },
                CallArgument::String("true".to_string()),
            ]),
            None,
        )),
        newline: true,
    });
    let branches = if_node.branches.clone().drain(1..).collect::<Vec<_>>();
    for (i, branch) in branches.into_iter().enumerate() {
        // attach this branch's codegen node to the v-if root.
        let parent_condition = codegen_node.get_parent_condition();
        parent_condition.alternate = create_children_codegen_node(branch, key + i + 1, context);
    }
    if_node.codegen_node = Some(codegen_node);
}

#[derive(Debug, Clone, Default)]
pub struct TransformIf(Vec<TransformIfState>);

impl StructuralDirectiveTransform for TransformIf {
    fn matches(&self, name: &String) -> bool {
        matches!(name.as_str(), "if" | "else" | "else-if")
    }
}

impl NodeTransformState for TransformIf {
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

            let mut node_removed = false;
            for dir in dirs {
                if dir.name == "if" {
                    let if_node = if let TemplateChildNode::Element(node) = &children[i] {
                        let branch = IfBranchNode::new(node, dir);
                        let if_node = IfNode {
                            branches: vec![branch],
                            codegen_node: None,
                            loc: node.loc().clone(),
                        };
                        if_node
                    } else {
                        unreachable!()
                    };

                    children[i] = TemplateChildNode::If(if_node);

                    self.0.push(TransformIfState { index: i });
                } else {
                    // locate the adjacent v-if
                    // let comments = Vec::new();
                    let mut j = i;
                    loop {
                        if j == 0 {
                            break;
                        }
                        j -= 1;

                        if matches!(children[j], TemplateChildNode::If(_)) {
                            // move the node to the if node's branches
                            let TemplateChildNode::Element(node) = children.remove(i) else {
                                unreachable!();
                            };
                            debug_assert!(!node_removed);
                            node_removed = true;
                            let branch = IfBranchNode::new(&node, dir.clone());

                            let mut branch = TemplateChildNode::IfBranch(branch);
                            let transform_node = TransformNode::TemplateChild(&mut branch);
                            // since the branch was removed, it will not be traversed.
                            // make sure to traverse here.
                            context.traverse_node(transform_node);

                            let TemplateChildNode::IfBranch(branch) = branch else {
                                unreachable!();
                            };
                            let TemplateChildNode::If(sibling) = &mut children[j] else {
                                unreachable!();
                            };
                            sibling.branches.push(branch);
                        } else {
                        }
                    }
                }
            }

            if !node_removed {
                i += 1;
            }
        }
    }

    fn pre_exit(&mut self, node: &mut TransformNode, context: &mut TransformContext) {
        let Some(children) = node.children_mut() else {
            unreachable!();
        };
        for (key, state) in self.0.drain(..).enumerate() {
            let TransformIfState { index } = state;
            let TemplateChildNode::If(if_node) = &mut children[index] else {
                unreachable!();
            };
            exit(context, if_node, key);
        }
    }
}

fn create_children_codegen_node(
    branch: IfBranchNode,
    key_index: usize,
    context: &mut TransformContext,
) -> JSChildNode {
    let key_property = Property::new(
        ExpressionNode::new_simple("key", Some(true), None, None),
        JSChildNode::Simple(SimpleExpressionNode::new(
            key_index.to_string(),
            Some(false),
            Some(SourceLocation::loc_stub()),
            Some(crate::ConstantTypes::CanCache),
        )),
    );
    let IfBranchNode { children, .. } = branch;
    let need_fragment_wrapper = children.len() != 1 || children[0].type_() != NodeTypes::Element;
    if need_fragment_wrapper {
        let patch_flag = PatchFlags::StableFragment;

        let tag = context.helper(Fragment.to_string());
        JSChildNode::VNodeCall(VNodeCall::new(
            Some(context),
            VNodeCallTag::Symbol(tag),
            Some(PropsExpression::Object(ObjectExpression::new(
                vec![key_property],
                None,
            ))),
            Some(VNodeCallChildren::TemplateChildNodeList(children)),
            Some(patch_flag),
            Some(true),
            Some(false),
            /* isComponent */
            Some(false),
            Some(branch.loc),
        ))
    } else {
        let mut ret = if let TemplateChildNode::Element(node) = &children[0] {
            match node {
                ElementNode::PlainElement(node) => match &node.codegen_node {
                    Some(PlainElementNodeCodegenNode::VNodeCall(node)) => {
                        JSChildNode::VNodeCall(node.clone())
                    }
                    _ => {
                        todo!()
                    }
                },
                ElementNode::Component(node) => match &node.codegen_node {
                    Some(ComponentNodeCodegenNode::VNodeCall(node)) => {
                        JSChildNode::VNodeCall(node.clone())
                    }
                    _ => {
                        todo!()
                    }
                },
                _ => {
                    todo!()
                }
            }
        } else {
            unreachable!();
        };

        if let JSChildNode::VNodeCall(node) = &mut ret {
            convert_to_block(node, context);

            inject_prop(node, key_property, context);
        }
        ret
    }
}
