use crate::{
    error::Result, parser::TotalModifier, rollresult::DiceResult, rollresult::RollHistory,
    rollresult::Value,
};

/// Carry the result of one roll and an history of the steps taken.
///
/// Usually created through [`RollResult::new_single()`] function.
#[derive(Debug, Clone)]
pub struct SingleRollResult {
    /// Result of the roll. In the case of option `t` and/or `f` used, it's the number of `success -
    /// failure`
    total: i64,
    /// History of the steps taken that lead to this result.
    history: Vec<RollHistory>,
    /// Internal usage field to avoid computing a total if it's already done.
    dirty: bool,
    constant: Option<f64>,
}

impl SingleRollResult {
    /// Create an empty `SingleRollResult`
    pub(crate) fn new() -> Self {
        Self {
            total: 0,
            history: Vec::new(),
            dirty: true,
            constant: None,
        }
    }

    /// Create a `SingleRollResult` with already a total. Used to carry constant value.
    pub(crate) fn with_total(total: i64) -> Self {
        Self {
            total,
            history: vec![RollHistory::Value(Value::Int(total))],
            dirty: false,
            constant: None,
        }
    }

    /// Create a `SingleRollResult` with already a total. Used to carry float constant value.
    pub(crate) fn with_float(f: f64) -> Self {
        Self {
            total: f as i64,
            history: vec![RollHistory::Value(Value::Float(f))],
            dirty: false,
            constant: Some(f),
        }
    }

    #[cfg(feature = "ova")]
    /// Create a `SingleRollResult` with a history and a total.
    pub(crate) fn with_total_and_hist(total: u64, history: Vec<DiceResult>) -> Self {
        Self {
            total: total as i64,
            history: vec![RollHistory::Roll(history)],
            dirty: false,
            constant: None,
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
                    RollHistory::Value(v) => acc.push(v.get_value()),
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
                | TotalModifier::TargetFailureDouble(_, _, _)
                | TotalModifier::TargetEnum(_)
                | TotalModifier::Fudge => (),
            }

            let slice = match modifier {
                TotalModifier::KeepHi(n) => &flat[flat.len() - n..],
                TotalModifier::KeepLo(n) => &flat[..n],
                TotalModifier::DropHi(n) => &flat[..flat.len() - n],
                TotalModifier::DropLo(n) => &flat[n..],
                TotalModifier::None(_)
                | TotalModifier::TargetFailureDouble(_, _, _)
                | TotalModifier::TargetEnum(_)
                | TotalModifier::Fudge => flat.as_slice(),
            };

            self.total = match modifier {
                TotalModifier::TargetFailureDouble(t, f, d) => slice.iter().fold(0, |acc, &x| {
                    let x = x as u64;
                    if d > 0 && x >= d {
                        acc + 2
                    } else if t > 0 && x >= t {
                        acc + 1
                    } else if f > 0 && x <= f {
                        acc - 1
                    } else {
                        acc
                    }
                }),
                TotalModifier::TargetEnum(v) => slice.iter().fold(0, |acc, &x| {
                    if v.contains(&(x as u64)) {
                        acc + 1
                    } else {
                        acc
                    }
                }),
                TotalModifier::Fudge => slice.iter().fold(0, |acc, &x| {
                    if x <= 2 {
                        acc - 1
                    } else if x <= 4 {
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

    /// Says if the used value for math operation is 0
    ///
    /// If there's a constant stored, we'll use it and if not, `total` is used instead
    pub fn is_zero(&self) -> bool {
        if let Some(c) = self.constant {
            c == 0.0
        } else {
            self.total == 0
        }
    }

    /// Turn the vector of `RollHistory` to a `String`
    pub fn to_string_history(&self) -> String {
        self.history.iter().fold(String::new(), |mut s, v| {
            s.push_str(v.to_string().as_str());
            s
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

impl std::ops::Add for SingleRollResult {
    type Output = Self;

    fn add(mut self, mut rhs: Self) -> Self::Output {
        merge_history(&mut self, &mut rhs, " + ");
        let total = match (self.constant, rhs.constant) {
            (None, None) => self.total + rhs.total,
            (None, Some(constant)) => (self.total as f64 + constant).trunc() as i64,
            (Some(constant), None) => (constant + rhs.total as f64).trunc() as i64,
            (Some(lconstant), Some(rconstant)) => (lconstant + rconstant).trunc() as i64,
        };
        SingleRollResult {
            total,
            history: self.history,
            dirty: false,
            constant: None,
        }
    }
}

impl std::ops::Sub for SingleRollResult {
    type Output = Self;

    fn sub(mut self, mut rhs: Self) -> Self::Output {
        merge_history(&mut self, &mut rhs, " - ");
        let total = match (self.constant, rhs.constant) {
            (None, None) => self.total - rhs.total,
            (None, Some(constant)) => (self.total as f64 - constant).trunc() as i64,
            (Some(constant), None) => (constant - rhs.total as f64).trunc() as i64,
            (Some(lconstant), Some(rconstant)) => (lconstant - rconstant).trunc() as i64,
        };
        SingleRollResult {
            total,
            history: self.history,
            dirty: false,
            constant: None,
        }
    }
}

impl std::ops::Mul for SingleRollResult {
    type Output = Self;

    fn mul(mut self, mut rhs: Self) -> Self::Output {
        merge_history(&mut self, &mut rhs, " * ");
        let total = match (self.constant, rhs.constant) {
            (None, None) => self.total * rhs.total,
            (None, Some(constant)) => (self.total as f64 * constant).trunc() as i64,
            (Some(constant), None) => (constant * rhs.total as f64).trunc() as i64,
            (Some(lconstant), Some(rconstant)) => (lconstant * rconstant).trunc() as i64,
        };
        SingleRollResult {
            total,
            history: self.history,
            dirty: false,
            constant: None,
        }
    }
}

impl std::ops::Div for SingleRollResult {
    type Output = Self;

    fn div(mut self, mut rhs: Self) -> Self::Output {
        merge_history(&mut self, &mut rhs, " / ");
        let total = match (self.constant, rhs.constant) {
            (None, None) => self.total / rhs.total,
            (None, Some(constant)) => (self.total as f64 / constant).trunc() as i64,
            (Some(constant), None) => (constant / rhs.total as f64).trunc() as i64,
            (Some(lconstant), Some(rconstant)) => (lconstant / rconstant).trunc() as i64,
        };
        SingleRollResult {
            total,
            history: self.history,
            dirty: false,
            constant: None,
        }
    }
}
