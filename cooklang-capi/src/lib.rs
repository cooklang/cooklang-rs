#[macro_use]
mod error;
mod ccooklang;
mod model;
mod result;

use std::ffi::CString;

pub use ccooklang::*;
pub use error::*;
pub use model::*;
pub use result::*;

#[no_mangle]
pub extern "C" fn cooklang_print_version() {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("cooklang-capi {VERSION}");
}

/// Custom wrapper for [CString::new]. If the input contains a NULL, it is
/// truncated
fn cstring_new<T: Into<Vec<u8>>>(val: T) -> CString {
    match CString::new(val) {
        Ok(s) => s,
        Err(err) => {
            // If the input val has a NULL just show as much as we can.
            let nul = err.nul_position();
            let s = err.into_vec();
            CString::new(s[0..nul].to_owned()).unwrap()
        }
    }
}
