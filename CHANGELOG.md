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