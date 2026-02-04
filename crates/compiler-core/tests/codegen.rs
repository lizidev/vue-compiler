mod test_utils;

#[cfg(test)]
mod compiler_codegen {
    use crate::test_utils::{create_element_with_codegen, gen_flag_text};
    use vue_compiler_core::{
        ArrayExpression, ArrayExpressionElement, CacheExpression, CallArgument, CallCallee,
        CallExpression, CodegenMode, CodegenOptions, CodegenResult, CompoundExpressionNode,
        CompoundExpressionNodeChild, CreateComment, CreateElementVNode, CreateVNode,
        ExpressionNode, ForCodegenNode, ForNode, ForParseResult, ForRenderListExpression, Fragment,
        IfCodegenNode, IfConditionalExpression, IfNode, InterpolationNode, JSChildNode,
        ObjectExpression, Property, PropsExpression, RenderList, ResolveComponent,
        ResolveDirective, RootCodegenNode, RootNode, SSRCodegenNode, SimpleExpressionNode,
        SourceLocation, TemplateChildNode, TemplateLiteral, TemplateLiteralElement,
        ToDisplayString, VNodeCallChildren, generate,
    };
    use vue_compiler_shared::PatchFlags;

    #[test]
    fn module_mode_preamble() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.helpers.insert(CreateVNode.to_string());
            root.helpers.insert(ResolveDirective.to_string());
            root
        };
        let CodegenResult { code, .. } = generate(
            root,
            CodegenOptions {
                mode: Some(CodegenMode::Module),
                ..Default::default()
            },
        );

        assert!(code.contains(&format!(
            "import {{ {} as _{0}, {} as _{1} }} from \"vue\"",
            CreateVNode.to_string(),
            ResolveDirective.to_string()
        )));
    }

    #[test]
    fn module_mode_preamble_w_optimize_imports_true() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.helpers.insert(CreateVNode.to_string());
            root.helpers.insert(ResolveDirective.to_string());
            root
        };
        let CodegenResult { code, .. } = generate(
            root,
            CodegenOptions {
                mode: Some(CodegenMode::Module),
                optimize_imports: Some(true),
                ..Default::default()
            },
        );
        assert!(code.contains(&format!(
            "import {{ {}, {} }} from \"vue\"",
            CreateVNode.to_string(),
            ResolveDirective.to_string()
        )));
        assert!(code.contains(&format!(
            "const _{} = {0}, _{} = {1}",
            CreateVNode.to_string(),
            ResolveDirective.to_string()
        )));
    }

    #[test]
    fn function_mode_preamble() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.helpers.insert(CreateVNode.to_string());
            root.helpers.insert(ResolveDirective.to_string());
            root
        };
        let CodegenResult { code, .. } = generate(
            root,
            CodegenOptions {
                mode: Some(CodegenMode::Function),
                ..Default::default()
            },
        );

        assert!(code.contains("const _Vue = Vue"));
        assert!(code.contains(&format!(
            "const {{ {}: _{0}, {}: _{1} }} = _Vue",
            CreateVNode.to_string(),
            ResolveDirective.to_string()
        )));
    }

    #[test]
    fn function_mode_preamble_w_prefix_identifiers_true() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.helpers.insert(CreateVNode.to_string());
            root.helpers.insert(ResolveDirective.to_string());
            root
        };
        let CodegenResult { code, .. } = generate(
            root,
            CodegenOptions {
                mode: Some(CodegenMode::Function),
                prefix_identifiers: Some(true),
                ..Default::default()
            },
        );
        assert!(!code.contains("const _Vue = Vue"));

        assert!(!code.contains(&format!(
            "const {{ {}: _${0}, ${}: _${1} }} = Vue",
            CreateVNode.to_string(),
            ResolveDirective.to_string()
        ),))
    }

    #[test]
    fn assets_temps() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.components = vec!["Foo", "bar-baz", "barbaz", "Qux__self"]
                .into_iter()
                .map(String::from)
                .collect();
            root.directives = vec!["my_dir_0", "my_dir_1"]
                .into_iter()
                .map(String::from)
                .collect();
            root.temps = 3;
            root
        };
        let CodegenResult { code, .. } = generate(
            root,
            CodegenOptions {
                mode: Some(CodegenMode::Function),
                ..Default::default()
            },
        );
        assert!(code.contains(&format!(
            "const _component_Foo = _{}(\"Foo\")\n",
            ResolveComponent.to_string()
        )));
        assert!(code.contains(&format!(
            "const _component_bar_baz = _{}(\"bar-baz\")\n",
            ResolveComponent.to_string()
        )));
        assert!(code.contains(&format!(
            "const _component_barbaz = _{}(\"barbaz\")\n",
            ResolveComponent.to_string()
        )));
        assert!(code.contains(&format!(
            "const _component_Qux = _{}(\"Qux\", true)\n",
            ResolveComponent.to_string()
        )));
        assert!(code.contains(&format!(
            "const _directive_my_dir_0 = _{}(\"my_dir_0\")\n",
            ResolveDirective.to_string()
        )));
        assert!(code.contains(&format!(
            "const _directive_my_dir_1 = _{}(\"my_dir_1\")\n",
            ResolveDirective.to_string()
        )));
        assert!(code.contains("let _temp0, _temp1, _temp2"));
    }

    #[test]
    fn hoists() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.hoists = vec![
                Some(JSChildNode::Simple(SimpleExpressionNode::new(
                    "hello".to_string(),
                    Some(false),
                    Some(SourceLocation::loc_stub()),
                    None,
                ))),
                Some(JSChildNode::Object(ObjectExpression::new(
                    vec![Property::new(
                        ExpressionNode::new_simple(
                            "id",
                            Some(true),
                            Some(SourceLocation::loc_stub()),
                            None,
                        ),
                        JSChildNode::Simple(SimpleExpressionNode::new(
                            "foo",
                            Some(true),
                            Some(SourceLocation::loc_stub()),
                            None,
                        )),
                    )],
                    Some(SourceLocation::loc_stub()),
                ))),
            ];
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());
        assert!(code.contains("const _hoisted_1 = hello"));
        assert!(code.contains("const _hoisted_2 = { id: \"foo\" }"));
    }

    #[test]
    fn temps() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.temps = 3;
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());
        assert!(code.contains("let _temp0, _temp1, _temp2"));
    }

    #[test]
    fn static_text() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::TemplateChild(TemplateChildNode::new_text(
                "hello",
                SourceLocation::loc_stub(),
            )));
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());
        assert!(code.contains("return \"hello\""));
    }

    #[test]
    fn interpolation() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::TemplateChild(
                TemplateChildNode::new_interpolation(
                    ExpressionNode::new_simple(
                        "hello",
                        Some(false),
                        Some(SourceLocation::loc_stub()),
                        None,
                    ),
                    SourceLocation::loc_stub(),
                ),
            ));
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());
        assert!(code.contains(&format!("return _{}(hello)", ToDisplayString.to_string())));
    }

    #[test]
    fn comment() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::TemplateChild(
                TemplateChildNode::new_comment("foo", SourceLocation::loc_stub()),
            ));
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());
        assert!(code.contains(&format!("return _{}(\"foo\")", CreateComment.to_string())));
    }

    #[test]
    fn compound_expression() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::TemplateChild(
                TemplateChildNode::new_compound(
                    vec![
                        CompoundExpressionNodeChild::String("_ctx.".to_string()),
                        CompoundExpressionNodeChild::Simple(SimpleExpressionNode::new(
                            "foo",
                            Some(false),
                            Some(SourceLocation::loc_stub()),
                            None,
                        )),
                        CompoundExpressionNodeChild::String(" + ".to_string()),
                        CompoundExpressionNodeChild::Interpolation(InterpolationNode::new(
                            ExpressionNode::Simple(SimpleExpressionNode::new(
                                "bar",
                                Some(false),
                                Some(SourceLocation::loc_stub()),
                                None,
                            )),
                            SourceLocation::loc_stub(),
                        )),
                        // nested compound
                        CompoundExpressionNodeChild::Compound(CompoundExpressionNode::new(
                            vec![
                                CompoundExpressionNodeChild::String(" + ".to_string()),
                                CompoundExpressionNodeChild::String("nested".to_string()),
                            ],
                            None,
                        )),
                    ],
                    None,
                ),
            ));
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());
        assert!(code.contains(&format!(
            "return _ctx.foo + _{}(bar) + nested",
            ToDisplayString.to_string()
        )));
    }

    #[test]
    fn if_node() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::TemplateChild(TemplateChildNode::If(
                IfNode {
                    branches: Vec::new(),
                    codegen_node: Some(IfCodegenNode::IfConditional(IfConditionalExpression {
                        test: JSChildNode::Simple(SimpleExpressionNode::new(
                            "foo",
                            Some(false),
                            None,
                            None,
                        )),
                        consequent: JSChildNode::Simple(SimpleExpressionNode::new(
                            "bar",
                            Some(false),
                            None,
                            None,
                        )),
                        alternate: JSChildNode::Simple(SimpleExpressionNode::new(
                            "baz",
                            Some(false),
                            None,
                            None,
                        )),
                        newline: true,
                    })),
                    loc: SourceLocation::loc_stub(),
                },
            )));
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());
        assert_ne!(code.split_once("return foo"), None);
        let Some((_, code)) = code.split_once("return foo") else {
            unreachable!();
        };
        let code = code.trim_start();
        assert!(code.starts_with("?"));

        let code = code.split_at(1).1;
        let code = code.trim_start();
        assert!(code.starts_with("bar"));

        let code = code.split_at(3).1;
        let code = code.trim_start();
        assert!(code.starts_with(":"));

        let code = code.split_at(1).1;
        let code = code.trim_start();
        assert!(code.starts_with("baz"));
    }

    /// forNode
    #[test]
    fn for_node() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::TemplateChild(TemplateChildNode::For(
                ForNode {
                    source: ExpressionNode::new_simple("foo", Some(false), None, None),
                    value_alias: None,
                    key_alias: None,
                    object_index_alias: None,
                    children: Vec::new(),
                    parse_result: ForParseResult {
                        source: ExpressionNode::new_simple("", None, None, None),
                        value: None,
                        key: None,
                        index: None,
                        finalized: false,
                    },
                    codegen_node: Some(ForCodegenNode {
                        tag: Fragment.to_string(),
                        children: ForRenderListExpression::new(
                            CallCallee::Symbol(RenderList.to_string()),
                            None,
                            None,
                        ),
                        patch_flag: PatchFlags::Text,
                        disable_tracking: true,
                        is_component: false,
                        loc: SourceLocation::loc_stub(),
                    }),
                    loc: SourceLocation::loc_stub(),
                },
            )));
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());

        assert!(code.contains("openBlock(true)"));
    }

    /// forNode with constant expression
    #[test]
    fn for_node_with_constant_expression() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::TemplateChild(TemplateChildNode::For(
                ForNode {
                    source: ExpressionNode::new_simple(
                        "1 + 2",
                        Some(false),
                        Some(SourceLocation::loc_stub()),
                        Some(vue_compiler_core::ConstantTypes::CanStringify),
                    ),
                    value_alias: None,

                    key_alias: None,
                    object_index_alias: None,
                    children: Vec::new(),
                    parse_result: ForParseResult {
                        source: ExpressionNode::new_simple("", None, None, None),
                        value: None,
                        key: None,
                        index: None,
                        finalized: false,
                    },
                    codegen_node: Some(ForCodegenNode {
                        tag: Fragment.to_string(),
                        children: ForRenderListExpression::new(
                            CallCallee::Symbol(RenderList.to_string()),
                            None,
                            None,
                        ),
                        patch_flag: PatchFlags::StableFragment,
                        disable_tracking: false,
                        is_component: false,
                        loc: SourceLocation::loc_stub(),
                    }),
                    loc: SourceLocation::loc_stub(),
                },
            )));
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());

        assert!(code.contains("openBlock()"));
    }

    #[test]
    /// Element (callExpression + objectExpression + TemplateChildNode[])
    fn element_call_expression_object_expression_template_child_node() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::TemplateChild(TemplateChildNode::Element(
                create_element_with_codegen(
                    r#""div""#,
                    Some(PropsExpression::Object(ObjectExpression::new(
                        vec![
                            Property::new(
                                ExpressionNode::new_simple(
                                    "id",
                                    Some(true),
                                    Some(SourceLocation::loc_stub()),
                                    None,
                                ),
                                JSChildNode::Simple(SimpleExpressionNode::new(
                                    "foo",
                                    Some(true),
                                    Some(SourceLocation::loc_stub()),
                                    None,
                                )),
                            ),
                            Property::new(
                                ExpressionNode::new_simple(
                                    "prop",
                                    Some(false),
                                    Some(SourceLocation::loc_stub()),
                                    None,
                                ),
                                JSChildNode::Simple(SimpleExpressionNode::new(
                                    "bar",
                                    Some(false),
                                    Some(SourceLocation::loc_stub()),
                                    None,
                                )),
                            ),
                            // compound expression as computed key
                            Property::new(
                                ExpressionNode::new_compound(
                                    vec![
                                        CompoundExpressionNodeChild::String("foo + ".to_string()),
                                        CompoundExpressionNodeChild::Simple(
                                            SimpleExpressionNode::new(
                                                "bar",
                                                Some(false),
                                                Some(SourceLocation::loc_stub()),
                                                None,
                                            ),
                                        ),
                                    ],
                                    Some(SourceLocation::loc_stub()),
                                ),
                                JSChildNode::Simple(SimpleExpressionNode::new(
                                    "bar",
                                    Some(false),
                                    Some(SourceLocation::loc_stub()),
                                    None,
                                )),
                            ),
                        ],
                        Some(SourceLocation::loc_stub()),
                    ))),
                    // ChildNode[]
                    Some(VNodeCallChildren::TemplateChildNodeList(vec![
                        TemplateChildNode::Element(create_element_with_codegen(
                            r#""p""#,
                            Some(PropsExpression::Object(ObjectExpression::new(
                                vec![Property::new(
                                    // should quote the key!
                                    ExpressionNode::new_simple(
                                        "some-key",
                                        Some(true),
                                        Some(SourceLocation::loc_stub()),
                                        None,
                                    ),
                                    JSChildNode::Simple(SimpleExpressionNode::new(
                                        "foo",
                                        Some(true),
                                        Some(SourceLocation::loc_stub()),
                                        None,
                                    )),
                                )],
                                Some(SourceLocation::loc_stub()),
                            ))),
                            None,
                            None,
                        )),
                    ])),
                    Some(PatchFlags::FullProps),
                ),
            )));
            root
        };
        let mut options = CodegenOptions::default();
        options.global_compile_time_constants.__dev__ = true;
        let CodegenResult { code, .. } = generate(root, options);

        assert!(code.contains(&format!(
            r#"
    return _{}("div", {{
      id: "foo",
      [prop]: bar,
      [foo + bar]: bar
    }}, [
      _{0}("p", {{ "some-key": "foo" }})
    ], {})"#,
            CreateElementVNode.to_string(),
            gen_flag_text(PatchFlags::FullProps)
        )));
    }

    #[test]
    fn array_expression() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::JSChild(JSChildNode::Array(
                ArrayExpression::new(
                    vec![
                        ArrayExpressionElement::Simple(SimpleExpressionNode::new(
                            "foo",
                            Some(false),
                            None,
                            None,
                        )),
                        ArrayExpressionElement::Call(CallExpression::new(
                            "bar",
                            Some(vec![CallArgument::String("baz".to_string())]),
                            None,
                        )),
                    ],
                    None,
                ),
            )));
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());

        assert!(code.contains(
            "
    return [
      foo,
      bar(baz)
    ]"
        ));
    }

    #[test]
    fn conditional_expression() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::TemplateChild(TemplateChildNode::If(
                IfNode {
                    branches: Vec::new(),
                    codegen_node: Some(IfCodegenNode::IfConditional(IfConditionalExpression {
                        test: JSChildNode::Simple(SimpleExpressionNode::new(
                            "ok",
                            Some(false),
                            None,
                            None,
                        )),
                        consequent: JSChildNode::Call(CallExpression::new("foo", None, None)),
                        alternate: JSChildNode::IfConditional(Box::new(IfConditionalExpression {
                            test: JSChildNode::Simple(SimpleExpressionNode::new(
                                "orNot",
                                Some(false),
                                None,
                                None,
                            )),
                            consequent: JSChildNode::Call(CallExpression::new("bar", None, None)),
                            alternate: JSChildNode::Call(CallExpression::new("baz", None, None)),
                            newline: true,
                        })),
                        newline: true,
                    })),
                    loc: SourceLocation::loc_stub(),
                },
            )));
            root
        };
        let CodegenResult { code, .. } = generate(root, CodegenOptions::default());

        assert!(code.contains(
            "
    return ok
      ? foo()
      : orNot
        ? bar()
        : baz()"
        ));
    }

    #[test]
    fn cache_expression() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::JSChild(JSChildNode::Cache(Box::new(
                CacheExpression::new(
                    1,
                    JSChildNode::Simple(SimpleExpressionNode::new("foo", Some(false), None, None)),
                    None,
                    None,
                ),
            ))));
            root
        };

        let CodegenResult { code, .. } = generate(
            root,
            CodegenOptions {
                mode: Some(CodegenMode::Module),
                prefix_identifiers: Some(true),
                ..Default::default()
            },
        );

        assert!(code.contains("_cache[1] || (_cache[1] = foo)"));
    }

    #[test]
    /// CacheExpression w/ isVOnce: true
    fn cache_expression_w_is_v_once_true() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::JSChild(JSChildNode::Cache(Box::new(
                CacheExpression::new(
                    1,
                    JSChildNode::Simple(SimpleExpressionNode::new("foo", Some(false), None, None)),
                    Some(true),
                    None,
                ),
            ))));
            root
        };

        let CodegenResult { code, .. } = generate(
            root,
            CodegenOptions {
                mode: Some(CodegenMode::Module),
                prefix_identifiers: Some(true),
                ..Default::default()
            },
        );
        assert!(
            code.contains(
                "
  _cache[1] || (
    _setBlockTracking(-1),
    (_cache[1] = foo).cacheIndex = 1,
    _setBlockTracking(1),
    _cache[1]
  )"
                .trim()
            )
        );
    }

    #[test]
    fn template_literal() {
        let root = {
            let mut root = RootNode::new(Vec::new(), None);
            root.codegen_node = Some(RootCodegenNode::JSChild(JSChildNode::Call(
                CallExpression::new(
                    "_push",
                    Some(vec![CallArgument::SSRCodegen(
                        SSRCodegenNode::TemplateLiteral(TemplateLiteral::new(vec![
                            TemplateLiteralElement::String("foo".to_string()),
                            TemplateLiteralElement::JSChild(JSChildNode::Call(
                                CallExpression::new(
                                    "_renderAttr",
                                    Some(vec![
                                        CallArgument::String("id".to_string()),
                                        CallArgument::String("foo".to_string()),
                                    ]),
                                    None,
                                ),
                            )),
                            TemplateLiteralElement::String("bar".to_string()),
                        ])),
                    )]),
                    None,
                ),
            )));
            root
        };

        let CodegenResult { code, .. } = generate(
            root,
            CodegenOptions {
                mode: Some(CodegenMode::Module),
                ssr: Some(true),
                ..Default::default()
            },
        );

        assert!(
            code.contains(
                "
export function ssrRender(_ctx, _push, _parent, _attrs) {
  _push(`foo${_renderAttr(id, foo)}bar`)
}"
                .trim()
            )
        );
    }
}
