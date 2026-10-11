# Reviewing a locale

Each `<tag>.toml` here is one language: the unit names cooklang understands and
writes for it, and a comment listing names it deliberately does not.

Cooklang will not write unit names for a language until someone who speaks it
has confirmed them. This is what that means and how to do it. It takes about
twenty minutes and needs no programming.

If you only want to answer questions, you need step 1 and step 4.

## 1. See what the locale would write

```sh
cargo run --example show-locale-as-md -- sv
```

That prints every unit, the symbol a converted amount would be written with,
every spelling that would be understood as input, and what the plural forms
come out as. Nothing in it is live: the locale is refused until it is reviewed.

`locales/sv.toml` is the same information as an editable file, plus a comment
at the foot listing names that were refused and why.

## 2. What we are asking

**Would a recipe in this language actually write this?** Not "is it a correct
translation" — CLDR's names are usually correct translations and still wrong
for a recipe. A Swedish dictionary will tell you `kopp` means cup. A Swedish
*recipe* measures in decilitres and never writes a cup at all.

Three questions per unit:

- **The symbol** is written back after a conversion, so it must be what a
  recipe prints and must not change with the number. An inflected plural is
  wrong: Polish `łyżeczki` was replaced with `ł. cz.` for this reason.
- **The aliases** are what a cook may type. More is fine, as long as each one
  can only mean that unit here.
- **The long forms** are checked by the numbers shown. `1 litr`, `2 litry`,
  `5 litrów` — if any row is ungrammatical, say so.

## 3. What should be excluded

This is the part that matters most, because the cost is asymmetric: writing
the wrong amount is much worse than writing an unfamiliar one. **If in doubt,
exclude.** A unit with no name is written as its identifier — `2 us cup` —
which is ugly but unambiguous, and nobody is misled.

Exclude a name when:

- **The word means a different amount here.** The commonest and most dangerous
  case. A "cup" is 180 to 284 ml and a "pint" 473 to 568 ml depending on the
  country. If `kopp` means a 150 ml coffee cup in your language, it must not
  resolve to the 236.6 ml US cup.
- **It is a borrowed English word rather than something anyone writes.** CLDR
  gives every language a name for the imperial gallon. Most kitchens have
  never used one. If a recipe would never contain the word, exclude it.
- **It is untranslated English that CLDR inherited.** `pint Imp.` appears
  verbatim in more than twenty languages because nobody translated it. It is
  not a name in your language.
- **It is ambiguous with another unit, or with an ordinary word.** A spelling
  that could mean two units is worse than no spelling. So is one that appears
  constantly in ordinary prose.
- **You are not sure.** Say "I don't know" and we will leave it out and record
  that it is unresearched. That is a complete and useful answer.

Do **not** exclude a name only because it is rare or old-fashioned. Input
spellings are cheap: understanding a word a cook might type costs nothing,
as long as it cannot mean the wrong amount.

**A name you accept is hard to take back.** Once a locale is in use, adding a
symbol or alias later is harmless: a recipe that did not parse now does.
Removing or changing one can quietly break recipes already written with it,
in ways nobody may notice. So leaving a name out is the reversible choice,
and the one to make when you are unsure whether recipes in your language
would use it.

That includes whole units your kitchens never use. A new locale is seeded with
names for every unit whose amount is the same everywhere, such as feet and
inches, because those names cannot mean the wrong amount, not because a
recipe in your language would write them. Whether to keep them is your call;
the Danish, Finnish, Polish and Swedish reviews left them out.

## 4. Reply

Open an issue at <https://github.com/cooklang/cooklang-rs/issues>, or send the
notes however you like. Per unit, one line is plenty:

```
metric tablespoon   right
us cup              wrong -- nobody writes this; "kopp" is a 150 ml coffee cup
nordic spice measure  right, but the plural should be "kryddmått" for 2 as well
imperial pint       never used in Swedish recipes, leave it out
pound               not sure
```

Partial answers are welcome. A locale nobody has looked at is refused
entirely, so any correction is an improvement on nothing.

**Please say where you checked.** A cookbook, a recipe site, a language
authority — whatever you used. Every name in this repository carries its
source, and a review is recorded the same way: who reviewed it, when, and
against what. We will add that to `locales/<tag>.toml` and the locale starts
working.

## What happens to your answers

A maintainer edits `<tag>.toml`: accepted names become `[[unit]]` entries,
excluded ones move to the comment at the foot with your reason, and one line
records the review:

```toml
reviewed = { date = 2026-10-06, by = "a native speaker, Stockholm", sources = ["Vår kokbok, 28th ed."] }
```

`by` is a role and a place rather than a name — enough to judge the claim
without putting a private individual in a public repository. `date` is a bare
TOML date, not a quoted string. The build refuses an entry with an empty `by`
or no `sources`.

Nothing else is needed. A name resolves only by being listed in that file, so
excluding one means deleting a line, and the locale starts working the moment
`reviewed` is there.
