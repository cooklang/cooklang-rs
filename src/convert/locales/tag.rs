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
    /// Precedence, most specific first: `-u-ms-` names a measurement system
    /// outright, `-u-rg-` overrides the region, then the tag's own region.
    /// A region gets its official system (ADR 0005), so `en-GB` is metric and
    /// only `en-GB-u-ms-uksystem` is imperial. `-u-mu-` then names a
    /// temperature unit outright, for temperature only; it never supplies a
    /// region. A value we have no rows for is an error, not a guess.
    pub(crate) fn audience(&self) -> Result<Audience, RegionError> {
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
    use icu_locale_core::subtags::region;

    fn audience_of(tag: &str) -> Result<Audience, RegionError> {
        LocaleTag::parse(tag).unwrap().audience()
    }

    #[test]
    fn the_tags_own_region_is_the_audience() {
        assert_eq!(
            audience_of("es-MX").unwrap(),
            Audience::region(region!("MX"))
        );
    }

    #[test]
    fn a_tag_without_a_region_fails_only_when_the_region_is_needed() {
        assert!(LocaleTag::parse("es").is_ok(), "parsing must succeed");
        assert_eq!(
            audience_of("es"),
            Err(RegionError::Missing("es".to_string())),
            "but choosing units must not"
        );
    }

    #[test]
    fn measurement_system_selects_a_system_not_a_region() {
        assert_eq!(
            audience_of("en-US-u-ms-metric").unwrap(),
            Audience::system(System::Metric)
        );
        assert_eq!(
            audience_of("en-GB-u-ms-ussystem").unwrap(),
            Audience::system(System::UsCustomary)
        );
        // the UK is officially metric, so this is the only way to ask for
        // imperial output, and it must not collapse to the GB region
        assert_eq!(
            audience_of("en-GB-u-ms-uksystem").unwrap(),
            Audience::system(System::Imperial)
        );
        assert_eq!(
            audience_of("es-MX-u-ms-uksystem").unwrap(),
            Audience::system(System::Imperial)
        );
        assert_eq!(
            audience_of("en-u-ms-metric").unwrap(),
            Audience::system(System::Metric)
        );
    }

    #[test]
    fn an_unsupported_measurement_system_is_an_error_not_a_guess() {
        assert_eq!(
            audience_of("en-US-u-ms-haa"),
            Err(RegionError::UnsupportedMeasurementSystem {
                tag: "en-US-u-ms-haa".to_string(),
                system: "haa".to_string(),
            })
        );
    }

    #[test]
    fn region_override_reads_the_subdivision() {
        assert_eq!(
            audience_of("en-GB-u-rg-uszzzz").unwrap(),
            Audience::region(region!("US"))
        );
        assert_eq!(
            audience_of("en-US-u-rg-gbzzzz").unwrap(),
            Audience::region(region!("GB"))
        );
        assert_eq!(
            audience_of("en-GB-u-rg-usca").unwrap(),
            Audience::region(region!("US"))
        );
    }

    #[test]
    fn measurement_system_wins_over_region_override() {
        assert_eq!(
            audience_of("en-GB-u-ms-metric-rg-uszzzz").unwrap(),
            Audience::system(System::Metric)
        );
    }

    #[test]
    fn a_temperature_unit_override_is_read_from_mu() {
        // the English-Canadian case: metric region, Fahrenheit ovens
        assert_eq!(
            audience_of("en-CA-u-mu-fahrenhe").unwrap(),
            Audience::region(region!("CA")).with_temperature(TemperatureUnit::Fahrenheit)
        );
        assert_eq!(
            audience_of("en-US-u-mu-celsius").unwrap(),
            Audience::region(region!("US")).with_temperature(TemperatureUnit::Celsius)
        );
        // composes with a system asked for outright
        assert_eq!(
            audience_of("en-GB-u-ms-metric-mu-fahrenhe").unwrap(),
            Audience::system(System::Metric).with_temperature(TemperatureUnit::Fahrenheit)
        );
        // and does not supply a region
        assert_eq!(
            audience_of("en-u-mu-celsius"),
            Err(RegionError::Missing("en-u-mu-celsius".to_string()))
        );
    }

    #[test]
    fn an_unsupported_temperature_unit_is_an_error_not_a_guess() {
        assert_eq!(
            audience_of("en-US-u-mu-kelvin"),
            Err(RegionError::UnsupportedTemperatureUnit {
                tag: "en-US-u-mu-kelvin".to_string(),
                unit: "kelvin".to_string(),
            })
        );
    }

    #[test]
    fn invalid_tags_are_rejected() {
        assert!(LocaleTag::parse("not a tag").is_err());
        assert!(LocaleTag::parse("").is_err());
    }

    #[test]
    fn the_language_identifier_drops_extensions() {
        // this is what locale data is keyed by; truncation would reach the
        // same place for these, but only by accident
        for (tag, id) in [
            ("en-GB-u-ms-metric", "en-GB"),
            ("es-MX-u-rg-uszzzz", "es-MX"),
            ("en-u-ms-uksystem", "en"),
            ("EN-gb", "en-GB"),
        ] {
            assert_eq!(LocaleTag::parse(tag).unwrap().language_identifier(), id);
        }
    }
}
