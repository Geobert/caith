use std::{
    fmt::Display,
    ops::Deref,
    ops::{Add, Div, Mul, Sub},
};

use crate::{error::Result, parser::TotalModifier};

/// Used to mark a dice roll if its result is a critic
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Critic {
    /// Normal result
    No,
    /// Minimum reached
    Min,
    /// Maximum reached
    Max,
}

/// Carry one dice result, and a marker field to say if it the result is a min, max, or none
#[derive(Debug, Clone, Copy, Hash)]
pub struct DiceResult {
    /// The side of the dice that was rolled
    pub res: u64,
    /// If the result was remarkable (critic)
    pub crit: Critic,
}

impl DiceResult {
    /// Create a `DiceResult`
    pub fn new(value: u64, sides: u64) -> Self {
        DiceResult {
            res: value,
            crit: if value == sides {
                Critic::Max
            } else if value == 1 {
                Critic::Min
            } else {
                Critic::No
            },
        }
    }
}

impl PartialEq for DiceResult {
    fn eq(&self, other: &Self) -> bool {
        self.res == other.res
    }
}

impl Eq for DiceResult {}

impl PartialOrd for DiceResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

impl Ord for DiceResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.res.cmp(&other.res)
    }
}

impl Deref for DiceResult {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.res
    }
}

/// In a `RollResult` history, we either have a vector of the roll, or a separator between different
/// dices. Ex: `1d6 + 1d6`, we will have a [`RollHistory::Roll`] followed by [`RollHistory::Separator`] and
/// another [`RollHistory::Roll`]
#[derive(Debug, Clone)]
pub enum RollHistory {
    /// A roll with normal dices
    Roll(Vec<DiceResult>),
    /// A roll with Fudge dices
    Fudge(Vec<u64>),
    /// Was not a roll, but just a value
    Value(i64),
    /// An operation between roll and/or value
    Separator(&'static str),
}

/// Distinguish between a simple roll and a repeated roll using `^`
#[derive(Debug, Clone)]
pub enum RollResultType {
    /// A single roll
    Single(SingleRollResult),
    /// An expression repeated multiple times (using the `^` operator)
    Repeated(RepeatedRollResult),
}

/// A `RollResult` contains either a single roll result, or if the roll is repeated, a list of the
/// same roll different results. And a reason if needed.
#[derive(Debug, Clone)]
pub struct RollResult {
    result: RollResultType,
    reason: Option<String>,
}

impl RollResult {
    /// Create a `RollResult` with only one single roll
    pub fn new_single(r: SingleRollResult) -> Self {
        RollResult {
            result: RollResultType::Single(r),
            reason: None,
        }
    }

    /// Create a `RollResult` with a repeated roll results
    pub fn new_repeated(v: Vec<SingleRollResult>, total: Option<i64>) -> Self {
        RollResult {
            result: RollResultType::Repeated(RepeatedRollResult { rolls: v, total }),
            reason: None,
        }
    }

    /// Add a comment to the result
    pub fn add_reason(&mut self, reason: String) {
        self.reason = Some(reason);
    }

    /// Get the comment, if any
    pub fn get_reason(&self) -> Option<&String> {
        self.reason.as_ref()
    }

    /// Return the result
    pub fn get_result(&self) -> &RollResultType {
        &self.result
    }

    /// If the result is a single roll, it will return it
    pub fn as_single(&self) -> Option<&SingleRollResult> {
        match &self.result {
            RollResultType::Single(result) => Some(result),
            RollResultType::Repeated(_) => None,
        }
    }

    /// If the result is a repeated roll, it will return it
    pub fn as_repeated(&self) -> Option<&RepeatedRollResult> {
        match &self.result {
            RollResultType::Single(_) => None,
            RollResultType::Repeated(results) => Some(&results),
        }
    }
}

/// Represent a repeated roll. Can store the sum of all the roll if asked to.
/// Usually created through `RollResult::new_repeated()` function.
#[derive(Debug, Clone)]
pub struct RepeatedRollResult {
    rolls: Vec<SingleRollResult>,
    total: Option<i64>,
}

impl Deref for RepeatedRollResult {
    type Target = Vec<SingleRollResult>;

    fn deref(&self) -> &Self::Target {
        &self.rolls
    }
}

impl RepeatedRollResult {
    /// If the repeated roll was asked with a total, this will return the computed total.
    pub fn get_total(&self) -> Option<i64> {
        self.total
    }
}

/// Carry the result of one roll and an history of the steps taken
/// Usually created through `RollResult::new_single()` function.
#[derive(Debug, Clone)]
pub struct SingleRollResult {
    /// Result of the roll. In the case of option `t` and/or `f` used, it's the number of `success -
    /// failure`
    total: i64,
    /// History of the steps taken that lead to this result.
    history: Vec<RollHistory>,
    /// Internal usage field to avoid computing a total if it's already done.
    dirty: bool,
}

impl SingleRollResult {
    /// Create an empty `SingleRollResult`
    pub(crate) fn new() -> Self {
        Self {
            total: 0,
            history: Vec::new(),
            dirty: true,
        }
    }

    /// Create a `SingleRollResult` with already a total. Used to carry constant value.
    pub(crate) fn with_total(total: i64) -> Self {
        Self {
            total,
            history: vec![RollHistory::Value(total)],
            dirty: false,
        }
    }

    /// Create a `SingleRollResult` with a history and a total. Used to carry an OVA result.
    pub(crate) fn new_ova(total: u64, history: Vec<DiceResult>) -> Self {
        Self {
            total: total as i64,
            history: vec![RollHistory::Roll(history)],
            dirty: false,
        }
    }

    /// Get the history of the result
    pub fn get_history(&self) -> &Vec<RollHistory> {
        &self.history
    }

    /// Add a step in the history
    pub(crate) fn add_history(&mut self, mut history: Vec<DiceResult>, is_fudge: bool) {
        self.dirty = true;
        history.sort_unstable_by(|a, b| b.cmp(a));
        self.history.push(if is_fudge {
            RollHistory::Fudge(history.iter().map(|r| r.res).collect())
        } else {
            RollHistory::Roll(history)
        });
    }

    /// Compute the total value according to some modifier
    pub(crate) fn compute_total(&mut self, modifier: TotalModifier) -> Result<i64> {
        if self.dirty {
            self.dirty = false;
            let mut flat = self.history.iter().fold(Vec::new(), |mut acc, h| {
                match h {
                    RollHistory::Roll(r) => {
                        let mut c = r.iter().map(|u| u.res as i64).collect();
                        acc.append(&mut c);
                    }
                    RollHistory::Fudge(r) => {
                        let mut c = r.iter().map(|u| *u as i64).collect();
                        acc.append(&mut c);
                    }
                    RollHistory::Value(v) => acc.push(*v),
                    RollHistory::Separator(_) => (),
                };
                acc
            });
            flat.sort_unstable();
            let flat = flat;
            match modifier {
                TotalModifier::KeepHi(n)
                | TotalModifier::KeepLo(n)
                | TotalModifier::DropHi(n)
                | TotalModifier::DropLo(n) => {
                    if n > flat.len() {
                        return Err("Not enough dice to keep or drop".into());
                    }
                }
                TotalModifier::None(_)
                | TotalModifier::TargetFailure(_, _)
                | TotalModifier::Fudge => (),
            }

            let slice = match modifier {
                TotalModifier::KeepHi(n) => &flat[flat.len() - n..],
                TotalModifier::KeepLo(n) => &flat[..n],
                TotalModifier::DropHi(n) => &flat[..flat.len() - n],
                TotalModifier::DropLo(n) => &flat[n..],
                TotalModifier::None(_)
                | TotalModifier::TargetFailure(_, _)
                | TotalModifier::Fudge => flat.as_slice(),
            };

            self.total = match modifier {
                TotalModifier::TargetFailure(t, f) => slice.iter().fold(0, |acc, x| {
                    let x = *x as u64;
                    if x >= t {
                        acc + 1
                    } else if x <= f {
                        acc - 1
                    } else {
                        acc
                    }
                }),
                TotalModifier::Fudge => slice.iter().fold(0, |acc, x| {
                    if *x <= 2 {
                        acc - 1
                    } else if *x <= 4 {
                        acc
                    } else {
                        acc + 1
                    }
                }),
                _ => slice.iter().sum::<i64>(),
            };
        }

        Ok(self.total)
    }

    /// Get the result value
    pub fn get_total(&self) -> i64 {
        self.total
    }

    /// Turn the vector of `RollHistory` to a `String`
    pub fn to_string_history(&self) -> String {
        self.history.iter().fold(String::new(), |mut s, v| match v {
            RollHistory::Roll(v) => {
                s.push('[');
                let len = v.len();
                v.iter().enumerate().for_each(|(i, r)| {
                    s.push_str(&r.res.to_string());
                    if i < len - 1 {
                        s.push_str(", ");
                    }
                });
                s.push(']');
                s
            }
            RollHistory::Fudge(v) => {
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
                s.push_str(&v.to_string());
                s
            }
            RollHistory::Separator(sep) => {
                s.push_str(sep);
                s
            }
        })
    }

    /// Turn the `RollResult` to a readable String, with or without markdown formatting.
    pub fn to_string(&self, md: bool) -> String {
        if self.history.is_empty() {
            if md {
                format!("`{}`", self.total)
            } else {
                format!("{}", self.total)
            }
        } else {
            let s = self.to_string_history();
            format!(
                "{1}{0}{1} = {2}{3}{2}",
                s,
                if md { "`" } else { "" },
                if md { "**" } else { "" },
                self.get_total()
            )
        }
    }
}

fn merge_history(left: &mut SingleRollResult, right: &mut SingleRollResult, op: &'static str) {
    if !right.history.is_empty() {
        left.history.push(RollHistory::Separator(op));
        left.history.append(&mut right.history);
    }
}

impl Add for SingleRollResult {
    type Output = Self;

    fn add(mut self, mut rhs: Self) -> Self::Output {
        merge_history(&mut self, &mut rhs, " + ");
        SingleRollResult {
            total: self.total + rhs.total,
            history: self.history,
            dirty: false,
        }
    }
}

impl Sub for SingleRollResult {
    type Output = Self;

    fn sub(mut self, mut rhs: Self) -> Self::Output {
        merge_history(&mut self, &mut rhs, " - ");
        SingleRollResult {
            total: self.total - rhs.total,
            history: self.history,
            dirty: false,
        }
    }
}

impl Mul for SingleRollResult {
    type Output = Self;

    fn mul(mut self, mut rhs: Self) -> Self::Output {
        merge_history(&mut self, &mut rhs, " * ");
        SingleRollResult {
            total: self.total * rhs.total,
            history: self.history,
            dirty: false,
        }
    }
}

impl Div for SingleRollResult {
    type Output = Self;

    fn div(mut self, mut rhs: Self) -> Self::Output {
        merge_history(&mut self, &mut rhs, " / ");
        SingleRollResult {
            total: self.total / rhs.total,
            history: self.history,
            dirty: false,
        }
    }
}

impl Display for RollResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.result {
            RollResultType::Single(roll_result) => {
                write!(f, "{}", roll_result.to_string(true))?;
                if let Some(reason) = &self.reason {
                    write!(f, ", Reason: `{}`", reason)?;
                }
            }
            RollResultType::Repeated(repeated_result) => match repeated_result.total {
                Some(total) => {
                    repeated_result
                        .rolls
                        .iter()
                        .try_for_each(|res| writeln!(f, "`{}`", res.to_string_history()))?;
                    write!(f, "Sum: **{}**", total)?;
                    if let Some(reason) = &self.reason {
                        write!(f, ", Reason: `{}`", reason)?;
                    }
                }
                None => {
                    repeated_result
                        .rolls
                        .iter()
                        .try_for_each(|res| writeln!(f, "{}", res.to_string(true)))?;
                    if let Some(reason) = &self.reason {
                        write!(f, "Reason: `{}`", reason)?;
                    }
                }
            },
        }

        Ok(())
    }
}

impl Display for SingleRollResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string(true))?;
        Ok(())
    }
}
