//! A recipe with a `locale` reads and writes its units through that locale wherever they are read, and the parser
//! never chooses one for it (ADR 7, ADR 8).

use cooklang::convert::{Audience, LocaleTag, Naming, Unit};
use cooklang::ingredient_list::IngredientList;
use cooklang::metadata::RecipeTime;
use cooklang::{
    Converter, CooklangParser, Extensions, Quantity, Recipe, RecipeResult, SemanticEq, Value,
};
use icu_locale_core::subtags::region;

fn parse(src: &str) -> RecipeResult {
    CooklangParser::new(Extensions::all()).parse(src)
}

fn recipe(src: &str) -> Recipe {
    let result = parse(src);
    assert!(!result.report().has_errors(), "{:?}", result.report());
    result.into_output().unwrap()
}

fn swedish(body: &str) -> Recipe {
    recipe(&format!(
        "---\nlocale: sv-SE\nprep time: 30 minuter\n---\n{body}"
    ))
}

fn errors(src: &str) -> Vec<String> {
    parse(src)
        .report()
        .errors()
        .map(|e| format!("{e:?}"))
        .collect()
}

fn warnings(src: &str) -> Vec<String> {
    parse(src)
        .report()
        .warnings()
        .map(|w| format!("{w:?}"))
        .collect()
}

fn warns_about(src: &str, text: &str) -> bool {
    warnings(src).iter().any(|w| w.contains(text))
}

/// Every quantity of a recipe as it prints, in document order.
fn written(recipe: &Recipe) -> Vec<String> {
    recipe
        .ingredients
        .iter()
        .filter_map(|i| i.quantity.as_ref())
        .chain(recipe.cookware.iter().filter_map(|c| c.quantity.as_ref()))
        .chain(recipe.timers.iter().filter_map(|t| t.quantity.as_ref()))
        .chain(&recipe.inline_quantities)
        .map(|q| q.to_string())
        .collect()
}

/// The parser is configured by its extensions alone: which locale a recipe is read through comes from the recipe, and
/// the converter it is then served by is its naming's.
#[test]
fn the_parser_chooses_no_locale() {
    let parser = CooklangParser::new(Extensions::all());
    assert_eq!(parser, CooklangParser::default());

    // an untagged recipe is read through the legacy locale: msk is Swedish, so it is not an inline quantity here
    let untagged = recipe("Rör ner 2 msk socker.\n");
    assert!(untagged.inline_quantities.is_empty());
    assert_eq!(untagged.naming(), Naming::Legacy);
    assert_eq!(untagged.converter(), Converter::legacy());
    assert_eq!(Converter::legacy().naming(), Naming::Legacy);

    // the same parser reads a tagged one through its own locale
    let tagged = recipe("---\nlocale: sv-SE\n---\nRör ner 2 msk socker.\n");
    assert_eq!(tagged.inline_quantities.len(), 1);
    assert_eq!(tagged.inline_quantities[0].unit(), Some("msk"));
    let sv = Naming::Tagged(LocaleTag::parse("sv-SE").unwrap());
    assert_eq!(tagged.naming(), sv);
    assert_eq!(tagged.converter(), Converter::for_naming(&sv).unwrap());
    assert_eq!(tagged.converter(), Converter::for_recipe(&tagged).unwrap());
    assert_eq!(tagged.converter().naming(), sv);
}

#[test]
fn a_reference_is_checked_against_the_locale_units() {
    // msk is a Swedish tablespoon: a volume, like dl, so the reference can be totalled
    let tagged = ">> locale: sv-SE\nVispa @mjölk{5%dl} med @&mjölk{2%msk}.\n";
    assert!(errors(tagged).is_empty(), "{:?}", errors(tagged));
    assert!(!warns_about(tagged, "Incompatible units"));
    // untagged, the legacy locale does not know msk
    let untagged = "Vispa @mjölk{5%dl} med @&mjölk{2%msk}.\n";
    assert!(warns_about(untagged, "Incompatible units"));
}

#[test]
fn a_timer_unit_is_read_through_the_locale() {
    assert!(errors(">> locale: sv-SE\nJäs i ~{1%tim}.\n").is_empty());
    assert!(errors("Jäs i ~{1%tim}.\n")
        .iter()
        .any(|e| e.contains("Unknown timer unit")));
    assert!(errors(">> locale: sv-SE\nJäs i ~{1%kg}.\n")
        .iter()
        .any(|e| e.contains("Timer unit is not time")));
}

#[test]
fn an_unreviewed_or_unknown_locale_is_a_parse_error() {
    let de = errors(">> locale: de\n@mehl{100%g}\n");
    assert!(
        de.iter().any(|e| e.contains("has not been reviewed")),
        "{de:?}"
    );
    let xh = errors(">> locale: xh\n@x{1%g}\n");
    assert!(xh.iter().any(|e| e.contains("no locale covers")), "{xh:?}");
}

/// The author of a refused recipe may have no checkout of this crate, so the error names what they can do in the
/// recipe itself before it says where a review happens.
#[test]
fn a_refused_locale_says_how_to_read_the_recipe_again() {
    let says_how = |src: &str| {
        let errors = errors(src);
        errors.iter().any(|e| {
            e.contains("Remove the `locale` key") && e.contains("locales/README.md in cooklang-rs")
        }) && !errors.iter().any(|e| e.contains("cargo run"))
    };
    // unreviewed, front matter and metadata line
    assert!(says_how("---\nlocale: de-DE\n---\n@mehl{100%g}\n"));
    assert!(says_how(">> locale: de-DE\n@mehl{100%g}\n"));
    // unknown
    assert!(says_how("---\nlocale: xh-ZA\n---\n@x{1%g}\n"));
}

/// Fitting units after scaling needs an audience, so a tag that gives none is refused up front rather than guessed at
/// later (ADR 7).
#[test]
fn a_tag_must_determine_an_audience() {
    let sv = errors(">> locale: sv\n@mjöl{1%dl}\n");
    assert!(
        sv.iter()
            .any(|e| e.contains("no region") && e.contains("-u-ms-metric")),
        "{sv:?}"
    );
    assert!(errors(">> locale: sv-SE\n@mjöl{1%dl}\n").is_empty());
    assert!(errors(">> locale: sv-u-ms-metric\n@mjöl{1%dl}\n").is_empty());
}

#[test]
fn a_malformed_locale_is_a_warning_like_any_other_bad_key() {
    let src = ">> locale: sv_SE\n@x{1%g}\n";
    assert!(errors(src).is_empty());
    assert!(warns_about(src, "Unsupported value for key: 'locale'"));
}

#[test]
fn an_untagged_recipe_is_told_to_add_a_locale() {
    let tell = |src: &str| warns_about(src, "no `locale`");
    assert!(tell("@flour{1%g}\n"));
    assert!(!tell("---\nlocale: sv-SE\n---\n@mjöl{1%g}\n"));
    // a locale that is refused, malformed or written too late has its own diagnostic, and the recipe does have one
    assert!(!tell("---\nlocale: xh-ZA\n---\n@x{1%g}\n"));
    assert!(!tell("---\nlocale: sv_SE\n---\n@x{1%g}\n"));
    assert!(!tell("@x{1%g}\n>> locale: sv-SE\n"));
    // the canonical parser is the same rule
    let canonical = CooklangParser::new(Extensions::empty()).parse("@flour{1%g}\n");
    assert!(canonical
        .report()
        .warnings()
        .any(|w| w.message.contains("no `locale`")));
}

/// What refuses a late `>> locale:` is a unit written above it, not a component: an ingredient with no quantity has
/// nothing to re-read. A refused line is still metadata, and nothing after it is read through the locale.
#[test]
fn a_locale_line_is_refused_only_after_a_unit_or_a_time_key() {
    let late = |src: &str| warns_about(src, "after") && warns_about(src, "locale");

    let no_unit_yet =
        "Add @salt.\nUse #pan{1}.\nWait ~{5}.\n>> locale: sv-SE\nRör ner 2 msk socker.\n";
    assert!(!late(no_unit_yet), "{:?}", warnings(no_unit_yet));
    assert_eq!(recipe(no_unit_yet).inline_quantities.len(), 1);

    for src in [
        "Add @salt{1%tsp}.\n>> locale: sv-SE\n",
        "Use #pan{1%large}.\n>> locale: sv-SE\n",
        "Wait ~{5%min}.\n>> locale: sv-SE\n",
        "Heat to 180°C.\n>> locale: sv-SE\n",
        ">> prep time: 30 min\n>> locale: sv-SE\n",
    ] {
        assert!(late(src), "{src:?}");
    }

    let refused = "Vispa @mjölk{5%dl}.\n>> locale: sv-SE\nRör ner 2 msk socker.\n";
    assert!(late(refused));
    let parsed = recipe(refused);
    assert!(parsed.inline_quantities.is_empty());
    assert_eq!(parsed.metadata.locale(), Some("sv-SE"));
}

/// Front matter is one block, so the locale applies to every key in it whatever order they are written in.
#[test]
fn a_front_matter_locale_applies_to_the_keys_above_it() {
    let above = "---\nprep time: 30 minuter\nlocale: sv-SE\n---\n@mjöl{1%g}\n";
    let below = "---\nlocale: sv-SE\nprep time: 30 minuter\n---\n@mjöl{1%g}\n";
    for src in [above, below] {
        assert!(
            !warns_about(src, "Unsupported value"),
            "{:?}",
            warnings(src)
        );
    }
    // the key itself is still checked and labelled in the loop
    let bad = "---\nprep time: 30 minuter\nlocale: sv_SE\n---\n@mjöl{1%g}\n";
    assert!(warns_about(bad, "Unsupported value for key: 'locale'"));
}

#[test]
fn time_is_read_through_the_recipes_own_locale() {
    let recipe = swedish("@mjöl{1%g}\n");
    assert_eq!(
        recipe.metadata.time(),
        Some(RecipeTime::Composed {
            prep_time: Some(30),
            cook_time: None
        })
    );
}

#[test]
fn grouping_adds_through_the_recipes_own_locale() {
    let recipe =
        swedish("Vispa @mjölk{5%dl} med @&mjölk{2%msk}.\nUse #skål{1%dl} and #&skål{1%msk}.\n");
    let grouped: Vec<String> = recipe
        .group_ingredients()
        .iter()
        .map(|g| g.quantity.to_string())
        .collect();
    assert_eq!(grouped, ["5.3 dl"]);
    let cookware: Vec<String> = recipe
        .group_cookware()
        .iter()
        .map(|g| g.quantity.to_string())
        .collect();
    assert_eq!(cookware, ["1.15 dl"]);
}

#[test]
fn equality_compares_the_units_the_recipes_were_read_with() {
    let a = swedish("@smör{1%msk}\n");
    let b = swedish("@smör{15%ml}\n");
    assert!(a.equals(&b));
    assert!(a.metadata.equals(&b.metadata));
    // a quantity built by hand has not been read, so it compares as text
    let msk = Quantity::new(Value::from(1.0), Some("msk".into()));
    let ml = Quantity::new(Value::from(15.0), Some("ml".into()));
    assert!(!msk.equals(&ml));
    let sv = a.converter();
    assert!(msk.read_with(&sv).equals(&ml.read_with(&sv)));
}

/// ADR 8: the parser records what each unit names, so nothing after it needs the locale again.
#[test]
fn the_parser_records_what_each_unit_names() {
    let tagged =
        swedish("Smält @smör{2%msk} i #kastrull{1%dl}. Rör ner 2 msk socker. Jäs i ~{1%tim}.\n");
    let id = |q: &Quantity| q.unit_id().map(Unit::id);
    assert_eq!(
        id(tagged.ingredients[0].quantity.as_ref().unwrap()),
        Some("metric tablespoon")
    );
    assert_eq!(
        id(tagged.cookware[0].quantity.as_ref().unwrap()),
        Some("deciliter")
    );
    assert_eq!(id(&tagged.inline_quantities[0]), Some("metric tablespoon"));
    assert_eq!(
        id(tagged.timers[0].quantity.as_ref().unwrap()),
        Some("hour")
    );
    // an untagged recipe reads through the legacy locale
    let untagged = recipe("@flour{1%cup} and @salt{1%pinch}\n");
    assert_eq!(
        id(untagged.ingredients[0].quantity.as_ref().unwrap()),
        Some("us cup")
    );
    assert_eq!(id(untagged.ingredients[1].quantity.as_ref().unwrap()), None);
}

#[test]
fn a_deserialised_recipe_is_read_again_through_its_own_locale() {
    let recipe = swedish("@smör{2%msk}\n");
    let json = serde_json::to_string(&recipe).unwrap();
    let stripped = json.replace(r#""unit_id":"metric tablespoon","#, "");
    assert_ne!(json, stripped);
    let mut back: Recipe = serde_json::from_str(&stripped).unwrap();
    assert_eq!(
        back.ingredients[0].quantity.as_ref().unwrap().unit_id(),
        None
    );
    back.read_units();
    assert!(back.equals(&recipe));
}

/// A Swedish recipe, written for a Swedish reader and for an American.
#[test]
fn a_swedish_recipe_end_to_end() {
    let recipe = recipe(
        "---\nlocale: sv-SE\n---\n\
         Vispa @mjölk{5%dl} med @smör{2%msk} och @salt{1%krm} i #kastrull{1%dl}. \
         Jäs i ~{1%tim}. Grädda i 200 °C.\n",
    );
    let tag = recipe.locale_tag().expect("tagged");

    // for a reader in Sweden: the region's own rows, the locale's names
    let mut for_sweden = recipe.clone();
    let errors = for_sweden.convert_for(tag.audience().unwrap());
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        written(&for_sweden),
        ["5 dl", "2 msk", "1 krm", "1 dl", "1 h", "200 °C"]
    );

    // for an American: US rows, still the recipe's own names; the US units sv declines to name are written by
    // identifier (ADR 4). 2 msk is 30 ml, a fluid ounce: the row above the tablespoon (ADR 5), where bundled would
    // have written 2 tbsp
    let mut for_america = recipe.clone();
    let errors = for_america.convert_for(Audience::region(region!("US")));
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        written(&for_america),
        [
            "17 us fluid ounce",
            "1 us fluid ounce",
            "0.203 us teaspoon",
            "3 1/2 us fluid ounce",
            "1 h",
            "392 °F"
        ]
    );
}

/// A list built from recipes in several locales adds what it recognises in one unit (ADR 7, "Aggregation converts
/// into the target first").
#[test]
fn a_swedish_and_an_untagged_recipe_add_up_in_the_target_locale() {
    let sv = Converter::for_tag(&LocaleTag::parse("sv-SE").unwrap()).unwrap();
    let swedish = recipe("---\nlocale: sv-SE\n---\nSmält @smör{2%msk}.\n");
    let untagged = recipe("Melt @smör{1%tbsp} and @mjöl{1%cup}.\n");

    let mut list = IngredientList::new();
    list.add_recipe(&swedish, &sv, false);
    list.add_recipe(&untagged, &sv, false);
    let entries: Vec<(String, String)> = list
        .iter()
        .map(|(name, group)| (name.clone(), group.to_string()))
        .collect();
    // a Swedish spoon and a US spoon are one amount, in the target's unit; a unit the target has no name for is
    // written by its identifier
    assert_eq!(
        entries,
        [
            ("smör".to_string(), "2.986 msk".to_string()),
            ("mjöl".to_string(), "1 us cup".to_string()),
        ]
    );

    // the legacy target names no metric spoon, so the sum is written by the unit's identifier, and the cup by
    // legacy's own symbol
    let legacy = Converter::legacy();
    let mut list = IngredientList::new();
    list.add_recipe(&swedish, &legacy, false);
    list.add_recipe(&untagged, &legacy, false);
    let entries: Vec<String> = list.iter().map(|(_, g)| g.to_string()).collect();
    assert_eq!(entries, ["2.986 metric tablespoon", "1 c"]);
}
