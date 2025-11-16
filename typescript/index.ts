import {
    version,
    Parser,
    NameAndUrl,
    RecipeTime,
    Servings, Section, Ingredient, Cookware, Timer, Quantity, ScaledRecipeWithReport, GroupedQuantity,
    ingredient_should_be_listed, ingredient_display_name, grouped_quantity_is_empty, grouped_quantity_display,
    cookware_should_be_listed, cookware_display_name, Content, Step, quantity_display, GroupedIndexAndQuantity,
    Value, Item
} from "./pkg/cooklang_wasm.js";

export {
    version,
    Parser,
    ingredient_should_be_listed,
    ingredient_display_name,
    grouped_quantity_is_empty,
    grouped_quantity_display,
    cookware_should_be_listed,
    cookware_display_name,
    quantity_display
};
export type {ScaledRecipeWithReport, Value, Quantity, Ingredient, Cookware, Timer, Section, Content, Step, Item} from "./pkg/cooklang_wasm.js";

// ============================================================================
// Numeric Value Extraction Helpers
// ============================================================================

/**
 * Extract a numeric value from a WASM Value type.
 *
 * For ranges, returns the start value.
 * For text values, returns null.
 *
 * @param value - The Value to extract from
 * @returns The numeric value or null if not a number/range
 *
 * @example
 * ```typescript
 * const value = ingredient.quantity?.value;
 * const numeric = getNumericValue(value); // 2.5
 * ```
 */
export function getNumericValue(value: Value | null | undefined): number | null {
    if (!value) {
        return null;
    }

    if (value.type === 'number') {
        // WASM returns nested structure: { type: "number", value: { type: "regular", value: 3 } }
        // The type definitions are incomplete, so we need to cast
        const numValue = value.value as any;
        return Number(numValue.value);
    } else if (value.type === 'range') {
        // Range structure: { type: "range", value: { start: { type: "regular", value: X }, end: { ... } } }
        // Return start of range
        const rangeValue = value.value as any;
        return Number(rangeValue.start.value);
    }
    return null;
}

/**
 * Extract the numeric value from a Quantity.
 *
 * Convenience wrapper around getNumericValue for Quantity objects.
 *
 * @param quantity - The Quantity to extract from
 * @returns The numeric value or null
 *
 * @example
 * ```typescript
 * const qty = getQuantityValue(ingredient.quantity); // 2.5
 * ```
 */
export function getQuantityValue(quantity: Quantity | null | undefined): number | null {
    return quantity ? getNumericValue(quantity.value) : null;
}

/**
 * Extract the unit string from a Quantity.
 *
 * @param quantity - The Quantity to extract from
 * @returns The unit string or null if no unit
 *
 * @example
 * ```typescript
 * const unit = getQuantityUnit(ingredient.quantity); // "cups"
 * ```
 */
export function getQuantityUnit(quantity: Quantity | null | undefined): string | null {
    return quantity?.unit ?? null;
}

// ============================================================================
// Flat List Helpers
// ============================================================================

/**
 * Simple ingredient with display-ready values.
 * For more control, use recipe.groupedIngredients with the display functions.
 */
export interface FlatIngredient {
    /** Display name of the ingredient */
    name: string;
    /** Numeric quantity (start of range if range), or null if none */
    quantity: number | null;
    /** Unit string, or null if none */
    unit: string | null;
    /** Formatted display text for the quantity (e.g., "1-2 cups", "3/4 tsp") */
    displayText: string | null;
    /** Optional note/modifier (e.g., "finely chopped") */
    note: string | null;
}

/**
 * Simple cookware with display-ready values.
 */
export interface FlatCookware {
    /** Display name of the cookware */
    name: string;
    /** Numeric quantity, or null if none */
    quantity: number | null;
    /** Formatted display text for the quantity */
    displayText: string | null;
    /** Optional note/modifier */
    note: string | null;
}

/**
 * Simple timer with display-ready values.
 */
export interface FlatTimer {
    /** Optional timer name */
    name: string | null;
    /** Numeric quantity (in seconds after unit conversion), or null if none */
    quantity: number | null;
    /** Unit string (e.g., "minutes", "hours"), or null if none */
    unit: string | null;
    /** Formatted display text for the quantity */
    displayText: string | null;
}

/**
 * Get a flat list of all ingredients with simple, display-ready values.
 *
 * This is a convenience function for simple use cases. For more control over
 * grouping and display, use recipe.groupedIngredients with the display functions.
 *
 * @param recipe - The parsed recipe
 * @returns Array of flat ingredient objects
 *
 * @example
 * ```typescript
 * const ingredients = getFlatIngredients(recipe);
 * ingredients.forEach(ing => {
 *   console.log(`${ing.displayText || ''} ${ing.name}`);
 * });
 * ```
 */
export function getFlatIngredients(recipe: CooklangRecipe): FlatIngredient[] {
    return recipe.ingredients.map(ing => ({
        name: ingredient_display_name(ing),
        quantity: getQuantityValue(ing.quantity),
        unit: getQuantityUnit(ing.quantity),
        displayText: ing.quantity ? quantity_display(ing.quantity) : null,
        note: ing.note
    }));
}

/**
 * Get a flat list of all cookware with simple, display-ready values.
 *
 * @param recipe - The parsed recipe
 * @returns Array of flat cookware objects
 *
 * @example
 * ```typescript
 * const cookware = getFlatCookware(recipe);
 * cookware.forEach(cw => {
 *   console.log(`${cw.displayText || ''} ${cw.name}`);
 * });
 * ```
 */
export function getFlatCookware(recipe: CooklangRecipe): FlatCookware[] {
    return recipe.cookware.map(cw => ({
        name: cookware_display_name(cw),
        quantity: getQuantityValue(cw.quantity),
        displayText: cw.quantity ? quantity_display(cw.quantity) : null,
        note: cw.note
    }));
}

/**
 * Get a flat list of all timers from the recipe.
 *
 * @param recipe - The parsed recipe
 * @returns Array of flat timer objects
 *
 * @example
 * ```typescript
 * const timers = getFlatTimers(recipe);
 * timers.forEach(timer => {
 *   console.log(`${timer.name}: ${timer.displayText}`);
 * });
 * ```
 */
export function getFlatTimers(recipe: CooklangRecipe): FlatTimer[] {
    return recipe.timers.map(tm => ({
        name: tm.name,
        quantity: getQuantityValue(tm.quantity),
        unit: getQuantityUnit(tm.quantity),
        displayText: tm.quantity ? quantity_display(tm.quantity) : null
    }));
}

// ============================================================================
// Recipe and Parser Classes
// ============================================================================

export class CooklangRecipe {
    // Metadata
    title?: string;
    description?: string;
    tags: Set<string>;
    author?: NameAndUrl;
    source?: NameAndUrl;
    course: any;
    time?: RecipeTime;
    servings?: Servings;
    difficulty: any;
    cuisine: any;
    diet: any;
    images: any;
    locale?: [string, string | null];
    custom_metadata: Map<any, any>;

    // Data
    rawMetadata: Map<any, any>;
    sections: Section[];
    ingredients: Ingredient[];
    cookware: Cookware[];
    timers: Timer[];
    inlineQuantities: Quantity[];

    // Preprocessed
    groupedIngredients: [Ingredient, GroupedQuantity][];
    groupedCookware: [Cookware, GroupedQuantity][];

    constructor(raw: ScaledRecipeWithReport,
                groupedIngredients: GroupedIndexAndQuantity[],
                groupedCookware: GroupedIndexAndQuantity[]) {
        this.title = raw.metadata.title;
        this.description = raw.metadata.description;
        this.tags = new Set(raw.metadata.tags);
        this.author = raw.metadata.author;
        this.source = raw.metadata.source;
        this.course = raw.metadata.course;
        this.time = raw.metadata.time;
        this.servings = raw.metadata.servings;
        this.difficulty = raw.metadata.difficulty;
        this.cuisine = raw.metadata.cuisine;
        this.diet = raw.metadata.diet;
        this.images = raw.metadata.images;
        this.locale = raw.metadata.locale;

        this.custom_metadata = new Map();
        for (let key in raw.metadata.custom)
            this.custom_metadata.set(key, raw.metadata.custom[key]);

        this.rawMetadata = new Map();
        for (let key in raw.recipe.raw_metadata.map)
            this.rawMetadata.set(key, raw.recipe.raw_metadata.map[key]);

        this.sections = raw.recipe.sections;
        this.ingredients = raw.recipe.ingredients;
        this.cookware = raw.recipe.cookware;
        this.timers = raw.recipe.timers;
        this.inlineQuantities = raw.recipe.inline_quantities;

        this.groupedIngredients = groupedIngredients.map((iaq) => [this.ingredients[iaq.index], iaq.quantity]);
        this.groupedCookware = groupedCookware.map((iaq) => [this.cookware[iaq.index], iaq.quantity]);
    }
}

export class CooklangParser {
    private parser: Parser;

    constructor() {
        this.parser = new Parser();
    }

    parse(input: string, scale?: number | null): [CooklangRecipe, string] {
        let raw = this.parser.parse(input, scale);
        return [new CooklangRecipe(raw, this.parser.group_ingredients(raw), this.parser.group_cookware(raw)), raw.report];
    }

    set units(value: boolean) {
        this.parser.load_units = value;
    }

    get units(): boolean {
        return this.parser.load_units
    }

    set extensions(value: number) {
        this.parser.extensions = value;
    }

    get extensions(): number {
        return this.parser.extensions
    }
}

export class HTMLRenderer {
    protected result!: string;
    protected recipe!: CooklangRecipe;

    render(recipe: CooklangRecipe): string {
        this.result = "";
        this.recipe = recipe;

        const groupedIngredients = recipe.groupedIngredients;
        const groupedCookware = recipe.groupedCookware;

        this.renderMetadata(recipe.rawMetadata);
        this.renderGroupedIngredients(groupedIngredients);
        this.renderGroupedCookwares(groupedCookware);

        if (groupedCookware.length > 0 || groupedIngredients.length > 0) {
            this.result += `<hr>`;
        }

        this.renderSections(recipe.sections);

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

    protected renderGroupedIngredient(name: string, quantity: string | null, note: string | null) {
        this.result += "<li>";
        this.result += `<b>${name}</b>`;

        if (quantity)
            this.result += `: ${quantity}`;

        if (note)
            this.result += ` (${note})`;

        this.result += "</li>";
    }

    protected renderGroupedCookwares(cookwares: [Cookware, GroupedQuantity][]) {
        if (cookwares.length > 0) {
            this.result += "<h2>Cookware:</h2>";
            this.result += "<ul>";

            for (const [cookware, quantity] of cookwares) {
                this.renderGroupedCookwareHelper(cookware, quantity);
            }

            this.result += "</ul>";
        }
    }

    protected renderGroupedCookwareHelper(cookware: Cookware, quantity: GroupedQuantity) {
        if (cookware_should_be_listed(cookware)) {
            const cookwareName = cookware_display_name(cookware);

            const quantityString = !grouped_quantity_is_empty(quantity) ?
                grouped_quantity_display(quantity)
                : null;

            this.renderGroupedCookware(cookwareName, quantityString, cookware.note);
        }
    }

    protected renderGroupedCookware(name: string, quantity: string | null, note: string | null) {
        this.result += "<li>";
        this.result += `<b>${name}</b>`;

        if (quantity)
            this.result += `: ${quantity}`;

        if (note)
            this.result += ` (${note})`;

        this.result += "</li>";
    }

    protected renderSections(sections: Section[]) {
        for (let s_index = 0; s_index < sections.length; s_index++) {
            const section = sections[s_index];
            const s_num = s_index + 1;

            if (section.name) {
                this.result += `<h3>(${s_num}) ${section.name}</h3>`;
            } else if (sections.length > 1) {
                this.result += `<h3>Section ${s_num}</h3>`;
            }

            for (const content of section.content) {
                this.renderContent(section, content);
            }
        }
    }

    protected renderContent(current_section: Section, content: Content) {
        switch (content.type) {
            case "text":
                this.result += `<p>${content.value}</p>>`;
                break;
            case "step":
                this.renderStep(current_section, content.value);
                break;
        }
    }

    protected renderStep(current_section: Section, step: Step) {
        this.result += `<p><b>${step.number}. </b>`;
        for (const item of step.items) {
            switch (item.type) {
                case "text":
                    this.result += item.value;
                    break
                case "ingredient":
                    this.renderInlineIngredient(current_section, this.recipe.ingredients[item.index]);
                    break;
                case "timer":
                    this.renderInlineTimer(this.recipe.timers[item.index]);
                    break;
                case "inlineQuantity":
                    this.renderInlineQuantity(this.recipe.inlineQuantities[item.index]);
                    break;
                case "cookware":
                    this.renderInlineCookware(this.recipe.cookware[item.index]);
                    break;
            }
        }
        this.result += "</p>";
    }

    protected renderInlineIngredient(current_section: Section, ingredient: Ingredient) {
        this.result += "<span class='ingredient'>";

        this.result += ingredient_display_name(ingredient);

        if (ingredient.quantity) {
            this.result += `<i>(${quantity_display(ingredient.quantity)})</i>`;
        }
        if (ingredient.relation.relation.type === "reference") {
            const index = ingredient.relation.relation.references_to;
            switch (ingredient.relation.reference_target) {
                case "ingredient":
                    break;
                case "step":
                    if (current_section.content[index].type === "step") {
                        `<i>(from step ${current_section.content[index].value.number})</i>`
                    }
                    break;
                case "section":
                    const sect = index + 1;
                    `<i>(from section ${sect})</i>`
                    break;
            }
        }


        this.result += "</span>";
    }

    protected renderInlineTimer(timer: Timer) {
        this.result += "<span class='timer'>";

        if (timer.name) {
            this.result += `(${timer.name})`;
        }
        if (timer.quantity) {
            this.result += `<i>${quantity_display(timer.quantity)}</i>`;
        }

        this.result += "</span>";
    }

    private renderInlineQuantity(quantity: Quantity) {
        this.result += `<i class="temp">(${quantity_display(quantity)})</i>`;
    }

    private renderInlineCookware(cookware: Cookware) {
        this.result += "<span class='cookware'>";

        this.result += cookware_display_name(cookware);

        if (cookware.quantity) {
            this.result += `<i>(${quantity_display(cookware.quantity)})</i>`;
        }

        this.result += "</span>";
    }
}
