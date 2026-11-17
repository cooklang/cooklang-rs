# cooklang-rs

[![crates.io](https://img.shields.io/crates/v/cooklang)](https://crates.io/crates/cooklang)
[![docs.rs](https://img.shields.io/docsrs/cooklang)](https://docs.rs/cooklang/)
[![license](https://img.shields.io/crates/l/cooklang)](./LICENSE)

Cooklang parser in rust with opt-in extensions.

**All regular cooklang files parse as the same recipe**, the extensions are a
superset of the original cooklang format. Also, the **extensions can be turned
off**, so the parser can be used for regular cooklang if you don't like them.

You can see a detailed list of all extensions explained [here](./extensions.md).

The parser also includes:

- Rich error report with annotated code spans. ([like this
  one](https://github.com/Zheoni/cooklang-chef/blob/main/images/error_report.png))
- Unit conversion.
- Recipe scaling.
- A parser for cooklang aisle configuration file.

This parser powers [CookCLI](https://github.com/cooklang/cookcli), the
[Cooklang playground](https://cooklang.github.io/cooklang-rs/), and the Cook
mobile apps and desktop editor, which share recipes through
[Cook Cloud sync](https://cook.md/).

## License

Unit names are seeded from [Unicode CLDR](https://cldr.unicode.org/), whose data is governed by
the [Unicode License V3](./locales/UNICODE-LICENSE). That licence covers the CLDR-derived names in
`locales/*.toml`, the plural rules and samples quoted in `src/convert/locales/plural.rs`, and the code
generated from them. Nothing else:
every hand-authored name, every refusal and its reason, and all other source in this repository is
under the [MIT license](./LICENSE). Each locale file records the CLDR release it was
seeded from; CLDR-derived and hand-written names sit side by side in it and are not marked one by one. The crate is
therefore declared `MIT AND Unicode-3.0`, meaning it *contains* material under both rather than
being dual-licensed; a downstream distributor keeps both notices, which is the same obligation each
licence imposes on its own.
