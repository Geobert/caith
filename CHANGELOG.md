2.0.0
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

1.0.1
- RollResult and RollHistory are `Clone` (by mcpar-land)

1.0.0
- Add `Roller` to expose stable API for future optimization
- BREAKING: Removed free function `roll` and `find_first_dice`, use `Roller` to get all the dices
- BREAKING: reason starts with `:` instead of `!` 
- `!` is now an alias for `ie`
- on both exploding dice (`!` and `e`), if number is omitted, default to dice max
- remove dep to `once_cell`

0.5.0
- Add `find_first_dice`: look for the first dice of the expression and return it

0.4.0
- Support for Fudge/Fate dice

0.3.0
- Result is printed bold
- Keep operations as separator
- Keep literal value in the result display

0.2.0
- Error reporting
- Accept no nb dice (default to 1): `d6` == `1d6`
- Accept uppercase `D`

0.1.x
- First release, subsequent releases were metadata changes for crates.io