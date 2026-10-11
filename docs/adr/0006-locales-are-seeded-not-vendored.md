# Locales are seeded from CLDR once, not vendored

Supersedes [ADR 0003](./0003-cldr-for-names-only.md), whose core decision this keeps: CLDR is a
source of unit **names** only, never of conversion factors. What changes is the delivery.

A locale is now one hand-editable file, `locales/<tag>.toml`, holding its plural rules, every name
it resolves, and every name it refuses with the reason. It is seeded once from CLDR by
`xtask/new-locale.sh` and hand-edited from then on. CLDR's JSON is not vendored, not committed, and
not read by the build.

## Why

ADR 0003 vendored a faithful subset of CLDR's JSON and layered our decisions on top of it in four
places: `adjudicated.toml` for verdicts, `symbols.toml` for unusable short forms, `locales/<tag>.toml`
for names CLDR lacks, and `units.json` as a key bridge, merged by 331 lines of `build.rs` in a
precedence order you had to know to read any of it.

That is cldr-json's architecture, and it is right for cldr-json: a living, multi-contributor
database covering every locale and every unit, where contributors learn the layout because the
layout is the project. Cooklang needs a small frozen subset of it. Importing the structure imported
the shape of someone else's problem — answering "what does `sv` name?" meant reading six files and
simulating a merge.

The usual defence of the layering is that it keeps upstream pristine so a CLDR bump is reviewable.
It does not deliver that. The vendored files were single-line minified JSON, so a bump produced a
38-file diff in which every changed file was one unreadable line; the 862-row unadjudicated list
existed as a proxy for a review the format made impossible.

ADR 0003's own reason for default-reject was to avoid "making vendoring a human task" across thirty
locales. Since then the review gate refuses any locale no speaker has confirmed, so a human task per
locale is mandatory before anyone can be served. That benefit is no longer received, and cooking
units change slowly enough that a once-per-locale conversion is cheap.

## What this costs

- **Adding a locale is a human task.** `xtask/new-locale.sh` makes starting one cheap, but nothing
  arrives automatically.
- **A CLDR fix does not propagate.** If Unicode corrects a Polish plural we do not get it; someone
  re-seeds or edits by hand. Deliberate: a name change is something to review, not absorb.
- **More lines, all of them readable.** 38 files of about 230 lines replace 1610 lines of overlay
  *plus* 736 KB of opaque JSON. The reduction is in code, vocabulary and the number of places to
  look, not in bytes.
- **`reason` is not required on a name.** Serde no longer enforces that a hand-authored name is
  cited; the review gate is what asks about an uncited one. A weaker guarantee, stated plainly.

## Consequences

- **Refusal is a comment, because that is all it ever was.** Names a locale must not resolve are
  listed in prose at the foot of its file, with the reason. They were briefly `[[refused]]` TOML
  entries, which was a mistake: deleting all 453 of them changed not one of the 40 snapshots,
  because absence of a name is what stops it resolving. Dressing prose as data invites a reader to
  think the build enforces it. What the comment is genuinely for is the audit trail
  `adjudicated.toml` used to hold, and the list a reviewer works through — without it, dropping the
  CLDR JSON would have destroyed 857 candidate names.
- **Refusing a string for one unit is how it is given to another.** Finnish refuses CLDR's `mm` for
  the millimetre because in a Finnish recipe `mm` is the 1 ml *maustemitta*. The two stanzas sit
  next to each other in `locales/fi.toml`; four overlay layers never showed this.
- **A spaced identifier cannot be refused.** It resolves in every locale (ADR 0004), so a refusal
  naming one would be false and is dropped. Three such strings existed, all untranslated English in
  CLDR coinciding with our identifiers: `nl`'s `ounce` and `pound`, and `id`'s `metric cup`. The old
  unadjudicated list asserted they were unresolvable when they never were.
- **Provenance is per locale.** `cldr_version` sits in each file, because locales are seeded when
  someone comes to review them and will not share a release.
- **The review marker lives with the locale.** `reviewed = { date, by, sources }` is a key in the
  locale's own file; there is no separate list. The question asked is "is `sv` ready?", which the
  file answers.
- **`Unit::needs_a_verdict` replaces the `needs_verdict` list.** Whether a unit's amount varies by
  region is a property of the unit, not of a config file.
- **The build reads only TOML.** `serde_json` left `[build-dependencies]`; it is now an optional
  dependency used solely by `cargo xtask new-locale`.
- **Two files stay global, for stated reasons.** `parent-locales.toml`, because 193 of its 199
  children are tags we have no file for, and it is consulted for whatever tag a recipe asks for.
  And `plural-samples.json`, which is a *test fixture*, not build input: it carries CLDR's own
  `@integer`/`@decimal` samples, the oracle for more than a thousand assertions about the plural
  evaluator.

## Licence

Carried forward from ADR 0003, unchanged in substance.

CLDR data is under the Unicode License V3 (SPDX `Unicode-3.0`): MIT-like, with **no copyleft**. Its
grant explicitly includes the right to *modify*, so converting the JSON to TOML and then editing or
deleting strings is within it. Its single condition is that the copyright and permission notice
appear with all copies of the data or in associated documentation.

`locales/UNICODE-LICENSE` holds that notice beside the data it covers, and the crate is declared
`MIT AND Unicode-3.0` — which means the package *contains* material under both licences, not that
everything is dual-licensed. Because there is no copyleft, mixing CLDR-derived and hand-authored
names in one file relicenses nothing: our refusal reasons, hand-authored names and `reviewed` blocks
stay MIT.

Each locale file says so in its header, and records which CLDR release it was seeded from. Provenance
is marked per file rather than per entry: a `source = "cldr"` field on every name would be noise for
a question nobody asks per entry, and relying on `reason` being present only on hand-authored names
would be a coincidence rather than a rule — someone will eventually write a reason for keeping a
CLDR string.

One further restriction in the licence: Unicode's name may not be used to promote cooklang. It is
not.
