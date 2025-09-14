import { CooklangParser } from "./parser.js";
import { type ScaledRecipeWithReport } from "../pkg/cooklang_wasm";

export type Renderer = (parser: CooklangParser) => {
  render: (recipeString: string) => any;
};

export const CooklangRendererBase = {
  prettyString(parser: CooklangParser) {
    return {
      // TODO fix return with actual pretty string
      render: (recipeString: string) => recipeString,
      // only for class CooklangRecipe, not required on other external renderers
      renderWithParsed: (parsed: ScaledRecipeWithReport) =>
        "eventually pretty string",
    };
  },
  html(parser: CooklangParser) {
    return {
      // TODO fix return with actual html string
      render: (recipeString: string) => recipeString,
      // only for class CooklangRecipe, not required on other external renderers
      renderWithParsed: (parsed: ScaledRecipeWithReport) => "eventually html",
    };
  },
  debug(parser: CooklangParser) {
    // TODO debug parse this then return
    return {
      render: (recipeString: string) => ({
        version: CooklangParser.version,
        ast: recipeString,
        events: recipeString,
      }),
    };
  },
  recipe(parser: CooklangParser) {
    return {
      render: (recipeString: string) => {
        const parsed = parser.parse(recipeString);
        return recipe(parsed);
      },
    };
  },
};

export const recipe = (rawParsed: ScaledRecipeWithReport) => {
  return {
    ...rawParsed.recipe,
    ingredients: new Map(
      rawParsed.recipe.ingredients.map((recipe) => [recipe.name, recipe])
    ),
  };
};
