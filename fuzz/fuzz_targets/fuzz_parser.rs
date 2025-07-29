#![no_main]

use libfuzzer_sys::fuzz_target;

use cooklang::{Converter, CooklangParser, Extensions};

fuzz_target!(|contents: &str| {
    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let _ = parser.parse(&contents);
});
