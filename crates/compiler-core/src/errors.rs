use crate::{ast::SourceLocation, options::ErrorHandlingOptions};

#[derive(Debug)]
pub struct CompilerError {
    pub message: String,
    pub code: ErrorCodes,
    pub loc: Option<SourceLocation>,
}

impl PartialEq for CompilerError {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code && self.loc == other.loc
    }
}

impl CompilerError {
    pub fn new(code: ErrorCodes, loc: Option<SourceLocation>) -> Self {
        // const msg =
        //   __DEV__ || !__BROWSER__
        //     ? (messages || errorMessages)[code] + (additionalMessage || ``)
        //     : `https://vuejs.org/error-reference/#compiler-${code}`
        // const error = new SyntaxError(String(msg)) as InferCompilerError<T>
        Self {
            code,
            loc,
            message: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct DefaultErrorHandlingOptions;

impl ErrorHandlingOptions for DefaultErrorHandlingOptions {}

#[derive(Debug, PartialEq)]
pub enum ErrorCodes {
    // parse errors
    // ABRUPT_CLOSING_OF_EMPTY_COMMENT,
    CdataInHtmlContent,
    DuplicateAttribute,
    // END_TAG_WITH_ATTRIBUTES,
    // END_TAG_WITH_TRAILING_SOLIDUS,
    EOFBeforeTagName,
    EOFInCdata,
    EOFInComment,
    // EOF_IN_SCRIPT_HTML_COMMENT_LIKE_TEXT,
    EOFInTag,
    // INCORRECTLY_CLOSED_COMMENT,
    // INCORRECTLY_OPENED_COMMENT,
    // INVALID_FIRST_CHARACTER_OF_TAG_NAME,
    MissingAttributeValue,
    MissingEndTagName,
    // MISSING_WHITESPACE_BETWEEN_ATTRIBUTES,
    // NESTED_COMMENT,
    UnexpectedCharacterInAttributeName,
    UnexpectedCharacterInUnquotedAttributeValue,
    UnexpectedEqualsSignBeforeAttributeName,
    // UNEXPECTED_NULL_CHARACTER,
    UnexpectedQuestionMarkInsteadOfTagName,
    UnexpectedSolidusInTag,

    // Vue-specific parse errors
    XInvalidEndTag,
    XMissingEndTag,
    XMissingInterpolationEnd,
    XMissingDirectiveName,
    XMissingDynamicDirectiveArgumentEnd,
    // // transform errors
    // X_V_IF_NO_EXPRESSION,
    // X_V_IF_SAME_KEY,
    // X_V_ELSE_NO_ADJACENT_IF,
    // X_V_FOR_NO_EXPRESSION,
    // X_V_FOR_MALFORMED_EXPRESSION,
    // X_V_FOR_TEMPLATE_KEY_PLACEMENT,
    // X_V_BIND_NO_EXPRESSION,
    // X_V_ON_NO_EXPRESSION,
    // X_V_SLOT_UNEXPECTED_DIRECTIVE_ON_SLOT_OUTLET,
    // X_V_SLOT_MIXED_SLOT_USAGE,
    // X_V_SLOT_DUPLICATE_SLOT_NAMES,
    // X_V_SLOT_EXTRANEOUS_DEFAULT_SLOT_CHILDREN,
    // X_V_SLOT_MISPLACED,
    // X_V_MODEL_NO_EXPRESSION,
    // X_V_MODEL_MALFORMED_EXPRESSION,
    // X_V_MODEL_ON_SCOPE_VARIABLE,
    // X_V_MODEL_ON_PROPS,
    // X_V_MODEL_ON_CONST,
    // X_INVALID_EXPRESSION,
    // X_KEEP_ALIVE_INVALID_CHILDREN,

    // // generic errors
    // X_PREFIX_ID_NOT_SUPPORTED,
    // X_MODULE_MODE_NOT_SUPPORTED,
    // X_CACHE_HANDLER_NOT_SUPPORTED,
    // X_SCOPE_ID_NOT_SUPPORTED,
    // X_VNODE_HOOKS,

    // // placed here to preserve order for the current minor
    // // TODO adjust order in 3.5
    // X_V_BIND_INVALID_SAME_NAME_ARGUMENT,

    // // Special value for higher-order compilers to pick up the last code
    // // to avoid collision of error codes. This should always be kept as the last
    // // item.
    // __EXTEND_POINT__,
}
