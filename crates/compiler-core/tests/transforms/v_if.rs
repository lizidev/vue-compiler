#[cfg(test)]
mod compiler_v_if {
    use vue_compiler_core::{
        CompilerOptions, IfNode, RootNode, TemplateChildNode, base_parse as parse, transform,
        transform_element, transform_if,
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
            // Box::new(TransformVBindShorthand),
            transform_if,
            transform_element,
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
        use super::{IfTransformResult, parse_with_if_transform};
        use vue_compiler_core::{ElementTypes, ExpressionNode, TemplateChildNode};

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
            // let ElementNode::Component(node) = node else {
            //     unreachable!();
            // };
            // assert!(matches!(
            //     &node.codegen_node,
            //     Some(ComponentNodeCodegenNode::VNodeCall(node))
            //     if node.is_block
            // ))
        }

        /// v-if + v-else
        #[test]
        fn v_if_v_else() {
            let IfTransformResult { node, .. } =
                parse_with_if_transform("<div v-if=\"ok\"/><p v-else/>", None, None);
            assert!(node.branches.len() == 2);

            let branche = &node.branches[0];
            assert!(matches!(
                &branche.condition,
                Some(ExpressionNode::Simple(condition))
                if condition.content ==  "ok"
            ));
            assert!(branche.children.len() == 1);
            assert!(matches!(
                &branche.children[0],
                TemplateChildNode::Element(node)
                if node.tag() == "div"
            ));

            let branche = &node.branches[1];
            assert!(matches!(&branche.condition, None));
            assert!(branche.children.len() == 1);
            assert!(matches!(
                &branche.children[0],
                TemplateChildNode::Element(node)
                if node.tag() == "p"
            ));
        }

        /// v-if + v-else-if
        #[test]
        fn v_if_v_else_if() {
            let IfTransformResult { node, .. } =
                parse_with_if_transform("<div v-if=\"ok\"/><p v-else-if=\"orNot\"/>", None, None);
            assert!(node.branches.len() == 2);

            let branche = &node.branches[0];
            assert!(matches!(
                &branche.condition,
                Some(ExpressionNode::Simple(condition))
                if condition.content ==  "ok"
            ));
            assert!(branche.children.len() == 1);
            assert!(matches!(
                &branche.children[0],
                TemplateChildNode::Element(node)
                if node.tag() == "div"
            ));

            let branche = &node.branches[1];
            assert!(matches!(
                &branche.condition,
                Some(ExpressionNode::Simple(condition))
                if condition.content ==  "orNot"
            ));
            assert!(branche.children.len() == 1);
            assert!(matches!(
                &branche.children[0],
                TemplateChildNode::Element(node)
                if node.tag() == "p"
            ));
        }

        /// v-if + v-else-if + v-else
        #[test]
        fn v_if_v_else_if_v_else() {
            let IfTransformResult { node, .. } = parse_with_if_transform(
                "<div v-if=\"ok\"/><p v-else-if=\"orNot\"/><template v-else>fine</template>",
                None,
                None,
            );
            assert!(node.branches.len() == 3);

            let branche = &node.branches[0];
            assert!(matches!(
                &branche.condition,
                Some(ExpressionNode::Simple(condition))
                if condition.content ==  "ok"
            ));
            assert!(branche.children.len() == 1);
            assert!(matches!(
                &branche.children[0],
                TemplateChildNode::Element(node)
                if node.tag() == "div"
            ));

            let branche = &node.branches[1];
            assert!(matches!(
                &branche.condition,
                Some(ExpressionNode::Simple(condition))
                if condition.content ==  "orNot"
            ));
            assert!(branche.children.len() == 1);
            assert!(matches!(
                &branche.children[0],
                TemplateChildNode::Element(node)
                if node.tag() == "p"
            ));

            let branche = &node.branches[2];
            assert!(matches!(&branche.condition, None));
            assert!(branche.children.len() == 1);
            assert!(matches!(
                &branche.children[0],
                TemplateChildNode::Text(node)
                if node.content == "fine"
            ));
        }
    }

    mod codegen {
        use super::{IfTransformResult, parse_with_if_transform};
        use insta::assert_snapshot;
        use vue_compiler_core::generate;

        #[test]
        fn basic_v_if() {
            let IfTransformResult { root, .. } =
                parse_with_if_transform("<div v-if=\"ok\"/>", None, None);
            assert_snapshot!(generate(root, Default::default()).code);
        }
    }
}
