use super::units::Unit;
use super::Converter;
use crate::quantity::{Quantity, Value};

#[test]
fn convert_preserves_scalable() {
    // converting rebuilds the quantity, and must not lose the flag: the legacy converter went through `Quantity::new`,
    // which resets it
    let quantity = Quantity {
        value: Value::from(1.0),
        unit: Some(crate::quantity::WrittenUnit::unresolved("l")),
        scalable: true,
    };
    let milliliter = Unit::from_id("milliliter").unwrap();

    let quantity = quantity.read_with(&Converter::legacy());
    let converted = Converter::legacy().convert(&quantity, milliliter).unwrap();
    assert!(converted.scalable());
    assert_eq!(converted.unit(), Some("ml"));
}

mod convert_for {
    use super::*;
    use crate::convert::locales::preference::{Audience, System};
    use crate::convert::ConvertError;
    use crate::quantity::Number;
    use icu_locale_core::subtags::region;

    /// Read through the legacy locale, as the parser would read it.
    fn q(value: Value, unit: &str) -> Quantity {
        Quantity::new(value, Some(unit.to_string())).read_with(&Converter::legacy())
    }

    fn range(start: f64, end: f64) -> Value {
        Value::Range {
            start: Number::Regular(start),
            end: Number::Regular(end),
        }
    }

    /// The numbers in a value, so a range can be checked end by end.
    fn numbers(q: &Quantity) -> Vec<f64> {
        match q.value() {
            Value::Number(n) => vec![n.value()],
            Value::Range { start, end } => vec![start.value(), end.value()],
            Value::Text(t) => panic!("text value '{t}'"),
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn the_audience_chooses_the_unit() {
        let v2 = Converter::legacy();
        let metric = Audience::system(System::Metric);
        let us = Audience::region(region!("US"));

        let out = v2
            .convert_for(&q(Value::from(1500.0), "ml"), metric)
            .unwrap();
        assert_eq!(out.unit(), Some("l"));
        assert!(close(numbers(&out)[0], 1.5));

        // 500 ml is 2.11 cups, 5.7 % over two and so no fraction of a cup;
        // it falls to fluid ounces, where 16.9 rounds to a whole
        let out = v2.convert_for(&q(Value::from(500.0), "ml"), us).unwrap();
        assert_eq!(out.to_string(), "17 fl oz");
        assert!(close(numbers(&out)[0], 16.907), "{out}");

        // a US reader asking for metric gets millilitres, not a cup
        let out = v2.convert_for(&q(Value::from(1.0), "cup"), metric).unwrap();
        assert_eq!(out.unit(), Some("ml"));
        assert!(close(numbers(&out)[0], 236.588), "{out}");
    }

    #[test]
    fn a_range_is_judged_by_its_start() {
        let v2 = Converter::legacy();
        let metric = Audience::system(System::Metric);
        let us = Audience::region(region!("US"));

        let out = v2
            .convert_for(&q(range(900.0, 1100.0), "ml"), metric)
            .unwrap();
        assert_eq!(out.unit(), Some("ml"));
        assert_eq!(numbers(&out), [900.0, 1100.0]);

        let out = v2.convert_for(&q(range(1.0, 2.0), "l"), us).unwrap();
        assert_eq!(out.to_string(), "4 1/4-8 1/2 c");
        let ends = numbers(&out);
        assert!(close(ends[0], 4.2268) && close(ends[1], 8.4535), "{out}");
    }

    #[test]
    fn the_errors_are_the_same_as_convert() {
        let v2 = Converter::legacy();
        let metric = Audience::system(System::Metric);
        assert!(matches!(
            v2.convert_for(
                &Quantity::new(Value::from(1.0), None).read_with(&v2),
                metric
            ),
            Err(ConvertError::NoUnit)
        ));
        assert!(matches!(
            v2.convert_for(&q(Value::Text("some".into()), "l"), metric),
            Err(ConvertError::TextValue(_))
        ));
        assert!(matches!(
            v2.convert_for(&q(Value::from(1.0), "bunch"), metric),
            Err(ConvertError::UnknownUnit(ref u)) if u == "bunch"
        ));
    }
}

/// A recipe tagged with a reviewed locale reads its units through it.
mod parsed {
    use crate::{CooklangParser, Extensions};

    fn parse(src: &str) -> crate::RecipeResult {
        CooklangParser::new(Extensions::all()).parse(src)
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

    #[test]
    fn a_reference_is_checked_against_the_locale_units() {
        // msk is a Swedish tablespoon: a volume, like dl, so the reference
        // can be totalled
        let tagged = ">> locale: sv-SE\nVispa @mjölk{5%dl} med @&mjölk{2%msk}.\n";
        assert!(errors(tagged).is_empty(), "{:?}", errors(tagged));
        assert!(!warns_about(tagged, "Incompatible units"));
        // untagged, the legacy locale does not know msk
        let untagged = "Vispa @mjölk{5%dl} med @&mjölk{2%msk}.\n";
        assert!(warns_about(untagged, "Incompatible units"));
    }

    #[test]
    fn an_inline_quantity_is_found_through_the_locale() {
        let tagged = ">> locale: sv-SE\nRör ner 2 msk socker.\n";
        let recipe = parse(tagged).into_output().unwrap();
        assert_eq!(recipe.inline_quantities.len(), 1);
        assert_eq!(recipe.inline_quantities[0].unit(), Some("msk"));

        let untagged = "Rör ner 2 msk socker.\n";
        let recipe = parse(untagged).into_output().unwrap();
        assert!(recipe.inline_quantities.is_empty());
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
    fn front_matter_sets_the_locale_too() {
        let src = "---\nlocale: sv-SE\n---\nRör ner 2 msk socker.\n";
        let recipe = parse(src).into_output().unwrap();
        assert_eq!(recipe.inline_quantities.len(), 1);
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

    /// Fitting units after scaling needs an audience, so a tag that gives none is refused up front rather than guessed
    /// at later (ADR 7).
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

    /// A Swedish recipe, written for a Swedish reader and for an American.
    #[test]
    fn a_swedish_recipe_end_to_end() {
        use super::Converter;
        use crate::convert::locales::preference::Audience;
        use icu_locale_core::subtags::region;

        let src = "---\nlocale: sv-SE\n---\n\
            Vispa @mjölk{5%dl} med @smör{2%msk} och @salt{1%krm} i #kastrull{1%dl}. \
            Jäs i ~{1%tim}. Grädda i 200 °C.\n";
        let result = parse(src);
        assert!(!result.report().has_errors(), "{:?}", result.report());
        let recipe = result.into_output().unwrap();
        let tag = recipe.locale_tag().expect("tagged");
        // the recipe reads and writes through its own locale, so nothing is passed in; the tag only says who the second
        // rendering is for
        assert_eq!(recipe.converter(), Converter::for_tag(&tag).unwrap());

        let written = |recipe: &crate::Recipe| -> Vec<String> {
            recipe
                .ingredients
                .iter()
                .filter_map(|i| i.quantity.as_ref())
                .chain(recipe.cookware.iter().filter_map(|c| c.quantity.as_ref()))
                .chain(recipe.timers.iter().filter_map(|t| t.quantity.as_ref()))
                .chain(&recipe.inline_quantities)
                .map(|q| q.to_string())
                .collect()
        };

        // for a reader in Sweden: the region's own rows, the locale's names
        let mut swedish = recipe.clone();
        let errors = swedish.convert_for(tag.audience().unwrap());
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(
            written(&swedish),
            ["5 dl", "2 msk", "1 krm", "1 dl", "1 h", "200 °C"]
        );

        // for an American: US rows, still the recipe's own names; the US units sv declines to name are written by
        // identifier (ADR 4). 2 msk is 30 ml, a fluid ounce: the row above the tablespoon (ADR 5), where bundled would
        // have written 2 tbsp
        let mut american = recipe.clone();
        let errors = american.convert_for(Audience::region(region!("US")));
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(
            written(&american),
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

    #[test]
    fn a_malformed_locale_is_a_warning_like_any_other_bad_key() {
        let src = ">> locale: sv_SE\n@x{1%g}\n";
        assert!(errors(src).is_empty());
        assert!(warns_about(src, "Unsupported value for key: 'locale'"));
    }
}

/// What the parser says about `locale` itself (ADR 7).
mod locale_warnings {
    use crate::{CooklangParser, Extensions};

    fn warnings(parser: &CooklangParser, src: &str) -> Vec<String> {
        parser
            .parse(src)
            .report()
            .warnings()
            .map(|w| w.message.to_string())
            .collect()
    }

    #[test]
    fn an_untagged_recipe_is_told_to_add_a_locale_on_the_new_pipeline_only() {
        let tell = |w: &[String]| w.iter().any(|m| m.contains("no `locale`"));
        let new = CooklangParser::new(Extensions::all());
        assert!(tell(&warnings(&new, "@flour{1%g}\n")));
        assert!(!tell(&warnings(
            &new,
            "---\nlocale: sv-SE\n---\n@mjöl{1%g}\n"
        )));
        // the canonical parser is the same rule
        assert!(tell(&warnings(
            &CooklangParser::new(Extensions::empty()),
            "@flour{1%g}\n"
        )));
    }

    /// What refuses a late `>> locale:` is a unit written above it, not a component: an ingredient with no quantity has
    /// nothing to re-read.
    #[test]
    fn a_locale_line_is_refused_only_after_a_unit_or_a_time_key() {
        let new = CooklangParser::new(Extensions::all());
        let late = |m: &String| m.contains("after") && m.contains("locale");

        let no_unit_yet =
            "Add @salt.\nUse #pan{1}.\nWait ~{5}.\n>> locale: sv-SE\nRör ner 2 msk socker.\n";
        let result = new.parse(no_unit_yet);
        let w: Vec<String> = result
            .report()
            .warnings()
            .map(|w| w.message.to_string())
            .collect();
        assert!(!w.iter().any(late), "{w:?}");
        assert_eq!(result.into_output().unwrap().inline_quantities.len(), 1);

        for src in [
            "Add @salt{1%tsp}.\n>> locale: sv-SE\n",
            "Use #pan{1%large}.\n>> locale: sv-SE\n",
            "Wait ~{5%min}.\n>> locale: sv-SE\n",
            "Heat to 180°C.\n>> locale: sv-SE\n",
            ">> prep time: 30 min\n>> locale: sv-SE\n",
        ] {
            assert!(warnings(&new, src).iter().any(late), "{src:?}");
        }
    }

    /// Front matter is one block, so the locale applies to every key in it whatever order they are written in.
    #[test]
    fn a_front_matter_locale_applies_to_the_keys_above_it() {
        let new = CooklangParser::new(Extensions::all());
        let above = "---\nprep time: 30 minuter\nlocale: sv-SE\n---\n@mjöl{1%g}\n";
        let below = "---\nlocale: sv-SE\nprep time: 30 minuter\n---\n@mjöl{1%g}\n";
        for src in [above, below] {
            let w = warnings(&new, src);
            assert!(
                !w.iter().any(|m| m.contains("Unsupported value")),
                "{src:?}: {w:?}"
            );
        }
        // the key itself is still checked and labelled in the loop
        let bad = "---\nprep time: 30 minuter\nlocale: sv_SE\n---\n@mjöl{1%g}\n";
        let w = warnings(&new, bad);
        assert!(
            w.iter()
                .any(|m| m.contains("Unsupported value for key: 'locale'")),
            "{w:?}"
        );
    }

    #[test]
    fn a_locale_line_after_the_first_quantity_is_ignored_with_a_warning() {
        let new = CooklangParser::new(Extensions::all());
        let src = "Vispa @mjölk{5%dl}.\n>> locale: sv-SE\nRör ner 2 msk socker.\n";
        let result = new.parse(src);
        let warnings: Vec<String> = result
            .report()
            .warnings()
            .map(|w| w.message.to_string())
            .collect();
        assert!(
            warnings
                .iter()
                .any(|m| m.contains("after") && m.contains("locale")),
            "{warnings:?}"
        );
        // msk is not read through the Swedish locale, so no inline quantity
        let recipe = result.into_output().unwrap();
        assert!(recipe.inline_quantities.is_empty());
        // the key is still metadata
        assert_eq!(recipe.metadata.locale(), Some("sv-SE"));

        // before any quantity, a `>>` line is as good as front matter
        let src = ">> locale: sv-SE\nVispa @mjölk{5%dl}.\nRör ner 2 msk socker.\n";
        let recipe = new.parse(src).into_output().unwrap();
        assert_eq!(recipe.inline_quantities.len(), 1);
    }
}

/// A recipe's naming, which the parser never chooses (ADR 7).
mod naming {
    use crate::convert::{LocaleTag, Naming};
    use crate::{Converter, CooklangParser, Extensions};

    #[test]
    fn a_recipe_names_its_locale_or_the_legacy_one() {
        let parser = CooklangParser::new(Extensions::all());
        let untagged = parser.parse("@flour{1%g}\n").into_output().unwrap();
        assert_eq!(untagged.naming(), Naming::Legacy);
        let tagged = parser
            .parse("---\nlocale: sv-SE\n---\n@mjöl{1%g}\n")
            .into_output()
            .unwrap();
        assert_eq!(
            tagged.naming(),
            Naming::Tagged(LocaleTag::parse("sv-SE").unwrap())
        );
        // the converter a naming calls for is the one the recipe is served by
        let sv = Converter::for_naming(&tagged.naming()).unwrap();
        assert_eq!(sv.naming(), tagged.naming());
        assert_eq!(Converter::for_recipe(&tagged).unwrap(), sv);
        assert_eq!(Converter::legacy().naming(), Naming::Legacy);
    }

    /// The parser is configured by its extensions alone: which locale a recipe is read through comes from the recipe.
    #[test]
    fn the_parser_chooses_no_locale() {
        let parser = CooklangParser::new(Extensions::all());
        assert_eq!(parser, CooklangParser::default());

        // an untagged recipe is read through the legacy locale: msk is Swedish, so it is not an inline quantity here
        let untagged = parser
            .parse("Rör ner 2 msk socker.\n")
            .into_output()
            .unwrap();
        assert!(untagged.inline_quantities.is_empty());
        assert_eq!(untagged.naming(), Naming::Legacy);

        // the same parser reads a tagged one through its own locale
        let tagged = parser
            .parse("---\nlocale: sv-SE\n---\nRör ner 2 msk socker.\n")
            .into_output()
            .unwrap();
        assert_eq!(tagged.inline_quantities.len(), 1);
        assert_eq!(
            tagged.naming(),
            Naming::Tagged(LocaleTag::parse("sv-SE").unwrap())
        );
    }
}

/// A recipe reads its units through its own naming wherever they are read, not only at parse time: a caller cannot hand
/// it the wrong converter.
mod own_naming {
    use crate::metadata::RecipeTime;
    use crate::{Converter, CooklangParser, Extensions, Quantity, SemanticEq, Value};

    fn swedish(body: &str) -> crate::Recipe {
        let parser = CooklangParser::new(Extensions::all());
        let src = format!("---\nlocale: sv-SE\nprep time: 30 minuter\n---\n{body}");
        let result = parser.parse(&src);
        assert!(!result.report().has_errors(), "{:?}", result.report());
        result.into_output().unwrap()
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
        use crate::convert::Unit;
        let recipe = swedish(
            "Smält @smör{2%msk} i #kastrull{1%dl}. Rör ner 2 msk socker. Jäs i ~{1%tim}.\n",
        );
        let id = |q: &Quantity| q.unit_id().map(Unit::id);
        assert_eq!(
            id(recipe.ingredients[0].quantity.as_ref().unwrap()),
            Some("metric tablespoon")
        );
        assert_eq!(
            id(recipe.cookware[0].quantity.as_ref().unwrap()),
            Some("deciliter")
        );
        assert_eq!(id(&recipe.inline_quantities[0]), Some("metric tablespoon"));
        assert_eq!(
            id(recipe.timers[0].quantity.as_ref().unwrap()),
            Some("hour")
        );
        // an untagged recipe reads through the legacy locale
        let untagged = CooklangParser::default()
            .parse("@flour{1%cup} and @salt{1%pinch}\n")
            .into_output()
            .unwrap();
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
        let mut back: crate::Recipe = serde_json::from_str(&stripped).unwrap();
        assert_eq!(
            back.ingredients[0].quantity.as_ref().unwrap().unit_id(),
            None
        );
        back.read_units();
        assert!(back.equals(&recipe));
    }

    #[test]
    fn the_converter_a_recipe_reads_through_is_its_namings() {
        let recipe = swedish("@mjöl{1%g}\n");
        assert_eq!(recipe.converter(), Converter::for_recipe(&recipe).unwrap());
        let untagged = CooklangParser::default()
            .parse("@flour{1%g}\n")
            .into_output()
            .unwrap();
        assert_eq!(untagged.converter(), Converter::legacy());
    }
}

/// A list built from recipes in several locales adds what it recognises in one unit (ADR 7, "Aggregation converts into
/// the target first").
mod lists {
    use crate::convert::LocaleTag;
    use crate::ingredient_list::IngredientList;
    use crate::{Converter, CooklangParser, Extensions};

    #[test]
    fn a_swedish_and_an_untagged_recipe_add_up_in_the_target_locale() {
        let sv = Converter::for_tag(&LocaleTag::parse("sv-SE").unwrap()).unwrap();
        let parser = CooklangParser::new(Extensions::all());
        let swedish = parser
            .parse("---\nlocale: sv-SE\n---\nSmält @smör{2%msk}.\n")
            .into_output()
            .unwrap();
        let untagged = parser
            .parse("Melt @smör{1%tbsp} and @mjöl{1%cup}.\n")
            .into_output()
            .unwrap();

        let mut list = IngredientList::new();
        list.add_recipe(&swedish, &sv, false);
        list.add_recipe(&untagged, &sv, false);
        let entries: Vec<(String, String)> = list
            .iter()
            .map(|(name, group)| (name.clone(), group.to_string()))
            .collect();
        // a Swedish spoon and a US spoon are one amount, in the target's unit;
        // a unit the target has no name for is written by its identifier
        assert_eq!(
            entries,
            [
                ("smör".to_string(), "2.986 msk".to_string()),
                ("mjöl".to_string(), "1 us cup".to_string()),
            ]
        );

        // the legacy target names no metric spoon, so the sum is written by
        // the unit's identifier, and the cup by legacy's own symbol
        let legacy = Converter::legacy();
        let mut list = IngredientList::new();
        list.add_recipe(&swedish, &legacy, false);
        list.add_recipe(&untagged, &legacy, false);
        let entries: Vec<String> = list.iter().map(|(_, g)| g.to_string()).collect();
        assert_eq!(entries, ["2.986 metric tablespoon", "1 c"]);
    }
}
