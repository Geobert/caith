use pest::{iterators::Pairs, Parser};
use rollresult::RollResult;

mod error;
mod parser;
mod rollresult;
pub use error::*;

use parser::{RollParser, Rule};
pub use rollresult::SingleRollResult;

const REASON_CHAR: char = ':';

/// An object holding the query
///
/// It has no advantage compare to free function that would take `&str` as parameter (like previous
/// version) but it provides a stable API for the day `pest` can have a `Send` type as the parse
/// result.
///
/// see [Pest's issue](https://github.com/pest-parser/pest/issues/472)
/// and [Forum topic](https://users.rust-lang.org/t/how-to-deal-with-external-type-which-is-send-and-sync/47530)
///
#[derive(Clone)]
pub struct Roller(String);

impl Roller {
    /// Store the input
    ///
    /// At version 1.0.0, it always returns Ok(Self).
    ///
    /// This is to have a stable API for further optimization where the parsing is done here (so it
    /// can fail) and saved, see `Roller` documentation above.
    ///
    pub fn new(input: &str) -> Result<Self> {
        Ok(Roller(input.to_owned()))
    }

    /// Evaluate and roll the dices
    pub fn roll(&self) -> Result<RollResult> {
        let mut pairs = RollParser::parse(Rule::command, &self.0)?;
        let expr_type = pairs.next().unwrap();
        let mut roll_res = match expr_type.as_rule() {
            Rule::expr => RollResult::new_single(parser::compute(expr_type.into_inner())?),
            Rule::repeated_expr => {
                let mut pairs = expr_type.into_inner();
                let expr = pairs.next().unwrap();
                let number = pairs.next().unwrap().as_str().parse::<i64>().unwrap();
                if number <= 0 {
                    return Err("Can't repeat 0 times or negatively".into());
                } else {
                    let results: Result<Vec<SingleRollResult>> =
                        (0..number).try_fold(Vec::new(), |mut res, _| {
                            let c = parser::compute(expr.clone().into_inner())?;
                            res.push(c);
                            Ok(res)
                        });
                    RollResult::new_repeated(results?)
                }
            }
            _ => unreachable!(),
        };

        if let Some(reason) = pairs.next() {
            if reason.as_rule() == Rule::reason {
                roll_res.add_reason(reason.as_str()[1..].trim().to_owned());
            }
        }
        Ok(roll_res)
    }

    /// Get an iterator on the dices in the expression
    ///
    /// # Examples
    ///
    /// ```
    /// use caith::Roller;
    ///
    /// let r = Roller::new("1d6 + 1d4 + 1d10 + 1d20").unwrap();
    /// assert_eq!(vec!["1d6", "1d4", "1d10", "1d20"], r.dices().expect("Error on parse").collect::<Vec<_>>());
    /// ```
    pub fn dices(&self) -> Result<Dices> {
        let pairs = RollParser::parse(Rule::command, &self.0)?
            .next()
            .unwrap()
            .into_inner();
        Ok(Dices { pairs })
    }

    /// Give back the query string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Removes the reason from the Roller
    pub fn trim_reason(&mut self) {
        if let Some(idx) = self.0.find(REASON_CHAR) {
            self.0 = self.0[..idx].to_owned()
        }
    }
}

/// Iterator that lazily returns each dice of the expression
///
/// See `Roller::dices()` for example
///
pub struct Dices<'a> {
    pairs: Pairs<'a, Rule>,
}

impl<'a> Iterator for Dices<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        parser::find_first_dice(&mut self.pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_test() {
        let r = Roller::new("(2d6 + 6) ^ 8 : test").unwrap();
        r.dices()
            .expect("Error while parsing")
            .for_each(|d| eprintln!("{}", d));

        eprintln!("{}\n{}", r.as_str(), r.roll().unwrap());
    }
}
