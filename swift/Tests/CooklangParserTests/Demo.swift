import CooklangParser
import XCTest

class Demo: XCTestCase {
  func test_demo() {
    let recipe = """
      Preheat the oven to 180C.

      Mix @flour{2%cups} with @baking powder{1%tsp}.

      Add @eggs{2} and stir for ~{2%minutes}.
      """

    let parsed = CooklangParser.parseRecipe(input: recipe)

    let expected = CooklangRecipe(
      metadata: [:],
      steps: [
        CooklangParser.Step(items: [CooklangParser.Item.text(value: "Preheat the oven to 180C.")]),
        CooklangParser.Step(items: [
          CooklangParser.Item.text(value: "Mix "),
          CooklangParser.Item.ingredient(
            name: "flour",
            amount: Optional(
              CooklangParser.Amount(
                quantity: CooklangParser.Value.number(value: 2.0), units: Optional("cups")))),
          CooklangParser.Item.text(value: " with "),
          CooklangParser.Item.ingredient(
            name: "baking powder",
            amount: Optional(
              CooklangParser.Amount(
                quantity: CooklangParser.Value.number(value: 1.0), units: Optional("tsp")))),
          CooklangParser.Item.text(value: "."),
        ]),
        CooklangParser.Step(items: [
          CooklangParser.Item.text(value: "Add "),
          CooklangParser.Item.ingredient(
            name: "eggs",
            amount: Optional(
              CooklangParser.Amount(quantity: CooklangParser.Value.number(value: 2.0), units: nil))),
          CooklangParser.Item.text(value: " and stir for "),
          CooklangParser.Item.timer(
            name: nil,
            amount: Optional(
              CooklangParser.Amount(
                quantity: CooklangParser.Value.number(value: 2.0), units: Optional("minutes")))),
          CooklangParser.Item.text(value: "."),
        ]),
      ],
      ingredients: [
        "baking powder": [
          CooklangParser.GroupedQuantityKey(
            name: "tsp", unitType: CooklangParser.QuantityType.number):
            CooklangParser.Value.number(value: 1.0)
        ],
        "eggs": [
          CooklangParser.GroupedQuantityKey(name: "", unitType: CooklangParser.QuantityType.number):
            CooklangParser.Value.number(value: 2.0)
        ],
        "flour": [
          CooklangParser.GroupedQuantityKey(
            name: "cups", unitType: CooklangParser.QuantityType.number):
            CooklangParser.Value.number(value: 2.0)
        ],
      ],
      cookware: []
    )

    XCTAssertEqual(parsed, expected)
  }
}
