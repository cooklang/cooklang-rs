use std::{
    ffi::{c_char, CStr},
    ptr,
};

use cooklang::{
    convert::{units_file::UnitsFile, Converter},
    CooklangParser, Extensions,
};

use crate::{Error, ErrorKind, ParseResult};

pub const COOK_EXT_MULTILINE_STEPS: u32 = 1 << 0;
pub const COOK_EXT_COMPONENT_MODIFIERS: u32 = 1 << 1;
pub const COOK_EXT_COMPONENT_NOTE: u32 = 1 << 2;
pub const COOK_EXT_COMPONENT_ALIAS: u32 = 1 << 3;
pub const COOK_EXT_SECTIONS: u32 = 1 << 4;
pub const COOK_EXT_ADVANCED_UNITS: u32 = 1 << 5;
pub const COOK_EXT_MODES: u32 = 1 << 6;
pub const COOK_EXT_TEMPERATURE: u32 = 1 << 7;
pub const COOK_EXT_TEXT_STEPS: u32 = 1 << 8;
pub const COOK_EXT_RANGE_VALUES: u32 = 1 << 9;

pub struct CookParser(pub(crate) CooklangParser);

/// Creates a new parser.
///
/// Creating the parser is not cheap, so for parsing multiple recipes it's not
/// optimal to recreate it every time.
#[no_mangle]
pub extern "C" fn cook_parser_new(extensions: u32) -> *const CookParser {
    cook_parser_new_with_converter(extensions, ptr::null(), 0, ptr::null_mut())
}

/// Creates a new parser with custom units.
///
/// Creating the parser is not cheap, so for parsing multiple recipes it's not
/// optimal to recreate it every time.
///
/// Adding custom units can fail.
#[no_mangle]
pub extern "C" fn cook_parser_new_with_converter(
    extensions: u32,
    units_files: *const *const c_char,
    units_files_len: usize,
    error: *mut Error,
) -> *const CookParser {
    let mut builder = CooklangParser::builder();
    builder.set_extensions(Extensions::from_bits_truncate(extensions));
    if !units_files.is_null() && units_files_len > 0 {
        let mut converter_builder = Converter::builder();
        let units_files_slice = unsafe { std::slice::from_raw_parts(units_files, units_files_len) };
        for file_path_ptr in units_files_slice {
            let file_path_cstr = unsafe { CStr::from_ptr(*file_path_ptr) };
            // ! Maybe this should allow non utf8 paths
            let file_path_str = unwrap_or_bail!(error, file_path_cstr.to_str());
            let content = unwrap_or_bail!(error, std::fs::read_to_string(file_path_str));
            let units_file: UnitsFile = unwrap_or_bail!(
                error,
                toml::from_str(&content).map_err(ErrorKind::ParseUnitsFile)
            );
            unwrap_or_bail!(error, converter_builder.add_units_file(units_file));
        }
        let converter = unwrap_or_bail!(error, converter_builder.finish());
        builder.set_converter(converter);
    }

    Box::into_raw(Box::new(CookParser(builder.finish())))
}

/// Free the given parser.
///
/// This must be called at most once.
#[no_mangle]
pub extern "C" fn cook_parser_free(parser: *const CookParser) {
    unsafe { drop(Box::from_raw(parser as *mut CookParser)) }
}

/// Parse a recipe.
///
/// The `error` param is for fatal errors like when the input is not utf-8,
/// not parsing errors of bad syntax.
///
/// The result must be freed with `cook_result_free`.
#[no_mangle]
pub extern "C" fn cook_parse(
    parser: *const CookParser,
    input: *const c_char,
    recipe_name: *const c_char,
    error: *mut Error,
) -> *const ParseResult {
    let parser = unsafe { &*parser };

    let input = unsafe { CStr::from_ptr(input) };
    let input = unwrap_or_bail!(error, input.to_str());

    let recipe_name = unsafe { CStr::from_ptr(recipe_name) };
    let recipe_name = unwrap_or_bail!(error, recipe_name.to_str());

    let result = parser.0.parse(input, recipe_name).map(crate::Recipe::new);
    Box::into_raw(Box::new(ParseResult::from(result)))
}

/// Parse a recipe, only metadata.
///
/// The `error` param is for fatal errors like when the input is not utf-8,
/// not parsing errors of bad syntax.
///
/// The result must be freed with `cook_result_free`.
#[no_mangle]
pub extern "C" fn cook_parse_metadata(
    parser: *const CookParser,
    input: *const c_char,
    error: *mut Error,
) -> *const ParseResult {
    let parser = unsafe { &*parser };

    let input = unsafe { CStr::from_ptr(input) };
    let input = unwrap_or_bail!(error, input.to_str());

    let result = parser.0.parse_metadata(input).map(crate::Metadata::new);
    Box::into_raw(Box::new(ParseResult::from(result)))
}

/// Parse a recipe into an AST, no analysis
///
/// The `error` param is for fatal errors like when the input is not utf-8,
/// not parsing errors of bad syntax.
#[no_mangle]
pub extern "C" fn cook_parse_ast(
    input: *const c_char,
    extensions: u32,
    error: *mut Error,
) -> *const ParseResult {
    let input = unsafe { CStr::from_ptr(input) };
    let input = unwrap_or_bail!(error, input.to_str());

    let extensions = Extensions::from_bits_truncate(extensions);

    let (val, warnings, errors) = cooklang::parser::parse(input, extensions).into_tuple();

    let val = val.map(crate::Ast);
    let warnings = warnings
        .into_iter()
        .map(cooklang::error::CooklangWarning::from)
        .collect();
    let errors = errors
        .into_iter()
        .map(cooklang::error::CooklangError::from)
        .collect();

    let result = cooklang::error::PassResult::new(val, warnings, errors);
    Box::into_raw(Box::new(ParseResult::from(result)))
}
