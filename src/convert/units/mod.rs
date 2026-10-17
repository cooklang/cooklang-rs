//! Pure unit conversion for the units used in cooking.
//!
//! This module only converts numbers between units. Names, aliases, translations, unit systems, best-unit selection and
//! fraction display belong to the layer above it, `convert::locales`. The set of units is closed: adding a unit is a
//! code change.
//!
//! Each unit has a stable [`Unit::id`] for that layer to bind to, so a locale
//! can decide that `cup` means [`VolumeUnit::UsCup`] or some other cup.

mod definitions;

#[cfg(test)]
mod tests;

use strum::VariantArray;

pub use definitions::{LengthUnit, MassUnit, TemperatureUnit, TimeUnit, VolumeUnit};

/// The kind of thing a unit measures. Only units of the same physical quantity
/// convert into each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalQuantity {
    Volume,
    Mass,
    Length,
    Time,
    Temperature,
}

impl From<PhysicalQuantity> for crate::convert_legacy::PhysicalQuantity {
    fn from(quantity: PhysicalQuantity) -> Self {
        match quantity {
            PhysicalQuantity::Volume => Self::Volume,
            PhysicalQuantity::Mass => Self::Mass,
            PhysicalQuantity::Length => Self::Length,
            PhysicalQuantity::Time => Self::Time,
            PhysicalQuantity::Temperature => Self::Temperature,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    Volume(VolumeUnit),
    Mass(MassUnit),
    Length(LengthUnit),
    Time(TimeUnit),
    Temperature(TemperatureUnit),
}

/// Tried to convert between units of different physical quantities.
#[derive(Debug, thiserror::Error)]
#[error("cannot convert {from:?} to {to:?}")]
pub struct MixedQuantities {
    pub(crate) from: PhysicalQuantity,
    pub(crate) to: PhysicalQuantity,
}

impl Unit {
    /// Every unit, grouped by physical quantity.
    pub fn all() -> impl Iterator<Item = Unit> {
        VolumeUnit::VARIANTS
            .iter()
            .copied()
            .map(Unit::Volume)
            .chain(MassUnit::VARIANTS.iter().copied().map(Unit::Mass))
            .chain(LengthUnit::VARIANTS.iter().copied().map(Unit::Length))
            .chain(TimeUnit::VARIANTS.iter().copied().map(Unit::Time))
            .chain(
                TemperatureUnit::VARIANTS
                    .iter()
                    .copied()
                    .map(Unit::Temperature),
            )
    }

    /// Stable identifier: lowercase words separated by spaces, with any regional or system variant first, as in `us
    /// cup` or `imperial pint`. Unique across all units.
    ///
    /// Recipe-facing: it resolves as input in every locale and is what a conversion writes when a locale has no symbol
    /// for the unit (ADR 4, ADR 2).
    pub fn id(self) -> &'static str {
        match self {
            Unit::Volume(u) => u.id(),
            Unit::Mass(u) => u.id(),
            Unit::Length(u) => u.id(),
            Unit::Time(u) => u.id(),
            Unit::Temperature(u) => u.id(),
        }
    }

    pub fn from_id(id: &str) -> Option<Unit> {
        Unit::all().find(|unit| unit.id() == id)
    }

    /// The smallest metric unit of a physical quantity, which every other
    /// one converts into for comparison: `ml`, `mg`, `mm`, `s`, `°C`.
    pub(crate) fn metric_base(quantity: PhysicalQuantity) -> Unit {
        match quantity {
            PhysicalQuantity::Volume => Unit::Volume(VolumeUnit::Milliliter),
            PhysicalQuantity::Mass => Unit::Mass(MassUnit::Milligram),
            PhysicalQuantity::Length => Unit::Length(LengthUnit::Millimeter),
            PhysicalQuantity::Time => Unit::Time(TimeUnit::Second),
            PhysicalQuantity::Temperature => Unit::Temperature(TemperatureUnit::Celsius),
        }
    }

    pub fn physical_quantity(self) -> PhysicalQuantity {
        match self {
            Unit::Volume(_) => PhysicalQuantity::Volume,
            Unit::Mass(_) => PhysicalQuantity::Mass,
            Unit::Length(_) => PhysicalQuantity::Length,
            Unit::Time(_) => PhysicalQuantity::Time,
            Unit::Temperature(_) => PhysicalQuantity::Temperature,
        }
    }

    /// Converts `value` from one unit to another of the same physical quantity.
    pub fn convert(value: f64, from: Unit, to: Unit) -> Result<f64, MixedQuantities> {
        match (from, to) {
            (Unit::Volume(from), Unit::Volume(to)) => Ok(VolumeUnit::convert(value, from, to)),
            (Unit::Mass(from), Unit::Mass(to)) => Ok(MassUnit::convert(value, from, to)),
            (Unit::Length(from), Unit::Length(to)) => Ok(LengthUnit::convert(value, from, to)),
            (Unit::Time(from), Unit::Time(to)) => Ok(TimeUnit::convert(value, from, to)),
            (Unit::Temperature(from), Unit::Temperature(to)) => {
                Ok(TemperatureUnit::convert(value, from, to))
            }
            _ => Err(MixedQuantities {
                from: from.physical_quantity(),
                to: to.physical_quantity(),
            }),
        }
    }
}
