//! A [cooklang](https://cooklang.org/) parser with opt-in extensions.
//!
//! The extensions creates a superset of the original cooklang language and can
//! be turned off. To see a detailed list go to [extensions](https://github.com/cooklang/cooklang-rs/blob/main/extensions.md).
//!
//! Also includes:
//! - Rich error report with annotated code spans.
//! - Unit conversion.
//! - Recipe scaling.
//! - A parser for cooklang aisle configuration file.

#![deny(rustdoc::broken_intra_doc_links)]

#[cfg(feature = "aisle")]
pub mod aisle;
pub mod ast;
pub mod convert;
pub mod error;
pub mod ingredient_list;
pub mod metadata;
pub mod model;
pub mod parser;
pub mod quantity;
pub mod scale;

mod analysis;
mod context;
mod lexer;
mod located;
mod span;

use bitflags::bitflags;
use convert::Converter;
use error::{CooklangError, CooklangWarning, PassResult};
use metadata::Metadata;
pub use model::{Recipe, ScaledRecipe};

bitflags! {
    /// Extensions bitflags
    ///
    /// This allows to enable or disable the extensions. See [extensions](https://github.com/Zheoni/cooklang-rs/blob/main/docs/extensions.md)
    /// for a detailed explanation of all of them.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct Extensions: u32 {
        /// Steps separation is a blank line, not a line break. This may break
        /// compatibility with other cooklang parsers.
        const MULTILINE_STEPS          = 1 << 0;
        const COMPONENT_MODIFIERS      = 1 << 1;
        const COMPONENT_NOTE           = 1 << 2;
        const COMPONENT_ALIAS          = 1 << 3;
        const SECTIONS                 = 1 << 4;
        const ADVANCED_UNITS           = 1 << 5;
        const MODES                    = 1 << 6;
        const TEMPERATURE              = 1 << 7;
        const TEXT_STEPS               = 1 << 8;
        const RANGE_VALUES             = 1 << 9;
        const TIMER_REQUIRES_TIME      = 1 << 10;
        /// This extensions also enables [Self::COMPONENT_MODIFIERS].
        const INTERMEDIATE_INGREDIENTS = 1 << 11 | Self::COMPONENT_MODIFIERS.bits();

        /// Enables [Self::COMPONENT_MODIFIERS], [Self::COMPONENT_NOTE] and [Self::COMPONENT_ALIAS]
        const COMPONENT_ALL = Self::COMPONENT_MODIFIERS.bits()
                                | Self::COMPONENT_ALIAS.bits()
                                | Self::COMPONENT_NOTE.bits();

        /// Enables a subset of extensions to maximize compatibility with other
        /// cooklang parsers.
        ///
        /// Currently it enables all the extensions except [Self::MULTILINE_STEPS].
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
    fn default() -> Self {
        Self::all()
    }
}

/// A cooklang parser
#[derive(Debug, Default, Clone)]
pub struct CooklangParser {
    extensions: Extensions,
    converter: Converter,
}

/// A helper parser builder.
///
/// If no [Converter] given, [Converter::default] will be used. Note that
/// [Converter::default] changes depending on the `bundled_units` feature.
#[derive(Debug, Default, Clone)]
pub struct CooklangParserBuilder {
    extensions: Extensions,
    converter: Option<Converter>,
}

impl CooklangParserBuilder {
    pub fn with_converter(mut self, converter: Converter) -> Self {
        self.set_converter(converter);
        self
    }

    pub fn set_converter(&mut self, converter: Converter) -> &mut Self {
        self.converter = Some(converter);
        self
    }

    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.set_extensions(extensions);
        self
    }

    pub fn set_extensions(&mut self, extensions: Extensions) -> &mut Self {
        self.extensions = extensions;
        self
    }

    pub fn finish(self) -> CooklangParser {
        let converter = self.converter.unwrap_or_default();
        CooklangParser {
            extensions: self.extensions,
            converter,
        }
    }
}

pub type RecipeResult = PassResult<Recipe, CooklangError, CooklangWarning>;
pub type MetadataResult = PassResult<Metadata, CooklangError, CooklangWarning>;

pub type RecipeRefChecker<'a> = Box<dyn Fn(&str) -> bool + 'a>;

impl CooklangParser {
    /// Start initializing a new parser
    pub fn builder() -> CooklangParserBuilder {
        CooklangParserBuilder::default()
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

    /// Same as [Self::parse] but with a function that checks if a recipe
    /// reference exists. If the function returns `false` for a recipe reference,
    /// it will be considered an error.
    #[tracing::instrument(level = "debug", name = "parse", skip_all, fields(len = input.len()))]
    pub fn parse_with_recipe_ref_checker(
        &self,
        input: &str,
        recipe_name: &str,
        recipe_ref_checker: Option<RecipeRefChecker>,
    ) -> RecipeResult {
        let mut r = parser::parse(input, self.extensions).into_context_result();
        if r.invalid() {
            return r.discard_output();
        }
        let ast = r.take_output().unwrap();
        analysis::parse_ast(ast, self.extensions, &self.converter, recipe_ref_checker)
            .into_context_result()
            .merge(r)
            .map(|c| Recipe {
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
    /// This is a bit faster than [Self::parse] if you only want the metadata
    #[tracing::instrument(level = "debug", name = "metadata", skip_all, fields(len = input.len()))]
    pub fn parse_metadata(&self, input: &str) -> MetadataResult {
        let mut r = parser::parse_metadata(input).into_context_result();
        if r.invalid() {
            return r.discard_output();
        }
        let ast = r.take_output().unwrap();
        analysis::parse_ast(ast, Extensions::empty(), &self.converter, None)
            .into_context_result()
            .merge(r)
            .map(|c| c.metadata)
    }
}

/// Parse a recipe with a default [CooklangParser]
pub fn parse(input: &str, recipe_name: &str) -> PassResult<Recipe, CooklangError, CooklangWarning> {
    CooklangParser::default().parse(input, recipe_name)
}
