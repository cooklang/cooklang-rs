import {describe, expect, it} from "vitest";
import {parse_ingredient, parse_ingredient_list, ingredient_edit_text, add_ingredient_quantities, cleaned_ingredient_name, ingredient_amount_quantity, ingredient_amount_chip} from "../index.js";

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

    it("adds stored quantities", () => {
        expect(add_ingredient_quantities("2%l", "1%l")).toEqual("3%l");
        expect(add_ingredient_quantities(undefined, undefined)).toBeUndefined();
        expect(() => add_ingredient_quantities("2%l", "2")).toThrow("cannot be added");
    });

    it("cleans a name", () => {
        expect(cleaned_ingredient_name("  tomatoes  ")).toEqual("tomatoes");
    });

    it("writes an amount as a quantity and a chip", () => {
        expect(ingredient_amount_quantity(2, "l")).toEqual("2%l");
        expect(ingredient_amount_quantity(3, undefined)).toEqual("3");
        expect(ingredient_amount_chip(2, "l")).toEqual("2 l");
        expect(ingredient_amount_chip(3, undefined)).toEqual("x3");
    });
});
