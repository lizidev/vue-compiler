use crate::{
    ast::TemplateChildNode,
    transform::{NodeTransform, TransformContext},
};

#[derive(Debug, Clone)]
pub struct TransformVBindShorthand;

impl NodeTransform for TransformVBindShorthand {
    fn transform(&mut self, node: &mut TemplateChildNode, context: &mut TransformContext) {
        let TemplateChildNode::Element(node) = node else {
            return;
        };

        let _ = node;
        let _ = context;
        // for prop in node.props() {}
    }

    fn clone_box(&self) -> Box<dyn NodeTransform> {
        Box::new(self.clone())
    }
}
