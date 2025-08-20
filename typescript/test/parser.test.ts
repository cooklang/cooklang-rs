import { beforeAll, describe, expect, it } from "vitest";
import { CooklangParser } from "../src/parser";

it("returns version number", () => {
  const parser = new CooklangParser();
  expect(parser.version).toBeDefined();
});

describe("create and use instance", () => {
  let recipe: CooklangParser;
  beforeAll(() => {
    const recipeString = "hello";
    recipe = new CooklangParser(recipeString);
  });

  it("returns pretty stringified recipe", () => {
    expect(recipe.renderPrettyString()).toEqual("hello");
  });

  it("returns basic html recipe", () => {
    expect(recipe.renderPrettyString()).toEqual("hello");
  });

  it("returns metadata list", () => {
    expect(recipe.metadata).toEqual({});
  });

  it("returns ingredients list", () => {
    expect(recipe.ingredients).toEqual(new Map());
  });

  it("returns sections list", () => {
    expect(recipe.sections).toEqual([]);
  });

  it("returns cookware list", () => {
    expect(recipe.cookware).toEqual(new Map());
  });

  it("returns timers list", () => {
    expect(recipe.timers).toEqual([]);
  });
});

describe("functional", () => {
  const parser = new CooklangParser();
  const recipe = "hello";
  it("returns pretty stringified recipe", () => {
    const parsedRecipe = parser.renderPrettyString(recipe);
    expect(parsedRecipe).toEqual("hello");
  });

  it("returns basic html recipe", () => {
    const parsedRecipe = parser.renderHTML(recipe);
    expect(parsedRecipe).toEqual("hello");
  });

  it("returns full parse of recipe string", () => {
    const parsedRecipe = parser.parse(recipe);
    expect(parsedRecipe).toEqual({
      cookware: new Map(),
      ingredients: new Map(),
      metadata: {},
      sections: [],
      timers: [],
    });
  });

  it("returns metadata list", () => {
    const parsedRecipe = parser.parse(recipe);
    expect(parsedRecipe.metadata).toEqual({});
  });

  it("returns ingredients list", () => {
    const parsedRecipe = parser.parse(recipe);
    expect(parsedRecipe.ingredients).toEqual(new Map());
  });

  it("returns sections list", () => {
    const parsedRecipe = parser.parse(recipe);
    expect(parsedRecipe.sections).toEqual([]);
  });

  it("returns cookware list", () => {
    const parsedRecipe = parser.parse(recipe);
    expect(parsedRecipe.cookware).toEqual(new Map());
  });

  it("returns timers list", () => {
    const parsedRecipe = parser.parse(recipe);
    expect(parsedRecipe.timers).toEqual([]);
  });
});
