mod parser_options;

use vue_compiler_core::{ParserOptions, RootNode, base_parse};

pub use crate::parser_options::parser_options;

pub fn parse(template: &str, options: Option<ParserOptions>) -> RootNode {
    base_parse(template, options)
}
