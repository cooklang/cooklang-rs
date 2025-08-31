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
