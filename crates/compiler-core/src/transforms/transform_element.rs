use crate::{
    ast::{
        BaseElementProps, CallArgument, CallCallee, CallExpression, DirectiveNode, ElementNode,
        ElementTypes, ExpressionNode, JSChildNode, NodeTypes, ObjectExpression,
        PlainElementNodeCodegenNode, Property, SimpleExpressionNode, TemplateChildNode,
        TemplateTextChildNode, VNodeCall, VNodeCallChildren,
    },
    runtime_helpers::NormalizeClass,
    transform::{DirectiveTransformResult, NodeTransform, TransformContext, TransformNode},
};
use vue_compiler_shared::PatchFlags;

/// generate a JavaScript AST for this element's codegen
#[derive(Debug, Clone)]
pub struct TransformElement;

impl NodeTransform for TransformElement {
    fn exit(&mut self, context: &mut TransformContext) {
        post_transform_element(context);
    }

    fn clone_box(&self) -> Box<dyn NodeTransform> {
        Box::new(self.clone())
    }
}

/// perform the work on exit, after all child expressions have been
/// processed and merged.
fn post_transform_element(context: &mut TransformContext) {
    let Some(current_node) = context.current_node else {
        unreachable!();
    };

    let node = unsafe { &mut *current_node };
    let TransformNode::TemplateChild(TemplateChildNode::Element(node)) = node else {
        return;
    };

    if !matches!(
        node.tag_type(),
        ElementTypes::Element | ElementTypes::Component
    ) {
        return;
    }

    let is_component = matches!(node.tag_type(), ElementTypes::Component);

    let mut vnode_props = None::<PropsExpression>;
    let mut vnode_children = None::<VNodeCallChildren>;
    let mut patch_flag = None::<PatchFlags>;

    let mut should_use_block = !is_component &&
        // <svg> and <foreignObject> must be forced into blocks so that block
        // updates inside get proper isSVG flag at runtime. (#639, #643)
        // This is technically web-specific, but splitting the logic out of core
        // leads to too much unnecessary complexity.
        (node.tag() == "svg" || node.tag() == "foreignObject" || node.tag() == "math");

    if node.props().len() > 0 {
        let props_build_result =
            build_props(node, context, node.props(), is_component, false, false);

        vnode_props = props_build_result.props;
        patch_flag = props_build_result.patch_flag;

        if props_build_result.should_use_block {
            should_use_block = true;
        }
    }

    // children
    if node.children().len() > 0 {
        if node.children().iter().len() == 1 {
            let Some(child) = node.children().first() else {
                unreachable!();
            };
            let has_dynamic_text_child = matches!(
                child.type_(),
                NodeTypes::Interpolation | NodeTypes::CompoundExpression
            );
            // pass directly if the only child is a text node
            // (plain / interpolation / expression)
            if has_dynamic_text_child || child.type_() == NodeTypes::Text {
                vnode_children = Some(VNodeCallChildren::TemplateTextChildNode(
                    TemplateTextChildNode::from(child.clone()),
                ));
            } else {
                vnode_children = Some(VNodeCallChildren::TemplateChildNodeList(
                    node.children().clone(),
                ));
            }
        } else {
            vnode_children = Some(VNodeCallChildren::TemplateChildNodeList(
                node.children().clone(),
            ));
        }
    }

    if let ElementNode::PlainElement(node) = node {
        let vnode_call = VNodeCall::new(
            Some(context),
            format!("\"{}\"", node.tag),
            vnode_props,
            vnode_children,
            patch_flag,
            Some(should_use_block),
            Some(false),
            Some(false),
            Some(node.loc.clone()),
        );
        node.codegen_node = Some(PlainElementNodeCodegenNode::VNodeCall(vnode_call));
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum PropsExpression {
    Object(ObjectExpression),
    // ObjectExpression | CallExpression | ExpressionNode
}

struct PropsBuildResult {
    props: Option<PropsExpression>,
    directives: Vec<DirectiveNode>,
    patch_flag: Option<PatchFlags>,
    should_use_block: bool,
}

fn build_props<'a>(
    node: &'a ElementNode,
    context: &mut TransformContext,
    props: &'a Vec<BaseElementProps>,
    is_component: bool,
    is_dynamic_component: bool,
    ssr: bool,
) -> PropsBuildResult {
    let mut properties: Vec<Property> = Vec::new();
    let mut runtime_directives: Vec<DirectiveNode> = Vec::new();
    let mut should_use_block = false;

    let mut patch_flag = None::<PatchFlags>;
    let mut has_class_binding = false;
    let mut has_dynamic_keys = false;

    for prop in props {
        match prop {
            BaseElementProps::Attribute(prop) => {
                let is_static = Some(true);

                let (value, loc) = if let Some(node) = &prop.value {
                    (node.content.clone(), node.loc.clone())
                } else {
                    (String::new(), prop.loc.clone())
                };
                properties.push(Property::new(
                    ExpressionNode::new_simple(
                        &prop.name,
                        Some(true),
                        Some(prop.name_loc.clone()),
                        None,
                    ),
                    JSChildNode::Simple(SimpleExpressionNode::new(
                        value,
                        is_static,
                        Some(loc),
                        None,
                    )),
                ));
            }
            BaseElementProps::Directive(prop) => {
                let is_v_on = prop.name == "on";
                let directive_transform = context.directive_transforms.get(&prop.name).cloned();
                if let Some(mut directive_transform) = directive_transform {
                    let DirectiveTransformResult { props } =
                        directive_transform.transform(prop, node, context);

                    if !context.ssr {
                        props.iter().for_each(|prop| {
                            if let ExpressionNode::Simple(key) = &prop.key
                                && key.is_static
                            {
                                let name = &key.content;
                                if name == "class" {
                                    has_class_binding = true;
                                }
                            } else {
                                has_dynamic_keys = true;
                            }
                        });
                    }

                    if is_v_on {
                    } else {
                        properties.extend(props);
                    }
                }
            }
        }
    }

    let mut props_expression = None::<PropsExpression>;
    if properties.len() > 0 {
        props_expression = Some(PropsExpression::Object(ObjectExpression::new(
            properties,
            Some(node.loc().clone()),
        )));
    }

    // patchFlag analysis
    if has_dynamic_keys {
    } else {
        if has_class_binding && !is_component {
            patch_flag = Some(patch_flag.map_or(PatchFlags::Class, |f| f | PatchFlags::Class));
        }
    }

    if !context.in_ssr
        && let Some(props_expression) = &mut props_expression
    {
        if let PropsExpression::Object(props_expression) = props_expression {
            // means that there is no v-bind,
            // but still need to deal with dynamic key binding
            let mut class_key_index = None;
            let mut style_key_index = None;
            let mut has_dynamic_key = false;

            for (i, p) in props_expression.properties.iter().enumerate() {
                let key = &p.key;
                if let ExpressionNode::Simple(key) = key
                    && key.is_static
                {
                    if key.content == "class" {
                        class_key_index = Some(i);
                    } else if key.content == "style" {
                        style_key_index = Some(i);
                    }
                } else if !key.is_handler_key().unwrap_or_default() {
                    has_dynamic_key = true;
                }
            }

            if !has_dynamic_key {
                if let Some(i) = class_key_index
                    && let Some(class_prop) = props_expression.properties.get_mut(i)
                    && !class_prop.value.is_static_exp()
                {
                    let callee = context.helper(NormalizeClass.to_string());
                    class_prop.value = JSChildNode::Call(CallExpression::new(
                        CallCallee::Symbol(callee),
                        Some(vec![CallArgument::JSChild(class_prop.value.clone())]),
                        None,
                    ))
                }
                if let Some(i) = style_key_index
                    && let Some(style_prop) = props_expression.properties.get_mut(i)
                {
                }
            } else {
            }
        }
    }

    PropsBuildResult {
        props: props_expression,
        directives: runtime_directives,
        patch_flag,
        should_use_block,
    }
}
