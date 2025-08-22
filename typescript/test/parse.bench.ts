import { describe, bench } from "vitest";
import { CooklangRecipe } from "..";

describe("Recipe Parse", async () => {
    bench("optimized", async () => {
        const recipeRaw = "this could be a @recipe";
        CooklangRecipe.fromString(recipeRaw);
    });
    bench("naive", async () => {
        const recipeRaw = "this could be a @recipe";
        CooklangRecipe.fromStringNaive(recipeRaw);
    });
})