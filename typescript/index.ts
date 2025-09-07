import {
    version,
    Parser,
    InterpretedMetadata,
    NameAndUrl,
    RecipeTime,
    Servings, Section, Ingredient, Cookware, Timer, Quantity, Recipe, ScaledRecipeWithReport, GroupedQuantity,
    ingredient_should_be_listed, ingredient_display_name, grouped_quantity_is_empty, grouped_quantity_display
} from "./pkg/cooklang_wasm.js";

export {version, Parser};
export type {ScaledRecipeWithReport} from "./pkg/cooklang_wasm.js";

export class CooklangRecipe {
    // Inner
    private parser: Parser;
    private rawString_: string;
    private rawRecipe: ScaledRecipeWithReport;

    // TODO: make all of those private with public getter
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
            this.rawString = raw;
        } else {
            this.rawString_ = raw;
        }
    }

    set rawString(raw: string) {
        this.rawString_ = raw;
        this.rawRecipe = this.parser.parse(raw);
        this.setMetadata(this.rawRecipe.metadata);
        this.setData(this.rawRecipe.recipe);
    }

    get rawString(): string {
        return this.rawString_;
    }

    get groupedIngredients(): [Ingredient, GroupedQuantity][] {
        const groups = this.parser.group_ingredients(this.rawRecipe);
        return groups.map((iaq) => [this.ingredients[iaq.index], iaq.quantity]);
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

export class HTMLRenderer {
    protected result = "";

    render(recipe: CooklangRecipe): string {
        this.result = "";

        this.renderMetadata(recipe.rawMetadata);
        this.renderGroupedIngredients(recipe.groupedIngredients);

        return this.result;
    }

    protected renderMetadata(metadata: Map<any, any>) {
        if (metadata.size > 0) {
            this.result += "<ul>";

            for (const [key, value] of metadata)
                this.renderMetadatum(key, value);

            this.result += "</ul>";
            this.result += "<hr>";
        }
    }

    protected renderMetadatum(key: any, value: any) {
        this.result += "<li class='metadata'>";
        this.result += `<span class='key'>${key}</span>: <span class='value'>${value}</span>`;
        this.result += "</li>";
    }

    protected renderGroupedIngredients(ingredients: [Ingredient, GroupedQuantity][]) {
        if (ingredients.length > 0) {
            this.result += "<h2>Ingredients:</h2>";
            this.result += "<ul>";

            for (const [ingredient, quantity] of ingredients) {
                this.renderGroupedIngredientHelper(ingredient, quantity);
            }

            this.result += "</ul>";
        }
    }

    protected renderGroupedIngredientHelper(ingredient: Ingredient, quantity: GroupedQuantity) {
        if (ingredient_should_be_listed(ingredient)) {
            const ingredientName = ingredient_display_name(ingredient);

            const quantityString = !grouped_quantity_is_empty(quantity) ?
                grouped_quantity_display(quantity)
                : null;

            this.renderGroupedIngredient(ingredientName, quantityString, ingredient.note);
        }
    }

    protected renderGroupedIngredient(name: string, quantity: string, note: string) {
        this.result += "<li class='ingredients'>";
        this.result += `<b>${name}</b>`;

        if (quantity)
            this.result += `: ${quantity}`;

        if (note)
            this.result += ` (${note})`;

        this.result += "</li>";
    }
}