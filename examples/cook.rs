fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let bin = args.next().unwrap();
    let in_file = match args.next() {
        Some(path) => path,
        None => panic!("Usage: {bin} <input_file> [output_file|STDOUT]"),
    };
    let out_file: Option<Box<dyn std::io::Write>> = match args.next().as_deref() {
        Some("STDOUT") => Some(Box::new(std::io::stdout().lock())),
        Some(path) => Some(Box::new(std::fs::File::create(path)?)),
        None => None,
    };

    let input = std::fs::read_to_string(&in_file)?;

    match cooklang::parse(&input).into_result() {
        Ok((recipe, warnings)) => {
            warnings.eprint(&in_file, &input, true)?;
            if let Some(mut out) = out_file {
                write!(out, "{:#?}", recipe)?;
            }
        }
        Err(e) => {
            e.eprint(&in_file, &input, true)?;
            Err("failed to parse")?;
        }
    }
    Ok(())
}
