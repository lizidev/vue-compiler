use crate::{
    ast::{
        BaseElementProps, DirectiveNode, ElementNode, ExpressionNode, NodeTypes, ObjectExpression,
        Property, PropsExpression, VNodeCall,
    },
    codegen::AssetType,
    tokenizer::is_whitespace,
    transform::TransformContext,
};

pub fn is_v_pre(p: &BaseElementProps) -> bool {
    if let BaseElementProps::Directive(dir) = p {
        dir.name == "pre"
    } else {
        false
    }
}

pub fn is_core_component(tag: &str) -> Option<String> {
    match tag {
        "Teleport" | "teleport" => Some("TELEPORT".to_string()),
        "Suspense" | "suspense" => Some("SUSPENSE".to_string()),
        "KeepAlive" | "keep-alive" => Some("KEEP_ALIVE".to_string()),
        "BaseTransition" | "base-transition" => Some("BASE_TRANSITION".to_string()),
        _ => None,
    }
}

pub fn is_simple_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let chars: Vec<char> = name.chars().collect();

    let first_char = chars[0];
    if first_char.is_ascii_digit() {
        return false;
    }

    for c in chars {
        let is_valid = match c {
            '$' => true,
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => true,
            c if c as u32 >= 0xA0 => true,
            _ => false,
        };

        if !is_valid {
            return false;
        }
    }
    true
}

pub fn find_dir(
    node: &ElementNode,
    name: &str,
    allow_empty: Option<bool>,
) -> Option<DirectiveNode> {
    let allow_empty = allow_empty.unwrap_or_default();
    for prop in node.props() {
        if let BaseElementProps::Directive(p) = prop
            && (allow_empty || p.exp.is_some())
            && p.name == name
        {
            return Some(p.clone());
        }
    }
    None
}

pub fn find_prop(
    node: &ElementNode,
    name: &str,
    dynamic_only: Option<bool>,
    allow_empty: Option<bool>,
) -> Option<BaseElementProps> {
    let dynamic_only = dynamic_only.unwrap_or_default();
    let allow_empty = allow_empty.unwrap_or_default();
    for prop in node.props() {
        match prop {
            BaseElementProps::Attribute(prop) => {
                if dynamic_only {
                    continue;
                }
                if prop.name == name && (prop.value.is_some() || allow_empty) {
                    return Some(BaseElementProps::Attribute(prop.clone()));
                }
            }
            BaseElementProps::Directive(prop) => {
                if prop.name == "bind"
                    && (prop.exp.is_some() || allow_empty)
                    && is_static_arg_of(&prop.arg, name)
                {
                    return Some(BaseElementProps::Directive(prop.clone()));
                }
            }
        }
    }
    None
}

pub fn is_static_arg_of(arg: &Option<ExpressionNode>, name: &str) -> bool {
    let Some(arg) = arg else {
        return false;
    };
    if let ExpressionNode::Simple(arg) = arg
        && arg.is_static
        && arg.content == name
    {
        true
    } else {
        false
    }
}

#[inline]
pub fn is_text(type_: NodeTypes) -> bool {
    matches!(type_, NodeTypes::Text | NodeTypes::Interpolation)
}

pub fn inject_prop(node: &mut VNodeCall, prop: Property, context: &mut TransformContext) {
    // ObjectExpression | CallExpression | undefined;
    let mut props_with_injection = None::<PropsExpression>;
    // /**
    //  * 1. mergeProps(...)
    //  * 2. toHandlers(...)
    //  * 3. normalizeProps(...)
    //  * 4. normalizeProps(guardReactiveProps(...))
    //  *
    //  * we need to get the real props before normalization
    //  */
    // let props =
    //   node.type === NodeTypes.VNODE_CALL ? node.props : node.arguments[2]
    // let callPath: CallExpression[] = []
    // let parentCall: CallExpression | undefined
    // if (
    //   props &&
    //   !isString(props) &&
    //   props.type === NodeTypes.JS_CALL_EXPRESSION
    // ) {
    //   const ret = getUnnormalizedProps(props)
    //   props = ret[0]
    //   callPath = ret[1]
    //   parentCall = callPath[callPath.length - 1]
    // }

    if node.props.is_none() {
        props_with_injection = Some(PropsExpression::Object(ObjectExpression::new(
            vec![prop],
            None,
        )));
    }
    // } else if (props.type === NodeTypes.JS_CALL_EXPRESSION) {
    //   // merged props... add ours
    //   // only inject key to object literal if it's the first argument so that
    //   // if doesn't override user provided keys
    //   const first = props.arguments[0] as string | JSChildNode
    //   if (!isString(first) && first.type === NodeTypes.JS_OBJECT_EXPRESSION) {
    //     // #6631
    //     if (!hasProp(prop, first)) {
    //       first.properties.unshift(prop)
    //     }
    //   } else {
    //     if (props.callee === TO_HANDLERS) {
    //       // #2366
    //       propsWithInjection = createCallExpression(context.helper(MERGE_PROPS), [
    //         createObjectExpression([prop]),
    //         props,
    //       ])
    //     } else {
    //       props.arguments.unshift(createObjectExpression([prop]))
    //     }
    //   }
    //   !propsWithInjection && (propsWithInjection = props)
    // } else if (props.type === NodeTypes.JS_OBJECT_EXPRESSION) {
    //   if (!hasProp(prop, props)) {
    //     props.properties.unshift(prop)
    //   }
    //   propsWithInjection = props
    // } else {
    //   // single v-bind with expression, return a merged replacement
    //   propsWithInjection = createCallExpression(context.helper(MERGE_PROPS), [
    //     createObjectExpression([prop]),
    //     props,
    //   ])
    //   // in the case of nested helper call, e.g. `normalizeProps(guardReactiveProps(props))`,
    //   // it will be rewritten as `normalizeProps(mergeProps({ key: 0 }, props))`,
    //   // the `guardReactiveProps` will no longer be needed
    //   if (parentCall && parentCall.callee === GUARD_REACTIVE_PROPS) {
    //     parentCall = callPath[callPath.length - 2]
    //   }
    // }
    // if (node.type === NodeTypes.VNODE_CALL) {
    //   if (parentCall) {
    //     parentCall.arguments[0] = propsWithInjection
    //   } else {
    node.props = props_with_injection;
    //   }
    // } else {
    //   if (parentCall) {
    //     parentCall.arguments[0] = propsWithInjection
    //   } else {
    //     node.arguments[2] = propsWithInjection
    //   }
    // }
}

pub fn to_valid_asset_id(name: &String, type_: &AssetType) -> String {
    // see issue#4422, we need adding identifier on validAssetId if variable `name` has specific character
    let name: String = name
        .chars()
        .map(|c| {
            if !c.is_ascii_alphanumeric() && c != '_' {
                if c == '-' {
                    '_'.to_string()
                } else {
                    (c as u32).to_string()
                }
            } else {
                c.to_string()
            }
        })
        .collect();
    format!("_{}_{}", type_.to_string(), name)
}

/// forAliasRE: /([\s\S]*?)\s+(?:in|of)\s+(\S[\s\S]*)/
pub fn match_for_alias(text: &str) -> Option<(String, String)> {
    let mut in_text = text;
    let mut in_left = String::new();

    let mut of_text = text;
    let mut of_left = String::new();

    fn find(text: &mut &str, left_text: &mut String, pat: &str) -> Option<(String, String)> {
        let Some((left, right)) = text.split_once(pat) else {
            unreachable!();
        };

        if !left_text.is_empty() {
            left_text.push_str(pat);
        }
        left_text.push_str(left);

        if left.chars().last().is_some_and(|c| c.is_whitespace())
            && right.chars().next().is_some_and(|c| c.is_whitespace())
        {
            return Some((
                left_text.trim_end().to_string(),
                right.trim_start().to_string(),
            ));
        }
        *text = right;
        None
    }
    loop {
        let in_index = in_text.find("in");
        let of_index = of_text.find("of");
        let result = match (in_index, of_index) {
            (Some(in_index), Some(of_index)) => {
                if in_left.len() + in_index <= of_left.len() + of_index {
                    find(&mut in_text, &mut in_left, "in")
                } else {
                    find(&mut of_text, &mut of_left, "of")
                }
            }
            (Some(_), None) => find(&mut in_text, &mut in_left, "in"),
            (None, Some(_)) => find(&mut of_text, &mut of_left, "of"),
            (None, None) => {
                return None;
            }
        };
        if result.is_some() {
            return result;
        }
    }
}

pub fn is_all_whitespace(str: &str) -> bool {
    !str.chars().any(|c| !is_whitespace(c as u32))
}

/// Global compile-time constants
#[derive(Debug, Default, Clone, Copy)]
pub struct GlobalCompileTimeConstants {
    pub __dev__: bool,
    pub __test__: bool,
    pub __browser__: bool,
}

#[test]
fn test_match_for_alias() {
    assert!(match_for_alias("text").is_none());
    for pat in ["in", "of"] {
        assert_eq!(
            match_for_alias(&format!("a {pat} b")),
            Some(("a".to_string(), "b".to_string()))
        );
        assert_eq!(
            match_for_alias(&format!("a {pat} in b")),
            Some(("a".to_string(), "in b".to_string()))
        );
    }
}
