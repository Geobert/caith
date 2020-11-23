#![warn(missing_docs)]
#![warn(broken_intra_doc_links)]
//! `caith` is a dice roll expression parser and roller.
//!
//! See [README.md](https://github.com/Geobert/caith/blob/master/README.md) for more details

use pest::{
    iterators::{Pair, Pairs},
    Parser,
};

mod error;
mod parser;
mod rollresult;
pub use error::*;

use parser::{RollParser, Rule};
use rand::Rng;
pub use rollresult::*;

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
#[derive(Clone, Debug)]
pub struct Roller(String);

impl Roller {
    /// Store the input
    ///
    /// As of version 2.0.0, it always returns `Ok(Self)`.
    ///
    /// This is to have a stable API for further optimization where the parsing is done here (so it
    /// can fail) and saved, see `Roller` documentation above.
    ///
    pub fn new(input: &str) -> Result<Self> {
        Ok(Roller(input.to_owned()))
    }

    /// Evaluate and roll the dices with default Rng source (`rand::thread_rng()`)
    pub fn roll(&self) -> Result<RollResult> {
        self.roll_with(&mut rand::thread_rng())
    }

    /// Evaluate and roll the dices with provided rng source
    pub fn roll_with<RNG: Rng>(&self, rng: &mut RNG) -> Result<RollResult> {
        let mut pairs = RollParser::parse(Rule::command, &self.0)?;
        let expr_type = pairs.next().unwrap();
        let mut roll_res = match expr_type.as_rule() {
            Rule::expr => RollResult::new_single(parser::compute(expr_type.into_inner(), rng)?),
            Rule::repeated_expr => Roller::process_repeated_expr(expr_type, rng)?,
            Rule::ova => {
                let mut pairs = expr_type.into_inner();
                let number = pairs.next().unwrap().as_str().parse::<i64>().unwrap();
                if number == 0 {
                    return Err("Can't roll 0 dices".into());
                } else {
                    let res = parser::roll_dice(number.abs() as u64, 6, rng);
                    Roller::compute_ova(res, number)
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

    fn process_repeated_expr<RNG: Rng>(expr_type: Pair<Rule>, rng: &mut RNG) -> Result<RollResult> {
        let mut pairs = expr_type.into_inner();
        let expr = pairs.next().unwrap();
        let maybe_option = pairs.next().unwrap();
        let (number, sum_all, sort) = match maybe_option.as_rule() {
            Rule::number => (maybe_option.as_str().parse::<i64>().unwrap(), false, false),
            Rule::add => (
                pairs.next().unwrap().as_str().parse::<i64>().unwrap(),
                true,
                false,
            ),
            Rule::sort => (
                pairs.next().unwrap().as_str().parse::<i64>().unwrap(),
                false,
                true,
            ),
            _ => unreachable!(),
        };
        if number <= 0 {
            Err("Can't repeat 0 times or negatively".into())
        } else {
            let results: Result<Vec<SingleRollResult>> =
                (0..number).try_fold(Vec::new(), |mut res, _| {
                    let c = parser::compute(expr.clone().into_inner(), rng)?;
                    res.push(c);
                    Ok(res)
                });
            let mut results = results?;
            if sort {
                results.sort_unstable_by(|a, b| a.get_total().cmp(&b.get_total()));
            }
            let total = if sum_all {
                Some(
                    results
                        .iter()
                        .fold(0, |acc, current| acc + current.get_total()),
                )
            } else {
                None
            };
            Ok(RollResult::new_repeated(results, total))
        }
    }

    fn compute_ova(mut res: Vec<DiceResult>, number: i64) -> RollResult {
        res.sort_unstable();
        let total = if number > 0 {
            let mut last_side = 0;
            let mut current_res = 0;
            res.iter().fold(0, |acc, current| {
                if last_side != current.res {
                    last_side = current.res;
                    if acc > current_res {
                        current_res = acc;
                    }
                    current.res
                } else {
                    acc + current.res
                }
            });
            current_res
        } else {
            res.first()
                .expect("Impossible, that mean we rolled 0 dices")
                .res
        };

        RollResult::new_single(SingleRollResult::new_ova(total, res))
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
    fn get_repeat_test() {
        let r = Roller::new("(2d6 + 6) ^ 8 : test").unwrap();
        let roll_res = r.roll().unwrap();
        match roll_res.get_result() {
            rollresult::RollResultType::Single(_) => unreachable!(),
            rollresult::RollResultType::Repeated(rep) => {
                for res in rep.iter() {
                    eprintln!("{}", res)
                }
            }
        }
        eprintln!();
        for res in roll_res.as_repeated().unwrap().iter() {
            eprintln!("{}", res)
        }

        eprintln!();
        eprintln!("{}", roll_res);
    }

    #[test]
    fn get_repeat_sort_test() {
        let r = Roller::new("(2d6 + 6) ^# 8 : test").unwrap();
        let roll_res = r.roll().unwrap();

        eprintln!("{}", roll_res);
    }

    #[test]
    fn get_repeat_sum_test() {
        let r = Roller::new("(2d6 + 6) ^+ 2 : test").unwrap();
        let roll_res = r.roll().unwrap();
        match roll_res.get_result() {
            rollresult::RollResultType::Single(_) => unreachable!(),
            rollresult::RollResultType::Repeated(rep) => {
                for res in rep.iter() {
                    eprintln!("{}", res)
                }
            }
        }
        eprintln!();
        // for res in roll_res.as_repeated().unwrap().iter() {
        //     eprintln!("{}", res)
        // }
        eprintln!("{}", roll_res);
    }

    #[test]
    fn get_single_test() {
        let r = Roller::new("2d6 + 6 : test").unwrap();
        let roll_res = r.roll().unwrap();
        match roll_res.get_result() {
            rollresult::RollResultType::Single(res) => eprintln!("{}", res),
            rollresult::RollResultType::Repeated(_) => unreachable!(),
        }
        eprintln!();
        eprintln!("{}", roll_res.as_single().unwrap());
    }

    #[test]
    fn ova_test() {
        let res = vec![1, 1, 2, 4, 4, 4, 4, 5, 5, 5, 6]
            .into_iter()
            .map(|i| DiceResult::new(i as u64, 6))
            .collect();
        assert_eq!(
            16,
            Roller::compute_ova(res, 1).as_single().unwrap().get_total()
        );

        let res = vec![1, 1, 2, 5, 5, 5, 6]
            .into_iter()
            .map(|i| DiceResult::new(i as u64, 6))
            .collect();
        assert_eq!(
            1,
            Roller::compute_ova(res, -1)
                .as_single()
                .unwrap()
                .get_total()
        );

        let r = Roller::new("ova(12)").unwrap();
        eprintln!("{}", r.roll().unwrap());

        let r = Roller::new("ova(-5)").unwrap();
        eprintln!("{}", r.roll().unwrap());
    }

    #[test]
    fn sandbox_test() {
        let r = Roller::new("10d6 e3 e3 + 4").unwrap();
        r.dices()
            .expect("Error while parsing")
            .for_each(|d| eprintln!("{}", d));

        eprintln!("{}\n{}", r.as_str(), r.roll().unwrap());
    }
}
