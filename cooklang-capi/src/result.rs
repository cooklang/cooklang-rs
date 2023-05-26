use std::{
    any::Any,
    ffi::{c_char, CStr, CString},
    ptr,
};

use cooklang::error::{CooklangError, CooklangWarning, Report};

use crate::{cstring_new, Error};

pub struct ParseResult {
    value: Option<Box<dyn Any>>,
    report: cooklang::error::Report<CooklangError, CooklangWarning>,
    fancy_report: Option<CString>,
}

impl<T: 'static> From<cooklang::error::PassResult<T, CooklangError, CooklangWarning>>
    for ParseResult
{
    fn from(value: cooklang::error::PassResult<T, CooklangError, CooklangWarning>) -> Self {
        let (value, warnings, errors) = value.into_tuple();
        let report = Report::new(errors, warnings);

        let boxed_value = value.map(|v| Box::new(v) as Box<dyn Any>);

        Self {
            value: boxed_value,
            report,
            fancy_report: None,
        }
    }
}

/// Free the resources of the result object.
#[no_mangle]
pub extern "C" fn cook_result_free(result: *mut ParseResult) {
    unsafe { drop(Box::from_raw(result)) }
}

/// Checks if the result is valid and contains some output
#[no_mangle]
pub extern "C" fn cook_result_is_valid(result: *const ParseResult) -> bool {
    let r = unsafe { &*result };
    r.value.is_some()
}

/// Get the inner Recipe value
///
/// If there is no value, returns NULL.
///
/// If the result was not created with `cook_parse` this will panic.
#[no_mangle]
pub extern "C" fn cook_result_get_recipe(result: *const ParseResult) -> *const cooklang::Recipe {
    let r = unsafe { &*result };
    if let Some(value) = &r.value {
        value.downcast_ref().expect("not recipe")
    } else {
        ptr::null()
    }
}

/// Get the inner Metadata value
///
/// If there is no value, returns NULL.
///
/// If the result was not created with `cook_parse_metadata` this will panic.
#[no_mangle]
pub extern "C" fn cook_result_get_metadata(
    result: *const ParseResult,
) -> *const cooklang::metadata::Metadata {
    let r = unsafe { &*result };
    if let Some(value) = &r.value {
        value.downcast_ref().expect("not metadata")
    } else {
        ptr::null()
    }
}

/// Get the inner AST value
///
/// If there is no value, returns NULL.
///
/// If the result was not created with `cook_parse_ast` this will panic.
#[no_mangle]
pub extern "C" fn cook_result_get_ast<'a>(
    result: *const ParseResult,
) -> *const cooklang::ast::Ast<'a> {
    let r = unsafe { &*result };
    if let Some(value) = &r.value {
        value.downcast_ref().expect("not ast")
    } else {
        ptr::null()
    }
}

/// Generates a fancy report string.
///
/// It may contain warnings and/or errors.
///
/// If no warnings or errors exists, NULL will be returned.
#[no_mangle]
pub extern "C" fn cook_result_fancy_report(
    result: *mut ParseResult,
    file_name: *const c_char,
    source_code: *const c_char,
    hide_warnings: bool,
    error: *mut Error,
) -> *const c_char {
    let result = unsafe { &mut *result };

    if !result.report.has_errors() && !result.report.has_warnings() {
        return ptr::null();
    }

    let source_code = unsafe { CStr::from_ptr(source_code) };
    let source_code = unwrap_or_bail!(error, source_code.to_str());

    let file_name = unsafe { CStr::from_ptr(file_name) };
    let file_name = unwrap_or_bail!(error, file_name.to_str());

    let fancy_report = {
        let mut buf = Vec::new();
        unwrap_or_bail!(
            error,
            result
                .report
                .write(file_name, source_code, hide_warnings, &mut buf)
        );
        cstring_new(buf)
    };

    let p = fancy_report.as_ptr();
    result.fancy_report = Some(fancy_report);
    p
}

/// Prints a fancy report to stdout
#[no_mangle]
pub extern "C" fn cook_result_print(
    result: *const ParseResult,
    file_name: *const c_char,
    source_code: *const c_char,
    hide_warnings: bool,
    error: *mut Error,
) {
    let result = unsafe { &*result };

    if !result.report.has_errors() && !result.report.has_warnings() {
        return;
    }

    let source_code = unsafe { CStr::from_ptr(source_code) };
    let source_code = unwrap_or_bail!(error, source_code.to_str(); ());

    let file_name = unsafe { CStr::from_ptr(file_name) };
    let file_name = unwrap_or_bail!(error, file_name.to_str(); ());

    unwrap_or_bail!(error, result.report.print(file_name, source_code, hide_warnings); ());
}

/// Prints a fancy report to stderr
#[no_mangle]
pub extern "C" fn cook_result_eprint(
    result: *const ParseResult,
    file_name: *const c_char,
    source_code: *const c_char,
    hide_warnings: bool,
    error: *mut Error,
) {
    let result = unsafe { &*result };

    if !result.report.has_errors() && !result.report.has_warnings() {
        return;
    }

    let source_code = unsafe { CStr::from_ptr(source_code) };
    let source_code = unwrap_or_bail!(error, source_code.to_str(); ());

    let file_name = unsafe { CStr::from_ptr(file_name) };
    let file_name = unwrap_or_bail!(error, file_name.to_str(); ());

    unwrap_or_bail!(error, result.report.eprint(file_name, source_code, hide_warnings); ());
}

/*

   TODO

   Access to the individual errors and warnings

*/
