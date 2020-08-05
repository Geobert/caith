use std::{error::Error, fmt::Display};

use crate::parser::*;

pub type Result<T> = std::result::Result<T, RollError>;

#[derive(Debug)]
pub enum RollError {
    ParseError(pest::error::Error<Rule>),
}

impl Display for RollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl Error for RollError {}

impl From<pest::error::Error<Rule>> for RollError {
    fn from(e: pest::error::Error<Rule>) -> Self {
        RollError::ParseError(e)
    }
}
