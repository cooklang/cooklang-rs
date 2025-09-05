import {it, expectTypeOf, expect} from "vitest";
import {Parser} from "../index.js";
import type {ScaledRecipeWithReport} from "../index.js";
import {Metadata} from "../pkg/cooklang_wasm.js";

it("generates Recipe type", async () => {
    const parser = new Parser();
    const recipeRaw = "this could be a recipe";
    const recipe = parser.parse(recipeRaw);
    expectTypeOf(recipe).toEqualTypeOf<ScaledRecipeWithReport>();
});

it("generates raw metadata", async () => {
    const parser = new Parser();
    const recipeRaw = `
---
title: value
---
aaa bbb
    `;
    const recipe = parser.parse(recipeRaw);
    expectTypeOf(recipe.recipe.raw_metadata).toEqualTypeOf<Metadata>();
    expect(recipe.recipe.raw_metadata.map["title"]).equals("value");
});
