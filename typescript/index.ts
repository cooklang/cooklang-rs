import {
    version,
    Parser,
    InterpretedMetadata,
    NameAndUrl,
    RecipeTime,
    Servings
} from "./pkg/cooklang_wasm";

export {version, Parser};
export type {ScaledRecipeWithReport} from "./pkg/cooklang_wasm";

export class CooklangRecipe {
    title?: string;
    description?: string;
    tags?: string[];
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

    constructor(raw?: string) {
        if (raw) {
            const parser = new Parser();
            const recipe = parser.parse(raw);
            this.setMetadata(recipe.metadata);
        }
    }

    private setMetadata(metadata: InterpretedMetadata) {
        this.title = metadata.title;
        this.description = metadata.description;
        this.tags = metadata.tags;
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
}