import { CooklangParser, ScaledRecipeWithReport } from "./parser.js";
import { CooklangRendererBase, recipe } from "./renderers.js";

export class CooklangRecipe extends CooklangParser {
  #parsed: ScaledRecipeWithReport | null = null;
  metadata = {};
  ingredients = new Map();
  // TODO should we use something other than array here?
  sections = [];
  cookware = new Map();
  timers = [];
  constructor(raw: string) {
    super();
    // @ts-expect-error
    this.use = void 0; // disable use method on instance
    this.render = {} as Record<string, () => any>;

    this.render.prettyString = () =>
      CooklangRendererBase["prettyString"](this).renderWithParsed(
        this.#parsed!
      );
    this.render.html = () =>
      CooklangRendererBase["html"](this).renderWithParsed(this.#parsed!);

    this.raw = raw;
  }

  #setRecipe(rawParsed: ScaledRecipeWithReport) {
    const constructed = recipe(rawParsed);
    this.metadata = constructed.metadata;
    this.ingredients = constructed.ingredients;
    this.sections = constructed.sections;
    this.cookware = constructed.cookware;
    this.timers = constructed.timers;
  }

  set raw(raw: string) {
    const parsed = this.parse(raw);
    this.#parsed = parsed;
    this.#setRecipe(parsed);
  }

  get raw() {
    return this.raw;
  }
}
