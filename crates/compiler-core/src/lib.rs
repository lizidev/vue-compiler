mod ast;
mod codegen;
mod compile;
mod errors;
mod options;
mod parser;
mod runtime_helpers;
mod tokenizer;
mod transform;
mod transforms;
mod utils;

pub use compile::base_compile;

pub use ast::*;

// Also expose lower level APIs & types
pub use crate::codegen::{CodegenResult, generate};
pub use crate::compile::BaseCompileSource;
pub use crate::errors::{CompilerError, ErrorCodes};
pub use crate::options::{
    CodegenMode, CodegenOptions, CompilerOptions, ErrorHandlingOptions, ParserOptions,
    TransformOptions,
};
pub use crate::parser::base_parse;
pub use crate::runtime_helpers::*;
pub use crate::tokenizer::ParseMode;
pub use crate::transform::transform;
pub use crate::transforms::{
    transform_element::transform_element,
    // transform_v_bind_shorthand::TransformVBindShorthand,
    v_if::transform_if,
};
pub use crate::utils::GlobalCompileTimeConstants;
