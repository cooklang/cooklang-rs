//! Which locale a caller is asking for.
//!
//! A [`LocaleTag`] is a BCP 47 tag such as `es-MX`. Its *language* selects how
//! units are written; its *region* selects which units are preferred. The two
//! are independent: `es` is enough to name a tablespoon `cucharada`, but not
//! enough to decide between a 240 ml and a 250 ml cup. So a tag with no region
//! parses happily and only fails when a region is actually needed.

use icu_locale_core::extensions::unicode::key;
use icu_locale_core::subtags::Region;
use icu_locale_core::Locale;

use super::preference::{Audience, System, Units};
use crate::convert::units::TemperatureUnit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleTag(Locale);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("'{0}' is not a valid locale tag")]
pub struct InvalidTag(String);

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum RegionError {
    #[error(
        "locale '{0}' has no region, so there is no way to choose units for its readers: add \
         the region the recipe is written for, as `sv-SE` does, or ask for a measurement \
         system outright (`{0}-u-ms-metric`, `{0}-u-ms-ussystem`, `{0}-u-ms-uksystem`)"
    )]
    Missing(String),
    #[error("locale '{tag}' asks for measurement system '{system}', which is not supported")]
    UnsupportedMeasurementSystem { tag: String, system: String },
    #[error("locale '{tag}' asks for temperature unit '{unit}', which is not supported")]
    UnsupportedTemperatureUnit { tag: String, unit: String },
}

impl LocaleTag {
    pub fn parse(text: &str) -> Result<Self, InvalidTag> {
        Locale::try_from_str(text)
            .map(LocaleTag)
            .map_err(|_| InvalidTag(text.to_string()))
    }

    /// The language-script-region part, without extensions. This is what
    /// CLDR keys its locale data by.
    pub(crate) fn language_identifier(&self) -> String {
        self.0.id.to_string()
    }

    /// Who converted values are written for.
    ///
    /// Precedence, most specific first: `-u-ms-` names a measurement system outright, `-u-rg-` overrides the region,
    /// then the tag's own region, so `en-GB` is metric and only `en-GB-u-ms-uksystem` is imperial (ADR 5). `-u-mu-`
    /// then names a temperature unit outright, for temperature only; it never supplies a region. A value we have no
    /// rows for is an error, not a guess.
    pub fn audience(&self) -> Result<Audience, RegionError> {
        let units = self.units()?;
        let temperature = match self.0.extensions.unicode.keywords.get(&key!("mu")) {
            None => None,
            Some(value) => match value.to_string().as_str() {
                "celsius" => Some(TemperatureUnit::Celsius),
                "fahrenhe" => Some(TemperatureUnit::Fahrenheit),
                other => {
                    return Err(RegionError::UnsupportedTemperatureUnit {
                        tag: self.to_string(),
                        unit: other.to_string(),
                    })
                }
            },
        };
        Ok(Audience { units, temperature })
    }

    fn units(&self) -> Result<Units, RegionError> {
        if let Some(value) = self.0.extensions.unicode.keywords.get(&key!("ms")) {
            let system = value.to_string();
            return match system.as_str() {
                "metric" => Ok(Units::System(System::Metric)),
                "ussystem" => Ok(Units::System(System::UsCustomary)),
                "uksystem" => Ok(Units::System(System::Imperial)),
                _ => Err(RegionError::UnsupportedMeasurementSystem {
                    tag: self.to_string(),
                    system,
                }),
            };
        }
        if let Some(value) = self.0.extensions.unicode.keywords.get(&key!("rg")) {
            // an `rg` value is a subdivision id: a region followed by a
            // subdivision, as in `uszzzz` for the whole of the US
            if let Some(region) = region_of_subdivision(&value.to_string()) {
                return Ok(Units::Region(region));
            }
        }
        self.0
            .id
            .region
            .map(Units::Region)
            .ok_or_else(|| RegionError::Missing(self.to_string()))
    }
}

impl std::fmt::Display for LocaleTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn region_of_subdivision(id: &str) -> Option<Region> {
    // a region subtag is two letters or three digits (UTS #35, `unicode_region_subtag`)
    let len = if id.starts_with(|c: char| c.is_ascii_digit()) {
        3
    } else {
        2
    };
    let region = id.get(..len)?.to_ascii_uppercase();
    Region::try_from_str(&region).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use icu_locale_core::subtags::Region;
    use TemperatureUnit::{Celsius, Fahrenheit};

    const METRIC: Audience = Audience::system(System::Metric);
    const US: Audience = Audience::system(System::UsCustomary);
    const IMPERIAL: Audience = Audience::system(System::Imperial);

    fn region(code: &str) -> Audience {
        Audience::region(Region::try_from_str(code).unwrap())
    }

    #[track_caller]
    fn assert_audience(tag: &str, expected: Audience) {
        assert_eq!(
            LocaleTag::parse(tag).unwrap().audience(),
            Ok(expected),
            "{tag}"
        );
    }

    #[track_caller]
    fn assert_no_audience(tag: &str, expected: RegionError) {
        assert_eq!(
            LocaleTag::parse(tag).unwrap().audience(),
            Err(expected),
            "{tag}"
        );
    }

    #[test]
    fn the_tags_own_region_is_the_audience() {
        assert_audience("es-MX", region("MX"));
        assert_audience("en-GB", region("GB"));
    }

    #[test]
    fn a_tag_without_a_region_fails_only_when_the_region_is_needed() {
        assert!(LocaleTag::parse("es").is_ok(), "parsing must succeed");
        assert_no_audience("es", RegionError::Missing("es".to_string()));
    }

    #[test]
    fn measurement_system_selects_a_system_not_a_region() {
        assert_audience("en-US-u-ms-metric", METRIC);
        assert_audience("en-GB-u-ms-ussystem", US);
        // the UK is officially metric, so this is the only way to ask for imperial output, and it must not collapse
        // to the GB region
        assert_audience("en-GB-u-ms-uksystem", IMPERIAL);
        assert_audience("es-MX-u-ms-uksystem", IMPERIAL);
        assert_audience("en-u-ms-metric", METRIC);
    }

    #[test]
    fn region_override_reads_the_subdivision() {
        assert_audience("en-GB-u-rg-uszzzz", region("US"));
        assert_audience("en-US-u-rg-gbzzzz", region("GB"));
        assert_audience("en-GB-u-rg-usca", region("US"));
    }

    #[test]
    fn measurement_system_wins_over_region_override() {
        assert_audience("en-GB-u-ms-metric-rg-uszzzz", METRIC);
    }

    #[test]
    fn a_temperature_unit_override_is_read_from_mu() {
        // the English-Canadian case: metric region, Fahrenheit ovens
        assert_audience(
            "en-CA-u-mu-fahrenhe",
            region("CA").with_temperature(Fahrenheit),
        );
        assert_audience("en-US-u-mu-celsius", region("US").with_temperature(Celsius));
        // composes with a system asked for outright
        assert_audience(
            "en-GB-u-ms-metric-mu-fahrenhe",
            METRIC.with_temperature(Fahrenheit),
        );
        // and does not supply a region
        assert_no_audience(
            "en-u-mu-celsius",
            RegionError::Missing("en-u-mu-celsius".to_string()),
        );
    }

    #[test]
    fn an_unsupported_value_is_an_error_not_a_guess() {
        assert_no_audience(
            "en-US-u-ms-haa",
            RegionError::UnsupportedMeasurementSystem {
                tag: "en-US-u-ms-haa".to_string(),
                system: "haa".to_string(),
            },
        );
        assert_no_audience(
            "en-US-u-mu-kelvin",
            RegionError::UnsupportedTemperatureUnit {
                tag: "en-US-u-mu-kelvin".to_string(),
                unit: "kelvin".to_string(),
            },
        );
    }

    #[test]
    fn invalid_tags_are_rejected() {
        assert!(LocaleTag::parse("not a tag").is_err());
        assert!(LocaleTag::parse("").is_err());
    }

    #[track_caller]
    fn assert_identifier(tag: &str, expected: &str) {
        assert_eq!(
            LocaleTag::parse(tag).unwrap().language_identifier(),
            expected
        );
    }

    #[test]
    fn the_language_identifier_drops_extensions() {
        // this is what locale data is keyed by; truncation would reach the same place for these, but only by
        // accident
        assert_identifier("en-GB-u-ms-metric", "en-GB");
        assert_identifier("es-MX-u-rg-uszzzz", "es-MX");
        assert_identifier("en-u-ms-uksystem", "en");
        assert_identifier("EN-gb", "en-GB");
    }
}
