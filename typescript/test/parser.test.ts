import { describe, expect, it } from "vitest";
import { CooklangParser, CooklangRendererBase } from "../src";

const recipeString = "Make your first recipe with an @ingredient!";

it("returns version number", () => {
  expect(CooklangParser.version).toBeDefined();
});

describe("parser with functional render", () => {
  const parser = new CooklangParser();

  it("returns full parse of recipe string", () => {
    const parsedRecipe = parser.parse(recipeString);
    expect(typeof parsedRecipe).toEqual("object");
  });

  it("returns pretty stringified recipe", () => {
    expect(
      parser.render(CooklangRendererBase.prettyString, recipeString)
    ).toEqual("Make your first recipe with an @ingredient!");
  });

  it("returns html recipe", () => {
    expect(parser.render(CooklangRendererBase.html, recipeString)).toEqual(
      "Make your first recipe with an @ingredient!"
    );
  });
});

describe("parser using renderer", () => {
  const parser = new CooklangParser().use(CooklangRendererBase);

  it("returns full parse of recipe string", () => {
    const parsedRecipe = parser.parse(recipeString);
    expect(typeof parsedRecipe).toEqual("object");
  });

  it("returns pretty stringified recipe", () => {
    expect(parser.render.prettyString(recipeString)).toEqual(
      "Make your first recipe with an @ingredient!"
    );
  });

  it("returns html recipe", () => {
    expect(parser.render.html(recipeString)).toEqual(
      "Make your first recipe with an @ingredient!"
    );
  });
});
