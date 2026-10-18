//! The snapshot that pins every decision the naming and preference layers make. Each module's own rules are tested
//! beside it, and what the locale files must obey is checked by `build.rs` as it reads them.

mod snapshot;
