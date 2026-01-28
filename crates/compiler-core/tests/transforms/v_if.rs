#[cfg(test)]
mod compiler_v_if {
    use vue_compiler_core::{
        CompilerOptions, IfNode, RootNode, TemplateChildNode, TransformElement, TransformIf,
        TransformOptions, TransformVBindShorthand, base_parse as parse, transform,
    };

    struct IfTransformResult {
        root: RootNode,
        node: IfNode,
    }

    fn parse_with_if_transform(
        template: &str,
        options: Option<CompilerOptions>,
        return_index: Option<usize>,
    ) -> IfTransformResult {
        let return_index = return_index.unwrap_or_default();
        let (parser_options, mut transform_options, _) = options.unwrap_or_default().into();
        let mut ast = parse(template, Some(parser_options));

        transform_options.node_transforms = Some(vec![
            Box::new(TransformVBindShorthand),
            Box::new(TransformIf::default()),
            Box::new(TransformElement),
        ]);
        transform(&mut ast, transform_options);
        // if (!options.onError) {
        //   expect(ast.children.length).toBe(childrenLen)
        //   for (let i = 0; i < childrenLen; i++) {
        //     expect(ast.children[i].type).toBe(NodeTypes.IF)
        //   }
        // }
        let node = ast.children[return_index].clone();
        assert!(matches!(node, TemplateChildNode::If(_)));
        let TemplateChildNode::If(node) = node else {
            unreachable!();
        };
        IfTransformResult {
            root: ast,
            node: node,
        }
    }
    mod transform {
        use vue_compiler_core::{ElementTypes, ExpressionNode, TemplateChildNode};

        use super::{IfTransformResult, parse_with_if_transform};

        #[test]
        fn basic_v_if() {
            let IfTransformResult { node, .. } =
                parse_with_if_transform("<div v-if=\"ok\"/>", None, None);
            assert!(node.branches.len() == 1);
            let branche = &node.branches[0];
            assert!(matches!(
                &branche.condition,
                Some(ExpressionNode::Simple(condition))
                if condition.content ==  "ok"
            ));

            assert!(branche.children.len() == 1);
            let node = &branche.children[0];
            assert!(matches!(
                &node,
                TemplateChildNode::Element(node)
                if node.tag() == "div"
            ));
        }

        #[test]
        fn template_v_if() {
            let IfTransformResult { node, .. } = parse_with_if_transform(
                "<template v-if=\"ok\"><div/>hello<p/></template>",
                None,
                None,
            );
            assert!(node.branches.len() == 1);
            let branche = &node.branches[0];
            assert!(matches!(
                &branche.condition,
                Some(ExpressionNode::Simple(condition))
                if condition.content ==  "ok"
            ));

            assert!(branche.children.len() == 3);
            let node = &branche.children[0];
            assert!(matches!(
                &node,
                TemplateChildNode::Element(node)
                if node.tag() == "div"
            ));
            let node = &branche.children[1];
            assert!(matches!(
                &node,
                TemplateChildNode::Text(node)
                if node.content == "hello"
            ));
            let node = &branche.children[2];
            assert!(matches!(
                &node,
                TemplateChildNode::Element(node)
                if node.tag() == "p"
            ));
        }

        #[test]
        fn component_v_if() {
            let IfTransformResult { node, .. } =
                parse_with_if_transform("<Component v-if=\"ok\"></Component>", None, None);
            assert!(node.branches.len() == 1);
            let branche = &node.branches[0];
            assert!(matches!(
                &branche.condition,
                Some(ExpressionNode::Simple(condition))
                if condition.content ==  "ok"
            ));

            assert!(branche.children.len() == 1);
            let node = &branche.children[0];
            assert!(matches!(
                &node,
                TemplateChildNode::Element(node)
                if node.tag() == "Component"
            ));
            let TemplateChildNode::Element(node) = node else {
                unreachable!();
            };
            assert_eq!(node.tag_type(), ElementTypes::Component);
            // #2058 since a component may fail to resolve and fallback to a plain
            // element, it still needs to be made a block
            // assert!(node)
        }
    }
}
