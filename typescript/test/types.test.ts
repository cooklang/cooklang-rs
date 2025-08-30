import {it, expectTypeOf, expect} from "vitest";
import {Parser} from "..";
import type {ScaledRecipeWithReport} from "..";
import {Metadata} from "../pkg";

it("generates Recipe type", async () => {
    const parser = new Parser();
    const recipeRaw = "this could be a recipe";
    const recipe = parser.parse(recipeRaw);
    expectTypeOf(recipe).toEqualTypeOf<ScaledRecipeWithReport>();
});

it("generates metadata", async () => {
    const parser = new Parser();
    const recipeRaw = `
---
key: value
---
aaa bbb
    `;
    const recipe = parser.parse(recipeRaw);
    expectTypeOf(recipe.recipe.metadata).toEqualTypeOf<Metadata>();
    expect(recipe.recipe.metadata.map["key"]).equals("value");
});
