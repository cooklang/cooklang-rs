import {
  version,
  Parser as RustParser,
  type ScaledRecipeWithReport,
} from "../pkg/cooklang_wasm";

// for temporary backwards compatibility, let's export it with the old name
const Parser = RustParser;
export { version, Parser, type ScaledRecipeWithReport };

class CooklangRecipe {
  metadata = {};
  ingredients = new Map();
  // TODO should we use something other than array here?
  sections = [];
  cookware = new Map();
  timers = [];
  constructor(rawParsed?: ScaledRecipeWithReport) {
    if (rawParsed) {
      this.setRecipe(rawParsed);
    }
  }

  setRecipe(rawParsed: ScaledRecipeWithReport) {
    this.metadata = {};
    // this.ingredients = [];
    // this.steps = [];
    // this.cookware = [];
    // this.timers = [];
  }
}

class CooklangParser extends CooklangRecipe {
  public version: string;
  public extensionList: string[];
  constructor(public rawContent?: string) {
    super();
    this.version = version();
    this.extensionList = [] as string[];
  }

  set raw(raw: string) {
    this.rawContent = raw;
  }

  get raw() {
    if (!this.rawContent)
      throw new Error("recipe not set, call .raw(content) to set it first");
    return this.rawContent;
  }

  #handleFunctionalOrInstance(instanceInput: string | undefined) {
    if (this.rawContent) {
      if (instanceInput)
        throw new Error("recipe already set, create a new instance");
      return this.rawContent;
    }
    if (!instanceInput) {
      throw new Error("pass a recipe as a string or generate a new instance");
    }
    return instanceInput;
  }

  // TODO create issue to fill this in
  set extensions(extensions: string[]) {
    this.extensionList = extensions;
  }

  get extensions() {
    if (!this.extensionList) throw new Error("TODO");
    return this.extensionList;
  }

  // TODO create issue for this
  renderPrettyString(recipeString?: string) {
    const input = this.#handleFunctionalOrInstance(recipeString);
    // TODO renderPrettyString this then return
    return input;
  }

  renderHTML(recipeString?: string) {
    const input = this.#handleFunctionalOrInstance(recipeString);
    // TODO renderHTML this then return
    return input;
  }

  parseRaw(recipeString?: string) {
    const input = this.#handleFunctionalOrInstance(recipeString);
    // TODO parseRaw this then return
    return input;
  }

  // TODO return fully typed JS Object
  parse(recipeString?: string) {
    const input = this.#handleFunctionalOrInstance(recipeString);
    // TODO actually parse
    const parsed = {
      recipe: { ingredients: [input] },
    } as unknown as ScaledRecipeWithReport;
    if (this.rawContent) {
      this.setRecipe(parsed);
    }
    if (!this.rawContent && recipeString) {
      const direct = new CooklangRecipe(parsed);
      return direct;
    } else {
      throw new Error("should never reach this");
    }
  }

  debug(recipeString?: string): {
    version: string;
    ast: string;
    events: string;
  } {
    const input = this.#handleFunctionalOrInstance(recipeString);
    // TODO debug parse this then return
    return { version: this.version, ast: input, events: input };
  }
}

export { CooklangParser };
