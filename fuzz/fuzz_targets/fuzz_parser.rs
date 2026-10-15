#![no_main]

use libfuzzer_sys::fuzz_target;

use cooklang::{CooklangParser, Extensions};

fuzz_target!(|contents: &str| {
    let parser = CooklangParser::new(Extensions::all());
    let _ = parser.parse(&contents);
});
