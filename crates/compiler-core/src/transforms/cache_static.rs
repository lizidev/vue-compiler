use crate::{
    ast::{
        CompoundExpressionNode, CompoundExpressionNodeChild, ConstantTypes, ElementNode,
        ElementTypes, ExpressionNode, RootNode, TemplateChildNode,
    },
    transform::TransformContext,
};

pub fn get_single_element_root(root: &RootNode) -> Option<ElementNode> {
    let children = root
        .children
        .iter()
        .filter(|x| !matches!(x, TemplateChildNode::Comment(_)));
    if children.count() == 1 {
        let mut children = root
            .children
            .iter()
            .filter(|x| !matches!(x, TemplateChildNode::Comment(_)));
        if let Some(node) = children.next()
            && let TemplateChildNode::Element(node) = node
            && !matches!(node.tag_type(), ElementTypes::Slot)
        {
            return Some(node.clone());
        }
    }
    None
}

pub fn get_constant_type(
    node: &TemplateChildNode,
    _context: &mut TransformContext,
) -> ConstantTypes {
    match node {
        TemplateChildNode::Text(_) | TemplateChildNode::Comment(_) => ConstantTypes::CanStringify,
        TemplateChildNode::Interpolation(node) => match &node.content {
            ExpressionNode::Simple(node) => node.const_type,
            ExpressionNode::Compound(node) => get_constant_type_with_compound(node),
        },
        TemplateChildNode::Compound(node) => get_constant_type_with_compound(node),
        _ => {
            todo!()
        }
    }
}

pub fn get_constant_type_with_compound(node: &CompoundExpressionNode) -> ConstantTypes {
    let mut return_type = ConstantTypes::CanStringify;
    for child in &node.children {
        let child_type = match child {
            CompoundExpressionNodeChild::Simple(node) => node.const_type,
            CompoundExpressionNodeChild::Compound(node) => get_constant_type_with_compound(node),
            CompoundExpressionNodeChild::Interpolation(node) => match &node.content {
                ExpressionNode::Simple(node) => node.const_type,
                ExpressionNode::Compound(node) => get_constant_type_with_compound(node),
            },
            CompoundExpressionNodeChild::Text(_) => ConstantTypes::NotConstant,
            CompoundExpressionNodeChild::String(_) => {
                continue;
            }
        };
        if child_type == ConstantTypes::NotConstant {
            return ConstantTypes::NotConstant;
        } else if child_type < return_type {
            return_type = child_type;
        }
    }
    return_type
}
