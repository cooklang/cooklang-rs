# Measurement precision in cooking, and marking a quantity exact

Researched 2026-10-05 for ADR 1 follow-up items 4 (fraction display) and 5 (equality tolerance).
Written to answer two questions:

1. What rounding error may `convert_v2` introduce when it converts, scales or picks a display
   unit, before a cook would notice or a recipe would fail?
2. Is there a defensible syntax for marking an ingredient or quantity **exempt from rounding**, and
   what floor should replace the tolerance for it?

**Source labels** (as in `cooking-units.md`):
- `[n]` points to a numbered reference at the end.
- **(2°)** the claim rests only on a secondary source: encyclopedia, blog, forum, retailer or
  recipe site.
- Unmarked sources are primary: legislation, a regulator, a standards body, a manufacturer's own
  label, or an author of record in the field.
- **(calc.)** marks a value computed here from an exact ratio, not printed in the source.
- **(observed)** marks a value read off this repository by running it, not from a source.

---

## 1. Three different things get called "precision"

Most of the confusion in this area comes from conflating three quantities that differ by orders of
magnitude. Keeping them apart is most of the answer.

| | What it is | Typical magnitude | Axis |
|---|---|---|---|
| **Measurement noise** | How far the cook's actual dose lands from the written number | 0.02% to 25%, set by the instrument and by the mass actually weighed (§2) | Absolute (a scale division) or relative (volumetric technique) |
| **Recipe tolerance band** | How far the dose may drift before the dish changes | ±10% typical, ±5% for structure, 0% at a legal ceiling | Relative |
| **Rounding budget** | How much error *we* may add in converting and displaying | should sit an order of magnitude below the band | Relative |

A rounding budget is not a measurement requirement, and the parser's obligation is narrower than
either: **do not consume any of the band the recipe author was using.**

It is tempting to add that the kitchen is so much worse than any budget we would pick that the
budget hardly matters. That is true of a cook with a 1 g scale and false of a cook with a 1 mg one,
and §2 shows the crossover is not where intuition puts it: at the masses people actually weigh, on
instruments that cost about 30 euro, measurement noise falls to 0.02–0.2% and **our rounding becomes
the dominant error term**. The budget matters most for exactly the ingredients where it is tightest.

A single relative tolerance is still the right shape for a *budget*, and the wrong shape for a
measurement model (§2) or for a legal ceiling (§4).

## 2. What the kitchen can actually deliver (absolute, not relative)

Legal metrology expresses weighing tolerance in **scale divisions** `d`, not in percent. For a
Class III instrument NIST Handbook 44 allows 1 `d` for loads of 0–500 `d`, 2 `d` for 501–2 000 `d`,
3 `d` for 2 001–4 000 `d` and 5 `d` above that [1]. `d` is the displayed resolution; the accuracy
class is defined against the verification division `e` [2].

What is on sale, at ordinary kitchen retail rather than from a laboratory supplier:

| Class | `d` | Max | Price |
|---|---|---|---|
| Household kitchen scale | 1 g | 2–5 kg | — |
| Precision kitchen scale | 0.01 g | 200 g | NOD Precisionsvåg, 329 SEK ≈ €29 at Bagaren och Kocken [28] |
| Milligram scale | 0.001 g | 50 g | USD 15–46 (Walmart, eBay, American Weigh Gemini-50) (2°) [29] |

**The relative error is set by the mass on the pan, not by the dose the recipe writes**, and those
are not the same number. A cook who needs 0.2 g of gellan does not weigh 0.2 g. They either scale
the batch until the dose is weighable — 0.04% of 6.25 kg of liquid is 2.5 g (calc.), a 12.5× scale-up
from a 500 g batch — or weigh a practical mass and disperse it as a stock. So the row that describes
real practice is the practical mass, not the written one:

| Mass weighed | `d` = 1 g | `d` = 0.1 g | `d` = 0.01 g | `d` = 0.001 g |
|---|---|---|---|---|
| 500 g flour | 0.1% | 0.01% | 0.001% | — |
| 10 g salt (2% of 500 g flour) | **5%** | 0.5% | 0.05% | 0.005% |
| 5.66 g cure #1 (§4) | **8.8%** | 0.88% | 0.088% | 0.009% |
| 2.5 g gellan, practical mass | **20%** | 2% | 0.2% | 0.02% |
| 0.2 g gellan, as written for 500 g | — | **25%** | 2.5% | 0.25% |

(calc. throughout, as the half-division the display rounds away. NIST Handbook 44 permits a further
1 `d` of instrument error on top of that for Class III [1], so these are the optimistic figures.)

Three conclusions, and all three constrain the design:

- **0.5% relative is reachable, and reachable cheaply.** At a practical 2.5 g it needs only a
  0.01 g scale (0.2%); a 1 mg scale gives 0.02%. Even a literal 0.2 g dose comes in at 0.25% on a
  1 mg scale. So 0.5% for additives is a *measurement-grounded* figure, not merely a policy choice.
- **At those masses our rounding is the dominant error term.** A 1% rounding budget applied to a
  dose a cook weighed to 0.02% throws away a factor of fifty. This is the inversion that matters:
  for precision work the parser, not the scale, is the limiting instrument, which is the general
  form of the argument §4 makes for legal ceilings.
- **Scaling is how a small dose becomes weighable, so rounding during scaling is rounding at
  precisely the wrong moment.** The 12.5× scale-up above is the cook's own precision technique.
  Rounding the scaled result corrupts the ratio at the one step taken to protect it — see §10
  question 5.

Cooking Issues sets the floor for hydrocolloid work at the 0.01 g class: "A scale accurate to
0.1 g is essential; in some cases, it is helpful to have a scale accurate to .01 g." [3] That
predates cheap milligram scales and should be read as a minimum, not a ceiling.

### Volumetric measurement is far worse

King Arthur's own figure for a cup of all-purpose flour is 4¼ oz / 120 g, "correct if you use the
stir, then lightly spoon and level method" [4]. Scoop-and-sweep on the same flour gives about 140 g
(2°) [5] — a **16.7%** spread (calc.) on the identical written quantity, with no error on the cook's
part. Modernist Cuisine draws the engineering conclusion: "Because volume measurements are often
rounded to the nearest spoon or cup, you should not multiply or divide volumes when scaling a recipe
up or down" [6].

This matters to us for one reason: a cup, a tablespoon and a teaspoon are *already* ±8% containers.
Rounding a converted value to a nice fraction of a cup adds nothing to the error a cup carries. A
written `cup` is therefore evidence that the author was not working to a tolerance at all, and is
the safest place to round aggressively.

## 3. Recipe tolerance bands by ingredient class

| Class | Stated band | Relative width | Source |
|---|---|---|---|
| Salt in bread | 1.8–2.2% of flour, centre 2% | **±10%** | [7] (2°), [8] (2°) |
| Instant yeast | 0.3% → 2.45–3.00 h bulk; 1.0% → 1.00–1.15 h | 3× dose ≈ 2.5× time, so roughly linear; no band given | [9] (2°) |
| Xanthan gum | 0.2–1% general; 0.10–0.25% vinaigrette; 0.05–0.20% sauce | 2.5× within one application | [3], [10] (2°) |
| Agar | 0.5–1% | 2× | [3] |
| Gellan | **0.04%–1%** | 25× | [3] |
| Sodium alginate | 0.5–1% | 2× | [3] |
| Gelatin | 1.0–1.5% | 1.5× | [3] |
| Carrageenan | 0.75–1% water, 0.35–0.5% milk | 1.3× | [3] |
| Sodium citrate (cheese sauce) | 4 g per 100 g cheese + 93 g liquid; 2–3% of total | — | [6], [11] (2°) |
| Sodium citrate (sequestrant) | 0.07–0.4%, by water hardness | 5.7× | [11] (2°) |
| Fermented sausage salt | 2.5%, "generally 25–30 g per kg" | ±18% | [12] (2°), [13] (2°) |
| Sous-vide water/core temperature | thermometer "accurate to 0.1 °C" recommended | 0.1 °C on 54.4 °C = **0.18%** | [14], [15] |
| Nitrite in cured meat | a **ceiling**, not a band | see §4 | [16], [17], [18] |

Two readings of this table:

- **The published bands are wide — ±10% or a 2–25× range — not 1%.** The hydrocolloid ranges are
  ranges of *intent* (thin sauce vs. thick paste), not tolerances, but even the one genuine
  tolerance band found, bread salt at 1.8–2.2%, is ±10%. Cooking Issues says the quiet part out
  loud about a 2% error in a hydrocolloid mix: "this mix is technically incorrect (this is a 1.96%
  solution), the effect of the small additional hydrocolloid is negligible" [3].
- **Temperature is the one quantity with a stated precision in the sources**, and it is absolute
  (0.1 °C), not relative. 0.18% relative at 54.4 °C, but 0.05% at 180 °C and meaningless as a
  relative figure at all — a 1% relative tolerance on a Celsius *point* is nonsense, because the
  scale's zero is arbitrary. Temperature points must not be given a relative tolerance.

## 4. Hard ceilings: where the question is rounding *direction*, not size

Curing salt is the case the brief has in mind, and it is not a tolerance problem. It is a
**ceiling**: any rounding upward is a compliance failure at any tolerance.

**United States.** 9 CFR 424.21(c) sets ingoing limits as mass per 100 lb of meat, not as ppm [16]:

| Use | Nitrite limit | As ppm (calc.) |
|---|---|---|
| Comminuted | ¼ oz / 100 lb | **156.25** |
| Dry cure | 1 oz / 100 lb | 625 |
| Pickle, 10% pump | 2 lb / 100 gal | — |

and a finished-product cap: the use of nitrites, nitrates or a combination "shall not result in
more" than 200 ppm of nitrite calculated as sodium nitrite [16]. The familiar "156 ppm" figure is
itself a rounded statement of exactly 156.25 ppm (calc.) — the regulation's own canonical number has
already been rounded once, which is a fair warning about rounding it again.

**European Union.** Regulation (EU) 2023/2108 replaced the 150 mg/kg added-amount limit in
Regulation (EC) 1333/2008 Annex II. From **9 October 2025** the maximum added amount is 80 mg/kg
expressed as nitrite ion for meat products in general (equivalent to 120 mg/kg as sodium nitrite),
55 mg/kg for sterilised meat products, and 100 or 105 mg/kg for named traditionally cured products;
new residual limits of 25, 45 or 50 mg/kg also apply [17] (2°). Both figures are in force as of this
writing.

**Why this needs an exactness marker, concretely.** Prague powder #1 is 6.25% sodium nitrite,
93.75% salt (2°) [18]. Hitting 156 ppm on 5 lb (2 268 g) of meat requires

```
2268 g × 156 ppm / 62500 = 5.661 g
```

(calc.; the 62 500 divisor is the 6.25% strength expressed in ppm) [19] (2°). The published
teaspoon-weight of the same powder ranges from 4.0 g to 6.4 g across sources (2°) [19][20][21],
giving 110–176 ppm (calc.) for the identical written instruction "1 level teaspoon per 5 lb". The
top of that range **exceeds the comminuted limit**. So:

- The volumetric form of this instruction cannot be made compliant, whatever we do. One source
  states the rule plainly: "The only mathematically safe way to calculate curing salt is by weight
  in grams" (2°) [19].
- A gravimetric `5.66 g` **is** compliant (155.97 ppm, calc.), and is destroyed by rounding up to
  `5.7 g` (157.1 ppm, calc.) or to `6 g` (165.3 ppm, calc.). 5.7 g already breaches 156.25.
- The FDA has issued warnings over home-curing nitrite incidents, including hospitalisations for
  methaemoglobinaemia (2°) [18][22].

The same shape, a one-sided legal ceiling on an additive expressed as a fraction of batch mass,
recurs for preservatives and processing aids generally. The design conclusion is that an exactness
marker must suppress rounding **in both directions**, because the author may be sitting just under a
ceiling (nitrite) or just over a floor (a minimum effective dose), and the markup cannot tell which.

## 5. Verdict on the two remembered figures

The brief recalled ~1% for ordinary home cooking and ~0.5% for curing salts and additives. Against
the evidence above:

- **1% as a rounding budget for ordinary cooking: supported, with room to spare.** The one real
  tolerance band found is ±10% (bread salt). 1% consumes a tenth of it. It is also far below
  volumetric measurement noise (16.7% for a cup of flour). As a *stated* figure in a source, 1% does
  not appear — no source found states a relative precision requirement for home cooking at all, so
  this is a defensible policy choice rather than a looked-up fact.
- **0.5% for additives: supported as a measurement figure.** At the masses people actually weigh it
  is comfortably reachable on a €29 scale and beaten tenfold by a €20 one (§2). It is a real
  requirement, not just a policy choice.
- **But 0.5% is still the wrong instrument for a legal ceiling**, because there the failure mode is
  direction, not magnitude: 5.66 g → 5.68 g is 0.35%, inside 0.5%, and still breaches 156.25 ppm.
  Any relative tolerance buys a factor; exemption is what the ceiling case needs.

**So the finding is four tiers, with the brief's two in the middle:**

| Tier | Budget | Applies to |
|---|---|---|
| Written in a volumetric container (cup, tbsp, tsp, pinch) | the container's own granularity — round to a usable fraction | the author stated no tolerance |
| Ordinary gravimetric and volumetric quantities | **1% relative** | the default |
| Additives weighed on a precision scale | **0.5% relative**, and arguably tighter — §2 says 0.02% is reachable | hydrocolloids, leaveners, cures, anything dosed as a fraction of batch mass |
| Marked exact | **no rounding**; only a float-noise floor (§9) | legal ceilings, and anything the author marked |

The awkward part is tiers 2 and 3: **the parser cannot tell an additive from a seasoning**, because
that is a fact about food, not about markup. Three ways out — classify ingredients (a hopeless and
unmaintainable list), require a marker for tier 3 (which collapses it into tier 4), or derive the
tier from the number as written, which is what §8 option C does and what §2's correction makes the
strongest of the three.

## 6. What this crate does today (observed)

`Converter::float_tolerance` returns the fraction-display `accuracy` when fractions are enabled for
a unit, otherwise `CONVERSION_EPSILON` = 1e-9 (`src/convert_legacy/mod.rs:202`, `:223`). The layering is
unit → quantity → system → `all` → default (`src/convert_legacy/mod.rs:269`). With `units.toml`'s
`[fractions]` block — `all = { enabled = true, accuracy = 0.01 }`, `metric = false`,
`imperial = { enabled = true, accuracy = 0.05 }` — `Converter::bundled()` therefore gives
(observed, by calling `float_tolerance` on each unit):

| Unit as written | Tolerance |
|---|---|
| no unit | 0.009999999776482582 |
| `g` `kg` `ml` `l` | 1e-9 |
| `tsp` `tbsp` `cup` `lb` `oz` `fl oz` | 0.05000000074505806 |
| `min` `C` `F` | 1e-9 |

Three things to carry into the design:

- **The spread is 5×10⁷.** `1 cup` equals anything within **5%** of a cup, while `250 ml` equals
  only what is within 1e-9. Two ways of writing the same volume get wildly different equality. This
  is ADR 1 item 5 made concrete.
- **The ragged values are an `f32` artefact.** `FractionsConfig::accuracy` is `f32`
  (`src/convert_legacy/units_file.rs:147`), so 0.01 widens to 0.009999999776482582 and 0.05 to
  0.05000000074505806. Whatever replaces this should hold the tolerance as `f64`.
- **The unitless tolerance is already 1%**, which happens to match the figure in the brief — but by
  way of `[fractions] all.accuracy`, i.e. as a display setting, not as a decision about equality.

## 7. Prior art for marking a quantity exact

**Metrology marks exactness on the number, not on the substance.** Three established notations, all
operating on digits [23]:

- **Overbar** on the last significant figure: 1̅300 has three significant figures, "precise to the
  nearest ten".
- **Underline**, the same idea: "1300" with the 3 underlined has two.
- **Trailing decimal point**: "1300." states that the trailing zeros *are* significant.

And the recommended alternatives, because none of the three is universal: change the prefix
(1300 g → 1.30 kg), use scientific notation, state it in words ("20,000 to 2 s.f."), or use
plus-minus ("20,000 ± 1%") [23]. The symbol **≈** communicates that a value has been rounded [24].

**No recipe markup format was found that marks a quantity as exact or exempt from rounding.** The
closest prior art is in tooling rather than format:

- A Go Cooklang library offers `PreciseMode` (29.5735 ml/oz) versus `BartenderMode` (30 ml/oz, with
  rounding to friendly values) — a **global mode**, not a per-quantity property (2°) [25].
- A recipe app's issue report describes exactly the failure this marker would prevent: fixed
  4-decimal rounding "collapses tiny recipe quantities to zero and compounds drift across edits",
  with the remedies discussed being more stored precision or **rounding only at display time, not
  at write time** (2°) [26].
- Cooklang's own spec has no precision concept; quantities are value + unit [27].

Two lessons. First, "round for display only, never in the stored value" is the structural fix and is
independent of any marker — a marker is for the display layer, which is where the information is
lost. Second, the metrology convention argues that exactness belongs to the **quantity**, not the
ingredient, which cuts against the brief's framing of "a precise ingredient". The same substance can
appear twice in one recipe with different needs (a weighed 5.66 g cure, and a `pinch` of the same
salt for seasoning), and an ingredient-level flag cannot express that.

Against that: Cooklang's existing precision-adjacent markers are all ingredient modifiers, a cook
reading a recipe thinks "the cure is the dangerous one", and the ingredient-level form is the one
that survives being referenced (`@&cure{}`) and aggregated into a shopping list. Both readings are
defensible and the choice is a decision for the repository owner, not a research finding.

## 8. Candidate syntaxes

What is free. The lexer produces `@ # ~ ? + / * & | % = { } ( ) . : > -`
(`src/lexer/mod.rs:110–151`). The modifier position before an ingredient name consumes `@` (recipe),
`&` (reference), `?` (optional), `+` (new) and `-` (hidden) (`src/parser/step.rs:169–178`,
`extensions.md`). `#` and `~` are component sigils; `|` is the alias separator; `%` is the unit
separator; `/` builds fractions; `=` appears inside intermediate-preparation references (`&(=2)`);
`.` `:` `>` serve metadata and comments.

**`*` is lexed as `TokenKind::Star` and never read by the parser** — `grep` for `T![*]` and
`TokenKind::Star` finds only the lexer's own definition and nothing in `src/parser/`. It is the only
unused single-character token.

| Option | Form | For | Against |
|---|---|---|---|
| **A. Ingredient modifier** | `@*sodium nitrite{5.66%g}` | `*` is genuinely free (verified); reads as a footnote/asterisk, which already connotes "attention"; fits the existing modifier mechanism and bitflags exactly; travels through references and the shopping list | Attaches to the substance, not the measurement — cannot distinguish a weighed dose from a pinch of the same thing; `*` is a strong emphasis marker in Markdown, where many of these recipes live |
| **B. Quantity marker** | `@sodium nitrite{*5.66%g}` | Matches metrology: exactness is a property of the number; per-occurrence; extends naturally to timers and inline quantities, which have no modifier position | New grammar in the quantity parser; placement next to `%` and fractions needs care; invisible in an ingredient list that shows only names |
| **C. Significant figures** | `@sodium nitrite{5.660%g}` | No new syntax at all; the standard convention [23]; expresses *how* exact, not just that it is | `Number::Regular` is an `f64` and does not record written digits (`src/quantity.rs:79`), and `Display` already rounds to 3 decimal places (`src/quantity.rs:60`) — so `5.660` and `5.66` are indistinguishable today. Needs the parser to carry a digit count. Also silent: a cook cannot see that it did anything |
| **D. Metadata list** | `>> [precise]: sodium nitrite, cure #1` | Uses the existing `[mode]`/`[duplicate]` special-key mechanism; one declaration covers every occurrence; zero step-text noise | Per-recipe, so it does not travel with a shared or referenced ingredient; duplicates the ingredient name as a string, which the alias and reference layers will fight |

A combination is also coherent: **A or B as the marker, plus C's digit count as the optional
refinement**, since they answer different questions ("do not round this" vs. "this is good to three
figures"). And note that **B generalises and A does not**: `~{5.66%min}` and the inline temperature
extension have quantities but no ingredient.

### Why option C does more work than it first appears

§5's tier problem is that the parser cannot know which ingredients are additives. Option C dissolves
it: take the budget to be **half a unit in the last digit the author wrote**, and every tier falls
out of the text with no classification and no list.

| As written | Half-ulp band | Implied budget | Tier it lands in |
|---|---|---|---|
| `1 cup` | 0.5–1.5 cup | 50% | self-rounding container |
| `2 tsp` | 1.5–2.5 tsp | 25% | self-rounding container |
| `250 ml` | 249.5–250.5 ml | 0.2% | ordinary |
| `2.5 g` | 2.45–2.55 g | 2% | ordinary |
| `5.66 g` | 5.655–5.665 g | 0.088% | additive |
| `5.661 g` | 5.6605–5.6615 g | 0.0088% | additive |

Three things to note. The nitrite case **is already protected**: 5.7 g lies outside `5.66 g`'s band,
so the breach in §4 cannot happen under this rule without any marker at all. The budgets it implies
track §2's achievable precision closely — `5.66 g` implies 0.088%, which is exactly what a 0.01 g
scale delivers at that mass — because an author writing three figures is an author who weighed to
three figures. And it is the standard convention [23], so it needs no teaching.

What it does not do, and what still needs a marker: forbid **conversion** into a container that
cannot express the value (§10 question 6), and pin the ratio under **scaling** (§10 question 5),
where the digit count of the input says nothing about the digits the output deserves. So C is a
complement to A or B, not a replacement — but it is the part that removes the tier guesswork.

Its cost is unchanged and real: `Number::Regular` is an `f64` that does not record written digits,
and `Display` already rounds to 3 decimal places, so the parser must start carrying a digit count
before any of this is available.

One further argument for B that is worth weighing: §5's top tier keys off the **unit as written**
(a `cup` is self-rounding). That is already a property of the quantity. Putting the exactness marker
in the same place keeps the whole precision policy on one object.

## 9. The floor for an exact quantity

The brief suggests 1e-8 "because float math is wonky". For the record on what the error actually is:

- `CONVERSION_EPSILON` is already **1e-9** (`src/convert_legacy/mod.rs:223`) and is what metric units get
  today (§6).
- ADR 1 records that `convert_v2` uses the legal definitions rather than `units.toml`'s rounded
  ratios, "largest difference about 1.2e-7 relative, for the teaspoon". So the *converter's own*
  disagreement with the legacy one is **~1.2e-7**, two orders of magnitude above 1e-9 and one
  order above 1e-8. Any floor below that will make the two converters disagree about equality.
- `f64` has 52 bits of mantissa, so one rounding is ~1.1e-16 relative; a chain of a dozen
  multiply/divide conversion steps stays below ~1e-15. **Float noise alone does not justify 1e-8**
  — it justifies something nearer 1e-14.

So 1e-8 is a reasonable round number but it is justified by *conversion-chain* error, not by float
representation, and it is still smaller than the 1.2e-7 definitional difference. Three defensible
choices, in increasing strictness: **1e-6** (comfortably above any conversion artefact, still
10 000× tighter than 1% and far tighter than any culinary band), **1e-8** (the brief's figure), or
**0** with exact comparison for quantities whose units are identical, falling back to a floor only
when a conversion happened. The third is the most honest — rounding should not be suppressed and
then quietly reintroduced — but it makes the tolerance depend on whether a conversion occurred,
which is a behaviour worth pinning in a test either way.

Separately, and regardless of the marker: §7's second lesson is that **the stored value should never
be rounded**; rounding belongs to display [26]. If that holds, the exactness marker only has to
reach the display layer, and the equality tolerance (item 5) stops depending on display
configuration — which is the whole point of ADR 1 item 5.

## 10. Open questions for the owner

1. **Ingredient-level or quantity-level?** §7 and §8 argue quantity-level (B); §8 lists the real
   costs. Only B covers timers and inline temperatures.
2. **Is the digit-count rule (§8 option C) wanted as the default budget**, in place of a fixed 1%?
   It removes the tier-2/tier-3 guesswork entirely and already protects the nitrite case, at the
   price of making `Number` carry the written digit count. If it is, the fixed tiers in §5 become a
   fallback for values with no digits to read, and the marker's job shrinks to scaling and
   conversion refusal.
3. **Is the top tier of §5 wanted at all** — does a quantity written `cup` get rounded to a usable
   fraction of a cup rather than to 1%? That is current behaviour by accident (the 5% imperial
   accuracy), and it may be right behaviour on purpose.
4. **Temperature points must be excluded from any relative tolerance** (§3). `units.toml` already
   disables fractions for `temperature`, so they currently get 1e-9 — an absolute band in kelvin
   (0.1 K, from [14][15]) is the defensible replacement, not a relative figure.
5. **Does an exact quantity refuse to scale**, or scale and then refuse to round? 5.66 g of cure for
   5 lb of meat is a *ratio*; doubling the recipe must double it exactly, not round it. This is the
   sharpest case, because §2 shows scaling is the cook's *own* precision technique — the way a 0.2 g
   dose becomes a weighable 2.5 g. Rounding a scaled exact quantity defeats the step the cook took
   to get precision in the first place. Note the digit-count rule cannot answer this on its own: the
   input's digits do not say how many the output deserves.
6. **Should an exact quantity refuse unit conversion** into a container that cannot express it? §4
   shows "1 tsp of cure #1" is unsafe in principle, not merely imprecise. Refusing, in the spirit of
   "explicit over implicit", would be consistent with the rest of `convert_v2`.

---

## References

1. NIST Handbook 44, Section 2.20 (Scales). Class III tolerance: 1 `d` for 0–500 `d`, 2 `d` for
   501–2 000 `d`, 3 `d` for 2 001–4 000 `d`, 5 `d` above.
   <https://www.nist.gov/document/2026-nist-handbook-44-section-220>
2. NIST Office of Weights and Measures, "What Is The Difference Between The Scale Division, d, And
   The Verification Scale Division, e". <https://www.nist.gov/document/e-vs-d-explanatory-new>
3. Dave Arnold / Cooking Issues, "Hydrocolloids Primer". Usage percentages for xanthan, agar,
   gellan, alginate, gelatin, carrageenan; "A scale accurate to 0.1 g is essential; in some cases,
   it is helpful to have a scale accurate to .01 g."
   <https://cookingissues.com/primers/hydrocolloids-primer/>
4. King Arthur Baking, "Recipe Success Guide". 1 cup AP flour = 4¼ oz / 120 g by stir–spoon–level.
   <https://www.kingarthurbaking.com/recipes/resources/recipe-success-guide>
5. Pastries Like a Pro, "The Frustrating Facts about Measuring Flour". Spoon-and-sweep ≈ 125 g,
   scoop-and-sweep ≈ 140 g. (2°) <https://pastrieslikeapro.com/frustrating-facts-measuring-flour/>
6. Modernist Cuisine, "How to Scale a Recipe". Baker's percentages; "Because volume measurements are
   often rounded to the nearest spoon or cup, you should not multiply or divide volumes when scaling
   a recipe up or down." <https://modernistcuisine.com/mc/how-to-scale-a-recipe/>
7. Weekend Bakery, "Salt in bread baking: how much and why". (2°)
   <https://www.weekendbakery.com/posts/salt-in-bread-baking-how-much-and-why/>
8. Matthew James Duffy, "Baker's Percentages Made Simple". Salt 1.8–2.2% of flour. (2°)
   <https://matthewjamesduffy.com/understanding-bakers-percentages/>
9. BAKERpedia, "Yeast Performance Testing"; King Arthur Baking, "The comprehensive guide to baking
   with yeast". Instant yeast 0.3% → 2.45–3.00 h bulk, 1.0% → 1.00–1.15 h. (2°)
   <https://bakerpedia.com/processes/yeast-performance-testing/>,
   <https://www.kingarthurbaking.com/blog/2023/05/10/guide-to-baking-with-yeast>
10. Cape Crystal Brands, "Xanthan Gum Conversion Table". Vinaigrette 0.10–0.25%, sauce 0.05–0.20%,
    soup 0.03–0.12%; "0.50% = 5 g per 1,000 g". (2°)
    <https://www.capecrystalbrands.com/blogs/cape-crystal-brands/xanthan-gum-conversion-table>
11. Molecular Recipes, "Sodium Citrate (Citras)". 2.0–3.0% of liquid + cheese; 0.07–0.4% as a
    sequestrant. (2°) <http://www.molecularrecipes.com/hydrocolloid-guide/sodium-citrate/>
12. Startercultures.eu, "Making dry cured sausages at home — step by step". Salt 2.5%; nitrite
    120 ppm; a 1 g scale resolution is insufficient for nitrite or starter culture. (2°)
    <https://startercultures.eu/making-dry-cured-sausages-at-home-step-by-step/>
13. Meats and Sausages, "Traditionally made fermented sausages". 25–30 g salt per kg. (2°)
    <https://www.meatsandsausages.com/sausage-types/fermented-sausage/traditional>
14. NSW Food Authority, "Sous vide food safety precautions". Temperatures checked with a
    tip-sensitive digital thermometer accurate to 0.1 °C; up to 6 h allowed to reach 54.4 °C.
    <https://www.foodauthority.nsw.gov.au/sites/default/files/_Documents/scienceandtechnical/sous_vide_food_safey_precautions.pdf>
15. Douglas Baldwin, "A Practical Guide to Sous Vide Cooking".
    <https://douglasbaldwin.com/sous-vide.html>
16. 9 CFR § 424.21(c) (FSIS). Nitrite: ¼ oz/100 lb comminuted, 1 oz/100 lb dry cure, 2 lb/100 gal
    pickle at 10% pump; nitrate: 2¾ oz/100 lb comminuted, 3½ oz/100 lb dry cure; combined use
    "shall not result in more" than 200 ppm nitrite as sodium nitrite in the finished product. Read
    via Cornell LII, an unofficial mirror of the CFR text.
    <https://www.law.cornell.edu/cfr/text/9/424.21>
17. Heraxfood, "Lower maximum levels of nitrites applicable from 9 October 2025". Regulation (EU)
    2023/2108 amending (EC) 1333/2008 Annex II: added amount 80 mg/kg as nitrite ion for meat
    products in general (120 mg/kg as sodium nitrite), 55 mg/kg sterilised, 100/105 mg/kg for named
    traditional products; residual limits 25/45/50 and 30/65/105 mg/kg. (2°) — EUR-Lex returned
    empty content for CELEX:32023R2108 and for the ELI URL on 2026-10-05, so the primary text was
    not read.
    <https://www.heraxfood.com/en/news/lower-maximum-levels-of-nitrites-applicable-from-9-october-2025->
18. Wikipedia, "Curing salt", and Prague powder #1 guides. 6.25% sodium nitrite / 93.75% NaCl; FDA
    warning after three home-curing incidents with hospitalisation for methaemoglobinaemia. (2°)
    <https://en.wikipedia.org/wiki/Curing_salt>
19. Calcady, "Meat Curing (Prague Powder #1) Calculator". `Prague Powder_g = (Total Mass_g × Target
    PPM) / 62500`; 5.661 g for 156 ppm on 2 268 g; "The only mathematically safe way to calculate
    curing salt is by weight in grams." (2°)
    <https://calcady.com/food/meat-curing-pink-salt-calculator/>
20. SmokingMeatForums, "Weight of cure #1". Teaspoon reported 4.0–5.5 g depending on brand. (2°)
    <https://www.smokingmeatforums.com/threads/weight-of-cure-1.286929/>
21. CooksInfo, "Prague Powder"; Preserve & Pickle, "What Is Curing Salt & Prague Powder". 1 level
    tsp ≈ 5.5–6.0 g per 5 lb. (2°) <https://www.cooksinfo.com/prague-powder>,
    <https://preserveandpickle.com/curing-salts-prague-powder/>
22. Destination BBQ, "How to Use Cure #1 Safely". (2°)
    <https://destination-bbq.com/cure-1-prague-powder/>
23. Wikipedia, "Significant figures". Overbar, underline and trailing-decimal-point conventions, and
    the recommended alternatives (prefix change, scientific notation, explicit statement,
    plus-minus). (2°) <https://en.wikipedia.org/wiki/Significant_figures>
24. eCampusOntario, "Accuracy, Precision, and Rounding Rules". ≈ communicates that a value has been
    rounded. (2°)
    <https://ecampusontario.pressbooks.pub/prehealthsciencesmath1/chapter/2-2-accuracy-precision-and-rounding-rules/>
25. `github.com/hilli/cooklang` (Go). `RoundToNiceFraction`, `FormatAsFraction`, and
    `ConversionMode` = `PreciseMode` (29.5735 ml/oz) | `BartenderMode` (30 ml/oz). (2°)
    <https://pkg.go.dev/github.com/hilli/cooklang>
26. manojmanivannan/MealPlanner issue #44, "Ingredient update: 4-decimal rounding collapses tiny
    recipe quantities to zero and compounds drift across edits". Remedies discussed: store more
    precision, or round only for display rather than at write time. (2°)
    <https://github.com/manojmanivannan/MealPlanner/issues/44>
27. Cooklang specification. <https://cooklang.org/docs/spec/>
28. Bagaren och Kocken, "NOD Precisionsvåg 0,01 g". Resolution 0,01 g, max 200 g, 329 SEK; sold for
    weighing spices and yeast in baking and homebrewing. A retailer's own product page.
    <https://www.bagarenochkocken.se/p/nod-precisionsvaag-001-g_71116/>
29. Retail listings for 50 g × 0.001 g milligram scales: Walmart USD 14.76–18.87, eBay USD 22,
    American Weigh Scales Gemini-50 USD 45.99, all including a 20 g calibration weight. (2°)
    <https://awscales.com/zeo50-high-precision-digital-50g-x-0-001g-milligram-scale/>
