import {it, expect, describe} from "vitest";
import {
    CooklangParser,
    getNumericValue,
    getQuantityValue,
    getQuantityUnit,
    getFlatIngredients,
    getFlatCookware,
    getFlatTimers
} from "../index.js";

describe("Numeric Value Helpers", () => {
    it("extracts numeric values from quantities", () => {
        const input = `
Mix @flour{2%cups} with @water{500%ml}.
        `;

        const parser = new CooklangParser();
        const [recipe] = parser.parse(input);

        // Test getQuantityValue
        const flourQty = getQuantityValue(recipe.ingredients[0].quantity);
        expect(flourQty).toEqual(2);

        const waterQty = getQuantityValue(recipe.ingredients[1].quantity);
        expect(waterQty).toEqual(500);
    });

    it("extracts units from quantities", () => {
        const input = `
Mix @flour{2%cups} with @water{500%ml}.
        `;

        const parser = new CooklangParser();
        const [recipe] = parser.parse(input);

        const flourUnit = getQuantityUnit(recipe.ingredients[0].quantity);
        expect(flourUnit).toEqual("cups");

        const waterUnit = getQuantityUnit(recipe.ingredients[1].quantity);
        expect(waterUnit).toEqual("ml");
    });

    it("handles null quantities", () => {
        const input = `
Mix @flour with @water.
        `;

        const parser = new CooklangParser();
        const [recipe] = parser.parse(input);

        const flourQty = getQuantityValue(recipe.ingredients[0].quantity);
        expect(flourQty).toBeNull();

        const flourUnit = getQuantityUnit(recipe.ingredients[0].quantity);
        expect(flourUnit).toBeNull();
    });

    it("extracts start value from ranges", () => {
        const input = `
Add @sugar{1-2%cups}.
        `;

        const parser = new CooklangParser();
        const [recipe] = parser.parse(input);

        const value = getNumericValue(recipe.ingredients[0].quantity?.value);
        expect(value).toEqual(1); // Should return start of range
    });
});

describe("Flat List Helpers", () => {
    it("creates flat ingredient list", () => {
        const input = `
Mix @flour{2%cups} with @water{500%ml} and @salt.
        `;

        const parser = new CooklangParser();
        const [recipe] = parser.parse(input);

        const ingredients = getFlatIngredients(recipe);

        expect(ingredients).toHaveLength(3);

        expect(ingredients[0].name).toEqual("flour");
        expect(ingredients[0].quantity).toEqual(2);
        expect(ingredients[0].unit).toEqual("cups");
        expect(ingredients[0].displayText).toBeTruthy();

        expect(ingredients[1].name).toEqual("water");
        expect(ingredients[1].quantity).toEqual(500);
        expect(ingredients[1].unit).toEqual("ml");

        expect(ingredients[2].name).toEqual("salt");
        expect(ingredients[2].quantity).toBeNull();
        expect(ingredients[2].unit).toBeNull();
    });

    it("creates flat cookware list", () => {
        const input = `
Use #pot and #pan{2}.
        `;

        const parser = new CooklangParser();
        const [recipe] = parser.parse(input);

        const cookware = getFlatCookware(recipe);

        expect(cookware).toHaveLength(2);

        expect(cookware[0].name).toEqual("pot");
        expect(cookware[0].quantity).toBeNull();

        expect(cookware[1].name).toEqual("pan");
        expect(cookware[1].quantity).toEqual(2);
    });

    it("creates flat timer list", () => {
        const input = `
Cook for ~{10%minutes} then ~bake{30%minutes}.
        `;

        const parser = new CooklangParser();
        const [recipe] = parser.parse(input);

        const timers = getFlatTimers(recipe);

        expect(timers).toHaveLength(2);

        expect(timers[0].name).toBeNull();
        expect(timers[0].quantity).toEqual(10);
        expect(timers[0].unit).toEqual("minutes");
        expect(timers[0].displayText).toBeTruthy();

        expect(timers[1].name).toEqual("bake");
        expect(timers[1].quantity).toEqual(30);
        expect(timers[1].unit).toEqual("minutes");
    });

});
