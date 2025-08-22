import { beforeAll, describe, expect, it } from "vitest";
import { CooklangParser } from "../src/parser";

const recipeString = "Make your first recipe with an @ingredient!";
const assertParsed = {
  cookware: new Map(),
  ingredients: new Map([
    [
      "ingredient",
      {
        alias: null,
        name: "ingredient",
        note: null,
        quantity: null,
        reference: null,
        relation: {
          defined_in_step: true,
          reference_target: null,
          referenced_from: [],
          type: "definition",
        },
      },
    ],
  ]),
  metadata: {},
  sections: [],
  timers: [],
};

it("returns version number", () => {
  const parser = new CooklangParser();
  expect(parser.version).toBeDefined();
});

describe("create and use instance", () => {
  let recipe: CooklangParser;
  beforeAll(() => {
    recipe = new CooklangParser(recipeString);
    console.dir(recipe);
  });

  it("returns pretty stringified recipe", () => {
    expect(recipe.renderPrettyString()).toEqual(
      "Make your first recipe with an @ingredient!"
    );
  });

  it("returns basic html recipe", () => {
    expect(recipe.renderHTML()).toEqual(
      "Make your first recipe with an @ingredient!"
    );
  });

  it("returns metadata list", () => {
    expect(recipe.metadata).toEqual(assertParsed.metadata);
  });

  it("returns ingredients list", () => {
    expect(recipe.ingredients).toEqual(assertParsed.ingredients);
  });

  it("returns sections list", () => {
    expect(recipe.sections).toEqual(assertParsed.sections);
  });

  it("returns cookware list", () => {
    expect(recipe.cookware).toEqual(assertParsed.cookware);
  });

  it("returns timers list", () => {
    expect(recipe.timers).toEqual(assertParsed.timers);
  });
});

describe("functional", () => {
  const parser = new CooklangParser();
  it("returns pretty stringified recipe", () => {
    const parsedRecipe = parser.renderPrettyString(recipeString);
    expect(parsedRecipe).toEqual("Make your first recipe with an @ingredient!");
  });

  it("returns basic html recipe", () => {
    const parsedRecipe = parser.renderHTML(recipeString);
    expect(parsedRecipe).toEqual("Make your first recipe with an @ingredient!");
  });

  it("returns full parse of recipe string", () => {
    const parsedRecipe = parser.parse(recipeString);
    expect(parsedRecipe).toEqual(assertParsed);
  });

  it("returns metadata list", () => {
    const parsedRecipe = parser.parse(recipeString);
    expect(parsedRecipe.metadata).toEqual(assertParsed.metadata);
  });

  it("returns ingredients list", () => {
    const parsedRecipe = parser.parse(recipeString);
    expect(parsedRecipe.ingredients).toEqual(assertParsed.ingredients);
  });

  it("returns sections list", () => {
    const parsedRecipe = parser.parse(recipeString);
    expect(parsedRecipe.sections).toEqual(assertParsed.sections);
  });

  it("returns cookware list", () => {
    const parsedRecipe = parser.parse(recipeString);
    expect(parsedRecipe.cookware).toEqual(assertParsed.cookware);
  });

  it("returns timers list", () => {
    const parsedRecipe = parser.parse(recipeString);
    expect(parsedRecipe.timers).toEqual(assertParsed.timers);
  });
});
