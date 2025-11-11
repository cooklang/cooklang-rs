import { it, expect, describe } from "vitest";
import { Parser, type DebugInfo } from "../index.js";

describe("debug_info", () => {
    it("returns comprehensive debug information", () => {
        const parser = new Parser();
        const recipe = "-- title: Test Recipe\n\n@ingredient{1}\nSome @step.";

        const debug: DebugInfo = parser.debug_info(recipe);

        // Check all fields exist
        expect(debug).toHaveProperty("version");
        expect(debug).toHaveProperty("extensions");
        expect(debug).toHaveProperty("load_units");
        expect(debug).toHaveProperty("ast");
        expect(debug).toHaveProperty("events");
        expect(debug).toHaveProperty("full_recipe");
        expect(debug).toHaveProperty("metadata");
        expect(debug).toHaveProperty("report");

        // Check types
        expect(typeof debug.version).toBe("string");
        expect(typeof debug.extensions).toBe("number");
        expect(typeof debug.load_units).toBe("boolean");
        expect(typeof debug.ast).toBe("string");
        expect(typeof debug.events).toBe("string");
        expect(typeof debug.full_recipe).toBe("string");
        expect(typeof debug.metadata).toBe("string");
        expect(typeof debug.report).toBe("string");

        // Check version is valid
        expect(debug.version.length).toBeGreaterThan(0);
        expect(debug.version).not.toBe("<unspecified version>");

        // Check extensions is a valid bitmask
        expect(debug.extensions).toBeGreaterThanOrEqual(0);

        // Check that AST and recipe contain expected content (they're JSON)
        const ast = JSON.parse(debug.ast);
        expect(ast).toHaveProperty("blocks");

        const fullRecipe = JSON.parse(debug.full_recipe);
        expect(fullRecipe).toHaveProperty("ingredients");

        expect(debug.events.length).toBeGreaterThan(0);
    });

    it("respects parser configuration", () => {
        const parser = new Parser();
        parser.load_units = false;
        parser.extensions = 0;

        const recipe = "test";
        const debug = parser.debug_info(recipe);

        expect(debug.load_units).toBe(false);
        expect(debug.extensions).toBe(0);
    });

    it("includes parser report with diagnostics", () => {
        const parser = new Parser();
        const recipe = "-- invalid meta\n\ninvalid recipe";

        const debug = parser.debug_info(recipe);

        // Report should contain HTML even for errors
        expect(typeof debug.report).toBe("string");
    });
});
