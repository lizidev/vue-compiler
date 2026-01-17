#[cfg(test)]
mod text {
    use vue_compiler_core::{Position, SourceLocation, TemplateChildNode};
    use vue_compiler_dom::{parse, parser_options};

    #[test]
    fn textarea_handles_comments_elements_as_just_text() {
        let ast = parse(
            "<textarea>some<div>text</div>and<!--comment--></textarea>",
            Some(parser_options()),
        );
        let element = ast.children.first();
        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(element)) = element {
            let text = element.children().first();
            assert!(element.children().len() == 1);
            assert_eq!(
                text,
                Some(&TemplateChildNode::new_text(
                    "some<div>text</div>and<!--comment-->",
                    SourceLocation {
                        start: Position {
                            offset: 10,
                            line: 1,
                            column: 11,
                        },
                        end: Position {
                            offset: 46,
                            line: 1,
                            column: 47,
                        },
                        source: "some<div>text</div>and<!--comment-->".to_string(),
                    },
                ))
            );
        }
    }
}
