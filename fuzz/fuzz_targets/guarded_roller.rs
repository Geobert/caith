#![no_main]
use libfuzzer_sys::fuzz_target;

use ::caith::Roller;
// I used the dice parser from my own dice crate to guard
// the parser from this crate from seeing invalid dice expressions.
// The dice parser from my crate only accepts a subset of this crate's
// dice language, but it works to demonstrate a failure inside
// the roller itself, beyond the parsing problems.
use ::mice::parse::parse_expression;

fuzz_target!(|data: &str| {
    if let Ok((rest, _)) = parse_expression(data.as_bytes()) {
        if rest.is_empty() {
            if let Ok(roller) = Roller::new(data) {
                let _ = roller.roll();
            }
        }
    }
});
