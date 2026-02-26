//! Cooklang [aisle configuration](https://cooklang.org/docs/spec/#the-shopping-list-specification) parser
//!
//! This module is only available with the `aisle` [feaure](crate::_features).
//!
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    error::{CowStr, Label, RichError, SourceDiag, SourceReport, Stage},
    span::Span,
    PassResult,
};

/// Represents a aisle configuration file
///
/// This type also implements [`Serialize`] and [`Deserialize`], so if you don't
/// like the cooklang shopping list format you can swap it with any [`serde`]
/// format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AisleConf<'a> {
    /// List of categories
    #[serde(borrow)]
    pub categories: Vec<Category<'a>>,
    // optimizationo for consecutive calls os `ingredients_info`
    #[serde(skip)]
    len: std::cell::Cell<usize>,
}

/// A category, or aisle
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category<'a> {
    /// Name of the category
    #[serde(borrow)]
    pub name: &'a str,
    /// List of ingredients belonging to this category
    pub ingredients: Vec<Ingredient<'a>>,
}

/// An ingredient belonging to a [`Category`]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ingredient<'a> {
    /// List of names of the ingredient
    #[serde(borrow)]
    pub names: Vec<&'a str>,
}

/// Information about an ingredient extracted with [`AisleConf::ingredients_info`]
pub struct IngredientInfo<'a> {
    /// Name of the ingredient
    pub name: &'a str,
    /// Common name, the first in the aisle configuration
    ///
    /// This is the name that should be used in lists.
    pub common_name: &'a str,
    /// Category the ingredient is in
    pub category: &'a str,
}

impl AisleConf<'_> {
    /// Returns a reversed configuration, where each key is an ingredient
    /// and the value is its category.
    #[deprecated = "Use `ingredients_info` instead"]
    pub fn reverse(&self) -> HashMap<String, &str> {
        self.ingredients_info()
            .into_iter()
            .map(|(n, i)| (n, i.name))
            .collect()
    }

    /// Returns the sort key (`category_index`, `ingredient_index`) for an ingredient.
    ///
    /// Performs case-insensitive lookup. Returns `None` if the ingredient
    /// is not found in any category. Synonym names return the same position
    /// as their primary name.
    pub fn ingredient_sort_key(&self, name: &str) -> Option<(usize, usize)> {
        let lower = name.to_lowercase();
        for (cat_idx, category) in self.categories.iter().enumerate() {
            for (igr_idx, ingredient) in category.ingredients.iter().enumerate() {
                for n in &ingredient.names {
                    if n.to_lowercase() == lower {
                        return Some((cat_idx, igr_idx));
                    }
                }
            }
        }
        None
    }

    /// Returns a reversed configuration, where each ingredient has a
    /// corresponding [`IngredientInfo`]
    ///
    /// The keys are lowercase for case-insensitive lookups. Use
    /// `name.to_lowercase()` when looking up ingredients.
    pub fn ingredients_info(&self) -> HashMap<String, IngredientInfo<'_>> {
        let mut map = HashMap::with_capacity(self.len.get());
        for cat in &self.categories {
            for igr in &cat.ingredients {
                let Some(common_name) = igr.names.first() else {
                    continue;
                };

                for name in &igr.names {
                    let info = IngredientInfo {
                        name,
                        common_name,
                        category: cat.name,
                    };
                    // Store lowercase key for case-insensitive lookups
                    map.insert(name.to_lowercase(), info);
                }
            }
        }
        self.len.set(map.len());
        map
    }
}

/// Core parsing logic that can either return errors or collect warnings
fn parse_core<'i>(
    input: &'i str,
    lenient: bool,
    mut report: Option<&mut SourceReport>,
) -> Result<AisleConf<'i>, AisleConfError> {
    let mut categories: Vec<Category> = Vec::new();
    let mut current_category: Option<Category> = None;

    let mut used_categories = HashSet::new();
    let mut used_names = HashSet::new();

    let calc_span = |s: &str| {
        let s_ptr = s.as_ptr();
        let input_ptr = input.as_ptr();
        // SAFETY: only used when `s` is an slice of the original input str
        assert!(s_ptr >= input_ptr);
        assert!(s_ptr <= unsafe { input_ptr.add(input.len() - 1) });
        let offset = unsafe { s_ptr.offset_from(input_ptr) };
        let offset = offset as usize;
        Span::new(offset, offset + s.len())
    };

    for mut line in input.lines() {
        // strip comment
        if let Some((l, _)) = line.split_once("//") {
            line = l;
        }
        // strip whitespace
        line = line.trim_ascii();

        if line.starts_with('[') && line.ends_with(']') {
            let name = &line[1..line.len() - 1];
            if name.contains('|') {
                if lenient {
                    if let Some(report) = report.as_mut() {
                        let warning = SourceDiag::warning(
                            "Invalid category name: contains '|' character",
                            (
                                calc_span(name),
                                Some("category names cannot contain '|'".into()),
                            ),
                            Stage::Parse,
                        );
                        report.push(warning);
                    }
                    continue;
                } else {
                    return Err(AisleConfError::Parse {
                        span: calc_span(name),
                        message: "Invalid category name".to_string(),
                    });
                }
            }

            if let Some(&other) = used_categories.get(name) {
                if lenient {
                    if let Some(report) = report.as_mut() {
                        let warning = SourceDiag::warning(
                            format!("Duplicate category: '{name}'"),
                            (calc_span(name), Some("duplicate found here".into())),
                            Stage::Parse,
                        );
                        report.push(warning);
                    }
                    continue;
                } else {
                    return Err(AisleConfError::DuplicateCategory {
                        name: name.to_string(),
                        first_span: calc_span(other),
                        second_span: calc_span(name),
                    });
                }
            }

            used_categories.insert(name);

            let new_cat = Category {
                name,
                ingredients: Vec::new(),
            };
            if let Some(cat) = current_category.replace(new_cat) {
                categories.push(cat);
            }
        } else if !line.is_empty() {
            let mut names = Vec::new();
            for mut n in line.split('|') {
                n = n.trim();
                if let Some(&other) = used_names.get(n) {
                    if lenient {
                        if let Some(report) = report.as_mut() {
                            let warning = SourceDiag::warning(
                                format!("Duplicate ingredient: '{n}'"),
                                (calc_span(n), Some("duplicate found here".into())),
                                Stage::Parse,
                            );
                            report.push(warning);
                        }
                        continue;
                    } else {
                        return Err(AisleConfError::DuplicateIngredient {
                            name: n.to_string(),
                            first_span: calc_span(other),
                            second_span: calc_span(n),
                        });
                    }
                }
                used_names.insert(n);
                names.push(n);
            }

            // Only add ingredient if it has at least one name
            if !names.is_empty() {
                if let Some(cat) = &mut current_category {
                    cat.ingredients.push(Ingredient { names });
                } else if lenient {
                    if let Some(report) = report.as_mut() {
                        let warning = SourceDiag::warning(
                            "Ingredient found before any category",
                            (
                                calc_span(line),
                                Some("add a category before listing ingredients".into()),
                            ),
                            Stage::Parse,
                        );
                        report.push(warning);
                    }
                } else {
                    return Err(AisleConfError::Parse {
                        span: calc_span(line),
                        message: "Expected category".to_string(),
                    });
                }
            }
        }
    }

    if let Some(cat) = current_category {
        categories.push(cat);
    }

    Ok(AisleConf {
        categories,
        len: std::cell::Cell::new(0),
    })
}

/// Parse an [`AisleConf`] with the cooklang shopping list format
pub fn parse(input: &str) -> Result<AisleConf<'_>, AisleConfError> {
    parse_core(input, false, None)
}

/// Parse aisle configuration with lenient handling of duplicates
///
/// This function returns a [`PassResult`] which includes both the parsed configuration
/// and any warnings that occurred during parsing. Duplicate ingredients will be
/// reported as warnings rather than errors.
///
/// # Examples
///
/// ```
/// let aisle_conf = r#"
/// [fruit and vegetables]
/// potato
/// apple
/// "#;
///
/// let result = cooklang::aisle::parse_lenient(aisle_conf);
/// let (parsed, warnings) = result.into_result().unwrap();
///
/// assert_eq!(parsed.categories.len(), 1);
/// assert_eq!(parsed.categories[0].ingredients.len(), 2);
/// ```
pub fn parse_lenient(input: &str) -> PassResult<AisleConf<'_>> {
    let mut report = SourceReport::empty();

    let conf =
        parse_core(input, true, Some(&mut report)).expect("lenient parsing should never fail");
    PassResult::new(Some(conf), report)
}

/// Write an [`AisleConf`] in the cooklang shopping list format
pub fn write(conf: &AisleConf, mut write: impl std::io::Write) -> std::io::Result<()> {
    let w = &mut write;
    for category in &conf.categories {
        writeln!(w, "[{}]", category.name)?;
        for ingredient in &category.ingredients {
            if !ingredient.names.is_empty() {
                let mut iter = ingredient.names.iter();
                write!(w, "{}", iter.next().unwrap())?;
                for name in iter {
                    write!(w, "|{name}")?;
                }
                writeln!(w)?
            }
        }
        writeln!(w)?;
    }

    Ok(())
}

/// Error generated by [`parse`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AisleConfError {
    #[error("Error parsing input: {message}")]
    Parse { span: Span, message: String },
    #[error("Duplicate category: '{name}'")]
    DuplicateCategory {
        /// Duplicated category name
        name: String,
        /// The first location where the category was found
        first_span: Span,
        /// The second location where the category was found
        second_span: Span,
    },
    #[error("Duplicate ingredient: '{name}'")]
    DuplicateIngredient {
        /// Duplicated ingredient name
        name: String,
        /// The first location where the ingredient was found
        first_span: Span,
        /// The second location where the ingredient was found
        second_span: Span,
    },
}

impl RichError for AisleConfError {
    fn labels(&self) -> Cow<'_, [Label]> {
        use crate::error::label;
        match self {
            AisleConfError::Parse { span, .. } => vec![label!(span)],
            AisleConfError::DuplicateCategory {
                first_span,
                second_span,
                ..
            } => vec![
                label!(second_span, "this category"),
                label!(first_span, "was first defined here"),
            ],
            AisleConfError::DuplicateIngredient {
                first_span,
                second_span,
                ..
            } => vec![
                label!(second_span, "this ingredient"),
                label!(first_span, "was first defined here"),
            ],
        }
        .into()
    }

    fn hints(&self) -> Cow<'_, [CowStr]> {
        match self {
            AisleConfError::DuplicateCategory { .. } => {
                vec!["Remove the duplicate category".into()]
            }
            AisleConfError::DuplicateIngredient { .. } => {
                vec!["Remove the duplicate ingredient".into()]
            }
            _ => {
                vec![]
            }
        }
        .into()
    }

    fn severity(&self) -> crate::error::Severity {
        crate::error::Severity::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_aisle() {
        let input = r#"
[produce]
potatoes

[dairy]
milk
butter
"#;
        let a = parse(input).unwrap();
        assert_eq!(
            a.categories,
            vec![
                Category {
                    name: "produce",
                    ingredients: vec![Ingredient {
                        names: vec!["potatoes"]
                    }]
                },
                Category {
                    name: "dairy",
                    ingredients: vec![
                        Ingredient {
                            names: vec!["milk"],
                        },
                        Ingredient {
                            names: vec!["butter"],
                        },
                    ],
                },
            ]
        )
    }

    #[test]
    fn empty_file() {
        let a = parse("").unwrap();
        assert!(a.categories.is_empty());
    }

    #[test]
    fn empty_category() {
        let input = r#"
[empty]
"#;
        let a = parse(input).unwrap();
        assert_eq!(
            a.categories,
            vec![Category {
                name: "empty",
                ingredients: vec![]
            }]
        )
    }

    #[test]
    fn no_space() {
        let input = r#"
[produce]
potatoes
[dairy]
milk
"#;
        let a = parse(input).unwrap();
        assert_eq!(
            a.categories,
            vec![
                Category {
                    name: "produce",
                    ingredients: vec![Ingredient {
                        names: vec!["potatoes"]
                    }]
                },
                Category {
                    name: "dairy",
                    ingredients: vec![Ingredient {
                        names: vec!["milk"],
                    }],
                },
            ]
        )
    }

    #[test]
    fn synonyms() {
        let input = r#"[canned goods]
tuna|chicken of the sea
"#;
        let a = parse(input).unwrap();
        assert_eq!(
            a.categories,
            vec![Category {
                name: "canned goods",
                ingredients: vec![Ingredient {
                    names: vec!["tuna", "chicken of the sea"]
                }]
            }]
        )
    }

    #[test]
    fn synonym_lookup() {
        let input = r#"[canned goods]
tuna|chicken of the sea
"#;
        let a = parse(input).unwrap();
        let p = a.ingredients_info();
        // Keys are lowercase for case-insensitive lookup
        assert_eq!(
            vec!["tuna", "tuna"],
            ["tuna", "chicken of the sea"]
                .iter()
                .map(|igr| p.get(&igr.to_lowercase()).unwrap().common_name)
                .collect::<Vec<&str>>()
        )
    }

    #[test]
    fn case_insensitive_lookup() {
        let input = r#"[spices]
chili flakes
"#;
        let a = parse(input).unwrap();
        let p = a.ingredients_info();
        // All case variants should find the same ingredient
        assert!(p.get("chili flakes").is_some());
        assert!(p.get(&"Chili flakes".to_lowercase()).is_some());
        assert!(p.get(&"CHILI FLAKES".to_lowercase()).is_some());
        assert_eq!(
            p.get("chili flakes").unwrap().common_name,
            p.get(&"Chili Flakes".to_lowercase()).unwrap().common_name
        );
    }

    #[test]
    fn duplicate_ingredient() {
        // lf/crlf problem :)
        let input = "[first]\nme\n[seconds]\nme";
        let e = parse(input).unwrap_err();
        assert_eq!(
            e,
            AisleConfError::DuplicateIngredient {
                name: "me".into(),
                first_span: Span::new(8, 10),
                second_span: Span::new(21, 23)
            }
        )
    }

    #[test]
    fn duplicate_category() {
        // lf/crlf problem :)
        let input = "[cat]\n[cat]\n";
        let e = parse(input).unwrap_err();
        assert_eq!(
            e,
            AisleConfError::DuplicateCategory {
                name: "cat".into(),
                first_span: Span::new(1, 4),
                second_span: Span::new(7, 10)
            }
        )
    }

    const CONF: &str = r#"
[produce]
potatoes

[dairy]
milk
butter
[deli]
chicken

[canned goods]
tuna|chicken of the sea

[empty category]
[another]
"#;

    #[test]
    fn full_shopping_list() {
        let got = parse(CONF).unwrap();

        let expected = vec![
            Category {
                name: "produce",
                ingredients: vec![Ingredient {
                    names: vec!["potatoes"],
                }],
            },
            Category {
                name: "dairy",
                ingredients: vec![
                    Ingredient {
                        names: vec!["milk"],
                    },
                    Ingredient {
                        names: vec!["butter"],
                    },
                ],
            },
            Category {
                name: "deli",
                ingredients: vec![Ingredient {
                    names: vec!["chicken"],
                }],
            },
            Category {
                name: "canned goods",
                ingredients: vec![Ingredient {
                    names: vec!["tuna", "chicken of the sea"],
                }],
            },
            Category {
                name: "empty category",
                ingredients: vec![],
            },
            Category {
                name: "another",
                ingredients: vec![],
            },
        ];

        assert_eq!(expected, got.categories);
    }

    #[test]
    fn conf_write() {
        let got = parse(CONF).unwrap();
        let mut buffer = Vec::new();
        write(&got, &mut buffer).unwrap();
        let serialized = String::from_utf8(buffer).unwrap();
        let got2 = parse(&serialized).unwrap();
        assert_eq!(got, got2);
    }

    #[test]
    fn parse_lenient_with_duplicates() {
        let input = r#"
[dairy]
milk
cheese

[produce]
apple
apple
banana

[meat]
chicken
apple
"#;
        let result = parse_lenient(input);
        let (parsed, warnings) = result.into_result().unwrap();

        // Should have warnings but still parse successfully
        assert!(warnings.has_warnings());
        // Count warnings
        let warning_count = warnings.iter().count();
        assert_eq!(warning_count, 2); // Two duplicate 'apple' entries

        // Check that duplicates were skipped
        assert_eq!(parsed.categories.len(), 3);

        // Check produce category only has apple once
        let produce = &parsed.categories[1];
        assert_eq!(produce.name, "produce");
        assert_eq!(produce.ingredients.len(), 2); // apple and banana
        assert_eq!(produce.ingredients[0].names, vec!["apple"]);
        assert_eq!(produce.ingredients[1].names, vec!["banana"]);

        // Check meat category doesn't have apple
        let meat = &parsed.categories[2];
        assert_eq!(meat.name, "meat");
        assert_eq!(meat.ingredients.len(), 1); // only chicken
        assert_eq!(meat.ingredients[0].names, vec!["chicken"]);
    }

    #[test]
    fn test_ingredient_sort_key() {
        let aisle_conf = r#"
[produce]
apple
banana
carrot

[dairy]
milk
butter | unsalted butter
"#;
        let aisle = parse(aisle_conf).unwrap();

        // Items in aisle.conf get (category_index, ingredient_index)
        assert_eq!(aisle.ingredient_sort_key("apple"), Some((0, 0)));
        assert_eq!(aisle.ingredient_sort_key("banana"), Some((0, 1)));
        assert_eq!(aisle.ingredient_sort_key("carrot"), Some((0, 2)));
        assert_eq!(aisle.ingredient_sort_key("milk"), Some((1, 0)));
        assert_eq!(aisle.ingredient_sort_key("butter"), Some((1, 1)));
        // Synonyms map to the same position
        assert_eq!(aisle.ingredient_sort_key("unsalted butter"), Some((1, 1)));
        // Case insensitive
        assert_eq!(aisle.ingredient_sort_key("Apple"), Some((0, 0)));
        // Unknown ingredient returns None
        assert_eq!(aisle.ingredient_sort_key("mystery"), None);
    }

    #[test]
    fn parse_lenient_with_all_error_types() {
        let input = r#"
orphan ingredient
[dairy|invalid]
milk
[dairy]
cheese
[produce]
apple
"#;
        let result = parse_lenient(input);
        let (parsed, warnings) = result.into_result().unwrap();

        // Should have warnings but still parse successfully
        assert!(warnings.has_warnings());
        let warning_count = warnings.iter().count();
        assert_eq!(warning_count, 3); // orphan ingredient, invalid category, duplicate category

        // Check that we got the valid parts
        assert_eq!(parsed.categories.len(), 2);
        assert_eq!(parsed.categories[0].name, "dairy");
        assert_eq!(parsed.categories[0].ingredients.len(), 1);
        assert_eq!(parsed.categories[0].ingredients[0].names, vec!["cheese"]);
        assert_eq!(parsed.categories[1].name, "produce");
        assert_eq!(parsed.categories[1].ingredients.len(), 1);
        assert_eq!(parsed.categories[1].ingredients[0].names, vec!["apple"]);
    }
}
