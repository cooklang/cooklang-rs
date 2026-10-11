# Cooklang units

The vocabulary for units and unit conversion in this crate: what a unit is, how conversion relates units, and the layers above conversion (naming, unit systems, best-unit selection).

## Language

### Conversion

**Physical quantity**:
The kind of thing a unit measures: volume, mass, length, time or temperature. Only units of the same physical quantity convert into each other.
_Avoid_: Dimension, quantity (a `Quantity` is a value with a unit)

**Unit**:
A fixed amount of a physical quantity that values are expressed in, such as the milliliter or the US cup. A unit is defined by what it measures, not by what it is called.

**Reference unit**:
The unit of a physical quantity that every other unit of it is defined against: liter, gram, meter, second and degree Celsius.
_Avoid_: Base unit, ratio-1 unit, canonical unit

**Mixed quantities**:
An attempt to convert between units of different physical quantities, such as milliliters to grams.
_Avoid_: Incompatible units

**Temperature point**:
A position on a temperature scale, such as an oven setting. Conversion of temperatures always means converting points.
_Avoid_: Temperature delta, temperature difference (a rise of 10 °C is 18 °F, not 50 °F; deltas are not converted)

### Naming

**Identifier**:
The stable, locale-independent key of a unit: lowercase words joined by `-`, with any regional or system variant first, such as `us cup` or `metric cup`. It is what aliases and translations point at. Its spaced form (`metric cup`) resolves as input in every locale.
_Avoid_: Symbol, name, key

**Symbol**:
The string a locale writes for a unit after a conversion, such as `ml` or `tbsp`. A locale has at most one symbol per unit, and it also resolves as input.
_Avoid_: Identifier

**Written form**:
What a conversion writes for a unit: the locale's symbol, or the identifier's spaced form when the locale has none.

**Alias**:
Any other text that resolves to a unit, chosen per locale. The same alias can resolve to different units in different locales: `cup` can mean a US cup or a Japanese cup.

**Locale**:
How one language writes units: a symbol and aliases for the units it names. A locale may leave a unit unnamed. It covers unit naming only, not number formatting.

**Adjudication**:
Deciding what a word for a regionally variable unit means in one locale: that a Swedish `kopp` is a 150 ml coffee cup and not the US cup, so it must not resolve. The act, not a mechanism — a locale resolves the names it lists and nothing else.
_Avoid_: Mapping, override, review (adjudication decides what a word means; review confirms the locale writes what recipes write)

**Seeded locale**:
A locale whose names were taken from CLDR once and hand-edited from then on. Its file is the whole truth for that locale: nothing is merged over it, and nothing re-reads CLDR.
_Avoid_: Vendored locale

**Refusal**:
A name a locale does not resolve, written down with the reason. Absence is what makes it unresolvable, so a refusal is a record rather than a mechanism, and it is kept as prose: the reason it was excluded, and the list a reviewer still has to judge. Excluding a name for one unit is how it is freed for another: Finnish does not name the millimetre `mm` because `mm` is its 1 ml spice measure.
_Avoid_: Rejection (suggests a mechanism)

**Review**:
The confirmation, by a speaker of the language, that what a locale writes is what recipes in that language write. It is not adjudication: a locale can have a verdict on every ambiguous word and still be unreviewed. A locale is refused until it has one.
_Avoid_: Adjudication, translation

**Legacy locale**:
The locale whose symbols and aliases are exactly the names `units.toml` gives the bundled units. It names nothing else.

### Above conversion

**Unit system**:
A family of units a reader prefers: metric, US customary or imperial. It is a display preference, not a property of a unit; a metric teaspoon belongs to no system cleanly.
_Avoid_: Locale

**US customary**:
The United States kitchen units: the 236.59 ml cup, the 473 ml pint, the 29.57 ml fluid ounce and their spoons. The legacy `System::Imperial` means US customary and is kept for compatibility.
_Avoid_: Imperial (for US units), American

**Imperial**:
The British statutory units of the Weights and Measures Act 1985: the 568 ml pint, the 28.41 ml fluid ounce, the quart and the gallon.
_Avoid_: UK units, British units

**Official system**:
The unit system a region's law prescribes, which decides the units a conversion writes for a reader there. Only the United States is US customary; every other region, including the United Kingdom, is metric.

**Best unit**:
The unit within a unit system that a value displays best in, such as 1.5 l instead of 1500 ml.

**Base unit**:
The smallest best unit of a physical quantity in a unit system, such as ml for metric volume or tsp for US customary volume.
_Avoid_: Reference unit
