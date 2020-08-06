use std::{error::Error, fmt::Display};

use crate::parser::*;

pub type Result<T> = std::result::Result<T, RollError>;

#[derive(Debug)]
pub enum RollError {
    ParseError(pest::error::Error<Rule>),
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
