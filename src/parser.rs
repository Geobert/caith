use once_cell::sync::Lazy;
use pest::{
    iterators::{Pair, Pairs},
    prec_climber::{Assoc, Operator, PrecClimber},
};
use pest_derive::Parser;
use rand::{thread_rng, Rng};

use crate::rollresult::RollResult;

static PREC_CLIMBER: Lazy<PrecClimber<Rule>> = Lazy::new(|| {
    use self::Assoc::*;
    use self::Rule::*;

    PrecClimber::new(vec![
        Operator::new(add, Left) | Operator::new(sub, Left),
        Operator::new(mul, Left) | Operator::new(div, Left),
    ])
});

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
    None,
}

fn extract_option_value(option: Pair<Rule>) -> u64 {
    option
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .parse::<u64>()
        .unwrap()
}

fn compute_option(
    rolls: &mut RollResult,
    sides: u64,
    res: Vec<u64>,
    option: Pair<Rule>,
) -> OptionResult {
    let (modifier, mut res) = match option.as_rule() {
        Rule::explode => {
            let value = extract_option_value(option);
            let nb = res.iter().filter(|x| **x == value).count() as u64;
            rolls.add_history(res.clone());
            let res = if nb > 0 {
                let res = roll_dice(nb, sides);
                rolls.add_history(res.clone());
                res
            } else {
                res
            };
            (TotalModifier::None, res)
        }
        Rule::i_explode => {
            let value = extract_option_value(option);
            rolls.add_history(res.clone());
            let mut nb = res.into_iter().filter(|x| *x == value).count() as u64;
            let mut res = Vec::new();
            while nb > 0 {
                res = roll_dice(nb, sides);
                nb = res.iter().filter(|x| **x == value).count() as u64;
                rolls.add_history(res.clone());
            }
            (TotalModifier::None, res)
        }
        Rule::reroll => {
            let value = extract_option_value(option);
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
                rolls.add_history(res.clone());
            }
            (TotalModifier::None, res)
        }
        Rule::i_reroll => {
            let value = extract_option_value(option);
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
                rolls.add_history(res.clone());
            }
            (TotalModifier::None, res)
        }
        Rule::keep_hi => {
            let value = extract_option_value(option);
            rolls.add_history(res.clone());
            (TotalModifier::KeepHi(value as usize), res)
        }
        Rule::keep_lo => {
            let value = extract_option_value(option);
            rolls.add_history(res.clone());
            (TotalModifier::KeepLo(value as usize), res)
        }
        Rule::drop_hi => {
            let value = extract_option_value(option);
            rolls.add_history(res.clone());
            (TotalModifier::DropHi(value as usize), res)
        }
        Rule::drop_lo => {
            let value = extract_option_value(option);
            rolls.add_history(res.clone());
            (TotalModifier::DropLo(value as usize), res)
        }
        Rule::target => {
            let value = extract_option_value(option);
            (TotalModifier::TargetFailure(value, 0), res)
        }
        Rule::failure => {
            let value = extract_option_value(option);
            (TotalModifier::TargetFailure(0, value), res)
        }
        _ => unreachable!("{:#?}", option),
    };
    res.sort_unstable();
    let res = match modifier {
        TotalModifier::KeepHi(n) => res[res.len() - n..].to_vec(),
        TotalModifier::KeepLo(n) => res[..n].to_vec(),
        TotalModifier::DropHi(n) => res[..res.len() - n].to_vec(),
        TotalModifier::DropLo(n) => res[n..].to_vec(),
        TotalModifier::None | TotalModifier::TargetFailure(_, _) => res,
    };
    OptionResult { res, modifier }
}

struct OptionResult {
    res: Vec<u64>,
    modifier: TotalModifier,
}

fn compute_roll(mut dice: Pairs<Rule>) -> RollResult {
    let mut rolls = RollResult::new();
    let nb = dice.next().unwrap().as_str().parse::<u64>().unwrap();
    let sides = dice.next().unwrap().as_str().parse::<u64>().unwrap();
    let mut res = roll_dice(nb, sides);
    let mut modifier = TotalModifier::None;
    let mut next_option = dice.next();
    if next_option.is_some() {
        while next_option.is_some() {
            let option = next_option.unwrap();
            let opt_res = compute_option(&mut rolls, sides, res, option);
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
                        rolls.add_history(res.clone());
                        opt_res.modifier
                    }
                },
                _ => opt_res.modifier,
            };
            next_option = dice.next();
        }
        rolls.compute_total(modifier);
    } else {
        rolls.add_history(res);
        rolls.compute_total(TotalModifier::None);
    }

    rolls
}

pub(crate) fn compute(expr: Pairs<Rule>) -> RollResult {
    PREC_CLIMBER.climb(
        expr,
        |pair: Pair<Rule>| match pair.as_rule() {
            Rule::number => RollResult::with_total(pair.as_str().parse::<i64>().unwrap()),
            Rule::expr => compute(pair.into_inner()),
            Rule::dice => compute_roll(pair.into_inner()),
            _ => unreachable!("{:#?}", pair),
        },
        |lhs: RollResult, op: Pair<Rule>, rhs: RollResult| match op.as_rule() {
            Rule::add => lhs + rhs,
            Rule::sub => lhs - rhs,
            Rule::mul => lhs * rhs,
            Rule::div => lhs / rhs,
            _ => unreachable!(),
        },
    )
}

fn roll_dice(num: u64, sides: u64) -> Vec<u64> {
    let mut rng = thread_rng();
    (0..num).map(|_| rng.gen_range(1, sides + 1)).collect()
}
