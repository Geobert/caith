#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, deny(broken_intra_doc_links))]
#![warn(missing_docs)]
#![warn(broken_intra_doc_links)]
//! A dice roller library written in Rust (and also a card drawer).
//!
//! This crate aims at providing everything needed for playing tabletop RPG.
//!
//! The different features are inspired by [DiceMaiden](https://github.com/Humblemonk/DiceMaiden)
//! and [Sidekick](https://github.com/ArtemGr/Sidekick).
//!
//! [Dìsle](https://github.com/Geobert/disle/) is a Discord bot build upon `caith`.
//!
//! # Usage
//!
//! ```
//! use caith::{Roller, RollResult, RollResultType};
//!
//! // ...
//! let result = Roller::new("1d6 : initiative").unwrap().roll().unwrap();
//! printf("{}", result);
//! ```
//!
//! # Syntax
//!
//! ```text
//! xdy [OPTIONS] [TARGET] [FAILURE] [! REASON]
//!
//! roll `x` dice(s) with `y` sides
//!
//! `y` can also be "F" or "f" for fudge dice. In this case, no option applies and ignored if provided.
//!
//! Options:
//! + - / * : modifiers
//! e# : Explode value. If number is omitted, we use dice sides
//! ie# or !# : Indefinite explode value, If number is omitted, we use dice sides
//! K#  : Keeping # highest (upperacse "K")
//! k#  : Keeping # lowest (lowercase "k")
//! D#  : Dropping the highest (uppercase "D")
//! d#  : Dropping the lowest (lowercase "d")
//! r#  : Reroll if <= value
//! ir# : Indefinite reroll if <= value
//!
//! Target:
//! t#  : minimum value to count as success
//! tt# : minimum value to count as two successes
//! t[<list of numbers>] : enumeration of values considered as success
//!
//! Failure:
//! f# : value under which it's counted as failure
//!
//! Repetition:
//! a roll can be repeated with `^` operator: `(2d6 + 6) ^ 8` will roll eight times the expression.
//!
//! Summed repetition:
//! with the `^+` operator, the roll will be repeated and all the totals summed.
//!
//! Sorted repetition:
//! with the `^#` operator, the roll will be repeated and sorted by total.
//!
//! Reason:
//! : : Any text after `:` will be a comment
//! ```
//!
//! # Helpers
//!
//! Some helpers are provided to interpret the roll result according to specific RPG rules.
//! See the helpers documentation for more details.
//!
//! You'll need to add the feature flag of the helpers that you need.
//!
//! At the moment, the supported feature flags are:
//! - `ova`: helper for "OVA: The Anime Role-Playing Game result"
//! - `cde`: helper for "Hong Kong, Les Chroniques de l'étrange"
//!
//! None is activated by default
//!
//! # Cards
//!
//! `caith` can create a standard deck of 52 cards plus optional Jokers if the feature `cards`
//! is activated. See [`cards::Deck`].
//!
//! # Examples
//!
//! These examples are directly taken from DiceMaiden's Readme:
//!
//! `2d6 + 3d10` : Roll two six-sided dice and three ten-sided dice.
//!
//! `3d6 + 5` : Roll three six-sided dice and add five. Other supported static modifiers are
//! add (+), subtract (-), multiply (*), and divide (/).
//!
//! `3d6 * 1.5` : Roll three six-sided dice and add 50%.
//!
//! `3d6 e6` : Roll three six-sided dice and explode on sixes. Some game systems call this 'open
//! ended' dice. If the number rolled is greater than or equal to the value given for this option,
//! the die is rolled again and added to the total. If no number is given for this option, it is
//! assumed to be the same as the number of sides on the die. Thus, '3d6 e' is the same as '3d6 e6'.
//! The dice will only explode once with this command. Use `ie` for indefinite explosions.
//!
//! `3d6 ie6` or `3d6!` : Roll three six-sided dice and explode on sixes indefinitely within reason.
//! We will cap explosions at 100 rolls to prevent abuse.
//!
//! `3d10 d1` : Roll three ten-sided dice and drop one die. The lowest value will be dropped first.  
//!
//! `3d10 K2` : Roll three ten-sided dice and keep two. The highest value rolled will be kept.
//! Using lowercase `k` will keep the lowest.
//!
//! `4d6 r2` : Roll four six-sided dice and reroll any that are equal to or less than two once.
//! Use `ir` for indefinite rerolls.
//!
//! `4d6 ir2` : Roll four six-sided dice and reroll any that are equal to or less than two (and do
//! the same to those dice). This is capped at 100 rerolls per die to prevent abuse.
//!
//! `6d10 t7` : Roll six ten-sided dice and any that are seven or higher are counted as a success.
//! The dice in the roll are not added together for a total. Any die that meets or exceeds the
//! target number is added to a total of successes.
//!
//! `5d10 t8 f1` : f# denotes a failure number that each dice must match or be beneath in order to
//! count against successes. These work as a sort of negative success and are totalled together as
//! described above. In the example roll, roll five ten-sided dice and each dice that is 8 or higher
//! is a success and subtract each one. The total may be negative. If the option is given a 0 value,
//! that is the same as not having the option at all thus a normal sum of all dice in the roll is
//! performed instead.
//!
//! `5d10 t8 tt10` : 8 and 9 are counted as success, 10 are counted twice.
//!
//! `3d6 t[2,4,6]` : only even result will count as success (handy for games like "Knight").
//!
//! `4d10 k3` : Roll four ten-sided dice and keep the lowest three dice rolled.
//!
//! `4d6 : Hello World!`: Roll four six-sided dice and add comment to the roll.
//!
//! These commands can be combined. For example:
//!
//! `10d6 e6 K8 +4` : Roll ten six-sided dice , explode on sixes and keep eight of the highest rolls
//! and add four.
//!

use pest::{
    iterators::{Pair, Pairs},
    Parser,
};

pub mod helpers;

mod error;
mod parser;
mod rollresult;

#[cfg(feature = "cards")]
#[cfg_attr(docsrs, doc(cfg(feature = "cards")))]
pub mod cards;

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
        self.rng.gen_range(1..1 + sides)
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
