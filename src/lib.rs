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
//! If you just want **to parse a single** `cooklang` file, see [`parse`].
//!
//! If you are going to parse more than one, or want to change the
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
//! let (recipe, _warnings) = parser.parse("This is an @example").into_result()?;
//! assert_eq!(recipe.ingredients.len(), 1);
//! assert_eq!(recipe.ingredients[0].name, "example");
//! # assert!(_warnings.is_empty());
//! # Ok::<(), cooklang::error::SourceReport>(())
//! ```
//!
//! Recipes can be scaled and converted. But the following applies:
//! - Parsing returns a [`ScalableRecipe`].
//! - Only [`ScalableRecipe`] can be [`scaled`](ScalableRecipe::scale) or
//!   [`default_scaled`](ScalableRecipe::default_scale) **only once** to obtain
//!   a [`ScaledRecipe`].
//! - Only [`ScaledRecipe`] can be [`converted`](ScaledRecipe::convert).

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
pub mod text;

mod lexer;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use error::PassResult;

pub use analysis::ParseOptions;
pub use convert::Converter;
pub use located::Located;
pub use metadata::Metadata;
pub use model::*;
pub use parser::Modifiers;
pub use quantity::{
    GroupedQuantity, Quantity, ScalableQuantity, ScalableValue, ScaledQuantity, Value,
};
pub use span::Span;
pub use text::Text;

bitflags! {
    /// Extensions bitflags
    ///
    /// This allows to enable or disable the extensions. See [extensions](_extensions)
    /// for a detailed explanation of all of them.
    ///
    /// [`Extensions::default`] enables all extensions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Extensions: u32 {
        /// Enables the [`Modifiers`](crate::ast::Modifiers)
        const COMPONENT_MODIFIERS      = 1 << 1;
        /// Alias with `@igr|alias{}`
        const COMPONENT_ALIAS          = 1 << 3;
        /// Enable extra checks with units and allows to omit the `%` in simple
        /// cases like `@igr{10 kg}`
        const ADVANCED_UNITS           = 1 << 5;
        /// Set the parsing mode with special metadata keys
        /// `>> [key inside square brackets]: value`
        const MODES                    = 1 << 6;
        /// Searches for inline quantities in all the recipe text
        const INLINE_QUANTITIES        = 1 << 7;
        /// Add support for range values `@igr{2-3}`
        const RANGE_VALUES             = 1 << 9;
        /// Creating a timer without a time becomes an error
        const TIMER_REQUIRES_TIME      = 1 << 10;
        /// This extensions also enables [`Self::COMPONENT_MODIFIERS`].
        const INTERMEDIATE_PREPARATIONS = 1 << 11 | Self::COMPONENT_MODIFIERS.bits();

        /// Enables a subset of extensions to maximize compatibility with other
        /// cooklang parsers.
        ///
        /// Currently it enables all the extensions except
        /// [`Self::TIMER_REQUIRES_TIME`].
        ///
        /// **ADDITIONS TO THE EXTENSIONS THIS ENABLES WILL NOT BE CONSIDERED A BREAKING CHANGE**
        const COMPAT = Self::COMPONENT_MODIFIERS.bits()
                        | Self::COMPONENT_ALIAS.bits()
                        | Self::ADVANCED_UNITS.bits()
                        | Self::MODES.bits()
                        | Self::INLINE_QUANTITIES.bits()
                        | Self::RANGE_VALUES.bits()
                        | Self::INTERMEDIATE_PREPARATIONS.bits();
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
///
/// You can also skip using this struct and use [`parser::PullParser`] and [`analysis::parse_events`].
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CooklangParser {
    extensions: Extensions,
    converter: Converter,
}

pub type RecipeResult = PassResult<ScalableRecipe>;
pub type MetadataResult = PassResult<Metadata>;

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

    /// Creates a new extended parser
    ///
    /// This enables all extensions and uses the bundled units.
    /// It is encouraged to reuse the parser and not rebuild it every time.
    #[cfg(feature = "bundled_units")]
    pub fn extended() -> Self {
        Self::new(Extensions::all(), Converter::bundled())
    }

    /// Creates a new canonical parser
    ///
    /// This disables all extensions and does not use units.
    ///
    /// It is encouraged to reuse the parser and not rebuild it every time.
    pub fn canonical() -> Self {
        Self::new(Extensions::empty(), Converter::empty())
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
    pub fn parse(&self, input: &str) -> RecipeResult {
        self.parse_with_options(input, ParseOptions::default())
    }

    /// Same as [`Self::parse`] but with aditional options
    #[tracing::instrument(level = "debug", name = "parse", skip_all, fields(len = input.len()))]
    pub fn parse_with_options(&self, input: &str, options: ParseOptions) -> RecipeResult {
        let mut parser = parser::PullParser::new(input, self.extensions);
        analysis::parse_events(
            &mut parser,
            input,
            self.extensions,
            &self.converter,
            options,
        )
    }

    /// Parse only the metadata of a recipe
    ///
    /// This is a bit faster than [`Self::parse`] if you only want the metadata
    pub fn parse_metadata(&self, input: &str) -> MetadataResult {
        self.parse_metadata_with_options(input, ParseOptions::default())
    }

    /// Same as [`Self::parse_metadata`] but with aditional options
    #[tracing::instrument(level = "debug", name = "metadata", skip_all, fields(len = input.len()))]
    pub fn parse_metadata_with_options(
        &self,
        input: &str,
        options: ParseOptions,
    ) -> MetadataResult {
        let parser = parser::PullParser::new(input, self.extensions);
        let meta_events = parser.into_meta_iter();
        analysis::parse_events(
            meta_events,
            input,
            self.extensions,
            &self.converter,
            options,
        )
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
pub fn parse(input: &str) -> RecipeResult {
    CooklangParser::default().parse(input)
}
