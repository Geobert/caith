use std::ops::Deref;

use crate::rollresult::SingleRollResult;

/// Represent a repeated roll.
///
/// Can store the sum of all the roll if asked to. Usually created through
/// [`RollResult::new_repeated()`] function.
#[derive(Debug, Clone)]
pub struct RepeatedRollResult {
    pub(crate) rolls: Vec<SingleRollResult>,
    pub(crate) total: Option<i64>,
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
