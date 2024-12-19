//! Cooklang [aisle configuration](https://cooklang.org/docs/spec/#the-shopping-list-specification) parser
//!
//! This module is only available with the `aisle` [feaure](crate::_features).
//!
use std::{borrow::Cow, collections::HashMap};

use pest::Parser;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    error::{CowStr, Label, RichError},
    span::Span,
};

// So [parser::Rule] is not public
mod parser {
    use pest_derive::Parser;
    #[derive(Parser)]
    #[grammar = "aisle/grammar.pest"]
    pub struct AisleConfParser;
}
use parser::{AisleConfParser, Rule};

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
    // opt for `reverse`
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

impl AisleConf<'_> {
    /// Returns a reversed configuration, where each key is an ingredient
    /// and the value is its category.
    pub fn reverse(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::with_capacity(self.len.get());
        for cat in &self.categories {
            for igr in &cat.ingredients {
                for name in &igr.names {
                    map.insert(*name, cat.name);
                }
            }
        }
        self.len.set(map.len());
        map
    }

    /// Returns a mapping from each ingredient to the 'common name',
    /// the first alias given in its entry.
    pub fn common_names(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::with_capacity(self.len.get());
        for cat in &self.categories {
            for igr in &cat.ingredients {
                let Some(common_name) = igr.names.first() else {
                    continue;
                };
                for name in &igr.names {
                    map.insert(*name, *common_name);
                }
            }
        }
        self.len.set(map.len());
        map
    }
}

/// Parse an [`AisleConf`] with the cooklang shopping list format
pub fn parse(input: &str) -> Result<AisleConf, AisleConfError> {
    let pairs =
        AisleConfParser::parse(Rule::shopping_list, input).map_err(|e| AisleConfError::Parse {
            span: e.location.into(),
            message: e.variant.message().to_string(),
        })?;

    let mut categories = Vec::new();
    let mut categories_span = HashMap::new();
    let mut names_span = HashMap::new();

    for p in pairs.take_while(|p| p.as_rule() != Rule::EOI) {
        let mut pairs = p.into_inner();
        let name_pair = pairs.next().expect("name");
        let name = name_pair.as_str().trim();
        let current_span = Span::from(name_pair.as_span());

        if let Some(other) = categories_span.insert(name, current_span) {
            return Err(AisleConfError::DuplicateCategory {
                name: name.to_string(),
                first_span: other,
                second_span: current_span,
            });
        }

        let mut ingredients = Vec::new();
        for p in pairs {
            assert_eq!(p.as_rule(), Rule::ingredient, "expected ingredient");
            let mut names = Vec::with_capacity(1);
            for p in p.into_inner() {
                assert_eq!(p.as_rule(), Rule::name, "expected name");
                let name = p.as_str().trim();
                let span = Span::from(p.as_span());
                if let Some(other) = names_span.insert(name, span) {
                    return Err(AisleConfError::DuplicateIngredient {
                        name: name.to_string(),
                        first_span: other,
                        second_span: span,
                    });
                }
                names.push(name);
            }
            ingredients.push(Ingredient { names });
        }
        let category = Category { name, ingredients };

        categories.push(category);
    }

    Ok(AisleConf {
        categories,
        len: std::cell::Cell::new(names_span.len()),
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

impl From<pest::Span<'_>> for Span {
    fn from(value: pest::Span) -> Self {
        Self::new(value.start(), value.end())
    }
}

impl From<pest::error::InputLocation> for Span {
    fn from(value: pest::error::InputLocation) -> Self {
        match value {
            pest::error::InputLocation::Pos(p) => (p..p).into(),
            pest::error::InputLocation::Span((start, end)) => (start..end).into(),
        }
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
        let p = a.common_names();
        assert_eq!(
            vec!["tuna", "tuna"],
            ["tuna", "chicken of the sea"]
                .iter()
                .map(|igr| *p.get(igr).unwrap())
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
