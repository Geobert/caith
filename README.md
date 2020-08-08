[![Docs](https://docs.rs/caith/badge.svg)](https://docs.rs/caith)
[![Crates.io](https://img.shields.io/crates/d/caith.svg)](https://crates.io/crates/caith)
[![Crates.io](https://img.shields.io/crates/v/caith.svg)](https://crates.io/crates/caith)

# Caith

A dice roller library written in Rust.

The different features are totally inspired by https://github.com/Humblemonk/DiceMaiden 

# Usage

```rust
use caith::{roll, RollResult};

// ...
let result = roll("1d6 ! initiative")?;

printf("{} ({})", result.get_total(), result.get_reason().unwrap());

printf("{}", result);
```

# Syntax

```
xdy [OPTIONS] [TARGET] [FAILURE] [! REASON]

roll `x` dice(s) with `y` sides

`y` can also be "F" or "f" for fudge dice. In this case, no option applies if provided they will be ignored.

Options:
+ - / * : modifiers
e# : Explode value
ie# : Indefinite explode value
K#  : Keeping # highest (upperacse "K")
k#  : Keeping # lowest (lowercase "k")
D#  : Dropping the highest (uppercase "D")
d#  : Dropping the lowest (lowercase "d")
r#  : Reroll if <= value
ir# : Indefinite reroll if <= value

Target:
t# : minimum value to count as success

Failure:
f# : value under which it's counted as failure

Reason:
! : Any text after `!` will be a comment
```

# Examples

These examples are directly taken from DiceMaiden's Readme:

`2d6 + 3d10` : Roll two six-sided dice and three ten-sided dice.

`3d6 + 5` : Roll three six-sided dice and add five. Other supported static modifiers are add (+), subtract (-), multiply (*), and divide (/).

`3d6 e6` : Roll three six-sided dice and explode on sixes. Some game systems call this 'open ended' dice. If the number rolled is greater than or equal to the value given for this option, the die is rolled again and added to the total. If no number is given for this option, it is assumed to be the same as the number of sides on the die. Thus, '3d6 e' is the same as '3d6 e6'. The dice will only explode once with this command. Use "ie" for indefinite explosions.

`3d6 ie6` : Roll three six-sided dice and explode on sixes indefinitely within reason. We will cap explosions at 100 rolls to prevent abuse.

`3d10 d1` : Roll three ten-sided dice and drop one die. The lowest value will be dropped first.  **NOTE:** These dice are dropped before any dice are kept with the following `k` command. Order of operations is : roll dice, drop dice, keep dice

`3d10 K2` : Roll three ten-sided dice and keep two. The highest value rolled will be kept.
Using lowercase `k` will keep the lowest.

`4d6 r2` : Roll four six-sided dice and reroll any that are equal to or less than two once. Use ir for indefinite rerolls.

`4d6 ir2` : Roll four six-sided dice and reroll any that are equal to or less than two (and do the same to those dice). This is capped at 100 rerolls per die to prevent abuse.

`6d10 t7` : Roll six ten-sided dice and any that are seven or higher are counted as a success. The dice in the roll are not added together for a total. Any die that meets or exceeds the target number is added to a total of successes.

`5d10 t8 f1` : f# denotes a failure number that each dice must match or be beneath in order to count against successes. These work as a sort of negative success and are totaled together as described above. In the example roll, roll five ten-sided dice and each dice that is 8 or higher is a success and subtract each one. The total may be negative. If the option is given a 0 value, that is the same as not having the option at all thus a normal sum of all dice in the roll is performed instead.

`4d10 k3` : Roll four ten-sided dice and keep the lowest three dice rolled.

`4d6 ! Hello World!`: Roll four six-sided dice and add comment to the roll.

These commands can be combined. For example:

`10d6 e6 K8 +4` : Roll ten six-sided dice , explode on sixes and keep eight of the highest rolls and add four.
