use std::time::Instant;

use cooklang::CooklangParser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let bin = args.next().unwrap();
    let in_file = match args.next() {
        Some(path) => path,
        None => panic!("Usage: {bin} <input_file> [extended] [n]"),
    };
    let extended = match args.next().as_deref() {
        Some("true") => true,
        Some("false") => false,
        Some(_) => panic!("extended must be true or false"),
        None => true,
    };
    let n = match args.next() {
        Some(s) => s.parse::<u128>()?,
        None => 100,
    };

    let input = std::fs::read_to_string(&in_file)?;

    let parser = if extended {
        println!("extended parser");
        CooklangParser::extended()
    } else {
        println!("canonical parser");
        CooklangParser::canonical()
    };

    // warmup
    for _ in 0..n {
        parser.parse(&input);
    }

    let start = Instant::now();
    for _ in 0..n {
        parser.parse(&input);
    }
    let elapsed = start.elapsed();

    println!("{n} runs");
    println!("{} us", (elapsed.as_nanos() / n) as f64 / 1000.0);
    Ok(())
}
