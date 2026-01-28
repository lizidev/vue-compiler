use std::{cell::RefCell, sync::Arc};
use vue_compiler_core::{CompilerError, ErrorHandlingOptions};

#[derive(Debug, Clone)]
struct TestErrorHandlingOptions {
    errors: Arc<RefCell<Vec<CompilerError>>>,
}

impl TestErrorHandlingOptions {
    fn new() -> Self {
        Self {
            errors: Default::default(),
        }
    }

    fn try_unwrap(self) -> Vec<CompilerError> {
        Arc::try_unwrap(self.errors).unwrap().into_inner()
    }
}

impl ErrorHandlingOptions for TestErrorHandlingOptions {
    fn on_error(&mut self, error: CompilerError) {
        self.errors.borrow_mut().push(error);
    }
}

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
    use super::TestErrorHandlingOptions;
    use vue_compiler_core::{
        AttributeNode, BaseElementProps, CompilerError, ConstantTypes, DirectiveNode, ElementNode,
        ElementTypes, ErrorCodes, ExpressionNode, Namespaces, NodeTypes, ParseMode, ParserOptions,
        PlainElementNode, Position, SimpleExpressionNode, SourceLocation, TemplateChildNode,
        TextNode, base_parse,
    };

    #[test]
    fn simple_div() {
        let ast = base_parse("<div>hello</div>", None);
        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
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
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
                    props: Vec::new(),
                    children: vec![],
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
                    props: Vec::new(),
                    children: vec![],
                    is_self_closing: Some(true),
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                    ns: Namespaces::HTML as u32,
                    tag: "img".to_string(),
                    props: Vec::new(),
                    children: vec![],
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                    ns: Namespaces::HTML as u32,
                    tag: "img".to_string(),
                    props: Vec::new(),
                    children: vec![],
                    is_self_closing: Some(true),
                    codegen_node: None,
                    ssr_codegen_node: None,
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
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag_type(), ElementTypes::Template);
        } else if let Some(TemplateChildNode::Element(ElementNode::Template(el))) = element {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag_type(), ElementTypes::Template);
        }
    }

    #[test]
    fn template_element_without_directives() {
        let ast = base_parse("<template></template>", None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(ElementNode::PlainElement(el))) = element {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag_type(), ElementTypes::Element);
        } else if let Some(TemplateChildNode::Element(ElementNode::Template(el))) = element {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag_type(), ElementTypes::Element);
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
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type(), ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "comp");
            assert_eq!(el.tag_type(), ElementTypes::Component);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[2] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "Comp");
            assert_eq!(el.tag_type(), ElementTypes::Component);
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
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type(), ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type(), ElementTypes::Component);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[2] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "Comp");
            assert_eq!(el.tag_type(), ElementTypes::Component);
        }
    }

    #[test]
    fn is_casting_without_is_native_tag() {
        let ast = base_parse(r#"<div></div><div is="vue:foo"></div><Comp></Comp>"#, None);

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[0] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type(), ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type(), ElementTypes::Component);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[2] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "Comp");
            assert_eq!(el.tag_type(), ElementTypes::Component);
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
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type(), ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "comp");
            assert_eq!(el.tag_type(), ElementTypes::Element);
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
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "div");
            assert_eq!(el.tag_type(), ElementTypes::Element);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "comp");
            assert_eq!(el.tag_type(), ElementTypes::Component);
        }
    }

    #[test]
    fn slot_element() {
        let ast = base_parse("<slot></slot><Comp></Comp>", None);

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[0] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "slot");
            assert_eq!(el.tag_type(), ElementTypes::Slot);
        }

        if let TemplateChildNode::Element(ElementNode::PlainElement(el)) = &ast.children[1] {
            assert_eq!(el.type_(), NodeTypes::Element);
            assert_eq!(el.tag, "Comp");
            assert_eq!(el.tag_type(), ElementTypes::Component);
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
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
                    props: vec![BaseElementProps::Attribute(AttributeNode {
                        name: "id".to_string(),
                        name_loc: SourceLocation {
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
                        value: None,
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
                    })],
                    children: Vec::new(),
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
                    props: vec![BaseElementProps::Attribute(AttributeNode {
                        name: "id".to_string(),
                        name_loc: SourceLocation {
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
                            source: r#"id="""#.to_string(),
                        },
                    })],
                    children: Vec::new(),
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
                    props: vec![BaseElementProps::Attribute(AttributeNode {
                        name: "id".to_string(),
                        name_loc: SourceLocation {
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
                            source: "id=''".to_string(),
                        },
                    })],
                    children: Vec::new(),
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
                    props: vec![BaseElementProps::Attribute(AttributeNode {
                        name: "id".to_string(),
                        name_loc: SourceLocation {
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
                    })],
                    children: Vec::new(),
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
                    props: vec![BaseElementProps::Attribute(AttributeNode {
                        name: "id".to_string(),
                        name_loc: SourceLocation {
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
                    })],
                    children: Vec::new(),
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
                    props: vec![BaseElementProps::Attribute(AttributeNode {
                        name: "id".to_string(),
                        name_loc: SourceLocation {
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
                    })],
                    children: Vec::new(),
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                    ns: Namespaces::HTML as u32,
                    tag: "script".to_string(),
                    props: vec![
                        BaseElementProps::Attribute(AttributeNode {
                            name: "setup".to_string(),
                            name_loc: SourceLocation {
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
                                value: None,
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
                        }),BaseElementProps::Attribute(AttributeNode {
                                name: "lang".to_string(),
                                name_loc: SourceLocation {
                                    start: Position {
                                        offset: 14,
                                        line: 1,
                                        column: 15,
                                    },
                                    end: Position {
                                        offset: 18,
                                        line: 1,
                                        column: 19,
                                    },
                                    source: "lang".to_string(),
                                },
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
                        }),BaseElementProps::Attribute(AttributeNode {
                                name: "generic".to_string(),
                                name_loc: SourceLocation {
                                    start: Position {
                                        offset: 24,
                                        line: 1,
                                        column: 25,
                                    },
                                    end: Position {
                                        offset: 31,
                                        line: 1,
                                        column: 32,
                                    },
                                    source: "generic".to_string(),
                                },
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
                        })],
                    children: Vec::new(),
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
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
                }
            )))
        );
    }

    #[test]
    fn multiple_attributes() {
        let ast = base_parse(r#"<div id=a class="c" inert style=''></div>"#, None);

        let element = ast.children.first();

        assert_eq!(
            element,
            Some(&TemplateChildNode::Element(ElementNode::PlainElement(
                PlainElementNode {
                    ns: Namespaces::HTML as u32,
                    tag: "div".to_string(),
                    props: vec![
                        BaseElementProps::Attribute(AttributeNode {
                            name: "id".to_string(),
                            name_loc: SourceLocation {
                                start: Position {
                                    offset: 5,
                                    line: 1,
                                    column: 6
                                },
                                end: Position {
                                    offset: 7,
                                    line: 1,
                                    column: 8,
                                },
                                source: "id".to_string(),
                            },
                            value: Some(TextNode::new(
                                "a",
                                SourceLocation {
                                    start: Position {
                                        offset: 8,
                                        line: 1,
                                        column: 9
                                    },
                                    end: Position {
                                        offset: 9,
                                        line: 1,
                                        column: 10
                                    },
                                    source: "a".to_string()
                                }
                            )),
                            loc: SourceLocation {
                                start: Position {
                                    offset: 5,
                                    line: 1,
                                    column: 6
                                },
                                end: Position {
                                    offset: 9,
                                    line: 1,
                                    column: 10,
                                },
                                source: "id=a".to_string(),
                            },
                        }),
                        BaseElementProps::Attribute(AttributeNode {
                            name: "class".to_string(),
                            name_loc: SourceLocation {
                                start: Position {
                                    offset: 10,
                                    line: 1,
                                    column: 11
                                },
                                end: Position {
                                    offset: 15,
                                    line: 1,
                                    column: 16,
                                },
                                source: "class".to_string(),
                            },
                            value: Some(TextNode::new(
                                "c",
                                SourceLocation {
                                    start: Position {
                                        offset: 16,
                                        line: 1,
                                        column: 17
                                    },
                                    end: Position {
                                        offset: 19,
                                        line: 1,
                                        column: 20
                                    },
                                    source: r#""c""#.to_string()
                                }
                            )),
                            loc: SourceLocation {
                                start: Position {
                                    offset: 10,
                                    line: 1,
                                    column: 11
                                },
                                end: Position {
                                    offset: 19,
                                    line: 1,
                                    column: 20,
                                },
                                source: r#"class="c""#.to_string(),
                            },
                        }),
                        BaseElementProps::Attribute(AttributeNode {
                            name: "inert".to_string(),
                            name_loc: SourceLocation {
                                start: Position {
                                    offset: 20,
                                    line: 1,
                                    column: 21
                                },
                                end: Position {
                                    offset: 25,
                                    line: 1,
                                    column: 26,
                                },
                                source: "inert".to_string(),
                            },
                            value: None,
                            loc: SourceLocation {
                                start: Position {
                                    offset: 20,
                                    line: 1,
                                    column: 21
                                },
                                end: Position {
                                    offset: 25,
                                    line: 1,
                                    column: 26,
                                },
                                source: "inert".to_string(),
                            },
                        }),
                        BaseElementProps::Attribute(AttributeNode {
                            name: "style".to_string(),
                            name_loc: SourceLocation {
                                start: Position {
                                    offset: 26,
                                    line: 1,
                                    column: 27
                                },
                                end: Position {
                                    offset: 31,
                                    line: 1,
                                    column: 32,
                                },
                                source: "style".to_string(),
                            },
                            value: Some(TextNode::new(
                                "",
                                SourceLocation {
                                    start: Position {
                                        offset: 32,
                                        line: 1,
                                        column: 33
                                    },
                                    end: Position {
                                        offset: 34,
                                        line: 1,
                                        column: 35
                                    },
                                    source: "''".to_string()
                                }
                            )),
                            loc: SourceLocation {
                                start: Position {
                                    offset: 26,
                                    line: 1,
                                    column: 27
                                },
                                end: Position {
                                    offset: 34,
                                    line: 1,
                                    column: 35,
                                },
                                source: "style=''".to_string(),
                            },
                        }),
                    ],
                    children: Vec::new(),
                    is_self_closing: None,
                    codegen_node: None,
                    ssr_codegen_node: None,
                    loc: SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 41,
                            line: 1,
                            column: 42,
                        },
                        source: r#"<div id=a class="c" inert style=''></div>"#.to_string(),
                    },
                }
            )))
        );
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
                    name: "if".to_string(),
                    raw_name: Some("v-if".to_string()),
                    exp: None,
                    arg: None,
                    modifiers: Vec::new(),
                    for_parse_result: None,
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
                })
            );
        }
    }

    #[test]
    fn directive_with_value() {
        let ast = base_parse(r#"<div v-if="a"/>"#, None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(el)) = element {
            let directive = &el.props()[0];
            assert_eq!(
                directive,
                &BaseElementProps::Directive(DirectiveNode {
                    name: "if".to_string(),
                    raw_name: Some("v-if".to_string()),
                    exp: Some(ExpressionNode::new_simple(
                        "a".to_string(),
                        Some(false),
                        Some(SourceLocation {
                            start: Position {
                                offset: 11,
                                line: 1,
                                column: 12
                            },
                            end: Position {
                                offset: 12,
                                line: 1,
                                column: 13
                            },
                            source: "a".to_string()
                        }),
                        Some(ConstantTypes::NotConstant)
                    )),
                    arg: None,
                    modifiers: Vec::new(),
                    for_parse_result: None,
                    loc: SourceLocation {
                        start: Position {
                            offset: 5,
                            line: 1,
                            column: 6,
                        },
                        end: Position {
                            offset: 13,
                            line: 1,
                            column: 14
                        },
                        source: r#"v-if="a""#.to_string()
                    },
                })
            );
        }
    }

    #[test]
    fn directive_with_argument() {
        let ast = base_parse("<div v-on:click/>", None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(el)) = element {
            let directive = &el.props()[0];
            assert_eq!(
                directive,
                &BaseElementProps::Directive(DirectiveNode {
                    name: "on".to_string(),
                    raw_name: Some("v-on:click".to_string()),
                    exp: None,
                    arg: Some(ExpressionNode::new_simple(
                        "click".to_string(),
                        Some(true),
                        Some(SourceLocation {
                            start: Position {
                                offset: 10,
                                line: 1,
                                column: 11
                            },
                            end: Position {
                                offset: 15,
                                line: 1,
                                column: 16
                            },
                            source: "click".to_string()
                        }),
                        Some(ConstantTypes::CanStringify)
                    )),
                    modifiers: Vec::new(),
                    for_parse_result: None,
                    loc: SourceLocation {
                        start: Position {
                            offset: 5,
                            line: 1,
                            column: 6,
                        },
                        end: Position {
                            offset: 15,
                            line: 1,
                            column: 16
                        },
                        source: "v-on:click".to_string()
                    },
                })
            );
        }
    }

    /// #3494
    /// directive argument edge case
    #[test]
    fn directive_argument_edge_case() {
        let ast = base_parse("<div v-slot:slot />", None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(el)) = element {
            let directive = &el.props()[0];
            assert!(matches!(directive, BaseElementProps::Directive(_)));
            if let BaseElementProps::Directive(directive) = directive {
                assert!(directive.arg.is_some());
                if let Some(arg) = &directive.arg {
                    assert_eq!(
                        arg.loc().start,
                        Position {
                            offset: 12,
                            line: 1,
                            column: 13,
                        }
                    );
                    assert_eq!(
                        arg.loc().end,
                        Position {
                            offset: 16,
                            line: 1,
                            column: 17,
                        }
                    );
                }
            }
        }
    }

    /// https://github.com/vuejs/language-tools/issues/2710
    /// directive argument edge case (2)
    #[test]
    fn directive_argument_edge_case_2() {
        let ast = base_parse("<div #item.item />", None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        let Some(TemplateChildNode::Element(el)) = element else {
            unreachable!();
        };
        let directive = &el.props()[0];
        assert!(matches!(directive, BaseElementProps::Directive(_)));
        let BaseElementProps::Directive(directive) = directive else {
            unreachable!();
        };
        assert!(directive.arg.is_some());
        let Some(arg) = &directive.arg else {
            unreachable!();
        };
        assert!(matches!(arg, ExpressionNode::Simple(_)));
        let ExpressionNode::Simple(arg) = arg else {
            unreachable!();
        };
        assert_eq!(arg.content, "item.item");
        assert_eq!(
            arg.loc.start,
            Position {
                offset: 6,
                line: 1,
                column: 7,
            }
        );
        assert_eq!(
            arg.loc.end,
            Position {
                offset: 15,
                line: 1,
                column: 16,
            }
        );
    }

    #[test]
    fn directive_with_dynamic_argument() {
        let ast = base_parse("<div v-on:[event]/>", None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(el)) = element {
            let directive = &el.props()[0];
            assert_eq!(
                directive,
                &BaseElementProps::Directive(DirectiveNode {
                    name: "on".to_string(),
                    raw_name: Some("v-on:[event]".to_string()),
                    exp: None,
                    arg: Some(ExpressionNode::new_simple(
                        "event".to_string(),
                        Some(false),
                        Some(SourceLocation {
                            start: Position {
                                offset: 10,
                                line: 1,
                                column: 11
                            },
                            end: Position {
                                offset: 17,
                                line: 1,
                                column: 18
                            },
                            source: "[event]".to_string()
                        }),
                        Some(ConstantTypes::NotConstant)
                    )),
                    modifiers: Vec::new(),
                    for_parse_result: None,
                    loc: SourceLocation {
                        start: Position {
                            offset: 5,
                            line: 1,
                            column: 6,
                        },
                        end: Position {
                            offset: 17,
                            line: 1,
                            column: 18
                        },
                        source: "v-on:[event]".to_string()
                    },
                })
            );
        }
    }

    #[test]
    fn directive_with_a_modifier() {
        let ast = base_parse("<div v-on.enter/>", None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(el)) = element {
            let directive = &el.props()[0];
            assert_eq!(
                directive,
                &BaseElementProps::Directive(DirectiveNode {
                    name: "on".to_string(),
                    raw_name: Some("v-on.enter".to_string()),
                    exp: None,
                    arg: None,
                    modifiers: vec![SimpleExpressionNode::new(
                        "enter".to_string(),
                        Some(true),
                        Some(SourceLocation {
                            start: Position {
                                offset: 10,
                                line: 1,
                                column: 11,
                            },
                            end: Position {
                                offset: 15,
                                line: 1,
                                column: 16
                            },
                            source: "enter".to_string()
                        }),
                        Some(ConstantTypes::CanStringify)
                    )],
                    for_parse_result: None,
                    loc: SourceLocation {
                        start: Position {
                            offset: 5,
                            line: 1,
                            column: 6,
                        },
                        end: Position {
                            offset: 15,
                            line: 1,
                            column: 16
                        },
                        source: "v-on.enter".to_string()
                    },
                })
            );
        }
    }

    #[test]
    fn directive_with_two_modifiers() {
        let ast = base_parse("<div v-on.enter.exact/>", None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(el)) = element {
            let directive = &el.props()[0];
            assert_eq!(
                directive,
                &BaseElementProps::Directive(DirectiveNode {
                    name: "on".to_string(),
                    raw_name: Some("v-on.enter.exact".to_string()),
                    exp: None,
                    arg: None,
                    modifiers: vec![
                        SimpleExpressionNode::new(
                            "enter".to_string(),
                            Some(true),
                            Some(SourceLocation {
                                start: Position {
                                    offset: 10,
                                    line: 1,
                                    column: 11,
                                },
                                end: Position {
                                    offset: 15,
                                    line: 1,
                                    column: 16
                                },
                                source: "enter".to_string()
                            }),
                            Some(ConstantTypes::CanStringify)
                        ),
                        SimpleExpressionNode::new(
                            "exact".to_string(),
                            Some(true),
                            Some(SourceLocation {
                                start: Position {
                                    offset: 16,
                                    line: 1,
                                    column: 17,
                                },
                                end: Position {
                                    offset: 21,
                                    line: 1,
                                    column: 22
                                },
                                source: "exact".to_string()
                            }),
                            Some(ConstantTypes::CanStringify)
                        )
                    ],
                    for_parse_result: None,
                    loc: SourceLocation {
                        start: Position {
                            offset: 5,
                            line: 1,
                            column: 6,
                        },
                        end: Position {
                            offset: 21,
                            line: 1,
                            column: 22
                        },
                        source: "v-on.enter.exact".to_string()
                    },
                })
            );
        }
    }

    /// directive with argument and modifiers
    #[test]
    fn directive_with_argument_and_modifiers() {
        let ast = base_parse("<div v-on:click.enter.exact/>", None);
        let element = ast.children.first();

        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(el)) = element {
            let directive = &el.props()[0];
            assert_eq!(
                directive,
                &BaseElementProps::Directive(DirectiveNode {
                    name: "on".to_string(),
                    raw_name: Some("v-on:click.enter.exact".to_string()),
                    exp: None,
                    arg: Some(ExpressionNode::new_simple(
                        "click".to_string(),
                        Some(true),
                        Some(SourceLocation {
                            start: Position {
                                offset: 10,
                                line: 1,
                                column: 11
                            },
                            end: Position {
                                offset: 15,
                                line: 1,
                                column: 16
                            },
                            source: "click".to_string()
                        }),
                        Some(ConstantTypes::CanStringify)
                    )),
                    modifiers: vec![
                        SimpleExpressionNode::new(
                            "enter".to_string(),
                            Some(true),
                            Some(SourceLocation {
                                start: Position {
                                    offset: 16,
                                    line: 1,
                                    column: 17,
                                },
                                end: Position {
                                    offset: 21,
                                    line: 1,
                                    column: 22
                                },
                                source: "enter".to_string()
                            }),
                            Some(ConstantTypes::CanStringify)
                        ),
                        SimpleExpressionNode::new(
                            "exact".to_string(),
                            Some(true),
                            Some(SourceLocation {
                                start: Position {
                                    offset: 22,
                                    line: 1,
                                    column: 23,
                                },
                                end: Position {
                                    offset: 27,
                                    line: 1,
                                    column: 28
                                },
                                source: "exact".to_string()
                            }),
                            Some(ConstantTypes::CanStringify)
                        )
                    ],
                    for_parse_result: None,
                    loc: SourceLocation {
                        start: Position {
                            offset: 5,
                            line: 1,
                            column: 6,
                        },
                        end: Position {
                            offset: 27,
                            line: 1,
                            column: 28
                        },
                        source: "v-on:click.enter.exact".to_string()
                    },
                })
            );
        }
    }

    /// directive with dynamic argument and modifiers
    #[test]
    fn directive_with_dynamic_argument_and_modifiers() {
        let ast = base_parse("<div v-on:[a.b].camel/>", None);
        let element = ast.children.first();
        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        let Some(TemplateChildNode::Element(el)) = element else {
            return;
        };
        let directive = el.props().first();
        assert!(matches!(directive, Some(&BaseElementProps::Directive(_))));
        let Some(BaseElementProps::Directive(directive)) = directive else {
            return;
        };
        assert_eq!(
            directive,
            &DirectiveNode {
                name: "on".to_string(),
                raw_name: Some("v-on:[a.b].camel".to_string()),
                exp: None,
                arg: Some(ExpressionNode::new_simple(
                    "a.b",
                    Some(false),
                    Some(SourceLocation {
                        start: Position {
                            offset: 10,
                            line: 1,
                            column: 11,
                        },
                        end: Position {
                            offset: 15,
                            line: 1,
                            column: 16,
                        },
                        source: "[a.b]".to_string(),
                    }),
                    Some(ConstantTypes::NotConstant),
                )),
                modifiers: vec![SimpleExpressionNode::new(
                    "camel",
                    Some(true),
                    Some(SourceLocation {
                        start: Position {
                            offset: 16,
                            line: 1,
                            column: 17,
                        },
                        end: Position {
                            offset: 21,
                            line: 1,
                            column: 22,
                        },
                        source: "camel".to_string(),
                    }),
                    Some(ConstantTypes::CanCache),
                )],
                for_parse_result: None,
                loc: SourceLocation {
                    start: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    end: Position {
                        offset: 21,
                        line: 1,
                        column: 22,
                    },
                    source: "v-on:[a.b].camel".to_string(),
                },
            }
        );
    }

    /// directive with no name
    #[test]
    fn directive_with_no_name() {
        let error_handling_options = TestErrorHandlingOptions::new();
        let ast = base_parse(
            "<div v-/>",
            Some(ParserOptions {
                error_handling_options: Box::new(error_handling_options.clone()),
                ..Default::default()
            }),
        );
        let errors = error_handling_options.try_unwrap();
        assert_eq!(
            errors,
            vec![CompilerError::new(
                ErrorCodes::XMissingDirectiveName,
                Some(SourceLocation {
                    start: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    end: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    source: String::new(),
                })
            )]
        );
        let element = ast.children.first();
        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        let Some(TemplateChildNode::Element(el)) = element else {
            return;
        };
        let directive = el.props().first();
        assert!(matches!(directive, Some(&BaseElementProps::Attribute(_))));
        let Some(BaseElementProps::Attribute(directive)) = directive else {
            return;
        };
        assert_eq!(
            directive,
            &AttributeNode {
                name: "v-".to_string(),
                name_loc: SourceLocation {
                    start: Position {
                        offset: 5,
                        line: 1,
                        column: 6
                    },
                    end: Position {
                        offset: 7,
                        line: 1,
                        column: 8
                    },
                    source: "v-".to_string(),
                },
                value: None,
                loc: SourceLocation {
                    start: Position {
                        offset: 5,
                        line: 1,
                        column: 6
                    },
                    end: Position {
                        offset: 7,
                        line: 1,
                        column: 8
                    },
                    source: "v-".to_string(),
                },
            }
        )
    }

    /// v-bind shorthand
    #[test]
    fn v_bind_shorthand() {
        let ast = base_parse("<div :a=b />", None);
        let element = ast.children.first();
        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        let Some(TemplateChildNode::Element(el)) = element else {
            return;
        };
        let directive = el.props().first();
        assert!(matches!(directive, Some(&BaseElementProps::Directive(_))));
        let Some(BaseElementProps::Directive(directive)) = directive else {
            return;
        };
        assert_eq!(
            directive,
            &DirectiveNode {
                name: "bind".to_string(),
                raw_name: Some(":a".to_string()),
                exp: Some(ExpressionNode::new_simple(
                    "b",
                    Some(false),
                    Some(SourceLocation {
                        start: Position {
                            offset: 8,
                            line: 1,
                            column: 9,
                        },
                        end: Position {
                            offset: 9,
                            line: 1,
                            column: 10,
                        },
                        source: "b".to_string(),
                    }),
                    Some(ConstantTypes::NotConstant),
                )),
                arg: Some(ExpressionNode::new_simple(
                    "a",
                    Some(true),
                    Some(SourceLocation {
                        start: Position {
                            offset: 6,
                            line: 1,
                            column: 7,
                        },
                        end: Position {
                            offset: 7,
                            line: 1,
                            column: 8,
                        },
                        source: "a".to_string(),
                    }),
                    Some(ConstantTypes::CanStringify),
                )),
                modifiers: Vec::new(),
                for_parse_result: None,
                loc: SourceLocation {
                    start: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    end: Position {
                        offset: 9,
                        line: 1,
                        column: 10,
                    },
                    source: ":a=b".to_string(),
                },
            }
        );
    }

    /// v-bind .prop shorthand
    #[test]
    fn v_bind_prop_shorthand() {
        let ast = base_parse("<div .a=b />", None);
        let element = ast.children.first();
        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        let Some(TemplateChildNode::Element(el)) = element else {
            return;
        };
        let directive = el.props().first();
        assert!(matches!(directive, Some(&BaseElementProps::Directive(_))));
        let Some(BaseElementProps::Directive(directive)) = directive else {
            return;
        };
        assert_eq!(
            directive,
            &DirectiveNode {
                name: "bind".to_string(),
                raw_name: Some(".a".to_string()),
                exp: Some(ExpressionNode::new_simple(
                    "b",
                    Some(false),
                    Some(SourceLocation {
                        start: Position {
                            offset: 8,
                            line: 1,
                            column: 9,
                        },
                        end: Position {
                            offset: 9,
                            line: 1,
                            column: 10,
                        },
                        source: "b".to_string(),
                    }),
                    Some(ConstantTypes::NotConstant),
                )),
                arg: Some(ExpressionNode::new_simple(
                    "a",
                    Some(true),
                    Some(SourceLocation {
                        start: Position {
                            offset: 6,
                            line: 1,
                            column: 7,
                        },
                        end: Position {
                            offset: 7,
                            line: 1,
                            column: 8,
                        },
                        source: "a".to_string(),
                    }),
                    Some(ConstantTypes::CanStringify),
                )),
                modifiers: vec![SimpleExpressionNode::new(
                    "prop",
                    Some(false),
                    Some(SourceLocation {
                        start: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        end: Position {
                            offset: 0,
                            line: 1,
                            column: 1,
                        },
                        source: "".to_string(),
                    }),
                    Some(ConstantTypes::NotConstant)
                )],
                for_parse_result: None,
                loc: SourceLocation {
                    start: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    end: Position {
                        offset: 9,
                        line: 1,
                        column: 10,
                    },
                    source: ".a=b".to_string(),
                },
            }
        );
    }

    /// v-bind shorthand with modifier
    #[test]
    fn v_bind_shorthand_with_modifier() {
        let ast = base_parse("<div :a.sync=b />", None);
        let element = ast.children.first();
        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        let Some(TemplateChildNode::Element(el)) = element else {
            return;
        };
        let directive = el.props().first();
        assert!(matches!(directive, Some(&BaseElementProps::Directive(_))));
        let Some(BaseElementProps::Directive(directive)) = directive else {
            return;
        };
        assert_eq!(
            directive,
            &DirectiveNode {
                name: "bind".to_string(),
                raw_name: Some(":a.sync".to_string()),
                exp: Some(ExpressionNode::new_simple(
                    "b",
                    Some(false),
                    Some(SourceLocation {
                        start: Position {
                            offset: 13,
                            line: 1,
                            column: 14,
                        },
                        end: Position {
                            offset: 14,
                            line: 1,
                            column: 15,
                        },
                        source: "b".to_string(),
                    }),
                    Some(ConstantTypes::NotConstant),
                )),
                arg: Some(ExpressionNode::new_simple(
                    "a",
                    Some(true),
                    Some(SourceLocation {
                        start: Position {
                            offset: 6,
                            line: 1,
                            column: 7,
                        },
                        end: Position {
                            offset: 7,
                            line: 1,
                            column: 8,
                        },
                        source: "a".to_string(),
                    }),
                    Some(ConstantTypes::CanStringify),
                )),
                modifiers: vec![SimpleExpressionNode::new(
                    "sync",
                    Some(true),
                    Some(SourceLocation {
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
                        source: "sync".to_string(),
                    }),
                    Some(ConstantTypes::CanCache)
                )],
                for_parse_result: None,
                loc: SourceLocation {
                    start: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    end: Position {
                        offset: 14,
                        line: 1,
                        column: 15,
                    },
                    source: ":a.sync=b".to_string(),
                },
            }
        );
    }

    /// v-on shorthand
    #[test]
    fn v_on_shorthand() {
        let ast = base_parse("<div @a=b />", None);
        let element = ast.children.first();
        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        let Some(TemplateChildNode::Element(el)) = element else {
            return;
        };
        let directive = el.props().first();
        assert!(matches!(directive, Some(&BaseElementProps::Directive(_))));
        let Some(BaseElementProps::Directive(directive)) = directive else {
            return;
        };
        assert_eq!(
            directive,
            &DirectiveNode {
                name: "on".to_string(),
                raw_name: Some("@a".to_string()),
                exp: Some(ExpressionNode::new_simple(
                    "b",
                    Some(false),
                    Some(SourceLocation {
                        start: Position {
                            offset: 8,
                            line: 1,
                            column: 9,
                        },
                        end: Position {
                            offset: 9,
                            line: 1,
                            column: 10,
                        },
                        source: "b".to_string(),
                    }),
                    Some(ConstantTypes::NotConstant),
                )),
                arg: Some(ExpressionNode::new_simple(
                    "a",
                    Some(true),
                    Some(SourceLocation {
                        start: Position {
                            offset: 6,
                            line: 1,
                            column: 7,
                        },
                        end: Position {
                            offset: 7,
                            line: 1,
                            column: 8,
                        },
                        source: "a".to_string(),
                    }),
                    Some(ConstantTypes::CanStringify),
                )),
                modifiers: Vec::new(),
                for_parse_result: None,
                loc: SourceLocation {
                    start: Position {
                        offset: 5,
                        line: 1,
                        column: 6,
                    },
                    end: Position {
                        offset: 9,
                        line: 1,
                        column: 10,
                    },
                    source: "@a=b".to_string(),
                },
            }
        );
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
                    name: ":id".to_string(),
                    name_loc: SourceLocation {
                        start: Position {
                            offset: 11,
                            line: 1,
                            column: 12,
                        },
                        end: Position {
                            offset: 12,
                            line: 1,
                            column: 13,
                        },
                        source: ":".to_string(),
                    },
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
                })]
            );
        }

        //TODO
    }
}

#[cfg(test)]
mod edge_cases {
    use vue_compiler_core::{
        GlobalCompileTimeConstants, ParserOptions, TemplateChildNode, base_parse,
    };

    #[test]
    fn self_closing_single_tag() {
        let ast = base_parse(r#"<div :class="{ some: condition }" />"#, None);

        assert!(ast.children.len() == 1);
        let element = ast.children.first();
        assert!(matches!(element, Some(TemplateChildNode::Element(_))));
        if let Some(TemplateChildNode::Element(element)) = element {
            assert_eq!(element.tag(), "div");
        }
    }

    #[test]
    fn self_closing_multiple_tag() {
        let ast = base_parse(
            "<div :class=\"{ some: condition }\" />\n<p v-bind:style=\"{ color: 'red' }\"/>",
            None,
        );

        assert_eq!(ast.children.len(), 2);
        assert!(matches!(ast.children[0], TemplateChildNode::Element(_)));
        if let TemplateChildNode::Element(element) = &ast.children[0] {
            assert_eq!(element.tag(), "div");
        }
        if let TemplateChildNode::Element(element) = &ast.children[1] {
            assert_eq!(element.tag(), "p");
        }
    }

    #[test]
    fn valid_html() {
        let ast = base_parse(
            "<div :class=\"{ some: condition }\">\n  <p v-bind:style=\"{ color: 'red' }\"/>\n  <!-- a comment with <html> inside it -->\n</div>",
            Some(ParserOptions::default_with_global_compile_time_constants(
                GlobalCompileTimeConstants {
                    __dev__: true,
                    __test__: false,
                    __browser__: false,
                },
            )),
        );

        assert_eq!(ast.children.len(), 1);
        assert!(matches!(ast.children[0], TemplateChildNode::Element(_)));
        let TemplateChildNode::Element(el) = &ast.children[0] else {
            unreachable!();
        };
        assert_eq!(el.tag(), "div");
        assert_eq!(el.children().len(), 2);
        assert!(matches!(el.children()[0], TemplateChildNode::Element(_)));
        if let TemplateChildNode::Element(el) = &el.children()[0] {
            assert_eq!(el.tag(), "p");
        };
        assert!(matches!(el.children()[1], TemplateChildNode::Comment(_)));
    }
}

#[cfg(test)]
mod decode_entities_option {
    use vue_compiler_core::base_parse;

    fn use_decode_by_default() {
        let ast = base_parse("&gt;&lt;&amp;&apos;&quot;&foo;", None);

        assert!(ast.children.len() == 1);
    }
}

/// whitespace management when adopting strategy condense
#[cfg(test)]
mod whitespace_management_when_adopting_strategy_condense {
    use vue_compiler_core::{ParseMode, ParserOptions, TemplateChildNode, base_parse};

    /// should NOT condense whitespaces in RCDATA text mode
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

/// expression parsing
#[cfg(test)]
mod expression_parsing {
    use vue_compiler_core::{BaseElementProps, ParserOptions, TemplateChildNode, base_parse};

    /// v-for
    #[test]
    fn v_for() {
        let ast = base_parse(
            r#"<div v-for="({ a, b }, key, index) of a.b" />"#,
            Some(ParserOptions {
                prefix_identifiers: Some(true),
                ..Default::default()
            }),
        );
        let element = ast.children.first();
        assert!(matches!(element, Some(&TemplateChildNode::Element(_))));
        let Some(TemplateChildNode::Element(el)) = element else {
            return;
        };
        let directive = el.props().first();
        assert!(matches!(directive, Some(&BaseElementProps::Directive(_))));
        let Some(BaseElementProps::Directive(directive)) = directive else {
            return;
        };
        assert!(directive.for_parse_result.is_some());
    }
}
