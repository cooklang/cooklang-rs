import { version, Parser, Recipe } from "./pkg/cooklang_wasm";

export { version, Parser };
export type { ScaledRecipeWithReport } from "./pkg/cooklang_wasm";

type RecipeRenderer<T> = (recipe: Recipe) => T;

export class CooklangRecipe {

    private recipe: Recipe;

    static parser = new Parser();

    static fromString(raw: string) {
        const recipe = CooklangRecipe.parser.parse(raw);
        return new CooklangRecipe(recipe.recipe);
    }

    static fromStringNaive(raw: string) {
        const parser = new Parser();
        const recipe = parser.parse(raw);
        return new CooklangRecipe(recipe.recipe);
    }

    static async fromFile(fp: File) {
        return CooklangRecipe.fromString(await fp.text());
    }

    constructor(recipe: Recipe) {
        this.recipe = recipe;
    }

    public render<T>(renderer: RecipeRenderer<T>) {
        return renderer(this.recipe);
    }

    get ingredients() {
        return this.recipe.ingredients;
    }

    get metadata() {
        return {}  // Todo: implement
    }

}
