use std::fmt::Display;

use crate::rollresult::DiceResult;

/// Carry a constant, either an `i64` or a `f64`.
#[derive(Debug, Clone)]
pub enum Value {
    /// Integer variant
    Int(i64),
    /// Float variant
    Float(f64),
}

impl Value {
    /// Get the value as `i64`.
    pub fn get_value(&self) -> i64 {
        match *self {
            Value::Int(i) => i,
            Value::Float(f) => f as i64,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match *self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
        };
        write!(f, "{}", s)
    }
}

/// Carry one step of the history that led to the result.
///
/// In a [`super::RollResult`]'s history, we either have a vector of the roll, or a separator
/// between different dices. Ex: for `1d6 + 1d6`, we will have a [`RollHistory::Roll`] followed by
/// [`RollHistory::Separator`] and another [`RollHistory::Roll`].
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum RollHistory {
    /// Rolls which include rerolls.
    /// Should be followed by a Roll with the final results.
    ReRolls(Vec<Vec<DiceResult>>),
    /// A roll with normal dices
    Roll(Vec<DiceResult>),
    /// A roll with Fudge dices
    Fudge(Vec<u64>),
    /// Was not a roll, but just a value
    Value(Value),
    /// An operation between roll and/or value
    Separator(&'static str),
    /// Open parenthesis
    OpenParenthesis,
    /// Close parenthesis
    CloseParenthesis,
}

impl Display for RollHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RollHistory::ReRolls(v) => {
                let s2 = v
                    .iter()
                    .map(|r| {
                        r.iter()
                            .map(|r| r.res.to_string())
                            .collect::<Vec<_>>()
                            .join(" -> ")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}] -> ", s2)
            }
            RollHistory::Roll(v) => {
                let s2 = v
                    .iter()
                    .map(|r| r.res.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", s2)
            }
            RollHistory::Fudge(v) => {
                let mut s = String::new();
                s.push('[');
                let len = v.len();
                v.iter().enumerate().for_each(|(i, r)| {
                    let r = if *r <= 2 {
                        "-"
                    } else if *r <= 4 {
                        "▢"
                    } else {
                        "+"
                    };
                    s.push_str(r);
                    if i < len - 1 {
                        s.push_str(", ");
                    }
                });
                s.push(']');
                s
            }
            RollHistory::Value(v) => {
                let mut s = String::new();
                s.push_str(&v.to_string());
                s
            }
            RollHistory::Separator(sep) => {
                let mut s = String::new();
                s.push_str(sep);
                s
            }
            RollHistory::OpenParenthesis => "(".to_string(),
            RollHistory::CloseParenthesis => ")".to_string(),
        };
        write!(f, "{}", s)
    }
}
