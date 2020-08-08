use pest::Parser;

mod error;
mod parser;
mod rollresult;
pub use error::*;

use parser::{compute, RollParser, Rule};
pub use rollresult::RollResult;

/// Compute a roll expression
pub fn roll(input: &str) -> Result<RollResult> {
    let mut pairs = RollParser::parse(Rule::command, input)?;
    let mut roll_res = compute(pairs.next().unwrap().into_inner())?;
    if let Some(reason) = pairs.next() {
        if reason.as_rule() == Rule::reason {
            roll_res.add_reason(reason.as_str()[1..].trim().to_owned());
        }
    }
    Ok(roll_res)
}

/// Look for the first dice in expression and return it
pub fn find_first_dice(input: &str) -> Result<String> {
    let mut pairs = RollParser::parse(Rule::command, input)?;
    parser::find_first_dice(pairs.next().unwrap().into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_test() {
        eprintln!("{}", roll("4dF ! pouet").unwrap());
    }
}
