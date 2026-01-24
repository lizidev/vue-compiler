use vue_compiler_shared::PatchFlags;

use crate::{
    ast::{
        CallArgument, CallExpression, ExpressionNode, IfBranchNode, IfCodegenNode,
        IfConditionalExpression, IfNode, JSChildNode, ObjectExpression, Property, PropsExpression,
        SimpleExpressionNode, SourceLocation, TemplateChildNode, VNodeCall, VNodeCallChildren,
    },
    runtime_helpers::{CreateComment, Fragment},
    transform::{NodeTransform, StructuralDirectiveTransform, TransformContext, TransformNode},
};

#[derive(Debug, Clone)]
struct TransformIfState {
    branch: IfBranchNode,
    is_root: bool,
    key: usize,
}

impl TransformIfState {
    fn exit(self, context: &mut TransformContext, if_node: &mut IfNode) {
        let TransformIfState {
            branch,
            is_root,
            key,
        } = self;
        if is_root {
            let Some(condition) = branch.condition.clone() else {
                unreachable!();
            };

            if_node.codegen_node = Some(IfCodegenNode::IfConditional(IfConditionalExpression {
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
            }));
        } else {
            // attach this branch's codegen node to the v-if root.
            let Some(node) = &mut if_node.codegen_node else {
                unreachable!();
            };

            let parent_condition = node.get_parent_condition();

            let key = key + if_node.branches.len() - 1;
            parent_condition.alternate = create_children_codegen_node(branch, key, context);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TransformIf(Vec<TransformIfState>);

impl StructuralDirectiveTransform for TransformIf {
    fn matches(&self, name: &String) -> bool {
        matches!(name.as_str(), "if" | "else" | "else-if")
    }
}

impl NodeTransform for TransformIf {
    fn transform(&mut self, node: &mut TemplateChildNode, context: &mut TransformContext) {
        let Some(dirs) = StructuralDirectiveTransform::transform(self, node) else {
            return;
        };

        let TemplateChildNode::Element(node) = node else {
            return;
        };
        self.0.clear();
        for dir in dirs {
            if dir.name == "if" {
                let branch = IfBranchNode::new(node, dir);
                let if_node = IfNode {
                    branches: vec![branch.clone()],
                    codegen_node: None,
                    loc: node.loc().clone(),
                };
                context.replace_node(TemplateChildNode::If(if_node));

                let on_exit = process_codegen(context, branch, true);
                self.0.push(on_exit);
            } else {
                // locate the adjacent v-if
                let Some(parent) = context.parent.clone() else {
                    unreachable!();
                };
                let siblings = parent.children_mut();
                // let comments = Vec::new();
                let mut i = context.child_index;
                loop {
                    if i == 0 {
                        break;
                    }
                    i -= 1;
                    let sibling = siblings.get_mut(i);
                    if let Some(sibling) = sibling
                        && let TemplateChildNode::If(sibling) = sibling
                    {
                        // move the node to the if node's branches
                        context.remove_node(None);
                        let branch = IfBranchNode::new(node, dir.clone());
                        let on_exit = process_codegen(context, branch.clone(), false);
                        // since the branch was removed, it will not be traversed.
                        // make sure to traverse here.
                        let mut branch = TemplateChildNode::IfBranch(branch);
                        context.traverse_node(&mut branch);
                        // call on exit
                        on_exit.exit(context, sibling);
                        let TemplateChildNode::IfBranch(branch) = branch else {
                            unreachable!();
                        };
                        sibling.branches.push(branch);
                        // make sure to reset currentNode after traversal to indicate this
                        // node has been removed.
                        context.current_node = None;
                    } else {
                    }
                }
            }
        }
    }

    fn exit(&mut self, context: &mut TransformContext) {
        for state in self.0.drain(..) {
            let if_node = {
                let Some(current_node) = context.current_node else {
                    unreachable!();
                };
                let if_node = unsafe { &mut *current_node };
                let TransformNode::TemplateChild(TemplateChildNode::If(if_node)) = if_node else {
                    unreachable!();
                };
                if_node
            };

            state.exit(context, if_node);
        }
    }

    fn clone_box(&self) -> Box<dyn NodeTransform> {
        Box::new(self.clone())
    }
}

fn process_codegen(
    context: &TransformContext,
    branch: IfBranchNode,
    is_root: bool,
) -> TransformIfState {
    // #1587: We need to dynamically increment the key based on the current
    // node's sibling nodes, since chained v-if/else branches are
    // rendered at the same depth
    let Some(parent) = &context.parent else {
        unreachable!();
    };

    let siblings = parent.children();
    let mut i = context.child_index;
    let mut key = 0;

    loop {
        if i == 0 {
            break;
        }
        i -= 1;
        if let Some(sibling) = siblings.get(i)
            && let TemplateChildNode::If(sibling) = sibling
        {
            key += sibling.branches.len();
        }
    }

    // Exit callback. Complete the codegenNode when all children have been
    // transformed.
    TransformIfState {
        branch,
        is_root,
        key,
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

    let patch_flag = PatchFlags::StableFragment;

    let tag = context.helper(Fragment.to_string());
    JSChildNode::VNodeCall(VNodeCall::new(
        Some(context),
        tag,
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
    // const firstChild = children[0]
    // const needFragmentWrapper =
    //   children.length !== 1 || firstChild.type !== NodeTypes.ELEMENT
    // if (needFragmentWrapper) {
    //   if (children.length === 1 && firstChild.type === NodeTypes.FOR) {
    //     // optimize away nested fragments when child is a ForNode
    //     const vnodeCall = firstChild.codegenNode!
    //     injectProp(vnodeCall, keyProperty, context)
    //     return vnodeCall
    //   } else {
    //     let patchFlag = PatchFlags.STABLE_FRAGMENT
    //     // check if the fragment actually contains a single valid child with
    //     // the rest being comments
    //     if (
    //       __DEV__ &&
    //       !branch.isTemplateIf &&
    //       children.filter(c => c.type !== NodeTypes.COMMENT).length === 1
    //     ) {
    //       patchFlag |= PatchFlags.DEV_ROOT_FRAGMENT
    //     }

    //     return createVNodeCall(
    //       context,
    //       helper(FRAGMENT),
    //       createObjectExpression([keyProperty]),
    //       children,
    //       patchFlag,
    //       undefined,
    //       undefined,
    //       true,
    //       false,
    //       false /* isComponent */,
    //       branch.loc,
    //     )
    //   }
    // } else {
    //   const ret = (firstChild as ElementNode).codegenNode as
    //     | BlockCodegenNode
    //     | MemoExpression
    //   const vnodeCall = getMemoedVNodeCall(ret)
    //   // Change createVNode to createBlock.
    //   if (vnodeCall.type === NodeTypes.VNODE_CALL) {
    //     convertToBlock(vnodeCall, context)
    //   }
    //   // inject branch key
    //   injectProp(vnodeCall, keyProperty, context)
    //   return ret
    // }
}
