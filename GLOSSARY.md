# cooklang-rs

A parser for Cooklang recipes that also converts, scales and aggregates their quantities. The
terms below are the ones the unit-conversion work uses; the rest of the crate follows the
Cooklang specification's own vocabulary.

## Language

### Locales

**Locale**:
The set of unit names, symbols and spellings one group of readers uses, identified by a BCP 47
tag such as `sv-SE`.
_Avoid_: language, translation, units file

**Locale tag**:
The BCP 47 tag that identifies a locale and, through its region and `-u-` extensions, its
audience.
_Avoid_: `xx_YY`, language code

**Reviewed locale**:
A locale whose unit names a person has judged correct. Only reviewed locales may be named by a
recipe.
_Avoid_: supported locale, vendored locale

**Legacy locale**:
The frozen locale that names exactly the units the crate knew before locales existed. It is
what an untagged recipe reads its units through. It is data, and it stays.
_Avoid_: legacy (alone), default locale, bundled units

**Naming**:
Which locale a recipe reads its units through: the legacy locale when the recipe is untagged,
otherwise the reviewed locale its tag names.
_Avoid_: the recipe's converter, the recipe's language

### Readers

**Audience**:
The readers a quantity is written for: the unit system their region uses officially, or one
asked for outright, plus any temperature unit asked for outright.
_Avoid_: target system, output locale

**Preference**:
The audience's choice of unit and fraction for one physical quantity at one size, such as
writing a quarter cup rather than four tablespoons.
_Avoid_: best unit (alone), rounding rule

### Migration

**Old converter**:
The pre-locale conversion code, driven by a units file. It is being replaced and will be
removed; only its unit names survive, as the legacy locale.
_Avoid_: legacy converter, legacy (alone), v1
