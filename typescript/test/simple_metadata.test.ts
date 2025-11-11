import { it, expect } from "vitest";
import { Parser } from "../index.js";

it("should handle simple metadata", () => {
    const parser = new Parser();
    const recipe = `---
author: jack black
---
Write your @recipe here!`;

    console.log("Calling debug_info with simple metadata...");
    const debug = parser.debug_info(recipe);
    console.log("Success!");
    expect(debug.metadata.length).toBeGreaterThan(0);
});
