import {it, expect} from "vitest";
import {CooklangRecipe} from "../index.js";

it("keeps raw string", async () => {
    const input = `
---
title: Some title
description: Some description
---
some step
    `;

    const recipe = new CooklangRecipe(input);
    expect(recipe.rawString).toEqual(input);

});

it("can change raw string", async () => {
    const input = `
---
title: Some title
description: Some description
---
some step
    `;

    const recipe = new CooklangRecipe("");
    expect(recipe.rawString).toEqual("");
    expect(recipe.title).toEqual(undefined);

    recipe.rawString = input;
    expect(recipe.rawString).toEqual(input);
    expect(recipe.title).toEqual("Some title");
});

it("reads interpreted metadata", async () => {
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
    expect(recipe.tags).toEqual(new Set(["tag1", "tag2"]));
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

it("reads data", async () => {
    const input = `
---
title: Some title
---
A step. @ingredient #ware ~timer{2min}
    `;

    const recipe = new CooklangRecipe(input);
    expect(recipe.rawMetadata).toEqual(new Map([
        ["title", "Some title"],
    ]));
    expect(recipe.sections[0].content[0].type).toEqual("step");
    expect(recipe.ingredients[0].name).toEqual("ingredient");
    expect(recipe.cookware[0].name).toEqual("ware");
    expect(recipe.timers[0].name).toEqual("timer");
    expect(recipe.inlineQuantities).toEqual([]);
});