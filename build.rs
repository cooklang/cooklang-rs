fn main() {
    #[cfg(feature = "bundled_units")]
    {
        println!("cargo::rerun-if-changed=units.toml");
        generate_bundled();
    }
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
