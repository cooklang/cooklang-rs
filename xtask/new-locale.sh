#!/usr/bin/env bash
# Starts a locale cooklang does not have yet.
#
# Fetches CLDR's unit names for one language and writes locales/<tag>.toml
# with the unambiguous names accepted and the regionally variable ones left
# pending a verdict. The fetched JSON is a temporary file and is never
# committed: cooklang does not vendor CLDR, it seeds from it once per locale
# and hand-edits from then on. See docs/adr/locale-aware-units.md (decision 6).
#
# The locale is refused at runtime until someone who speaks the language
# confirms it and adds a `reviewed` key.
#
# Usage: xtask/new-locale.sh <bcp47-tag> [cldr-version]
set -euo pipefail

TAG="${1:?usage: xtask/new-locale.sh <bcp47-tag> [cldr-version]}"
VERSION="${2:-48.2.3}"
BASE="https://raw.githubusercontent.com/unicode-org/cldr-json/${VERSION}/cldr-json"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

curl -fsS "$BASE/cldr-units-full/main/$TAG/units.json" > "$TMP/units.json"
curl -fsS "$BASE/cldr-core/supplemental/plurals.json"   > "$TMP/plurals.json"

cargo xtask new-locale "$TAG" "$VERSION" "$TMP/units.json" "$TMP/plurals.json"
