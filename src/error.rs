use std::{error::Error, fmt::Display};

use crate::parser::*;

/// Result type used accross the library
pub type Result<T> = std::result::Result<T, RollError>;

/// The error reported
#[derive(Debug)]
pub enum RollError {
    /// Error while parsing the expression, emitted by `pest`
    ParseError(pest::error::Error<Rule>),
    /// Any other error while walking the AST, the String contains an explaination of what happened
    ParamError(String),
}

impl Display for RollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RollError::ParseError(e) => write!(f, "{}", e),
            RollError::ParamError(e) => write!(f, "{}", e),
        }
    }
}

impl Error for RollError {}

impl From<pest::error::Error<Rule>> for RollError {
    fn from(e: pest::error::Error<Rule>) -> Self {
        RollError::ParseError(e)
    }
}

impl From<&str> for RollError {
    fn from(e: &str) -> Self {
        RollError::ParamError(e.to_string())
    }
}

impl From<String> for RollError {
    fn from(e: String) -> Self {
        Self::ParamError(e)
    }
}
