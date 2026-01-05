use vue_compiler_core::{BaseElementProps, Namespaces, ParseMode, ParserOptions};

pub fn parser_options() -> ParserOptions {
    ParserOptions {
        parse_mode: ParseMode::HTML,
        // is_native_tag: (),
        // is_void_tag: (),
        is_pre_tag: Box::new(|tag| tag == "pre"),
        is_built_in_component: Some(Box::new(|tag| {
            if tag == "Transition" || tag == "transition" {
                Some(())
            } else if tag == "TransitionGroup" || tag == "transition-group" {
                Some(())
            } else {
                None
            }
        })),
        // https://html.spec.whatwg.org/multipage/parsing.html#tree-construction-dispatcher
        get_namespace: Box::new(|tag, parent, root_namespace| {
            let mut ns = if let Some(parent) = parent {
                parent.ns().clone() as u32
            } else {
                root_namespace
            };
            if let Some(parent) = parent {
                if ns == Namespaces::MathML as u32 {
                    if parent.tag() == "annotation-xml" {
                        if tag == "svg" {
                            return Namespaces::SVG as u32;
                        }
                        if parent.props().iter().any(|a| {
                            if let BaseElementProps::Attribute(a) = a
                                && a.name == "encoding"
                                && let Some(value) = &a.value
                                && (value.content == "text/html"
                                    || value.content == "application/xhtml+xml")
                            {
                                true
                            } else {
                                false
                            }
                        }) {
                            ns = Namespaces::HTML as u32;
                        }
                    } else if matches_tag_rule(parent.tag())
                        && tag != "mglyph"
                        && tag != "malignmark"
                    {
                        ns = Namespaces::HTML as u32;
                    }
                } else if ns == Namespaces::SVG as u32 {
                    if parent.tag() == "foreignObject"
                        || parent.tag() == "desc"
                        || parent.tag() == "title"
                    {
                        ns = Namespaces::HTML as u32;
                    }
                }
            }

            if ns == Namespaces::HTML as u32 {
                if tag == "svg" {
                    return Namespaces::SVG as u32;
                }
                if tag == "math" {
                    return Namespaces::MathML as u32;
                }
            }
            ns
        }),
        ..Default::default()
    }
}

fn matches_tag_rule(tag: &str) -> bool {
    let tag_len = tag.len();

    if tag_len != 2 && tag_len != 5 {
        return false;
    }

    matches!(tag, "mi" | "mo" | "mn" | "ms" | "mtext")
}
