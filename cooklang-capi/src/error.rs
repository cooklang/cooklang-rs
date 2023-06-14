use std::{
    ffi::{c_char, CString},
    fmt,
};

use crate::cstring_new;

/// cbindgen:rename="lowercase"
pub struct Error {
    kind: ErrorKind,
    message: Option<CString>,
}

pub enum ErrorKind {
    None,
    NonUtf8,
    IoError(std::io::Error),
    ParseUnitsFile(toml::de::Error),
    ConverterBuilderError(cooklang::convert::builder::ConverterBuilderError),
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            message: None,
        }
    }

    pub fn is_err(&self) -> bool {
        match self.kind {
            ErrorKind::None => false,
            _ => true,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::None => write!(f, "no error"),
            ErrorKind::NonUtf8 => write!(f, "non utf-8 input"),
            ErrorKind::IoError(ref e) => e.fmt(f),
            ErrorKind::ParseUnitsFile(ref e) => e.fmt(f),
            ErrorKind::ConverterBuilderError(ref e) => e.fmt(f),
        }
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<std::str::Utf8Error> for ErrorKind {
    fn from(_value: std::str::Utf8Error) -> Self {
        Self::NonUtf8
    }
}

impl From<cooklang::convert::builder::ConverterBuilderError> for ErrorKind {
    fn from(value: cooklang::convert::builder::ConverterBuilderError) -> Self {
        Self::ConverterBuilderError(value)
    }
}

/// Allocates space for an error.
///
/// If error information is desired, this function should be called to create
/// a CookError pointer. Then the pointer can be passed to any function that
/// can raise an error.
///
/// If NULL is passed to these functions, no error information will be received.
#[no_mangle]
pub extern "C" fn cook_error_new() -> *mut Error {
    Box::into_raw(Box::new(Error::new(ErrorKind::None)))
}

/// Free the error given.
///
/// This must be called at most once.
#[no_mangle]
pub extern "C" fn cook_error_free(err: *mut Error) {
    unsafe { drop(Box::from_raw(err)) }
}

/// cbindgen:rename-all=SCREAMING_SNAKE_CASE
/// cbindgen:prefix-with-name
#[repr(C)]
pub enum CookErrorCode {
    None = 0,
    NonUtf8,
    IoError,
    ParseUnitsFile,
    ConverterBuilder,
}

/// Get a code for the error.
#[no_mangle]
pub extern "C" fn cook_error_code(err: *const Error) -> CookErrorCode {
    let err = unsafe { &*err };
    match err.kind {
        ErrorKind::None => CookErrorCode::None,
        ErrorKind::NonUtf8 => CookErrorCode::NonUtf8,
        ErrorKind::IoError(_) => CookErrorCode::IoError,
        ErrorKind::ParseUnitsFile(_) => CookErrorCode::ParseUnitsFile,
        ErrorKind::ConverterBuilderError(_) => CookErrorCode::ConverterBuilder,
    }
}

/// Get an error message from the error given.
///
/// The string will be freed when `cook_error_free` is called
#[no_mangle]
pub extern "C" fn cook_error_msg(err: *mut Error) -> *const c_char {
    let err = unsafe { &mut *err };
    let cmsg = cstring_new(format!("{}", err));
    let p = cmsg.as_ptr();
    err.message = Some(cmsg);
    p
}

macro_rules! unwrap_or_bail {
    ($err:expr, $option:expr, $kind:expr) => {
        if let Some(val) = $option {
            val
        } else {
            if !$err.is_null() {
                unsafe {
                    *$err = Error::new($kind.into());
                }
            }
            return std::ptr::null();
        }
    };
    ($err:expr, $result:expr) => {
        unwrap_or_bail!($err, $result; std::ptr::null())
    };
    ($err:expr, $result:expr; $ret:expr) => {
        match $result {
            Ok(val) => val,
            Err(kind) => {
                if !$err.is_null() {
                    unsafe {
                        *$err = Error::new(kind.into());
                    }
                }
                return $ret;
            }
        }
    };
}
