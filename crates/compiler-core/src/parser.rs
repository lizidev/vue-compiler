use crate::{
    ast::{
        AttributeNode, BaseElementProps, ConstantTypes, DirectiveNode, ElementNode, ElementTypes,
        ExpressionNode, ForParseResult, Namespaces, NodeTypes, PlainElementNode, RootNode,
        SimpleExpressionNode, SourceLocation, TemplateChildNode, TextNode,
    },
    errors::{CompilerError, ErrorCodes},
    options::{ParserOptions, Whitespace},
    tokenizer::{CharCodes, QuoteType, State, Tokenizer, is_whitespace, to_char_codes},
    utils::{
        GlobalCompileTimeConstants, is_all_whitespace, is_core_component, is_v_pre, match_for_alias,
    },
};

#[derive(Debug)]
pub struct ParserContext<'a> {
    current_options: ParserOptions,
    current_root: RootNode,

    // parser state
    current_input: &'a str,
    current_open_tag: Option<ElementNode>,
    current_prop: Option<BaseElementProps>,
    current_attr_value: String,
    current_attr_start_index: Option<usize>,
    current_attr_end_index: Option<usize>,
    in_pre: i32,
    in_v_pre: bool,
    pub stack: Vec<ElementNode>,

    pub global_compile_time_constants: GlobalCompileTimeConstants,
}

impl<'a> Tokenizer<'a> {
    pub fn get_slice(&self, start: usize, end: usize) -> String {
        self.context.current_input[start..end].to_string()
    }

    fn look_ahead(&self, index: usize, c: u32) -> usize {
        let buffer_len = self.buffer.len();
        for (i, c2) in self.buffer.split_at(index).1.iter().enumerate() {
            if i >= buffer_len - 1 {
                return index + i;
            }
            if *c2 as u32 == c {
                return index + i;
            }
        }
        unreachable!()
    }

    fn back_track(&self, index: usize, c: u32) -> usize {
        for (i, c2) in self.buffer.split_at(index + 1).0.iter().enumerate().rev() {
            if *c2 as u32 == c {
                return i;
            }
        }
        unreachable!()
    }

    fn is_component(&self, el: &ElementNode) -> bool {
        if let Some(is_custom_element) = &self.context.current_options.is_custom_element
            && is_custom_element(el.tag()).unwrap_or_default()
        {
            return false;
        }

        if el.tag() == "component"
            || is_upper_case(el.tag().chars().nth(0).unwrap_or_default() as u32)
            || is_core_component(el.tag()).is_some()
        {
            return true;
        }

        if let Some(is_built_in_component) = &self.context.current_options.is_built_in_component {
            if is_built_in_component(el.tag()).is_some() {
                return true;
            }
        }

        if let Some(is_native_tag) = &self.context.current_options.is_native_tag {
            if !is_native_tag(el.tag()) {
                return true;
            }
        }
        // at this point the tag should be a native tag, but check for potential "is"
        // casting
        for prop in el.props() {
            if let BaseElementProps::Attribute(prop) = prop {
                if prop.name == "is"
                    && let Some(value) = &prop.value
                {
                    if value.content.starts_with("vue:") {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn add_node(&mut self, node: TemplateChildNode) {
        if let Some(parent) = self.context.stack.first_mut() {
            parent.children_mut().push(node);
        } else {
            self.context.current_root.children.push(node);
        }
    }

    fn get_loc(&self, start: usize, end: Option<usize>) -> SourceLocation {
        let end = end.unwrap_or(start);

        return SourceLocation {
            start: self.get_pos(start),
            end: self.get_pos(end),
            source: self.get_slice(start, end),
        };
    }

    fn set_loc_end(&self, loc: &mut SourceLocation, end: usize) {
        loc.end = self.get_pos(end);
        loc.source = self.get_slice(loc.start.offset, end);
    }

    fn end_open_tag(&mut self, end: usize) {
        let Some(mut current_open_tag) = self.context.current_open_tag.take() else {
            unreachable!();
        };

        if self.in_sfc_root() {
            // in SFC mode, generate locations for root-level tags' inner content.
            // currentOpenTag!.innerLoc = getLoc(end + 1, end + 1)
        }
        if current_open_tag.ns() == &(Namespaces::HTML as u32)
            && (self.context.current_options.is_pre_tag)(current_open_tag.tag())
        {
            self.context.in_pre += 1;
        }
        if (self.context.current_options.is_void_tag)(current_open_tag.tag()) {
            self.on_close_tag(&mut current_open_tag, end, None);
            self.add_node(TemplateChildNode::Element(current_open_tag));
        } else {
            if current_open_tag.ns() == &(Namespaces::SVG as u32)
                || current_open_tag.ns() == &(Namespaces::MathML as u32)
            {
                self.in_xml = true;
            }
            self.context.stack.insert(0, current_open_tag);
        }
    }

    fn on_text(&mut self, content: String, start: usize, end: usize) {
        if self.context.global_compile_time_constants.__browser__ {
            if let Some(el) = self.context.stack.first() {
                if el.tag() != "script" && el.tag() != "style" && content.contains('&') {
                    // content = currentOptions.decodeEntities!(content, false)
                }
            } else if content.contains('&') {
                // content = currentOptions.decodeEntities!(content, false)
            }
        }

        if let Some(parent) = self.context.stack.first() {
            if let Some(TemplateChildNode::Text(last_node)) = parent.children().last() {
                let mut loc = last_node.loc.clone();
                self.set_loc_end(&mut loc, end);

                let Some(parent) = self.context.stack.first_mut() else {
                    unreachable!()
                };
                let Some(TemplateChildNode::Text(last_node)) = parent.children_mut().last_mut()
                else {
                    unreachable!()
                };

                last_node.content.push_str(&content);
                last_node.loc = loc;
            } else {
                let loc = self.get_loc(start, Some(end));

                let Some(parent) = self.context.stack.first_mut() else {
                    unreachable!()
                };

                parent
                    .children_mut()
                    .push(TemplateChildNode::Text(TextNode::new(content, loc)))
            }
        } else {
            if let Some(TemplateChildNode::Text(last_node)) =
                self.context.current_root.children.last()
            {
                let mut loc = last_node.loc.clone();
                self.set_loc_end(&mut loc, end);

                let Some(TemplateChildNode::Text(last_node)) =
                    self.context.current_root.children.last_mut()
                else {
                    unreachable!()
                };

                last_node.content.push_str(&content);
                last_node.loc = loc;
            } else {
                let loc = self.get_loc(start, Some(end));
                self.context
                    .current_root
                    .children
                    .push(TemplateChildNode::Text(TextNode::new(content, loc)))
            }
        }
    }

    fn on_close_tag(&mut self, el: &mut ElementNode, end: usize, is_implied: Option<bool>) {
        let is_implied = is_implied.unwrap_or_default();

        // attach end position
        if is_implied {
            // implied close, end should be backtracked to close
            self.set_loc_end(el.loc_mut(), self.back_track(end, CharCodes::Lt as u32))
        } else {
            self.set_loc_end(el.loc_mut(), self.look_ahead(end, CharCodes::Gt as u32) + 1)
        }

        if !self.context.in_v_pre {
            if el.tag() == "slot" {
                *el.tag_type_mut() = ElementTypes::Slot;
            } else if is_fragment_template(el) {
                *el.tag_type_mut() = ElementTypes::Template;
            } else if self.is_component(el) {
                *el.tag_type_mut() = ElementTypes::Component;
            }
        }

        // whitespace management
        if !self.in_rc_data {
            let children = el.children_mut().drain(..).collect();
            *el.children_mut() = condense_whitespace(
                children,
                self.context.current_options.whitespace != Some(Whitespace::Preserve),
                self.context.in_pre,
            );
        }

        if self.context.in_v_pre {
            self.in_v_pre = false;
            self.context.in_v_pre = false;
        }
    }

    fn create_exp(
        &self,
        content: String,
        is_static: Option<bool>,
        loc: SourceLocation,
        const_type: Option<ConstantTypes>,
        parse_mode: Option<ExpParseMode>,
    ) -> SimpleExpressionNode {
        let parse_mode = parse_mode.unwrap_or(ExpParseMode::Normal);
        let exp = SimpleExpressionNode::new(
            content.clone(),
            Some(is_static.unwrap_or_default()),
            Some(loc),
            Some(const_type.unwrap_or(ConstantTypes::NotConstant)),
        );

        if !self.context.global_compile_time_constants.__browser__
            && !is_static.unwrap_or_default()
            && self
                .context
                .current_options
                .prefix_identifiers
                .unwrap_or_default()
            && parse_mode != ExpParseMode::Skip
            && !content.trim().is_empty()
        {
            // if (isSimpleIdentifier(content)) {
            //     exp.ast = null // fast path
            //     return exp
            // }
            // try {
            //     const plugins = currentOptions.expressionPlugins
            //     const options: BabelOptions = {
            //     plugins: plugins ? [...plugins, 'typescript'] : ['typescript'],
            //     }
            //     if (parseMode === ExpParseMode.Statements) {
            //     // v-on with multi-inline-statements, pad 1 char
            //     exp.ast = parse(` ${content} `, options).program
            //     } else if (parseMode === ExpParseMode.Params) {
            //     exp.ast = parseExpression(`(${content})=>{}`, options)
            //     } else {
            //     // normal exp, wrap with parens
            //     exp.ast = parseExpression(`(${content})`, options)
            //     }
            // } catch (e: any) {
            //     exp.ast = false // indicate an error
            //     emitError(ErrorCodes.X_INVALID_EXPRESSION, loc.start.offset, e.message)
            // }
        }
        exp
    }

    fn create_alias_expression(
        &self,
        loc: &SourceLocation,
        content: String,
        offset: usize,
        as_param: Option<bool>,
    ) -> SimpleExpressionNode {
        let as_param = as_param.unwrap_or_default();
        let start = loc.start.offset + offset;
        let end = start + content.len();

        let loc = self.get_loc(start, Some(end));
        self.create_exp(
            content,
            Some(false),
            loc,
            Some(ConstantTypes::NotConstant),
            Some(if as_param {
                ExpParseMode::Params
            } else {
                ExpParseMode::Normal
            }),
        )
    }

    fn parse_for_expression(&self, input: &SimpleExpressionNode) -> Option<ForParseResult> {
        let in_match = match_for_alias(&input.content);

        let Some(in_match) = in_match else {
            return None;
        };

        let (lhs, rhs) = in_match;

        let Some(offset) = input.content.find(&rhs) else {
            unreachable!();
        };
        let mut result = ForParseResult {
            source: ExpressionNode::Simple(self.create_alias_expression(
                &input.loc,
                rhs.trim().to_string(),
                offset,
                None,
            )),
            value: None,
            key: None,
            index: None,
            finalized: false,
        };

        let value_content = {
            let mut content = lhs.trim();
            if content.chars().next() == Some('(') {
                content = &content[1..];
            }
            if content.chars().last() == Some(')') {
                content = &content[..(content.len() - 1)];
            }
            content.to_string()
        };
        let Some(trimmed_offset) = lhs.find(&value_content) else {
            unreachable!();
        };

        // TODO

        if value_content.len() != 0 {
            result.value = Some(ExpressionNode::Simple(self.create_alias_expression(
                &input.loc,
                value_content,
                trimmed_offset,
                Some(true),
            )));
        }

        Some(result)
    }

    fn emit_error(&mut self, code: ErrorCodes, index: usize) {
        let loc = self.get_loc(index, Some(index));

        self.context
            .current_options
            .error_handling_options
            .on_error(CompilerError::new(code, Some(loc)));
    }
}

/// Callbacks
impl<'a> Tokenizer<'a> {
    pub fn onerr(&mut self, code: ErrorCodes, index: usize) {
        self.emit_error(code, index);
    }

    pub fn ontext(&mut self, start: usize, end: usize) {
        self.on_text(self.get_slice(start, end), start, end);
    }

    pub fn oninterpolation(&mut self, start: usize, end: usize) {
        if self.context.in_v_pre {
            return self.on_text(self.get_slice(start, end), start, end);
        }
        let inner_start = {
            let mut inner_start = start + self.delimiter_open.len();
            while is_whitespace(self.buffer[inner_start] as u32) {
                inner_start += 1;
            }
            inner_start
        };
        let inner_end = {
            let mut inner_end = end - self.delimiter_close.len();
            while is_whitespace(self.buffer[inner_end - 1] as u32) {
                inner_end -= 1;
            }
            inner_end
        };
        let exp = self.get_slice(inner_start, inner_end);
        // decode entities for backwards compat
        if exp.contains('&') {
            if self.context.global_compile_time_constants.__browser__ {
                // exp = currentOptions.decodeEntities!(exp, false)
            } else {
                // exp = decodeHTML(exp)
            }
        }

        let loc = self.get_loc(inner_start, Some(inner_end));
        let exp = self.create_exp(exp, Some(false), loc, None, None);
        let loc = self.get_loc(start, Some(end));
        self.add_node(TemplateChildNode::new_interpolation(
            ExpressionNode::Simple(exp),
            loc,
        ));
    }

    pub fn onopentagname(&mut self, start: usize, end: usize) {
        let name = self.get_slice(start, end);
        let loc = self.get_loc(start - 1, Some(end));
        let ns = (self.context.current_options.get_namespace)(
            &name,
            self.context.stack.first(),
            self.context.current_options.ns.clone() as u32,
        );
        self.context.current_open_tag = Some(ElementNode::PlainElement(PlainElementNode {
            ns,
            tag: name,
            tag_type: ElementTypes::Element,
            props: Vec::new(),
            children: Vec::new(),
            is_self_closing: None,
            codegen_node: None,
            ssr_codegen_node: None,
            loc,
        }))
    }

    pub fn onopentagend(&mut self, end: usize) {
        self.end_open_tag(end);
    }

    pub fn onclosetag(&mut self, start: usize, end: usize) {
        let name = self.get_slice(start, end);
        if !(self.context.current_options.is_void_tag)(&name) {
            let mut found = false;
            let mut index = 0;
            for (i, e) in self.context.stack.iter().enumerate() {
                if e.tag().to_lowercase() == name.to_lowercase() {
                    found = true;
                    if i > 0 {
                        self.emit_error(
                            ErrorCodes::XMissingEndTag,
                            self.context.stack[0].loc().start.offset,
                        );
                    }
                    index = i;
                    break;
                }
            }
            if found {
                for j in 0..=index {
                    let mut el = self.context.stack.remove(0);
                    self.on_close_tag(&mut el, end, Some(j < index));
                    self.add_node(TemplateChildNode::Element(el));
                }
            } else {
                self.emit_error(
                    ErrorCodes::XInvalidEndTag,
                    self.back_track(start, CharCodes::Lt as u32),
                );
            }
        }
    }

    pub fn onselfclosingtag(&mut self, end: usize) {
        let name = if let Some(current_open_tag) = self.context.current_open_tag.as_mut() {
            *current_open_tag.is_self_closing_mut() = Some(true);
            current_open_tag.tag().clone()
        } else {
            unreachable!();
        };

        self.end_open_tag(end);
        if self
            .context
            .stack
            .first()
            .is_some_and(|el| el.tag() == &name)
        {
            let mut el = self.context.stack.remove(0);
            self.on_close_tag(&mut el, end, None);
            self.add_node(TemplateChildNode::Element(el));
        }
    }

    pub fn onattribname(&mut self, start: usize, end: usize) {
        // plain attribute
        self.context.current_prop = Some(BaseElementProps::Attribute(AttributeNode {
            name: self.get_slice(start, end),
            name_loc: self.get_loc(start, Some(end)),
            value: None,
            loc: self.get_loc(start, None),
        }));
    }

    pub fn ondirname(&mut self, start: usize, end: usize) {
        let raw = self.get_slice(start, end);
        let name = if raw == "." || raw == ":" {
            "bind".to_string()
        } else if raw == "@" {
            "on".to_string()
        } else if raw == "#" {
            "slot".to_string()
        } else {
            raw.split_at(2).1.to_string()
        };

        if !self.context.in_v_pre && name.is_empty() {
            self.emit_error(ErrorCodes::XMissingDirectiveName, start);
        }

        if self.context.in_v_pre || name.is_empty() {
            self.context.current_prop = Some(BaseElementProps::Attribute(AttributeNode {
                name: raw,
                name_loc: self.get_loc(start, Some(end)),
                value: None,
                loc: self.get_loc(start, None),
            }));
        } else {
            let modifiers = if raw == "." {
                vec![SimpleExpressionNode::new(
                    "prop".to_string(),
                    None,
                    None,
                    None,
                )]
            } else {
                Vec::new()
            };
            self.context.current_prop = Some(BaseElementProps::Directive(DirectiveNode {
                name: name.clone(),
                raw_name: Some(raw),
                exp: None,
                arg: None,
                modifiers,
                for_parse_result: None,
                loc: self.get_loc(start, None),
            }));
            if name == "pre" {
                self.in_v_pre = true;
                self.context.in_v_pre = true;
                // currentVPreBoundary = currentOpenTag
                // // convert dirs before this one to attributes
                // const props = currentOpenTag!.props
                // for (let i = 0; i < props.length; i++) {
                //   if (props[i].type === NodeTypes.DIRECTIVE) {
                //     props[i] = dirToAttr(props[i] as DirectiveNode)
                //   }
                // }
            }
        }
    }

    pub fn ondirarg(&mut self, start: usize, end: usize) {
        if start == end {
            return;
        }
        let arg = self.get_slice(start, end);
        let Some(current_prop) = &self.context.current_prop else {
            unreachable!();
        };
        if self.context.in_v_pre && !is_v_pre(current_prop) {
            let Some(BaseElementProps::Attribute(attr)) = &mut self.context.current_prop else {
                unreachable!();
            };
            attr.name.push_str(&arg);
            // setLocEnd((currentProp as AttributeNode).nameLoc, end)
        } else {
            let is_static = !arg.starts_with('[');
            let content = if is_static {
                arg
            } else {
                arg[1..(arg.len() - 1)].to_string()
            };
            let loc = self.get_loc(start, Some(end));
            let const_type = if is_static {
                ConstantTypes::CanStringify
            } else {
                ConstantTypes::NotConstant
            };
            let exp = self.create_exp(content, Some(is_static), loc, Some(const_type), None);
            let Some(BaseElementProps::Directive(dir)) = &mut self.context.current_prop else {
                unreachable!();
            };

            dir.arg = Some(ExpressionNode::Simple(exp));
        }
    }

    pub fn ondirmodifier(&mut self, start: usize, end: usize) {
        let dir_mod = self.get_slice(start, end);
        if self.context.in_v_pre
            && let Some(prop) = &self.context.current_prop
            && !is_v_pre(prop)
        {
            // ;(currentProp as AttributeNode).name += '.' + mod
            // setLocEnd((currentProp as AttributeNode).nameLoc, end)
            todo!()
        } else if matches!(
            &self.context.current_prop,
            Some(BaseElementProps::Directive(prop))
            if prop.name == "slot"
        ) {
            // slot has no modifiers, special case for edge cases like
            // https://github.com/vuejs/language-tools/issues/2710
            let mut arg_loc: Option<SourceLocation> = None;
            if let Some(BaseElementProps::Directive(dir)) = &self.context.current_prop
                && let Some(arg) = &dir.arg
            {
                debug_assert!(matches!(arg, ExpressionNode::Simple(_)));
                if let ExpressionNode::Simple(arg) = arg {
                    let mut loc = arg.loc.clone();
                    self.set_loc_end(&mut loc, end);
                    arg_loc = Some(loc);
                }
            }
            if let Some(BaseElementProps::Directive(dir)) = &mut self.context.current_prop
                && let Some(arg) = &mut dir.arg
            {
                debug_assert!(matches!(arg, ExpressionNode::Simple(_)));
                if let ExpressionNode::Simple(arg) = arg {
                    arg.content.push_str(&format!(".{dir_mod}"));
                    let Some(arg_loc) = arg_loc else {
                        unreachable!();
                    };
                    arg.loc = arg_loc;
                }
            }
        } else {
            let exp = SimpleExpressionNode::new(
                dir_mod,
                Some(true),
                Some(self.get_loc(start, Some(end))),
                None,
            );
            let Some(BaseElementProps::Directive(dir)) = self.context.current_prop.as_mut() else {
                unreachable!();
            };
            dir.modifiers.push(exp);
        }
    }

    pub fn onattribdata(&mut self, start: usize, end: usize) {
        self.context
            .current_attr_value
            .push_str(&self.get_slice(start, end));
        if self.context.current_attr_start_index.is_none() {
            self.context.current_attr_start_index = Some(start);
        }
        self.context.current_attr_end_index = Some(end);
    }

    pub fn onattribnameend(&mut self, end: usize) {
        let Some(current_prop) = &self.context.current_prop else {
            unreachable!();
        };
        let start = current_prop.loc().start.offset;
        let name = self.get_slice(start, end);
        if let Some(BaseElementProps::Directive(dir)) = self.context.current_prop.as_mut() {
            dir.raw_name = Some(name.clone());
        }

        // check duplicate attrs
        let Some(current_open_tag) = &self.context.current_open_tag else {
            unreachable!();
        };
        if current_open_tag.props().iter().any(|p| match p {
            BaseElementProps::Attribute(attr) => attr.name == name,
            BaseElementProps::Directive(dir) => dir.raw_name.as_ref() == Some(&name),
        }) {
            self.emit_error(ErrorCodes::DuplicateAttribute, start);
        }
    }

    pub fn onattribend(&mut self, quote: QuoteType, end: usize) {
        if self.context.current_open_tag.is_some() && self.context.current_prop.is_some() {
            // finalize end pos
            if let Some(current_prop) = &self.context.current_prop {
                let mut loc = current_prop.loc().clone();
                self.set_loc_end(&mut loc, end);
                if let Some(current_prop) = self.context.current_prop.as_mut() {
                    *current_prop.loc_mut() = loc;
                }
            }

            if quote != QuoteType::NoValue {
                if self.context.global_compile_time_constants.__browser__
                    && self.context.current_attr_value.contains('&')
                {
                    //       currentAttrValue = currentOptions.decodeEntities!(
                    //         currentAttrValue,
                    //         true,
                    //       )
                }

                if matches!(
                    self.context.current_prop,
                    Some(BaseElementProps::Attribute(_))
                ) {
                    // assign value

                    // condense whitespaces in class
                    if let Some(prop) = &self.context.current_prop
                        && prop.name() == "class"
                    {
                        self.context.current_attr_value =
                            condense(self.context.current_attr_value.clone())
                                .trim()
                                .to_string();
                    }

                    if quote == QuoteType::Unquoted && !self.context.current_attr_value.is_empty() {
                        self.emit_error(ErrorCodes::MissingAttributeValue, end);
                    }

                    let Some(current_attr_start_index) = self.context.current_attr_start_index
                    else {
                        unreachable!();
                    };

                    let Some(current_attr_end_index) = self.context.current_attr_end_index else {
                        unreachable!();
                    };

                    let current_attr_value = self.context.current_attr_value.clone();
                    let loc = if quote == QuoteType::Unquoted {
                        self.get_loc(current_attr_start_index, Some(current_attr_end_index))
                    } else {
                        self.get_loc(
                            current_attr_start_index - 1,
                            Some(current_attr_end_index + 1),
                        )
                    };

                    if let Some(BaseElementProps::Attribute(current_prop)) =
                        self.context.current_prop.as_mut()
                    {
                        current_prop.value = Some(TextNode::new(current_attr_value, loc))
                    }

                    if self.in_sfc_root() {
                        if let Some(el) = &self.context.current_open_tag
                            && el.tag() == "template"
                        {
                            if let Some(prop) = &self.context.current_prop
                                && prop.name() == "lang"
                            {
                                if !self.context.current_attr_value.is_empty()
                                    && self.context.current_attr_value != "html"
                                {
                                    // SFC root template with preprocessor lang, force tokenizer to
                                    // RCDATA mode
                                    self.enter_rc_data(to_char_codes("</template".to_string()), 0);
                                }
                            }
                        }
                    }
                } else {
                    // directive
                    let mut exp_parse_mode = ExpParseMode::Normal;
                    if !self.context.global_compile_time_constants.__browser__ {
                        let Some(current_prop) = &self.context.current_prop else {
                            unreachable!();
                        };
                        if current_prop.name() == "for" {
                            exp_parse_mode = ExpParseMode::Skip;
                        } else if current_prop.name() == "slot" {
                            exp_parse_mode = ExpParseMode::Params;
                        } else if current_prop.name() == "on"
                            && self.context.current_attr_value.contains(';')
                        {
                            exp_parse_mode = ExpParseMode::Statements;
                        }
                    }
                    let Some(current_attr_start_index) = self.context.current_attr_start_index
                    else {
                        unreachable!();
                    };

                    let Some(current_attr_end_index) = self.context.current_attr_end_index else {
                        unreachable!();
                    };
                    let loc = self.get_loc(current_attr_start_index, Some(current_attr_end_index));
                    let exp = self.create_exp(
                        self.context.current_attr_value.clone(),
                        Some(false),
                        loc,
                        Some(ConstantTypes::NotConstant),
                        Some(exp_parse_mode),
                    );
                    if matches!(
                        self.context.current_prop,
                        Some(BaseElementProps::Directive(_))
                    ) {
                        let for_parse_result = if matches!(
                            &self.context.current_prop,
                            Some(BaseElementProps::Directive(prop))
                            if prop.name == "for"
                        ) {
                            Some(self.parse_for_expression(&exp))
                        } else {
                            None
                        };
                        let Some(BaseElementProps::Directive(current_prop)) =
                            &mut self.context.current_prop
                        else {
                            unreachable!();
                        };
                        current_prop.exp = Some(ExpressionNode::Simple(exp));
                        if let Some(for_parse_result) = for_parse_result {
                            current_prop.for_parse_result = for_parse_result;
                        }
                    }
                }
            }

            let Some(current_prop) = &self.context.current_prop else {
                unreachable!();
            };

            if !matches!(current_prop, BaseElementProps::Directive(_))
                || current_prop.name() != "pre"
            {
                if let Some(current_open_tag) = self.context.current_open_tag.as_mut() {
                    current_open_tag.props_mut().push(current_prop.clone());
                }
            }
        }
        self.context.current_attr_value = String::new();
        self.context.current_attr_start_index = None;
        self.context.current_attr_end_index = None;
    }

    pub fn oncomment(&mut self, start: usize, end: usize) {
        if self.context.current_options.comments.unwrap_or_default() {
            let content = self.get_slice(start, end);
            let loc = self.get_loc(start - 4, Some(end + 3));
            self.add_node(TemplateChildNode::new_comment(content, loc));
        }
    }

    pub fn onend(&mut self) {
        let end = self.context.current_input.len();
        // EOF ERRORS
        if (self.context.global_compile_time_constants.__dev__
            || !self.context.global_compile_time_constants.__browser__)
            && self.state != State::Text
        {
            match self.state {
                State::BeforeTagName | State::BeforeClosingTagName => {
                    self.emit_error(ErrorCodes::EOFBeforeTagName, end);
                }
                State::Interpolation | State::InterpolationClose => {
                    let Some(section_start) = self.section_start else {
                        unreachable!()
                    };
                    self.emit_error(ErrorCodes::XMissingInterpolationEnd, section_start);
                }
                State::InCommentLike => {
                    if self.current_sequence == self.sequences.cdata_end {
                        self.emit_error(ErrorCodes::EOFInCdata, end);
                    } else {
                        self.emit_error(ErrorCodes::EOFInComment, end);
                    }
                }
                State::InTagName
                | State::InSelfClosingTag
                | State::InClosingTagName
                | State::BeforeAttrName
                | State::InAttrName
                | State::InDirName
                | State::InDirArg
                | State::InDirDynamicArg
                | State::InDirModifier
                | State::AfterAttrName
                | State::BeforeAttrValue
                | State::InAttrValueDq
                | State::InAttrValueSq
                | State::InAttrValueNq => {
                    self.emit_error(ErrorCodes::EOFInTag, end);
                }
                _ => {
                    // println!("{:#?}", self.state);
                }
            }
        }

        let stack = self.context.stack.drain(..).collect::<Vec<ElementNode>>();
        for mut item in stack {
            self.on_close_tag(&mut item, end - 1, None);
            let offset = item.loc().start.offset;
            self.add_node(TemplateChildNode::Element(item));
            self.emit_error(ErrorCodes::XMissingEndTag, offset);
        }
    }

    pub fn oncdata(&mut self, start: usize, end: usize) {
        if let Some(el) = self.context.stack.first()
            && el.ns() != &(Namespaces::HTML as u32)
        {
            self.on_text(self.get_slice(start, end), start, end);
        } else {
            self.emit_error(ErrorCodes::CdataInHtmlContent, start - 9);
        }
    }

    pub fn onprocessinginstruction(&mut self, start: usize, _end: usize) {
        // ignore as we do not have runtime handling for this, only check error
        let ns = if let Some(el) = self.context.stack.first() {
            el.ns().clone()
        } else {
            self.context.current_options.ns.clone() as u32
        };
        if ns == Namespaces::HTML as u32 {
            self.emit_error(
                ErrorCodes::UnexpectedQuestionMarkInsteadOfTagName,
                start - 1,
            );
        }
    }
}

const SPECIAL_TEMPLATE_DIR: [&'static str; 5] = ["if", "else", "else-if", "for", "slot"];
fn is_fragment_template(el: &ElementNode) -> bool {
    if el.tag() == "template" {
        for prop in el.props() {
            if let BaseElementProps::Directive(dir) = prop {
                if SPECIAL_TEMPLATE_DIR.contains(&dir.name.as_str()) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_upper_case(c: u32) -> bool {
    c > 64 && c < 91
}

/// should_condense: currentOptions.whitespace !== 'preserve'
fn condense_whitespace(
    nodes: Vec<TemplateChildNode>,
    should_condense: bool,
    in_pre: i32,
) -> Vec<TemplateChildNode> {
    let mut nodes: Vec<Option<TemplateChildNode>> =
        nodes.into_iter().map(|node| Some(node)).collect();
    for i in 0..nodes.len() {
        if matches!(nodes[i], Some(TemplateChildNode::Text(_))) {
            if in_pre == 0 {
                if let Some(TemplateChildNode::Text(node)) = &nodes[i]
                    && is_all_whitespace(&node.content)
                {
                    let prev = if i > 0
                        && let Some(node) = &nodes[i - 1]
                    {
                        Some(node.type_().clone())
                    } else {
                        None
                    };
                    let next = if i != nodes.len() - 1
                        && let Some(node) = &nodes[i + 1]
                    {
                        Some(node.type_().clone())
                    } else {
                        None
                    };
                    // Remove if:
                    // - the whitespace is the first or last node, or:
                    // - (condense mode) the whitespace is between two comments, or:
                    // - (condense mode) the whitespace is between comment and element, or:
                    // - (condense mode) the whitespace is between two elements AND contains newline
                    if prev.is_none() || next.is_none() {
                        nodes[i] = None;
                    } else if let Some(prev) = prev
                        && let Some(next) = next
                        && should_condense
                        && ((prev == NodeTypes::Comment
                            && (next == NodeTypes::Comment || next == NodeTypes::Element))
                            || (prev == NodeTypes::Element
                                && (next == NodeTypes::Comment
                                    || (next == NodeTypes::Element
                                        && has_newline_char(&node.content)))))
                    {
                        nodes[i] = None;
                    } else {
                        // Otherwise, the whitespace is condensed into a single space
                        let Some(TemplateChildNode::Text(node)) = &mut nodes[i] else {
                            unreachable!();
                        };
                        node.content = " ".to_string();
                    }
                } else if should_condense {
                    // in condense mode, consecutive whitespaces in text are condensed
                    // down to a single space.
                    let Some(TemplateChildNode::Text(node)) = &mut nodes[i] else {
                        unreachable!();
                    };
                    node.content = condense(node.content.clone());
                }
            } else {
                // #6410 normalize windows newlines in <pre>:
                // in SSR, browsers normalize server-rendered \r\n into a single \n
                // in the DOM
                let Some(TemplateChildNode::Text(node)) = &mut nodes[i] else {
                    unreachable!();
                };
                node.content = node.content.replace("\r\n", "\n")
            }
        }
    }

    nodes.into_iter().flatten().collect()
}

fn has_newline_char(str: &str) -> bool {
    str.chars()
        .any(|c| c as u32 == CharCodes::NewLine || c as u32 == CharCodes::CarriageReturn)
}

fn condense(str: String) -> String {
    let mut ret = String::new();
    let mut prev_char_is_whitespace = false;
    for c in str.chars() {
        if is_whitespace(c as u32) {
            if !prev_char_is_whitespace {
                ret.push(' ');
                prev_char_is_whitespace = true;
            }
        } else {
            ret.push(c);
            prev_char_is_whitespace = false;
        }
    }
    ret
}

#[derive(Debug, PartialEq)]
enum ExpParseMode {
    Normal,
    Params,
    Statements,
    Skip,
}

pub fn base_parse(input: &str, options: Option<ParserOptions>) -> RootNode {
    let options = options.unwrap_or_default();

    let global_compile_time_constants = options.global_compile_time_constants.clone();

    let context = ParserContext {
        current_options: options,
        current_root: RootNode::new(vec![], Some(input.to_string())),

        current_input: input,
        current_open_tag: None,
        current_prop: None,
        current_attr_value: String::new(),
        current_attr_start_index: None,
        current_attr_end_index: None,
        in_pre: 0,
        in_v_pre: false,
        stack: Vec::new(),

        global_compile_time_constants,
    };

    let mut tokenizer = Tokenizer::new(context);

    tokenizer.mode = tokenizer.context.current_options.parse_mode.clone();

    tokenizer.in_xml = tokenizer.context.current_options.ns == Namespaces::SVG
        || tokenizer.context.current_options.ns == Namespaces::MathML;

    tokenizer.parse(input);

    let ParserContext {
        mut current_root,
        current_options,
        in_pre,
        ..
    } = tokenizer.context;

    let children = current_root.children.drain(..).collect();
    current_root.children = condense_whitespace(
        children,
        current_options.whitespace != Some(Whitespace::Preserve),
        in_pre,
    );

    current_root
}
