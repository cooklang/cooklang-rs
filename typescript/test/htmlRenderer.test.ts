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
    const output = "<h2>Ingredients:</h2><ul><li><b>cat</b>: 3 (black)</li></ul><hr><b>1. </b>    <span class='ingredient'>cat<i>(3)</i></span>";

    const recipe = new CooklangRecipe(input);
    const renderer = new HTMLRenderer();
    expect(renderer.render(recipe)).toEqual(output);
});

it("renders cookware", async () => {
    const input = `
    #cauldron{3}(magic)
    `;
    const output = "<h2>Cookware:</h2><ul><li><b>cauldron</b>: 3 (magic)</li></ul><hr><b>1. </b>    <span class='ingredient'>cauldron<i>(3)</i></span>";

    const recipe = new CooklangRecipe(input);
    const renderer = new HTMLRenderer();
    expect(renderer.render(recipe)).toEqual(output);
});

it("renders timer", async () => {
    const input = `
    ~eon{5min}
    `;
    const output = "<b>1. </b>    <span class='timer'>(eon)<i>(5min)</i></span>";

    const recipe = new CooklangRecipe(input);
    const renderer = new HTMLRenderer();
    expect(renderer.render(recipe)).toEqual(output);
});

it("renders sections and steps", async () => {
    const input = `
=== aaa
    a
    
    b
    
=== bbb
    c
    
    d
    `;
    const output = "<h3>(1) aaa</h3><b>1. </b>    a<b>2. </b>    b<h3>(2) bbb</h3><b>1. </b>    c<b>2. </b>    d";

    const recipe = new CooklangRecipe(input);
    const renderer = new HTMLRenderer();
    expect(renderer.render(recipe)).toEqual(output);
});