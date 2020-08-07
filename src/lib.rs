use pest::Parser;

mod error;
mod parser;
mod rollresult;
pub use error::*;

use parser::{compute, RollParser, Rule};
pub use rollresult::RollResult;

/// Execute a roll command
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_test() {
        eprintln!("{}", roll("4dF ! pouet").unwrap());
    }
}
