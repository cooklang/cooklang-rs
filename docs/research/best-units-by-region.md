# Best units by region: what home-cooking recipes actually write amounts in

Researched 2026-10-04 as the evidence base for region rows in `convert::preference`
(`src/convert/locales/preference.rs`; see `docs/adr/locale-aware-units.md` (decision 5)). Today
the `PREFERENCES` table is keyed by system only: Metric (`liter → milliliter`, `kilogram → gram →
milligram`, `meter → centimeter → millimeter`, `celsius`), US customary and Imperial. A region falls
back to its official system; only `US` is US customary. No region has rows of its own.

**Scope.** For each region and physical quantity (volume, mass, length, oven temperature): the
ordered chain of units a recipe writes amounts in, largest to smallest; the switching thresholds
where any convention exists; how certain the evidence is; and the source. Only where practice
differs from plain metric (l/ml, kg/g, cm, °C) or plain US customary (cup/fl oz/tbsp/tsp, lb/oz, in,
°F) is a row proposed. Unit *values* (how big a dl, a 大さじ or a dag is) are not re-researched;
they are cited by line from [`cooking-units.md`](cooking-units.md). CLDR's `unitPreferenceData` is
not used as evidence: [`cldr-unit-data.md`](cldr-unit-data.md) section 3 shows it has no cooking
usage.

**Method.** The question was split into eight region clusters and researched in parallel with
web search and page fetches on 2026-10-04. Every cluster hit the search-engine quota before it ran
out of leads, so gaps are listed per region and in "Open questions". A page that *states* a
convention ("1 szklanka ma zawsze pojemność 250 ml") outranks a page that merely *exhibits* one;
where only exhibits exist, the row says so and cites 2–3 concrete recipe URLs with the amounts as
printed.

**Source labels.** As in `cooking-units.md`:
- `[n]` points to the numbered reference list at the end.
- **(2°)** marks a secondary source: Wikipedia, a blog, a forum, press, a conversion-table site.
- **(snippet)** marks a page seen only as a search-engine result, not fetched.
- Any source not marked is primary for the claim it supports: a government or nutrition agency, a
  standards or language authority, a public broadcaster, or a national recipe publisher's own page
  about its own house style or its own recipe text.

**Certainty column.**
- **defined**: a government, standards or nutrition body fixes the unit or the convention.
- **style guide**: a publisher states the convention on a page about its own recipes.
- **observed**: inferred from recipe exhibits only; nobody was found stating it.

**Reading a chain.** `l ≥ 1 → dl ≥ ½ → msk ≥ 1` means: write litres from 1 l, decilitres from ½ dl,
tablespoons from 1 msk, and the last unit below that. Thresholds in *italics* are inferred from
exhibits, not stated anywhere. `ml (odd)` means millilitres are used only for amounts that are not a
clean fraction of the cup or spoon.

---

## 1. Cross-cutting findings

1. **No source in any region states a volume switching threshold.** Publishers define the units
   (dl = 100 ml, msk = 15 ml, 1 dl ≈ 6–7 msk) and stop. Every threshold below is observed.
2. **Spoons are universal.** Every metric region except mainland China, South Africa's traditional
   register and Spain's Directo al Paladar writes 5–45 ml amounts as tablespoons and teaspoons, not
   millilitres. The Metric system row has no spoons, so a 15 ml value converted for a German or
   Italian reader is written `15 ml`, which is not what their recipes print. Whether to put
   `metric tablespoon`/`metric teaspoon` into the Metric system row itself, rather than into every
   region row, is a design decision this note flags but does not take (section 6).
3. **The decilitre has five homes, not one.** Sweden, Norway, Denmark, Finland, Iceland, Switzerland,
   Hungary and Belgium (Flanders) write liquids in dl; Portugal is split; Czechia and Slovakia keep
   it only in the traditional register. The Netherlands, Germany, Austria, Italy and Spain do not.
4. **The centilitre has one home.** France writes liquids 5–75 cl in cl. Nobody else does, except
   Nordic drinks recipes for spirits.
5. **Decagrams in edited recipe output are a Hungarian fact and otherwise a register.** Hungary
   writes `25 dkg liszt` on every major site, down to `½ dkg só`. In Austria, Poland, Czechia and
   Slovakia the edited publishers (gutekueche.at, Plachutta, Sacher, Aniagotuje, Kwestia Smaku,
   Toprecepty, Varecha's modern recipes) write grams; dag/dkg survives in user-submitted and
   traditional recipes.
6. **The 100 g units (hg, etto, ons) do not appear in recipe output anywhere.** They are shop and
   speech units. Nor does Pfund, pond, livre or Danish pund.
7. **Cups are the unit of the English-speaking metric world and of East Asia and Latin America, at
   five different sizes**: 250 ml (AU, NZ, ZA, CA, PL szklanka, CO Nestlé), 240 ml (BR, MX
   probably, TW, Colombia's Resolución 810 allows 200 *or* 240), 200 ml (JP, KR, IN per NIN, BR
   labels), 236.6 ml (Nestlé AR's table), and no fixed size (RU/UA стакан, which no recipe page
   states).
8. **Oven temperature is Celsius everywhere except the US and English Canada.** Gas mark (UK, IE),
   thermostat (FR) and verbal bands (BR, AR, old IT and CZ) are printed *beside* °C, never instead
   of it, at every current publisher checked. Fan-oven figures are the live secondary display in
   AU, NZ, UK, DE, CH, NL.
9. **Canada's convention splits by language, not region.** English Canada prints imperial first with
   metric in brackets and °F first; Quebec prints metric first with imperial in brackets and °C
   first. A region-keyed row cannot express this.

---

## 2. Volume

| Region | Chain (largest → smallest) | Thresholds / notes | Certainty | Sources |
|---|---|---|---|---|
| SE | l → dl → msk (15) → tsk (5) → krm (1) | l from 1 l but `10 dl vatten` also printed; dl down to ½ dl; spoons for 1–3½ msk (Livsmedelsverket writes `3,5 msk vatten` = 52 ml, not ½ dl); tsk down to ½ tsk; krm for 1–2 krm. ml never appears as an amount, only in package sizes (`1 burk kokosmjölk (à 400 ml)`). Köket.se: "det går ungefär 6–7 msk på 1 dl". Values: cooking-units.md lines 560–562. | style guide (unit sizes); *observed* (thresholds) | [1][2][3][4][5][6] |
| NO | l → dl → ss (15) → ts (5) → krm (1); bs 10 ml defined, rare | l from 1 l (`1 l kjøttkraft`, `1,5 l vann`); dl with decimals (`1,5 dl`, `4,5 dl`); ss up to 4 ss; ts to ¼ ts (`0,25 ts pepper`). SNL and MatPrat define the sizes; mattenett.no (school source) prescribes ml/dl/l and no spoons, a minority position MatPrat does not follow. Språkrådet: abbreviations ss, ts, dl without full stop. Values: line 563. | style guide; *observed* thresholds | [8][9][10][11][12][13] |
| DK | l (¼ l, ½ l, 1 l) → dl → spsk (15) → tsk (5) → knsp | Litre fractions preferred over `15 dl`: `½ l suppe`; Arla DK's own table lists ¼ l, ½ l, 1 l as units. dl to ½ dl; spsk up to 5 spsk (Fødevarestyrelsen: `5 spsk. peanutbutter` = 75 ml); Danish baking gives flour and sugar in g with dl in brackets (`150 g Hvedemel (ca. 2½ dl)`). cl only for spirits (`6 cl vodka` beside `1 dl piskefløde`). Values: line 566. | style guide (Arla DK states spoon sizes); *observed* | [14][15][16][17] |
| FI | l → dl → rkl (15) → tl (5) → mm (1) | `1 l maitoa` but `8 dl vettä`: l from 1 l; dl to ½ dl (`½ dl siirappia`); rkl up to 4 rkl (60 ml); tl to ½ tl. Martat and Yhteishyvä state 1 dl = 6–7 rkl, measures level. Kielitoimisto lists rkl, tl, dl, l as dot-less abbreviations; mm is not in its list. Values: line 568. | style guide; *observed* | [18][19][20][21][22] |
| IS | l → dl → msk → tsk; bolli 2.5 dl | MS states `1 lítri = 10 dl = 4 bollar`, `1 dl = 6–7 msk`; exhibit `6–7 dl mjólk`. Brief check only. | style guide | [25] |
| DE | l → ml → EL (15) → TL (5) → Msp. | ml for ≥ 50 ml; EL up to about 5–6 EL (`etwa 3 EL Milch`); Tasse 150 ml exists only as a conversion aid (Küchengötter). No dl or cl. `1,5 l` vs `1500 ml` not resolved. Values: lines 447–449. | *observed* | [33][35] |
| AT | l (¼ l, ⅛ l, ⅜ l) → ml → EL → TL | Fractions of a litre are the Austrian trait: Plachutta `¼ l Milch`; ichkoche `3/8 l Milch`. gutekueche.at writes `150 ml Milch`. One ichkoche user page writes `50 cl`, atypical. | *observed* | [26][29][30] |
| CH | l → dl → EL (12–15) → TL/KL (5) | dl with one decimal from 1 dl (`3,6 dl Vollrahm`, `2,5 dl Rahm`, `6 dl Milch`); EL up to 3 EL; ml rare; cl for drinks. Dish size in litres (`ofenfeste Form (ca. 2 ½ Liter)`). Swissmilk states `1 TL = 5 ml / 1 EL = 12–15 ml`. | style guide (spoons); *observed* (dl) | [36][38][39][40] |
| NL | l → ml → el → tl | Allerhande (snippet): `300 ml halfvolle melk`, `750 ml verse slagroom`, `1 liter volle melk`. dl survives in Voedingscentrum's table and older recipes. **Voedingscentrum defines 1 eetlepel = 12 ml, 1 theelepel = 2 ml**, which recipe sites do not follow (15/5). Values: line 475. | defined (Voedingscentrum, conflicting); *observed* | [41][42] |
| BE (nl) | l → dl → cl/ml → el → tl | VRT Dagelijkse kost: `4 dl Melk`, `1 dl room`, `80 ml room`, `50 ml melk`, `1 liter kippenbouillon`; dl dominant, cl and ml both appear below 1 dl. | *observed* | [44][45] |
| FR | l → cl → c. à soupe (1.5 cl) → c. à café (0.5 cl) | cl from 5 cl to 75 cl (`25 cl de lait`, `60 cl de lait`, `5 cl`); l from 1 l; spoons for 1–3 c. à s.; ml only for official nutrition pages (mangerbouger uses both `300 ml` and `25 cl`). dl absent from recipe output. `verre` only as a gimmick recipe. 750g: professionals weigh liquids in g. Values: lines 485–486. | *observed* | [48][49][50][51][52][53][54][55] |
| IT | l → ml → cucchiaio → cucchiaino | `1 l`, `1,5 l`, `1/2 litro`, `500 ml`, `100 ml`; Giallozafferano weighs small liquids in g (`Vino bianco 50 g`); `1 bicchiere di vino` survives for wine. dl not observed on any editorial page. Values: lines 496–497. | *observed* | [59][60][61][62][63] |
| ES | l → ml → cucharada → cucharadita | `1 l`, `1/4 de l`, `500 ml`, `180 ml`; Directo al Paladar writes even 10–15 ml in ml (`Zumo de limón 15 ml`); Hogarmania writes `1 cucharadita`. No cl, no vaso or taza in output. Values: line 506. | *observed* | [68][69][70][71][72] |
| PT | l → dl → ml → colher de sopa → colher de chá → colher de café | Split: Teleculinária and Vaqueiro write dl from 0,5 dl to 7 dl (`4 dl de natas`, `0,5 dl de brandy`); Pingo Doce mixes (`1,5 dl natas` vs `250 ml leite`); DGS/PNPAS writes ml and annotates spoons in g (`1 colher de sopa (10g) Azeite`). `1 l`/`1 L` from 1 l. chávena only in conversion tables (2°). Values: lines 508–509. | *observed*, split | [74][75][76][77][78][79][81][82] |
| GB | metric: l → ml → tbsp (15) → dsp (10, defined, unused) → tsp (5); imperial after a slash or in brackets: pint → fl oz | Metric first always: `300ml/10fl oz`, `275ml/½ pint`, `150ml/¼ pint`, `85ml/3fl oz` (BBC Food) or metric only (Good Food, Waitrose, Sainsbury's, Nigella, Jamie). Pint from ½ pint (BBC writes both `275ml/½ pint` and `300ml/10fl oz`); 568 ml = 1 pint (GFW), 570 ml (Delia); 1 l = 1¾ pint. No cups, quarts or gallons. Guild of Food Writers recommends metric only; Guardian style: metric, imperial in brackets "wherever useful". Delia's own tablespoon is 20 ml. Values: lines 144–157. | style guide (GFW, Guardian, Delia, Good Food); *observed* (BBC Food) | [84][85][86][87][88][89][90][91][92][94][96][97] |
| IE | metric only in current output; legacy `150ml/¼pt` | Bord Bia `125ml water`, `300ml milk`; Avoca `550ml buttermilk`; RTÉ `100ml milk`; Odlums legacy recipes carry imperial. No Irish body statement found (safefood blocked). | *observed* | [98][99][100][102][103] |
| AU | cup (250) → ½ ⅓ ¼ cup → tbsp (**20**) → tsp (5) → ½ ¼ tsp; ml (odd) | Clean fractions in cups (`1 cup milk`); odd liquids in ml (`100ml buttermilk`, `200mL water`). Single unit per line at AWW, SBS, ABC; Donna Hay prints both (`½ cup (125ml)`). SBS Cook's Notes on every recipe: "1 tablespoon equals 20 ml; 1 cup equals 250 ml". Eat for Health writes `1 cup (250ml)`. AS 1325 text not reached. Values: lines 191–193. | style guide; defined (standard not read) | [107][108][109][110][111][112][113][114] |
| NZ | cup (250) → Tbsp (15) → dsp (10, Edmonds) → tsp (5); mL (odd) | Edmonds: "New Zealand Standard metric cup and spoon measures are used in all recipes … 1 Tbsp = 15 millilitres (Note an Australian Tbsp is 20ml)". RNZ `250mL boiling water`, `80mL oil` beside `⅓ cup brown sugar`. Values: lines 196–197. | style guide | [115][116][117][118][119][120][121] |
| ZA | two registers: (a) all-ml down to 3 ml; (b) cup (250) → tbsp (15) → tsp (5), ml (odd); (c) dual metric-first | Huletts: `250ml flour, 5ml baking powder, 3ml salt, 60ml butter` — no spoons at all; Food & Home: `1 ½ cups dates … 300 ml pouring cream` and `125ml (½ cup) vegetable oil`, `7,5ml bicarbonate` (decimal comma). No Department of Health or SABS page reached. Values: line 198. | style guide (sizes); *observed*, split | [122][123][124][125][126] |
| CA | both units on every line. en: cup (250) → tbsp → tsp with mL in brackets; fr: ml first, tasse in brackets | en: `1 1/4 cups (310 mL)`, `1/2 tsp (2 mL)`, `8 cups (2 litres)`; Dietitians of Canada rounds metric (`1/4 cup (50 mL)`). fr (Ricardo): `125 ml (1/2 tasse)`, `5 ml (1 c. à thé)`, `2 litres (8 tasses)`. Health Canada writes metric first in both languages. Ricardo: "sans mélanger les systèmes". Values: lines 183–184. | style guide (Health Canada, Ricardo) | [127][128][129][130][131][132][133][135] |
| IN | cup (size unsettled) → ½ ¼ cup → tbsp → tsp → ¼ tsp | ml almost never in recipe lines; katori is a nutrition portion unit, not a recipe unit. NIN 2024: `1 cup/Katori = 200ml`; IDA sheet lists both a 250 ml large and 200 ml medium cup; Archana's Kitchen says 240 ml; Tarla Dalal 200 ml (snippet). Values: lines 294–295. | defined (NIN, nutrition only); *observed* | [137][138][139][140][141][142][143][144] |
| JP | カップ (200) → ml (cc) → 大さじ (15) → 小さじ (5) → 少々; rice in 合 | Kikkoman, Ajinomoto, DELISH KITCHEN, Kurashiru, Mizkan all state 200/15/5 and warn that 250 ml cups exist and the rice cup is 180 ml = 1合. NHK prefers cup fractions (`だし カップ3/4`); Kikkoman writes both (`1と1/2カップ（300ml）`); Cookpad users still write `cc` (`水 400cc`, `薄口醬油 13cc`). Spoons up to 大さじ4; below 小さじ¼ → 少々. Every rice recipe writes `米 2合`. No JIS standard exists for measuring cups or spoons (chef 樋口直哉; the JIS S index has no S 2050/S 3009) (2°). Values: lines 206–213. | style guide | [145][146][147][148][149][150][151][152][153][154][155][157][158][159][160] |
| KR | 컵 (200) → ml → 큰술/T (15) → 작은술/t (5); informal 종이컵 (180), 밥숟가락, 숟갈 | 만개의레시피 measuring guide states all of these and `1종이컵 = 180ml`; 우리의식탁 the same. RDA's 200/15/5 reached only via press (2°). Exhibits use 숟갈 (household spoon) and 종이컵 loosely (`쌀뜨물 종이컵 5컵 (700ml)` = 140 ml a cup). Values: lines 271–272. | style guide | [161][162][163][164][165] |
| CN | 克 even for liquids → 勺 (household ceramic spoon, undefined) / 汤匙 (15) / 茶匙 (5) → 适量; 杯 (240) baking only | 下厨房 top 红烧肉: `料酒 20克, 老抽 2克, 生抽 5克, 食用油 5克, 盐 适量`; alternative all-vague style `生抽 适量`; baking posts in g with `香草精 1小勺`. ml not seen. Values: lines 233–234. | *observed* | [166][167][168][169][170] |
| TW | 杯 (240, "mostly") → cc/毫升/ml → 大匙 (15) → 小匙 (5) | HPA 食物代換表: `1杯 = 240毫升`, `烹調用油1茶匙(5公克)`. iCook: `醬油 6大匙=90CC, 米酒 200CC`; `紹興酒 50毫升`; one author writes everything in ml down to `烏醋 5ml`; another all in 克 (`紹興酒 200 克`). iCook states no site convention. | defined (HPA, nutrition); *observed* | [171][172][173][174][175][176] |
| HK | 克 → 茶匙/湯匙; 杯 unverified | Government EatSmart recipe: `鹽 0.5克, 植物油 1茶匙`. Commercial HK sites could not be fetched. | *observed*, thin | [177] |
| RU | л → мл → стакан (200 or 250, never stated on the recipe page) → ст. л. (15–20) → ч. л. (5) | Eda.ru `Молоко — 2 стакана`; Gastronom hedges `2 1/2 стакана (350 г)` and its table lists both the 250 cm³ thin glass and the 200 cm³ faceted glass with no default. л from 0,5 л. Tablespoon 15 ml (calculators), 18 ml (Soviet tables), 20 ml (kp.ru) (2°). Values: lines 532–536. | *observed*; glass size ambiguous | [180][181][182][183][184][185][186] |
| UA | л → мл → ст. л. → ч. л.; склянка in media/legacy | Klopotenko writes ml and g (`340 мл молока`, `1 л молока`, `50 мл олії`); `1,5-2 склянки борошна` in press reprints (snippet). | *observed* | [188][189][190] |
| PL | l → ml → szklanka (250) → łyżka (15) → łyżeczka (5) | Stated: Moje Wypieki "1 szklanka ma zawsze pojemność 250 ml, 1 łyżka … 15 ml, 1 łyżeczka … 5 ml"; Aniagotuje the same with ml on every line (`1 szklanka mleka - 250 ml`). Kwestia Smaku uses `3/4 szklanki wody` unstated. l from 0,5 l; `650 ml wody` also. Values: line 550. | style guide | [191][192][193][194][195] |
| CZ | l → ml → hrnek (250) → lžíce (15) → lžička (5); dl in older recipes | Toprecepty: `200 ml mléka`, `150 ml oleje`, `500 ml mléka`. hrnek = 250 ml stated by three publishers, but recepty.cz's own table also says `3 dl … 1 hrnek`. `1dl oleje` in hobby recipes (2°). Values: line 552. | style guide (hrnek); *observed* (ml) | [198][199][200][201][202] |
| SK | modern: ml; traditional: dcl | Varecha: `500 ml mlieko` (modern) vs `2 dcl mlieko, 1/2 dcl olej` (traditional). Spelling **dcl**. | *observed* | [206][207] |
| HU | l → dl → ml (rare) → ek (15) → tk/kk (5) → mk (2) | **dl confirmed as the normal liquid unit**: Mindmegette `2 dl tej`, `1 dl olaj … 2.5 dl tej`, `1.25 dl tej`; Horváth Ilona `2 dl tej`. Fractions of dl rather than ml. Street Kitchen (chef register) writes `350 ml tej`. bögre rare in exhibits; dl used instead. Spoon sizes only from 2° tables. Values: line 554. | *observed* (strong) | [208][209][210][211][212][213][214][215] |
| MX | taza → cucharada → cucharadita; ml rare | Kiwilimón stores amounts and defaults to `2 tazas de harina`, `10 cucharadas de mantequilla` (unit toggle "Gramos, litros / Tazas, Cucharadas" in page source); Cocina Fácil `2 tazas de frijoles`, `3 cdas.`. SMAE (nutrition): cucharada 15 ml, cucharadita 5 ml, taza "¼ de litro" = 250 ml — the 240 ml figure for Mexico rests on a blog (2°). Values: lines 598–599. | *observed*; cup size unresolved | [227][228][229] |
| BR | litro → ml → xícara (chá) (240) → colher (sopa) (15) → colher (sobremesa) (10) → colher (chá) (5) → colher (café) (2,5); lata 395 g as a unit | Stated by three publishers: TudoGostoso "A xícara de chá (240 ml) é a nossa medida padrão"; Panelinha "1 xícara (chá) = 240 ml … a de 240 ml"; Receitas Nestlé 240/15/5/2,5. ANVISA labels differ (200/10/5; lines 593–594). TudoGostoso's second article contradicts itself (250 ml). Exhibit: `2 e 1/2 xícaras (chá) de farinha`, `1 xícara (chá) de leite`, `2 medidas (da lata)`. | style guide; defined (labels, different values) | [217][218][219][220][221][222][224][225] |
| AR | litro (`1 LT`) → cc/ml → taza (dry goods) → cda. → cdita. | Cocineros Argentinos (TV Pública): `Crema 200 cc`, `Aceite 20 cc`, `1 LT de leche`; Nestlé AR: `300 ml de leche` beside `1 taza de azúcar`. GAPA (Ministerio de Salud) gives all quantities in "g/cc" and has no household-measure table — **cc is the official Argentine volume idiom**. Nestlé AR's table is US-derived (taza 236 ml, libra 450 g). | defined (GAPA, cc); *observed* | [231][233][234][235][236][237][238] |
| CO | litro → ml → taza/pocillo → cucharada → cucharadita | Resolución 810/2021 art. 13 (labels): "1 taza equivale 200 ml o 240 ml; 1 vaso 200 ml o 240 ml; cucharadita 5 ml; cucharada 15 ml". ICBF GABA portions in "pocillo chocolatero" with no ml value. Nestlé CO writes `1 taza de agua (250 ml)`, `6 tazas de agua (1500 ml)`, `1 taza o pocillo de arroz (200 g)`. | defined (labels, two sizes); *observed* (250) | [240][241][242][243] |
| CL, PE | taza 200 (CL official portions, snippet) / measured real cups 280–426 ml (PE TAFERA) | Brief pass only. Peru's TAFERA does not standardise a cup; it measures household ones. | defined (nutrition) | [246][247] |

---

## 3. Mass

| Region | Chain | Thresholds / notes | Certainty | Sources |
|---|---|---|---|---|
| SE, NO, DK, FI, IS | kg → g | g and kg in every recipe fetched. hg listed as a "recipe unit" by recepten.se but never used in an amount; Wiktionary's hekto examples are shop talk (2°). Danish pund: madensverden's "pund til pund kage" is written `200 g hvedemel` and explains a pund as 500 g (2°). Lines 567. | *observed* | [2][3][4][16][21] |
| DE | kg → g; Pck. for packets | `300 g Butter`, `375 g Weizenmehl`, `20 g Kakao`, `1 Pck.` No Pfund in any publisher amount; only in headlines (`1 Pfund Hackfleisch`, body says 500 g) (2°). Line 445. | *observed* | [33] |
| AT | kg → g at edited publishers; kg → dag → g in traditional and user recipes | **dag**: ichkoche.at user recipes `50 dag glattes Mehl, 25 dag Staubzucker, 25 dag Butter`, `20 dag Mehl`; always written `dag` (never dkg or Deka on the pages fetched); down to `2 dag Butter` (snippet). **g**: gutekueche.at `250 g Butter … 500 g Weizenmehl`; Plachutta all g; Sacher's own recipe `130 g Kuvertüre … 140 g Weizenmehl`; ichkoche's own Topfengugelhupf `250 g Topfen`. ORF Nachlese and Kronen Zeitung not reached. Line 446. | *observed*, split by register | [26][27][28][29][30][31] |
| CH | kg → g | `1,3 kg Äpfel`, `500 g Mehl`, `500 g Magerquark`. No Pfund. | *observed* | [38][39] |
| NL, BE | kg → g | `500 g gehakt` (snippet); VRT `800 g Aardappelen`. ons/pond are butcher's-counter units (`Gehakt (pond)`) (2°). Lines 473–474. | *observed* | [42][44] |
| FR | kg → g | `300 g de farine`, `1.5 kg de groseilles`. No livre. Line 484. | *observed* | [48][51] |
| IT | kg → g | `Burro 100 g`, `1 kg circa di arista`. etto only in a GZ *blog* (`tre etti e mezzo di Carnaroli`) and in commentary that it is spoken, not written (2°). hg never. Line 495. | *observed* | [59][63][66][67] |
| ES, PT | kg → g | `Azúcar 350 g`, `Pollo entero de aprox 1.700g`; `1 kg de bacalhau`, `1,500 kg de couve lombarda`. | *observed* | [68][70][76][80] |
| GB | metric: kg → g; imperial after slash: lb (+ oz) → oz | `225g/8oz`, `30g/1¼oz`, `500g/1lb 2oz`, `1 x 2kg/4½lb whole chicken` (BBC Food); metric only elsewhere. Rounding tables: Delia 1 oz = 25 g, 4 oz = 110 g, 1 lb = 450 g; GFW 1 oz = 25/30 g, 4 oz = 115 g, 450 g = 1 lb, 500 g = 1 lb 2 oz, 1 kg = 2 lb 4 oz. lb from 450 g; kg from about 1 kg. Line 150. | style guide (tables); *observed* (form) | [84][85][86][90][92][93] |
| IE | kg → g; legacy `225g/8oz` | Bord Bia `450g lean minced beef`, `900g potatoes`; RTÉ 2012 `500 g (1lb 2oz)`, 2020–26 metric only; Odlums legacy `25g/1 oz`. | *observed* | [98][102][103][104][105] |
| AU, NZ, ZA | kg → g | `125g butter`, `750g potatoes`, `600 g beef mince`, `1 kg sugar`. oz/lb only in AWW's and Edmonds' conversion tables. | *observed* | [107][111][115][121][125] |
| CA | en: lb (+ g in brackets) for meat, g for packaged goods; fr: g first | `1 lb (450 g) white mushrooms` / `454 g (1 lb) d'agneau`; `1 package light cream cheese … 250 g`. Butter sold in 454 g packs (2°). | *observed* | [132][133][135][136] |
| IN | g when bought by weight or for baking; counts otherwise | `200 grams Cocoa Powder, 24 grams Butter` beside `3/4 cup Buttermilk`. | *observed* | [144] |
| JP, KR | g | `豚肩ロース肉（塊）600g`, `鶏もも肉 200g`; `배추김치 420g, 돼지목살 300g`. No 斤, 근 or kg seen. | *observed* | [154][160][164] |
| CN | 克; 斤 occasionally in amounts | `五花肉 500克`; `排骨2斤、五花肉1斤` (Baidu snippet, 2°). Lines 227–228. | *observed* | [168][170] |
| TW | 克/g; 台斤 in amounts | `五花肉 1斤` beside `蒜 30克`; `豬五花 400克`; `五花肉 900 克`. HPA uses 公克. Line 254. | *observed* | [173][174][176] |
| HK | 克 in government recipes; 斤/両 in markets | `鱈魚柳 120克`. HK Customs: 1 catty ≈ 0.61 kg. Lines 240–241. | *observed* | [177][178] |
| RU, UA | кг → г | `20 г`, `125 г`, `350 г`, `130 г борошна`. | *observed* | [180][181][188] |
| PL | kg → g (modern); kg → dag/dkg (traditional) | Aniagotuje `500 g mąki`; Kwestia Smaku `125 g mąki, 80 g masła`; Dom Wypieków `1 kg mąki … 16 g cukru`. Traditional: `10 dkg drożdży` (PrzyslijPrzepis); `50 dag mąki, 20 dag cukru pudru, 12 dag masła, 1 dag sody` (snippet, 2°). Both spellings. dag recipes go down to 1 dag, then switch to spoons; no `12,5 dag` seen. Line 549. | *observed*, split | [192][194][195][196][197] |
| CZ, SK | kg → g (modern); dkg (classic) | Toprecepty `25 g sádla, 30 g droždí`; Sandtnerová `4 dkg mouky`; Varecha traditional `30 dkg múka, 5 dkg cukor`. cs.wikipedia: Czechs *shop* in dkg below 1 kg (2°). Line 551. | *observed*, split | [200][201][203][204][206][207] |
| HU | kg (also ½ kg) → dkg → fractional dkg → spoons | **Confirmed on every major site**: `25 dkg liszt, 20 dkg cukor`, `40 dkg cukor`, `15 dkg kristálycukor, 5 dkg porcukor`, `30 dkg sertéshús`; Horváth Ilona `1/2 kg liszt, 25 dkg vaj, 2 dkg élesztő`, `1/2 dkg só`. **Below 1 dkg, fractions of dkg, not g**: `0.9 dkg sütőpor`, `2 dkg kakaópor`. kg at 1 kg and ½ kg. Street Kitchen writes g but slips in `20 dkg szilvalekvár`. Line 553. | *observed* (strong) | [208][209][210][211][212][213][214] |
| MX, BR, AR | kg → g | `160 g de azúcar`, `Batata 1 kg`; BR pack sizes `395 g`. "kilo"/"cuarto" usage not verified. | *observed* | [225][234][235] |
| CO | libra (500 g) for meat, potatoes, grain; g for small items | Nestlé CO: `1 libra de sobrebarriga (500 g)`, `½ libra de carne de cerdo (250 g)`, `¼ de libra de arveja (125 g)`. SIC (2024) recommends g/kg but says the libra will stay in use (2°). Line 590. | *observed* (one publisher, consistent) | [242][244] |

---

## 4. Length

| Region | Chain | Thresholds / notes | Certainty | Sources |
|---|---|---|---|---|
| All metric Europe, Nordic, JP, KR, CN, TW, LatAm | cm (Ø for round tins); mm for thickness; tin volume in litres (SE, CH) | `Springform (Ø 26 cm)`, `Ø 24 cm`, `23×34 cm-es tepsibe`, `stampo da 20 cm`, `molde de 25 centímetros`, `forma retangular (20 x 30 cm)`, `直径18cmの丸型`, `tortownicy o średnicy 24 cm`. Sweden also sizes tins by volume (`en form som rymmer ca 1 ½ liter`), Switzerland too (`Form (ca. 2 ½ Liter)`). Japanese 号 (3 cm × n) and Chinese 寸 (6寸 ≈ 15 cm) are tin-retail labels; recipes themselves write cm (NHK) or 寸 (下厨房 baking). | *observed* | [7][23][32][33][40][59][71][156][166][194][210][226] |
| GB | cm with inches after a slash or in brackets; Good Food's tin table is inch-first | `two 20cm/8in sandwich tins`, `23cm / 9 inch springform`; Waitrose metric only; Sainsbury's `7 inch sandwich tins`. Tables: 18 cm = 7 in, 20 = 8, 23 = 9, 25.5 = 10. Line 152. | style guide (tables); *observed* | [84][85][86][90][94][96][97] |
| IE | cm; inches at one contributor | Avoca `3 x 23cm springform cake tins`; RTÉ `40cm x 25cm` but Donal Skehan `three 8 inch round cake tins`. | *observed* | [100][103][105] |
| AU, NZ, ZA | cm; dual at Donna Hay and Food & Home | `31cm x 21cm slab pan`, `Cut 5cm rounds`; `24cm (9½ inch)`; `22cm (9in) round cake pan`. | *observed* | [110][111][112][124] |
| CA | dual: `23 x 13-cm (9 x 5-inch)` / `23 X 13 cm (9 X 5 po)` | Never metric-only. | *observed* | [132][133][135] |
| IN | **inches** | `1 1/2 inch Ginger`, `2 inch Ginger`, `1 inch Cinnamon Stick`, `cut into 1 inch pieces`. Tin sizes not found. | *observed* | [143][144] |

---

## 5. Oven temperature

| Region | Chain | Thresholds / notes | Certainty | Sources |
|---|---|---|---|---|
| SE, NO, DK, FI | °C, often written `grader`/`°` without C; mode named in DK | `175 grader`; `175° eller 160° varmluft`; `200° – traditionel ovn`; `200°C`, `250 asteessa`. Norwegian tables give descriptive bands (`middels varm ovn 200–250 °C`) (2°). | *observed* | [3][7][16][21][23] |
| DE, AT, CH | °C for Ober-/Unterhitze, with Umluft/Heissluft = −20 °C as the second figure | Dr. Oetker prints both in every recipe (`Ober- und Unterhitze: etwa 180 °C / Heißluft: etwa 160 °C`) and states the −20 °C rule; Swissmilk states it and prints both in either order; AT writes `Backrohr auf 180° Grad (Ober-/Unterhitze)`. **Gas Stufe: no publisher-printed table found**; Dr. Oetker's current recipes carry no gas line; 2° tables give Stufe 1–8 = 150/160/180/200/220/240/260/280 °C (kuechentipps: Stufe 1 = 140). Line 121. | style guide (Oetker, Swissmilk); Stufe 2° only | [29][30][33][34][37][38][46][47] |
| NL, BE | °C, electric oven default; hetelucht as a percentage | puurgezond: "voor een elektrische oven … heteluchtoven … ongeveer 10% verlagen"; Laura's Bakery −20 % (2°). VRT `160 °C`, Lekker van bij ons `180 °C hetelucht`. | *observed* | [43][44][45] |
| FR | °C with thermostat in brackets at Marmiton and CuisineAZ; °C only at 750g and mangerbouger | `four moyen (thermostat 4-5, 140°C)`; `th.8 (240°C)`; `150 °C (Th. 5)`; Th n ≈ 30 °C × n (Th 6 = 180, Th 7 = 210) (2°). No AFNOR source. Line 120. | *observed* | [49][52][53][57][58] |
| IT | °C with mode (`forno statico preriscaldato a 190°`); verbal bands only in tables and old recipes | Donna Moderna: moderato 180–190 °C; GZ blog: moderato 175–190; ricetteecooking: 170–190 — the bands disagree (2°). Lines 122–125 (NIST's bands). | *observed* | [61][63][64][65][66] |
| ES, PT | °C, mode named in ES | `180 ºC con calor arriba y abajo`; `180 ºC`, `180 graus`. | *observed* | [69][71][77][78] |
| GB | °C / fan °C / gas mark at every publisher; °F at some | BBC Food `180C/160C Fan/Gas 4` (no °F); Good Food `190C/fan 170C/gas 5` (no °F; its table has no °F column); Waitrose `180 degrees C, gas mark 4`; Sainsbury's `180°C, fan 160°C, gas 4`; Nigella `gas mark 4/180°C/160°C Fan/350ºF`; Jamie `200°C/400°F/gas 6`; Delia's table Gas/°F/°C, fan = −20. Lines 116–119. | style guide (tables); *observed* | [84][86][88][90][94][95][96][97] |
| IE | gas mark first at Bord Bia, with °F in brackets | Bord Bia `Gas Mark 4, 180°C (350°F)`; Odlums `220°C/Fan 200°C/425°F/Gas Mark 8`; RTÉ follows the contributor (`190°C/170°C fan/375°F/Gas Mark 5`; `190°C fan/210°C/gas mark 6½`); Avoca °C only. | *observed* | [98][99][100][102][103][105] |
| AU, NZ, ZA | °C, with a fan-forced figure in AU/NZ | AWW `220°C (200°C fan-forced)`; SBS states "reduce by 20˚C" for fan; NZ Herald `200C/180C Fan`; Chelsea's default is bake, Langbein's is fan-bake (+10 °C for conventional); ZA `180°C` with no fan note. Donna Hay `160°C (325°F)`. Line 194. | style guide | [109][111][112][116][118][119][122][124] |
| CA | en: °F first, °C in brackets; fr: °C first | `400 °F (200 °C)`, `350º F (180º C)`, `375F (190C)`; Ricardo FR `132 °C (269 °F)`. 2022 poll: 59 % of Canadians use °F for ovens (2°). | *observed* (uniform) | [131][132][134][135][136] |
| IN, JP, KR, CN, TW, HK | °C (`度`) | `180 degree celsius`; `170℃`; `上下火150度`; `180度`. Stovetop in 強/中/弱火, 센/중/약불, 大/中/小火. | *observed* | [142][144][154][156][169][170][177] |
| RU, UA, PL, CZ, HU | °C; verbal bands and gas dial `деления`/`fokozat` only as conversion aids | `170-180 градусов`; `180 stopni C (góra i dół bez termoobiegu)`; `180 °C`; `180 fokra`. Forum tables for 5–9-division gas dials (2°); Receptneked's blog `1. fokozat= 160 C fok … 8. fokozat= 275`; no recipe specifies heat that way. | *observed* | [181][187][194][200][210][213][216] |
| BR | band name with °C in brackets as house style | Nestlé BR `forno médio (180ºC)`; TudoGostoso `180° C`. Bands (2°): baixo 150–160, médio 170–190, quente 200–220, muito quente 230–250. | style guide (Nestlé BR form) | [224][225][226][230] |
| AR | band names stated by a publisher | Cucinare/eltrece: "muy bajo 110°, bajo 150°, moderado 180°, caliente 200°, muy caliente 240°-260°"; Paulina `mínimo-medio (180º)`; Nestlé AR `temperatura media 180ºC`. | style guide | [236][237][238][239] |
| MX, CO | °C; °F on US editions and in one CO table | Cocina Fácil `180oC`; us.kiwilimon `350 °F (190 °C)` (snippet); colombia.com `350 F / 180 C = Moderado` (2°). | *observed* | [229][245] |

---

## 6. Regions that should NOT get a row

The system fallback (Metric, or US customary for `US`) already writes what these regions' recipes
print, for the quantity named. "Metric + spoons" means the region would be served by the Metric
row *if* that row carried `metric tablespoon` and `metric teaspoon`; without them it is served for
amounts ≥ 50 ml only.

| Region | Quantity | Why plain metric (or US customary) already matches | Evidence |
|---|---|---|---|
| DE | volume | l → ml → EL → TL: metric + spoons. No dl, cl or Tasse in output. | [33][35] |
| DE, CH, FR, IT, ES, NL, BE, Nordic, JP, KR, RU, UA, MX, BR, AR | mass | g and kg only. Pfund, livre, etto, ons, hg, pund, 斤, 근 do not appear as recipe amounts. | section 3 |
| IT, ES | volume | l → ml → spoons; dl and cl absent; Directo al Paladar writes even 15 ml as `15 ml`. | [59]–[63], [68]–[72] |
| NL | volume | l → ml → el → tl in Allerhande. dl is the older register. | [41][42] |
| AT | volume | l → ml → EL → TL; the litre fractions (`¼ l`) are a formatting matter, not a unit. | [29][30] |
| CZ, SK, PL | volume (liquids) | ml at Toprecepty, Varecha (modern) and Aniagotuje; szklanka/hrnek is used for dry goods, and a converter cannot tell flour from milk. | [192][200][201][206] |
| CZ, SK, PL | mass | Edited publishers write g; dkg/dag is the traditional register. | [192][194][195][200][201][206] |
| RU, UA | volume | ml at Klopotenko and in Gastronom's hedged form; the стакан has no stated size on any recipe page, so a converter cannot choose between 200 and 250 ml. | [182][183][188] |
| GB, IE | volume, mass, length | Metric is always first and often alone; GFW recommends metric only; the imperial figure is a bracketed secondary indication, which is a different feature from best-unit choice. Confirms ADR 0005's "GB has no rows". | [84][85][89][90][96][98][100] |
| GB, IE | temperature | °C is always present; gas mark and °F are secondary displays. | section 5 |
| AU, NZ, ZA, CA, IN, JP, KR, CN, TW, HK, MX, BR, AR, CO | temperature | °C (except English Canada, section 7). | section 5 |
| AU, NZ, ZA, IN, JP, KR, CN, TW, HK, LatAm | mass | g (CO libra and occasional 斤/台斤 excepted, section 7). | section 3 |
| CN, HK | volume | 克 for liquids and 适量; ml is the closest expressible fallback and is understood. A mass-for-volume rule needs densities, outside this table. | [166]–[170], [177] |
| AR | volume (liquids) | cc/ml for liquids is metric; "cc" is a *name* for `milliliter` in es-AR, a locale matter. tazas are for dry goods. | [233][234][238] |
| Everywhere except IN, CA, GB, IE | length | cm. | section 4 |

---

## 7. Proposed rows

Format: `(Region, Quantity, [unit ≥ threshold, …])`, largest first, using the identifiers in
`src/convert/units/definitions.rs`. A threshold is the amount of *that unit* the value must
reach. Thresholds below 1 are inferred from exhibits (`½ dl`, `¼ cup`, `½ tsk`); none is stated by
any source. Units marked **missing** do not exist in `definitions.rs`.

### 7.1 Rows the evidence supports

| Row | Rationale |
|---|---|
| `(SE, Volume, [liter ≥ 1, deciliter ≥ 0.5, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.5, nordic-spice-measure ≥ 1])` | Section 2, SE. 52 ml would become `½ dl` where Livsmedelsverket wrote `3,5 msk`; the overlap zone is 3–4 spoons ≈ ½ dl and either is idiomatic. `10 dl vatten` also occurs, so `liter ≥ 1` is a choice, not a rule. |
| `(NO, Volume, [liter ≥ 1, deciliter ≥ 0.5, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25, nordic-spice-measure ≥ 1])` | MatPrat writes `1,5 l` and `0,25 ts`. No `½ dl` exhibit was obtained for Norway; the 0.5 is carried over from SE/DK/FI. |
| `(DK, Volume, [liter ≥ 0.25, deciliter ≥ 0.5, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.5])` | Danish writes `½ l`, `¼ l` rather than `5 dl`, `2½ dl`; Arla DK lists ¼ l, ½ l, 1 l as units. No krm. Whether `0,25 l` renders as `¼ l` is a formatting matter outside this note. |
| `(FI, Volume, [liter ≥ 1, deciliter ≥ 0.5, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.5, nordic-spice-measure ≥ 1])` | `1 l maitoa` but `8 dl vettä`; `½ dl siirappia`; `4 rkl`; `½ tl`. |
| `(IS, Volume, …)` as SE | One source; brief check. |
| `(CH, Volume, [liter ≥ 1, deciliter ≥ 1, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` | `3,6 dl`, `1 dl Milch`, `¾ TL Salz`. No ½ dl exhibit; Swiss decimals (`7,25 dl`) suggest dl is kept down to 1 dl, then EL. |
| `(HU, Volume, [liter ≥ 1, deciliter ≥ 0.5, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` | `2 dl tej`, `1.25 dl tej`, `0.25 tk só`. The 0.5 dl threshold is inferred; Mindmegette writes `1 dl olaj` and `3 ek tej` (45 ml). mk (2 ml) exists but is 2° only; leave it out. |
| `(HU, Mass, [kilogram ≥ 0.5, decagram ≥ 0.5, gram ≥ 1])` | `1/2 kg liszt`; `25 dkg`; `2 dkg élesztő`; `0.9 dkg sütőpor`; `1/2 dkg só`. Hungarian does not drop to grams; the `gram` row is a floor for sub-5 g values no exhibit covers. The strongest non-Nordic case in this note. |
| `(FR, Volume, [liter ≥ 1, centiliter ≥ 5, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.5])` | `60 cl`, `25 cl`, `5 cl`, `3 cuillères à soupe`; a 5 cl threshold keeps 15–45 ml as spoons, as Marmiton prints them. 1,5 cl = 1 c. à s. is the French definition. |
| `(BE, Volume, [liter ≥ 1, deciliter ≥ 1, milliliter ≥ 50, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` | VRT: `4 dl`, `1 dl room`, `80 ml room`, `50 ml melk`, `1 el`. Flemish only; Francophone Belgium not researched. A `milliliter` row between dl and spoons expresses "50–99 ml in ml". |
| `(AU, Volume, [metric-cup ≥ 0.25, australian-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` — **missing**: `australian tablespoon` (20 ml; line 191) | SBS, AWW, Edmonds, Chelsea all state the 20 ml spoon. `ml (odd)`, i.e. `100ml buttermilk` rather than `⅖ cup`, cannot be expressed by a threshold table; it needs a "clean fraction, else ml" rule. |
| `(NZ, Volume, [metric-cup ≥ 0.25, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` | Edmonds' stated standard. Same `ml (odd)` caveat. |
| `(CA, Volume, [metric-cup ≥ 0.25, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` | Health Canada 250/15/5; both languages use the 250 ml cup; only the bracket order differs. |
| `(JP, Volume, [japanese-cup ≥ 0.25, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` — **missing**: `japanese cup` (200 ml; line 211). Note CLDR's `cup-jp` is 100 ml (cldr-unit-data.md section 5), so the id must not be seeded from it | Five publishers state 200/15/5; NHK writes `カップ3/4`. `米 2合` is ingredient-specific (rice) and needs **missing** `go` (180.39 ml; line 206) plus an ingredient rule, outside a quantity row. `cc` vs `ml` is a name. |
| `(KR, Volume, [korean-cup ≥ 0.25, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` — **missing**: a 200 ml cup; the same unit as Japan's by value (line 271), so one id (`cup-200ml`?) could serve both | 만개의레시피 and 우리의식탁 state it. |
| `(BR, Volume, [liter ≥ 1, cup-240ml ≥ 0.25, metric-tablespoon ≥ 1, metric-dessert-spoon ≥ 1, metric-teaspoon ≥ 0.5])` — **missing**: a 240 ml cup (`us cup` is 236.6 ml, within 1.5 %; the FDA label cup is 240, line 92) | Three publishers state 240/15/5; colher de sobremesa 10 ml and colher de café 2,5 ml are 2° or single-publisher. Labels use 200/10 (ANVISA) — a different context. |
| `(MX, Volume, [liter ≥ 1, cup-240ml-or-metric-cup ≥ 0.25, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.5])` | Kiwilimón and Cocina Fácil write tazas; the cup size (240 vs SMAE's 250) is unresolved (section 8). |
| `(CO, Mass, [kilogram ≥ 1, colombian-libra ≥ 0.25, gram ≥ 1])` — **missing**: `colombian libra` (500 g; line 590) | `¼ de libra de arveja (125 g)`, `½ libra`, `1 libra`. One publisher, consistently, plus the SIC statement. |
| `(IN, Length, [inch ≥ 0.5, centimeter ≥ 1])` | `1 1/2 inch Ginger` at Archana's and Sanjeev Kapoor. Thin: three exhibits, all ginger and cinnamon. |

### 7.2 Rows that need a decision first

| Row | What is unresolved |
|---|---|
| `(AT, Mass, [kilogram ≥ 1, decagram ≥ 1, gram ≥ 1])` | ADR 0005 names Austrian decagrams as the example region row. The evidence: ichkoche.at user recipes write `25 dag`; gutekueche.at, Plachutta, Sacher, Dr. Oetker AT and ichkoche's own editorial recipe write g. Adding the row serves the traditional register and surprises readers of every edited Austrian publisher. |
| `(PT, Volume, [liter ≥ 1, deciliter ≥ 0.5, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.5])` | Teleculinária and Vaqueiro: dl. DGS and Pingo Doce (half its recipes): ml. Two of four. |
| `(ZA, Volume, …)` as NZ | Huletts and Food24 write all-ml down to `3ml salt`; Food & Home writes cups. Either register is defensible; the all-ml one is what the Metric fallback produces (with spoons written as ml). |
| `(IN, Volume, [cup ≥ 0.25, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` | Which cup: NIN 200 ml (nutrition), Archana's 240, IDA's "large" 250. No recipe site states its size except Tarla Dalal (200, snippet). |
| `(CO, Volume, …)` | Resolución 810 permits 200 *or* 240 ml; Nestlé CO exhibits 250. |
| `(TW, Volume, [cup-240ml ≥ 0.25, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.25])` | HPA defines 240 ml for nutrition; iCook exhibits use cc/ml more than 杯. Spoons only would also match (`6大匙=90CC`). |
| `(PL, Volume, [liter ≥ 1, metric-cup ≥ 0.25, metric-tablespoon ≥ 1, metric-teaspoon ≥ 0.5])` | szklanka = 250 ml is stated by two publishers, but it is used for flour and sugar; water is `650 ml`. |
| `(CA, Temperature, [fahrenheit])` | English Canada prints °F first everywhere checked; Quebec °C first. The region row cannot see the language. Options: key on `en-CA` at the locale layer, or leave CA Celsius and accept that English-Canadian output is wrong-order. |
| `(US, …)` dual display, `(GB, …)` bracketed imperial, `(FR, …)` thermostat, `(GB/IE, …)` gas mark, `(BR/AR, …)` band names, fan-oven second figure | All are *secondary* displays printed beside the primary unit, not best-unit choices. They need a separate feature (and for gas mark and thermostat a point table, cooking-units.md lines 109–125). The UK rounding tables in section 3 are the data such a feature would need (25 g = 1 oz, 450 g = 1 lb, 500 g = 1 lb 2 oz, 568 ml = 1 pint, 150 ml = ¼ pint). |

### 7.3 Cross-cutting decision: spoons in the Metric system row

Every region row above repeats `metric-tablespoon ≥ 1, metric-teaspoon ≥ …`. The alternative is to
change the Metric system row to `[liter ≥ 1, milliliter ≥ 50, metric-tablespoon ≥ 1,
metric-teaspoon ≥ 0.25, milliliter ≥ 0]` (or similar) so that DE, IT, ES, NL, AT, CZ, PL, RU, UA,
JP mass-style regions get spoons without a row each. Evidence for: sections 2 and 6 — spoons are
printed for 5–45 ml in every one of those regions. Evidence against: mainland China (克), South
Africa's traditional register (`5ml baking powder`), Directo al Paladar (`15 ml`), Argentina
(`Aceite 20 cc`), Portugal's DGS and India's NIN (spoon sizes vary, so they annotate grams). This
note does not decide it; it changes every metric reader's output and is an ADR-level change.

### 7.4 Refinement of the Imperial system row (for `-u-ms-uksystem`)

UK exhibits suggest `imperial-pint ≥ 0.5` rather than `≥ 1` (`275ml/½ pint` is as common as
`300ml/10fl oz`; `150ml/¼ pint` at Odlums), and `pound ≥ 1` with a compound `1lb 2oz` for 500 g.
Both are secondary-indication practice and 2–3 exhibits each; recorded, not proposed.

### 7.5 Units a region row would need that do not exist

| Unit | Value | Needed by | Value source |
|---|---|---|---|
| Australian tablespoon | 20 ml | AU | cooking-units.md line 191 |
| 200 ml cup (カップ, 컵, NIN cup) | 200 ml | JP, KR, (IN) | lines 211, 271, 294 |
| 240 ml cup (xícara, taza, 杯 TW, FDA cup) | 240 ml | BR, MX, TW, (CO) | lines 92, 597; [220][221][171] |
| gō 合 | 180.39 ml (nominal 180) | JP rice | lines 206–207 |
| Colombian libra | 500 g | CO | line 590 |
| gas mark, thermostat, Stufe | point tables, non-linear | GB, IE, FR, DE secondary display | lines 116–121 |
| mokkáskanál | 2 ml (2°) | HU, optional | [215] |
| colher de café | 2.5 ml (one publisher) | BR, optional | [221] |

---

## 8. Open questions / unverified

**No primary source could be found for:**
- Any volume switching threshold, anywhere. All thresholds in section 7 are inferred.
- Any national standard for measuring cups or spoons outside the Thai and Indonesian nutrition
  rules already in `cooking-units.md`: no SIS/NS/DS/SFS (Nordic), DIN/ÖNORM/SN, AFNOR/UNI/UNE/IPQ,
  BS (BS 1340 is kerbs, not spoons), CGSB (Canada), BIS (India), JIS (Japan: chef 樋口直哉 says there
  is none and the JIS S index has no S 2050/S 3009 — the numbers circulating on blog tools look
  wrong), KS (Korea), CNS (Taiwan), NOM/ABNT/IRAM/NTC (Latin America). AS 1325 (Australia) exists
  but its text was behind a paywall.
- A government or food-agency statement of *recipe* conventions in any country. Food agencies
  define nutrition household measures (Voedingscentrum 12/2 ml spoons; NIN 200 ml cup; HPA 240 ml
  cup; GAPA g/cc; ANVISA 200/10 ml; Resolución 810 200-or-240 ml) and these often **differ from
  recipe practice** in the same country.
- A public-broadcaster recipe style guide (SVT, NRK, DR, Yle, ARD/ZDF, ORF, SRF, NPO, France TV, RAI,
  RTVE, RTP, NHK, KBS, CCTV, TVP, ČT, MTVA, ABC, SBS beyond its Cook's Notes box, RNZ, SABC, CBC,
  Canal Once, TV Brasil, TV Pública, Señal Colombia). Only the BBC (Good Food conversion guide),
  VRT, TV Pública/Cocineros Argentinos, NHK and RTÉ were read, and only as exhibits.
- Which stakan a Russian or Ukrainian recipe means (200 or 250 ml) and whether ГОСТ 8022-75 or
  5717-81 defines the faceted glass.
- The Russian tablespoon (15, 18 or 20 ml) and the десертная ложка value (no page gave one).
- Mexico's culinary taza (240 per a blog; 250 per SMAE's "¼ de litro"); Colombia's pocillo in ml;
  India's recipe cup; Hong Kong's 杯 and 湯匙 in commercial recipes.
- Whether German recipes still print a gas Stufe line (Chefkoch could not be fetched); whether
  "Tasse" or "Pfd." are live Chefkoch units.
- The Austrian edited press (ORF Nachlese, Kronen Zeitung) on dag vs g.
- Belgian francophone recipes; Italian dl in print cookbooks; Czech Vareni.cz and Apetit; Hungarian
  Nosalty; Polish Przepisy.pl; Russian Povarenok; taste.com.au, Coles, Woolworths, CSIRO; Canadian
  Living, Coup de pouce, CBC; Tarla Dalal; Food24 and Snowflake (live); Guardian and Tesco recipe
  pages; safefood and FSAI; Allerhande, Smulweb, Libelle Lekker; Globo/Ana Maria Braga; Recetas
  Nestlé MX and ES; Cocina Delirante. All blocked automated fetching, returned 404, or were not
  reached before the search quota ran out.

**Conflicts between sources:**
- Voedingscentrum (NL) defines 1 eetlepel = 12 ml and 1 theelepel = 2 ml; Dutch recipe sites use
  15/5 [41].
- Swissmilk says 1 EL = 12–15 ml; ichkoche and Dr. Oetker say 15 [36][26].
- Delia's tablespoon is 20 ml; the UK norm (GFW) is 15 [87][85].
- recepty.cz says hrnek = 250 ml and, in the same article, 3 dl [198].
- TudoGostoso says the xícara is 240 ml in one article and 250 ml in another [219][222].
- Brazilian labels (ANVISA 200 ml / 10 ml) vs recipes (240 / 15) [217][219].
- UK rounding: 1 oz = 25 g (Delia) vs 25/30 g (GFW); 4 oz = 110 g vs 115 g; ½ pint = 275 ml vs
  284 ml exact; 1 pint = 570 ml (Delia) vs 568 ml (GFW) [85][86].
- Italian oven bands: moderato = 170–190, 175–190 or 180–190 °C depending on the table [65][66].
- German gas Stufe 1 = 150 °C (one table) or 140 °C (another) [46][47].
- Korean 종이컵: stated 180 ml [161], exhibited as 140 ml (`5컵 (700ml)`) [165], described as
  180–190 ml (2°).
- Hungarian bögre: 2,5 dl [215] or 2–2,5 dl (2°); in practice Mindmegette writes dl.

**Not researched:** Francophone Belgium and Switzerland, Luxembourg, Greece, Turkey, Israel, the Arab
world, Iran, South-East Asia (recipe practice; unit values are in `cooking-units.md`), Africa
outside South Africa, the Caribbean, Chile and Peru beyond one source each.

---

## References

All fetched 2026-10-04 unless marked (snippet). Grouped by region cluster in the order the tables
cite them.

Nordic
- [1] Köket.se, vikt och volymer. https://www.koket.se/vikt-och-volymer-nar-du-ska-baka-eller-laga-mat
- [2] recepten.se, vikt och volym. https://www.recepten.se/artiklar/vikt_volym.html
- [3] Livsmedelsverket, blomkålspaj (Nyckelhålsrecept). https://www.livsmedelsverket.se/matvanor-halsa--miljo/nyckelhalet/nyckelhalsrecept/vegetariska-recept/blomkalspaj/
- [4] Arla SE, pannkakor. https://www.arla.se/recept/pannkakor/
- [5] ICA, fisk- och saffranssoppa (`10 dl vatten`). https://www.ica.se/recept/fisk-och-saffranssoppa-459025/
- [6] Arla SE, köttsoppa (`1⅕ liter vatten`). https://www.arla.se/recept/kottsoppa/
- [7] Arla SE, sockerkaka (tin by volume; `175° eller 160° varmluft`). https://www.arla.se/recept/sockerkaka/
- [8] Store norske leksikon, Målenheter for matlaging. https://snl.no/m%C3%A5lenheter_for_matlaging
- [9] MatPrat, mål og vekt. https://www.matprat.no/artikler/mengde-mal-og-vekt/mal-og-vekt/
- [10] Språkrådet, forkortelser. https://sprakradet.no/godt-og-korrekt-sprak/rettskriving-og-grammatikk/forkortelser/
- [11] mattenett.no, begrunnelse for valg av mengdeenheter. http://www.mattenett.no/begrunnelse_for_valg_av_mengdeen.htm
- [12] MatPrat, kjøttkaker. https://www.matprat.no/oppskrifter/tradisjon/kjottkaker/
- [13] MatPrat, kjøttsuppe. https://www.matprat.no/oppskrifter/tradisjon/kjottsuppe/
- [14] Arla DK, danske mål og vægte. https://www.arla.dk/opskrifter/leksikon/viden/danske-mal-og-vagte/
- [15] Fødevarestyrelsen, Alt om mad, opskrifter. https://foedevarestyrelsen.dk/kost-og-foedevarer/alt-om-mad/soed-balance/foraeldre/opskrifter-og-inspiration/opskrifter
- [16] Arla DK, drømmekage fra Brovst. https://www.arla.dk/opskrifter/drommekage-fra-brovst/
- [17] (2°) Valdemarsro, espresso martini (`6 cl vodka`). https://www.valdemarsro.dk/espresso-martini/
- [18] Martat, ruoka-aineiden mitat ja painot. https://www.martat.fi/marttakoulu/ruoka/ruoanvalmistus/ruoka-aineiden-mitat-ja-painot/
- [19] Yhteishyvä, mittojen muuntotaulukko. https://yhteishyva.fi/ruoka-ja-reseptit/leivontavinkit-mittojen-muuntotaulukko/article-34042
- [20] Kielitoimiston ohjepankki, ruoka-alan lyhenteet. https://kielitoimistonohjepankki.fi/ohje/ruoka-alan-lyhenteet/
- [21] Valio, pannukakku. https://www.valio.fi/reseptit/pannukakku/
- [22] Yhteishyvä, paras kaalilaatikko. https://yhteishyva.fi/reseptit/paras-kaalilaatikko/recipe-4544
- [23] Valio, mustikkapiirakka (`Ø 24 cm`, `200°C`). https://www.valio.fi/reseptit/mustikkapiirakka/
- [24] (2°) sv.wikipedia, Kokboksmått. https://sv.wikipedia.org/wiki/Kokboksm%C3%A5tt
- [25] MS / Gott í matinn, hagnýtar upplýsingar; pönnukökur. https://www.gottimatinn.is/god-rad/hagnytar-upplysingar ; https://www.gottimatinn.is/uppskriftir/ponnukokur

Germany, Austria, Switzerland, Netherlands, Belgium
- [26] ichkoche.at, Mengenangaben und Umrechnungen. https://www.ichkoche.at/mengenangaben-und-umrechnungen-artikel-2730
- [27] ichkoche.at, Butterkekse (`50 dag glattes Mehl`). https://ichkoche.at/butterkekse-rezept-144582
- [28] ichkoche.at, Topfengugelhupf (`250 g Topfen`; `150 Grad Heißluft`). https://www.ichkoche.at/topfengugelhupf-rezept-234204
- [29] gutekueche.at, Gugelhupf Grundrezept. https://www.gutekueche.at/gugelhupf-grundrezept-rezept-21835
- [30] Brandstätter Verlag, Plachutta Kaiserschmarren. https://www.brandstaetterverlag.com/stories/plachutta-rezept-kaiserschmarren/
- [31] Hotel Sacher, Original Sacher-Torte Rezept. https://www.sacher.com/de/original-sacher-torte/rezept/
- [32] Dr. Oetker AT, Maße von Backformen. https://www.oetker.at/inspiration/tipps-tricks/t/masse-von-backformen
- [33] Dr. Oetker DE, Marmorkuchen. https://www.oetker.de/rezepte/r/marmorkuchen
- [34] Dr. Oetker DE, Umluft oder Ober-/Unterhitze. https://www.oetker.de/inspiration/tipps-tricks/t/umluft-oder-ober-und-unterhitze
- [35] Küchengötter, Maße und Gewichte. https://www.kuechengoetter.de/kuechenlexikon/masse-und-gewichte
- [36] Swissmilk, Gramm und Löffel. https://www.swissmilk.ch/de/rezepte-kochideen/tipps-tricks/gramm-und-loeffel-mengenangaben-im-rezept-umrechen/
- [37] Swissmilk, Ober-/Unterhitze und Umluft. https://www.swissmilk.ch/de/rezepte-kochideen/tipps-tricks/unterschied-ober-und-unterhitze-und-umluft/
- [38] Swissmilk, gedeckter Apfelkuchen; Rahm-Apfeltarte; Schokoladentorte. https://www.swissmilk.ch/de/rezepte-kochideen/rezepte/KO2023_SBLV_07/gedeckter-apfelkuchen/ ; https://www.swissmilk.ch/de/rezepte-kochideen/rezepte/KB_BZ2000_205/rahm-apfeltarte/ ; https://www.swissmilk.ch/de/rezepte-kochideen/rezepte/LM201003_19/schokoladentorte/
- [39] Betty Bossi, Pfirsich-Himbeer-Quarktorte. https://www.bettybossi.ch/de/rezepte/rezept/pfirsich-himbeer-quarktorte-10004530/
- [40] Fooby, Lasagne al forno. https://fooby.ch/de/rezepte/14229/lasagne-al-forno
- [41] Voedingscentrum, hoeveel gram/milliliter in theelepel en eetlepel. https://mobiel.voedingscentrum.nl/nl/gezonde-recepten/kookhulp/hoeveel-gram-milliliter-theelepel-eetlepel.aspx
- [42] (snippet) Allerhande, Hollandse kaassaus; slagroomtaart; kruidige chocolademelk. https://www.ah.nl/allerhande/recept/R-R810330/hollandse-kaassaus ; https://www.ah.nl/allerhande/recept/R-R10086/slagroomtaart ; https://www.ah.nl/allerhande/recept/R-R421988/kruidige-chocolademelk
- [43] puurgezond.nl, oventemperatuur elektrisch vs hetelucht. https://www.puurgezond.nl/eten/oven/recepten/de-temperatuur-die-jullie-vermelden-bij-recepten-is-dat-voor-een-elektrische-oven-of-voor-een-heteluchtoven
- [44] VRT Dagelijkse kost, gratin dauphinois; vol-au-vent; kalfsblanquette; appeltaart met vanilleroom. https://dagelijksekost.vrt.be/gerechten/gratin-dauphinois ; https://dagelijksekost.vrt.be/gerechten/vol-au-vent ; https://dagelijksekost.vrt.be/gerechten/kalfsblanquette-met-puree-en-bimi ; https://dagelijksekost.vrt.be/gerechten/appeltaart-met-vanilleroom
- [45] Lekker van bij ons (VLAM), vis uit de oven. https://www.lekkervanbijons.be/recepten/vis-uit-de-oven-met-boerenkoolpesto-kerstomaten-en-krieltjes
- [46] (2°) gesundheit-ernaehrung-fitness.de, Backtemperaturen umrechnen. https://gesundheit-ernaehrung-fitness.de/backtemperaturen-umrechnen/
- [47] (2°) kuechentipps.de, Gasherd Stufe in Grad. https://www.kuechentipps.de/kuechentricks/gasherd-welche-stufe-ist-welche-temperatur-in-grad-celcius-und-fahrenheit/

France, Italy, Spain, Portugal
- [48] Marmiton, pâte à crêpes. https://www.marmiton.org/recettes/recette_pate-a-crepes_12372.aspx
- [49] Marmiton, recipe 23007 (served a flan aux poires; `thermostat 4-5, 140°C`). https://www.marmiton.org/recettes/recette_gratin-dauphinois_23007.aspx
- [50] 750g, crème brûlée. https://www.750g.com/la-meilleure-facon-de-faire-une-creme-brulee-r207468.htm
- [51] Marmiton, recipe 12977 (served gelée de groseilles; `1.5 kg`). https://www.marmiton.org/recettes/recette_quiche-lorraine_12977.aspx
- [52] Gastronomie Hauts-de-France, minis crèmes brûlées. https://gastronomie-hautsdefrance.fr/recettes/recette/minis-cremes-brulees/
- [53] mangerbouger.fr (Santé publique France), quiche poireaux roquefort. https://www.mangerbouger.fr/manger-mieux/la-fabrique-a-menus/recettes/2619-quiche-poireaux-roquefort
- [54] mangerbouger.fr, gratin de chou-fleur (`25 cl`). https://www.mangerbouger.fr/manger-mieux/la-fabrique-a-menus/recettes/778-gratin-de-chou-fleur
- [55] 750g, convertir des cl en grammes. https://www.750g.com/convertir-des-cl-en-grammes-et-d-autres-equivalences-a-connaitre-a31333.htm
- [56] (2°) Supertoinette, mesures et équivalences. https://www.supertoinette.com/mesures-equivalences-culinaires.html
- [57] CuisineAZ, recipe 1456 (served rôti de bœuf; `th.8 (240°C)`). https://www.cuisineaz.com/recettes/quiche-lorraine-1456.aspx
- [58] (2°) tempsdecuissonparfait.com, thermostat → °C. https://www.tempsdecuissonparfait.com/thermostat-four-conversion-temperature/
- [59] Giallozafferano, Besciamella. https://ricette.giallozafferano.it/Besciamella.html
- [60] Giallozafferano, Risotto alla milanese (`Vino bianco 50 g`). https://ricette.giallozafferano.it/Risotto-alla-milanese.html
- [61] Cucchiaio d'Argento, crème caramel. https://www.cucchiaio.it/ricetta/ricetta-creme-caramel-ricetta-base/
- [62] Cucchiaio d'Argento, panna cotta. https://www.cucchiaio.it/ricetta/ricetta-panna-cotta/
- [63] Cucchiaio d'Argento, arista al forno. https://www.cucchiaio.it/ricetta/ricetta-arista-forno/
- [64] Giallozafferano, torta margherita (`forno statico … 190°`). https://ricette.giallozafferano.it/Torta-margherita.html
- [65] Donna Moderna, come usare il forno. https://ricette.donnamoderna.com/come-usare-forno
- [66] (2°) Giallozafferano blog, tempi e temperature; risotto blog (`tre etti e mezzo`). https://blog.giallozafferano.it/margaretmarilena/tempi-e-temperature/ ; https://blog.giallozafferano.it/suditaliaincucina/risotto-alla-milanese-antico-tradizionale/
- [67] (2°) meshi.it, un etto. https://meshi.it/un-etto/
- [68] Directo al Paladar, dulce de leche. https://www.directoalpaladar.com/postres/como-hacer-dulce-de-leche-receta
- [69] Directo al Paladar, bizcocho de leche y mantequilla. https://www.directoalpaladar.com/postres/bizcocho-de-leche-y-mantequilla-receta
- [70] Directo al Paladar, pollo al ajillo. https://www.directoalpaladar.com/recetas-de-carnes-y-aves/pollo-al-ajillo-receta
- [71] Hogarmania, bizcocho de leche esponjoso. https://www.hogarmania.com/cocina/recetas/desayunos/bizcocho-leche-esponjoso.html
- [72] Hogarmania, arroz con leche (`1/4 de l de nata`). https://www.hogarmania.com/cocina/recetas/postres/arroz-leche-8720.html
- [73] (2°) Recetas de Esbieta, medidas y equivalencias. https://www.recetasdesbieta.com/medidas-y-equivalencias/
- [74] DGS / PNPAS, creme de abóbora e pinhão. https://alimentacaosaudavel.dgs.pt/receitas/creme-de-abobora-e-pinhao/
- [75] DGS / PNPAS, almôndegas de pescada. https://alimentacaosaudavel.dgs.pt/receitas/almondegas-de-pescada-e-grao-de-bico/
- [76] Teleculinária, monthly roundup with ingredient lists (`4 dl de natas`). https://teleculinaria.pt/?p=6074
- [77] Teleculinária, queques. https://teleculinaria.pt/receitas/queques
- [78] Pingo Doce, bolo de chocolate com mousse (`0,7 dl natas`). https://www.pingodoce.pt/receitas/bolo-chocolate-com-mousse/
- [79] Pingo Doce, bolo de leite (`250 ml leite`). https://www.pingodoce.pt/receitas/bolo-de-leite/
- [80] Pingo Doce, arroz doce (`1 L leite`). https://www.pingodoce.pt/receitas/arroz-doce/
- [81] Vaqueiro, doces de colher (`0,5 dl de leite`). https://www.vaqueiro.pt/receitas/doces-de-colher
- [82] Vaqueiro, doce de leite (`1 l de leite`). https://www.vaqueiro.pt/recipes/doce-de-leite-198044
- [83] (2°) anossavida.pt, tabela de conversão chávena. https://anossavida.pt/artigos/tabela-conversao-alimentos-chavena-colher-mililitros-centilitros

United Kingdom, Ireland
- [84] BBC Good Food, conversion guides. https://www.bbcgoodfood.com/howto/guide/conversion-guides
- [85] Guild of Food Writers, Metrication (2018-03-14). https://www.gfw.co.uk/2018/metrication/
- [86] Delia Online, oven temperatures and conversions. https://www.deliaonline.com/information-centre/oven-temperatures-and-conversions
- [87] Delia Online, Delia's tablespoon measurement. https://www.deliaonline.com/ask-lindsey/delias-tablespoon-measurement
- [88] BBC Good Food, classic Victoria sandwich. https://www.bbcgoodfood.com/recipes/classic-victoria-sandwich-recipe
- [89] Guardian and Observer style guide, M (metric system). https://www.theguardian.com/guardian-observer-style-guide-m
- [90] BBC Food, Mary Berry's Victoria sponge. https://www.bbc.co.uk/food/recipes/mary_berrys_perfect_34317
- [91] BBC Food, mini Yorkshire puddings. https://www.bbc.co.uk/food/recipes/miniyorkshirepudding_11253
- [92] BBC Food, easy roast chicken (`1 x 2kg/4½lb`). https://www.bbc.co.uk/food/recipes/easy_roast_chicken_with_29774
- [93] BBC Food, apple crumble; cinnamon autumn rice. https://www.bbc.co.uk/food/recipes/apple_crumble_with_63159 ; https://www.bbc.co.uk/food/recipes/cinnamon_autumn_rice_12458
- [94] Nigella, chocolate Guinness cake. https://www.nigella.com/recipes/chocolate-guinness-cake
- [95] Jamie Oliver, perfect roast chicken. https://www.jamieoliver.com/recipes/chicken/perfect-roast-chicken/
- [96] Waitrose, Victoria sponge. https://www.waitrose.com/ecom/recipe/victoria-sponge
- [97] Sainsbury's magazine, classic Victoria sponge. https://www.sainsburysmagazine.co.uk/recipes/baking/classic-victoria-sponge
- [98] Odlums, scones (legacy; `150ml/¼pt`). https://odlums.ie/recipes/scones/
- [99] Odlums, Super Flour classic scones; Bakers Tips. https://odlums.ie/recipes/super-flour-classic-scones/ ; https://odlums.ie/bakers-tips/
- [100] Avoca, Fern House pancakes; Irish spring lamb stew; clementine curd naked cake. https://www.avoca.com/blogs/recipes/our-famous-fern-house-pancakes ; https://www.avoca.com/blogs/recipes/irish-spring-lamb-stew ; https://www.avoca.com/blogs/recipes/clementine-curd-naked-cake
- [101] (2°) UK Metric Association, cooking. https://ukma.org.uk/practical-help/cooking/
- [102] Bord Bia, cottage pie (soda bread and pot roast lines from snippets). https://www.bordbia.ie/recipes/beef-recipes/cottage-pie/
- [103] RTÉ, Donal's Italian Kitchen tiramisu cake (2026). https://www.rte.ie/lifestyle/recipes/2026/0922/1592365-donals-italian-kitchen-tiramisu-cake/
- [104] RTÉ, simple Christmas cake (Dr Oetker, 2012; `500 g (1lb 2oz)`). https://www.rte.ie/lifestyle/recipes/2012/1126/744698-dr-oetker-christmas-cake/
- [105] RTÉ, the ultimate sausage roll (2020). https://www.rte.ie/lifestyle/recipes/2020/1021/1172913-the-ultimate-sausage-roll-recipe/
- [106] Delia Online, pumpkin soup (imperial-first text). https://www.deliaonline.com/recipes/books/delias-complete-how-to-cook/pumpkin-soup-with-toasted-sweetcorn

Australia, New Zealand, South Africa, Canada, India
- [107] Australian Women's Weekly Food, measurement conversions. https://www.womensweeklyfood.com.au/how-to/measurement-conversions-1313/
- [108] New Idea, recipe measurement conversions. https://www.newidea.com.au/food/tips-advice/recipe-measurement-conversions/
- [109] SBS Food, ricciarelli (Cook's Notes box). https://www.sbs.com.au/food/recipe/italian-almond-biscuits-ricciarelli/j422q2ntf
- [110] SBS Food, strawberry and white chocolate picnic cake. https://www.sbs.com.au/food/recipe/strawberry-and-white-chocolate-picnic-cake/ox9smry5x
- [111] AWW Food, small batch scones. https://www.womensweeklyfood.com.au/recipe/baking/small-batch-scones/
- [112] Donna Hay, almond cake with vanilla syrup. https://www.donnahay.com.au/recipes/desserts-and-baking/almond-cake-with-vanilla-syrup
- [113] ABC News, one-pan fish (Julia Busuttil Nishimura, 2026-10-03). https://www.abc.net.au/news/2026-10-03/one-pan-fish-lemon-marjoram-potatoes-julia-busuttil-nishimura/107059508
- [114] Nutrition Australia, Australian Dietary Guidelines standard serves (eatforhealth.gov.au timed out). https://nutritionaustralia.org/app/uploads/2022/03/Australian-Dietary-Guidelines-Standard-serves.pdf
- [115] Edmonds, weights and measures. https://edmondscooking.co.nz/how-tos/weights-measures
- [116] Chelsea Sugar, better baking: measurements. https://www.chelsea.co.nz/resources/better-baking/measurements
- [117] Healthy Food Guide NZ, weights and measures conversion tables. https://www.healthyfood.com/weights-and-measures-conversion-tables/
- [118] Annabel Langbein, conversions. https://www.langbein.com/conversions
- [119] NZ Herald, chicken pasta bake. https://www.nzherald.co.nz/eatwell/recipes/chicken-pasta-bake/FEVYJ5N7TRMB56JTRWJ3HBHVEY/
- [120] RNZ, budget friendly sticky toffee pudding. https://www.rnz.co.nz/collections/recipes/budget-friendly-sticky-toffee-pudding
- [121] Heart Foundation NZ, Italian-style meatballs; hasselback apples. https://www.heartfoundation.org.nz/wellbeing/healthy-recipes/italian-style-meatballs-with-super-veggie-mash-and-feta ; https://www.heartfoundation.org.nz/wellbeing/healthy-recipes/hasselback-apples
- [122] Huletts, malva pudding; amasi. https://www.hulettssugar.co.za/recipe/malva-pudding/ ; https://www.hulettssugar.co.za/recipe/amasi/
- [123] Food & Home SA, conversion chart. https://www.foodandhome.co.za/how-to/quantity-conversion-chart-will-ever-need
- [124] Food & Home SA, sticky date pudding. https://www.foodandhome.co.za/recipes/sticky-date-pudding-with-caramel-sauce
- [125] Food & Home SA, dark chocolate berry cake (`125ml (½ cup)`, `7,5ml`). https://www.foodandhome.co.za/recipes/super-rich-dark-chocolate-berry-cake
- [126] (2°) Annelien Pienaar, omskakelingstabelle. https://annelienpienaar.co.za/algemene-omskakelingstabelle/
- [127] Health Canada, measure ingredients like a pro (= cooking-units.md [18]). https://www.canada.ca/en/health-canada/services/food-guide/eating-support/kitchen/cooking-skills/measure-ingredients-pro.html
- [128] Health Canada, table of reference amounts (= cooking-units.md [19]). https://www.canada.ca/en/health-canada/services/technical-documents-labelling-requirements/nutrition-labelling-table-reference-amounts-food.html
- [129] Ricardo, liquid measures converter. https://www.ricardocuisine.com/en/converter/liquid-measures
- [130] Ricardo, savez-vous bien mesurer vos ingrédients. https://www.ricardocuisine.com/chroniques/techniques/6-savez-vous-bien-mesurer-vos-ingredients
- [131] Dairy Farmers of Canada, maple apple oatmeal muffins. https://dairyfarmersofcanada.ca/en/canadian-goodness/recipes/maple-apple-oatmeal-muffins
- [132] UnlockFood (Dietitians of Canada), apple maple cheesecake. https://www.unlockfood.ca/en/Recipes/Desserts-and-Baked-Goods/Apple-Maple-Cheesecake.aspx
- [133] Ricardo EN, recipes 4546 and 8631 (ids resolve to a caramel and a barley soup). https://www.ricardocuisine.com/en/recipes/4546-banana-bread ; https://www.ricardocuisine.com/en/recipes/8631-the-best-chocolate-cake
- [134] Anna Olson, baking conversions. https://annaolson.ca/baking-conversions/
- [135] Ricardo FR, recettes 4546, 4718, 8631. https://www.ricardocuisine.com/recettes/4546-pain-aux-bananes ; https://www.ricardocuisine.com/recettes/4718-pouding-chomeur ; https://www.ricardocuisine.com/recettes/8631-gateau-au-chocolat-le-meilleur
- [136] (2°) Wikipedia, Metrication in Canada. https://en.wikipedia.org/wiki/Metrication_in_Canada
- [137] ICMR-NIN, Dietary Guidelines for Indians 2024 (Annexure I). https://nin.res.in/dietaryguidelines/pdfjs/locale/DGI_2024.pdf
- [138] ICMR-NIN, Dietary Guidelines for Indians 2011 (= cooking-units.md [60]). https://www.nin.res.in/downloads/DietaryGuidelinesforNINwebsite.pdf
- [139] Indian Dietetic Association, standardization of cooked foods. https://idaindia.com/wp-content/uploads/2017/07/STANDARDIZATION-OF-COOKED-FOODS.pdf
- [140] Archana's Kitchen, weights and measurement chart. https://www.archanaskitchen.com/weights-measurement-chart-conversion-table
- [141] (2°) Gayathri's Cook Spot, measurements. https://gayathriscookspot.com/measurements/
- [142] Hebbar's Kitchen, rava cake. https://hebbarskitchen.com/rava-cake-recipe-suji-cake-recipe/
- [143] Archana's Kitchen, Hyderabadi vegetable biryani. https://www.archanaskitchen.com/recipe/hyderabadi-vegetable-biryani
- [144] Sanjeev Kapoor, dal; eggless chocolate cake. https://www.sanjeevkapoor.com/Recipe/Dal-Makhani-Sanjeev-Kapoor-Kitchen-FoodFood.html ; https://www.sanjeevkapoor.com/Recipe/Eggless-Chocolate-Cake-Sanjeev-Kapoor-Kitchen-FoodFood.html

Japan, Korea, China, Taiwan, Hong Kong
- [145] Kikkoman, 計量の道具. https://www.kikkoman.co.jp/homecook/basic/measure/tools.html
- [146] Kikkoman, 計量のしかた. https://www.kikkoman.co.jp/homecook/basic/measure/measure.html
- [147] 味の素パーク, 調味料の計量. https://park.ajinomoto.co.jp/recipe/basic/chomiryo/
- [148] DELISH KITCHEN, 1カップは何ml. https://delishkitchen.tv/articles/893
- [149] Kurashiru, 計量の基本 (recipe page). https://www.kurashiru.com/recipes/14c6b225-3821-4c56-b7d5-d45b6e5f6e83
- [150] Mizkan, 計量について. https://www.mizkan.co.jp/ouchirecipe/basic/measure/
- [151] 女子栄養大学, 計量カップ・スプーンの歴史 (dispatch page). https://www.eiyo.ac.jp/eiyojcspecial/dispatch/dispatch01.html
- [152] (2°) kikakurui.com, JIS S division index (no S 2050 / S 3009). https://kikakurui.com/s/
- [153] (2°) 樋口直哉 on X, "計量カップや計量スプーンにはJIS規格がない". https://x.com/naoya_foodlab/status/1078821890796339200
- [154] NHK きょうの料理, 塩こうじチャーシュー. https://www.kyounoryouri.jp/recipe/606671_塩こうじチャーシュー.html
- [155] NHK きょうの料理, ひとり茶碗蒸し (`だし カップ3/4`). https://www.kyounoryouri.jp/recipe/606675_ひとり茶碗蒸し.html
- [156] NHK きょうの料理, ふわふわチョコレートケーキ (`直径18cm`, `170℃`). https://www.kyounoryouri.jp/recipe/606678_ふわふわチョコレートケーキ.html
- [157] Kikkoman, 和食 029 (`1と1/2カップ（300ml）`, `米 2合`). https://www.kikkoman.co.jp/homecook/washoku/029/
- [158] 白ごはん.com, かやくご飯. https://www.sirogohan.com/recipe/kayakugohan/
- [159] Cookpad, recipe 19320472 (`水 400cc`). https://cookpad.com/jp/recipes/19320472
- [160] Kurashiru, recipe 66e9362f (`割烹白だし 大さじ4`). https://www.kurashiru.com/recipes/66e9362f-a88e-4d7b-8402-b87989269884
- [161] 만개의레시피, 계량 가이드. https://m.10000recipe.com/recipe/view_weight.html
- [162] 우리의식탁, 계량 가이드. https://wtable.co.kr/guide/62757
- [163] (2°) Vegan News (Korea), cooking measures citing RDA (= cooking-units.md [52]). https://www.vegannews.co.kr/news/article.html?no=389930
- [164] 만개의레시피, recipe 4295567 (김치찌개). https://www.10000recipe.com/recipe/4295567
- [165] 만개의레시피, recipe 6835685 (`종이컵 5컵 (700ml)`). https://www.10000recipe.com/recipe/6835685
- [166] 下厨房, user conversion post 104644191. https://m.xiachufang.com/recipe/104644191/
- [167] 下厨房, user conversion post 104731376. https://m.xiachufang.com/recipe/104731376/
- [168] 下厨房, 红烧肉 103885768 (`料酒 20克`). https://www.xiachufang.com/recipe/103885768/
- [169] 下厨房, recipe 101714088 (`生抽 适量`). https://www.xiachufang.com/recipe/101714088/
- [170] 下厨房, 戚风 104553625 (6寸; `上下火150度`). https://www.xiachufang.com/recipe/104553625/
- [171] 衛生福利部國民健康署, 食物代換表 (2019). https://www.hpa.gov.tw/Pages/Detail.aspx?nodeid=543&pid=8382
- [172] 愛料理 iCook blog, 一杯是多少. https://icook.tw/blogs/70998
- [173] iCook, 紅燒肉 467441 (`五花肉 1斤`, `醬油 6大匙=90CC`). https://icook.tw/recipes/467441
- [174] iCook, 紅燒肉 482018. https://icook.tw/recipes/482018
- [175] iCook, 紅燒肉 494729 (all ml). https://icook.tw/recipes/494729
- [176] iCook, 紅燒肉 471234 (all 克). https://icook.tw/recipes/471234
- [177] 衞生署 EatSmart, recipe 1370. https://restaurant.eatsmart.gov.hk/b5/content.aspx?content_id=1370
- [178] HK Customs and Excise, weights and measures (= cooking-units.md [40]). https://www.customs.gov.hk/en/consumer_protection/weights_measures/index.html
- [179] 中国营养学会, T/CNSS 020-2023 食物交换份. https://www.cnssci.net/uploads/allimg/20240607/1-24060G41PM29.pdf

Russia, Ukraine, Poland, Czechia, Slovakia, Hungary
- [180] Eda.ru (via Rambler), блины тонкие на молоке. https://eda.rambler.ru/recepty/vypechka-deserty/blini-tonkie-na-moloke-so-slivochnim-maslom-35053
- [181] RussianFood, recipe 116893. https://www.russianfood.com/recipes/recipe.php?rid=116893
- [182] Gastronom.ru, recipe 3283 (`2 1/2 стакана (350 г)`). https://www.gastronom.ru/recipe/3283/bliny-na-moloke
- [183] Gastronom.ru, таблица мер. https://www.gastronom.ru/recipe/measures
- [184] (2°) zaedok.ru, сколько грамм в стакане. https://zaedok.ru/articles/skolko-gramm-v-stakane/
- [185] (2°) ecalc.ru, столовая ложка в мл. https://ecalc.ru/tablespoon-to-ml/
- [186] Ялуторовский музейный комплекс, гранёный стакан. https://kultura-to.ru/new/index.php/yalutorovskij-muzejnyj-kompleks/item/28461-granenyj-stakan-simvol-sssr
- [187] (2°) say7.info forum, деления духовки. https://forum.say7.info/topic11379.html
- [188] Klopotenko, млинці. https://klopotenko.com/zustrichaemo-maslyanu-abo-yak-prigotuvatu-mlunci/
- [189] Вікна (vikna.tv), млинці від Клопотенка. https://vikna.tv/styl-zhyttya/yizha/mlynczi-vid-klopotenka-reczept-prygotuvannya-v-domashnih-umovah/
- [190] (2°) pro100media.com.ua, скільки грамів у склянці. https://pro100media.com.ua/kulinariya/skilky-hramiv-tsukru-v-lozhtsi-sklyantsi/
- [191] Moje Wypieki, przelicznik kulinarny. https://mojewypieki.com/info/przelicznik-kulinarny
- [192] Aniagotuje, ciasto na naleśniki. https://aniagotuje.pl/przepis/jak-zrobic-ciasto-na-nalesniki
- [193] Kwestia Smaku, naleśniki. https://www.kwestiasmaku.com/kuchnia_polska/nalesniki/nalesniki.html
- [194] Kwestia Smaku, sernik mocha. https://www.kwestiasmaku.com/przepis/sernik-mocha
- [195] Dom Wypieków, domowe ciasto drożdżowe. https://www.domwypiekow.com/domowe-ciasto-drozdzowe/
- [196] PrzyslijPrzepis, drożdżówka (`10 dkg drożdży`). https://www.przyslijprzepis.pl/przepis/drozdzowka-125
- [197] (2°, snippet) stylowi.pl, dag recipes. https://stylowi.pl/1959200 ; https://stylowi.pl/1016503
- [198] recepty.cz magazín, pečení bez váhy. https://magazin.recepty.cz/clanky/jak-pect-a-varit-bez-vahy-aneb-peceni-se-lzickou-lzici-a-hrnkem-95.html
- [199] Kupi.cz, jak vážit bez váhy. https://www.kupi.cz/magazin/amp/clanek/4178-jak-vazit-bez-vahy
- [200] Toprecepty, moravské koláče. https://www.toprecepty.cz/recept/1298-moravske-kolace/
- [201] Toprecepty, jihočeské kynuté koláče. https://www.toprecepty.cz/recept/29316-jihoceske-kynute-kolace/
- [202] (2°) naplavy.estranky.cz, původní recepty (`16dkg`, `1dl oleje`). https://naplavy.estranky.cz/clanky/puvodni/puvodni_-neroztridene-recepty.html
- [203] Sandtnerová transcription, závarky a vložky (`4 dkg mouky`). https://zlateprasatko.cz/sandtnerova-zavarky-a-vlozky
- [204] (2°) cs.wikipedia, Dekagram. https://cs.wikipedia.org/wiki/Dekagram
- [205] (2°) autorskesperky.com, teploty trouby. https://autorskesperky.com/blog/post/recepty.html
- [206] Varecha.sk, najlepšie pečené buchty (modern). https://varecha.pravda.sk/recepty/najlepsie-pecene-buchty/79290-recept.html
- [207] Varecha.sk, lekvárové buchty (`30 dkg`, `2 dcl`). https://varecha.pravda.sk/recepty/lekvarove-buchty-fotorecept/37586-recept.html
- [208] Mindmegette, tejes piskóta. https://www.mindmegette.hu/recept/tejes-piskota
- [209] Mindmegette, hamis piskóta (`25 dkg liszt`, `1 dl olaj`). https://www.mindmegette.hu/recept/hamis-piskota
- [210] Mindmegette, kókuszos piskóta (`0.9 dkg sütőpor`, `1.25 dl tej`, `23×34 cm`). https://mindmegette.hu/kokuszos-piskota.recept
- [211] (2°) tyukanyo.hu, quoting Horváth Ilona (`1/2 kg liszt, 25 dkg vaj`). https://tyukanyo.hu/karacsonyi-desszertek/
- [212] Street Kitchen, gőzgombóc (`350 ml tej`, `20 dkg szilvalekvár`). https://streetkitchen.hu/receptek/perfekt-gozgomboc-makkal-es-vaniliaszosszal
- [213] Mindmegette, epres piskótatekercs. https://www.mindmegette.hu/recept/epres-tejszinhabos-piskotatekercs
- [214] Receptneked, palócleves; mézes-sörös sült csirke. https://receptneked.hu/levesek/palocleves/ ; https://receptneked.hu/szarnyas/mezes-soros-sult-csirke-almaval/
- [215] (2°) sussvelem.com, mérés mérleg nélkül. https://www.sussvelem.com/meres-merleg-nelkul-blog
- [216] Nassolda (Receptneked blog), gázsütő fokozatai. https://nassolda.receptneked.hu/2015/02/03/hany-fokon-sut-vajon-a-gazsuto/

Latin America
- [217] ANVISA, RDC 359/2003 anexo (= cooking-units.md [142]). https://bvsms.saude.gov.br/bvs/saudelegis/anvisa/2003/anexo/anexo_res0359_23_12_2003.pdf
- [218] ANVISA, IN 75/2020 Anexo VII (mirror). https://in75.tabelanutricional.com.br/
- [219] TudoGostoso, tabela de medidas (= cooking-units.md [144]). https://www.tudogostoso.com.br/noticias/tudo-sobre-tabela-de-medidas-a340.htm
- [220] Panelinha, medidores: os curingas. https://www.panelinha.com.br/blog/pitadas/Medidores-os-curingas-de-qualquer-cozinha
- [221] Receitas Nestlé BR, medidas para os ingredientes. https://www.receitasnestle.com.br/artigos/medidas-para-os-ingredientes-na-cozinha
- [222] TudoGostoso, como converter e medir (250 ml article). https://www.tudogostoso.com.br/noticias/aprenda-como-converter-e-medir-ingredientes-em-gramas-liquidos-secos-e-mais-a7008.htm
- [223] (2°) Deliway, medidas caseiras (copo americano 190 ml). https://www.deliway.com.br/blog/medidas-caseiras
- [224] TudoGostoso, bolo de cenoura (recipe 23). https://www.tudogostoso.com.br/receita/23-bolo-de-cenoura.html
- [225] Receitas Nestlé BR, pudim de Leite Moça (`forno médio (180ºC)`). https://www.receitasnestle.com.br/receitas/pudim-de-leite-moca
- [226] Receitas Nestlé BR, bolo de cenoura (`forma retangular (20 x 30 cm)`). https://www.receitasnestle.com.br/receitas/bolo-de-cenoura
- [227] SMAE annex hosted by UNAM (`Taza … ¼ de litro`). https://fisiologia.facmed.unam.mx/wp-content/uploads/2019/02/2-Valoraci%C3%B3n-nutricional-Anexos.pdf
- [228] Kiwilimón, pastel de plátano (unit toggle in page source). https://www.kiwilimon.com/receta/postres/pasteles/pastel-de-platano
- [229] Cocina Fácil, molletes con chorizo. https://www.cocinafacil.com.mx/recetas/molletes-con-chorizo
- [230] (2°) Receiteria, temperatura de forno. https://www.receiteria.com.br/dicas/temperatura-de-forno/
- [231] Recetas Nestlé AR, equivalencias de cocina. https://www.recetasnestle.com.ar/escuela-de-sabor/tips/equivalencias-de-cocina
- [232] (2°) La Hora del Chef, equivalencias. https://lahoradelchef.com.ar/blog/equivalencias-cocina-cucharadas-gramos-tazas-mililitros
- [233] Ministerio de Salud (AR), Guías Alimentarias para la Población Argentina. https://www.argentina.gob.ar/sites/default/files/bancos/2020-08/guias-alimentarias-para-la-poblacion-argentina.pdf
- [234] Cocineros Argentinos (TV Pública), milanesas de cerdo con puré. https://cocinerosargentinos.com/recetas/carne/milanesas-de-cerdo-con-pure
- [235] Cocineros Argentinos, budín de pan (`Crema 200 cc`). https://cocinerosargentinos.com/recetas/postres/budin-de-pan-con-dulce-de-leche-y-bananas
- [236] Paulina Cocina, pan integral casero. https://www.paulinacocina.net/receta-pan-integral-casero/10252
- [237] Paulina Cocina, torta 1-2-3 (`mínimo-medio (180º)`). https://www.paulinacocina.net/torta-re-facil-torta-de-yogurt-1-2-3/2201
- [238] Recetas Nestlé AR, pastel 3 leches. https://www.recetasnestle.com.ar/recetas/pastel-3-leches
- [239] Cucinare / eltrece, temperatura del horno. https://www.eltrecetv.com.ar/cucinare/2019/08/19/temperatura-del-horno-todo-lo-que-tenes-que-saber/
- [240] Colombia, Resolución 810 de 2021 (MinSalud), art. 3 and 13. https://normas.cra.gov.co/gestor/docs/resolucion_minsaludps_0810_2021.htm ; https://faolex.fao.org/docs/pdf/col210685.pdf
- [241] ICBF / MinSalud, Guías Alimentarias Basadas en Alimentos. https://www.minsalud.gov.co/sites/rid/Lists/BibliotecaDigital/RIDE/VS/PP/SNA/guias-alimentarias-basadas-en-alimentos.pdf
- [242] Recetas Nestlé CO, sobrebarriga en salsa criolla; mondongo; estofado de carne. https://www.recetasnestle.com.co/recetas/sobrebarriga-en-salsa-criolla ; https://www.recetasnestle.com.co/recetas/mondongo ; https://www.recetasnestle.com.co/recetas/estofado-de-carne
- [243] Recetas Nestlé CO, arroz paisa; mazamorra (`1 taza de agua (250 ml)`). https://www.recetasnestle.com.co/recetas/arroz-paisa ; https://www.recetasnestle.com.co/recetas/mazamorra
- [244] (2°) El Tiempo (2024), SIC on the libra. https://www.eltiempo.com/economia/finanzas-personales/sic-explica-que-la-carne-y-otros-alimentos-no-se-deben-vender-por-libras-en-colombia-por-que-3343386
- [245] (2°) colombia.com, tabla de equivalencias. https://www.colombia.com/gastronomia/tabla-de-equivalencias/
- [246] CENAN/INS (Peru), TAFERA 2016. https://cdn.www.gob.pe/uploads/document/file/1427367/TAFERA%202016%20VF.pdf.pdf
- [247] (2°) Cocina Chilena de Pilar Hernández, medidas y conversiones. https://cocinachilena.cl/datos-medidas-y-conversiones/
