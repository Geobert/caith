#![no_main]
use libfuzzer_sys::fuzz_target;

use ::caith::Roller;
use ::mice::parse::parse_expression;

fuzz_target!(|data: &str| {
    if let Ok(roller) = Roller::new(data) {
        let _ = roller.roll();
    }
});
