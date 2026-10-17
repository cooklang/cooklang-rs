//! The closed set of units, one enum per physical quantity.
//!
//! Factors are the exact legal definitions, relative to the quantity's
//! reference unit (liter, gram, meter, second). A factor that rests on
//! convention rather than statute cites its row in
//! `docs/research/cooking-units.md`.
//!
//! Identifiers are the variant names as lowercase words, so renaming a
//! variant changes the identifier a locale binds to. A multi-word identifier
//! needs an explicit `serialize`, because no case style writes lowercase words
//! separated by spaces.

/// Converts between two units of a linear quantity, given their factors to
/// the reference unit.
fn linear(value: f64, from: f64, to: f64) -> f64 {
    // `from / to` first so converting a unit to itself is exact (`f / f == 1`).
    value * (from / to)
}

/// A unit of volume. The reference unit is the liter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::VariantArray, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum VolumeUnit {
    Kiloliter,
    Hectoliter,
    Decaliter,
    Liter,
    Deciliter,
    Centiliter,
    Milliliter,
    #[strum(serialize = "us teaspoon")]
    UsTeaspoon,
    #[strum(serialize = "us tablespoon")]
    UsTablespoon,
    #[strum(serialize = "us fluid ounce")]
    UsFluidOunce,
    #[strum(serialize = "us cup")]
    UsCup,
    #[strum(serialize = "us pint")]
    UsPint,
    #[strum(serialize = "us quart")]
    UsQuart,
    #[strum(serialize = "us gallon")]
    UsGallon,
    #[strum(serialize = "imperial fluid ounce")]
    ImperialFluidOunce,
    #[strum(serialize = "imperial pint")]
    ImperialPint,
    #[strum(serialize = "imperial quart")]
    ImperialQuart,
    #[strum(serialize = "imperial gallon")]
    ImperialGallon,
    #[strum(serialize = "metric teaspoon")]
    MetricTeaspoon,
    #[strum(serialize = "metric dessert spoon")]
    MetricDessertSpoon,
    #[strum(serialize = "metric tablespoon")]
    MetricTablespoon,
    #[strum(serialize = "metric cup")]
    MetricCup,
    #[strum(serialize = "nordic spice measure")]
    NordicSpiceMeasure,
    #[strum(serialize = "danish knife point")]
    DanishKnifePoint,
    #[strum(serialize = "australian tablespoon")]
    AustralianTablespoon,
    #[strum(serialize = "japanese cup")]
    JapaneseCup,
    #[strum(serialize = "us legal cup")]
    UsLegalCup,
}

impl VolumeUnit {
    pub fn id(self) -> &'static str {
        self.into()
    }

    fn factor(self) -> f64 {
        match self {
            Self::Kiloliter => 1e3,
            Self::Hectoliter => 1e2,
            Self::Decaliter => 1e1,
            Self::Liter => 1.0,
            Self::Deciliter => 1e-1,
            Self::Centiliter => 1e-2,
            Self::Milliliter => 1e-3,
            // US customary, all defined from the gallon (231 in³ = 3.785411784 l).
            Self::UsTeaspoon => 0.004_928_921_593_75,
            Self::UsTablespoon => 0.014_786_764_781_25,
            Self::UsFluidOunce => 0.029_573_529_562_5,
            Self::UsCup => 0.236_588_236_5,
            Self::UsPint => 0.473_176_473,
            Self::UsQuart => 0.946_352_946,
            Self::UsGallon => 3.785_411_784,
            // Imperial, all defined from the gallon (4.54609 l exactly,
            // Weights and Measures Act 1985 Sch. 1): 160 fl oz, 8 pt, 4 qt.
            Self::ImperialFluidOunce => 0.028_413_062_5,
            Self::ImperialPint => 0.568_261_25,
            Self::ImperialQuart => 1.136_522_5,
            Self::ImperialGallon => 4.546_09,
            // Metric kitchen measures: conventional round values used nearly
            // everywhere outside the US (docs/research/cooking-units.md).
            Self::MetricTeaspoon => 0.005,
            Self::MetricDessertSpoon => 0.010,
            Self::MetricTablespoon => 0.015,
            Self::MetricCup => 0.25,
            // The Swedish kryddmått, Norwegian kryddermål and Finnish
            // maustemitta: 1 ml by convention in all three.
            Self::NordicSpiceMeasure => 0.001,
            // The Danish knivspids, a knife point, 0.25 ml by convention.
            // Denmark only: Sweden and Norway have no knife-point measure,
            // and the German Messerspitze has no defined amount at all.
            Self::DanishKnifePoint => 0.00025,
            // 20 ml by Australian standard, four teaspoons.
            Self::AustralianTablespoon => 0.020,
            // The 200 ml measuring cup set by 女子栄養大学 in 1948, which
            // Korean recipes also use. Not CLDR's cup-jp, which is 100 ml.
            Self::JapaneseCup => 0.200,
            // The 240 ml cup of US nutrition labels (21 CFR 101.9), and of
            // Brazilian and Mexican recipes.
            Self::UsLegalCup => 0.240,
        }
    }

    pub fn convert(value: f64, from: Self, to: Self) -> f64 {
        linear(value, from.factor(), to.factor())
    }
}

/// A unit of mass. The reference unit is the gram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::VariantArray, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum MassUnit {
    Kilogram,
    Hectogram,
    Decagram,
    Gram,
    Decigram,
    Centigram,
    Milligram,
    Ounce,
    Pound,
    #[strum(serialize = "colombian libra")]
    ColombianLibra,
}

impl MassUnit {
    pub fn id(self) -> &'static str {
        self.into()
    }

    fn factor(self) -> f64 {
        match self {
            Self::Kilogram => 1e3,
            Self::Hectogram => 1e2,
            Self::Decagram => 1e1,
            Self::Gram => 1.0,
            Self::Decigram => 1e-1,
            Self::Centigram => 1e-2,
            Self::Milligram => 1e-3,
            // Avoirdupois, defined by the international yard and pound agreement.
            Self::Ounce => 28.349_523_125,
            Self::Pound => 453.592_37,
            // Ley 33 de 1905, as quoted by the SIC regulator: 500 g exactly.
            // Not the avoirdupois pound, hence an identifier of its own.
            Self::ColombianLibra => 500.0,
        }
    }

    pub fn convert(value: f64, from: Self, to: Self) -> f64 {
        linear(value, from.factor(), to.factor())
    }
}

/// A unit of length. The reference unit is the meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::VariantArray, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum LengthUnit {
    Kilometer,
    Hectometer,
    Decameter,
    Meter,
    Decimeter,
    Centimeter,
    Millimeter,
    Foot,
    Inch,
}

impl LengthUnit {
    pub fn id(self) -> &'static str {
        self.into()
    }

    fn factor(self) -> f64 {
        match self {
            Self::Kilometer => 1e3,
            Self::Hectometer => 1e2,
            Self::Decameter => 1e1,
            Self::Meter => 1.0,
            Self::Decimeter => 1e-1,
            Self::Centimeter => 1e-2,
            Self::Millimeter => 1e-3,
            Self::Foot => 0.3048,
            Self::Inch => 0.0254,
        }
    }

    pub fn convert(value: f64, from: Self, to: Self) -> f64 {
        linear(value, from.factor(), to.factor())
    }
}

/// A unit of time. The reference unit is the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::VariantArray, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
}

impl TimeUnit {
    pub fn id(self) -> &'static str {
        self.into()
    }

    fn factor(self) -> f64 {
        match self {
            Self::Second => 1.0,
            Self::Minute => 60.0,
            Self::Hour => 3600.0,
            Self::Day => 86400.0,
        }
    }

    pub fn convert(value: f64, from: Self, to: Self) -> f64 {
        linear(value, from.factor(), to.factor())
    }
}

/// A unit of temperature. The reference unit is the degree Celsius.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::VariantArray, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

impl TemperatureUnit {
    pub fn id(self) -> &'static str {
        self.into()
    }

    /// Converts a temperature point, such as an oven setting.
    pub fn convert(value: f64, from: Self, to: Self) -> f64 {
        match (from, to) {
            (Self::Celsius, Self::Celsius) | (Self::Fahrenheit, Self::Fahrenheit) => value,
            (Self::Celsius, Self::Fahrenheit) => (value * 9.0 / 5.0) + 32.0,
            (Self::Fahrenheit, Self::Celsius) => (value - 32.0) * (5.0 / 9.0),
        }
    }
}
