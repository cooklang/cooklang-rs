# cooklang (TypeScript wrapper)

Lightweight TypeScript wrapper through WASM for the Rust-based Cooklang parser.

This folder provides a thin JS/TS convenience layer around the WASM parser based on `cooklang-rs`. The primary exported class in this module is `CooklangParser` which can be used either as an instance (hold a recipe and operate on it) or as a functional utility (pass a recipe string to each method).

## Examples

### Instance Usage

This pattern holds a recipe on the parser instance in which all properties and methods then act upon.

```ts
import { CooklangParser } from "@cooklang/parser";

const fancyRecipe = "Write your @recipe here!";

// create a parser instance with a raw recipe string
const recipe = new CooklangParser(fancyRecipe);

// read basic fields populated by the wrapper
console.log(recipe.metadata); // TODO sample response
console.log(recipe.ingredients); // TODO sample response
console.log(recipe.sections); // TODO sample response

// render methods return the original string in the minimal implementation
console.log(recipe.renderPrettyString()); // TODO sample response
console.log(recipe.renderHTML()); // TODO sample response
```

### Functional Usage

This pattern passes a string directly and doesn't require keeping an instance around.

```ts
import { CooklangParser } from "@cooklang/parser";

const parser = new CooklangParser();
const recipeString = "Write your @recipe here!";

// functional helpers accept a recipe string and return rendered output
console.log(parser.renderPrettyString(recipeString)); // TODO sample response
console.log(parser.renderHTML(recipeString)); // TODO sample response

// `parse` returns a recipe class
const parsed = parser.parse(recipeString);
console.log(parsed.metadata); // TODO sample response
console.log(parsed.ingredients); // TODO sample response
console.log(parsed.sections); // TODO sample response
```
