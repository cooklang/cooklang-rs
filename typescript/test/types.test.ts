import { it, expectTypeOf } from "vitest";
import { Parser } from "..";
import type { ScaledRecipeWithReport } from "..";

it("generates Recipe type", async () => {
  const parser = new Parser();
  const recipeRaw = "this could be a recipe";
  const recipe = parser.parse(recipeRaw);
  expectTypeOf(recipe).toEqualTypeOf<ScaledRecipeWithReport>();
});
