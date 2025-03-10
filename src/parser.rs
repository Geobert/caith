use std::sync::{Arc, Once, RwLock};

use pest::{
    iterators::{Pair, Pairs},
    pratt_parser::PrattParser,
};
use pest_derive::Parser;

use crate::{error::Result, DiceResult, SingleRollResult};

pub trait DiceRollSource {
    fn roll_single_die(&mut self, sides: u64) -> u64;
}

#[derive(Parser)]
#[grammar = "caith.pest"]
pub(crate) struct RollParser;

// arbitrary limit to avoid OOM
const MAX_DICE_SIDES: u64 = 5000;
const MAX_NUMBER_OF_DICE: u64 = 5000;

// number represent nb dice to keep/drop
#[derive(Clone, PartialEq)]
pub(crate) enum TotalModifier {
    KeepHi(usize),
    KeepLo(usize),
    DropHi(usize),
    DropLo(usize),
    TargetFailureDouble(u64, u64, u64),
    TargetEnum(Vec<u64>),
    Fudge,
    None(Rule),
}

struct OptionResult {
    res: Vec<DiceResult>,
    modifier: TotalModifier,
}

// Struct to have a singleton of PrecClimber without using once_cell
#[derive(Clone)]
struct Climber {
    inner: Arc<RwLock<PrattParser<Rule>>>,
}

impl Climber {
    fn climb<'i, P, F, G, T>(&self, pairs: P, primary: F, infix: G) -> T
    where
        P: Iterator<Item = Pair<'i, Rule>>,
        F: FnMut(Pair<'i, Rule>) -> T,
        G: FnMut(T, Pair<'i, Rule>, T) -> T + 'i,
    {
        self.inner
            .read()
            .unwrap()
            .map_primary(primary)
            .map_infix(infix)
            .parse(pairs)
    }
}

fn get_climber() -> Climber {
    static mut PREC_CLIMBER: *const Climber = 0 as *const Climber;
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            use pest::pratt_parser::{Assoc, Op};

            // Make it
            let singleton = Climber {
                inner: Arc::new(RwLock::new(
                    PrattParser::new()
                        .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
                        .op(Op::infix(Rule::mul, Assoc::Left) | Op::infix(Rule::div, Assoc::Left)),
                )),
            };

            // Put it in the heap so it can outlive this call
            PREC_CLIMBER = std::mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*PREC_CLIMBER).clone()
    }
}

fn compute_explode<RNG: DiceRollSource>(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<DiceResult>,
    option: Pair<Rule>,
    prev_modifier: &TotalModifier,
    rng: &mut RNG,
) -> (TotalModifier, Vec<DiceResult>) {
    let value = extract_option_value(option).unwrap_or(sides);
    let nb = res.iter().filter(|x| x.res >= value).count() as u64;
    if prev_modifier != &TotalModifier::None(Rule::explode)
        && prev_modifier != &TotalModifier::None(Rule::i_explode)
    {
        rolls.add_history(res.clone(), false);
    }
    let res = if nb > 0 {
        let res = roll_dice(nb, sides, rng);
        rolls.add_history(res.clone(), false);
        res
    } else {
        res
    };
    (TotalModifier::None(Rule::explode), res)
}

fn compute_i_explode<RNG: DiceRollSource>(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<DiceResult>,
    option: Pair<Rule>,
    prev_modifier: &TotalModifier,
    rng: &mut RNG,
) -> (TotalModifier, Vec<DiceResult>) {
    let value = extract_option_value(option).unwrap_or(sides);
    if prev_modifier != &TotalModifier::None(Rule::explode)
        && prev_modifier != &TotalModifier::None(Rule::i_explode)
    {
        rolls.add_history(res.clone(), false);
    }
    let mut nb = res.into_iter().filter(|x| x.res >= value).count() as u64;
    let mut res = Vec::new();
    while nb > 0 {
        res = roll_dice(nb, sides, rng);
        nb = res.iter().filter(|x| x.res >= value).count() as u64;
        rolls.add_history(res.clone(), false);
    }
    (TotalModifier::None(Rule::i_explode), res)
}

fn compute_reroll<RNG: DiceRollSource>(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<DiceResult>,
    option: Pair<Rule>,
    rng: &mut RNG,
) -> (TotalModifier, Vec<DiceResult>) {
    let value = extract_option_value(option).unwrap();
    let mut has_rerolled = false;
    let mut rerolls: Vec<Vec<DiceResult>> = vec![];
    let res_new: Vec<DiceResult> = res
        .iter()
        .map(|x| {
            let mut inner = vec![*x];
            let result = if x.res <= value {
                has_rerolled = true;
                let rerolled = roll_dice(1, sides, rng)[0];
                inner.push(rerolled);
                rerolled
            } else {
                *x
            };
            rerolls.push(inner);
            result
        })
        .collect();

    if has_rerolled {
        rolls.add_rerolled_history(rerolls);
    }
    rolls.add_history(res_new.clone(), false);

    (TotalModifier::None(Rule::reroll), res_new)
}

fn compute_i_reroll<RNG: DiceRollSource>(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<DiceResult>,
    option: Pair<Rule>,
    rng: &mut RNG,
) -> (TotalModifier, Vec<DiceResult>) {
    let value = extract_option_value(option).unwrap();
    let mut has_rerolled = false;
    let res: Vec<DiceResult> = res
        .into_iter()
        .map(|x| {
            let mut x = x;
            while x.res <= value {
                has_rerolled = true;
                x = roll_dice(1, sides, rng)[0]
            }
            x
        })
        .collect();

    if has_rerolled {
        rolls.add_history(res.clone(), false);
    }
    (TotalModifier::None(Rule::i_reroll), res)
}

fn compute_option<RNG: DiceRollSource>(
    rolls: &mut SingleRollResult,
    sides: u64,
    res: Vec<DiceResult>,
    option: Pair<Rule>,
    rng: &mut RNG,
    prev_modifier: &TotalModifier,
) -> Result<OptionResult> {
    let (modifier, mut res) = match &option.as_rule() {
        Rule::explode => compute_explode(rolls, sides, res, option, prev_modifier, rng),
        Rule::i_explode => compute_i_explode(rolls, sides, res, option, prev_modifier, rng),
        Rule::reroll => compute_reroll(rolls, sides, res, option, rng),
        Rule::i_reroll => compute_i_reroll(rolls, sides, res, option, rng),
        Rule::keep_hi => {
            let value = extract_option_value(option).unwrap();
            if rolls.get_history().is_empty() {
                rolls.add_history(res.clone(), false);
            }
            (TotalModifier::KeepHi(value as usize), res)
        }
        Rule::keep_lo => {
            let value = extract_option_value(option).unwrap();
            if rolls.get_history().is_empty() {
                rolls.add_history(res.clone(), false);
            }
            (TotalModifier::KeepLo(value as usize), res)
        }
        Rule::drop_hi => {
            let value = extract_option_value(option).unwrap();
            if rolls.get_history().is_empty() {
                rolls.add_history(res.clone(), false);
            }
            (TotalModifier::DropHi(value as usize), res)
        }
        Rule::drop_lo => {
            let value = extract_option_value(option).unwrap();
            if rolls.get_history().is_empty() {
                rolls.add_history(res.clone(), false);
            }
            (TotalModifier::DropLo(value as usize), res)
        }
        Rule::target => {
            let value_or_enum = option.into_inner().next().unwrap();
            match value_or_enum.as_rule() {
                Rule::number => (
                    TotalModifier::TargetFailureDouble(
                        value_or_enum.as_str().parse::<u64>().unwrap(),
                        0,
                        0,
                    ),
                    res,
                ),
                Rule::target_enum => {
                    let numbers_list = value_or_enum.into_inner();
                    let numbers_list: Vec<_> = numbers_list
                        .map(|p| p.as_str().parse::<u64>().unwrap())
                        .collect();
                    (TotalModifier::TargetEnum(numbers_list), res)
                }
                _ => unreachable!(),
            }
        }
        Rule::double_target => {
            let value = extract_option_value(option).unwrap();
            (TotalModifier::TargetFailureDouble(0, 0, value), res)
        }
        Rule::failure => {
            let value = extract_option_value(option).unwrap();
            (TotalModifier::TargetFailureDouble(0, value, 0), res)
        }
        _ => unreachable!("{:#?}", option),
    };

    let n = match modifier {
        TotalModifier::KeepHi(n) | TotalModifier::KeepLo(n) => {
            if n > res.len() {
                res.len()
            } else {
                n
            }
        }
        TotalModifier::DropHi(n) | TotalModifier::DropLo(n) => {
            if n > res.len() {
                0
            } else {
                n
            }
        }
        TotalModifier::None(_)
        | TotalModifier::TargetFailureDouble(_, _, _)
        | TotalModifier::TargetEnum(_)
        | TotalModifier::Fudge => 0,
    };
    res.sort_unstable();
    let res = match modifier {
        TotalModifier::KeepHi(_) => res[res.len() - n..].to_vec(),
        TotalModifier::KeepLo(_) => res[..n].to_vec(),
        TotalModifier::DropHi(_) => res[..res.len() - n].to_vec(),
        TotalModifier::DropLo(_) => res[n..].to_vec(),
        TotalModifier::None(_)
        | TotalModifier::TargetFailureDouble(_, _, _)
        | TotalModifier::TargetEnum(_)
        | TotalModifier::Fudge => res,
    };
    Ok(OptionResult { res, modifier })
}

fn compute_roll<RNG: DiceRollSource>(
    mut dice: Pairs<Rule>,
    rng: &mut RNG,
) -> Result<SingleRollResult> {
    let mut rolls = SingleRollResult::new();
    let number_of_dice = dice.next().unwrap();
    let number_of_dice = match number_of_dice.as_rule() {
        Rule::nb_dice => {
            dice.next(); // skip `d` token
            let n = number_of_dice.as_str().parse::<u64>().unwrap();
            if n > MAX_NUMBER_OF_DICE {
                return Err(format!(
                    "Exceed maximum allowed number of dices ({})",
                    MAX_NUMBER_OF_DICE
                )
                .into());
            }
            n
        }
        Rule::roll => 1, // no number before `d`, assume 1 dice
        _ => unreachable!("{:?}", number_of_dice),
    };

    let pair = dice.next().unwrap();
    let (sides, is_fudge) = match pair.as_rule() {
        Rule::number => (pair.as_str().parse::<u64>().unwrap(), false),
        Rule::fudge => (6, true),
        _ => unreachable!("{:?}", pair),
    };

    if sides == 0 {
        return Err("Dice can't have 0 sides".into());
    } else if sides > MAX_DICE_SIDES {
        return Err(format!("Dice can't have more than {}", MAX_DICE_SIDES).into());
    }

    let mut res = roll_dice(number_of_dice, sides, rng);
    let mut modifier = TotalModifier::None(Rule::expr);
    let mut next_option = dice.next();
    if !is_fudge {
        if next_option.is_some() {
            while next_option.is_some() {
                let option = next_option.unwrap();
                let opt_res = compute_option(&mut rolls, sides, res, option, rng, &modifier)?;
                res = opt_res.res;
                modifier = match opt_res.modifier {
                    TotalModifier::TargetFailureDouble(t, f, d) => match modifier {
                        TotalModifier::TargetFailureDouble(ot, of, od) => {
                            if t > 0 {
                                TotalModifier::TargetFailureDouble(t, of, od)
                            } else if f > 0 {
                                TotalModifier::TargetFailureDouble(ot, f, od)
                            } else {
                                TotalModifier::TargetFailureDouble(ot, of, d)
                            }
                        }
                        _ => {
                            rolls.add_history(res.clone(), is_fudge);
                            opt_res.modifier
                        }
                    },
                    TotalModifier::TargetEnum(_) => {
                        rolls.add_history(res.clone(), is_fudge);
                        opt_res.modifier
                    }
                    _ => opt_res.modifier,
                };
                next_option = dice.next();
            }
        } else {
            rolls.add_history(res, is_fudge);
        }
        rolls.compute_total(modifier)?;
    } else {
        rolls.add_history(res, is_fudge);
        rolls.compute_total(if is_fudge {
            TotalModifier::Fudge
        } else {
            TotalModifier::None(Rule::expr)
        })?;
    }

    Ok(rolls)
}

// compute a whole roll expression
pub(crate) fn compute<RNG: DiceRollSource>(
    expr: Pairs<Rule>,
    rng: &mut RNG,
    is_block: bool,
) -> Result<SingleRollResult> {
    let res = get_climber().climb(
        expr,
        |pair: Pair<Rule>| match pair.as_rule() {
            Rule::integer => Ok(SingleRollResult::with_total(
                pair.as_str().replace(' ', "").parse::<i64>().unwrap(),
            )),
            Rule::float => Ok(SingleRollResult::with_float(
                pair.as_str().replace(' ', "").parse::<f64>().unwrap(),
            )),
            Rule::block_expr => {
                let expr = pair.into_inner().next().unwrap().into_inner();
                compute(expr, rng, true)
            }
            Rule::dice => compute_roll(pair.into_inner(), rng),
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
                    if rhs.is_zero() {
                        Err("Can't divide by zero".into())
                    } else {
                        Ok(lhs / rhs)
                    }
                }
                _ => unreachable!(),
            },
            (Err(e), _) => Err(e),
            (_, Err(e)) => Err(e),
        },
    );
    match res {
        Ok(mut single_roll_res) => {
            if is_block {
                single_roll_res.add_parenthesis();
            }
            Ok(single_roll_res)
        }
        e @ Err(_) => e,
    }
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

pub(crate) fn roll_dice<RNG: DiceRollSource>(
    num: u64,
    sides: u64,
    rng: &mut RNG,
) -> Vec<DiceResult> {
    (0..num)
        .map(|_| DiceResult::new(rng.roll_single_die(sides), sides))
        .collect()
}

fn extract_option_value(option: Pair<Rule>) -> Option<u64> {
    option
        .into_inner()
        .next()
        .map(|p| p.as_str().parse::<u64>().unwrap())
}
