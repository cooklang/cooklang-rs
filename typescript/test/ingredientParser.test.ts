import {describe, expect, it} from "vitest";
import {parse_ingredient, parse_ingredient_list, ingredient_edit_text} from "../index.js";

describe("ingredient parser", () => {
    it("reads an amount at the end", () => {
        const parsed = parse_ingredient("Sugar 1.5kg", undefined);
        expect(parsed.name).toEqual("Sugar");
        expect(parsed.amount).toEqual({value: 1.5, unit: "kg"});
    });

    it("matches the aisle configuration", () => {
        const parsed = parse_ingredient("Tomato 2", "[veg]\ntomatoes\n");
        expect(parsed.name).toEqual("tomatoes");
        expect(parsed.aisle_match?.category).toEqual("veg");
    });

    it("splits a pasted list", () => {
        const names = parse_ingredient_list("- Milk 2 l\n\n* 3 lemons", undefined).map(p => p.name);
        expect(names).toEqual(["Milk", "lemons"]);
    });

    it("builds the edit text", () => {
        expect(ingredient_edit_text("Milk", "2%l")).toEqual("Milk 2 l");
    });
});
