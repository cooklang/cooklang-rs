# cooklang-rs

[![crates.io](https://img.shields.io/crates/v/cooklang)](https://crates.io/crates/cooklang)
[![docs.rs](https://img.shields.io/docsrs/cooklang)](https://docs.rs/cooklang/)
[![license](https://img.shields.io/crates/l/cooklang)](./LICENSE)

Cooklang parser in rust with opt-in extensions.

**All regular cooklang files parse as the same recipe**, the extensions
are a superset of the original cooklang format. Also, the
**extensions can be turned off**, so the parser can be used for regular cooklang
if you don't like them.

You can see a detailed list of all extensions explained [here](./extensions.md).
