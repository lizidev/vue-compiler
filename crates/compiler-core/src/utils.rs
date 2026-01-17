use crate::{ast::BaseElementProps, codegen::AssetType, tokenizer::is_whitespace};

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
