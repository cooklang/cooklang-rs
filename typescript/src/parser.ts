import {
  version,
  Parser as RustParser,
  type ScaledRecipeWithReport,
} from "../pkg/cooklang_wasm";
import type { Renderer } from "./renderers";

// for temporary backwards compatibility, let's export it with the old name
const Parser = RustParser;
export { version, Parser, type ScaledRecipeWithReport };

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
