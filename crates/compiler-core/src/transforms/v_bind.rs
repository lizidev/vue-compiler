use crate::{
    ast::{DirectiveNode, ElementNode, JSChildNode, Property},
    transform::{DirectiveTransform, DirectiveTransformResult, TransformContext},
};

#[derive(Debug, Clone)]
pub struct TransformBind;

impl DirectiveTransform for TransformBind {
    fn transform(
        &mut self,
        dir: &DirectiveNode,
        _node: &ElementNode,
        _context: &TransformContext,
    ) -> DirectiveTransformResult {
        let Some(arg) = dir.arg.clone() else {
            unreachable!();
        };
        let Some(exp) = dir.exp.clone() else {
            unreachable!();
        };

        DirectiveTransformResult {
            props: vec![Property::new(arg, JSChildNode::from(exp))],
        }
    }

    fn clone_box(&self) -> Box<dyn DirectiveTransform> {
        Box::new(self.clone())
    }
}
