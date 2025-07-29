//! AUTO GENERATED WITH `gen_canonical_tests.py`
use super::{runner, TestCase};
use test_case::test_case;
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: Add a bit of chilli
source: 'Add a bit of chilli

  '
"#
; "BasicDirection")]
#[test_case(r#"
result:
  metadata: {}
  steps: []
source: '-- testing comments

  '
"#
; "Comments")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: thyme
      quantity: 2
      type: ingredient
      units: sprigs
    - type: text
      value: '  and some text'
source: '@thyme{2%sprigs} -- testing comments

  and some text

  '
"#
; "CommentsAfterIngredients")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: thyme
      quantity: 2
      type: ingredient
      units: sprigs
source: '-- testing comments

  @thyme{2%sprigs}

  '
"#
; "CommentsWithIngredients")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: Heat oven up to 200°C
source: 'Heat oven up to 200°C

  '
"#
; "DirectionsWithDegrees")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: Heat 5L of water
source: 'Heat 5L of water

  '
"#
; "DirectionsWithNumbers")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Add '
    - name: chilli
      quantity: 3
      type: ingredient
      units: items
    - type: text
      value: ', '
    - name: ginger
      quantity: 10
      type: ingredient
      units: g
    - type: text
      value: ' and '
    - name: milk
      quantity: 1
      type: ingredient
      units: l
    - type: text
      value: .
source: 'Add @chilli{3%items}, @ginger{10%g} and @milk{1%l}.

  '
"#
; "DirectionWithIngredient")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Fry in '
    - name: frying pan
      quantity: 1
      type: cookware
source: 'Fry in #frying pan{}

  '
"#
; "EquipmentMultipleWords")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Fry in '
    - name: 7-inch nonstick frying pan
      quantity: 1
      type: cookware
source: 'Fry in #7-inch nonstick frying pan{ }

  '
"#
; "EquipmentMultipleWordsWithLeadingNumber")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Fry in '
    - name: frying pan
      quantity: 1
      type: cookware
source: 'Fry in #frying pan{ }

  '
"#
; "EquipmentMultipleWordsWithSpaces")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Simmer in '
    - name: pan
      quantity: 1
      type: cookware
    - type: text
      value: ' for some time'
source: 'Simmer in #pan for some time

  '
"#
; "EquipmentOneWord")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: frying pan
      quantity: 2
      type: cookware
source: '#frying pan{2}

  '
"#
; "EquipmentQuantity")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: frying pan
      quantity: three
      type: cookware
source: '#frying pan{three}

  '
"#
; "EquipmentQuantityOneWord")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: frying pan
      quantity: two small
      type: cookware
source: '#frying pan{two small}

  '
"#
; "EquipmentQuantityMultipleWords")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: milk
      quantity: 0.5
      type: ingredient
      units: cup
source: '@milk{1/2%cup}

  '
"#
; "Fractions")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: knife cut about every 1/2 inches
source: 'knife cut about every 1/2 inches

  '
"#
; "FractionsInDirections")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: milk
      quantity: 01/2
      type: ingredient
      units: cup
source: '@milk{01/2%cup}

  '
"#
; "FractionsLike")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: milk
      quantity: 0.5
      type: ingredient
      units: cup
source: '@milk{1 / 2 %cup}

  '
"#
; "FractionsWithSpaces")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Top with '
    - name: 1000 island dressing
      quantity: some
      type: ingredient
      units: ''
source: 'Top with @1000 island dressing{ }

  '
"#
; "IngredientMultipleWordsWithLeadingNumber")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Add some '
    - name: "\U0001F9C2"
      quantity: some
      type: ingredient
      units: ''
source: "Add some @\U0001F9C2\n"
"#
; "IngredientWithEmoji")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: chilli
      quantity: 3
      type: ingredient
      units: items
source: '@chilli{3%items}

  '
"#
; "IngredientExplicitUnits")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: chilli
      quantity: 3
      type: ingredient
      units: items
source: '@chilli{ 3 % items }

  '
"#
; "IngredientExplicitUnitsWithSpaces")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: chilli
      quantity: 3
      type: ingredient
      units: ''
source: '@chilli{3}

  '
"#
; "IngredientImplicitUnits")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: chilli
      quantity: some
      type: ingredient
      units: ''
source: '@chilli

  '
"#
; "IngredientNoUnits")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: 5peppers
      quantity: some
      type: ingredient
      units: ''
source: '@5peppers

  '
"#
; "IngredientNoUnitsNotOnlyString")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: tipo 00 flour
      quantity: 250
      type: ingredient
      units: g
source: '@tipo 00 flour{250%g}

  '
"#
; "IngredientWithNumbers")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: chilli
      quantity: some
      type: ingredient
      units: ''
    - type: text
      value: ' cut into pieces'
source: '@chilli cut into pieces

  '
"#
; "IngredientWithoutStopper")]
#[test_case(r#"
result:
  metadata:
    sourced: babooshka
  steps: []
source: '>> sourced: babooshka

  '
"#
; "Metadata")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'hello >> sourced: babooshka'
source: 'hello >> sourced: babooshka

  '
"#
; "MetadataBreak")]
#[test_case(r#"
result:
  metadata:
    cooking time: 30 mins
  steps: []
source: '>> cooking time: 30 mins

  '
"#
; "MetadataMultiwordKey")]
#[test_case(r#"
result:
  metadata:
    cooking time: 30 mins
  steps: []
source: '>>cooking time    :30 mins

  '
"#
; "MetadataMultiwordKeyWithSpaces")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: Add a bit of chilli
  - - type: text
      value: Add a bit of hummus
source: 'Add a bit of chilli


  Add a bit of hummus

  '
"#
; "MultiLineDirections")]
#[test_case(r#"
result:
  metadata:
    Cook Time: 30 minutes
    Prep Time: 15 minutes
  steps: []
source: '>> Prep Time: 15 minutes

  >> Cook Time: 30 minutes

  '
"#
; "MultipleLines")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: hot chilli
      quantity: 3
      type: ingredient
      units: ''
source: '@hot chilli{3}

  '
"#
; "MultiWordIngredient")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: hot chilli
      quantity: some
      type: ingredient
      units: ''
source: '@hot chilli{}

  '
"#
; "MultiWordIngredientNoAmount")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: chilli
      quantity: some
      type: ingredient
      units: ''
    - type: text
      value: ' cut into pieces and '
    - name: garlic
      quantity: some
      type: ingredient
      units: ''
source: '@chilli cut into pieces and @garlic

  '
"#
; "MutipleIngredientsWithoutStopper")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: thyme
      quantity: few
      type: ingredient
      units: sprigs
source: '@thyme{few%sprigs}

  '
"#
; "QuantityAsText")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - name: water
      quantity: 7 k
      type: ingredient
      units: ''
source: '@water{7 k }

  '
"#
; "QuantityDigitalString")]
#[test_case(r#"
result:
  metadata:
    servings: 1|2|3
  steps: []
source: '>> servings: 1|2|3

  '
"#
; "Servings")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: Preheat the oven to 200℃/Fan 180°C.
source: 'Preheat the oven to 200℃/Fan 180°C.

  '
"#
; "SlashInText")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Fry for '
    - name: ''
      quantity: 1.5
      type: timer
      units: minutes
source: 'Fry for ~{1.5%minutes}

  '
"#
; "TimerDecimal")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Fry for '
    - name: ''
      quantity: 0.5
      type: timer
      units: hour
source: 'Fry for ~{1/2%hour}

  '
"#
; "TimerFractional")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Fry for '
    - name: ''
      quantity: 10
      type: timer
      units: minutes
source: 'Fry for ~{10%minutes}

  '
"#
; "TimerInteger")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Fry for '
    - name: potato
      quantity: 42
      type: timer
      units: minutes
source: 'Fry for ~potato{42%minutes}

  '
"#
; "TimerWithName")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Let it '
    - name: rest
      quantity: ''
      type: timer
      units: ''
    - type: text
      value: ' after plating'
source: 'Let it ~rest after plating

  '
"#
; "SingleWordTimer")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Let it '
    - name: rest
      quantity: ''
      type: timer
      units: ''
    - type: text
      value: ', then serve'
source: 'Let it ~rest, then serve

  '
"#
; "SingleWordTimerWithPunctuation")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Let it '
    - name: rest
      quantity: ''
      type: timer
      units: ''
    - type: text
      value: ⸫ then serve
source: 'Let it ~rest⸫ then serve

  '
"#
; "SingleWordTimerWithUnicodePunctuation")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Let it '
    - name: rest
      quantity: ''
      type: timer
      units: ''
    - type: text
      value:  then serve
source: 'Let it ~rest then serve

  '
"#
; "TimerWithUnicodeWhitespace")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: It is ~ 5
source: 'It is ~ 5

  '
"#
; "InvalidSingleWordTimer")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Add some '
    - name: chilli
      quantity: some
      type: ingredient
      units: ''
    - type: text
      value: ', then serve'
source: 'Add some @chilli, then serve

  '
"#
; "SingleWordIngredientWithPunctuation")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Add '
    - name: chilli
      quantity: some
      type: ingredient
      units: ''
    - type: text
      value: ⸫ then bake
source: 'Add @chilli⸫ then bake

  '
"#
; "SingleWordIngredientWithUnicodePunctuation")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Add '
    - name: chilli
      quantity: some
      type: ingredient
      units: ''
    - type: text
      value:  then bake
source: 'Add @chilli then bake

  '
"#
; "IngredientWithUnicodeWhitespace")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: Message me @ example
source: 'Message me @ example

  '
"#
; "InvalidSingleWordIngredient")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Place in '
    - name: pot
      quantity: 1
      type: cookware
      units: ''
    - type: text
      value: ', then boil'
source: 'Place in #pot, then boil

  '
"#
; "SingleWordCookwareWithPunctuation")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Place in '
    - name: pot
      quantity: 1
      type: cookware
      units: ''
    - type: text
      value: ⸫ then boil
source: 'Place in #pot⸫ then boil

  '
"#
; "SingleWordCookwareWithUnicodePunctuation")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Add to '
    - name: pot
      quantity: 1
      type: cookware
      units: ''
    - type: text
      value:  then boil
source: 'Add to #pot then boil

  '
"#
; "CookwareWithUnicodeWhitespace")]
#[test_case(r#"
result:
  metadata: {}
  steps:
  - - type: text
      value: 'Recipe # 5'
source: 'Recipe # 5

  '
"#
; "InvalidSingleWordCookware")]
fn canonical(input: &str) {
    let test_case: TestCase = serde_yaml::from_str(input).expect("Bad YAML input");
    runner(test_case);
}
