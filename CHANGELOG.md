# 4.1.1
- FIX: can put signed number in expr again

# 4.1.0
- For `cde` helpers, if the element was passed in French, the printing of the result will
  be in French as well

# 4.0.0
- BREAKING: remove `ova()` syntax from grammar, I want to avoid cluttering grammar with
  multiple interpretations of the roll result. I let the user of the crate to detect an
  OVA roll, and use helpers::compute_ova to get the result. More interpretations to come.
  Moreover, helpers are not active by default.
- NEW: target accepts enumeration of values to consider success: `3d6 t[2,4,6]` will count
  even dice as success.
- NEW: can make a deck of cards and draw cards from it. Feature gated under `cards`.

# 3.1.0
- NEW: double target `tt` option, see [@JamesLaverack's
  PR](https://github.com/Geobert/caith/pull/3)
- Enhance tests thanks to this PR.

# 3.0.3
- FIX: Total was 0 if expression contain only one dice.

# 3.0.1 and 3.0.2
- FIX: Total was 0 if expression contain only one number.

# 3.0.0
- NEW & BREAKING: can use float constant. Ex: `D6 * 1.5`. This small addition change the
  `RollResult` structure a little bit, as `RollHistory::Value` variant now holds a
  `Value` instead of `i64`. Hence the major version bump.

# 2.2.1
- FIX: issue #2

# 2.2.0
- NEW: added `Critic` to mark a dice roll as critic

# 2.1.0
- NEW: added `Roller::roll_with` in order to provide an external `rand::Rng` (idea by
  [rardiol](https://github.com/rardiol))
- NEW: `Roller` derives `Debug`

# 2.0.1
- supports for signed number after an operator: `d20 + -4` is accepted.

# 2.0.0
- NEW & BREAKING: support repeat a roll x times: `(2d6 + 6) ^ 8` will roll `2d6 + 6` eight times
    - to store this, `RollResult` has been refactored and break a bit of the API. To
      convert your code, either you were only using the `impl Display` and nothing change,
      but if you were calling some method on `RollResult` you'll need to adapt your code:
      ```rust
      let result = Roller::new("1d6 : initiative").unwrap().roll().unwrap();
      let result = result.as_single().unwrap();
      // old code
      ```
- NEW: repeated rolls can be summed: `(2d6 + 2) ^+ 6` will roll `2d6 + 2` six times and
  sum all the results.
- NEW: repeated rolls can be sorted: `(2d6 + 2) ^# 6` will sort the rolls
- NEW: OVA roll support: `ova(12)` or `ova(-5)`
- CHANGED: When specifying an explosion value, if the dice result is >= the value, it
  explodes. (It was exploding on exactly the value before, which made no sense)
- CHANGED: When printing SingleRollResult, "Result" was replaced by "="

# 1.0.1
- RollResult and RollHistory are `Clone` (by [mcpar-land](https://github.com/mcpar-land))

# 1.0.0
- Add `Roller` to expose stable API for future optimization
- BREAKING: Removed free function `roll` and `find_first_dice`, use `Roller` to get all the dices
- BREAKING: reason starts with `:` instead of `!` 
- `!` is now an alias for `ie`
- on both exploding dice (`!` and `e`), if number is omitted, default to dice max
- remove dep to `once_cell`

# 0.5.0
- Add `find_first_dice`: look for the first dice of the expression and return it

# 0.4.0
- Support for Fudge/Fate dice

# 0.3.0
- Result is printed bold
- Keep operations as separator
- Keep literal value in the result display

# 0.2.0
- Error reporting
- Accept no nb dice (default to 1): `d6` == `1d6`
- Accept uppercase `D`

# 0.1.x
- First release, subsequent releases were metadata changes for crates.io