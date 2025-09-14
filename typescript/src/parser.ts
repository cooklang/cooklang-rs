import {
  version,
  Parser as RustParser,
  type ScaledRecipeWithReport,
} from "../pkg/cooklang_wasm";

// for temporary backwards compatibility, let's export it with the old name
const Parser = RustParser;
export { version, Parser, type ScaledRecipeWithReport };

type Renderer = (parser: CooklangParser) => {
  render: (recipeString: string) => any;
};

export class CooklangParser {
  static version: string = version();
  public extensionList: string[];
  #rust_parser: RustParser;
  constructor() {
    this.extensionList = [] as string[];
    this.#rust_parser = new RustParser();
  }

  // TODO create issue to fill this in
  set extensions(extensions: string[]) {
    this.extensionList = extensions;
  }

  get extensions() {
    if (!this.extensionList) throw new Error("TODO");
    return this.extensionList;
  }

  use(...renderers: Record<string, Renderer>[]) {
    for (const rendererObject of renderers) {
      for (const key in rendererObject) {
        if ((this as any).render[key])
          throw new Error(`Renderer key ${key} already exists on parser`);
        const renderer = rendererObject[key];
        if (typeof renderer !== "function")
          throw new Error(`Renderer ${key} is not a function`);
        const instance = renderer(this);
        (this as any).render[key] = instance.render;
      }
    }
    return this;
  }

  render(renderFunction: Renderer, recipeString: string) {
    return renderFunction(this).render(recipeString);
  }

  parse(recipeString: string) {
    return this.#rust_parser.parse(recipeString);
  }
}

export const CooklangRendererBase = {
  prettyString(parser: CooklangParser) {
    return {
      // TODO fix return with actual pretty string
      render: (recipeString: string) => recipeString,
      // only for class CooklangRecipe, not required on other external renderers
      renderWithParsed: (parsed: ScaledRecipeWithReport) =>
        "eventually pretty string",
    };
  },
  html(parser: CooklangParser) {
    return {
      // TODO fix return with actual html string
      render: (recipeString: string) => recipeString,
      // only for class CooklangRecipe, not required on other external renderers
      renderWithParsed: (parsed: ScaledRecipeWithReport) => "eventually html",
    };
  },
  debug(parser: CooklangParser) {
    // TODO debug parse this then return
    return {
      render: (recipeString: string) => ({
        version: CooklangParser.version,
        ast: recipeString,
        events: recipeString,
      }),
    };
  },
  recipe(parser: CooklangParser) {
    return {
      render: (recipeString: string) => {
        const parsed = parser.parse(recipeString);
        return recipe(parsed);
      },
    };
  },
};

export const recipe = (rawParsed: ScaledRecipeWithReport) => {
  return {
    ...rawParsed.recipe,
    ingredients: new Map(
      rawParsed.recipe.ingredients.map((recipe) => [recipe.name, recipe])
    ),
  };
};

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
