//! The new converter reproduces the old one on untagged recipes.
//!
//! Every recipe in the corpus is parsed through both pipelines and the results
//! must be equal as written, warnings and errors included. The one difference
//! allowed is the warning the new pipeline adds to an untagged recipe. A
//! divergence is either ported or recorded in ADR 0009; there is no tolerance.
//!
//! This test is deleted together with the old converter (ADR 0009).

use cooklang::ingredient_list::IngredientList;
use cooklang::{
    Converter, CooklangParser, Extensions, LegacyConverter, LegacySemanticEq, Quantity, Recipe,
    SemanticEq,
};
use serde::Deserialize;

/// A recipe the two pipelines must agree on: where it came from, and its text.
struct Case {
    name: String,
    source: String,
}

/// The sources of the canonical spec cases, lifted from the generated test
/// file so the corpus cannot drift from the suite.
fn canonical_cases() -> Vec<Case> {
    #[derive(Deserialize)]
    struct TestCase {
        source: String,
    }
    const GENERATED: &str = include_str!("canonical_cases/mod.rs");
    const OPEN: &str = "#[test_case(r#\"";
    const CLOSE: &str = "\"#\n; \"";
    let mut cases = Vec::new();
    let mut rest = GENERATED;
    while let Some(start) = rest.find(OPEN) {
        let yaml_start = &rest[start + OPEN.len()..];
        let end = yaml_start.find(CLOSE).expect("unterminated test case");
        let yaml = &yaml_start[..end];
        let name_start = &yaml_start[end + CLOSE.len()..];
        let name_end = name_start.find('"').expect("unterminated test name");
        let case: TestCase = serde_yaml::from_str(yaml).expect("canonical case is yaml");
        cases.push(Case {
            name: format!("canonical/{}", &name_start[..name_end]),
            source: case.source,
        });
        rest = &name_start[name_end..];
    }
    assert!(
        cases.len() > 50,
        "found only {} canonical cases",
        cases.len()
    );
    cases
}

fn fixtures() -> Vec<Case> {
    [
        (
            "benches/test_recipe.cook",
            include_str!("../benches/test_recipe.cook"),
        ),
        (
            "benches/complex_test_recipe.cook",
            include_str!("../benches/complex_test_recipe.cook"),
        ),
        (
            "benches/frontmatter_test_recipe.cook",
            include_str!("../benches/frontmatter_test_recipe.cook"),
        ),
    ]
    .into_iter()
    .map(|(name, source)| Case {
        name: name.to_string(),
        source: source.to_string(),
    })
    .collect()
}

/// Recipes written to reach what the fixtures do not: every kind of unit the
/// old converter knows, references that must and must not add up, timers,
/// inline quantities, and the standard metadata keys that read units.
fn hand_written() -> Vec<Case> {
    let cases: &[(&str, &str)] = &[
        (
            "units/every-kind",
            "Mix @flour{250%g}, @water{1.5%dl}, @butter{2%tbsp}, @sugar{1/2%cup} and @salt{1%tsp}.\n\
             Add @oil{3%fl oz}, @stock{1%pint}, @milk{250%ml} and @rice{1%lb}.\n\
             Bake at 200°C for ~{45%min}, rest ~{1%h}, serve within ~{30%s}.\n\
             Cut @dough{30%cm} long, @chocolate{200%g} chopped, @cream{2%L}.\n",
        ),
        (
            "units/unknown-and-text",
            "Add @flour{2%handfuls}, @salt{a pinch}, @pepper{some%grinds} and @eggs{3}.\n\
             Use #pan{1} and #bowl{2%large}.\n",
        ),
        (
            "references/compatible",
            "Add @flour{250%g} to the bowl.\n\
             Add more @&flour{0.5%kg} later.\n\
             Finish with @&flour{10%oz}.\n",
        ),
        (
            "references/incompatible",
            "Add @water{250%ml} to the bowl.\n\
             Add more @&water{1%kg} later.\n",
        ),
        (
            "references/unknown-units",
            "Add @flour{2%handfuls}.\n\
             Add more @&flour{1%handful}.\n\
             Add @sugar{1%scoop}.\n\
             Add more @&sugar{1%scoop}.\n",
        ),
        (
            "inline/quantities",
            "Heat the oven to 180°C, then add 250 g of flour and 2 tbsp of oil.\n\
             Wait 10 minutes and 30 seconds.\n",
        ),
        (
            "metadata/time-and-yield",
            "---\n\
             servings: 4\n\
             yield: 500%g\n\
             time: 1h 30m\n\
             prep time: 20 minutes\n\
             cook time: 45 min\n\
             ---\n\
             Mix @flour{500%g} and @water{300%ml}.\n",
        ),
        (
            "metadata/time-composed",
            "---\n\
             time:\n  prep: 15 min\n  cook: 1 h\n\
             ---\n\
             Mix @flour{500%g}.\n",
        ),
        (
            "metadata/time-invalid",
            "---\n\
             time: three hours\n\
             prep time: 2 fortnights\n\
             ---\n\
             Mix @flour{500%g}.\n",
        ),
        (
            "metadata/legacy-lines",
            ">> servings: 2\n\
             >> time: 45 min\n\
             Mix @flour{500%g}.\n",
        ),
        (
            "timers/every-unit",
            "Wait ~{1%second}, ~{2%minutes}, ~{3%hours}, ~{4%sec}, ~{5%mins}, ~{6%h} and ~{7%days}.\n",
        ),
        (
            "ranges/units",
            "Add @flour{200-250%g} and @water{1-2%cups}, bake ~{20-25%min}.\n",
        ),
    ];
    cases
        .iter()
        .map(|(name, source)| Case {
            name: name.to_string(),
            source: source.to_string(),
        })
        .collect()
}

fn corpus() -> Vec<Case> {
    let mut cases = canonical_cases();
    cases.extend(fixtures());
    cases.extend(hand_written());
    cases
}

/// The warning the new pipeline adds to a recipe with no `locale`.
fn is_untagged_warning(message: &str) -> bool {
    message == cooklang::analysis::UNTAGGED_WARNING
}

fn messages(report: &cooklang::error::SourceReport, drop_untagged: bool) -> Vec<String> {
    let mut messages: Vec<String> = report
        .iter()
        .filter(|diag| !(drop_untagged && is_untagged_warning(&diag.message)))
        .map(|diag| format!("{:?}: {}", diag.severity, diag.message))
        .collect();
    messages.sort();
    messages
}

#[test]
fn parsing_matches_the_old_converter() {
    let mut divergences = Vec::new();
    for (ext_name, ext) in [
        ("canonical", Extensions::empty()),
        ("all", Extensions::all()),
    ] {
        let old = CooklangParser::new_legacy(ext, LegacyConverter::bundled());
        let new = CooklangParser::new(ext, Converter::legacy());
        for case in corpus() {
            let old_result = old.parse(&case.source);
            let new_result = new.parse(&case.source);
            let name = format!("{ext_name}/{}", case.name);
            if old_result.output() != new_result.output() {
                divergences.push(format!(
                    "{name}: recipes differ\n  old: {:#?}\n  new: {:#?}",
                    old_result.output(),
                    new_result.output()
                ));
            }
            let old_messages = messages(old_result.report(), false);
            let new_messages = messages(new_result.report(), true);
            if old_messages != new_messages {
                divergences.push(format!(
                    "{name}: diagnostics differ\n  old: {old_messages:?}\n  new: {new_messages:?}"
                ));
            }
        }
    }
    assert!(
        divergences.is_empty(),
        "{} divergence(s):\n\n{}",
        divergences.len(),
        divergences.join("\n\n")
    );
}

/// Renderings the new pipeline writes differently on purpose, as `(old, new)`,
/// all on the US side of the legacy locale, where the old converter fitted
/// a fraction before it applied its thresholds (ADR 0008, ADR 0009). Every
/// pair here must still occur, so the list cannot outlive what it explains.
const KNOWN_FIT_DIVERGENCES: &[(&str, &str)] = &[
    // The old converter searched every best unit for the smallest fraction
    // denominator, a whole number counting as denominator one, so a whole
    // number of the smaller unit beat a fraction of the larger. The new
    // converter keeps the unit the threshold chose.
    ("2 tsp", "2/3 tbsp"),
    ("4 tsp", "1 1/3 tbsp"),
    ("20 oz", "1 1/4 lb"),
    ("28 oz", "1 3/4 lb"),
    ("30 oz", "1 7/8 lb"),
    ("45 oz", "2 3/4 lb"),
    ("70 oz", "4 3/8 lb"),
    ("105 oz", "6 1/2 lb"),
    ("5-9.6 tsp", "1 2/3-3 1/3 tbsp"),
    // The same search on a tie: both fractions have denominator three, and
    // the old converter took the one with the smaller whole part.
    ("1/3 lb", "5 1/3 oz"),
    // The old converter capped the whole part of a fraction per unit, at
    // four tablespoons and five teaspoons, so 4.8 tbsp could not round up
    // to five there. The new converter has no such cap.
    ("4 2/3 tbsp", "5 tbsp"),
];

/// One quantity to compare: where it sits, how it prints, and the quantity
/// itself when there is one (metadata entries have none).
type Item = (String, String, Option<Quantity>);

/// Every quantity of a recipe, with where it sits, as it would be printed.
fn quantities(recipe: &Recipe) -> Vec<Item> {
    let mut out = Vec::new();
    for (i, ingredient) in recipe.ingredients.iter().enumerate() {
        if let Some(q) = &ingredient.quantity {
            out.push((
                format!("ingredient {i} {}", ingredient.name),
                q.to_string(),
                Some(q.clone()),
            ));
        }
    }
    for (i, cookware) in recipe.cookware.iter().enumerate() {
        if let Some(q) = &cookware.quantity {
            out.push((
                format!("cookware {i} {}", cookware.name),
                q.to_string(),
                Some(q.clone()),
            ));
        }
    }
    for (i, timer) in recipe.timers.iter().enumerate() {
        if let Some(q) = &timer.quantity {
            out.push((format!("timer {i}"), q.to_string(), Some(q.clone())));
        }
    }
    for (key, value) in recipe.metadata.map.iter() {
        out.push((format!("metadata {key:?}"), format!("{value:?}"), None));
    }
    out
}

/// What a differing rendering must satisfy to pass.
#[derive(Clone, Copy)]
enum Compare {
    /// The pair is in [`KNOWN_FIT_DIVERGENCES`]: for scaled recipes, whose
    /// fit is what the list pins.
    KnownRendering,
    /// The two denote the same amount, read by the old converter: for lists,
    /// whose arithmetic is what matters and whose inputs were fitted
    /// differently already, so their sums print differently too.
    SameAmount,
}

/// Compares two lists of quantities, reporting any difference `mode` does
/// not allow; returns the known pairs it saw.
fn compare_quantities(
    name: &str,
    old_q: &[Item],
    new_q: &[Item],
    mode: Compare,
    bundled: &LegacyConverter,
    divergences: &mut Vec<String>,
) -> Vec<(String, String)> {
    let mut seen = Vec::new();
    if old_q.len() != new_q.len() {
        let texts = |items: &[Item]| {
            items
                .iter()
                .map(|(w, t, _)| format!("{w}: {t}"))
                .collect::<Vec<_>>()
        };
        divergences.push(format!(
            "{name}: different quantity counts\n  old: {:?}\n  new: {:?}",
            texts(old_q),
            texts(new_q)
        ));
        return seen;
    }
    for ((where_old, old_text, old_qty), (where_new, new_text, new_qty)) in old_q.iter().zip(new_q)
    {
        if where_old != where_new {
            divergences.push(format!("{name}: {where_old} vs {where_new}"));
            continue;
        }
        if old_text == new_text {
            continue;
        }
        let allowed = match mode {
            Compare::KnownRendering => {
                let known = KNOWN_FIT_DIVERGENCES
                    .iter()
                    .any(|(o, n)| o == old_text && n == new_text);
                if known {
                    seen.push((old_text.clone(), new_text.clone()));
                }
                known
            }
            Compare::SameAmount => match (old_qty, new_qty) {
                (Some(o), Some(n)) => LegacySemanticEq::equals_legacy(o, n, bundled),
                _ => false,
            },
        };
        if !allowed {
            divergences.push(format!(
                "{name}: {where_old}: old `{old_text}`, new `{new_text}`"
            ));
        }
    }
    seen
}

/// Compares two scaled recipes.
fn compare_scaled(
    name: &str,
    old: &Recipe,
    new: &Recipe,
    bundled: &LegacyConverter,
    divergences: &mut Vec<String>,
) -> Vec<(String, String)> {
    compare_quantities(
        name,
        &quantities(old),
        &quantities(new),
        Compare::KnownRendering,
        bundled,
        divergences,
    )
}

#[test]
fn scaling_matches_the_old_converter() {
    let bundled = LegacyConverter::bundled();
    let mut divergences = Vec::new();
    let mut seen = Vec::new();
    for (ext_name, ext) in [
        ("canonical", Extensions::empty()),
        ("all", Extensions::all()),
    ] {
        let old = CooklangParser::new_legacy(ext, bundled.clone());
        let new = CooklangParser::new(ext, Converter::legacy());
        for case in corpus() {
            let Some(old_recipe) = old.parse(&case.source).into_output() else {
                continue;
            };
            let new_recipe = new
                .parse(&case.source)
                .into_output()
                .expect("parses on both");
            for factor in [0.5, 2.0, 3.0, 1.0 / 3.0, 0.1, 7.0] {
                let name = format!("{ext_name}/{} x{factor}", case.name);
                let mut o = old_recipe.clone();
                let mut n = new_recipe.clone();
                o.scale_legacy(factor, &bundled);
                n.scale(factor);
                seen.extend(compare_scaled(&name, &o, &n, &bundled, &mut divergences));
            }
            // the targets are the same calls with a factor worked out first
            let name = format!("{ext_name}/{} to 3 servings", case.name);
            let mut o = old_recipe.clone();
            let mut n = new_recipe.clone();
            let old_result = o
                .scale_to_servings_legacy(3.0, &bundled)
                .map_err(|e| e.to_string());
            let new_result = n.scale_to_servings(3.0).map_err(|e| e.to_string());
            if old_result != new_result {
                divergences.push(format!("{name}: {old_result:?} vs {new_result:?}"));
            }
            seen.extend(compare_scaled(&name, &o, &n, &bundled, &mut divergences));
            for (value, unit) in [(2.0, "kg"), (250.0, "g"), (1.0, "lb"), (3.0, "")] {
                let name = format!("{ext_name}/{} to {value} {unit}", case.name);
                let mut o = old_recipe.clone();
                let mut n = new_recipe.clone();
                let old_result = o
                    .scale_to_yield_legacy(value, unit, &bundled)
                    .map_err(|e| e.to_string());
                let new_result = n.scale_to_yield(value, unit).map_err(|e| e.to_string());
                if old_result != new_result {
                    divergences.push(format!("{name}: {old_result:?} vs {new_result:?}"));
                }
                seen.extend(compare_scaled(&name, &o, &n, &bundled, &mut divergences));
            }
        }
    }
    assert!(
        divergences.is_empty(),
        "{} divergence(s):\n\n{}",
        divergences.len(),
        divergences.join("\n")
    );
    for (o, n) in KNOWN_FIT_DIVERGENCES {
        assert!(
            seen.iter().any(|(so, sn)| so == o && sn == n),
            "known divergence `{o}` -> `{n}` no longer occurs; drop it"
        );
    }
}

/// Every quantity of a grouped list, with where it sits. A group's order
/// carries no meaning, and unknown units come out of a hash map, so each
/// group is sorted by its rendering.
fn grouped(entries: impl Iterator<Item = (String, Vec<Quantity>)>) -> Vec<Item> {
    let mut out = Vec::new();
    for (name, quantities) in entries {
        let mut quantities: Vec<(String, Quantity)> =
            quantities.into_iter().map(|q| (q.to_string(), q)).collect();
        quantities.sort_by(|a, b| a.0.cmp(&b.0));
        for (i, (text, q)) in quantities.into_iter().enumerate() {
            out.push((format!("{name} #{i}"), text, Some(q)));
        }
    }
    out
}

fn list_entries(list: &IngredientList) -> Vec<(String, Vec<Quantity>)> {
    list.iter()
        .map(|(name, group)| (name.clone(), group.iter().cloned().collect()))
        .collect()
}

#[test]
fn lists_match_the_old_converter() {
    let bundled = LegacyConverter::bundled();
    let v2 = Converter::legacy();
    let mut divergences = Vec::new();
    #[cfg(feature = "pantry")]
    let pantry = cooklang::pantry::parse(
        "[kitchen]\nwater = \"unlim\"\nsalt = \"0.5%tsp\"\nflour = \"100%g\"\noil = \"1%tbsp\"\nsugar = \"1%scoop\"\n",
    )
    .expect("a pantry");
    for (ext_name, ext) in [
        ("canonical", Extensions::empty()),
        ("all", Extensions::all()),
    ] {
        let old = CooklangParser::new_legacy(ext, bundled.clone());
        let new = CooklangParser::new(ext, Converter::legacy());
        for case in corpus() {
            let Some(old_recipe) = old.parse(&case.source).into_output() else {
                continue;
            };
            let new_recipe = new
                .parse(&case.source)
                .into_output()
                .expect("parses on both");
            for factor in [None, Some(2.0), Some(0.5), Some(1.0 / 3.0)] {
                let mut o = old_recipe.clone();
                let mut n = new_recipe.clone();
                let name = match factor {
                    Some(f) => {
                        o.scale_legacy(f, &bundled);
                        n.scale(f);
                        format!("{ext_name}/{} x{f}", case.name)
                    }
                    None => format!("{ext_name}/{}", case.name),
                };
                let mut compare = |what: &str, old_q: &[Item], new_q: &[Item]| {
                    compare_quantities(
                        &format!("{name} {what}"),
                        old_q,
                        new_q,
                        Compare::SameAmount,
                        &bundled,
                        &mut divergences,
                    );
                };

                let old_groups =
                    grouped(o.group_ingredients_legacy(&bundled).into_iter().map(|g| {
                        (
                            g.ingredient.name.clone(),
                            g.quantity.iter().cloned().collect(),
                        )
                    }));
                let new_groups = grouped(n.group_ingredients().into_iter().map(|g| {
                    (
                        g.ingredient.name.clone(),
                        g.quantity.iter().cloned().collect(),
                    )
                }));
                compare("ingredients", &old_groups, &new_groups);

                let old_cookware =
                    grouped(o.group_cookware_legacy(&bundled).into_iter().map(|g| {
                        (
                            g.cookware.name.clone(),
                            g.quantity.iter().cloned().collect(),
                        )
                    }));
                let new_cookware = grouped(n.group_cookware().into_iter().map(|g| {
                    (
                        g.cookware.name.clone(),
                        g.quantity.iter().cloned().collect(),
                    )
                }));
                compare("cookware", &old_cookware, &new_cookware);

                let old_list = IngredientList::from_recipe_legacy(&o, &bundled, true);
                let new_list = IngredientList::from_recipe(&n, &v2, true);
                let old_entries = list_entries(&old_list);
                let new_entries = list_entries(&new_list);
                compare(
                    "list",
                    &grouped(old_entries.iter().cloned()),
                    &grouped(new_entries.iter().cloned()),
                );

                // the pantry matches units by their text, so an entry the two
                // pipelines fitted to different units is subtracted from on
                // one side only; only entries that print the same are compared
                #[cfg(feature = "pantry")]
                {
                    let same: std::collections::HashSet<&String> = old_entries
                        .iter()
                        .zip(&new_entries)
                        .filter(|((on, oq), (nn, nq))| {
                            on == nn
                                && oq.iter().map(|q| q.to_string()).collect::<Vec<_>>()
                                    == nq.iter().map(|q| q.to_string()).collect::<Vec<_>>()
                        })
                        .map(|((name, _), _)| name)
                        .collect();
                    let old_left = old_list.subtract_pantry_legacy(&pantry, &bundled);
                    let new_left = new_list.subtract_pantry(&pantry);
                    let keep = |entries: Vec<(String, Vec<Quantity>)>| {
                        entries
                            .into_iter()
                            .filter(|(name, _)| same.contains(name))
                            .collect::<Vec<_>>()
                    };
                    compare(
                        "after pantry",
                        &grouped(keep(list_entries(&old_left)).into_iter()),
                        &grouped(keep(list_entries(&new_left)).into_iter()),
                    );
                }
            }
        }
    }
    assert!(
        divergences.is_empty(),
        "{} divergence(s):\n\n{}",
        divergences.len(),
        divergences.join("\n")
    );
}

#[test]
fn metadata_time_matches_the_old_converter() {
    let bundled = LegacyConverter::bundled();
    let mut divergences = Vec::new();
    for (ext_name, ext) in [
        ("canonical", Extensions::empty()),
        ("all", Extensions::all()),
    ] {
        let old = CooklangParser::new_legacy(ext, bundled.clone());
        let new = CooklangParser::new(ext, Converter::legacy());
        for case in corpus() {
            let Some(old_recipe) = old.parse(&case.source).into_output() else {
                continue;
            };
            let new_recipe = new
                .parse(&case.source)
                .into_output()
                .expect("parses on both");
            let old_time = old_recipe.metadata.time_legacy(&bundled);
            let new_time = new_recipe.metadata.time();
            if format!("{old_time:?}") != format!("{new_time:?}") {
                divergences.push(format!(
                    "{ext_name}/{}: old {old_time:?}, new {new_time:?}",
                    case.name
                ));
            }
        }
    }
    assert!(
        divergences.is_empty(),
        "{} divergence(s):\n\n{}",
        divergences.len(),
        divergences.join("\n")
    );
}

/// Both traits must give the same verdict on the same pair. Pairs are each
/// recipe against itself, against its scaled self and against every other
/// recipe, so both verdicts occur.
#[test]
fn semantic_equality_matches_the_old_converter() {
    let bundled = LegacyConverter::bundled();
    let old = CooklangParser::new_legacy(Extensions::all(), bundled.clone());
    let new = CooklangParser::new(Extensions::all(), Converter::legacy());
    let mut recipes: Vec<(String, Recipe, Recipe)> = Vec::new();
    for case in corpus() {
        let Some(o) = old.parse(&case.source).into_output() else {
            continue;
        };
        let n = new
            .parse(&case.source)
            .into_output()
            .expect("parses on both");
        for factor in [None, Some(2.0), Some(1.0 / 3.0)] {
            let (mut o, mut n) = (o.clone(), n.clone());
            let name = match factor {
                Some(f) => {
                    o.scale_legacy(f, &bundled);
                    n.scale(f);
                    format!("{} x{f}", case.name)
                }
                None => case.name.clone(),
            };
            recipes.push((name, o, n));
        }
    }
    let mut divergences = Vec::new();
    let mut equal_pairs = 0;
    for (a_name, a_old, a_new) in &recipes {
        for (b_name, b_old, b_new) in &recipes {
            let old_verdict = LegacySemanticEq::equals_legacy(a_old, b_old, &bundled);
            let new_verdict = SemanticEq::equals(a_new, b_new);
            if old_verdict {
                equal_pairs += 1;
            }
            if old_verdict != new_verdict {
                divergences.push(format!(
                    "{a_name} vs {b_name}: old {old_verdict}, new {new_verdict}"
                ));
            }
        }
    }
    assert!(equal_pairs >= recipes.len(), "every recipe equals itself");
    assert!(
        divergences.is_empty(),
        "{} divergence(s):\n\n{}",
        divergences.len(),
        divergences.join("\n")
    );
}
