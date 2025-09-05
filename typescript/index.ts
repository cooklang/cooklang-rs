import {
    version,
    Parser,
    InterpretedMetadata,
    NameAndUrl,
    RecipeTime,
    Servings, Section, Ingredient, Cookware, Timer, Quantity, Recipe
} from "./pkg/cooklang_wasm.js";

export {version, Parser};
export type {ScaledRecipeWithReport} from "./pkg/cooklang_wasm.js";

export class CooklangRecipe {
    // Inner
    private parser: Parser;
    private rawString: string;

    // Metadata
    title?: string;
    description?: string;
    tags?: Set<string>;
    author?: NameAndUrl;
    source?: NameAndUrl;
    course?: any;
    time?: RecipeTime;
    servings?: Servings;
    difficulty?: any;
    cuisine?: any;
    diet?: any;
    images?: any;
    locale?: [string, string?];
    custom_metadata: Map<any, any>;

    // Data
    rawMetadata?: Map<any, any>;
    sections?: Section[];
    ingredients?: Ingredient[];
    cookware?: Cookware[];
    timers?: Timer[];
    inlineQuantities?: Quantity[];

    constructor(raw?: string) {
        this.parser = new Parser();

        if (raw) {
            this.raw = raw;
        } else {
            this.rawString = raw;
        }
    }

    set raw(raw: string) {
        this.rawString = raw;
        const recipe = this.parser.parse(raw);
        this.setMetadata(recipe.metadata);
        this.setData(recipe.recipe);
    }

    get raw(): string {
        return this.rawString;
    }

    private setMetadata(metadata: InterpretedMetadata) {
        this.title = metadata.title;
        this.description = metadata.description;
        this.tags = new Set(metadata.tags);
        this.author = metadata.author;
        this.source = metadata.source;
        this.course = metadata.course;
        this.time = metadata.time;
        this.servings = metadata.servings;
        this.difficulty = metadata.difficulty;
        this.cuisine = metadata.cuisine;
        this.diet = metadata.diet;
        this.images = metadata.images;
        this.locale = metadata.locale;

        this.custom_metadata = new Map();
        for (let key in metadata.custom)
            this.custom_metadata.set(key, metadata.custom[key]);
    }

    private setData(data: Recipe) {
        this.rawMetadata = new Map();
        for (let key in data.raw_metadata.map)
            this.rawMetadata.set(key, data.raw_metadata.map[key]);

        this.sections = data.sections;
        this.ingredients = data.ingredients;
        this.cookware = data.cookware;
        this.timers = data.timers;
        this.inlineQuantities = data.inline_quantities;
    }
}