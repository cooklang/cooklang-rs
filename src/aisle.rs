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
    error::{CowStr, Label, RichError},
    span::Span,
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
    pub fn reverse(&self) -> HashMap<&str, &str> {
        self.ingredients_info()
            .into_iter()
            .map(|(n, i)| (n, i.name))
            .collect()
    }

    /// Returns a reversed configuration, where each ingredient has a
    /// corresponding [`IngredientInfo`]
    pub fn ingredients_info(&self) -> HashMap<&str, IngredientInfo> {
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
                    map.insert(*name, info);
                }
            }
        }
        self.len.set(map.len());
        map
    }
}

/// Parse an [`AisleConf`] with the cooklang shopping list format
pub fn parse(input: &str) -> Result<AisleConf, AisleConfError> {
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
                return Err(AisleConfError::Parse {
                    span: calc_span(name),
                    message: "Invalid category name".to_string(),
                });
            }

            if let Some(&other) = used_categories.get(name) {
                return Err(AisleConfError::DuplicateCategory {
                    name: name.to_string(),
                    first_span: calc_span(other),
                    second_span: calc_span(name),
                });
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
                    return Err(AisleConfError::DuplicateIngredient {
                        name: n.to_string(),
                        first_span: calc_span(other),
                        second_span: calc_span(n),
                    });
                }
                used_names.insert(n);
                names.push(n);
            }
            let names = line.split('|').map(str::trim).collect();
            if let Some(cat) = &mut current_category {
                cat.ingredients.push(Ingredient { names });
            } else {
                return Err(AisleConfError::Parse {
                    span: calc_span(line),
                    message: "Expected category".to_string(),
                });
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
                    write!(w, "|{}", name)?;
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
    fn labels(&self) -> Cow<[Label]> {
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

    fn hints(&self) -> Cow<[CowStr]> {
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
        assert_eq!(
            vec!["tuna", "tuna"],
            ["tuna", "chicken of the sea"]
                .iter()
                .map(|igr| p.get(igr).unwrap().common_name)
                .collect::<Vec<&str>>()
        )
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
}
