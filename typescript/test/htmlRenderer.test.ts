import {it, expect} from "vitest";
import {CooklangRecipe, HTMLRenderer} from "../index.js";

it("renders metadata", async () => {
    const input = `
---
title: Some title
description: Some description
---
    `;
    const output = "<ul>" +
        "<li class='metadata'><span class='key'>title</span>: <span class='value'>Some title</span></li>" +
        "<li class='metadata'><span class='key'>description</span>: <span class='value'>Some description</span></li>" +
        "</ul><hr>";

    const recipe = new CooklangRecipe(input);
    const renderer = new HTMLRenderer();
    expect(renderer.render(recipe)).toEqual(output);
});

it("renders ingredients", async () => {
    const input = `
    @cat{3}(black)
    `;
    const output = "<h2>Ingredients:</h2><ul><li class='ingredients'><b>cat</b>: 3 (black)</li></ul>";

    const recipe = new CooklangRecipe(input);
    const renderer = new HTMLRenderer();
    expect(renderer.render(recipe)).toEqual(output);
});