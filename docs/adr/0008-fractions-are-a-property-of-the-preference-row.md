# Fractions are a property of the preference row

A converted amount is written as a fraction — `1/2 cup`, `3 1/2 fl oz` — when the preference row
that chose its unit says so: `{ unit = "us cup", at_least = 0.25, fractions = 4 }` in
`locales/preferences.toml`. The row's `fractions` is the largest denominator allowed. A row without
it writes decimals. Decided 2026-10-08.

## The rule

`best_amount` walks the rows largest first, as `best_unit` does. The first row the value reaches
the threshold of takes it — unless that row writes fractions and no fraction within 5 % of the
amount exists with a denominator up to `fractions`, in which case the next row is tried. The last
row takes whatever is left, as a decimal. So `100 ml` for a US reader is no quarter or third of a
cup, and becomes `3 1/2 fl oz`; `0.5 cup` stays `1/2 cup`; `2 cups` stays `2 cups`.

A fraction is a display form, not a rounding. `Number::Fraction` carries the residual, so the stored
amount is exact and a conversion back drifts by nothing. Only what is printed is approximate, and by
at most 5 % — the legacy converter's imperial `accuracy`, kept until the precision tiers in
`docs/research/measurement-precision.md` are decided.

## Why on the row

- **Which units take fractions is regional, not a property of the unit.** Sweden writes `½ tsk`,
  Norway `0,25 ts`, for the same 5 ml spoon (`research/best-units-by-region.md` §2). The row is
  already where a region says how it writes a quantity.
- **"Odd amounts in the smaller unit" is a row transition.** Australia and New Zealand print clean
  cup fractions and otherwise millilitres (§7.1 AU, NZ). Falling through to the next row when no
  fraction fits expresses that without a second mechanism.
- **The legacy converter's unit-and-fraction search is not ported.** It ranked candidates across
  every unit by denominator first, so `1.5 lb` became `24 oz` and `0.26 cup` became `4 tbsp`. Its
  purpose — a quarter cup rather than four tablespoons — is what the thresholds now say directly
  (ADR 0005, `us cup ≥ 0.25`). Choosing the unit, then the fraction in it, keeps the two decisions
  reviewable separately in the snapshot.

## What it costs

- A fraction that fits the first unit always wins, even when the next unit would be whole:
  `1.1 lb` is `1 1/8 lb` if pounds allow eighths, `18 oz` if they allow only quarters. The row's
  denominator is therefore a choice about idiom, made per unit with a citation.
- Zero and negative amounts are never fractions; they are written as they are in the first unit
  they reach, or the last row.
- A range is judged by its start; the end is written in the start's unit, as a fraction if the
  row has one that fits, else as a decimal.
