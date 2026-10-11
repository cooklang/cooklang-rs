//! Tasks that run rarely and must not touch the library's build.
//!
//! ```sh
//! xtask/new-locale.sh hu       # start a locale we do not have
//! ```
//!
//! There is only the one task, and deliberately no command to re-render a
//! locale we already have: those files carry hand-written comments and
//! ordering, and a canonical re-render would flatten them. Once
//! `locales/<tag>.toml` exists it is edited by hand, which is the point of
//! ADR 0006.

mod seed;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, rest) = args
        .split_first()
        .map(|(c, r)| (c.as_str(), r))
        .unwrap_or(("", &[]));
    match cmd {
        "new-locale" => new_locale(rest),
        _ => {
            eprintln!("usage: cargo xtask new-locale <tag> <version> <units.json> <plurals.json>");
            eprintln!();
            eprintln!("Normally run through xtask/new-locale.sh, which fetches the two JSON");
            eprintln!("files first. There is no command to re-render an existing locale: those");
            eprintln!("files are edited by hand and carry comments a re-render would flatten.");
            std::process::ExitCode::from(2)
        }
    }
}

/// Starts a locale we do not have, from CLDR's own JSON.
///
/// The JSON arrives as files rather than being fetched here: downloading is
/// `new-locale.sh`'s job, so this needs no HTTP client and the fetched data
/// stays a temporary file that is never committed.
fn new_locale(args: &[String]) -> std::process::ExitCode {
    let [tag, version, units, plurals] = args else {
        eprintln!("usage: cargo xtask new-locale <tag> <version> <units.json> <plurals.json>");
        return std::process::ExitCode::from(2);
    };

    let read = |path: &String| match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) => {
            eprintln!("{path}: {error}");
            None
        }
    };
    let (Some(units), Some(plurals)) = (read(units), read(plurals)) else {
        return std::process::ExitCode::FAILURE;
    };

    // CLDR keys most plural rules by language alone, so a regional tag falls
    // back to its language's rules.
    let rules: serde_json::Value = match serde_json::from_str(&plurals) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("plurals.json: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let rules = &rules["supplemental"]["plurals-type-cardinal"];
    let language = tag.split('-').next().unwrap_or(tag);
    let for_tag = match (rules.get(tag), rules.get(language)) {
        (Some(r), _) | (None, Some(r)) => r,
        _ => {
            eprintln!("plurals.json has no rules for '{tag}' or '{language}'");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut plural_rules: Vec<(String, String)> = for_tag
        .as_object()
        .into_iter()
        .flat_map(|o| o.iter())
        .filter_map(|(field, value)| {
            let category = field.strip_prefix("pluralRule-count-")?;
            // the samples after `@` are CLDR's own test data, not a condition
            let condition = value.as_str()?.split('@').next().unwrap_or("").trim();
            Some((category.to_string(), condition.to_string()))
        })
        .collect();
    plural_rules.sort();

    match seed::render_new_locale(tag, version, &units, &plural_rules) {
        Err(error) => {
            eprintln!("{tag}: {error}");
            std::process::ExitCode::FAILURE
        }
        Ok(text) => {
            let path = format!("locales/{tag}.toml");
            if std::path::Path::new(&path).exists() {
                eprintln!("{path} already exists and is edited by hand; nothing to do");
                return std::process::ExitCode::FAILURE;
            }
            if let Err(error) = std::fs::write(&path, &text) {
                eprintln!("{path}: {error}");
                return std::process::ExitCode::FAILURE;
            }
            println!("{path} ({} lines)", text.lines().count());
            println!("Not reviewed: cooklang will refuse to serve '{tag}' until a");
            println!("`reviewed` key is added. See locales/{tag}.toml for what to do.");
            std::process::ExitCode::SUCCESS
        }
    }
}
