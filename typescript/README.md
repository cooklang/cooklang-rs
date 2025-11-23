# @cooklang/cooklang

Official [Cooklang](https://cooklang.org) parser for JavaScript and TypeScript.

This is a high-performance WASM implementation powered by the Rust [cooklang-rs](https://github.com/cooklang/cooklang-rs) parser. It provides fast, reliable recipe parsing with full support for the Cooklang specification.

## Installation

```bash
npm install @cooklang/cooklang
```

## Quick Start

```typescript
import { Parser } from '@cooklang/cooklang';

const parser = new Parser();
const recipe = parser.parse(`
>> servings: 4

Add @salt and @pepper to taste.
Cook for ~{10%minutes}.
`);

console.log(recipe.sections[0].content);
```

## Why WASM?

This package uses WebAssembly for several key benefits:

- **Performance**: Native-speed parsing, significantly faster than pure JavaScript
- **Reliability**: Shared implementation with the official Rust parser means consistent behavior across platforms
- **Maintainability**: Changes to the Cooklang spec are implemented once in Rust and automatically available here

## Migration from @cooklang/cooklang-ts

If you're migrating from the previous TypeScript-native `@cooklang/cooklang-ts` package (v1.x), please see the [Migration Guide](MIGRATION.md).

**Key differences:**
- Different package name: `@cooklang/cooklang-ts` → `@cooklang/cooklang`
- API changes: The WASM implementation has a different API surface
- Version numbering: This package tracks the Rust core version (currently 0.17.x)

## API Reference

### Parser

```typescript
import { Parser } from '@cooklang/cooklang';

const parser = new Parser();
const recipe = parser.parse(recipeText);
```

### Recipe Structure

The parsed recipe contains:

```typescript
interface Recipe {
  sections: Section[];
  metadata: Record<string, string>;
  ingredients: Ingredient[];
  cookware: Cookware[];
  timers: Timer[];
}
```

### Helper Functions

```typescript
import {
  ingredient_should_be_listed,
  ingredient_display_name,
  cookware_should_be_listed,
  cookware_display_name,
  quantity_display,
  grouped_quantity_display,
  grouped_quantity_is_empty
} from '@cooklang/cooklang';
```

### Value Extraction

Utility functions for working with quantities:

```typescript
import { getNumericValue, extractNumericRange } from '@cooklang/cooklang';

const value = ingredient.quantity?.value;
const numeric = getNumericValue(value); // 2.5
const range = extractNumericRange(value); // { start: 2, end: 3 }
```

## Version Synchronization

This package version tracks the Rust `cooklang-rs` version. For example:
- npm `@cooklang/cooklang@0.17.2` = Rust `cooklang-rs@0.17.2`

We use 0.x versioning to match the Rust core library. The parser is production-ready and actively maintained. We'll bump to 1.0 when the Rust core reaches 1.0.

## Browser Support

This package works in:
- Node.js 16+
- Modern browsers (Chrome, Firefox, Safari, Edge)
- Bundlers (Webpack, Vite, Rollup, etc.)

## Contributing

This package is part of the [cooklang-rs](https://github.com/cooklang/cooklang-rs) monorepo. Contributions are welcome!

## License

MIT - see [LICENSE](../LICENSE)

## Links

- [Cooklang Specification](https://cooklang.org/docs/spec/)
- [Cooklang Website](https://cooklang.org)
- [GitHub Repository](https://github.com/cooklang/cooklang-rs)
- [Issue Tracker](https://github.com/cooklang/cooklang-rs/issues)
