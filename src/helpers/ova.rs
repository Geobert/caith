use crate::{error::*, RollHistory, RollResult, SingleRollResult};

/// Interpret the roll result as OVA: The Anime Role-Playing Game result
///
/// ex:
/// ```
/// use caith::*;
///
/// let number: i64 = -4;
/// let roller = Roller::new(&format!("{}d6", number.abs())).unwrap();
/// let res = roller.roll().unwrap();
/// println!("{}", compute_ova(&res, number).unwrap());
/// ```
///
pub fn compute_ova(res: &RollResult, number: i32) -> Result<RollResult> {
    if number == 0 {
        return Err("Number can't be zero".into());
    }

    let res = res
        .as_single()
        .ok_or("Not a single roll result")?
        .get_history();
    if res.len() != 1 {
        Err("Should have only one roll".into())
    } else {
        let mut res = res
            .iter()
            .flat_map(|v| {
                if let RollHistory::Roll(dices_res) = v {
                    Some(dices_res)
                } else {
                    None
                }
            })
            .next()
            .ok_or("RollHistory must be a Roll variant")?
            .clone();
        res.sort_unstable();
        let total = if number > 0 {
            let mut last_side = 0;
            let mut current_res = 0;
            res.iter().fold(0, |acc, current| {
                if last_side != current.res {
                    last_side = current.res;
                    if acc > current_res {
                        current_res = acc;
                    }
                    current.res
                } else {
                    acc + current.res
                }
            });
            current_res
        } else {
            res.first()
                .expect("Impossible, that mean we rolled 0 dices")
                .res
        };

        Ok(RollResult::new_single(
            SingleRollResult::with_total_and_hist(total, res),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{rollresult, tests::IteratorDiceRollSource, Roller};

    use super::*;

    #[test]
    fn ova_test() {
        // positive 12
        // same values are added together, and we keep the highest result
        let roll_mock = vec![1, 2, 2, 2, 3, 3, 3, 4, 5, 5, 5, 6];
        let r = Roller::new("12d6").unwrap();
        let roll_res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut roll_mock.into_iter(),
            })
            .unwrap();
        let roll_res = compute_ova(&roll_res, 12).unwrap();
        match roll_res.get_result() {
            rollresult::RollResultType::Single(res) => assert_eq!(15, res.get_total()),
            rollresult::RollResultType::Repeated(_) => unreachable!(),
        }
        eprintln!("{}", roll_res);

        // negative 5
        // roll 5 dices, keep the lowest
        let roll_mock = vec![1, 3, 3, 5, 5];
        let r = Roller::new("5d6").unwrap();
        let roll_res = r
            .roll_with_source(&mut IteratorDiceRollSource {
                iterator: &mut roll_mock.into_iter(),
            })
            .unwrap();
        let roll_res = compute_ova(&roll_res, -5).unwrap();
        match roll_res.get_result() {
            rollresult::RollResultType::Single(res) => assert_eq!(1, res.get_total()),
            rollresult::RollResultType::Repeated(_) => unreachable!(),
        }

        eprintln!("{}", roll_res);
    }
}
