use std::{collections::HashMap, sync::Arc};

use enum_map::{enum_map, EnumMap};
use thiserror::Error;

use super::{
    convert_f64,
    units_file::{self, BestUnits, Extend, Precedence, SIPrefix, UnitEntry, Units, UnitsFile, SI},
    BestConversions, BestConversionsStore, Converter, Fractions, PhysicalQuantity, System, Unit,
    UnitIndex, UnknownUnit,
};

/// Builder to create a custom [`Converter`]
///
/// The builder uses [`UnitsFile`] to configure the converter. More than one
/// file can be layered. Order matters, as one file can extend the units of
/// another added before, or be overwritten by others after.
#[derive(Debug, Default)]
pub struct ConverterBuilder {
    all_units: Vec<UnitBuilder>,
    unit_index: UnitIndex,
    extend: Vec<Extend>,
    si: SI,
    fractions: Vec<units_file::Fractions>,
    best_units: EnumMap<PhysicalQuantity, Option<BestUnits>>,
    default_system: System,
}

#[derive(Debug)]
struct UnitBuilder {
    unit: Unit,
    is_expanded: bool,
    expand_si: bool,
    expanded_units: Option<EnumMap<SIPrefix, usize>>,
}

impl std::ops::Deref for UnitBuilder {
    type Target = Unit;

    fn deref(&self) -> &Self::Target {
        &self.unit
    }
}

impl std::ops::DerefMut for UnitBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.unit
    }
}

impl ConverterBuilder {
    /// New empty builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add the bundled units to the builder
    ///
    /// This is only available with the `bundled_units` feature.
    #[cfg(feature = "bundled_units")]
    pub fn with_bundled_units(mut self) -> Result<Self, ConverterBuilderError> {
        self.add_bundled_units()?;
        Ok(self)
    }

    /// Add the bundled units to the builder
    ///
    /// This is only available with the `bundled_units` feature.
    #[cfg(feature = "bundled_units")]
    pub fn add_bundled_units(&mut self) -> Result<&mut Self, ConverterBuilderError> {
        self.add_units_file(UnitsFile::bundled())?;
        Ok(self)
    }

    /// Add a [`UnitsFile`] to the builder
    pub fn with_units_file(mut self, units: UnitsFile) -> Result<Self, ConverterBuilderError> {
        self.add_units_file(units)?;
        Ok(self)
    }

    /// Add a [`UnitsFile`] to the builder
    pub fn add_units_file(&mut self, units: UnitsFile) -> Result<&mut Self, ConverterBuilderError> {
        for group in units.quantity {
            // Add all units to an index
            let mut add_units =
                |units: Vec<UnitEntry>, system| -> Result<(), ConverterBuilderError> {
                    for entry in units {
                        let unit = Unit {
                            names: entry.names,
                            symbols: entry.symbols,
                            aliases: entry.aliases,
                            ratio: entry.ratio,
                            difference: entry.difference,
                            physical_quantity: group.quantity,
                            system,
                        };
                        let _id = self.add_unit(UnitBuilder {
                            unit,
                            is_expanded: false,
                            expand_si: entry.expand_si,
                            expanded_units: None,
                        })?;
                    }
                    Ok(())
                };
            if let Some(units) = group.units {
                match units {
                    Units::Unified(units) => add_units(units, None)?,
                    Units::BySystem {
                        metric,
                        imperial,
                        unspecified,
                    } => {
                        add_units(metric, Some(System::Metric))?;
                        add_units(imperial, Some(System::Imperial))?;
                        add_units(unspecified, None)?;
                    }
                };
            }

            // store best units. this will always override
            if let Some(best_units) = group.best {
                if match &best_units {
                    BestUnits::Unified(v) => v.is_empty(),
                    BestUnits::BySystem { metric, imperial } => {
                        metric.is_empty() || imperial.is_empty()
                    }
                } {
                    return Err(ConverterBuilderError::EmptyBest {
                        reason: "empty list of units",
                        quantity: group.quantity,
                    });
                }
                self.best_units[group.quantity] = Some(best_units);
            }
        }

        // Store the extensions to apply them at the end
        if let Some(extend) = units.extend {
            self.extend.push(extend);
        }

        // Join the SI expansion settings
        if let Some(si) = units.si {
            self.si.prefixes = join_prefixes(&mut self.si.prefixes, si.prefixes, si.precedence);
            self.si.symbol_prefixes = join_prefixes(
                &mut self.si.symbol_prefixes,
                si.symbol_prefixes,
                si.precedence,
            );
            self.si.precedence = si.precedence;
        }

        if let Some(default_system) = units.default_system {
            self.default_system = default_system;
        }

        if let Some(fractions) = units.fractions {
            self.fractions.push(fractions);
        }

        Ok(self)
    }

    /// Consume the builder and return the new [`Converter`]
    pub fn finish(mut self) -> Result<Converter, ConverterBuilderError> {
        // expand the stored units
        for id in 0..self.all_units.len() {
            let unit = &self.all_units[id];
            if unit.expand_si {
                let new_units = expand_si(unit, &self.si)?;
                let mut new_units_ids = EnumMap::<SIPrefix, usize>::default();
                for (prefix, unit) in new_units.into_iter() {
                    new_units_ids[prefix] = self.add_unit(unit)?;
                }
                self.all_units[id].expanded_units = Some(new_units_ids);
            }
        }

        apply_extend_groups(
            self.extend,
            &mut self.all_units,
            &mut self.unit_index,
            &self.si,
        )?;

        let best = enum_map! {
            q =>  {
                if let Some(best_units) = &self.best_units[q] {
                    BestConversionsStore::new(best_units, &self.unit_index, &self.all_units)?
                } else {
                    return Err(ConverterBuilderError::EmptyBest { reason: "no best units given", quantity: q })
                }
            }
        };

        let quantity_index = {
            let mut index: EnumMap<PhysicalQuantity, Vec<usize>> = EnumMap::default();
            for (id, unit) in self.all_units.iter().enumerate() {
                index[unit.physical_quantity].push(id);
            }
            index
        };

        let fractions = build_fractions_config(&self.fractions, &self.unit_index, &self.all_units)?;

        Ok(Converter {
            all_units: self
                .all_units
                .into_iter()
                .map(|u| Arc::new(u.unit))
                .collect(),
            unit_index: self.unit_index,
            quantity_index,
            best,
            fractions,
            default_system: self.default_system,
        })
    }

    fn add_unit(&mut self, unit: UnitBuilder) -> Result<usize, ConverterBuilderError> {
        let id = self.all_units.len();
        self.unit_index.add_unit(&unit, id)?;
        self.all_units.push(unit);
        Ok(id)
    }
}

impl BestConversionsStore {
    fn new(
        best_units: &BestUnits,
        unit_index: &UnitIndex,
        all_units: &[UnitBuilder],
    ) -> Result<Self, ConverterBuilderError> {
        let v = match best_units {
            BestUnits::Unified(names) => {
                Self::Unified(BestConversions::new(names, unit_index, all_units)?)
            }
            BestUnits::BySystem { metric, imperial } => Self::BySystem {
                metric: BestConversions::new(metric, unit_index, all_units)?,
                imperial: BestConversions::new(imperial, unit_index, all_units)?,
            },
        };
        Ok(v)
    }
}

impl BestConversions {
    fn new(
        units: &[String],
        unit_index: &UnitIndex,
        all_units: &[UnitBuilder],
    ) -> Result<Self, ConverterBuilderError> {
        let mut units = units
            .iter()
            .map(|n| unit_index.get_unit_id(n))
            .collect::<Result<Vec<_>, _>>()?;

        units.sort_by(|a, b| {
            let a = &all_units[*a];
            let b = &all_units[*b];
            a.ratio
                .partial_cmp(&b.ratio)
                .unwrap_or(std::cmp::Ordering::Less)
        });

        let mut conversions = Vec::with_capacity(units.len());
        let mut units = units.into_iter();

        let base_unit = units.next().unwrap();
        conversions.push((1.0, base_unit));

        for unit in units {
            let v = convert_f64(1.0, &all_units[unit], &all_units[base_unit]);
            conversions.push((v, unit));
        }

        Ok(Self(conversions))
    }
}

fn apply_extend_groups(
    extend: Vec<Extend>,
    all_units: &mut [UnitBuilder],
    unit_index: &mut UnitIndex,
    si: &SI,
) -> Result<(), ConverterBuilderError> {
    for extend_group in extend {
        let Extend { precedence, units } = extend_group;

        let mut to_update = Vec::with_capacity(units.len());

        // First resolve keys with current config
        for (k, entry) in units {
            let id = unit_index.get_unit_id(k.as_str())?;
            if to_update.iter().any(|&(eid, _)| eid == id) {
                return Err(ConverterBuilderError::DuplicateExtendUnit { key: k });
            }
            if all_units[id].is_expanded
                && (entry.ratio.is_some()
                    || entry.difference.is_some()
                    || entry.names.is_some()
                    || entry.symbols.is_some())
            {
                return Err(ConverterBuilderError::InvalidExtendExpanded { key: k });
            }
            to_update.push((id, entry));
        }

        // Then apply updates
        for (id, entry) in to_update {
            // remove all entries from the unit and expansions from the index
            unit_index.remove_unit_rec(all_units, &all_units[id]);
            let unit = &mut all_units[id];

            // edit the unit
            if let Some(ratio) = entry.ratio {
                unit.ratio = ratio;
            }
            if let Some(difference) = entry.difference {
                unit.difference = difference;
            }
            if let Some(names) = entry.names {
                join_alias_vec(&mut unit.names, names, precedence);
            }
            if let Some(symbols) = entry.symbols {
                join_alias_vec(&mut unit.symbols, symbols, precedence);
            }
            if let Some(aliases) = entry.aliases {
                join_alias_vec(&mut unit.aliases, aliases, precedence);
            }

            // (re)add the new entries to the index
            if all_units[id].expand_si {
                update_expanded_units(id, all_units, unit_index, si)?;
            }
            unit_index.add_unit(&all_units[id], id)?;
        }
    }
    Ok(())
}

fn update_expanded_units(
    id: usize,
    all_units: &mut [UnitBuilder],
    unit_index: &mut UnitIndex,
    si: &SI,
) -> Result<(), ConverterBuilderError> {
    // update the expanded units
    let new_units = expand_si(&all_units[id], si)?;
    for (prefix, expanded_unit) in new_units.into_iter() {
        let expanded_id = all_units[id].expanded_units.as_ref().unwrap()[prefix];
        let old_unit_aliases = all_units[expanded_id].aliases.clone();
        all_units[expanded_id] = expanded_unit;
        all_units[expanded_id].aliases = old_unit_aliases;
        unit_index.add_unit(&all_units[expanded_id], expanded_id)?;
    }
    Ok(())
}

fn build_fractions_config(
    fractions: &[units_file::Fractions],
    unit_index: &UnitIndex,
    all_units: &[UnitBuilder],
) -> Result<Fractions, ConverterBuilderError> {
    let mut all = None;

    for cfg in fractions.iter() {
        all = cfg.all.map(|c| c.get()).or(all);
    }

    let mut metric = None;
    let mut imperial = None;
    let mut quantity = HashMap::new();

    for cfg in fractions.iter() {
        metric = cfg.metric.map(|c| c.get()).or(metric);
        imperial = cfg.imperial.map(|c| c.get()).or(imperial);
        for (q, cfg) in &cfg.quantity {
            quantity.insert(*q, cfg.get());
        }
    }

    let mut unit = HashMap::new();
    for cfg in fractions.iter() {
        for (key, cfg) in &cfg.unit {
            let unit_id = unit_index.get_unit_id(key)?;
            let u = &all_units[unit_id];

            let inherit = [
                quantity.get(&u.physical_quantity),
                u.system.and_then(|s| match s {
                    System::Metric => metric.as_ref(),
                    System::Imperial => imperial.as_ref(),
                }),
                all.as_ref(),
            ]
            .into_iter()
            .flatten()
            .copied()
            .reduce(|acc, e| acc.merge(e));

            let mut cfg = cfg.get();
            if let Some(inherit) = inherit {
                cfg = cfg.merge(inherit)
            }
            unit.insert(unit_id, cfg.define());
        }
    }
    Ok(Fractions {
        all: all.map(|c| c.define()),
        metric: metric.map(|c| c.define()),
        imperial: imperial.map(|c| c.define()),
        quantity: quantity.into_iter().map(|(q, c)| (q, c.define())).collect(),
        unit,
    })
}

fn join_alias_vec(target: &mut Vec<Arc<str>>, mut src: Vec<Arc<str>>, src_precedence: Precedence) {
    match src_precedence {
        Precedence::Before => {
            src.append(target);
            *target = src;
        }
        Precedence::After => {
            target.append(&mut src);
        }
        Precedence::Override => {
            *target = src;
        }
    }
}

fn join_prefixes(
    a: &mut Option<EnumMap<SIPrefix, Vec<String>>>,
    b: Option<EnumMap<SIPrefix, Vec<String>>>,
    b_precedence: Precedence,
) -> Option<EnumMap<SIPrefix, Vec<String>>> {
    let a = a.take();
    match (a, b) {
        (None, None) => None,
        (None, Some(v)) | (Some(v), None) => Some(v),
        (Some(mut a), Some(mut b)) => match b_precedence {
            Precedence::Before => {
                a.into_iter().for_each(|(p, v)| b[p].extend(v));
                Some(b)
            }
            Precedence::After => {
                b.into_iter().for_each(|(p, v)| a[p].extend(v));
                Some(a)
            }
            Precedence::Override => Some(b),
        },
    }
}

fn expand_si(
    unit: &UnitBuilder,
    si: &SI,
) -> Result<EnumMap<SIPrefix, UnitBuilder>, ConverterBuilderError> {
    assert!(unit.expand_si);
    let (Some(prefixes), Some(symbol_prefixes)) = (&si.prefixes, &si.symbol_prefixes) else {
        return Err(ConverterBuilderError::EmptySIPrefixes);
    };

    let map = enum_map! {
        prefix => {
            let names = prefixes[prefix]
                .iter()
                .flat_map(|p| unit.names.iter().map(move |n| format!("{p}{n}").into()))
                .collect();

            let symbols = symbol_prefixes[prefix]
                .iter()
                .flat_map(|p| unit.symbols.iter().map(move |n| format!("{p}{n}").into()))
                .collect();

            UnitBuilder {
                unit:

            Unit {
                names,
                symbols,
                aliases: Vec::new(),
                ratio: unit.ratio * prefix.ratio(),
                difference: unit.difference,
                physical_quantity: unit.physical_quantity,
                system: unit.system,
            },                expand_si: false,
            expanded_units: None,
            is_expanded: true
        }
        }
    };

    Ok(map)
}

impl UnitIndex {
    fn remove_unit(&mut self, unit: &Unit) {
        for key in unit.all_keys() {
            self.0.remove(key);
        }
    }

    fn remove_unit_rec(&mut self, all_units: &[UnitBuilder], unit: &UnitBuilder) {
        if let Some(expanded_units) = &unit.expanded_units {
            for (_, expanded) in expanded_units {
                self.remove_unit_rec(all_units, &all_units[*expanded]);
            }
        }
        self.remove_unit(unit);
    }

    fn add_unit(&mut self, unit: &Unit, id: usize) -> Result<usize, ConverterBuilderError> {
        let mut added = 0;
        for key in unit.all_keys() {
            if key.trim().is_empty() {
                return Err(ConverterBuilderError::EmptyUnitKey {
                    unit: unit.clone().into(),
                });
            }
            let maybe_other = self.0.insert(Arc::clone(key), id);
            if maybe_other.is_some() {
                return Err(ConverterBuilderError::DuplicateUnit {
                    name: key.to_string(),
                });
            }
            added += 1;
        }
        if added == 0 {
            return Err(ConverterBuilderError::EmptyUnit {
                unit: unit.clone().into(),
            });
        }
        Ok(added)
    }
}

/// Errors generated by [`ConverterBuilder`]
#[derive(Debug, Error)]
pub enum ConverterBuilderError {
    #[error("Duplicate unit: {name}")]
    DuplicateUnit { name: String },

    #[error("Duplicate unit in extend, another key points to the same unit: {key}")]
    DuplicateExtendUnit { key: String },

    #[error("Can only edit aliases in auto expanded unit: {key}")]
    InvalidExtendExpanded { key: String },

    #[error(transparent)]
    UnknownUnit(#[from] UnknownUnit),

    #[error("Unit without names or symbols in {}", unit.physical_quantity)]
    EmptyUnit { unit: Box<Unit> },

    #[error("Unit where a name, symbol or alias is empty in {}: {}", unit.physical_quantity, unit.names.first().or(unit.symbols.first()).or(unit.aliases.first()).map(|s| s.to_string()).unwrap_or_else(|| "-".to_string()))]
    EmptyUnitKey { unit: Box<Unit> },

    #[error("Best units for '{quantity}' empty: {reason}")]
    EmptyBest {
        reason: &'static str,
        quantity: PhysicalQuantity,
    },

    #[error("No SI prefixes found when expandind SI on a unit")]
    EmptySIPrefixes,
}
