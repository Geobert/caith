use std::sync::{Arc, Once, RwLock};

use pest::{
    iterators::{Pair, Pairs},
    prec_climber::{Assoc, Operator, PrecClimber},
};
use pest_derive::Parser;
use rand::{thread_rng, Rng};

use crate::{error::Result, SingleRollResult};

#[derive(Parser)]
#[grammar = "caith.pest"]
pub(crate) struct RollParser;

// number represent nb dice to keep/drop
#[derive(Copy, Clone)]
pub(crate) enum TotalModifier {
    KeepHi(usize),
    KeepLo(usize),
    DropHi(usize),
    DropLo(usize),
    TargetFailure(u64, u64),
    Fudge,
    None,
}

struct OptionResult {
    res: Vec<u64>,
    modifier: TotalModifier,
}

// Struct to have a singleton of PrecClimber without using once_cell
#[derive(Clone)]
struct Climber {
    inner: Arc<RwLock<PrecClimber<Rule>>>,
}

impl Climber {
    fn climb<'i, P, F, G, T>(&self, pairs: P, primary: F, infix: G) -> T
    where
        P: Iterator<Item = Pair<'i, Rule>>,
        F: FnMut(Pair<'i, Rule>) -> T,
        G: FnMut(T, Pair<'i, Rule>, T) -> T,
    {
        self.inner.read().unwrap().climb(pairs, primary, infix)
    }
}

fn get_climber() -> Climber {
    static mut PREC_CLIMBER: *const Climber = 0 as *const Climber;
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            use self::Assoc::*;
            use self::Rule::*;

            // Make it
            let singleton = Climber {
                inner: Arc::new(RwLock::new(PrecClimber::new(vec![
                    Operator::new(add, Left) | Operator::new(sub, Left),
                    Operator::new(mul, Left) | Operator::new(div, Left),
                ]))),
            };

            // Put it in the heap so it can outlive this call
            PREC_CLIMBER = std::mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*PREC_CLIMBER).clone()
    }
}

fn compute_explode(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<u64>,
    option: Pair<Rule>,
) -> (TotalModifier, Vec<u64>) {
    let value = extract_option_value(option).unwrap_or(sides);
    let nb = res.iter().filter(|x| **x >= value).count() as u64;
    rolls.add_history(res.clone(), false);
    let res = if nb > 0 {
        let res = roll_dice(nb, sides);
        rolls.add_history(res.clone(), false);
        res
    } else {
        res
    };
    (TotalModifier::None, res)
}

fn compute_i_explode(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<u64>,
    option: Pair<Rule>,
) -> (TotalModifier, Vec<u64>) {
    let value = extract_option_value(option).unwrap_or(sides);
    rolls.add_history(res.clone(), false);
    let mut nb = res.into_iter().filter(|x| *x >= value).count() as u64;
    let mut res = Vec::new();
    while nb > 0 {
        res = roll_dice(nb, sides);
        nb = res.iter().filter(|x| **x >= value).count() as u64;
        rolls.add_history(res.clone(), false);
    }
    (TotalModifier::None, res)
}

fn compute_reroll(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<u64>,
    option: Pair<Rule>,
) -> (TotalModifier, Vec<u64>) {
    let value = extract_option_value(option).unwrap();
    let mut has_rerolled = false;
    let res: Vec<u64> = res
        .into_iter()
        .map(|x| {
            if x <= value {
                has_rerolled = true;
                roll_dice(1, sides)[0]
            } else {
                x
            }
        })
        .collect();

    if has_rerolled {
        rolls.add_history(res.clone(), false);
    }
    (TotalModifier::None, res)
}

fn compute_i_reroll(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<u64>,
    option: Pair<Rule>,
) -> (TotalModifier, Vec<u64>) {
    let value = extract_option_value(option).unwrap();
    let mut has_rerolled = false;
    let res: Vec<u64> = res
        .into_iter()
        .map(|x| {
            let mut x = x;
            while x <= value {
                has_rerolled = true;
                x = roll_dice(1, sides)[0]
            }
            x
        })
        .collect();

    if has_rerolled {
        rolls.add_history(res.clone(), false);
    }
    (TotalModifier::None, res)
}

fn compute_option(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<u64>,
    option: Pair<Rule>,
) -> Result<OptionResult> {
    let (modifier, mut res) = match option.as_rule() {
        Rule::explode => compute_explode(rolls, sides, res, option),
        Rule::i_explode => compute_i_explode(rolls, sides, res, option),
        Rule::reroll => compute_reroll(rolls, sides, res, option),
        Rule::i_reroll => compute_i_reroll(rolls, sides, res, option),
        Rule::keep_hi => {
            let value = extract_option_value(option).unwrap();
            rolls.add_history(res.clone(), false);
            (TotalModifier::KeepHi(value as usize), res)
        }
        Rule::keep_lo => {
            let value = extract_option_value(option).unwrap();
            rolls.add_history(res.clone(), false);
            (TotalModifier::KeepLo(value as usize), res)
        }
        Rule::drop_hi => {
            let value = extract_option_value(option).unwrap();
            rolls.add_history(res.clone(), false);
            (TotalModifier::DropHi(value as usize), res)
        }
        Rule::drop_lo => {
            let value = extract_option_value(option).unwrap();
            rolls.add_history(res.clone(), false);
            (TotalModifier::DropLo(value as usize), res)
        }
        Rule::target => {
            let value = extract_option_value(option).unwrap();
            (TotalModifier::TargetFailure(value, 0), res)
        }
        Rule::failure => {
            let value = extract_option_value(option).unwrap();
            (TotalModifier::TargetFailure(0, value), res)
        }
        _ => unreachable!("{:#?}", option),
    };
    // check if we have enough dice to keep/drop
    match modifier {
        TotalModifier::KeepHi(n)
        | TotalModifier::KeepLo(n)
        | TotalModifier::DropHi(n)
        | TotalModifier::DropLo(n) => {
            if n > res.len() {
                return Err("Not enough dice to keep or drop".into());
            }
        }
        TotalModifier::None | TotalModifier::TargetFailure(_, _) | TotalModifier::Fudge => (),
    }
    res.sort_unstable();
    let res = match modifier {
        TotalModifier::KeepHi(n) => res[res.len() - n..].to_vec(),
        TotalModifier::KeepLo(n) => res[..n].to_vec(),
        TotalModifier::DropHi(n) => res[..res.len() - n].to_vec(),
        TotalModifier::DropLo(n) => res[n..].to_vec(),
        TotalModifier::None | TotalModifier::TargetFailure(_, _) | TotalModifier::Fudge => res,
    };
    Ok(OptionResult { res, modifier })
}

fn compute_roll(mut dice: Pairs<Rule>) -> Result<SingleRollResult> {
    let mut rolls = SingleRollResult::new();
    let maybe_nb = dice.next().unwrap();
    let nb = match maybe_nb.as_rule() {
        Rule::nb_dice => {
            dice.next(); // skip `roll`
            maybe_nb.as_str().parse::<u64>().unwrap()
        }
        Rule::roll => 1,
        _ => unreachable!("{:?}", maybe_nb),
    };

    let pair = dice.next().unwrap();
    let (sides, is_fudge) = match pair.as_rule() {
        Rule::number => (pair.as_str().parse::<u64>().unwrap(), false),
        Rule::fudge => (6, true),
        _ => unreachable!("{:?}", pair),
    };
    if sides == 0 {
        return Err("Dice can't have 0 sides".into());
    }
    let mut res = roll_dice(nb, sides);
    let mut modifier = TotalModifier::None;
    let mut next_option = dice.next();
    if !is_fudge && next_option.is_some() {
        while next_option.is_some() {
            let option = next_option.unwrap();
            let opt_res = compute_option(&mut rolls, sides, res, option)?;
            res = opt_res.res;
            modifier = match opt_res.modifier {
                TotalModifier::TargetFailure(t, f) => match modifier {
                    TotalModifier::TargetFailure(ot, of) => {
                        if t > 0 {
                            TotalModifier::TargetFailure(t, of)
                        } else {
                            TotalModifier::TargetFailure(ot, f)
                        }
                    }
                    _ => {
                        rolls.add_history(res.clone(), false);
                        opt_res.modifier
                    }
                },
                _ => opt_res.modifier,
            };
            next_option = dice.next();
        }
        rolls.compute_total(modifier);
    } else {
        rolls.add_history(res, is_fudge);
        rolls.compute_total(if is_fudge {
            TotalModifier::Fudge
        } else {
            TotalModifier::None
        });
    }

    Ok(rolls)
}

// compute a whole roll expression
pub(crate) fn compute(expr: Pairs<Rule>) -> Result<SingleRollResult> {
    get_climber().climb(
        expr,
        |pair: Pair<Rule>| match pair.as_rule() {
            Rule::integer => Ok(SingleRollResult::with_total(
                pair.as_str().parse::<i64>().unwrap(),
            )),
            Rule::expr => compute(pair.into_inner()),
            Rule::dice => compute_roll(pair.into_inner()),
            _ => unreachable!("{:#?}", pair),
        },
        |lhs: Result<SingleRollResult>, op: Pair<Rule>, rhs: Result<SingleRollResult>| match (
            lhs, rhs,
        ) {
            (Ok(lhs), Ok(rhs)) => match op.as_rule() {
                Rule::add => Ok(lhs + rhs),
                Rule::sub => Ok(lhs - rhs),
                Rule::mul => Ok(lhs * rhs),
                Rule::div => {
                    if rhs.get_total() != 0 {
                        Ok(lhs / rhs)
                    } else {
                        Err("Can't divide by zero".into())
                    }
                }
                _ => unreachable!(),
            },
            (Err(e), _) => Err(e),
            (_, Err(e)) => Err(e),
        },
    )
}

pub(crate) fn find_first_dice(expr: &mut Pairs<Rule>) -> Option<String> {
    let mut next_pair = expr.next();
    while next_pair.is_some() {
        let pair = next_pair.unwrap();
        match pair.as_rule() {
            Rule::expr => return find_first_dice(&mut pair.into_inner()),
            Rule::dice => return Some(pair.as_str().trim().to_owned()),
            _ => (),
        };
        next_pair = expr.next();
    }
    None
}

pub(crate) fn roll_dice(num: u64, sides: u64) -> Vec<u64> {
    let mut rng = thread_rng();
    (0..num).map(|_| rng.gen_range(1, sides + 1)).collect()
}

fn extract_option_value(option: Pair<Rule>) -> Option<u64> {
    option
        .into_inner()
        .next()
        .map(|p| p.as_str().parse::<u64>().unwrap())
}
