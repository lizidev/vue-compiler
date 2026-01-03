#[cfg(test)]
mod text {
    use std::{cell::RefCell, sync::Arc};
    use vue_compiler_core::{
        CompilerError, ErrorCodes, ErrorHandlingOptions, ParserOptions, Position, SourceLocation,
        TemplateChildNode, base_parse,
    };

    #[test]
    fn simple_text() {
        let ast = base_parse("some text", None);
        let text = ast.children.first();

        assert_eq!(
            text,
            Some(&TemplateChildNode::new_text(
                "some text",
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 9,
                        line: 1,
                        column: 10,
                    },
                    source: "some text".to_string(),
                },
            ))
        );
    }

    #[test]
    fn simple_text_with_invalid_end_tag() {
        #[derive(Debug)]
        struct TestErrorHandlingOptions {
            errors: Arc<RefCell<Vec<CompilerError>>>,
        }

        impl ErrorHandlingOptions for TestErrorHandlingOptions {
            fn on_error(&mut self, error: CompilerError) {
                self.errors.borrow_mut().push(error);
            }
        }

        let errors: Arc<RefCell<Vec<CompilerError>>> = Default::default();
        let error_handling_options = TestErrorHandlingOptions {
            errors: errors.clone(),
        };

        let ast = base_parse(
            "some text</div>",
            Some(ParserOptions {
                error_handling_options: Box::new(error_handling_options),
                ..Default::default()
            }),
        );
        let text = ast.children.first();

        let errors = Arc::try_unwrap(errors).unwrap().into_inner();
        assert_eq!(
            errors,
            vec![CompilerError::new(
                ErrorCodes::XInvalidEndTag,
                Some(SourceLocation {
                    start: Position {
                        offset: 9,
                        line: 1,
                        column: 10,
                    },
                    end: Position {
                        offset: 9,
                        line: 1,
                        column: 10,
                    },
                    source: String::new(),
                })
            )]
        );

        assert_eq!(
            text,
            Some(&TemplateChildNode::new_text(
                "some text",
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 9,
                        line: 1,
                        column: 10,
                    },
                    source: "some text".to_string(),
                },
            ))
        );
    }

    #[test]
    fn text_with_interpolation() {
        let ast = base_parse("some {{ foo + bar }} text", None);
        let text1 = ast.children.get(0);
        let text2 = ast.children.get(2);

        assert_eq!(
            text1,
            Some(&TemplateChildNode::new_text(
                "some ",
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    source: "some ".to_string(),
                },
            ))
        );
        assert_eq!(
            text2,
            Some(&TemplateChildNode::new_text(
                " text",
                SourceLocation {
                    start: Position {
                        offset: 20,
                        line: 1,
                        column: 21,
                    },
                    end: Position {
                        offset: 25,
                        line: 1,
                        column: 26,
                    },
                    source: " text".to_string(),
                },
            ))
        );
    }

    /// text with interpolation which has `<`
    #[test]
    fn text_with_interpolation_which_has_less() {
        let ast = base_parse("some {{ a<b && c>d }} text", None);
        let text1 = ast.children.get(0);
        let text2 = ast.children.get(2);

        assert_eq!(
            text1,
            Some(&TemplateChildNode::new_text(
                "some ",
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    source: "some ".to_string(),
                },
            ))
        );
        assert_eq!(
            text2,
            Some(&TemplateChildNode::new_text(
                " text",
                SourceLocation {
                    start: Position {
                        offset: 21,
                        line: 1,
                        column: 22,
                    },
                    end: Position {
                        offset: 26,
                        line: 1,
                        column: 27,
                    },
                    source: " text".to_string(),
                },
            ))
        );
    }

    #[test]
    fn text_with_mix_of_tags_and_interpolations() {
        let ast = base_parse("some <span>{{ foo < bar + foo }} text</span>", None);
        let text1 = ast.children.get(0);
        let text2 = ast.children.get(1);

        assert_eq!(
            text1,
            Some(&TemplateChildNode::new_text(
                "some ",
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    source: "some ".to_string(),
                },
            ))
        );

        assert!(matches!(text2, Some(TemplateChildNode::Element(_))));
        let Some(TemplateChildNode::Element(el)) = text2 else {
            return;
        };
        let text2 = el.children().get(1);
        assert_eq!(
            text2,
            Some(&TemplateChildNode::new_text(
                " text",
                SourceLocation {
                    start: Position {
                        offset: 32,
                        line: 1,
                        column: 33,
                    },
                    end: Position {
                        offset: 37,
                        line: 1,
                        column: 38,
                    },
                    source: " text".to_string(),
                },
            ))
        );
    }

    /// lonely "<" doesn\'t separate nodes
    #[test]
    fn lonely_less_doesnt_separate_nodes() {
        let ast = base_parse("a < b", None);
        // onError: err => {
        //   if (err.code !== ErrorCodes.INVALID_FIRST_CHARACTER_OF_TAG_NAME) {
        //     throw err
        //   }
        // },
        let text = ast.children.get(0);

        assert_eq!(
            text,
            Some(&TemplateChildNode::new_text(
                "a < b",
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    source: "a < b".to_string(),
                },
            ))
        );
    }

    /// lonely "{{" doesn\'t separate nodes
    #[test]
    fn lonely_delimiter_open_doesnt_separate_nodes() {
        let ast = base_parse("a {{ b", None);
        // onError: err => {
        // if (error.code !== ErrorCodes.X_MISSING_INTERPOLATION_END) {
        //   throw error
        // }
        // },
        let text = ast.children.get(0);

        assert_eq!(
            text,
            Some(&TemplateChildNode::new_text(
                "a {{ b",
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 6,
                        line: 1,
                        column: 7,
                    },
                    source: "a {{ b".to_string(),
                },
            ))
        );
    }
}

#[cfg(test)]
mod interpolation {
    use vue_compiler_core::{
        ConstantTypes, ExpressionNode, Position, SourceLocation, TemplateChildNode, base_parse,
    };

    #[test]
    fn simple_interpolation() {
        let ast = base_parse("{{message}}", None);
        let interpolation = ast.children.first();

        assert_eq!(
            interpolation,
            Some(&TemplateChildNode::new_interpolation(
                ExpressionNode::new_simple(
                    "message".to_string(),
                    Some(false),
                    Some(SourceLocation {
                        start: Position {
                            offset: 2,
                            line: 1,
                            column: 3,
                        },
                        end: Position {
                            offset: 9,
                            line: 1,
                            column: 10,
                        },
                        source: "message".to_string(),
                    }),
                    Some(ConstantTypes::NotConstant)
                ),
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 11,
                        line: 1,
                        column: 12,
                    },
                    source: "{{message}}".to_string(),
                },
            ))
        );
    }
}

#[cfg(test)]
mod comment {
    use vue_compiler_core::{
        ParserOptions, Position, SourceLocation, TemplateChildNode, base_parse,
    };

    #[test]
    fn empty_comment() {
        let ast = base_parse(
            "<!---->",
            Some(ParserOptions {
                comments: Some(true),
                ..Default::default()
            }),
        );
        let comment = ast.children.first();

        assert_eq!(
            comment,
            Some(&TemplateChildNode::new_comment(
                "",
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 7,
                        line: 1,
                        column: 8,
                    },
                    source: "<!---->".to_string(),
                },
            ))
        );
    }

    #[test]
    fn simple_comment() {
        let ast = base_parse(
            "<!--abc-->",
            Some(ParserOptions {
                comments: Some(true),
                ..Default::default()
            }),
        );
        let comment = ast.children.first();

        assert_eq!(
            comment,
            Some(&TemplateChildNode::new_comment(
                "abc",
                SourceLocation {
                    start: Position {
                        offset: 0,
                        line: 1,
                        column: 1,
                    },
                    end: Position {
                        offset: 10,
                        line: 1,
                        column: 11,
                    },
                    source: "<!--abc-->".to_string(),
                },
            ))
        );
    }
}

#[cfg(test)]
mod element {
    use vue_compiler_core::{
        Attribute, AttributeNode, BaseElement, BaseElementProps, Directive, DirectiveNode,
        ElementNode, ElementTypes, Namespaces, NodeTypes, ParseMode, ParserOptions,
        PlainElementNode, Position, SourceLocation, TemplateChildNode, TextNode, base_parse,
    };

    #[test]
    fn simple_div() {
        let ast = base_parse("<div>hello</div>", None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 16,
                            line: 1,
                            column: 17,
                        },
                        source: "<div>hello</div>".to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "div".to_string(),
                        tag_type: ElementTypes::Element,
                        props: Vec::new(),
                        children: vec![TemplateChildNode::new_text(
                            "hello",
                            SourceLocation {
                                start: Position {
                                    offset: 5,
                                    line: 1,
                                    column: 6,
                                },
                                end: Position {
                                    offset: 10,
                                    line: 1,
                                    column: 11,
                                },
                                source: "hello".to_string(),
                            }
                        )],
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn empty() {
        let ast = base_parse("<div></div>", None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 11,
                            line: 1,
                            column: 12,
                        },
                        source: "<div></div>".to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "div".to_string(),
                        tag_type: ElementTypes::Element,
                        props: Vec::new(),
                        children: vec![],
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn self_closing() {
        let ast = base_parse("<div/>after", None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 6,
                            line: 1,
                            column: 7,
                        },
                        source: "<div/>".to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "div".to_string(),
                        tag_type: ElementTypes::Element,
                        props: Vec::new(),
                        children: vec![],
                        is_self_closing: Some(true),
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn void_element() {
        let ast = base_parse(
            "<img>after",
            Some(ParserOptions {
                is_void_tag: Box::new(|tag| tag == "img"),
                ..Default::default()
            }),
        );
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 5,
                            line: 1,
                            column: 6,
                        },
                        source: "<img>".to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "img".to_string(),
                        tag_type: ElementTypes::Element,
                        props: Vec::new(),
                        children: vec![],
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn self_closing_void_element() {
        let ast = base_parse(
            "<img/>after",
            Some(ParserOptions {
                is_void_tag: Box::new(|tag| tag == "img"),
                ..Default::default()
            }),
        );
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 6,
                            line: 1,
                            column: 7,
                        },
                        source: "<img/>".to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "img".to_string(),
                        tag_type: ElementTypes::Element,
                        props: Vec::new(),
                        children: vec![],
                        is_self_closing: Some(true),
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn template_element_with_directives() {
        let ast = base_parse(r#"<template v-if="ok"></template>"#, None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(ElementNode::PlainElement(el))) = element {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag_type, ElementTypes::Template);
        } else if let Some(TemplateChildNode::Element(ElementNode::Template(el))) = element {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag_type, ElementTypes::Template);
        }
    }

    #[test]
    fn template_element_without_directives() {
        let ast = base_parse("<template></template>", None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(ElementNode::PlainElement(el))) = element {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag_type, ElementTypes::Element);
        } else if let Some(TemplateChildNode::Element(ElementNode::Template(el))) = element {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag_type, ElementTypes::Element);
        }
    }

    #[test]
    fn native_element_with_is_native_tag() {
        let ast = base_parse(
            "<div></div><comp></comp><Comp></Comp>",
            Some(ParserOptions {
                is_native_tag: Some(Box::new(|tag| tag == "div")),
                ..Default::default()
            }),
        );

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[0] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type, ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "comp");
            assert_eq!(el.tag_type, ElementTypes::Component);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[2] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "Comp");
            assert_eq!(el.tag_type, ElementTypes::Component);
        }
    }

    #[test]
    fn is_casting_with_is_native_tag() {
        let ast = base_parse(
            r#"<div></div><div is="vue:foo"></div><Comp></Comp>"#,
            Some(ParserOptions {
                is_native_tag: Some(Box::new(|tag| tag == "div")),
                ..Default::default()
            }),
        );

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[0] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type, ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type, ElementTypes::Component);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[2] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "Comp");
            assert_eq!(el.tag_type, ElementTypes::Component);
        }
    }

    #[test]
    fn is_casting_without_is_native_tag() {
        let ast = base_parse(r#"<div></div><div is="vue:foo"></div><Comp></Comp>"#, None);

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[0] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type, ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type, ElementTypes::Component);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[2] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "Comp");
            assert_eq!(el.tag_type, ElementTypes::Component);
        }
    }

    #[test]
    fn custom_element() {
        let ast = base_parse(
            r#"<div></div><comp></comp>"#,
            Some(ParserOptions {
                is_native_tag: Some(Box::new(|tag| tag == "div")),
                is_custom_element: Some(Box::new(|tag| Some(tag == "comp"))),
                ..Default::default()
            }),
        );

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[0] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type, ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "comp");
            assert_eq!(el.tag_type, ElementTypes::Element);
        }
    }

    #[test]
    fn built_in_component() {
        let ast = base_parse(
            "<div></div><comp></comp>",
            Some(ParserOptions {
                is_built_in_component: Some(Box::new(
                    |tag| if tag == "comp" { Some(()) } else { None },
                )),
                ..Default::default()
            }),
        );

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[0] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type, ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "comp");
            assert_eq!(el.tag_type, ElementTypes::Component);
        }
    }

    #[test]
    fn slot_element() {
        let ast = base_parse("<slot></slot><Comp></Comp>", None);

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[0] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "slot");
            assert_eq!(el.tag_type, ElementTypes::Slot);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_, NodeTypes::Element);
            assert_eq!(el.tag, "Comp");
            assert_eq!(el.tag_type, ElementTypes::Component);
        }
    }

    #[test]
    fn attribute_with_no_value() {
        let ast = base_parse("<div id></div>", None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 14,
                            line: 1,
                            column: 15,
                        },
                        source: "<div id></div>".to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "div".to_string(),
                        tag_type: ElementTypes::Element,
                        props: vec![BaseElementProps::Attribute(AttributeNode {
                            type_: NodeTypes::Attribute,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 5,
                                    line: 1,
                                    column: 6,
                                },
                                end: Position {
                                    offset: 7,
                                    line: 1,
                                    column: 8,
                                },
                                source: "id".to_string(),
                            },
                            inner: Attribute {
                                name: "id".to_string(),
                                value: None,
                            }
                        })],
                        children: Vec::new(),
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn attribute_with_empty_value_double_quote() {
        let ast = base_parse(r#"<div id=""></div>"#, None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 17,
                            line: 1,
                            column: 18,
                        },
                        source: r#"<div id=""></div>"#.to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "div".to_string(),
                        tag_type: ElementTypes::Element,
                        props: vec![BaseElementProps::Attribute(AttributeNode {
                            type_: NodeTypes::Attribute,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 5,
                                    line: 1,
                                    column: 6,
                                },
                                end: Position {
                                    offset: 7,
                                    line: 1,
                                    column: 8,
                                },
                                source: r#"id="""#.to_string(),
                            },
                            inner: Attribute {
                                name: "id".to_string(),
                                value: Some(TextNode::new(
                                    "",
                                    SourceLocation {
                                        start: Position {
                                            offset: 8,
                                            line: 1,
                                            column: 9,
                                        },
                                        end: Position {
                                            offset: 10,
                                            line: 1,
                                            column: 11,
                                        },
                                        source: r#""""#.to_string(),
                                    }
                                )),
                            }
                        })],
                        children: Vec::new(),
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn attribute_with_empty_value_single_quote() {
        let ast = base_parse("<div id=''></div>", None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 17,
                            line: 1,
                            column: 18,
                        },
                        source: "<div id=''></div>".to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "div".to_string(),
                        tag_type: ElementTypes::Element,
                        props: vec![BaseElementProps::Attribute(AttributeNode {
                            type_: NodeTypes::Attribute,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 5,
                                    line: 1,
                                    column: 6,
                                },
                                end: Position {
                                    offset: 7,
                                    line: 1,
                                    column: 8,
                                },
                                source: "id=''".to_string(),
                            },
                            inner: Attribute {
                                name: "id".to_string(),
                                value: Some(TextNode::new(
                                    "",
                                    SourceLocation {
                                        start: Position {
                                            offset: 8,
                                            line: 1,
                                            column: 9,
                                        },
                                        end: Position {
                                            offset: 10,
                                            line: 1,
                                            column: 11,
                                        },
                                        source: "''".to_string(),
                                    }
                                )),
                            }
                        })],
                        children: Vec::new(),
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn attribute_with_value_double_quote() {
        let ast = base_parse(r#"<div id=">'"></div>"#, None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 19,
                            line: 1,
                            column: 20,
                        },
                        source: r#"<div id=">'"></div>"#.to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "div".to_string(),
                        tag_type: ElementTypes::Element,
                        props: vec![BaseElementProps::Attribute(AttributeNode {
                            type_: NodeTypes::Attribute,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 5,
                                    line: 1,
                                    column: 6,
                                },
                                end: Position {
                                    offset: 12,
                                    line: 1,
                                    column: 13,
                                },
                                source: r#"id=">'""#.to_string(),
                            },
                            inner: Attribute {
                                name: "id".to_string(),
                                value: Some(TextNode::new(
                                    ">'",
                                    SourceLocation {
                                        start: Position {
                                            offset: 8,
                                            line: 1,
                                            column: 9,
                                        },
                                        end: Position {
                                            offset: 12,
                                            line: 1,
                                            column: 13,
                                        },
                                        source: r#"">'""#.to_string(),
                                    }
                                )),
                            }
                        })],
                        children: Vec::new(),
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn attribute_with_value_single_quote() {
        let ast = base_parse("<div id='>\"'></div>", None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 19,
                            line: 1,
                            column: 20,
                        },
                        source: "<div id='>\"'></div>".to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "div".to_string(),
                        tag_type: ElementTypes::Element,
                        props: vec![BaseElementProps::Attribute(AttributeNode {
                            type_: NodeTypes::Attribute,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 5,
                                    line: 1,
                                    column: 6,
                                },
                                end: Position {
                                    offset: 12,
                                    line: 1,
                                    column: 13,
                                },
                                source: "id='>\"'".to_string(),
                            },
                            inner: Attribute {
                                name: "id".to_string(),
                                value: Some(TextNode::new(
                                    ">\"",
                                    SourceLocation {
                                        start: Position {
                                            offset: 8,
                                            line: 1,
                                            column: 9,
                                        },
                                        end: Position {
                                            offset: 12,
                                            line: 1,
                                            column: 13,
                                        },
                                        source: "'>\"'".to_string(),
                                    }
                                )),
                            }
                        })],
                        children: Vec::new(),
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn attribute_with_value_unquoted() {
        let ast = base_parse("<div id=a/></div>", None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 17,
                            line: 1,
                            column: 18,
                        },
                        source: "<div id=a/></div>".to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "div".to_string(),
                        tag_type: ElementTypes::Element,
                        props: vec![BaseElementProps::Attribute(AttributeNode {
                            type_: NodeTypes::Attribute,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 5,
                                    line: 1,
                                    column: 6,
                                },
                                end: Position {
                                    offset: 10,
                                    line: 1,
                                    column: 11,
                                },
                                source: "id=a/".to_string(),
                            },
                            inner: Attribute {
                                name: "id".to_string(),
                                value: Some(TextNode::new(
                                    "a/",
                                    SourceLocation {
                                        start: Position {
                                            offset: 8,
                                            line: 1,
                                            column: 9,
                                        },
                                        end: Position {
                                            offset: 10,
                                            line: 1,
                                            column: 11,
                                        },
                                        source: "a/".to_string(),
                                    }
                                )),
                            }
                        })],
                        children: Vec::new(),
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn attribute_value_with_greater() {
        let ast = base_parse(
            r#"<script setup lang="ts" generic="T extends Record<string,string>"></script>"#,
            Some(ParserOptions {
                parse_mode: ParseMode::SFC,
                ..Default::default()
            }),
        );
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    type_: NodeTypes::Element,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 75,
                            line: 1,
                            column: 76,
                        },
                        source: r#"<script setup lang="ts" generic="T extends Record<string,string>"></script>"#.to_string(),
                    },
                    inner: BaseElement {
                        ns: Namespaces::HTML as u32,
                        tag: "script".to_string(),
                        tag_type: ElementTypes::Element,
                        props: vec![BaseElementProps::Attribute(AttributeNode {
                            type_: NodeTypes::Attribute,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 8,
                                    line: 1,
                                    column: 9,
                                },
                                end: Position {
                                    offset: 13,
                                    line: 1,
                                    column: 14,
                                },
                                source: "setup".to_string(),
                            },
                            inner: Attribute {
                                name: "setup".to_string(),
                                value: None,
                            }
                        }),BaseElementProps::Attribute(AttributeNode {
                            type_: NodeTypes::Attribute,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 14,
                                    line: 1,
                                    column: 15,
                                },
                                end: Position {
                                    offset: 23,
                                    line: 1,
                                    column: 24,
                                },
                                source: r#"lang="ts""#.to_string(),
                            },
                            inner: Attribute {
                                name: "lang".to_string(),
                                value: Some(TextNode::new(
                                    "ts",
                                    SourceLocation {
                                        start: Position {
                                            offset: 19,
                                            line: 1,
                                            column: 20,
                                        },
                                        end: Position {
                                            offset: 23,
                                            line: 1,
                                            column: 24,
                                        },
                                        source: r#""ts""#.to_string(),
                                    }
                                )),
                            }
                        }),BaseElementProps::Attribute(AttributeNode {
                            type_: NodeTypes::Attribute,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 24,
                                    line: 1,
                                    column: 25,
                                },
                                end: Position {
                                    offset: 65,
                                    line: 1,
                                    column: 66,
                                },
                                source: r#"generic="T extends Record<string,string>""#.to_string(),
                            },
                            inner: Attribute {
                                name: "generic".to_string(),
                                value: Some(TextNode::new(
                                    "T extends Record<string,string>",
                                    SourceLocation {
                                        start: Position {
                                            offset: 32,
                                            line: 1,
                                            column: 33,
                                        },
                                        end: Position {
                                            offset: 65,
                                            line: 1,
                                            column: 66,
                                        },
                                        source: r#""T extends Record<string,string>""#.to_string(),
                                    }
                                )),
                            }
                        })],
                        children: Vec::new(),
                        is_self_closing: None,
                        codegen_node: None,
                        ssr_codegen_node: None,
                    }
                }
            )))
        );
    }

    #[test]
    fn multiple_attributes() {
        let ast = base_parse(r#"<div id=a class="c" inert style=''></div>"#, None);

        let element = ast.children.first();

        // TODO
    }

    #[test]
    fn directive_with_no_value() {
        let ast = base_parse("<div v-if/>", None);

        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(el)) = element {
            let directive = &el.props()[0];
            assert_eq!(
                directive,
                &BaseElementProps::Directive(DirectiveNode {
                    type_: NodeTypes::Directive,
                    loc: SourceLocation {
                        start: Position {
                            offset: 5,
                            line: 1,
                            column: 6,
                        },
                        end: Position {
                            offset: 9,
                            line: 1,
                            column: 10
                        },
                        source: "v-if".to_string()
                    },
                    inner: Directive {
                        name: "if".to_string(),
                        raw_name: Some("v-if".to_string()),
                        exp: None,
                        arg: None,
                    }
                })
            );
        }
    }

    #[test]
    fn v_pre() {
        let ast = base_parse(
            r#"<div v-pre :id="foo"><Comp/>{{ bar }}</div>\n<div :id="foo"><Comp/>{{ bar }}</div>"#,
            None,
        );

        let div_with_pre = ast.children.first();

        assert!(matches!(
            div_with_pre,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(_)))
        ));
        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[0] {
            assert_eq!(
                el.props,
                vec![BaseElementProps::Attribute(AttributeNode {
                    type_: NodeTypes::Attribute,
                    loc: SourceLocation {
                        start: Position {
                            offset: 11,
                            line: 1,
                            column: 12,
                        },
                        end: Position {
                            offset: 20,
                            line: 1,
                            column: 21,
                        },
                        source: ":id=\"foo\"".to_string(),
                    },
                    inner: Attribute {
                        name: ":id".to_string(),
                        value: Some(TextNode::new(
                            "foo",
                            SourceLocation {
                                start: Position {
                                    offset: 15,
                                    line: 1,
                                    column: 16,
                                },
                                end: Position {
                                    offset: 20,
                                    line: 1,
                                    column: 21,
                                },
                                source: "\"foo\"".to_string(),
                            },
                        )),
                    }
                })]
            );
        }

        //TODO
    }

    #[test]
    fn should_not_condense_whitespaces_in_rc_data_text_mode() {
        let ast = base_parse(
            "<textarea>Text:\n   foo</textarea>",
            Some(ParserOptions {
                parse_mode: ParseMode::HTML,
                ..Default::default()
            }),
        );

        let pre_element = ast.children.first();

        assert!(matches!(pre_element, Some(TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(el)) = pre_element {
            assert!(el.children().len() == 1);

            let text = el.children().first();

            assert!(matches!(text, Some(TemplateChildNode::Text(_))));
            if let Some(TemplateChildNode::Text(text)) = text {
                assert_eq!(text.content, "Text:\n   foo");
            }
        }
    }
}
