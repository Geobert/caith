use std::ops::Deref;

/// Used to mark a dice roll if its result is a critic.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Critic {
    /// Normal result
    No,
    /// Minimum reached
    Min,
    /// Maximum reached
    Max,
}

/// Carry one dice result and a marker field to say if it the result is a min, max, or none.
#[derive(Debug, Clone, Copy)]
pub struct DiceResult {
    /// The side of the dice that was rolled
    pub res: u64,
    /// If the result was remarkable (critic)
    pub crit: Critic,
}

impl DiceResult {
    /// Create a `DiceResult`.
    ///
    /// - `value`: value rolled on the dice
    /// - `sides`: number of sides of the dice
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