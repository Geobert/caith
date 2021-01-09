#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, deny(broken_intra_doc_links))]
#![warn(missing_docs)]
#![warn(broken_intra_doc_links)]
//! `caith` is a dice roll expression parser and roller.
//!
//! See [README.md](https://github.com/Geobert/caith/blob/master/README.md) for more details

use pest::{
    iterators::{Pair, Pairs},
    Parser,
};

pub mod helpers;

mod error;
mod parser;
mod rollresult;

pub use error::*;
pub use rollresult::*;

use parser::{DiceRollSource, RollParser, Rule};
use rand::Rng;

const REASON_CHAR: char = ':';

/// An object holding the query.
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

struct RngDiceRollSource<'a, T>
where
    T: Rng,
{
    rng: &'a mut T,
}

impl<T> DiceRollSource for RngDiceRollSource<'_, T>
where
    T: Rng,
{
    fn roll_single_die(&mut self, sides: u64) -> u64 {
        self.rng.gen_range(1, 1 + sides)
    }
}

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
        self.roll_with_source(&mut RngDiceRollSource { rng })
    }

    /// Evaluate and roll the dice with provided dice roll source
    pub fn roll_with_source<RNG: DiceRollSource>(&self, rng: &mut RNG) -> Result<RollResult> {
        let mut pairs = RollParser::parse(Rule::command, &self.0)?;
        let expr_type = pairs.next().unwrap();
        let mut roll_res = match expr_type.as_rule() {
            Rule::expr => RollResult::new_single(parser::compute(expr_type.into_inner(), rng)?),
            Rule::repeated_expr => Roller::process_repeated_expr(expr_type, rng)?,
            _ => unreachable!(),
        };

        if let Some(reason) = pairs.next() {
            if reason.as_rule() == Rule::reason {
                roll_res.add_reason(reason.as_str()[1..].trim().to_owned());
            }
        }
        Ok(roll_res)
    }

    fn process_repeated_expr<RNG: DiceRollSource>(
        expr_type: Pair<Rule>,
        rng: &mut RNG,
    ) -> Result<RollResult> {
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
                results.sort_unstable_by(|a, b| a.get_total().partial_cmp(&b.get_total()).unwrap());
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

/// Iterator that lazily returns each dice of the expression.
///
/// See [`Roller::dices()`] for example
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

    pub(crate) struct IteratorDiceRollSource<'a, T>
    where
        T: Iterator<Item = u64>,
    {
        pub iterator: &'a mut T,
    }

    impl<T> DiceRollSource for IteratorDiceRollSource<'_, T>
    where
        T: Iterator<Item = u64>,
    {
        fn roll_single_die(&mut self, sides: u64) -> u64 {
            match self.iterator.next() {
                Some(value) => {
                    if value > sides {
                        panic!("Tried to return {} for a {} sided dice", value, sides)
                    }
                    println!("Dice {}", value);
                    value
                }
                None => panic!("Iterator out of values"),
            }
        }
    }

    #[test]
    fn get_repeat_test() {
        let r = Roller::new("(2d6 + 6) ^ 8 : test").unwrap();
        let roll_mock = vec![3, 5, 3, 5, 3, 5, 3, 5, 3, 5, 3, 5, 3, 5, 3, 5];
        let roll_res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut roll_mock.into_iter(),
            })
            .unwrap();
        match roll_res.get_result() {
            rollresult::RollResultType::Single(_) => unreachable!(),
            rollresult::RollResultType::Repeated(rep) => {
                assert_eq!(8, rep.len());
                for res in rep.iter() {
                    assert_eq!(14, res.get_total());
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
        let roll_mock = vec![3, 5, 1, 1, 6, 5, 3, 5, 4, 5, 2, 4, 3, 5, 1, 2];
        let mut expected = roll_mock
            .as_slice()
            .chunks(2)
            .map(|two| two[0] as i64 + two[1] as i64 + 6)
            .collect::<Vec<_>>();
        expected.sort_unstable();
        let roll_res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut roll_mock.into_iter(),
            })
            .unwrap();
        match roll_res.get_result() {
            rollresult::RollResultType::Single(_) => unreachable!(),
            rollresult::RollResultType::Repeated(rep) => {
                assert_eq!(8, rep.len());

                let res_vec = rep.iter().map(|r| r.get_total()).collect::<Vec<_>>();
                assert_eq!(expected, res_vec);
            }
        };
        eprintln!("{}", roll_res);
    }

    #[test]
    fn get_repeat_sum_test() {
        let r = Roller::new("(2d6 + 6) ^+ 2 : test").unwrap();
        let roll_mock = vec![3, 5, 4, 2];
        let expected = roll_mock
            .as_slice()
            .chunks(2)
            .map(|two| two[0] as i64 + two[1] as i64 + 6)
            .collect::<Vec<_>>();
        let expected: i64 = expected.iter().sum();
        let roll_res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut roll_mock.into_iter(),
            })
            .unwrap();
        match roll_res.get_result() {
            rollresult::RollResultType::Single(_) => unreachable!(),
            rollresult::RollResultType::Repeated(rep) => {
                assert_eq!(2, rep.len());
                assert_eq!(expected, rep.get_total().unwrap());
            }
        }
        eprintln!();
        eprintln!("{}", roll_res);
    }

    #[test]
    fn get_single_test() {
        let r = Roller::new("2d6 + 6 : test").unwrap();
        let roll_mock = vec![3, 5];
        let expected = roll_mock
            .as_slice()
            .chunks(2)
            .map(|two| two[0] as i64 + two[1] as i64)
            .collect::<Vec<_>>();
        let expected = expected.iter().sum::<i64>() + 6;
        let roll_res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut roll_mock.into_iter(),
            })
            .unwrap();
        match roll_res.get_result() {
            rollresult::RollResultType::Single(res) => assert_eq!(expected, res.get_total()),
            rollresult::RollResultType::Repeated(_) => unreachable!(),
        }
        eprintln!();
        eprintln!("{}", roll_res.as_single().unwrap());
    }

    #[test]
    fn one_value_test() {
        let r = Roller::new("20").unwrap();
        let res = r.roll().unwrap();
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            assert_eq!(20, res.get_total());
        } else {
            assert!(false);
        }
    }

    #[test]
    fn one_dice_test() {
        let r = Roller::new("d20").unwrap();
        let roll_mock = vec![8];
        let res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut roll_mock.into_iter(),
            })
            .unwrap();
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            assert_eq!(8, res.get_total());
        } else {
            unreachable!();
        }
    }

    #[test]
    fn float_mul_test() {
        let r = Roller::new("20 * 1.5").unwrap();
        let res = r.roll().unwrap();
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            assert_eq!(30, res.get_total());
        } else {
            unreachable!()
        }
    }

    #[test]
    fn float_add_test() {
        let r = Roller::new("20 + 1.5").unwrap();
        let res = r.roll().unwrap();
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            assert_eq!(21, res.get_total());
        } else {
            unreachable!()
        }
    }

    #[test]
    fn counting_roller_test() {
        let r = Roller::new("3d6").unwrap();
        let rolls = vec![3, 6, 3];
        let res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut rolls.into_iter(),
            })
            .unwrap();
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            assert_eq!(res.get_total(), 12);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn target_number_test() {
        let r = Roller::new("10d10 t7").unwrap();
        let res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut (1..11),
            })
            .unwrap();
        println!("{}", res);
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            // We rolled one of every number, with a target number of 7 we should score a success
            // on the 7, 8, 9, and 10. So four total.
            assert_eq!(res.get_total(), 4);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn target_number_double_test() {
        let r = Roller::new("10d10 t7 tt9").unwrap();
        let res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut (1..11),
            })
            .unwrap();
        println!("{}", res);
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            // We rolled one of every number. That's a success each for the 7 and 8, and two
            // success each for the 9 and 10. So a toal of six.
            assert_eq!(res.get_total(), 6);
        } else {
            assert!(false);
        }
    }

    // Where a user has asked for a doubles threashold that is lower than the single threashold,
    // the single threashold is ignored.
    #[test]
    fn target_number_double_lower_than_target_test() {
        let r = Roller::new("10d10 tt7 t9").unwrap();
        let res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut (1..11),
            })
            .unwrap();
        println!("{}", res);
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            // We rolled one of every number. That's two successes each for the 7, 8, 9, and 10.
            // So eight total.
            assert_eq!(res.get_total(), 8);
        } else {
            assert!(false);
        }
    }

    // Where a user has asked for a doubles without singles.
    #[test]
    fn target_number_double_only() {
        let r = Roller::new("10d10 tt8").unwrap();
        let res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut (1..11),
            })
            .unwrap();
        println!("{}", res);
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            // We rolled one of every number. That's two successes each for the 8, 9, and 10.
            // So six total.
            assert_eq!(res.get_total(), 6);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn target_enum() {
        let r = Roller::new("6d6 t[2,4,6]").unwrap();
        let res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut (1..7),
            })
            .unwrap();
        println!("{}", res);
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            // We rolled one of every number. That's half of them being even
            assert_eq!(res.get_total(), 3);
        } else {
            assert!(false);
        }

        let mock = vec![1, 2, 2, 4, 6, 3];
        let res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut mock.into_iter(),
            })
            .unwrap();
        println!("{}", res);
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            // We rolled one of every number. That's half of them being even
            assert_eq!(res.get_total(), 4);
        } else {
            assert!(false);
        }

        let mock = vec![1, 3, 3, 4, 6, 3];
        let res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut mock.into_iter(),
            })
            .unwrap();
        println!("{}", res);
        let res = res.get_result();
        if let RollResultType::Single(res) = res {
            // We rolled one of every number. That's half of them being even
            assert_eq!(res.get_total(), 2);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn sandbox_test() {
        let r = Roller::new("5d6 t[2,4,6]").unwrap();
        r.dices()
            .expect("Error while parsing")
            .for_each(|d| eprintln!("{}", d));

        eprintln!("{}\n{}", r.as_str(), r.roll().unwrap());
    }
}
