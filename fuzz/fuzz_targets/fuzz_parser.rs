#![no_main]

use libfuzzer_sys::fuzz_target;

use cooklang::{CooklangParser, Extensions, Converter};

fuzz_target!(|contents: &str| {
    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let _ = parser.parse(&contents);
});
