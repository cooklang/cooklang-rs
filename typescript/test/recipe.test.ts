import {it, expectTypeOf, expect} from "vitest";
import {CooklangRecipe, Parser} from "..";
import type {ScaledRecipeWithReport} from "..";

it("reads standard metadata", async () => {
    const input = `
---
title: Some title
description: Some description
tags: tag1,tag2
author: Author
source:
    name: wiki
    url: https://wikipedia.com
course: dinner
time: 3h 12min
servings: 23
difficulty: hard
cuisine: norwegian
diet: true
image: example.png
locale: en_US
custom1: 44
custom2: some string
---
    `;

    const recipe = new CooklangRecipe(input);
    expect(recipe.title).toEqual("Some title");
    expect(recipe.description).toEqual("Some description");
    expect(recipe.tags).toEqual(["tag1", "tag2"]);
    expect(recipe.author).toEqual({name: "Author", url: null});
    expect(recipe.source).toEqual({name: "wiki", url: "https://wikipedia.com"});
    expect(recipe.course).toEqual("dinner");
    expect(recipe.time).toEqual(192);
    expect(recipe.servings).toEqual(23);
    expect(recipe.difficulty).toEqual("hard");
    expect(recipe.cuisine).toEqual("norwegian");
    expect(recipe.diet).toEqual(true);
    expect(recipe.images).toEqual("example.png");
    expect(recipe.locale).toEqual(["en", "US"]);
    expect(recipe.custom_metadata.get("custom1")).toEqual(44);
    expect(recipe.custom_metadata.get("custom2")).toEqual("some string");
});