import {version, Parser, ScaledRecipeWithReport, Ingredient, FallibleResult} from "./pkg/cooklang_wasm";


export class CooklangRecipe {
    metadata = {};
    ingredients = new Map();
    // TODO should we use something other than array here?
    sections = [];
    cookware = new Map();
    timers = [];

    constructor(input?: string, extensions?: string[]) {
        if (input) {
            // TODO: extensions
            let parser = new Parser;
            let recipe = parser.parse(input);
            this.#setRecipe(recipe);
        }
    }

    set input(input: string) {
        let parser = new Parser;
        let recipe = parser.parse(input);
        this.#setRecipe(recipe);
    }

    #setRecipe(rawParsed: ScaledRecipeWithReport) {
        this.metadata = {};
        // this.ingredients = [];
        // this.steps = [];
        // this.cookware = [];
        // this.timers = [];
    }

    render<T>(renderer: Renderer<T>): T {
        throw new Error("implement por fawor")
    }
}

export class Renderer<Output> {
    renderIngredient(ingredient: Ingredient): Output {
        throw new Error("implement por fawor")
    }

    renderIngredientList(list: Output[]): Output {
        throw new Error("implement por fawor")
    }
}

export function debugCooklangRecipe(input: string, json: boolean, extensions?: string[]): FallibleResult {
    // TODO: extensions
    let parser = new Parser;
    return parser.parse_ast(input, json);
}
