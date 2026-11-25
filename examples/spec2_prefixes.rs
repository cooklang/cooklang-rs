//! cargo run --release --example spec2_prefixes -- recipes/*.cook
//! Parses every prefix of each file (what an editor sends while typing) and
//! builds its graph; reports any panic.
use cooklang::{spec2, Converter, CooklangParser, Extensions};
fn main() {
    let p = CooklangParser::new(Extensions::all(), Converter::bundled());
    let (mut runs, mut panics) = (0, 0);
    for path in std::env::args().skip(1) {
        let input = std::fs::read_to_string(&path).unwrap();
        for (i, _) in input
            .char_indices()
            .chain(std::iter::once((input.len(), ' ')))
        {
            let prefix = &input[..i];
            runs += 1;
            let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if let Some(r) = p.parse(prefix).into_output() {
                    spec2::graph(&r, &spec2::Selection::default());
                }
            }));
            if ok.is_err() {
                panics += 1;
                eprintln!("panic: {path} at byte {i}");
            }
        }
    }
    println!("{runs} prefixes, {panics} panics");
}
