# Debug Information in cooklang-ts

The TypeScript library provides a consolidated debug API to access comprehensive debug and version information about the parser.

## Comprehensive Debug Information

Get all debug information in a single call:

```typescript
import { Parser, type DebugInfo } from "@cooklang/cooklang-ts";

const parser = new Parser();
const recipe = "-- title: My Recipe\n\n@ingredient{1}\nSome @step.";

const debug: DebugInfo = parser.debug_info(recipe);

console.log(JSON.stringify(debug, null, 2));
// {
//   "version": "0.8.0",
//   "extensions": 4095,
//   "load_units": true,
//   "ast": "{ ... AST as JSON ... }",
//   "events": "[ ... parse events ... ]",
//   "full_recipe": "{ ... full recipe as JSON ... }",
//   "metadata": "{ ... parsed metadata ... }",
//   "report": "<html error report>"
// }
```

The `DebugInfo` object contains:
- **version**: Library version
- **extensions**: Currently enabled extensions (bitmask)
- **load_units**: Whether unit loading is enabled
- **ast**: AST representation in JSON format
- **events**: Parse events in debug format
- **full_recipe**: Complete parsed recipe in JSON format
- **metadata**: Parsed metadata fields
- **report**: HTML-formatted parsing errors and warnings

## Version Information

Get just the current version:

```typescript
import { version } from "@cooklang/cooklang-ts";

const currentVersion = version();
console.log(currentVersion); // e.g., "0.8.0"
```

## Individual Parser Debug Methods

For more granular control, you can call individual debug methods:

The `Parser` class (imported from the WASM bindings) provides several debug methods to inspect the parsing process:

### Getting the AST (Abstract Syntax Tree)

```typescript
import { Parser } from "@cooklang/cooklang-ts";

const parser = new Parser();
const recipe = "-- title: My Recipe\n\n@ingredient{1}\nSome @step.";

// Get AST in pretty-printed debug format
const { value, error } = parser.parse_ast(recipe, false);
console.log(value); // Debug representation of the AST

// Get AST as JSON
const { value: jsonAst, error: jsonError } = parser.parse_ast(recipe, true);
console.log(JSON.parse(jsonAst)); // Parsed AST as JSON object
```

### Getting Parse Events

```typescript
// Get low-level parse events
const events = parser.parse_events(recipe);
console.log(events); // Raw parser events in debug format
```

### Getting Full Parsed Recipe

```typescript
// Get the full parsed recipe with all details
const { value, error } = parser.parse_full(recipe, false);
console.log(value); // Full recipe in debug format

// Get as JSON
const { value: jsonRecipe, error: jsonError } = parser.parse_full(recipe, true);
const fullData = JSON.parse(jsonRecipe); // Full recipe as JSON
```

### Getting Standard Metadata

```typescript
// Get parsed standard metadata fields
const { value, error } = parser.std_metadata(recipe);
console.log(value); // Parsed metadata in debug format
```

## Parser Configuration

You can configure the parser for different debug scenarios:

```typescript
const parser = new Parser();

// Enable/disable unit conversion
parser.load_units = false;

// Configure extensions
parser.extensions = 0; // Disable all extensions
parser.extensions = (1 << 1) | (1 << 3); // Enable specific extensions
```

## Parse Results

When using the main `parse()` method, you also get diagnostic information:

```typescript
import { CooklangParser } from "@cooklang/cooklang-ts";

const parser = new CooklangParser();
const [recipe, report] = parser.parse(input);

// The report contains parsing diagnostics and warnings
console.log(report); // HTML-formatted error and warning messages
```

## Example: Creating an Issue Report

```typescript
import { Parser } from "@cooklang/cooklang-ts";

function createIssueReport(recipeInput: string) {
  const parser = new Parser();
  
  // Get comprehensive debug info in a single call
  const debug = parser.debug_info(recipeInput);
  
  // Format for issue reporting
  const report = `
## Debug Information

**Version:** ${debug.version}
**Extensions:** ${debug.extensions}
**Load Units:** ${debug.load_units}

### AST
\`\`\`json
${debug.ast}
\`\`\`

### Parse Report
${debug.report}

### Full Recipe
\`\`\`json
${debug.full_recipe}
\`\`\`
`;
  
  return report;
}
```

## See Also

- [Playground](../playground) - Interactive playground with debug information display
- [main.ts](./src/lib.rs) - WASM bindings source with full API documentation
