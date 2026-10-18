fn main() {
    #[cfg(feature = "bundled_units")]
    {
        println!("cargo::rerun-if-changed=units.toml");
        generate_bundled();
    }
    // `locales/` is the only data the build reads: one file per locale, plus
    // the preference rows and the parent chain. Each file registers itself,
    // because a directory alone does not make cargo notice an edited file.
    println!("cargo::rerun-if-changed=locales");
    generate_locales();
    generate_parent_locales();
    generate_preferences();
}

#[cfg(feature = "bundled_units")]
fn generate_bundled() {
    use quote::{format_ident, quote};

    let text = std::fs::read_to_string("units.toml").unwrap();
    let data: toml::Value = toml::from_str(&text).unwrap();
    let uf = data.as_table().unwrap();

    macro_rules! quote_enum {
        ($name:ident :: $variant:expr) => {{
            let s = $variant;
            let v = format_ident!("{}{}", s[..1].to_ascii_uppercase(), s[1..]);
            quote! { $name::#v }
        }};
    }

    fn none() -> proc_macro2::TokenStream {
        quote! { None }
    }
    fn some<T: quote::ToTokens>(v: T) -> proc_macro2::TokenStream {
        quote! { Some(#v) }
    }

    let default_system = uf
        .get("default_system")
        .map(|sys| {
            let sys = quote_enum!(System::sys.as_str().unwrap());
            quote! { Some(#sys) }
        })
        .unwrap_or_else(none);

    let si = uf
        .get("si")
        .map(|si| {
            // ! IMPORTANT: Same order as in SIPrefix enum
            const SIPREFIX: [&str; 6] = ["kilo", "hecto", "deca", "deci", "centi", "milli"];

            let prefixes = si
                .get("prefixes")
                .map(|pre| {
                    let pre = pre.as_table().unwrap();
                    let it = SIPREFIX.iter().map(|&prefix| {
                        let vals = pre.get(prefix).unwrap().as_array().unwrap();
                        let vals_it = vals.iter().map(|s| {
                            let s = s.as_str().unwrap();
                            quote!(#s.to_string())
                        });
                        quote!(vec![#(#vals_it),*])
                    });
                    quote! { Some(EnumMap::from_array([#(#it),*])) }
                })
                .unwrap_or_else(none);

            let symbol_prefixes = si
                .get("symbol_prefixes")
                .map(|pre| {
                    let pre = pre.as_table().unwrap();
                    let it = SIPREFIX.iter().map(|&prefix| {
                        let vals = pre.get(prefix).unwrap().as_array().unwrap();
                        let vals_it = vals.iter().map(|s| {
                            let s = s.as_str().unwrap();
                            quote!(#s.to_string())
                        });
                        quote!(vec![#(#vals_it),*])
                    });
                    quote! { Some(EnumMap::from_array([#(#it),*])) }
                })
                .unwrap_or_else(none);

            let precedence = si
                .get("precedence")
                .map(|p| quote_enum!(Precedence::p.as_str().unwrap()))
                .unwrap_or_else(|| quote!(Precedence::default()));

            quote! { Some(SI {
                prefixes: #prefixes,
                symbol_prefixes: #symbol_prefixes,
                precedence: #precedence,
            }) }
        })
        .unwrap_or_else(none);

    let fractions = uf
        .get("fractions")
        .map(|frac| {
            fn quote_fractions_config_wrapper(v: &toml::Value) -> proc_macro2::TokenStream {
                if let Some(b) = v.as_bool() {
                    quote! { FractionsConfigWrapper::Toggle(#b) }
                } else if let Some(v) = v.as_table() {
                    let enabled = v
                        .get("enabled")
                        .map(|v| v.as_bool().unwrap())
                        .map(some)
                        .unwrap_or_else(none);
                    let accuracy = v
                        .get("accuracy")
                        .map(|v| v.as_float().unwrap() as f32)
                        .map(some)
                        .unwrap_or_else(none);
                    let max_denominator = v
                        .get("max_denominator")
                        .map(|v| v.as_integer().unwrap() as u8)
                        .map(some)
                        .unwrap_or_else(none);
                    let max_whole = v
                        .get("max_whole")
                        .map(|v| v.as_integer().unwrap() as u32)
                        .map(some)
                        .unwrap_or_else(none);

                    quote! { FractionsConfigWrapper::Custom(FractionsConfigHelper {
                        enabled: #enabled,
                        accuracy: #accuracy,
                        max_denominator: #max_denominator,
                        max_whole: #max_whole,
                    }) }
                } else {
                    panic!("bad fractions value")
                }
            }

            let all = frac
                .get("all")
                .map(quote_fractions_config_wrapper)
                .map(some)
                .unwrap_or_else(none);
            let metric = frac
                .get("metric")
                .map(quote_fractions_config_wrapper)
                .map(some)
                .unwrap_or_else(none);
            let imperial = frac
                .get("imperial")
                .map(quote_fractions_config_wrapper)
                .map(some)
                .unwrap_or_else(none);

            let quantity = {
                let mut n = 0;
                let entries = frac
                    .get("quantity")
                    .map(|t| {
                        let t = t.as_table().unwrap();
                        n = t.len();
                        let entries = t.iter().map(|(k, v)| {
                            let q = quote_enum!(PhysicalQuantity::k);
                            let val = quote_fractions_config_wrapper(v);
                            quote! { m.insert(#q, #val); }
                        });
                        quote! { #(#entries)* }
                    })
                    .unwrap_or_default();
                quote! { {
                    let mut m = HashMap::with_capacity(#n);
                    #entries
                    m
                } }
            };
            let unit = {
                let mut n = 0;
                let entries = frac
                    .get("unit")
                    .map(|t| {
                        let t = t.as_table().unwrap();
                        n = t.len();
                        let entries = t.iter().map(|(k, v)| {
                            let val = quote_fractions_config_wrapper(v);
                            quote! { m.insert(#k.to_string(), #val); }
                        });
                        quote! { #(#entries)* }
                    })
                    .unwrap_or_default();
                quote! { {
                    let mut m = HashMap::with_capacity(#n);
                    #entries
                    m
                } }
            };

            quote! { Some(Fractions {
                all: #all,
                metric: #metric,
                imperial: #imperial,
                quantity: #quantity,
                unit: #unit,
            }) }
        })
        .unwrap_or_else(none);

    let extend = if uf.get("extend").is_some() {
        unimplemented!("base units.toml does not have extend");
    } else {
        quote! { None }
    };

    let quantity = uf
        .get("quantity")
        .map(|v| {
            let v = v.as_array().unwrap();
            let entries = v.iter().map(|qg| {
                let qg = qg.as_table().unwrap();

                let q = qg.get("quantity").unwrap().as_str().unwrap();
                let quantity = quote_enum!(PhysicalQuantity::q);

                let best = qg
                    .get("best")
                    .map(|b| {
                        let variant = if let Some(v) = b.as_array() {
                            let vals = v.iter().map(|v| {
                                let v = v.as_str().unwrap();
                                quote! {#v.to_string()}
                            });
                            quote! {Unified(vec![#(#vals),*])}
                        } else if let Some(t) = b.as_table() {
                            let metric = {
                                let v = t.get("metric").unwrap().as_array().unwrap();
                                let vals = v.iter().map(|v| {
                                    let v = v.as_str().unwrap();
                                    quote! {#v.to_string()}
                                });
                                quote! {vec![#(#vals),*]}
                            };
                            let imperial = {
                                let v = t.get("imperial").unwrap().as_array().unwrap();
                                let vals = v.iter().map(|v| {
                                    let v = v.as_str().unwrap();
                                    quote! {#v.to_string()}
                                });
                                quote! {vec![#(#vals),*]}
                            };
                            quote! { BySystem {
                                metric: #metric,
                                imperial: #imperial,
                            } }
                        } else {
                            panic!("Bad best units")
                        };
                        quote! { Some(BestUnits::#variant) }
                    })
                    .unwrap_or_else(none);

                let units = qg
                    .get("units")
                    .map(|un| {
                        fn quote_unit_entry(v: &toml::Value) -> proc_macro2::TokenStream {
                            let v = v.as_table().unwrap();
                            let names = {
                                let v = v.get("names").unwrap().as_array().unwrap();
                                let vals = v.iter().map(|s| {
                                    let s = s.as_str().unwrap();
                                    quote! { Arc::from(#s) }
                                });
                                quote! { vec![#(#vals),*] }
                            };
                            let symbols = {
                                let v = v.get("symbols").unwrap().as_array().unwrap();
                                let vals = v.iter().map(|s| {
                                    let s = s.as_str().unwrap();
                                    quote! { Arc::from(#s) }
                                });
                                quote! { vec![#(#vals),*] }
                            };
                            let aliases = if let Some(v) = v.get("aliases") {
                                let v = v.as_array().unwrap();
                                let vals = v.iter().map(|s| {
                                    let s = s.as_str().unwrap();
                                    quote! { Arc::from(#s) }
                                });
                                quote! { vec![#(#vals),*] }
                            } else {
                                quote!(vec![])
                            };
                            let ratio = {
                                let v = v.get("ratio").unwrap();
                                v.as_float().or_else(|| v.as_integer().map(|i| i as f64))
                            };
                            let difference = v
                                .get("difference")
                                .and_then(|v| {
                                    v.as_float().or_else(|| v.as_integer().map(|i| i as f64))
                                })
                                .unwrap_or_default();
                            let expand_si = v
                                .get("expand_si")
                                .and_then(|v| v.as_bool())
                                .unwrap_or_default();

                            quote! { UnitEntry {
                                names: #names,
                                symbols: #symbols,
                                aliases: #aliases,
                                ratio: #ratio,
                                difference: #difference,
                                expand_si: #expand_si,
                            } }
                        }

                        let variant = if let Some(v) = un.as_array() {
                            let v = v.iter().map(quote_unit_entry);
                            quote! { Unified(vec![#(#v),*]) }
                        } else if let Some(t) = un.as_table() {
                            let metric = t
                                .get("metric")
                                .map(|v| v.as_array().unwrap().iter())
                                .unwrap_or_default()
                                .map(quote_unit_entry);
                            let imperial = t
                                .get("imperial")
                                .map(|v| v.as_array().unwrap().iter())
                                .unwrap_or_default()
                                .map(quote_unit_entry);
                            let unspecified = t
                                .get("unspecified")
                                .map(|v| v.as_array().unwrap().iter())
                                .unwrap_or_default()
                                .map(quote_unit_entry);
                            quote! { BySystem {
                                metric: vec![#(#metric),*],
                                imperial: vec![#(#imperial),*],
                                unspecified: vec![#(#unspecified),*],
                            } }
                        } else {
                            panic!("Bad quantity units")
                        };
                        quote! { Some(Units::#variant) }
                    })
                    .unwrap_or_else(none);

                quote! {
                    QuantityGroup {
                        quantity: #quantity,
                        best: #best,
                        units: #units,
                    }
                }
            });
            quote! { vec![#(#entries),*] }
        })
        .unwrap_or_else(|| quote! { vec![] });

    let tokens = quote! {
        mod __bundled_units {
            use super::*;
            pub fn get_bundled() -> UnitsFile {
                UnitsFile {
                    default_system: #default_system,
                    si: #si,
                    fractions: #fractions,
                    extend: #extend,
                    quantity: #quantity,
                }
            }
        }
    };

    let synfile = syn::parse2(tokens).unwrap();
    let generated = prettyplease::unparse(&synfile);
    let outpath = format!("{}/bundled_units.rs", std::env::var("OUT_DIR").unwrap());
    std::fs::write(outpath, generated).unwrap();
}

/// Emits the locale name tables from `locales/<tag>.toml`.
///
/// One hand-editable file per locale, started by `xtask/new-locale.sh` and
/// edited since. There is no merge: the file states what the locale names, so
/// adding a name means adding a line and refusing one means not having it.
/// Names a locale must *not* resolve are a comment at the foot of its file --
/// prose, because that is all they ever were.
///
/// Returns the tags, which `generate_preferences` does not need but the
/// caller does.
fn generate_locales() -> Vec<String> {
    use std::collections::BTreeMap;

    #[derive(serde::Deserialize)]
    struct SeededLocale {
        /// Which CLDR release the names were seeded from. Provenance, not a
        /// live dependency: nothing re-reads CLDR.
        cldr_version: String,
        #[serde(default)]
        plural_rules: Vec<PluralRule>,
        #[serde(default)]
        unit: Vec<SeededUnit>,
        /// Present once a speaker has confirmed the locale. Absent means
        /// unreviewed, which `reviewed_for_tag` refuses to serve.
        reviewed: Option<Reviewed>,
    }
    #[derive(serde::Deserialize)]
    struct PluralRule {
        category: String,
        condition: String,
    }
    #[derive(serde::Deserialize)]
    struct SeededUnit {
        id: String,
        symbol: Option<String>,
        #[serde(default)]
        aliases: Vec<String>,
        #[serde(default)]
        long: BTreeMap<String, String>,
        /// Present when the name is hand-authored rather than seeded from
        /// CLDR. Required nowhere: the review gate is what asks about an
        /// uncited name.
        #[serde(default)]
        #[allow(dead_code)]
        reason: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct Reviewed {
        #[allow(dead_code)]
        date: toml::value::Datetime,
        by: String,
        sources: Vec<String>,
    }

    // every locales/*.toml except the two that are not locales
    let mut files: Vec<String> = std::fs::read_dir("locales")
        .expect("locales/ exists")
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            let stem = name.strip_suffix(".toml")?.to_string();
            (!matches!(stem.as_str(), "preferences" | "parent-locales")).then_some(stem)
        })
        .collect();
    files.sort();
    assert!(files.len() >= 6, "only {} locales found", files.len());

    // the metadata is borrowed and always read: `tag_for` walks the fallback chain and the review
    // gate checks `reviewed`, neither of which needs a locale built
    let mut meta = String::from("pub(crate) static LOCALE_META: &[(&str, &str, bool)] = &[\n");
    // one builder per locale, so asking for one does not build the other 37
    let mut built = String::from("pub(crate) static BUILDERS: &[fn() -> Arc<Locale>] = &[\n");

    for tag in &files {
        let path = format!("locales/{tag}.toml");
        println!("cargo::rerun-if-changed={path}");
        let text = std::fs::read_to_string(&path).unwrap();
        let locale: SeededLocale = toml::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"));

        let reviewed = match &locale.reviewed {
            Some(r) => {
                assert!(
                    !r.by.trim().is_empty(),
                    "{path}: `reviewed` has an empty `by`"
                );
                assert!(!r.sources.is_empty(), "{path}: `reviewed` cites no sources");
                true
            }
            None => false,
        };

        // sorted by identifier, so the generated table reads the same way
        // whatever order a file happens to list its units in
        let mut table: BTreeMap<&str, &SeededUnit> = BTreeMap::new();
        for unit in &locale.unit {
            assert!(
                table.insert(unit.id.as_str(), unit).is_none(),
                "{path}: '{}' is named twice",
                unit.id
            );
            assert!(
                unit.symbol.is_some() || !unit.aliases.is_empty(),
                "{path}: '{}' has neither a symbol nor an alias",
                unit.id
            );
        }

        let rules = locale
            .plural_rules
            .iter()
            .map(|r| format!("({:?}, {:?})", r.category, r.condition))
            .collect::<Vec<_>>()
            .join(", ");
        meta.push_str(&format!(
            "    ({tag:?}, {:?}, {reviewed}),\n",
            locale.cldr_version
        ));
        built.push_str(&format!("    || locale({tag:?}, &[{rules}], vec![\n"));
        for (id, unit) in table {
            let symbol = match &unit.symbol {
                Some(s) => format!("Some({s:?})"),
                None => "None".to_string(),
            };
            let mut aliases: Vec<&str> = unit.aliases.iter().map(String::as_str).collect();
            aliases.sort_unstable();
            aliases.dedup();
            let aliases = aliases
                .iter()
                .map(|a| format!("{a:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            let longs = unit
                .long
                .iter()
                .map(|(c, p)| format!("({c:?}, {p:?})"))
                .collect::<Vec<_>>()
                .join(", ");
            built.push_str(&format!(
                "        unit({id:?}, {symbol}, &[{aliases}], &[{longs}]),\n"
            ));
        }
        built.push_str("    ]),\n");
    }
    meta.push_str("];\n");
    built.push_str("];\n");

    let dest = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("cldr_locales.rs");
    std::fs::write(dest, meta + "\n" + &built).unwrap();

    files
}

/// Emits CLDR's explicit parent chain, so that `en-NZ` reaches `en-001` and
/// Commonwealth English rather than truncating to US English.
fn generate_parent_locales() {
    use std::collections::BTreeMap;

    #[derive(serde::Deserialize)]
    struct ParentLocales {
        parent: BTreeMap<String, String>,
    }
    let text = std::fs::read_to_string("locales/parent-locales.toml").unwrap();
    let parents = toml::from_str::<ParentLocales>(&text)
        .unwrap_or_else(|e| panic!("locales/parent-locales.toml: {e}"))
        .parent;

    let mut out = String::from("pub(crate) static PARENT_LOCALES: &[(&str, &str)] = &[\n");
    for (child, parent) in &parents {
        out.push_str(&format!("    ({child:?}, {parent:?}),\n"));
    }
    out.push_str("];\n");

    let dest = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("cldr_parents.rs");
    std::fs::write(dest, out).unwrap();
}

/// Emits the preference tables from `locales/preferences/` as enum
/// paths, so a unit that is not in `definitions.rs` fails to compile rather
/// than at the first lookup.
fn generate_preferences() {
    #[derive(serde::Deserialize, Default)]
    #[serde(default)]
    struct Preferences {
        system: Vec<SystemRows>,
        region: Vec<RegionRows>,
        /// The frozen rows of the legacy locale, keyed by system like `system`
        legacy: Vec<SystemRows>,
    }
    #[derive(serde::Deserialize)]
    struct SystemRows {
        system: String,
        quantity: String,
        rows: Vec<Threshold>,
        #[allow(dead_code)]
        reason: String,
    }
    #[derive(serde::Deserialize)]
    struct RegionRows {
        region: String,
        quantity: String,
        rows: Vec<Threshold>,
        #[allow(dead_code)]
        reason: String,
    }
    #[derive(serde::Deserialize)]
    struct Threshold {
        unit: String,
        at_least: f64,
        /// Largest denominator the amount may be written with in this unit;
        /// absent, it is written as a decimal.
        #[serde(default)]
        fractions: Option<u8>,
        /// When no fraction fits, write a decimal here instead of trying the
        /// next row.
        #[serde(default)]
        or_decimal: bool,
    }

    // `metric tablespoon` -> `MetricTablespoon`: every identifier is the variant name as lowercase words (ADR 2)
    fn variant(id: &str) -> String {
        id.split(' ')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect()
    }
    fn quantity(name: &str) -> (&'static str, &'static str) {
        match name {
            "volume" => ("Volume", "VolumeUnit"),
            "mass" => ("Mass", "MassUnit"),
            "length" => ("Length", "LengthUnit"),
            "time" => ("Time", "TimeUnit"),
            "temperature" => ("Temperature", "TemperatureUnit"),
            other => panic!("locales/preferences: unknown quantity '{other}'"),
        }
    }
    // `Number::MAX_FRACTION_DENOMINATOR`, which a build script cannot import: `Number::new_approx` asserts it, so a
    // larger row would panic on the first conversion that reaches it instead of failing here
    const MAX_FRACTION_DENOMINATOR: u8 = 64;
    fn rows(quantity_name: &str, rows: &[Threshold]) -> String {
        let (q, enum_name) = quantity(quantity_name);
        rows.iter()
            .map(|t| {
                if let Some(den) = t.fractions {
                    assert!(
                        (1..=MAX_FRACTION_DENOMINATOR).contains(&den),
                        "locales/preferences: `{}` has fractions = {den}, but the largest \
                         denominator a fraction is written with is {MAX_FRACTION_DENOMINATOR}",
                        t.unit
                    );
                }
                format!(
                    "Row {{ unit: Unit::{q}({enum_name}::{}), at_least: {:?}, fractions: {:?}, or_decimal: {} }}",
                    variant(&t.unit),
                    t.at_least,
                    t.fractions,
                    t.or_decimal
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    // the rows are split by what they key on: systems and the frozen legacy rows in one file,
    // regions in the other. Registered by name, since the directory watch is what the locale files
    // already do not rely on.
    let mut prefs = Preferences::default();
    for (name, file) in [
        ("system", "locales/preferences/system.toml"),
        ("region", "locales/preferences/region.toml"),
    ] {
        println!("cargo::rerun-if-changed={file}");
        let text = std::fs::read_to_string(file).unwrap_or_else(|e| panic!("{file}: {e}"));
        let read: Preferences =
            toml::from_str(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
        match name {
            "system" => {
                assert!(read.region.is_empty(), "{file}: region rows belong in region.toml");
                prefs.system = read.system;
                prefs.legacy = read.legacy;
            }
            _ => {
                assert!(
                    read.system.is_empty() && read.legacy.is_empty(),
                    "{file}: system rows belong in system.toml"
                );
                prefs.region = read.region;
            }
        }
    }

    fn system_table(name: &str, entries: &[SystemRows]) -> String {
        let mut out =
            format!("pub(crate) static {name}: &[(System, PhysicalQuantity, &[Row])] = &[\n");
        for s in entries {
            let system = match s.system.as_str() {
                "metric" => "Metric",
                "us-customary" => "UsCustomary",
                "imperial" => "Imperial",
                other => panic!("locales/preferences: unknown system '{other}'"),
            };
            let (q, _) = quantity(&s.quantity);
            out.push_str(&format!(
                "    (System::{system}, PhysicalQuantity::{q}, &[{}]),\n",
                rows(&s.quantity, &s.rows)
            ));
        }
        out.push_str("];\n");
        out
    }

    let mut out = system_table("PREFERENCES", &prefs.system);
    out.push_str(&system_table("LEGACY_PREFERENCES", &prefs.legacy));
    out.push_str(
        "pub(crate) static REGION_PREFERENCES: &[(Region, PhysicalQuantity, &[Row])] = &[\n",
    );
    for r in &prefs.region {
        let (q, _) = quantity(&r.quantity);
        out.push_str(&format!(
            "    (region!({:?}), PhysicalQuantity::{q}, &[{}]),\n",
            r.region,
            rows(&r.quantity, &r.rows)
        ));
    }
    out.push_str("];\n");

    let dest = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("preferences.rs");
    std::fs::write(dest, out).unwrap();
}
