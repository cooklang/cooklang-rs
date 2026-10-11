//! Prints one locale of the in-progress v2 converter as Markdown.
//!
//! ```sh
//! cargo run --example show-locale-as-md -- pl
//! cargo run --example show-locale-as-md -- en-NZ > en-NZ.md
//! ```

fn main() {
    let mut args = std::env::args();
    let bin = args
        .next()
        .unwrap_or_else(|| "show-locale-as-md".to_string());
    let (Some(tag), None) = (args.next(), args.next()) else {
        eprintln!("Usage: {bin} <bcp47-tag>");
        std::process::exit(2);
    };
    match cooklang::locale_markdown(&tag) {
        Ok(markdown) => print!("{markdown}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
