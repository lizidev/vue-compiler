use crate::ast::BaseElementProps;

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

/// Global compile-time constants
#[derive(Debug, Default, Clone, Copy)]
pub struct GlobalCompileTimeConstants {
    pub __dev__: bool,
    pub __test__: bool,
    pub __browser__: bool,
}
