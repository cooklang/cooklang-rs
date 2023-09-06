//! A [cooklang](https://cooklang.org/) parser with opt-in extensions.
//!
//! The extensions create a superset of the original cooklang language and can
//! be turned off. To see a detailed list go to [extensions](_extensions).
//!
//! Also includes:
//! - Rich error report with annotated code spans.
//! - Unit conversion.
//! - Recipe scaling.
//! - A parser for cooklang aisle configuration file.
//!
//! # Basic usage
//! If you just want to parse a **single** `cooklang` file, see [`parse`].
//!
//! As soon as you are going to parse more than one, or want to change the
//! configuration of the parser, construct a parser instance yourself.
//!
//! To construct a parser use [`CooklangParser::new`] or
//! [`CooklangParser::default`] if you want to configure the parser. You can
//! configure which [`Extensions`] are enabled and the [`Converter`] used to
//! convert and check units.
//!
//! ```rust
//! # use cooklang::{CooklangParser, Converter, Extensions};
//! // Create a parser
//! // (this is the default configuration)
//! let parser = CooklangParser::new(Extensions::all(), Converter::default());
//! # assert_eq!(parser, CooklangParser::default());
//! ```
//!
//! Then use the parser:
//!
//! ```rust
//! # use cooklang::CooklangParser;
//! # let parser = CooklangParser::default();
//! let src = "This is an @example.";
//! let name = "Example Recipe";
//! let (recipe, _warnings) = parser.parse(src, name).into_result()?;
//! assert_eq!(recipe.name, name);
//! assert_eq!(recipe.ingredients.len(), 1);
//! assert_eq!(recipe.ingredients[0].name, "example");
//! # assert!(_warnings.is_empty());
//! # Ok::<(), cooklang::error::CooklangReport>(())
//! ```
//!
//! Recipes can be scaled and converted. But the following applies:
//! - Parsing returns a [`ScalableRecipe`].
//! - Only [`ScalableRecipe`] can be [`scale`](ScalableRecipe::scale)d or
//!   [`default_scale`](ScalableRecipe::default_scale)d **only once** to obtain
//!   a [`ScaledRecipe`].
//! - Only [`ScaledRecipe`] can be [`convert`](ScaledRecipe::convert)ed.

#![warn(rustdoc::broken_intra_doc_links, clippy::doc_markdown)]

#[cfg(doc)]
pub mod _extensions {
    #![doc = include_str!("../extensions.md")]
}

#[cfg(doc)]
pub mod _features {
    //! This lib has 2 features, both enabled by default:
    //! - `bundled_units`. Includes a units file with the most common units for
    //!   recipes in English. These units are available to load when you want
    //!   without the need to read a file. The default
    //!   [`Converter`](crate::convert::Converter) use them if this feature is
    //!   enabled. [This is the bundled file](https://github.com/cooklang/cooklang-rs/blob/main/units.toml)
    //!
    //! - `aisle`. Enables the [`aisle`](crate::aisle) module.
}

#[cfg(feature = "aisle")]
pub mod aisle;
pub mod analysis;
pub mod ast;
pub mod convert;
pub mod error;
pub mod ingredient_list;
pub mod located;
pub mod metadata;
pub mod model;
pub mod parser;
pub mod quantity;
pub mod scale;
pub mod span;

#[cfg(feature = "bindings")]
mod bindings;

mod context;
mod lexer;

use bitflags::bitflags;

use error::{CooklangError, CooklangWarning, PassResult};

#[cfg(feature = "bindings")]
pub use bindings::*;

pub use convert::Converter;
pub use located::Located;
pub use metadata::Metadata;
pub use model::*;
pub use quantity::{
    GroupedQuantity, Quantity, QuantityUnit, ScalableQuantity, ScalableValue, ScaledQuantity,
    TotalQuantity, UnitInfo, Value,
};
pub use span::Span;

bitflags! {
    /// Extensions bitflags
    ///
    /// This allows to enable or disable the extensions. See [extensions](_extensions)
    /// for a detailed explanation of all of them.
    ///
    /// [`Extensions::default`] enables all extensions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct Extensions: u32 {
        /// Steps separation is a blank line, not a line break. This may break
        /// compatibility with other cooklang parsers.
        const MULTILINE_STEPS          = 1 << 0;
        /// Enables the [`Modifiers`](crate::ast::Modifiers)
        const COMPONENT_MODIFIERS      = 1 << 1;
        /// Notes with `@igr(note)`
        const COMPONENT_NOTE           = 1 << 2;
        /// Alias with `@igr|alias{}`
        const COMPONENT_ALIAS          = 1 << 3;
        /// Sections with `== Section ==` or `= Section`
        const SECTIONS                 = 1 << 4;
        /// Enable extra checks with units and allows to omit the `%` in simple
        /// cases like `@igr{10 kg}`
        const ADVANCED_UNITS           = 1 << 5;
        /// Set the parsing mode with special metadata keys
        /// `>> [key inside square brackets]: value`
        const MODES                    = 1 << 6;
        /// Searches for inline temperatures in all the recipe text
        const TEMPERATURE              = 1 << 7;
        /// Add text steps with `> This is a text step`
        const TEXT_STEPS               = 1 << 8;
        /// Add support for range values `@igr{2-3}`
        const RANGE_VALUES             = 1 << 9;
        /// Creating a timer without a time becomes an error
        const TIMER_REQUIRES_TIME      = 1 << 10;
        /// This extensions also enables [`Self::COMPONENT_MODIFIERS`].
        const INTERMEDIATE_INGREDIENTS = 1 << 11 | Self::COMPONENT_MODIFIERS.bits();

        /// Enables [`Self::COMPONENT_MODIFIERS`], [`Self::COMPONENT_NOTE`] and [`Self::COMPONENT_ALIAS`]
        const COMPONENT_ALL = Self::COMPONENT_MODIFIERS.bits()
                                | Self::COMPONENT_ALIAS.bits()
                                | Self::COMPONENT_NOTE.bits();

        /// Enables a subset of extensions to maximize compatibility with other
        /// cooklang parsers.
        ///
        /// Currently it enables all the extensions except
        /// [`Self::MULTILINE_STEPS`] and [`Self::TIMER_REQUIRES_TIME`].
        ///
        /// **ADDITIONS TO THE EXTENSIONS THIS ENABLES WILL NOT BE CONSIDERED A BREAKING CHANGE**
        const COMPAT = Self::COMPONENT_MODIFIERS.bits()
                        | Self::COMPONENT_NOTE.bits()
                        | Self::COMPONENT_ALIAS.bits()
                        | Self::SECTIONS.bits()
                        | Self::ADVANCED_UNITS.bits()
                        | Self::MODES.bits()
                        | Self::TEMPERATURE.bits()
                        | Self::TEXT_STEPS.bits()
                        | Self::RANGE_VALUES.bits()
                        | Self::INTERMEDIATE_INGREDIENTS.bits();
    }
}

impl Default for Extensions {
    /// Enables all extensions
    fn default() -> Self {
        Self::all()
    }
}

/// A cooklang parser
///
/// Instantiating this takes time and the first parse may take longer. So
/// you may want to create only one and reuse it.
///
/// The default parser enables all extensions.
///
/// The 2 main methods are [`CooklangParser::parse`] and [`CooklangParser::parse_metadata`].
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CooklangParser {
    extensions: Extensions,
    converter: Converter,
}

pub type RecipeResult = PassResult<ScalableRecipe, CooklangError, CooklangWarning>;
pub type MetadataResult = PassResult<Metadata, CooklangError, CooklangWarning>;

pub type RecipeRefChecker<'a> = Box<dyn Fn(&str) -> bool + 'a>;

impl CooklangParser {
    /// Creates a new parser.
    ///
    /// It is encouraged to reuse the parser and not rebuild it every time.
    pub fn new(extensions: Extensions, converter: Converter) -> Self {
        Self {
            extensions,
            converter,
        }
    }

    /// Get the parser inner converter
    pub fn converter(&self) -> &Converter {
        &self.converter
    }

    /// Get the enabled extensions
    pub fn extensions(&self) -> Extensions {
        self.extensions
    }

    /// Parse a recipe
    ///
    /// As in cooklang the name is external to the recipe, this must be given
    /// here too.
    pub fn parse(&self, input: &str, recipe_name: &str) -> RecipeResult {
        self.parse_with_recipe_ref_checker(input, recipe_name, None)
    }

    /// Same as [`Self::parse`] but with a function that checks if a recipe
    /// reference exists. If the function returns `false` for a recipe reference,
    /// it will be considered an error.
    #[tracing::instrument(level = "debug", name = "parse", skip_all, fields(len = input.len()))]
    pub fn parse_with_recipe_ref_checker(
        &self,
        input: &str,
        recipe_name: &str,
        recipe_ref_checker: Option<RecipeRefChecker>,
    ) -> RecipeResult {
        let mut parser = parser::PullParser::new(input, self.extensions);
        let result = analysis::parse_events(
            &mut parser,
            self.extensions,
            &self.converter,
            recipe_ref_checker,
        );

        result.map(|c| Recipe {
            name: recipe_name.to_string(),
            metadata: c.metadata,
            sections: c.sections,
            ingredients: c.ingredients,
            cookware: c.cookware,
            timers: c.timers,
            inline_quantities: c.inline_quantities,
            data: (),
        })
    }

    /// Parse only the metadata of a recipe
    ///
    /// This is a bit faster than [`Self::parse`] if you only want the metadata
    #[tracing::instrument(level = "debug", name = "metadata", skip_all, fields(len = input.len()))]
    pub fn parse_metadata(&self, input: &str) -> MetadataResult {
        let parser = parser::PullParser::new(input, self.extensions);
        let meta_events = parser.into_meta_iter();
        analysis::parse_events(meta_events, Extensions::empty(), &self.converter, None)
            .map(|c| c.metadata)
    }
}

/// Parse a recipe with a default [`CooklangParser`]. Avoid calling this in a loop.
///
/// The default parser enables all extensions.
///
/// **IMPORTANT:** If you are going to parse more than one recipe you may want
/// to only create one [`CooklangParser`] and reuse it. Every time this function
/// is called, an instance of a parser is constructed. Depending on the
/// configuration, creating an instance and the first call to that can take much
/// longer than later calls to [`CooklangParser::parse`].
pub fn parse(input: &str, recipe_name: &str) -> RecipeResult {
    CooklangParser::default().parse(input, recipe_name)
}
