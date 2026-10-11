# CLDR unit data: measurement systems, unit preferences and translation coverage

Compiled 2026-10-03.

Scope: three CLDR datasets relevant to seeding `convert_v2`'s locale names and to deciding which
units a region needs — the per-territory measurement system, the per-region unit preference lists
for the cooking-relevant categories (`volume`, `mass`, `temperature`, `length`), and the actual
per-locale translation coverage of the 30 volume/mass/temperature unit ids that matter for recipes.
Everything here is quoted from the sources below; nothing is paraphrased into a value. This file
does not re-derive the 30-of-38 coverage claim in
[`../outstanding.md`](../outstanding.md#seeding-locale-names-from-cldr), but section 5 records two
findings that contradict it.

## 1. Sources and versions

| dataset | URL | version pinned |
| --- | --- | --- |
| `measurementData` | `https://raw.githubusercontent.com/unicode-org/cldr/main/common/supplemental/supplementalData.xml` | `unicode-org/cldr` `main`, file last changed in commit `86be3214f0ba76a23765845ccc7b3c2e67247665` (2026-09-03) |
| `unitPreferenceData`, `convertUnits`, `unitConstants` | `https://raw.githubusercontent.com/unicode-org/cldr/main/common/supplemental/units.xml` | `unicode-org/cldr` `main`, file last changed in commit `fa4596ab2e7e1f82623fb7e3782d31db2b386fa5` (2026-08-08) |
| per-locale unit names | `https://raw.githubusercontent.com/unicode-org/cldr-json/main/cldr-json/cldr-units-full/main/<locale>/units.json` | `cldr-json` `main` @ `91c267402229a59e3ef2774544f001bf959e8809` (2026-09-30) |

Version numbers, as the repositories themselves state them:

- `unicode-org/cldr` `main` (HEAD `5b159148f1390735049e296723f6b847229b23ef`, 2026-10-02) is **CLDR 49 in
  development**: `common/dtd/ldmlSupplemental.dtd` declares
  `<!ATTLIST version cldrVersion CDATA #FIXED "49" >`. The `<version number="$Revision$"/>` element
  inside both XML files is an unexpanded placeholder and carries no version.
- `cldr-json/cldr-json/cldr-units-full/package.json` declares `"version": "48.2.3"`,
  `"cldrVersion": "48"`, `"unicodeVersion": "16.0.0"`, `"license": "Unicode-3.0"`.

**The two halves of this document are therefore from different CLDR releases** (49-dev for the
supplemental XML, 48.2.3 for the locale JSON). Nothing observed below depends on that, but a
re-pull should pin one release.

## 2. Measurement system by territory

The entire `<measurementData>` element, verbatim:

```xml
<measurementData>
      <!-- measurementSystem will be deprecated in next release; replaced by enhanced preferences -->
      <measurementSystem type="metric"  territories="001"/>
      <measurementSystem type="US"  territories="LR US"/>
      <measurementSystem type="metric" category="temperature" territories="LR MM"/>
      <measurementSystem type="US" category="temperature" territories="BS BZ KY PR PW"/>
      <measurementSystem type="UK"  territories="GB MM"/>
      <paperSize type="A4"  territories="001"/>
      <paperSize type="US-Letter"  territories="BZ CA CL CO CR GT MX NI PA PH PR SV US VE"/>
</measurementData>
```

Nine territories carry an explicit `measurementSystem`. These are the complete set of non-metric
exceptions:

| territory | general system | temperature override | effect |
| --- | --- | --- | --- |
| `US` | `US` | — (and listed again under `US`/`temperature`) | US customary throughout, Fahrenheit |
| `LR` | `US` | `metric` | US customary volumes and masses, **Celsius** |
| `GB` | `UK` | — | imperial throughout, Celsius (no Fahrenheit entry) |
| `MM` | `UK` | `metric` | imperial volumes and masses, **Celsius** |
| `BS` | — (inherits `001` metric) | `US` | metric units, **Fahrenheit** |
| `BZ` | — (inherits `001` metric) | `US` | metric units, **Fahrenheit** |
| `KY` | — (inherits `001` metric) | `US` | metric units, **Fahrenheit** |
| `PR` | — (inherits `001` metric) | `US` | metric units, **Fahrenheit** |
| `PW` | — (inherits `001` metric) | `US` | metric units, **Fahrenheit** |

The default is `<measurementSystem type="metric" territories="001"/>`: **every territory not listed
above is metric**, including `CA`, `AU`, `IN`, `IE`, `NZ` and `ZA`. `001` is the CLDR world region,
so this is a true fallback rather than an enumeration.

Caveat recorded in the data itself: the inline comment says `measurementSystem` "will be deprecated
in next release; replaced by enhanced preferences". The `type` values observed are exactly `metric`,
`US` and `UK`; the only `category` value observed is `temperature`.

## 3. Unit preferences

Every `<unitPreferences>` block in `unitPreferenceData` whose `category` is `volume`, `mass`,
`temperature` or `length`, in file order within each block. `unitPreferenceData` itself is marked
`draft="contributed"`.

**How resolution works.** For a (category, usage) pair, pick the rows whose `regions` contain the
target region, falling back to the rows for `001`; walk them in file order, which is descending
`geq`; take the first row whose `geq` the value (expressed in the category's base unit) is greater
than or equal to; if none matches, take the last row. Rows without `geq` therefore act as the
catch-all, and `skeleton` is a number-formatting pattern applied to the result, not a selection
criterion. Compound ids such as `foot-and-inch` mean a mixed-unit display. Only two `usageAlias`
entries exist in the whole file (`music-track` and `tv-program`, both to `media`), so no usage name
below is an alias of another.

**There is no cooking usage.** The complete list of `volume` usages is `default`, `fluid`, `oil`,
`vehicle`; there is no `cooking`, `recipe` or `food` usage in any of the four categories (`food`
exists only under `category="energy"`). `volume`/`fluid` is the closest fit.

### volume / default

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `cubic-meter` | — |
| `001` | — | `cubic-centimeter` | — |
| `US GB` | — | `cubic-foot` | — |
| `US GB` | — | `cubic-inch` | — |

### volume / fluid

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `liter` | — |
| `001` | — | `milliliter` | — |
| `US` | — | `gallon` | — |
| `US` | — | `quart` | — |
| `US` | — | `pint` | — |
| `US` | — | `cup` | — |
| `US` | — | `fluid-ounce` | — |
| `US` | — | `tablespoon` | — |
| `US` | — | `teaspoon` | — |
| `GB` | — | `gallon-imperial` | — |
| `GB` | — | `fluid-ounce-imperial` | — |

### volume / oil

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `barrel` | — |

### volume / vehicle

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `US` | — | `gallon` | — |
| `001` | — | `liter` | — |

### mass / default

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `tonne` | — |
| `001` | — | `kilogram` | — |
| `001` | — | `gram` | — |
| `001` | — | `milligram` | — |
| `001` | — | `microgram` | — |
| `US GB` | — | `ton` | — |
| `US GB` | — | `pound` | — |
| `US GB` | — | `ounce` | — |

### mass / person

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `kilogram` | — |
| `001` | — | `gram` | — |
| `US` | — | `pound` | — |
| `US` | — | `pound-and-ounce` | — |
| `GB` | — | `stone-and-pound` | — |
| `GB` | — | `pound-and-ounce` | — |
| `HK` | — | `pound-and-ounce` | — |

### temperature / default

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `celsius` | — |
| `US` | — | `fahrenheit` | — |

### temperature / weather

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `celsius` | — |
| `BS BZ KY PR PW US` | — | `fahrenheit` | — |

### length / default

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `kilometer` | — |
| `001` | — | `meter` | — |
| `001` | — | `centimeter` | — |
| `US GB` | — | `mile` | — |
| `US GB` | — | `foot` | — |
| `US GB` | — | `inch` | — |

### length / focal-length

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `millimeter` | — |

### length / person

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `centimeter` | — |
| `CA GB IN US` | — | `inch` | — |

### length / person-height

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `centimeter` | — |
| `CA GB IN US` | `3.0` | `foot-and-inch` | — |
| `CA GB IN US` | — | `inch` | — |
| `AT BE DZ EG ES FR HK ID IL IT JO MY SA SE TR VN` | — | `meter-and-centimeter` | — |

### length / rainfall

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `BR` | — | `centimeter` | — |
| `US` | — | `inch` | — |
| `001` | — | `millimeter` | — |

### length / road

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | `0.9` | `kilometer` | — |
| `001` | `300.0` | `meter` | `precision-increment/50` |
| `001` | `10` | `meter` | `precision-increment/10` |
| `001` | — | `meter` | `precision-increment/1` |
| `US` | `0.5` | `mile` | — |
| `US` | `100.0` | `foot` | `precision-increment/50` |
| `US` | `10` | `foot` | `precision-increment/10` |
| `US` | — | `foot` | `precision-increment/1` |
| `GB` | `0.5` | `mile` | — |
| `GB` | `100.0` | `yard` | `precision-increment/50` |
| `GB` | `10` | `yard` | `precision-increment/10` |
| `GB` | — | `yard` | `precision-increment/1` |
| `SE` | — | `mile-scandinavian` | — |
| `SE` | — | `kilometer` | — |
| `SE` | `300.0` | `meter` | `precision-increment/50` |
| `SE` | `10` | `meter` | `precision-increment/10` |
| `SE` | — | `meter` | `precision-increment/1` |

### length / snowfall

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | — | `centimeter` | — |
| `US` | — | `inch` | — |

### length / vehicle

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `GB US` | — | `foot-and-inch` | — |
| `001` | — | `meter` | — |

### length / visiblty

| regions | geq | unit | skeleton |
| --- | --- | --- | --- |
| `001` | `0.1` | `kilometer` | — |
| `001` | — | `meter` | — |
| `DE NL` | — | `meter` | — |
| `GB US` | — | `mile` | — |
| `GB US` | — | `foot` | — |

(The `usage` spelling `visiblty` is CLDR's, not a typo here. The `DE NL` row is unreachable under
the resolution rule above, because the `001` catch-all for `meter` precedes it and `DE`/`NL` have no
`geq` row of their own — recorded as observed.)

## 4. Translation coverage for the 30 recipe-relevant unit ids

### 4.1 Method, and whether absence means inheritance

**All 30 keys are present in all 38 locale files.** Across 39 files (the 38 locales plus `und`) x 30
keys = 1170 cells, zero were missing. That alone shows the `cldr-units-full` JSON is
**inheritance-resolved**, and it was confirmed directly against the XML the JSON is generated from:

- `common/main/ja.xml` contains no `unit type="volume-cup-imperial"` element at all (0 matches),
  yet `cldr-units-full/main/ja/units.json` has `volume-cup-imperial` with the value
  `{0} cup Imp.` — byte-identical to the value in `und/units.json`.
- Same for `common/main/de.xml`: no `volume-cup-imperial` and no `volume-pint-imperial`, while
  `de/units.json` carries `{0} cup Imp.` and `{0} pt Imp.`

So in these files a key is never absent, and a key you did not translate appears filled in from the
inheritance chain. Neither `cldr-json/README.md` nor `cldr-json/PACKAGES.md` states this in words
(a grep for "resolv", "inherit" and "fallback" found nothing on the subject), so the determination
above is empirical, from the XML-vs-JSON comparison.

The practical consequence: **you cannot detect untranslated units by looking for missing keys.** You
have to compare the value against the inherited one. Note also that CLDR's `und` long forms are
themselves abbreviations (`{0} c`, `{0} tbsp`, `{0}°C`), not English words, so a root-inherited long
pattern is visibly wrong as a long form.

Status codes used below, computed mechanically on the `long` / `unitPattern-count-other` string:

| code | meaning |
| --- | --- |
| `T` | locale's string differs from both `und` and `en` — a real translation (or a real regional English variant) |
| `**R**` | string is **byte-identical to `und`** — untranslated, inherited from root |
| `=en` | string is byte-identical to `en` but differs from `und` |
| `ref` | the `en` row itself, the comparison reference |
| `root` | the `und` row itself |

For a non-English locale, `=en` is an **English passthrough**: a real English word sitting in a
non-English locale. For `en-GB`/`en-AU`/`en-CA`/`en-IN`, `=en` simply means unchanged from `en`,
which is normal.

Locales covered: the 39 requested minus `pt-BR`, which **does not exist** in `cldr-units-full`
(HTTP 404; `pt` is the Brazilian-default locale in CLDR, so there is no `pt-BR` file). `root` also
404s; the root locale is published as `und`, which is included as a row. That is 38 locale files, 33
of them non-English.

Totals over the 1170 cells: 941 `T`, 123 `=en`, 106 `**R**`.

### 4.2 Variant volume units marked metric or imperial

| locale | cup-metric | cup-imperial | pint-metric | pint-imperial | fluid-ounce-metric | fluid-ounce-imperial | quart-imperial | gallon-imperial |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `en` | ref | ref | ref | ref | ref | ref | ref | ref |
| `en-GB` | =en | =en | =en | =en | =en | T | T | T |
| `en-AU` | =en | =en | =en | =en | =en | =en | T | T |
| `en-CA` | =en | =en | =en | =en | =en | T | T | T |
| `en-IN` | =en | =en | =en | =en | =en | T | T | T |
| `es` | T | **R** | T | **R** | T | T | T | T |
| `es-MX` | T | **R** | T | **R** | T | T | T | T |
| `es-US` | T | **R** | T | **R** | T | T | T | T |
| `fr` | T | **R** | T | **R** | T | T | T | T |
| `fr-CA` | T | **R** | T | **R** | T | T | T | T |
| `de` | T | **R** | T | **R** | T | T | T | T |
| `de-AT` | T | **R** | T | **R** | T | T | T | T |
| `it` | T | **R** | T | **R** | T | T | T | T |
| `pt` | T | **R** | T | **R** | T | T | T | T |
| `nl` | T | **R** | T | **R** | T | T | T | T |
| `ja` | T | **R** | T | **R** | T | T | T | T |
| `zh` | T | **R** | T | **R** | T | T | T | T |
| `zh-Hant` | T | **R** | T | **R** | T | T | T | T |
| `ko` | T | **R** | T | **R** | T | T | T | T |
| `ru` | T | **R** | T | **R** | T | T | T | T |
| `pl` | T | **R** | T | **R** | **R** | T | T | T |
| `sv` | T | **R** | T | **R** | T | =en | T | T |
| `da` | T | **R** | T | **R** | T | =en | T | T |
| `nb` | T | **R** | T | **R** | T | T | T | T |
| `fi` | T | **R** | T | **R** | T | T | T | T |
| `cs` | T | **R** | T | **R** | T | T | T | T |
| `hu` | T | **R** | T | **R** | T | T | T | T |
| `tr` | T | **R** | T | **R** | **R** | T | T | T |
| `ar` | T | **R** | T | **R** | T | T | T | T |
| `hi` | T | **R** | T | **R** | T | T | T | T |
| `th` | T | **R** | T | **R** | T | T | T | T |
| `vi` | T | **R** | T | **R** | T | T | T | T |
| `id` | T | **R** | T | **R** | T | T | T | T |
| `ms` | T | **R** | T | **R** | T | **R** | T | **R** |
| `he` | T | **R** | T | **R** | T | T | T | T |
| `el` | T | **R** | T | **R** | T | T | T | T |
| `uk` | T | **R** | T | **R** | T | T | T | T |
| `ro` | T | **R** | T | **R** | T | T | T | T |
| `und` | root | root | root | root | root | root | root | root |


### 4.3 Small-measure and cocktail units

| locale | dessert-spoon | dessert-spoon-imperial | drop | pinch | jigger | dram |
| --- | --- | --- | --- | --- | --- | --- |
| `en` | ref | ref | ref | ref | ref | ref |
| `en-GB` | T | T | =en | =en | =en | =en |
| `en-AU` | T | T | =en | =en | =en | =en |
| `en-CA` | T | T | =en | =en | =en | T |
| `en-IN` | T | T | =en | =en | =en | =en |
| `es` | T | T | T | T | T | T |
| `es-MX` | T | T | T | T | T | T |
| `es-US` | T | T | T | T | T | T |
| `fr` | T | T | T | T | =en | T |
| `fr-CA` | T | T | T | T | T | T |
| `de` | T | T | T | T | T | T |
| `de-AT` | T | T | T | T | T | T |
| `it` | T | T | T | T | **R** | T |
| `pt` | T | T | T | T | T | T |
| `nl` | T | T | T | T | =en | T |
| `ja` | T | T | T | T | T | T |
| `zh` | T | T | T | T | T | T |
| `zh-Hant` | T | T | T | T | T | T |
| `ko` | T | T | T | T | T | T |
| `ru` | T | T | T | T | T | T |
| `pl` | T | T | T | T | T | T |
| `sv` | T | T | T | T | T | T |
| `da` | T | T | T | T | **R** | T |
| `nb` | T | T | T | T | T | T |
| `fi` | T | T | T | T | T | T |
| `cs` | T | T | T | T | T | T |
| `hu` | T | T | T | T | T | T |
| `tr` | T | T | T | T | **R** | T |
| `ar` | T | T | T | T | T | T |
| `hi` | T | T | T | T | T | T |
| `th` | T | T | T | T | T | T |
| `vi` | T | T | T | T | **R** | T |
| `id` | T | T | T | T | T | T |
| `ms` | T | T | T | T | **R** | T |
| `he` | T | T | T | T | T | T |
| `el` | T | T | T | T | T | T |
| `uk` | T | T | T | T | T | T |
| `ro` | T | T | T | T | T | T |
| `und` | root | root | root | root | root | root |


### 4.4 Base volume units (bare ids: US in `en`, see section 5)

| locale | cup | pint | fluid-ounce | quart | gallon | tablespoon | teaspoon | liter | milliliter |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `en` | ref | ref | ref | ref | ref | ref | ref | ref | ref |
| `en-GB` | =en | =en | T | T | T | =en | =en | T | T |
| `en-AU` | =en | =en | T | T | T | =en | =en | T | T |
| `en-CA` | =en | =en | T | T | T | =en | =en | T | T |
| `en-IN` | =en | =en | T | T | T | =en | =en | T | T |
| `es` | T | T | T | T | T | T | T | T | T |
| `es-MX` | T | T | T | T | T | T | T | T | T |
| `es-US` | T | T | T | T | T | T | T | T | T |
| `fr` | T | T | T | =en | =en | T | T | T | T |
| `fr-CA` | T | T | T | T | T | T | T | T | T |
| `de` | T | T | T | T | T | T | T | T | T |
| `de-AT` | T | T | T | T | T | T | T | T | T |
| `it` | T | T | T | T | T | T | T | T | T |
| `pt` | T | =en | T | T | T | T | T | T | T |
| `nl` | T | T | T | T | T | T | T | T | T |
| `ja` | T | T | T | T | T | T | T | T | T |
| `zh` | T | T | T | T | T | T | T | T | T |
| `zh-Hant` | T | T | T | T | T | T | T | T | T |
| `ko` | T | T | T | T | T | T | T | T | T |
| `ru` | T | T | T | T | T | T | T | T | T |
| `pl` | T | T | T | T | T | T | T | T | T |
| `sv` | T | T | =en | T | T | T | T | T | T |
| `da` | =en | =en | T | T | =en | T | T | T | T |
| `nb` | T | T | T | T | T | T | T | T | T |
| `fi` | T | T | T | T | T | T | T | T | T |
| `cs` | T | T | T | T | T | T | T | T | T |
| `hu` | T | T | T | T | T | T | T | T | T |
| `tr` | T | T | T | T | T | T | T | T | T |
| `ar` | T | T | T | T | T | T | T | T | T |
| `hi` | T | T | T | T | T | T | T | T | T |
| `th` | T | T | T | T | T | T | T | T | T |
| `vi` | T | T | T | T | T | T | T | T | T |
| `id` | T | T | T | T | T | T | T | T | T |
| `ms` | T | T | T | T | T | T | T | T | T |
| `he` | T | T | T | T | T | T | T | T | T |
| `el` | T | T | T | T | T | T | T | T | T |
| `uk` | T | T | T | T | T | T | T | T | T |
| `ro` | T | T | T | T | T | T | T | T | T |
| `und` | root | root | root | root | root | root | root | root | root |


### 4.5 Mass and temperature

| locale | mass-gram | mass-kilogram | mass-ounce | mass-pound | mass-stone | temperature-celsius | temperature-fahrenheit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `en` | ref | ref | ref | ref | ref | ref | ref |
| `en-GB` | =en | =en | =en | =en | T | =en | =en |
| `en-AU` | =en | =en | =en | =en | T | =en | =en |
| `en-CA` | =en | =en | =en | =en | T | =en | =en |
| `en-IN` | =en | =en | =en | =en | T | =en | =en |
| `es` | T | T | T | T | =en | T | T |
| `es-MX` | T | T | T | T | =en | T | T |
| `es-US` | T | T | T | T | =en | T | T |
| `fr` | T | T | T | T | =en | T | T |
| `fr-CA` | T | T | T | T | T | T | T |
| `de` | T | T | T | T | T | T | T |
| `de-AT` | T | T | T | T | T | T | T |
| `it` | T | T | T | T | T | T | T |
| `pt` | T | T | T | T | =en | T | T |
| `nl` | T | T | T | T | T | T | T |
| `ja` | T | T | T | T | T | T | T |
| `zh` | T | T | T | T | T | T | T |
| `zh-Hant` | T | T | T | T | T | T | T |
| `ko` | T | T | T | T | T | T | T |
| `ru` | T | T | T | T | T | T | T |
| `pl` | T | T | T | T | T | T | T |
| `sv` | T | T | T | T | T | T | T |
| `da` | T | T | =en | T | T | T | T |
| `nb` | T | T | T | T | T | T | T |
| `fi` | T | T | T | T | T | T | T |
| `cs` | T | T | T | T | T | T | T |
| `hu` | T | T | T | T | T | T | T |
| `tr` | T | T | T | T | T | T | T |
| `ar` | T | T | T | T | T | T | T |
| `hi` | T | T | T | T | T | T | T |
| `th` | T | T | T | T | T | T | T |
| `vi` | T | T | T | T | **R** | T | T |
| `id` | T | T | T | T | T | T | T |
| `ms` | T | T | T | T | T | T | T |
| `he` | T | T | T | T | T | T | T |
| `el` | T | T | T | T | T | T | T |
| `uk` | T | T | T | T | T | T | T |
| `ro` | T | T | T | T | T | T | T |
| `und` | root | root | root | root | root | root | root |


### 4.6 Every root-inherited cell (`**R**`), verbatim

These are the untranslated units. 66 of the 76 non-root entries are the same two keys in every
non-English locale.

| locale | key | inherited value |
| --- | --- | --- |
| `es` | `volume-cup-imperial` | `{0} cup Imp.` |
| `es` | `volume-pint-imperial` | `{0} pt Imp.` |
| `es-MX` | `volume-cup-imperial` | `{0} cup Imp.` |
| `es-MX` | `volume-pint-imperial` | `{0} pt Imp.` |
| `es-US` | `volume-cup-imperial` | `{0} cup Imp.` |
| `es-US` | `volume-pint-imperial` | `{0} pt Imp.` |
| `fr` | `volume-cup-imperial` | `{0} cup Imp.` |
| `fr` | `volume-pint-imperial` | `{0} pt Imp.` |
| `fr-CA` | `volume-cup-imperial` | `{0} cup Imp.` |
| `fr-CA` | `volume-pint-imperial` | `{0} pt Imp.` |
| `de` | `volume-cup-imperial` | `{0} cup Imp.` |
| `de` | `volume-pint-imperial` | `{0} pt Imp.` |
| `de-AT` | `volume-cup-imperial` | `{0} cup Imp.` |
| `de-AT` | `volume-pint-imperial` | `{0} pt Imp.` |
| `it` | `volume-cup-imperial` | `{0} cup Imp.` |
| `it` | `volume-pint-imperial` | `{0} pt Imp.` |
| `it` | `volume-jigger` | `{0} jigger` |
| `pt` | `volume-cup-imperial` | `{0} cup Imp.` |
| `pt` | `volume-pint-imperial` | `{0} pt Imp.` |
| `nl` | `volume-cup-imperial` | `{0} cup Imp.` |
| `nl` | `volume-pint-imperial` | `{0} pt Imp.` |
| `ja` | `volume-cup-imperial` | `{0} cup Imp.` |
| `ja` | `volume-pint-imperial` | `{0} pt Imp.` |
| `zh` | `volume-cup-imperial` | `{0} cup Imp.` |
| `zh` | `volume-pint-imperial` | `{0} pt Imp.` |
| `zh-Hant` | `volume-cup-imperial` | `{0} cup Imp.` |
| `zh-Hant` | `volume-pint-imperial` | `{0} pt Imp.` |
| `ko` | `volume-cup-imperial` | `{0} cup Imp.` |
| `ko` | `volume-pint-imperial` | `{0} pt Imp.` |
| `ru` | `volume-cup-imperial` | `{0} cup Imp.` |
| `ru` | `volume-pint-imperial` | `{0} pt Imp.` |
| `pl` | `volume-cup-imperial` | `{0} cup Imp.` |
| `pl` | `volume-pint-imperial` | `{0} pt Imp.` |
| `pl` | `volume-fluid-ounce-metric` | `{0} fl oz m.` |
| `sv` | `volume-cup-imperial` | `{0} cup Imp.` |
| `sv` | `volume-pint-imperial` | `{0} pt Imp.` |
| `da` | `volume-cup-imperial` | `{0} cup Imp.` |
| `da` | `volume-pint-imperial` | `{0} pt Imp.` |
| `da` | `volume-jigger` | `{0} jigger` |
| `nb` | `volume-cup-imperial` | `{0} cup Imp.` |
| `nb` | `volume-pint-imperial` | `{0} pt Imp.` |
| `fi` | `volume-cup-imperial` | `{0} cup Imp.` |
| `fi` | `volume-pint-imperial` | `{0} pt Imp.` |
| `cs` | `volume-cup-imperial` | `{0} cup Imp.` |
| `cs` | `volume-pint-imperial` | `{0} pt Imp.` |
| `hu` | `volume-cup-imperial` | `{0} cup Imp.` |
| `hu` | `volume-pint-imperial` | `{0} pt Imp.` |
| `tr` | `volume-cup-imperial` | `{0} cup Imp.` |
| `tr` | `volume-pint-imperial` | `{0} pt Imp.` |
| `tr` | `volume-fluid-ounce-metric` | `{0} fl oz m.` |
| `tr` | `volume-jigger` | `{0} jigger` |
| `ar` | `volume-cup-imperial` | `{0} cup Imp.` |
| `ar` | `volume-pint-imperial` | `{0} pt Imp.` |
| `hi` | `volume-cup-imperial` | `{0} cup Imp.` |
| `hi` | `volume-pint-imperial` | `{0} pt Imp.` |
| `th` | `volume-cup-imperial` | `{0} cup Imp.` |
| `th` | `volume-pint-imperial` | `{0} pt Imp.` |
| `vi` | `volume-cup-imperial` | `{0} cup Imp.` |
| `vi` | `volume-pint-imperial` | `{0} pt Imp.` |
| `vi` | `volume-jigger` | `{0} jigger` |
| `vi` | `mass-stone` | `{0} st` |
| `id` | `volume-cup-imperial` | `{0} cup Imp.` |
| `id` | `volume-pint-imperial` | `{0} pt Imp.` |
| `ms` | `volume-cup-imperial` | `{0} cup Imp.` |
| `ms` | `volume-pint-imperial` | `{0} pt Imp.` |
| `ms` | `volume-fluid-ounce-imperial` | `{0} fl oz Imp.` |
| `ms` | `volume-gallon-imperial` | `{0} gal Imp.` |
| `ms` | `volume-jigger` | `{0} jigger` |
| `he` | `volume-cup-imperial` | `{0} cup Imp.` |
| `he` | `volume-pint-imperial` | `{0} pt Imp.` |
| `el` | `volume-cup-imperial` | `{0} cup Imp.` |
| `el` | `volume-pint-imperial` | `{0} pt Imp.` |
| `uk` | `volume-cup-imperial` | `{0} cup Imp.` |
| `uk` | `volume-pint-imperial` | `{0} pt Imp.` |
| `ro` | `volume-cup-imperial` | `{0} cup Imp.` |
| `ro` | `volume-pint-imperial` | `{0} pt Imp.` |

### 4.7 Every English-passthrough cell in a non-English locale (`=en`), verbatim

| locale | key | English value sitting in a non-English locale |
| --- | --- | --- |
| `es` | `mass-stone` | `{0} stones` |
| `es-MX` | `mass-stone` | `{0} stones` |
| `es-US` | `mass-stone` | `{0} stones` |
| `fr` | `volume-quart` | `{0} quarts` |
| `fr` | `volume-gallon` | `{0} gallons` |
| `fr` | `volume-jigger` | `{0} jiggers` |
| `fr` | `mass-stone` | `{0} stones` |
| `pt` | `volume-pint` | `{0} pints` |
| `pt` | `mass-stone` | `{0} stones` |
| `nl` | `volume-jigger` | `{0} jiggers` |
| `sv` | `volume-fluid-ounce` | `{0} fluid ounces` |
| `sv` | `volume-fluid-ounce-imperial` | `{0} Imp. fluid ounces` |
| `da` | `volume-cup` | `{0} cups` |
| `da` | `volume-pint` | `{0} pints` |
| `da` | `volume-fluid-ounce-imperial` | `{0} Imp. fluid ounces` |
| `da` | `volume-gallon` | `{0} gallons` |
| `da` | `mass-ounce` | `{0} ounces` |


### 4.8 The bare id is not globally US: `en` vs `en-GB`

`volume-gallon` and `volume-gallon-imperial` are fixed unit ids, but the *words* CLDR gives them
swap which one carries a qualifier depending on the locale. Long `count=other` patterns:

| key | `en` | `en-GB` |
| --- | --- | --- |
| `volume-cup` | `{0} cups` | `{0} cups` |
| `volume-cup-imperial` | `{0} cups Imperial` | `{0} cups Imperial` |
| `volume-pint` | `{0} pints` | `{0} pints` |
| `volume-pint-imperial` | `{0} pints Imperial` | `{0} pints Imperial` |
| `volume-fluid-ounce` | `{0} fluid ounces` | `{0} US fluid ounces` |
| `volume-fluid-ounce-imperial` | `{0} Imp. fluid ounces` | `{0} fluid ounces` |
| `volume-quart` | `{0} quarts` | `{0} US quarts` |
| `volume-quart-imperial` | `{0} Imp. quarts` | `{0} quarts` |
| `volume-gallon` | `{0} gallons` | `{0} US gallons` |
| `volume-gallon-imperial` | `{0} Imp. gallons` | `{0} gallons` |
| `volume-dessert-spoon` | `{0} dessert spoons` | `{0} US dessertspoons` |
| `volume-dessert-spoon-imperial` | `{0} Imp. dessert spoons` | `{0} dessertspoons` |

`en-AU`, `en-CA` and `en-IN` do the same for `quart`, `gallon` and `dessert-spoon`; `en-CA` and
`en-IN` also do it for `fluid-ounce`, while `en-AU` leaves `volume-fluid-ounce-imperial` at the `en`
string `{0} Imp. fluid ounces` even though it renames `volume-fluid-ounce` to
`{0} US fluid ounces` — so the pair is inconsistent in `en-AU`. Full values in appendix A.

## 5. What this means for cooklang

- **Only four regions need non-metric cooking units at all.** `US` and `LR` are US customary, `GB`
  and `MM` are UK/imperial, and `BS BZ KY PR PW` need Fahrenheit but metric volume and mass.
  Everything else — including `CA`, `AU`, `IN`, `IE`, `NZ` — falls through to `001` = metric. A
  display-preference layer keyed on these nine territories plus a metric default covers CLDR's whole
  model.
- **`volume`/`fluid` is the only usable preference list, and its `GB` row is nearly useless for
  recipes.** `GB` gets exactly two units, `gallon-imperial` and `fluid-ounce-imperial`. There is no
  `pint-imperial`, no `cup-imperial`, no imperial tablespoon or teaspoon in any preference row, so
  CLDR's preference data will never select the 568 ml pint. It cannot drive best-unit selection for
  British recipes; the imperial-volume work in `outstanding.md` has to define its own table.
- **There is no cooking usage in CLDR.** The `volume` usages are `default`, `fluid`, `oil`,
  `vehicle`; `food` exists only under `energy`. Recipe-scale unit choice is not something CLDR
  models, so ADR item 3 (best-unit selection) gets no help here.
- **For oven temperatures, use `temperature`/`default`, not `weather`.** `default` makes only `US`
  Fahrenheit; the five extra Fahrenheit territories (`BS BZ KY PR PW`) appear only under `weather`.
- **Translation coverage is genuinely broad.** 941 of 1170 cells are real translations. 17 of the 30
  keys have a locale-specific string in **all 33** non-English locales: `volume-cup-metric`,
  `volume-pint-metric`, `volume-quart-imperial`, `volume-tablespoon`, `volume-teaspoon`,
  `volume-dessert-spoon`, `volume-dessert-spoon-imperial`, `volume-drop`, `volume-pinch`,
  `volume-dram`, `volume-liter`, `volume-milliliter`, `mass-gram`, `mass-kilogram`, `mass-pound`,
  `temperature-celsius`, `temperature-fahrenheit`. Another five miss in exactly one locale each
  (`volume-cup` in `da`, `volume-fluid-ounce` in `sv`, `volume-quart` in `fr`,
  `volume-gallon-imperial` in `ms`, `mass-ounce` in `da`). CLDR is a good seed for the common
  units, and it fills the known
  `units/spanish.toml` gap: `es` has `{0} cucharada`/`{0} cucharadas` long and `{0} cda` short for
  `volume-tablespoon`, and `{0} cucharadita`/`{0} cucharaditas`/`{0} cdta` for `volume-teaspoon`.
- **Two keys are untranslated everywhere: `volume-cup-imperial` and `volume-pint-imperial`.** All 33
  non-English locales inherit `{0} cup Imp.` and `{0} pt Imp.` from root. If cooklang adds the
  imperial pint and cup, CLDR supplies no name for them outside English — those stay hand-authored,
  in every locale.
- **`volume-jigger` is effectively English-only**: root-inherited in `it`, `da`, `tr`, `vi`, `ms`,
  and an English passthrough (`{0} jiggers`) in `fr` and `nl`. `volume-fluid-ounce-metric` is
  untranslated in `pl` and `tr`; `ms` additionally inherits `fluid-ounce-imperial` and
  `gallon-imperial`. `mass-stone` is an English passthrough in `es`, `es-MX`, `es-US`, `fr`, `pt`
  and root-inherited in `vi`.
- **The bare ids are not globally US, and CLDR already encodes the asymmetry per locale.** In `en`,
  `volume-gallon` is `{0} gallons` and `volume-gallon-imperial` is `{0} Imp. gallons`; in `en-GB`
  those are `{0} US gallons` and `{0} gallons`. So the mapping direction `outstanding.md` worries
  about does not need to be settled globally — it is per-locale data, which is exactly the shape
  `convert::locale::Locale` already has.
- **Contradicts `outstanding.md`: CLDR 48 does have regional cooking units.** `volume-cup-jp`,
  `volume-kosaji` (`小さじ {0}` in `ja`, factor 5 ml), `volume-osaji` (`大さじ {0}`, 15 ml),
  `volume-shaku` (`{0} 勺`), `volume-sai`, `volume-to-jp`, `volume-koku` and `mass-fun` all exist,
  tagged `systems="jpsystem"`. But **CLDR's `cup-jp` is 100 ml** (`factor='1*100*0.000001'`), not
  the 200 ml Japanese cup in the research file — so this is new vocabulary, not a drop-in. Still
  genuinely absent: the 20 ml Australian tablespoon, the 240/200/180 ml cups, Dutch *ons*, German
  *Pfund*, Chinese jin.
- **CLDR also ships a per-unit `systems` attribute, which ADR item 2 can reuse directly.** The
  values observed are `si`, `si_acceptable`, `metric`, `metric_adjacent`, `ussystem`, `uksystem`,
  `jpsystem`, `astronomical`, `prefixable`, `person_age`. Note it classifies `cup-metric`,
  `pint-metric` and `fluid-ounce-metric` as `metric_adjacent`, not `metric` — matching the ADR's
  position that some units belong to no system cleanly.

## 6. Unverified and gaps

- **Whether `cldr-units-full` is documented as resolved: unknown.** The conclusion in section 4.1 is
  empirical (keys present in JSON but absent from the generating XML, with root values). No
  statement either way was found in `cldr-json/README.md` or `cldr-json/PACKAGES.md`.
- **`T` does not prove locale-authored.** It means "differs from `und` and from `en`". A locale that
  inherits from an intermediate parent — `es-MX` from `es`, `de-AT` from `de`, `fr-CA` from `fr` —
  scores `T` on strings it merely inherited. Only comparison against each locale's real parent chain
  would separate those, and that was not done.
- **Only the `long` / `unitPattern-count-other` form was compared.** The files also carry `long`
  `displayName` and `count-one` (plus `count-few`/`many`/`two` in the Slavic, Arabic and Celtic
  locales), and complete `short` and `narrow` blocks. Coverage of those widths per locale is
  unknown; spot checks (`es` short `{0} cda`, `{0} cdta`) show they exist.
- **The 39th locale, `pt-BR`, does not exist** in `cldr-units-full` (HTTP 404), and neither does
  `root` (published as `und`). No data is reported for them beyond that fact. All other 38 requested
  locales were fetched successfully.
- **CLDR release mismatch**: supplemental XML from `main` = CLDR 49 development, locale JSON =
  48.2.3. Whether CLDR 49 changes any preference row or unit name is unknown.
- **What replaces `measurementSystem` is unknown.** The data's own comment says it is deprecated
  "next release", "replaced by enhanced preferences", with no pointer to the replacement element.
- **Factor expression semantics are inferred, not read from the spec.** `a/b*c` evaluates as
  `a/(b*c)`: checked against `teaspoon` = `gal_to_m3/16*48` (1 US gal = 768 tsp, correct) and
  `sho_to_m3` = `2401/1331*1000` (= 1.804 l, the correct shō). Appendix B quotes the expressions
  verbatim rather than evaluating them.
- **No licence review was done** beyond recording `"license": "Unicode-3.0"` from
  `cldr-units-full/package.json`.

## Appendix A — verbatim long `count=other` patterns, all 38 locales

Grouped by unit id. Status codes as in section 4.1. `ref` = the `en` comparison reference,
`root` = `und` itself.

### `volume-cup`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} cups` |
| `en-GB` | =en | `{0} cups` |
| `en-AU` | =en | `{0} cups` |
| `en-CA` | =en | `{0} cups` |
| `en-IN` | =en | `{0} cups` |
| `es` | T | `{0} tazas` |
| `es-MX` | T | `{0} tazas` |
| `es-US` | T | `{0} tazas` |
| `fr` | T | `{0} tasses` |
| `fr-CA` | T | `{0} tasses` |
| `de` | T | `{0} Tassen` |
| `de-AT` | T | `{0} Tassen` |
| `it` | T | `{0} tazze` |
| `pt` | T | `{0} xícaras` |
| `nl` | T | `{0} cup` |
| `ja` | T | `{0} カップ - 米国` |
| `zh` | T | `{0}杯` |
| `zh-Hant` | T | `{0} 杯` |
| `ko` | T | `{0}컵` |
| `ru` | T | `{0} амер. чашки` |
| `pl` | T | `{0} ćwierćkwarty amerykańskiej` |
| `sv` | T | `{0} koppar` |
| `da` | =en | `{0} cups` |
| `nb` | T | `{0} kopper` |
| `fi` | T | `{0} kuppia` |
| `cs` | T | `{0} šálků` |
| `hu` | T | `{0} csésze` |
| `tr` | T | `{0} su bardağı` |
| `ar` | T | `{0} كوب` |
| `hi` | T | `{0} कप` |
| `th` | T | `{0} ถ้วย` |
| `vi` | T | `{0} tách` |
| `id` | T | `{0} cup` |
| `ms` | T | `{0} cawan` |
| `he` | T | `{0} כוסות` |
| `el` | T | `{0} κύπελλα` |
| `uk` | T | `{0} чашки` |
| `ro` | T | `{0} de căni` |
| `und` | root | `{0} c` |

### `volume-cup-metric`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} metric cups` |
| `en-GB` | =en | `{0} metric cups` |
| `en-AU` | =en | `{0} metric cups` |
| `en-CA` | =en | `{0} metric cups` |
| `en-IN` | =en | `{0} metric cups` |
| `es` | T | `{0} tazas métricas` |
| `es-MX` | T | `{0} tazas métricas` |
| `es-US` | T | `{0} tazas métricas` |
| `fr` | T | `{0} tasses métriques` |
| `fr-CA` | T | `{0} tasses métriques` |
| `de` | T | `{0} metrische Tassen` |
| `de-AT` | T | `{0} metrische Tassen` |
| `it` | T | `{0} tazze metriche` |
| `pt` | T | `{0} xícaras métricas` |
| `nl` | T | `{0} metrische cup` |
| `ja` | T | `{0} メトリックカップ` |
| `zh` | T | `{0}公制杯` |
| `zh-Hant` | T | `{0} 公制杯` |
| `ko` | T | `{0}미터식 컵` |
| `ru` | T | `{0} метрической чашки` |
| `pl` | T | `{0} ćwierćkwarty metrycznej` |
| `sv` | T | `{0} metriska koppar` |
| `da` | T | `{0} metriske kopper` |
| `nb` | T | `{0} metriske kopper` |
| `fi` | T | `{0} teekuppia` |
| `cs` | T | `{0} metrických šálků` |
| `hu` | T | `{0} bögre` |
| `tr` | T | `{0} metrik su bardağı` |
| `ar` | T | `{0} كوب متري` |
| `hi` | T | `{0} मीट्रिक कप` |
| `th` | T | `{0} ถ้วยเมตริก` |
| `vi` | T | `{0} cup khối` |
| `id` | T | `{0} metric cup` |
| `ms` | T | `{0} cawan metrik` |
| `he` | T | `{0} כ׳ מידה מטרית` |
| `el` | T | `{0} μετρικά κύπελλα` |
| `uk` | T | `{0} метричної чашки` |
| `ro` | T | `{0} de căni metrice` |
| `und` | root | `{0} mc` |

### `volume-cup-imperial`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} cups Imperial` |
| `en-GB` | =en | `{0} cups Imperial` |
| `en-AU` | =en | `{0} cups Imperial` |
| `en-CA` | =en | `{0} cups Imperial` |
| `en-IN` | =en | `{0} cups Imperial` |
| `es` | **R** | `{0} cup Imp.` |
| `es-MX` | **R** | `{0} cup Imp.` |
| `es-US` | **R** | `{0} cup Imp.` |
| `fr` | **R** | `{0} cup Imp.` |
| `fr-CA` | **R** | `{0} cup Imp.` |
| `de` | **R** | `{0} cup Imp.` |
| `de-AT` | **R** | `{0} cup Imp.` |
| `it` | **R** | `{0} cup Imp.` |
| `pt` | **R** | `{0} cup Imp.` |
| `nl` | **R** | `{0} cup Imp.` |
| `ja` | **R** | `{0} cup Imp.` |
| `zh` | **R** | `{0} cup Imp.` |
| `zh-Hant` | **R** | `{0} cup Imp.` |
| `ko` | **R** | `{0} cup Imp.` |
| `ru` | **R** | `{0} cup Imp.` |
| `pl` | **R** | `{0} cup Imp.` |
| `sv` | **R** | `{0} cup Imp.` |
| `da` | **R** | `{0} cup Imp.` |
| `nb` | **R** | `{0} cup Imp.` |
| `fi` | **R** | `{0} cup Imp.` |
| `cs` | **R** | `{0} cup Imp.` |
| `hu` | **R** | `{0} cup Imp.` |
| `tr` | **R** | `{0} cup Imp.` |
| `ar` | **R** | `{0} cup Imp.` |
| `hi` | **R** | `{0} cup Imp.` |
| `th` | **R** | `{0} cup Imp.` |
| `vi` | **R** | `{0} cup Imp.` |
| `id` | **R** | `{0} cup Imp.` |
| `ms` | **R** | `{0} cup Imp.` |
| `he` | **R** | `{0} cup Imp.` |
| `el` | **R** | `{0} cup Imp.` |
| `uk` | **R** | `{0} cup Imp.` |
| `ro` | **R** | `{0} cup Imp.` |
| `und` | root | `{0} cup Imp.` |

### `volume-pint`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} pints` |
| `en-GB` | =en | `{0} pints` |
| `en-AU` | =en | `{0} pints` |
| `en-CA` | =en | `{0} pints` |
| `en-IN` | =en | `{0} pints` |
| `es` | T | `{0} pintas` |
| `es-MX` | T | `{0} pintas` |
| `es-US` | T | `{0} pintas` |
| `fr` | T | `{0} pintes` |
| `fr-CA` | T | `{0} chopines` |
| `de` | T | `{0} Pints` |
| `de-AT` | T | `{0} Pints` |
| `it` | T | `{0} pinte` |
| `pt` | =en | `{0} pints` |
| `nl` | T | `{0} pint` |
| `ja` | T | `{0} パイント` |
| `zh` | T | `{0}品脱` |
| `zh-Hant` | T | `{0} 品脫` |
| `ko` | T | `{0}파인트` |
| `ru` | T | `{0} амер. пинты` |
| `pl` | T | `{0} półkwarty amerykańskiej` |
| `sv` | T | `{0} pint` |
| `da` | =en | `{0} pints` |
| `nb` | T | `{0} pint` |
| `fi` | T | `{0} pinttiä` |
| `cs` | T | `{0} pint` |
| `hu` | T | `{0} pint` |
| `tr` | T | `{0} pint` |
| `ar` | T | `{0} باينت` |
| `hi` | T | `{0} पाइंट` |
| `th` | T | `{0} ไพนต์` |
| `vi` | T | `{0} panh` |
| `id` | T | `{0} pint` |
| `ms` | T | `{0} pain` |
| `he` | T | `{0} פינט` |
| `el` | T | `{0} πίντες` |
| `uk` | T | `{0} пінти` |
| `ro` | T | `{0} de pinte` |
| `und` | root | `{0} pt` |

### `volume-pint-metric`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} metric pints` |
| `en-GB` | =en | `{0} metric pints` |
| `en-AU` | =en | `{0} metric pints` |
| `en-CA` | =en | `{0} metric pints` |
| `en-IN` | =en | `{0} metric pints` |
| `es` | T | `{0} pintas métricas` |
| `es-MX` | T | `{0} pintas métricas` |
| `es-US` | T | `{0} pintas métricas` |
| `fr` | T | `{0} pintes métriques` |
| `fr-CA` | T | `{0} pintes métriques` |
| `de` | T | `{0} metrische Pints` |
| `de-AT` | T | `{0} metrische Pints` |
| `it` | T | `{0} pinte metriche` |
| `pt` | T | `{0} pints métricos` |
| `nl` | T | `{0} metrische pint` |
| `ja` | T | `{0} メトリックパイント` |
| `zh` | T | `{0}公制品脱` |
| `zh-Hant` | T | `{0} 公制品脫` |
| `ko` | T | `{0}미터식 파인트` |
| `ru` | T | `{0} метрической пинты` |
| `pl` | T | `{0} półkwarty metrycznej` |
| `sv` | T | `{0} metriska pint` |
| `da` | T | `{0} metriske pints` |
| `nb` | T | `{0} metriske pint` |
| `fi` | T | `{0} tuoppia` |
| `cs` | T | `{0} metrických pint` |
| `hu` | T | `{0} metrikus pint` |
| `tr` | T | `{0} metrik pint` |
| `ar` | T | `{0} مكيال متري` |
| `hi` | T | `{0} मीट्रिक पिंट` |
| `th` | T | `{0} ไพนต์เมตริก` |
| `vi` | T | `{0} panh khối` |
| `id` | T | `{0} metric pint` |
| `ms` | T | `{0} pain metrik` |
| `he` | T | `{0} פינט מטרי` |
| `el` | T | `{0} μετρικές πίντες` |
| `uk` | T | `{0} метричної пінти` |
| `ro` | T | `{0} de pinte metrice` |
| `und` | root | `{0} mpt` |

### `volume-pint-imperial`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} pints Imperial` |
| `en-GB` | =en | `{0} pints Imperial` |
| `en-AU` | =en | `{0} pints Imperial` |
| `en-CA` | =en | `{0} pints Imperial` |
| `en-IN` | =en | `{0} pints Imperial` |
| `es` | **R** | `{0} pt Imp.` |
| `es-MX` | **R** | `{0} pt Imp.` |
| `es-US` | **R** | `{0} pt Imp.` |
| `fr` | **R** | `{0} pt Imp.` |
| `fr-CA` | **R** | `{0} pt Imp.` |
| `de` | **R** | `{0} pt Imp.` |
| `de-AT` | **R** | `{0} pt Imp.` |
| `it` | **R** | `{0} pt Imp.` |
| `pt` | **R** | `{0} pt Imp.` |
| `nl` | **R** | `{0} pt Imp.` |
| `ja` | **R** | `{0} pt Imp.` |
| `zh` | **R** | `{0} pt Imp.` |
| `zh-Hant` | **R** | `{0} pt Imp.` |
| `ko` | **R** | `{0} pt Imp.` |
| `ru` | **R** | `{0} pt Imp.` |
| `pl` | **R** | `{0} pt Imp.` |
| `sv` | **R** | `{0} pt Imp.` |
| `da` | **R** | `{0} pt Imp.` |
| `nb` | **R** | `{0} pt Imp.` |
| `fi` | **R** | `{0} pt Imp.` |
| `cs` | **R** | `{0} pt Imp.` |
| `hu` | **R** | `{0} pt Imp.` |
| `tr` | **R** | `{0} pt Imp.` |
| `ar` | **R** | `{0} pt Imp.` |
| `hi` | **R** | `{0} pt Imp.` |
| `th` | **R** | `{0} pt Imp.` |
| `vi` | **R** | `{0} pt Imp.` |
| `id` | **R** | `{0} pt Imp.` |
| `ms` | **R** | `{0} pt Imp.` |
| `he` | **R** | `{0} pt Imp.` |
| `el` | **R** | `{0} pt Imp.` |
| `uk` | **R** | `{0} pt Imp.` |
| `ro` | **R** | `{0} pt Imp.` |
| `und` | root | `{0} pt Imp.` |

### `volume-fluid-ounce`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} fluid ounces` |
| `en-GB` | T | `{0} US fluid ounces` |
| `en-AU` | T | `{0} US fluid ounces` |
| `en-CA` | T | `{0} US fluid ounces` |
| `en-IN` | T | `{0} US fluid ounces` |
| `es` | T | `{0} onzas líquidas` |
| `es-MX` | T | `{0} onzas líquidas` |
| `es-US` | T | `{0} onzas fluidas` |
| `fr` | T | `{0} onces liquides` |
| `fr-CA` | T | `{0} onces liquides` |
| `de` | T | `{0} Flüssigunzen` |
| `de-AT` | T | `{0} Flüssigunzen` |
| `it` | T | `{0} once liquide` |
| `pt` | T | `{0} onças fluidas` |
| `nl` | T | `{0} fluid ounce` |
| `ja` | T | `{0} 液量オンス` |
| `zh` | T | `{0}液盎司` |
| `zh-Hant` | T | `{0} 液盎司` |
| `ko` | T | `{0}액량 온스` |
| `ru` | T | `{0} амер. жидкой унции` |
| `pl` | T | `{0} uncji płynu amerykańskiej` |
| `sv` | =en | `{0} fluid ounces` |
| `da` | T | `{0} engelske fluid ounces` |
| `nb` | T | `{0} væskeunser` |
| `fi` | T | `{0} nesteunssia` |
| `cs` | T | `{0} dutých uncí` |
| `hu` | T | `{0} folyadékuncia` |
| `tr` | T | `{0} sıvı ons` |
| `ar` | T | `{0} أونصة سائلة` |
| `hi` | T | `{0} फ़्लूइड आउंस` |
| `th` | T | `{0} ฟลูอิดออนซ์` |
| `vi` | T | `{0} aoxơ chất lỏng` |
| `id` | T | `{0} fluid ounce` |
| `ms` | T | `{0} auns cecair` |
| `he` | T | `{0} אונקיות נוזלים` |
| `el` | T | `{0} ουγγιές όγκου` |
| `uk` | T | `{0} рідинної унції` |
| `ro` | T | `{0} de uncii lichide` |
| `und` | root | `{0} fl oz US` |

### `volume-fluid-ounce-metric`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} metric fluid ounces` |
| `en-GB` | =en | `{0} metric fluid ounces` |
| `en-AU` | =en | `{0} metric fluid ounces` |
| `en-CA` | =en | `{0} metric fluid ounces` |
| `en-IN` | =en | `{0} metric fluid ounces` |
| `es` | T | `{0} onzas líquidas métricas` |
| `es-MX` | T | `{0} onzas líquidas métricas` |
| `es-US` | T | `{0} onzas líquidas métricas` |
| `fr` | T | `{0} onces liquides métriques` |
| `fr-CA` | T | `{0} onces liquides métriques` |
| `de` | T | `{0} metrische Flüssigunzen` |
| `de-AT` | T | `{0} metrische Flüssigunzen` |
| `it` | T | `{0} once liquide metriche` |
| `pt` | T | `{0} onças fluidas métricas` |
| `nl` | T | `{0} metrische fluid ounces` |
| `ja` | T | `{0} メトリック液量オンス` |
| `zh` | T | `{0}公制液量盎司` |
| `zh-Hant` | T | `{0}公制液盎司` |
| `ko` | T | `{0}미터식 액량 온스` |
| `ru` | T | `{0} метрической жидкой унции` |
| `pl` | **R** | `{0} fl oz m.` |
| `sv` | T | `{0} metriska flytande uns` |
| `da` | T | `{0} metriske flydende ounces` |
| `nb` | T | `{0} metriske væskeunser` |
| `fi` | T | `{0} nesteunssia` |
| `cs` | T | `{0} metrických dutých uncí` |
| `hu` | T | `{0} metrikus folyadékuncia` |
| `tr` | **R** | `{0} fl oz m.` |
| `ar` | T | `{0} أونصة سائلة مترية` |
| `hi` | T | `{0} मेट्रिक फ़्लूड आउंस` |
| `th` | T | `{0} ฟลูอิดออนซ์เมตริก` |
| `vi` | T | `{0} ao-xơ chất lỏng hệ mét` |
| `id` | T | `{0} metric fluid ounce` |
| `ms` | T | `{0} auns cecair metrik` |
| `he` | T | `{0} אונקיית נוזל` |
| `el` | T | `{0} μετρικές ουγγιές υγρών` |
| `uk` | T | `{0} метричної рідинної унції` |
| `ro` | T | `{0} de uncii lichide metrice` |
| `und` | root | `{0} fl oz m.` |

### `volume-fluid-ounce-imperial`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} Imp. fluid ounces` |
| `en-GB` | T | `{0} fluid ounces` |
| `en-AU` | =en | `{0} Imp. fluid ounces` |
| `en-CA` | T | `{0} fluid ounces` |
| `en-IN` | T | `{0} fluid ounces` |
| `es` | T | `{0} onzas líquidas imperiales` |
| `es-MX` | T | `{0} onzas líquidas imperiales` |
| `es-US` | T | `{0} onzas fluidas imperiales` |
| `fr` | T | `{0} onces liquides impériales` |
| `fr-CA` | T | `{0} onces liquides impériales` |
| `de` | T | `{0} Imp. Flüssigunzen` |
| `de-AT` | T | `{0} Imp. Flüssigunzen` |
| `it` | T | `{0} once liquide imperiali` |
| `pt` | T | `{0} onças fluidas imperiais` |
| `nl` | T | `{0} Imp. fluid ounce` |
| `ja` | T | `{0} 英液量オンス` |
| `zh` | T | `{0}英制液盎司` |
| `zh-Hant` | T | `{0} 英液盎司` |
| `ko` | T | `{0}영국 액량 온스` |
| `ru` | T | `{0} имп. жидкой унции` |
| `pl` | T | `{0} uncji płynu angielskiej` |
| `sv` | =en | `{0} Imp. fluid ounces` |
| `da` | =en | `{0} Imp. fluid ounces` |
| `nb` | T | `{0} britiske væskeunser` |
| `fi` | T | `{0} br. nesteunssia` |
| `cs` | T | `{0} britských dutých uncí` |
| `hu` | T | `{0} bir. folyadék uncia` |
| `tr` | T | `{0} İng. sıvı ons` |
| `ar` | T | `{0} أونصة سائلة إمبراطورية` |
| `hi` | T | `{0} इम्पी॰ फ़्लुइड आउंस` |
| `th` | T | `{0} ฟลูอิดออนซ์ระบบอิมพีเรียล` |
| `vi` | T | `{0} Aoxơ chất lỏng theo hệ đo lường Anh` |
| `id` | T | `{0} Imp. fluid ounce` |
| `ms` | **R** | `{0} fl oz Imp.` |
| `he` | T | `{0} אונקיית נוזלים אימפריאלית` |
| `el` | T | `{0} αγγλοσαξονικές ουγγιές όγκου` |
| `uk` | T | `{0} англійської рідинної унції` |
| `ro` | T | `{0} de uncii lichide imperiale` |
| `und` | root | `{0} fl oz Imp.` |

### `volume-quart`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} quarts` |
| `en-GB` | T | `{0} US quarts` |
| `en-AU` | T | `{0} US quarts` |
| `en-CA` | T | `{0} US quarts` |
| `en-IN` | T | `{0} US quarts` |
| `es` | T | `{0} cuartos` |
| `es-MX` | T | `{0} cuartos` |
| `es-US` | T | `{0} cuartos` |
| `fr` | =en | `{0} quarts` |
| `fr-CA` | T | `{0} pintes` |
| `de` | T | `{0} Quart` |
| `de-AT` | T | `{0} Quart` |
| `it` | T | `{0} quarti` |
| `pt` | T | `{0} quartos` |
| `nl` | T | `{0} quart` |
| `ja` | T | `{0} クォート` |
| `zh` | T | `{0}夸脱` |
| `zh-Hant` | T | `{0} 夸脫` |
| `ko` | T | `{0}쿼트` |
| `ru` | T | `{0} амер. кварты` |
| `pl` | T | `{0} kwarty amerykańskiej` |
| `sv` | T | `{0} quart` |
| `da` | T | `{0} engelske quarts` |
| `nb` | T | `{0} quart` |
| `fi` | T | `{0} neljännesgallonaa` |
| `cs` | T | `{0} kvartů` |
| `hu` | T | `{0} quart` |
| `tr` | T | `{0} quart` |
| `ar` | T | `{0} ربع غالون` |
| `hi` | T | `{0} क्वार्ट` |
| `th` | T | `{0} ควอร์ต` |
| `vi` | T | `{0} quart` |
| `id` | T | `{0} quart` |
| `ms` | T | `{0} kuart` |
| `he` | T | `{0} קווארטות` |
| `el` | T | `{0} τέταρτα του γαλονιού` |
| `uk` | T | `{0} кварти` |
| `ro` | T | `{0} de quarte` |
| `und` | root | `{0} qt` |

### `volume-quart-imperial`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} Imp. quarts` |
| `en-GB` | T | `{0} quarts` |
| `en-AU` | T | `{0} quarts` |
| `en-CA` | T | `{0} quarts` |
| `en-IN` | T | `{0} quarts` |
| `es` | T | `{0} cuartos imperiales` |
| `es-MX` | T | `{0} cuartos imperiales` |
| `es-US` | T | `{0} cuartos imperiales` |
| `fr` | T | `{0} quarts impériaux` |
| `fr-CA` | T | `{0} pintes impériales` |
| `de` | T | `{0} Imp. Quart` |
| `de-AT` | T | `{0} Imp. Quart` |
| `it` | T | `{0} quarti imperiali` |
| `pt` | T | `{0} quartos imperiais` |
| `nl` | T | `{0} imp. quarts` |
| `ja` | T | `{0} 英クォート` |
| `zh` | T | `{0}英制夸脱` |
| `zh-Hant` | T | `{0} 英制夸脫` |
| `ko` | T | `{0}영국 쿼트` |
| `ru` | T | `{0} имп. кварты` |
| `pl` | T | `{0} kwarty angielskiej` |
| `sv` | T | `{0} brittiska quarts` |
| `da` | T | `{0} britiske quarts` |
| `nb` | T | `{0} britiske quart` |
| `fi` | T | `{0} br. neljännesgallonaa` |
| `cs` | T | `{0} britských kvartů` |
| `hu` | T | `{0} birodalmi kvart` |
| `tr` | T | `{0} İng. quart` |
| `ar` | T | `{0} ربع غالون إمبراطوري` |
| `hi` | T | `{0} क्वार्ट इम्पीरियल` |
| `th` | T | `{0} ควอร์ตอังกฤษ` |
| `vi` | T | `{0} lít Anh` |
| `id` | T | `{0} quart Imp.` |
| `ms` | T | `{0} kuart Imp.` |
| `he` | T | `{0} קווארטות אימפריאליות` |
| `el` | T | `{0} αγγλοσαξονικά τέταρτα του γαλονιού` |
| `uk` | T | `{0} англійської кварти` |
| `ro` | T | `{0} de quarte imperiale` |
| `und` | root | `{0} qt Imp.` |

### `volume-gallon`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} gallons` |
| `en-GB` | T | `{0} US gallons` |
| `en-AU` | T | `{0} US gallons` |
| `en-CA` | T | `{0} US gallons` |
| `en-IN` | T | `{0} US gallons` |
| `es` | T | `{0} galones` |
| `es-MX` | T | `{0} galones` |
| `es-US` | T | `{0} galones` |
| `fr` | =en | `{0} gallons` |
| `fr-CA` | T | `{0} gallons` |
| `de` | T | `{0} Gallonen` |
| `de-AT` | T | `{0} Gallonen` |
| `it` | T | `{0} galloni` |
| `pt` | T | `{0} galões` |
| `nl` | T | `{0} gallon` |
| `ja` | T | `{0} ガロン` |
| `zh` | T | `{0}加仑` |
| `zh-Hant` | T | `{0} 加侖` |
| `ko` | T | `{0}갤런` |
| `ru` | T | `{0} амер. галлона` |
| `pl` | T | `{0} galona amerykańskiego` |
| `sv` | T | `{0} gallon` |
| `da` | =en | `{0} gallons` |
| `nb` | T | `{0} gallon` |
| `fi` | T | `{0} am. gallonaa` |
| `cs` | T | `{0} galonů` |
| `hu` | T | `{0} gallon` |
| `tr` | T | `{0} galon` |
| `ar` | T | `{0} غالون` |
| `hi` | T | `{0} गैलन` |
| `th` | T | `{0} แกลลอน` |
| `vi` | T | `{0} gallon` |
| `id` | T | `{0} galon` |
| `ms` | T | `{0} gelen` |
| `he` | T | `{0} גלונים` |
| `el` | T | `{0} γαλόνια` |
| `uk` | T | `{0} галона` |
| `ro` | T | `{0} de galoane` |
| `und` | root | `{0} gal US` |

### `volume-gallon-imperial`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} Imp. gallons` |
| `en-GB` | T | `{0} gallons` |
| `en-AU` | T | `{0} gallons` |
| `en-CA` | T | `{0} gallons` |
| `en-IN` | T | `{0} gallons` |
| `es` | T | `{0} galones imperiales` |
| `es-MX` | T | `{0} galones imperiales` |
| `es-US` | T | `{0} galones imperiales` |
| `fr` | T | `{0} gallons impériaux` |
| `fr-CA` | T | `{0} gallons impériaux` |
| `de` | T | `{0} Imp. Gallonen` |
| `de-AT` | T | `{0} Imp. Gallonen` |
| `it` | T | `{0} galloni imperiali` |
| `pt` | T | `{0} galões imperiais` |
| `nl` | T | `{0} imp. gallon` |
| `ja` | T | `{0} 英ガロン` |
| `zh` | T | `{0}英制加仑` |
| `zh-Hant` | T | `{0} 英制加侖` |
| `ko` | T | `{0}영국식 갤런` |
| `ru` | T | `{0} имп. галлона` |
| `pl` | T | `{0} galona angielskiego` |
| `sv` | T | `{0} br. gallon` |
| `da` | T | `{0} engelske gallons` |
| `nb` | T | `{0} britiske gallon` |
| `fi` | T | `{0} br. gallonaa` |
| `cs` | T | `{0} britských galonů` |
| `hu` | T | `{0} birodalmi gallon` |
| `tr` | T | `{0} İng. galonu` |
| `ar` | T | `{0} غالون إمبراطوري` |
| `hi` | T | `{0} इम्पीरियल गैलन` |
| `th` | T | `{0} แกลลอนอังกฤษ` |
| `vi` | T | `{0} gallon Anh` |
| `id` | T | `{0} galon Imp.` |
| `ms` | **R** | `{0} gal Imp.` |
| `he` | T | `{0} גלון אימפריאלי` |
| `el` | T | `{0} αγγλοσαξονικά γαλόνια` |
| `uk` | T | `{0} англійського галона` |
| `ro` | T | `{0} de galoane imperiale` |
| `und` | root | `{0} gal Imp.` |

### `volume-tablespoon`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} tablespoons` |
| `en-GB` | =en | `{0} tablespoons` |
| `en-AU` | =en | `{0} tablespoons` |
| `en-CA` | =en | `{0} tablespoons` |
| `en-IN` | =en | `{0} tablespoons` |
| `es` | T | `{0} cucharadas` |
| `es-MX` | T | `{0} cucharadas` |
| `es-US` | T | `{0} cucharadas` |
| `fr` | T | `{0} cuillères à soupe` |
| `fr-CA` | T | `{0} cuillères à soupe` |
| `de` | T | `{0} Esslöffel` |
| `de-AT` | T | `{0} Esslöffel` |
| `it` | T | `{0} cucchiai da tavola` |
| `pt` | T | `{0} colheres de sopa` |
| `nl` | T | `{0} eetlepels` |
| `ja` | T | `{0} テーブルスプーン` |
| `zh` | T | `{0}汤匙` |
| `zh-Hant` | T | `{0} 湯匙` |
| `ko` | T | `{0}테이블스푼` |
| `ru` | T | `{0} столовой ложки` |
| `pl` | T | `{0} łyżki stołowej` |
| `sv` | T | `{0} matskedar` |
| `da` | T | `{0} spiseskeer` |
| `nb` | T | `{0} spiseskjeer` |
| `fi` | T | `{0} ruokalusikkaa` |
| `cs` | T | `{0} lžic` |
| `hu` | T | `{0} evőkanál` |
| `tr` | T | `{0} yemek kaşığı` |
| `ar` | T | `{0} ملعقة كبيرة` |
| `hi` | T | `{0} टेबलस्पून` |
| `th` | T | `{0} ช้อนโต๊ะ` |
| `vi` | T | `{0} muỗng canh` |
| `id` | T | `{0} sendok makan` |
| `ms` | T | `{0} sudu besar` |
| `he` | T | `{0} כפות` |
| `el` | T | `{0} κουταλιές της σούπας` |
| `uk` | T | `{0} столової ложки` |
| `ro` | T | `{0} de linguri` |
| `und` | root | `{0} tbsp` |

### `volume-teaspoon`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} teaspoons` |
| `en-GB` | =en | `{0} teaspoons` |
| `en-AU` | =en | `{0} teaspoons` |
| `en-CA` | =en | `{0} teaspoons` |
| `en-IN` | =en | `{0} teaspoons` |
| `es` | T | `{0} cucharaditas` |
| `es-MX` | T | `{0} cucharaditas` |
| `es-US` | T | `{0} cucharaditas` |
| `fr` | T | `{0} cuillères à café` |
| `fr-CA` | T | `{0} cuillères à thé` |
| `de` | T | `{0} Teelöffel` |
| `de-AT` | T | `{0} Teelöffel` |
| `it` | T | `{0} cucchiai da tè` |
| `pt` | T | `{0} colheres de chá` |
| `nl` | T | `{0} theelepels` |
| `ja` | T | `{0} ティースプーン` |
| `zh` | T | `{0}茶匙` |
| `zh-Hant` | T | `{0} 茶匙` |
| `ko` | T | `{0}티스푼` |
| `ru` | T | `{0} чайной ложки` |
| `pl` | T | `{0} łyżeczki` |
| `sv` | T | `{0} teskedar` |
| `da` | T | `{0} teskeer` |
| `nb` | T | `{0} teskjeer` |
| `fi` | T | `{0} teelusikkaa` |
| `cs` | T | `{0} lžiček` |
| `hu` | T | `{0} kávéskanál` |
| `tr` | T | `{0} çay kaşığı` |
| `ar` | T | `{0} ملعقة صغيرة` |
| `hi` | T | `{0} टी स्पून` |
| `th` | T | `{0} ช้อนชา` |
| `vi` | T | `{0} muỗng cà phê` |
| `id` | T | `{0} sendok teh` |
| `ms` | T | `{0} sudu teh` |
| `he` | T | `{0} כפיות` |
| `el` | T | `{0} κουταλιές του γλυκού` |
| `uk` | T | `{0} чайної ложки` |
| `ro` | T | `{0} de lingurițe` |
| `und` | root | `{0} tsp` |

### `volume-dessert-spoon`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} dessert spoons` |
| `en-GB` | T | `{0} US dessertspoons` |
| `en-AU` | T | `{0} US dessertspoons` |
| `en-CA` | T | `{0} US dessertspoons` |
| `en-IN` | T | `{0} US dessertspoons` |
| `es` | T | `{0} cucharadas de postre` |
| `es-MX` | T | `{0} cucharadas de postre` |
| `es-US` | T | `{0} cucharadas de postre` |
| `fr` | T | `{0} cuillères à dessert` |
| `fr-CA` | T | `{0} cuillères à dessert` |
| `de` | T | `{0} Dessertlöffel` |
| `de-AT` | T | `{0} Dessertlöffel` |
| `it` | T | `{0} cucchiaini da dessert` |
| `pt` | T | `{0} colheres de sobremesa` |
| `nl` | T | `{0} dessertlepels` |
| `ja` | T | `中さじ {0}` |
| `zh` | T | `{0}甜点匙` |
| `zh-Hant` | T | `{0} 甜品匙` |
| `ko` | T | `{0}디저트스푼` |
| `ru` | T | `{0} десертной ложки` |
| `pl` | T | `{0} łyżki deserowej` |
| `sv` | T | `{0} dessertskedar` |
| `da` | T | `{0} dessertskeer` |
| `nb` | T | `{0} barneskjeer` |
| `fi` | T | `{0} jälkiruokalusikkaa` |
| `cs` | T | `{0} dezertních lžiček` |
| `hu` | T | `{0} desszertkanál` |
| `tr` | T | `{0} tatlı kaşığı` |
| `ar` | T | `{0} ملعقة حلو` |
| `hi` | T | `{0} डेज़र्ट स्पून` |
| `th` | T | `{0} ช้อนขนม` |
| `vi` | T | `{0} thìa tráng miệng` |
| `id` | T | `{0} sendok dessert` |
| `ms` | T | `{0} sudu desert` |
| `he` | T | `{0} כפיות קינוחים` |
| `el` | T | `{0} κουταλιές φρούτου` |
| `uk` | T | `{0} десертної ложки` |
| `ro` | T | `{0} de linguri de desert` |
| `und` | root | `{0} dstspn` |

### `volume-dessert-spoon-imperial`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} Imp. dessert spoons` |
| `en-GB` | T | `{0} dessertspoons` |
| `en-AU` | T | `{0} dessertspoons` |
| `en-CA` | T | `{0} dessertspoons` |
| `en-IN` | T | `{0} dessertspoons` |
| `es` | T | `{0} cucharadas de postre imperiales` |
| `es-MX` | T | `{0} cucharadas de postre imperiales` |
| `es-US` | T | `{0} cucharadas de postre imperiales` |
| `fr` | T | `{0} cuillères à dessert impériales` |
| `fr-CA` | T | `{0} cuillères à dessert impériales` |
| `de` | T | `{0} Imp. Dessertlöffel` |
| `de-AT` | T | `{0} Imp. Dessertlöffel` |
| `it` | T | `{0} cucchiaini da dessert imperiali` |
| `pt` | T | `{0} colheres de sobremesa imperiais` |
| `nl` | T | `{0} imp. dessertlepels` |
| `ja` | T | `{0} 英デザートスプーン` |
| `zh` | T | `{0}英制甜点匙` |
| `zh-Hant` | T | `{0} 英制甜品匙` |
| `ko` | T | `{0}영국 디저트스푼` |
| `ru` | T | `{0} имп. десертной ложки` |
| `pl` | T | `{0} imperialnej łyżeczki deserowej` |
| `sv` | T | `{0} brittiska dessertskedar` |
| `da` | T | `{0} britiske dessertskeer` |
| `nb` | T | `{0} britiske barneskjeer` |
| `fi` | T | `{0} br. jälkiruokalusikkaa` |
| `cs` | T | `{0} britských dezertních lžiček` |
| `hu` | T | `{0} bir. desszertkanál` |
| `tr` | T | `{0} İng. tatlı kaşığı` |
| `ar` | T | `{0} ملعقة حلو إمبراطوري` |
| `hi` | T | `{0} इम्पीरियल डेज़र्टस्पून` |
| `th` | T | `{0} ช้อนขนม - อังกฤษ` |
| `vi` | T | `{0} thìa tráng miệng Anh` |
| `id` | T | `{0} sendok dessert Imp.` |
| `ms` | T | `{0} sudu desert Imp.` |
| `he` | T | `{0} כפיות קינוחים בארה"ב` |
| `el` | T | `{0} αγγλοσαξονικές κουταλιές φρούτου` |
| `uk` | T | `{0} англійської десертної ложки` |
| `ro` | T | `{0} de linguri de desert imperiale` |
| `und` | root | `{0} dstspn Imp` |

### `volume-drop`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} drops` |
| `en-GB` | =en | `{0} drops` |
| `en-AU` | =en | `{0} drops` |
| `en-CA` | =en | `{0} drops` |
| `en-IN` | =en | `{0} drops` |
| `es` | T | `{0} gotas` |
| `es-MX` | T | `{0} gotas` |
| `es-US` | T | `{0} gotas` |
| `fr` | T | `{0} gouttes` |
| `fr-CA` | T | `{0} gouttes` |
| `de` | T | `{0} Tropfen` |
| `de-AT` | T | `{0} Tropfen` |
| `it` | T | `{0} gocce` |
| `pt` | T | `{0} gotas` |
| `nl` | T | `{0} druppels` |
| `ja` | T | `{0} 滴` |
| `zh` | T | `{0}滴` |
| `zh-Hant` | T | `{0} 滴` |
| `ko` | T | `{0}방울` |
| `ru` | T | `{0} капли` |
| `pl` | T | `{0} kropli` |
| `sv` | T | `{0} droppar` |
| `da` | T | `{0} dråber` |
| `nb` | T | `{0} dråper` |
| `fi` | T | `{0} tippaa` |
| `cs` | T | `{0} kapek` |
| `hu` | T | `{0} csepp` |
| `tr` | T | `{0} damla` |
| `ar` | T | `{0} قطرة` |
| `hi` | T | `{0} बूंद` |
| `th` | T | `{0} หยด` |
| `vi` | T | `{0} giọt` |
| `id` | T | `{0} tetes` |
| `ms` | T | `{0} titis` |
| `he` | T | `{0} טיפות` |
| `el` | T | `{0} σταγόνες` |
| `uk` | T | `{0} краплі` |
| `ro` | T | `{0} de picături` |
| `und` | root | `{0} drop` |

### `volume-pinch`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} pinches` |
| `en-GB` | =en | `{0} pinches` |
| `en-AU` | =en | `{0} pinches` |
| `en-CA` | =en | `{0} pinches` |
| `en-IN` | =en | `{0} pinches` |
| `es` | T | `{0} pizcas` |
| `es-MX` | T | `{0} pizcas` |
| `es-US` | T | `{0} pizcas` |
| `fr` | T | `{0} pincées` |
| `fr-CA` | T | `{0} pincées` |
| `de` | T | `{0} Prisen` |
| `de-AT` | T | `{0} Prisen` |
| `it` | T | `{0} pizzichi` |
| `pt` | T | `{0} pitadas` |
| `nl` | T | `{0} snufjes` |
| `ja` | T | `{0} つまみ` |
| `zh` | T | `{0}撮` |
| `zh-Hant` | T | `{0} 小撮` |
| `ko` | T | `{0}꼬집` |
| `ru` | T | `{0} щепотки` |
| `pl` | T | `{0} szczypty` |
| `sv` | T | `{0} nypa` |
| `da` | T | `{0} knivspidser` |
| `nb` | T | `{0} klyper` |
| `fi` | T | `{0} ripausta` |
| `cs` | T | `{0} špetek` |
| `hu` | T | `{0} csipet` |
| `tr` | T | `{0} tutam` |
| `ar` | T | `{0} رشّة` |
| `hi` | T | `{0} चुटकी` |
| `th` | T | `{0} หยิบมือ` |
| `vi` | T | `{0} nhúm` |
| `id` | T | `{0} jumput` |
| `ms` | T | `{0} cubit` |
| `he` | T | `{0} פינץ'` |
| `el` | T | `{0} πρέζες` |
| `uk` | T | `{0} дрібки` |
| `ro` | T | `{0} de vârfuri de cuțit` |
| `und` | root | `{0} pinch` |

### `volume-jigger`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} jiggers` |
| `en-GB` | =en | `{0} jiggers` |
| `en-AU` | =en | `{0} jiggers` |
| `en-CA` | =en | `{0} jiggers` |
| `en-IN` | =en | `{0} jiggers` |
| `es` | T | `{0} vasos medidores` |
| `es-MX` | T | `{0} vasos medidores` |
| `es-US` | T | `{0} vasos medidores` |
| `fr` | =en | `{0} jiggers` |
| `fr-CA` | T | `{0} gobelets doseurs` |
| `de` | T | `{0} Jigger` |
| `de-AT` | T | `{0} Jigger` |
| `it` | **R** | `{0} jigger` |
| `pt` | T | `{0} dosadores` |
| `nl` | =en | `{0} jiggers` |
| `ja` | T | `{0} ジガー` |
| `zh` | T | `{0}量杯` |
| `zh-Hant` | T | `{0} 量酒杯` |
| `ko` | T | `{0}지거` |
| `ru` | T | `{0} джиггера` |
| `pl` | T | `{0} jiggera` |
| `sv` | T | `{0} mätglas` |
| `da` | **R** | `{0} jigger` |
| `nb` | T | `{0} shot` |
| `fi` | T | `{0} jiggeriä` |
| `cs` | T | `{0} barmanských odměrek` |
| `hu` | T | `{0} adagolópohár` |
| `tr` | **R** | `{0} jigger` |
| `ar` | T | `{0} قدح` |
| `hi` | T | `{0} जिगर` |
| `th` | T | `{0} จิกเกอร์` |
| `vi` | **R** | `{0} jigger` |
| `id` | T | `{0} sloki` |
| `ms` | **R** | `{0} jigger` |
| `he` | T | `{0} ג'יגר` |
| `el` | T | `{0} μεζούρες` |
| `uk` | T | `{0} мірна склянка` |
| `ro` | T | `{0} de jiggere` |
| `und` | root | `{0} jigger` |

### `volume-dram`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} drams` |
| `en-GB` | =en | `{0} drams` |
| `en-AU` | =en | `{0} drams` |
| `en-CA` | T | `{0} fluid drams` |
| `en-IN` | =en | `{0} drams` |
| `es` | T | `{0} dracmas líquidas` |
| `es-MX` | T | `{0} dracmas líquidas` |
| `es-US` | T | `{0} dreacmas fluidas` |
| `fr` | T | `{0} drachmes` |
| `fr-CA` | T | `{0} drachmes` |
| `de` | T | `{0} Dram` |
| `de-AT` | T | `{0} Dram` |
| `it` | T | `{0} dramme` |
| `pt` | T | `{0} dracmas líquidos` |
| `nl` | T | `{0} drachme` |
| `ja` | T | `{0} ドラム` |
| `zh` | T | `{0}打兰` |
| `zh-Hant` | T | `{0} 打蘭` |
| `ko` | T | `{0}영국 액량 드램` |
| `ru` | T | `{0} жидкой драхмы` |
| `pl` | T | `{0} drachmy płynu` |
| `sv` | T | `{0} dram` |
| `da` | T | `{0} britiske flydende dramme` |
| `nb` | T | `{0} dram` |
| `fi` | T | `{0} dramia` |
| `cs` | T | `{0} dutých dramů` |
| `hu` | T | `{0} dram` |
| `tr` | T | `{0} dram` |
| `ar` | T | `{0} درهم` |
| `hi` | T | `{0} ड्रम` |
| `th` | T | `{0} แดรม` |
| `vi` | T | `{0} dram` |
| `id` | T | `{0} dram` |
| `ms` | T | `{0} dram` |
| `he` | T | `{0} דראם אלכוהול` |
| `el` | T | `{0} δράμια` |
| `uk` | T | `{0} рідинної драхми` |
| `ro` | T | `{0} de drami lichizi` |
| `und` | root | `{0} dram fl` |

### `volume-liter`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} liters` |
| `en-GB` | T | `{0} litres` |
| `en-AU` | T | `{0} litres` |
| `en-CA` | T | `{0} litres` |
| `en-IN` | T | `{0} litres` |
| `es` | T | `{0} litros` |
| `es-MX` | T | `{0} litros` |
| `es-US` | T | `{0} litros` |
| `fr` | T | `{0} litres` |
| `fr-CA` | T | `{0} litres` |
| `de` | T | `{0} Liter` |
| `de-AT` | T | `{0} Liter` |
| `it` | T | `{0} litri` |
| `pt` | T | `{0} litros` |
| `nl` | T | `{0} liter` |
| `ja` | T | `{0} リットル` |
| `zh` | T | `{0}升` |
| `zh-Hant` | T | `{0} 公升` |
| `ko` | T | `{0}리터` |
| `ru` | T | `{0} литра` |
| `pl` | T | `{0} litra` |
| `sv` | T | `{0} liter` |
| `da` | T | `{0} liter` |
| `nb` | T | `{0} liter` |
| `fi` | T | `{0} litraa` |
| `cs` | T | `{0} litrů` |
| `hu` | T | `{0} liter` |
| `tr` | T | `{0} litre` |
| `ar` | T | `{0} لتر` |
| `hi` | T | `{0} लीटर` |
| `th` | T | `{0} ลิตร` |
| `vi` | T | `{0} lít` |
| `id` | T | `{0} liter` |
| `ms` | T | `{0} liter` |
| `he` | T | `{0} ליטר` |
| `el` | T | `{0} λίτρα` |
| `uk` | T | `{0} літра` |
| `ro` | T | `{0} de litri` |
| `und` | root | `{0} l` |

### `volume-milliliter`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} milliliters` |
| `en-GB` | T | `{0} millilitres` |
| `en-AU` | T | `{0} millilitres` |
| `en-CA` | T | `{0} millilitres` |
| `en-IN` | T | `{0} millilitres` |
| `es` | T | `{0} mililitros` |
| `es-MX` | T | `{0} mililitros` |
| `es-US` | T | `{0} mililitros` |
| `fr` | T | `{0} millilitres` |
| `fr-CA` | T | `{0} millilitres` |
| `de` | T | `{0} Milliliter` |
| `de-AT` | T | `{0} Milliliter` |
| `it` | T | `{0} millilitri` |
| `pt` | T | `{0} mililitros` |
| `nl` | T | `{0} milliliter` |
| `ja` | T | `{0} ミリリットル` |
| `zh` | T | `{0}毫升` |
| `zh-Hant` | T | `{0} 毫升` |
| `ko` | T | `{0}밀리리터` |
| `ru` | T | `{0} миллилитра` |
| `pl` | T | `{0} mililitra` |
| `sv` | T | `{0} milliliter` |
| `da` | T | `{0} milliliter` |
| `nb` | T | `{0} milliliter` |
| `fi` | T | `{0} millilitraa` |
| `cs` | T | `{0} mililitrů` |
| `hu` | T | `{0} milliliter` |
| `tr` | T | `{0} mililitre` |
| `ar` | T | `{0} مليلتر` |
| `hi` | T | `{0} मिलीलीटर` |
| `th` | T | `{0} มิลลิลิตร` |
| `vi` | T | `{0} mililít` |
| `id` | T | `{0} mililiter` |
| `ms` | T | `{0} mililiter` |
| `he` | T | `{0} מיליליטר` |
| `el` | T | `{0} χιλιοστόλιτρα` |
| `uk` | T | `{0} мілілітра` |
| `ro` | T | `{0} de mililitri` |
| `und` | root | `{0} mL` |

### `mass-gram`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} grams` |
| `en-GB` | =en | `{0} grams` |
| `en-AU` | =en | `{0} grams` |
| `en-CA` | =en | `{0} grams` |
| `en-IN` | =en | `{0} grams` |
| `es` | T | `{0} gramos` |
| `es-MX` | T | `{0} gramos` |
| `es-US` | T | `{0} gramos` |
| `fr` | T | `{0} grammes` |
| `fr-CA` | T | `{0} grammes` |
| `de` | T | `{0} Gramm` |
| `de-AT` | T | `{0} Gramm` |
| `it` | T | `{0} grammi` |
| `pt` | T | `{0} gramas` |
| `nl` | T | `{0} gram` |
| `ja` | T | `{0} グラム` |
| `zh` | T | `{0}克` |
| `zh-Hant` | T | `{0} 克` |
| `ko` | T | `{0}그램` |
| `ru` | T | `{0} грамма` |
| `pl` | T | `{0} grama` |
| `sv` | T | `{0} gram` |
| `da` | T | `{0} gram` |
| `nb` | T | `{0} gram` |
| `fi` | T | `{0} grammaa` |
| `cs` | T | `{0} gramů` |
| `hu` | T | `{0} gramm` |
| `tr` | T | `{0} gram` |
| `ar` | T | `{0} غرام` |
| `hi` | T | `{0} ग्राम` |
| `th` | T | `{0} กรัม` |
| `vi` | T | `{0} gam` |
| `id` | T | `{0} gram` |
| `ms` | T | `{0} gram` |
| `he` | T | `{0} גרם` |
| `el` | T | `{0} γραμμάρια` |
| `uk` | T | `{0} грама` |
| `ro` | T | `{0} de grame` |
| `und` | root | `{0} g` |

### `mass-kilogram`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} kilograms` |
| `en-GB` | =en | `{0} kilograms` |
| `en-AU` | =en | `{0} kilograms` |
| `en-CA` | =en | `{0} kilograms` |
| `en-IN` | =en | `{0} kilograms` |
| `es` | T | `{0} kilogramos` |
| `es-MX` | T | `{0} kilogramos` |
| `es-US` | T | `{0} kilogramos` |
| `fr` | T | `{0} kilogrammes` |
| `fr-CA` | T | `{0} kilogrammes` |
| `de` | T | `{0} Kilogramm` |
| `de-AT` | T | `{0} Kilogramm` |
| `it` | T | `{0} chilogrammi` |
| `pt` | T | `{0} quilogramas` |
| `nl` | T | `{0} kilogram` |
| `ja` | T | `{0} キログラム` |
| `zh` | T | `{0}千克` |
| `zh-Hant` | T | `{0} 公斤` |
| `ko` | T | `{0}킬로그램` |
| `ru` | T | `{0} килограмма` |
| `pl` | T | `{0} kilograma` |
| `sv` | T | `{0} kilogram` |
| `da` | T | `{0} kilogram` |
| `nb` | T | `{0} kilogram` |
| `fi` | T | `{0} kilogrammaa` |
| `cs` | T | `{0} kilogramů` |
| `hu` | T | `{0} kilogramm` |
| `tr` | T | `{0} kilogram` |
| `ar` | T | `{0} كيلوغرام` |
| `hi` | T | `{0} किलोग्राम` |
| `th` | T | `{0} กิโลกรัม` |
| `vi` | T | `{0} kilôgam` |
| `id` | T | `{0} kilogram` |
| `ms` | T | `{0} kilogram` |
| `he` | T | `{0} קילוגרם` |
| `el` | T | `{0} χιλιόγραμμα` |
| `uk` | T | `{0} кілограма` |
| `ro` | T | `{0} de kilograme` |
| `und` | root | `{0} kg` |

### `mass-ounce`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} ounces` |
| `en-GB` | =en | `{0} ounces` |
| `en-AU` | =en | `{0} ounces` |
| `en-CA` | =en | `{0} ounces` |
| `en-IN` | =en | `{0} ounces` |
| `es` | T | `{0} onzas` |
| `es-MX` | T | `{0} onzas` |
| `es-US` | T | `{0} onzas` |
| `fr` | T | `{0} onces` |
| `fr-CA` | T | `{0} onces` |
| `de` | T | `{0} Unzen` |
| `de-AT` | T | `{0} Unzen` |
| `it` | T | `{0} once` |
| `pt` | T | `{0} onças` |
| `nl` | T | `{0} ounce` |
| `ja` | T | `{0} オンス` |
| `zh` | T | `{0}盎司` |
| `zh-Hant` | T | `{0} 盎司` |
| `ko` | T | `{0}온스` |
| `ru` | T | `{0} унции` |
| `pl` | T | `{0} uncji` |
| `sv` | T | `{0} uns` |
| `da` | =en | `{0} ounces` |
| `nb` | T | `{0} unser` |
| `fi` | T | `{0} unssia` |
| `cs` | T | `{0} uncí` |
| `hu` | T | `{0} uncia` |
| `tr` | T | `{0} ons` |
| `ar` | T | `{0} أونصة` |
| `hi` | T | `{0} औंस` |
| `th` | T | `{0} ออนซ์` |
| `vi` | T | `{0} aoxơ` |
| `id` | T | `{0} ounce` |
| `ms` | T | `{0} auns` |
| `he` | T | `{0} אונקיות` |
| `el` | T | `{0} ουγγιές` |
| `uk` | T | `{0} унції` |
| `ro` | T | `{0} de uncii` |
| `und` | root | `{0} oz` |

### `mass-pound`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} pounds` |
| `en-GB` | =en | `{0} pounds` |
| `en-AU` | =en | `{0} pounds` |
| `en-CA` | =en | `{0} pounds` |
| `en-IN` | =en | `{0} pounds` |
| `es` | T | `{0} libras` |
| `es-MX` | T | `{0} libras` |
| `es-US` | T | `{0} libras` |
| `fr` | T | `{0} livres` |
| `fr-CA` | T | `{0} livres` |
| `de` | T | `{0} Pfund` |
| `de-AT` | T | `{0} Pfund` |
| `it` | T | `{0} libbre` |
| `pt` | T | `{0} libras` |
| `nl` | T | `{0} pound` |
| `ja` | T | `{0} ポンド` |
| `zh` | T | `{0}磅` |
| `zh-Hant` | T | `{0} 磅` |
| `ko` | T | `{0}파운드` |
| `ru` | T | `{0} фунта` |
| `pl` | T | `{0} funta` |
| `sv` | T | `{0} pund` |
| `da` | T | `{0} pund` |
| `nb` | T | `{0} pund` |
| `fi` | T | `{0} paunaa` |
| `cs` | T | `{0} liber` |
| `hu` | T | `{0} font` |
| `tr` | T | `{0} libre` |
| `ar` | T | `{0} رطل` |
| `hi` | T | `{0} पौंड` |
| `th` | T | `{0} ปอนด์` |
| `vi` | T | `{0} pao` |
| `id` | T | `{0} pound` |
| `ms` | T | `{0} paun` |
| `he` | T | `{0} פאונד` |
| `el` | T | `{0} λίβρες` |
| `uk` | T | `{0} фунта` |
| `ro` | T | `{0} de livre` |
| `und` | root | `{0} lb` |

### `mass-stone`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} stones` |
| `en-GB` | T | `{0} stone` |
| `en-AU` | T | `{0} stone` |
| `en-CA` | T | `{0} stone` |
| `en-IN` | T | `{0} stone` |
| `es` | =en | `{0} stones` |
| `es-MX` | =en | `{0} stones` |
| `es-US` | =en | `{0} stones` |
| `fr` | =en | `{0} stones` |
| `fr-CA` | T | `{0} stone` |
| `de` | T | `{0} Stones` |
| `de-AT` | T | `{0} Stones` |
| `it` | T | `{0} stone` |
| `pt` | =en | `{0} stones` |
| `nl` | T | `{0} stone` |
| `ja` | T | `{0} ストーン` |
| `zh` | T | `{0}英石` |
| `zh-Hant` | T | `{0} 英石` |
| `ko` | T | `{0}스톤` |
| `ru` | T | `{0} стоуна` |
| `pl` | T | `{0} kamienia` |
| `sv` | T | `{0} engelska stenar` |
| `da` | T | `{0} stone` |
| `nb` | T | `{0} stone` |
| `fi` | T | `{0} stonea` |
| `cs` | T | `{0} kamenů` |
| `hu` | T | `{0} stone` |
| `tr` | T | `{0} stone` |
| `ar` | T | `{0} ستون` |
| `hi` | T | `{0} 14 पौंड का बट्टा` |
| `th` | T | `{0} สโตน` |
| `vi` | **R** | `{0} st` |
| `id` | T | `{0} stone` |
| `ms` | T | `{0} stone` |
| `he` | T | `{0} סטון` |
| `el` | T | `{0} stone` |
| `uk` | T | `{0} стоуна` |
| `ro` | T | `{0} de stone` |
| `und` | root | `{0} st` |

### `temperature-celsius`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} degrees Celsius` |
| `en-GB` | =en | `{0} degrees Celsius` |
| `en-AU` | =en | `{0} degrees Celsius` |
| `en-CA` | =en | `{0} degrees Celsius` |
| `en-IN` | =en | `{0} degrees Celsius` |
| `es` | T | `{0} grados Celsius` |
| `es-MX` | T | `{0} grados Celsius` |
| `es-US` | T | `{0} grados Celsius` |
| `fr` | T | `{0} degrés Celsius` |
| `fr-CA` | T | `{0} degrés Celsius` |
| `de` | T | `{0} Grad Celsius` |
| `de-AT` | T | `{0} Grad Celsius` |
| `it` | T | `{0} gradi Celsius` |
| `pt` | T | `{0} graus Celsius` |
| `nl` | T | `{0} graden Celsius` |
| `ja` | T | `摂氏 {0} 度` |
| `zh` | T | `{0}摄氏度` |
| `zh-Hant` | T | `攝氏 {0} 度` |
| `ko` | T | `섭씨 {0}도` |
| `ru` | T | `{0} градуса Цельсия` |
| `pl` | T | `{0} stopnia Celsjusza` |
| `sv` | T | `{0} grader Celsius` |
| `da` | T | `{0} grader celsius` |
| `nb` | T | `{0} grader celsius` |
| `fi` | T | `{0} celsiusastetta` |
| `cs` | T | `{0} stupňů Celsia` |
| `hu` | T | `{0} Celsius-fok` |
| `tr` | T | `{0} santigrat derece` |
| `ar` | T | `{0} درجة مئوية` |
| `hi` | T | `{0} डिग्री सेल्सियस` |
| `th` | T | `{0} องศาเซลเซียส` |
| `vi` | T | `{0} độ C` |
| `id` | T | `{0} derajat Celsius` |
| `ms` | T | `{0} darjah Celsius` |
| `he` | T | `{0} מעלות צלזיוס` |
| `el` | T | `{0} βαθμοί Κελσίου` |
| `uk` | T | `{0} градуса Цельсія` |
| `ro` | T | `{0} de grade Celsius` |
| `und` | root | `{0}°C` |

### `temperature-fahrenheit`

| locale | status | long `count=other` pattern |
| --- | --- | --- |
| `en` | ref | `{0} degrees Fahrenheit` |
| `en-GB` | =en | `{0} degrees Fahrenheit` |
| `en-AU` | =en | `{0} degrees Fahrenheit` |
| `en-CA` | =en | `{0} degrees Fahrenheit` |
| `en-IN` | =en | `{0} degrees Fahrenheit` |
| `es` | T | `{0} grados Fahrenheit` |
| `es-MX` | T | `{0} grados Fahrenheit` |
| `es-US` | T | `{0} grados Fahrenheit` |
| `fr` | T | `{0} degrés Fahrenheit` |
| `fr-CA` | T | `{0} degrés Fahrenheit` |
| `de` | T | `{0} Grad Fahrenheit` |
| `de-AT` | T | `{0} Grad Fahrenheit` |
| `it` | T | `{0} gradi Fahrenheit` |
| `pt` | T | `{0} graus Fahrenheit` |
| `nl` | T | `{0} graden Fahrenheit` |
| `ja` | T | `華氏 {0} 度` |
| `zh` | T | `{0}华氏度` |
| `zh-Hant` | T | `華氏 {0} 度` |
| `ko` | T | `화씨 {0}도` |
| `ru` | T | `{0} градуса Фаренгейта` |
| `pl` | T | `{0} stopnia Fahrenheita` |
| `sv` | T | `{0} grader Fahrenheit` |
| `da` | T | `{0} grader fahrenheit` |
| `nb` | T | `{0} grader fahrenheit` |
| `fi` | T | `{0} fahrenheitastetta` |
| `cs` | T | `{0} stupňů Fahrenheita` |
| `hu` | T | `{0} Fahrenheit-fok` |
| `tr` | T | `{0} fahrenhayt derece` |
| `ar` | T | `{0} درجة فهرنهايت` |
| `hi` | T | `{0} डिग्री फ़ेरनहाइट` |
| `th` | T | `{0} องศาฟาเรนไฮต์` |
| `vi` | T | `{0} độ F` |
| `id` | T | `{0} derajat Fahrenheit` |
| `ms` | T | `{0} darjah Fahrenheit` |
| `he` | T | `{0} מעלות פרנהייט` |
| `el` | T | `{0} βαθμοί Φαρενάιτ` |
| `uk` | T | `{0} градуса Фаренгейта` |
| `ro` | T | `{0} de grade Fahrenheit` |
| `und` | root | `{0}°F` |

## Appendix B — CLDR definitions of the cooking units, verbatim

Constants referenced below, from `unitConstants` in `units.xml`:

```xml
<unitConstant constant="lb_to_kg" value="0.45359237"/>
<unitConstant constant="gal_to_m3" value="231*in3_to_m3"/>
<unitConstant constant="gal_imp_to_m3" value="0.00454609"/>
<unitConstant constant="sho_to_m3" value="2401/1331*1000" description="https://en.wikipedia.org/wiki/Japanese_units_of_measurement"/>
```

`convertUnit` rows for the mass and volume units relevant to recipes, verbatim and in file order:

```xml
<convertUnit source='grain' baseUnit='kilogram' factor='lb_to_kg/7000' systems="ussystem uksystem"/>
<convertUnit source='gram' baseUnit='kilogram' factor='0.001' systems="si metric prefixable"/>
<convertUnit source='ounce' baseUnit='kilogram' factor='lb_to_kg/16' systems="ussystem uksystem"/>
<convertUnit source='pound' baseUnit='kilogram' factor='lb_to_kg' systems="ussystem uksystem"/>
<convertUnit source='kilogram' baseUnit='kilogram' systems="si metric"/>
<convertUnit source='stone' baseUnit='kilogram' factor='lb_to_kg*14' systems="uksystem"/>
<convertUnit source='fun' baseUnit='kilogram' factor='1*3/8000' systems="jpsystem"/>
<convertUnit source='drop' baseUnit='cubic-meter' factor='gal_to_m3/128*576' systems="ussystem"/>
<convertUnit source='pinch' baseUnit='cubic-meter' factor='gal_to_m3/128*128' systems="ussystem"/>
<convertUnit source='dessert-spoon' baseUnit='cubic-meter' factor='gal_to_m3/16*128' systems="ussystem"/>
<convertUnit source='dessert-spoon-imperial' baseUnit='cubic-meter' factor='gal_imp_to_m3/16*128' systems="uksystem"/>
<convertUnit source='dram' baseUnit='cubic-meter' factor='gal_to_m3/128*8' systems="ussystem"/>
<convertUnit source='teaspoon' baseUnit='cubic-meter' factor='gal_to_m3/16*48' systems="ussystem"/>
<convertUnit source='tablespoon' baseUnit='cubic-meter' factor='gal_to_m3/256' systems="ussystem"/>
<convertUnit source='fluid-ounce-metric' baseUnit='cubic-meter' factor='0.03*0.001' systems="metric_adjacent"/>
<convertUnit source='fluid-ounce-imperial' baseUnit='cubic-meter' factor='gal_imp_to_m3/160' systems="uksystem"/>
<convertUnit source='fluid-ounce' baseUnit='cubic-meter' factor='gal_to_m3/128' systems="ussystem"/>
<convertUnit source='jigger' baseUnit='cubic-meter' factor='gal_to_m3*3/128*2' systems="ussystem"/>
<convertUnit source='cup' baseUnit='cubic-meter' factor='gal_to_m3/16' systems="ussystem"/>
<convertUnit source='cup-imperial' baseUnit='cubic-meter' factor='gal_imp_to_m3/16' systems="uksystem"/>
<convertUnit source='cup-metric' baseUnit='cubic-meter' factor='0.00025'  systems="metric_adjacent"/>
<convertUnit source='pint' baseUnit='cubic-meter' factor='gal_to_m3/8' systems="ussystem"/>
<convertUnit source='pint-metric' baseUnit='cubic-meter' factor='0.0005' systems="metric_adjacent"/>
<convertUnit source='pint-imperial' baseUnit='cubic-meter' factor='gal_imp_to_m3/8' systems="uksystem"/>
<convertUnit source='quart' baseUnit='cubic-meter' factor='gal_to_m3/4' systems="ussystem"/>
<convertUnit source='liter' baseUnit='cubic-meter' factor='0.001' systems="si_acceptable metric prefixable"/>
<convertUnit source='quart-imperial' baseUnit='cubic-meter' factor='gal_imp_to_m3/4' systems="uksystem"/>
<convertUnit source='gallon' baseUnit='cubic-meter' factor='gal_to_m3' systems="ussystem"/>
<convertUnit source='gallon-imperial' baseUnit='cubic-meter' factor='gal_imp_to_m3' systems="uksystem"/>
<convertUnit source='kosaji' baseUnit='cubic-meter' factor='1*5*0.000001' systems="jpsystem"/>
<convertUnit source='osaji' baseUnit='cubic-meter' factor='1*15*0.000001' systems="jpsystem"/>
<convertUnit source='cup-jp' baseUnit='cubic-meter' factor='1*100*0.000001' systems="jpsystem"/>
<convertUnit source='shaku' baseUnit='cubic-meter' factor='sho_to_m3/100' systems="jpsystem"/>
<convertUnit source='sai' baseUnit='cubic-meter' factor='sho_to_m3/1000' systems="jpsystem"/>
<convertUnit source='to-jp' baseUnit='cubic-meter' factor='sho_to_m3*10' systems="jpsystem"/>
<convertUnit source='koku' baseUnit='cubic-meter' factor='sho_to_m3*100' systems="jpsystem"/>
```

Japanese-system long `count=other` patterns in `ja`, for the ids `outstanding.md` records as
missing from CLDR:

| unit id | `ja` pattern | `en` pattern |
| --- | --- | --- |
| `volume-kosaji` | `小さじ {0}` | `{0} kosaji [JP]` |
| `volume-osaji` | `大さじ {0}` | `{0} osaji [JP]` |
| `volume-cup-jp` | `{0} カップ` | `{0} cup [JP]` |
| `volume-shaku` | `{0} 勺` | `{0} shaku [volume, JP]` |
| `volume-sai` | `{0} 才` | `{0} sai [JP]` |
| `volume-to-jp` | `{0} 斗` | `{0} to [JP]` |
| `volume-koku` | `{0} 石` | `{0} koku [JP]` |
| `mass-fun` | `{0} 分 - 質量` | `{0} fun [JP]` |
