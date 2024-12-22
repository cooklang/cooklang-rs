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

    let default_system = {
        if let Some(sys) = uf.get("default_system") {
            let sys = quote_enum!(System::sys.as_str().unwrap());
            quote! { Some(#sys) }
        } else {
            quote! { None }
        }
    };

    let si = {
        // ! IMPORTANT: Same order as in SIPrefix enum
        const SIPREFIX: [&str; 6] = ["kilo", "hecto", "deca", "deci", "centi", "milli"];
        if let Some(si) = uf.get("si") {
            let prefixes = {
                if let Some(pre) = si.get("prefixes") {
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
                } else {
                    quote!(None)
                }
            };

            let symbol_prefixes = {
                if let Some(pre) = si.get("symbol_prefixes") {
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
                } else {
                    quote!(None)
                }
            };

            let precedence = {
                if let Some(p) = si.get("precedence") {
                    quote_enum!(Precedence::p.as_str().unwrap())
                } else {
                    quote!(Precedence::default())
                }
            };

            quote! { Some(SI {
                prefixes: #prefixes,
                symbol_prefixes: #symbol_prefixes,
                precedence: #precedence,
            }) }
        } else {
            quote! { None }
        }
    };

    let fractions = {
        if let Some(frac) = uf.get("fractions") {
            macro_rules! quote_fractions_config_wrapper {
                ($v:expr) => {{
                    let v = $v;
                    if let Some(b) = v.as_bool() {
                        quote! { FractionsConfigWrapper::Toggle(#b) }
                    } else if let Some(v) = v.as_table() {
                        let enabled = if let Some(v) = v.get("enabled") {
                            let b = v.as_bool().unwrap();
                            quote! { Some(#b) }
                        } else {
                            quote! { None }
                        };
                        let accuracy = if let Some(v) = v.get("accuracy") {
                            let f = v.as_float().unwrap();
                            quote! { Some(#f as f32) }
                        } else {
                            quote! { None }
                        };
                        let max_denominator = if let Some(v) = v.get("max_denominator") {
                            let i = v.as_integer().unwrap();
                            quote! { Some(#i as u8) }
                        } else {
                            quote! { None }
                        };
                        let max_whole = if let Some(v) = v.get("max_whole") {
                            let i = v.as_integer().unwrap();
                            quote! { Some(#i as u32) }
                        } else {
                            quote! { None }
                        };

                        quote! { FractionsConfigWrapper::Custom(FractionsConfigHelper {
                            enabled: #enabled,
                            accuracy: #accuracy,
                            max_denominator: #max_denominator,
                            max_whole: #max_whole,
                        }) }
                    } else {
                        panic!("bad fractions value")
                    }
                }};
            }

            let all = if let Some(v) = frac.get("all") {
                let t = quote_fractions_config_wrapper!(v);
                quote! {Some(#t)}
            } else {
                quote! {None}
            };
            let metric = if let Some(v) = frac.get("metric") {
                let t = quote_fractions_config_wrapper!(v);
                quote! {Some(#t)}
            } else {
                quote! {None}
            };
            let imperial = if let Some(v) = frac.get("imperial") {
                let t = quote_fractions_config_wrapper!(v);
                quote! {Some(#t)}
            } else {
                quote! {None}
            };

            let quantity = {
                let mut n = 0;
                let entries = if let Some(t) = frac.get("quantity") {
                    let t = t.as_table().unwrap();
                    n = t.len();
                    let entries = t.iter().map(|(k, v)| {
                        let q = quote_enum!(PhysicalQuantity::k);
                        let val = quote_fractions_config_wrapper!(v);
                        quote! { m.insert(#q, #val); }
                    });
                    quote! { #(#entries)* }
                } else {
                    quote! {}
                };
                quote! { {
                    let mut m = HashMap::with_capacity(#n);
                    #entries
                    m
                } }
            };
            let unit = {
                let mut n = 0;
                let entries = if let Some(t) = frac.get("unit") {
                    let t = t.as_table().unwrap();
                    n = t.len();
                    let entries = t.iter().map(|(k, v)| {
                        let val = quote_fractions_config_wrapper!(v);
                        quote! { m.insert(#k.to_string(), #val); }
                    });
                    quote! { #(#entries)* }
                } else {
                    quote! {}
                };
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
        } else {
            quote! { None }
        }
    };

    let extend = if let Some(_) = uf.get("extend") {
        unimplemented!("base units.toml does not have extend");
    } else {
        quote! { None }
    };

    let quantity = if let Some(v) = uf.get("quantity") {
        let v = v.as_array().unwrap();
        let entries = v.iter().map(|qg| {
            let qg = qg.as_table().unwrap();

            let q = qg.get("quantity").unwrap().as_str().unwrap();
            let quantity = quote_enum!(PhysicalQuantity::q);

            let best = if let Some(b) = qg.get("best") {
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
            } else {
                quote!(None)
            };

            let units = if let Some(un) = qg.get("units") {
                macro_rules! quote_unit_entry {
                    ($v:expr) => {{
                        let v = $v;
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
                            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
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
                    }};
                }

                macro_rules! quote_unit_entry_vec {
                    ($v:expr) => {{
                        let vals = $v.iter().map(|v| {
                            let v = v.as_table().unwrap();
                            quote_unit_entry!(v)
                        });
                        quote! {vec![#(#vals),*]}
                    }};
                }

                let variant = if let Some(v) = un.as_array() {
                    let v = quote_unit_entry_vec!(v);
                    quote! { Unified(#v) }
                } else if let Some(t) = un.as_table() {
                    let metric = if let Some(v) = t.get("metric") {
                        quote_unit_entry_vec!(v.as_array().unwrap())
                    } else {
                        quote! {vec![]}
                    };
                    let imperial = if let Some(v) = t.get("imperial") {
                        quote_unit_entry_vec!(v.as_array().unwrap())
                    } else {
                        quote! {vec![]}
                    };
                    let unspecified = if let Some(v) = t.get("unspecified") {
                        quote_unit_entry_vec!(v.as_array().unwrap())
                    } else {
                        quote! {vec![]}
                    };
                    quote! { BySystem {
                        metric: #metric,
                        imperial: #imperial,
                        unspecified: #unspecified,
                    } }
                } else {
                    panic!("Bad quantity units")
                };
                quote! { Some(Units::#variant) }
            } else {
                quote! {None}
            };

            quote! {
                QuantityGroup {
                    quantity: #quantity,
                    best: #best,
                    units: #units,
                }
            }
        });
        quote! { vec![#(#entries),*] }
    } else {
        quote! {vec![]}
    };

    let tokens = quote! {
        fn get_bundled() -> UnitsFile {
            UnitsFile {
                default_system: #default_system,
                si: #si,
                fractions: #fractions,
                extend: #extend,
                quantity: #quantity,
            }
        }
    };

    let synfile = syn::parse2(tokens).unwrap();
    let generated = prettyplease::unparse(&synfile);
    // just one more hack won't hurt anyone
    let func_body = {
        let start = generated.find('{').unwrap();
        let end = generated.chars().rev().position(|c| c == '}').unwrap();
        &generated[start + 1..generated.len() - end - 1]
    };
    let outpath = format!("{}/bundled_units.rs", std::env::var("OUT_DIR").unwrap());
    std::fs::write(outpath, func_body).unwrap();
}
