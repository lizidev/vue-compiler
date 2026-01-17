use crate::ast::ObjectExpression;

#[derive(Debug, PartialEq, Clone)]
pub enum PropsExpression {
    Object(ObjectExpression),
    // ObjectExpression | CallExpression | ExpressionNode
}
