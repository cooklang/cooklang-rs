import {
    CooklangRecipe,
    ingredient_should_be_listed,
    ingredient_display_name,
    grouped_quantity_is_empty,
    grouped_quantity_display,
    cookware_should_be_listed,
    cookware_display_name,
    quantity_display,
} from "@cooklang/cooklang";
import type {Section, Step, Ingredient, Cookware, Timer, Quantity} from "@cooklang/cooklang";

// Builds the "Preview" tab: a readable recipe from the parsed data.
// Every piece of recipe text goes through esc() — the input is user-typed.

export function esc(value: unknown): string {
    return String(value)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
}

function formatTime(time: CooklangRecipe["time"]): string | null {
    if (time == null) return null;
    const mins = (m: number) => (m >= 60 ? `${Math.floor(m / 60)} h${m % 60 ? ` ${m % 60} min` : ""}` : `${m} min`);
    if (typeof time === "number") return mins(time);
    const parts = [];
    if (time.prep_time) parts.push(`prep ${mins(time.prep_time)}`);
    if (time.cook_time) parts.push(`cook ${mins(time.cook_time)}`);
    return parts.length ? parts.join(" · ") : null;
}

function renderMeta(recipe: CooklangRecipe): string {
    const chips: string[] = [];
    const time = formatTime(recipe.time);
    if (time) chips.push(`<span class="chip chip-meta">⏱ ${esc(time)}</span>`);
    for (const tag of recipe.tags) chips.push(`<span class="chip chip-tag">#${esc(tag)}</span>`);
    const source = recipe.source;
    if (source && (source.url || source.name)) {
        const label = source.name || source.url;
        chips.push(
            source.url && /^https?:\/\//.test(source.url)
                ? `<a class="chip chip-meta" href="${esc(source.url)}" target="_blank" rel="noopener">↗ ${esc(label)}</a>`
                : `<span class="chip chip-meta">${esc(label)}</span>`
        );
    }
    return chips.length ? `<div class="meta-chips">${chips.join("")}</div>` : "";
}

function renderIngredients(recipe: CooklangRecipe): string {
    const rows = recipe.groupedIngredients
        .filter(([ing]) => ingredient_should_be_listed(ing))
        .map(([ing, qty]) => {
            const amount = grouped_quantity_is_empty(qty) ? "" : grouped_quantity_display(qty);
            const ref = ing.reference ? `<span class="ref" title="Recipe reference">↪</span> ` : "";
            const note = ing.note ? ` <span class="note">(${esc(ing.note)})</span>` : "";
            return `<li><span>${ref}${esc(ingredient_display_name(ing))}${note}</span><span class="amount">${esc(amount)}</span></li>`;
        });
    if (!rows.length) return "";
    return `<section class="block"><h3 class="label label-ingredient">Ingredients</h3><ul class="ingredients">${rows.join("")}</ul></section>`;
}

function renderCookware(recipe: CooklangRecipe): string {
    const chips = recipe.groupedCookware
        .filter(([cw]) => cookware_should_be_listed(cw))
        .map(([cw, qty]) => {
            const amount = grouped_quantity_is_empty(qty) ? "" : ` ×${grouped_quantity_display(qty)}`;
            return `<span class="chip chip-cookware">${esc(cookware_display_name(cw))}${esc(amount)}</span>`;
        });
    if (!chips.length) return "";
    return `<section class="block"><h3 class="label label-cookware">Cookware</h3><div class="chips">${chips.join("")}</div></section>`;
}

function inlineIngredient(ing: Ingredient): string {
    const qty = ing.quantity ? ` <span class="qty">${esc(quantity_display(ing.quantity))}</span>` : "";
    return `<span class="tok tok-ingredient">${esc(ingredient_display_name(ing))}${qty}</span>`;
}

function inlineCookware(cw: Cookware): string {
    return `<span class="tok tok-cookware">${esc(cookware_display_name(cw))}</span>`;
}

function inlineTimer(timer: Timer): string {
    const parts = [];
    if (timer.name) parts.push(esc(timer.name));
    if (timer.quantity) parts.push(esc(quantity_display(timer.quantity)));
    return `<span class="tok tok-timer">⏱ ${parts.join(" · ")}</span>`;
}

function inlineQuantity(q: Quantity): string {
    return `<span class="tok tok-quantity">${esc(quantity_display(q))}</span>`;
}

function renderStep(recipe: CooklangRecipe, step: Step): string {
    let body = "";
    for (const item of step.items) {
        switch (item.type) {
            case "text": body += esc(item.value); break;
            case "ingredient": body += inlineIngredient(recipe.ingredients[item.index]); break;
            case "cookware": body += inlineCookware(recipe.cookware[item.index]); break;
            case "timer": body += inlineTimer(recipe.timers[item.index]); break;
            case "inlineQuantity": body += inlineQuantity(recipe.inlineQuantities[item.index]); break;
        }
    }
    return `<li class="step"><span class="step-num">${step.number}</span><p>${body}</p></li>`;
}

function renderSections(recipe: CooklangRecipe): string {
    const multiple = recipe.sections.length > 1;
    return recipe.sections.map((section: Section, i: number) => {
        const heading = section.name
            ? `<h3 class="section-title">${esc(section.name)}</h3>`
            : multiple ? `<h3 class="section-title">Section ${i + 1}</h3>` : "";
        const items = section.content.map((content) =>
            content.type === "step"
                ? renderStep(recipe, content.value)
                : `<li class="text-block"><p>${esc(content.value)}</p></li>`
        );
        return `${heading}<ol class="steps">${items.join("")}</ol>`;
    }).join("");
}

export interface PreviewOptions {
    servings: number | null; // target servings the recipe was scaled to, if any
}

export function renderPreview(recipe: CooklangRecipe, opts: PreviewOptions): string {
    const base = typeof recipe.servings === "number" ? recipe.servings : null;
    const shown = opts.servings ?? base;
    const stepper = `
      <div class="servings" title="Scale the recipe">
        <span class="servings-label">Servings</span>
        <button type="button" class="step-btn" data-servings-delta="-1" aria-label="Fewer servings">−</button>
        <span class="servings-value">${shown ?? "–"}</span>
        <button type="button" class="step-btn" data-servings-delta="1" aria-label="More servings">+</button>
      </div>`;
    const title = recipe.title ? esc(recipe.title) : "Untitled recipe";
    const description = recipe.description ? `<p class="description">${esc(recipe.description)}</p>` : "";
    const steps = renderSections(recipe);
    const empty = !recipe.sections.some((s) => s.content.length);
    return `
      <article class="recipe">
        <header class="recipe-head">
          <div>
            <h2 class="recipe-title">${title}</h2>
            ${description}
            ${renderMeta(recipe)}
          </div>
          ${stepper}
        </header>
        <div class="recipe-lists">
          ${renderIngredients(recipe)}
          ${renderCookware(recipe)}
        </div>
        ${empty ? `<p class="empty">Start typing a recipe on the left — try <span class="tok tok-ingredient">@eggs{3}</span>.</p>` : `<section class="block"><h3 class="label">Steps</h3>${steps}</section>`}
      </article>`;
}
