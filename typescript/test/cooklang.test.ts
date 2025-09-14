import { beforeAll, describe, expect, it } from "vitest";
import { CooklangRecipe, recipe as recipeFn } from "../src/parser";

const recipeString = "Make your first recipe with an @ingredient!";

it("returns version number", () => {
  expect(CooklangRecipe.version).toBeDefined();
});

describe("parser instance", () => {
  let recipe: any;
  let directParse: any;
  beforeAll(() => {
    recipe = new CooklangRecipe(recipeString);
    directParse = recipeFn(recipe.parse(recipeString));
  });

  it("returns pretty stringified recipe", () => {
    expect(recipe.render.prettyString()).toEqual("eventually pretty string");
  });

  it("returns basic html recipe", () => {
    expect(recipe.render.html()).toEqual("eventually html");
  });

  it("returns metadata list", () => {
    expect(recipe.metadata).toEqual(directParse.metadata);
  });

  it("returns ingredients list", () => {
    expect(recipe.ingredients).toEqual(directParse.ingredients);
  });

  it("returns sections list", () => {
    expect(recipe.sections).toEqual(directParse.sections);
  });

  it("returns cookware list", () => {
    expect(recipe.cookware).toEqual(directParse.cookware);
  });

  it("returns timers list", () => {
    expect(recipe.timers).toEqual(directParse.timers);
  });
});
