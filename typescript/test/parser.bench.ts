import { bench, describe } from "vitest";
import { CooklangParser, CooklangRecipe } from "../src";

const recipeString = "Make your first @recipe!";
describe("parser", () => {
  bench("instance", () => {
    const recipe = new CooklangRecipe(recipeString);
  });

  // init the parser outside of the bench which
  // technically saves a few cycles on the actual
  // bench result vs the instance bench, but this is
  // effectively the use case where you init once
  // and reuse that parser over and over
  const parser = new CooklangParser();
  bench("functional", () => {
    parser.parse(recipeString);
  });
});
