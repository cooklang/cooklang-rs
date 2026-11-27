import {describe, expect, it} from "vitest";
import {parse_ingredient, parse_ingredient_list, ingredient_edit_text, add_ingredient_quantities, cleaned_ingredient_name, ingredient_amount_quantity, ingredient_amount_chip, quantity_to_cooklang} from "../index.js";
import type {Quantity} from "../index.js";

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
        expect(ingredient_edit_text("Milk", ingredient_amount_quantity(2, "l"))).toEqual("Milk 2 l");
        expect(ingredient_edit_text("Bread", undefined)).toEqual("Bread");
    });

    it("adds stored quantities", () => {
        const sum = add_ingredient_quantities(ingredient_amount_quantity(2, "l"), ingredient_amount_quantity(1, "l"));
        expect(sum && quantity_to_cooklang(sum)).toEqual("3%l");
        expect(add_ingredient_quantities(undefined, undefined)).toBeUndefined();
        expect(() => add_ingredient_quantities(ingredient_amount_quantity(2, "l"), ingredient_amount_quantity(2, undefined))).toThrow("cannot be added");
    });

    it("writes a quantity built by hand", () => {
        const two_litres: Quantity = {
            value: {type: "number", value: {type: "regular", value: 2}},
            unit: "l",
            scalable: false,
        };
        expect(quantity_to_cooklang(two_litres)).toEqual("2%l");
        const half_cup: Quantity = {
            value: {type: "number", value: {type: "fraction", value: {whole: 0, num: 1, den: 2, err: 0}}},
            unit: "cup",
            scalable: false,
        };
        expect(quantity_to_cooklang(half_cup)).toEqual("1/2%cup");
    });

    it("cleans a name", () => {
        expect(cleaned_ingredient_name("  tomatoes  ")).toEqual("tomatoes");
    });

    it("writes an amount as a quantity and a chip", () => {
        expect(quantity_to_cooklang(ingredient_amount_quantity(2, "l")!)).toEqual("2%l");
        expect(quantity_to_cooklang(ingredient_amount_quantity(3, undefined)!)).toEqual("3");
        expect(quantity_to_cooklang(ingredient_amount_quantity(2 / 3, "cup")!)).toEqual("0.666667%cup");
        expect(ingredient_amount_quantity(0, "g")).toBeUndefined();
        expect(ingredient_amount_chip(2, "l")).toEqual("2 l");
        expect(ingredient_amount_chip(3, undefined)).toEqual("x3");
    });
});
