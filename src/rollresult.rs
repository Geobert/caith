use std::fmt::Display;

mod diceresult;
mod repeatedrollresult;
mod rollhistory;
mod singlerollresult;

pub use diceresult::*;
pub use repeatedrollresult::*;
pub use rollhistory::*;
pub use singlerollresult::*;

/// Distinguish between a simple roll and a repeated roll using `^`.
#[derive(Debug, Clone)]
pub enum RollResultType {
    /// A single roll
    Single(SingleRollResult),
    /// An expression repeated multiple times (using the `^` operator)
    Repeated(RepeatedRollResult),
}

/// Carry the result of the roll.
///
/// A `RollResult` contains either a single roll result, or if the roll is repeated, a list of the
/// same roll different results. And a reason if needed.
#[derive(Debug, Clone)]
pub struct RollResult {
    result: RollResultType,
    reason: Option<String>,
}

impl RollResult {
    /// Create a `RollResult` with only one single roll.
    pub fn new_single(r: SingleRollResult) -> Self {
        RollResult {
            result: RollResultType::Single(r),
            reason: None,
        }
    }

    /// Create a `RollResult` with a repeated roll results.
    pub fn new_repeated(v: Vec<SingleRollResult>, total: Option<i64>) -> Self {
        RollResult {
            result: RollResultType::Repeated(RepeatedRollResult { rolls: v, total }),
            reason: None,
        }
    }

    /// Add a comment to the result.
    pub fn add_reason(&mut self, reason: String) {
        self.reason = Some(reason);
    }

    /// Get the comment, if any.
    pub fn get_reason(&self) -> Option<&String> {
        self.reason.as_ref()
    }

    /// Return the result.
    pub fn get_result(&self) -> &RollResultType {
        &self.result
    }

    /// If the result is a single roll, it will return it.
    pub fn as_single(&self) -> Option<&SingleRollResult> {
        match &self.result {
            RollResultType::Single(result) => Some(result),
            RollResultType::Repeated(_) => None,
        }
    }

    /// If the result is a repeated roll, it will return it.
    pub fn as_repeated(&self) -> Option<&RepeatedRollResult> {
        match &self.result {
            RollResultType::Single(_) => None,
            RollResultType::Repeated(results) => Some(results),
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
            RollResultType::Repeated(repeated_result) => match repeated_result.get_total() {
                Some(total) => {
                    (*repeated_result)
                        .iter()
                        .try_for_each(|res| writeln!(f, "`{}`", res.to_string_history()))?;
                    write!(f, "Sum: **{}**", total)?;
                    if let Some(reason) = &self.reason {
                        write!(f, ", Reason: `{}`", reason)?;
                    }
                }
                None => {
                    (*repeated_result)
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
