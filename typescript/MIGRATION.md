# Migration Guide: @cooklang/cooklang-ts → @cooklang/cooklang

This guide helps you migrate from the TypeScript-native `@cooklang/cooklang-ts` (v1.x) to the new WASM-powered `@cooklang/cooklang` (v0.17+).

## Why Migrate?

The new WASM implementation offers:

- **Better performance**: Native-speed parsing via WebAssembly
- **Consistent behavior**: Shares implementation with the official Rust parser
- **Easier maintenance**: Updates to the Cooklang spec are implemented once
- **Active development**: This is the future of Cooklang parsing for JavaScript/TypeScript

## Quick Migration Checklist

- [ ] Update package name in `package.json`
- [ ] Update import statements
- [ ] Change `Recipe` class instantiation to `Parser.parse()`
- [ ] Update property access (e.g., `cookwares` → `cookware`)
- [ ] Remove deprecated methods like `toCooklang()` and `getImageURL()`
- [ ] Test your application thoroughly

## Installation

```bash
# Uninstall old package
npm uninstall @cooklang/cooklang-ts

# Install new package
npm install @cooklang/cooklang
```

## API Changes

### Package Name

```typescript
// Old
import { Recipe, Parser } from '@cooklang/cooklang-ts';

// New
import { Parser } from '@cooklang/cooklang';
```

### Recipe Parsing

The biggest change: the `Recipe` class is removed. Use `Parser.parse()` directly.

```typescript
// Old
import { Recipe } from '@cooklang/cooklang-ts';
const recipe = new Recipe(source);

// New
import { Parser } from '@cooklang/cooklang';
const parser = new Parser();
const recipe = parser.parse(source);
```

### Property Names

Some property names have changed:

```typescript
// Old
recipe.cookwares  // Array of cookware items
recipe.steps      // Array of steps

// New
recipe.cookware   // Array of cookware items (singular)
recipe.sections   // Array of sections (each section contains content/steps)
```

### Metadata Access

Metadata access is the same:

```typescript
// Both old and new
recipe.metadata.servings
recipe.metadata.source
```

### Ingredients and Cookware

The structure is different:

```typescript
// Old
recipe.ingredients  // Flat array of all ingredients
recipe.cookwares   // Flat array of all cookware

// New
recipe.ingredients  // Still available
recipe.cookware    // Singular name
```

### Steps vs Sections

The new parser uses "sections" instead of "steps":

```typescript
// Old
recipe.steps.forEach(step => {
  step.forEach(item => {
    if ('value' in item) {
      console.log(item.value); // Text content
    } else {
      console.log(item.type, item.name); // ingredient/cookware/timer
    }
  });
});

// New
recipe.sections.forEach(section => {
  section.content.forEach(step => {
    step.items.forEach(item => {
      if (item.type === 'text') {
        console.log(item.value);
      } else if (item.type === 'ingredient') {
        console.log(item.name);
      }
    });
  });
});
```

### Removed Features

The following features from the old package are **not available** in the new WASM version:

#### `toCooklang()` Method

The `Recipe.toCooklang()` method that generated Cooklang source from a recipe object is removed.

```typescript
// Old (NO LONGER AVAILABLE)
const recipe = new Recipe(source);
const cooklangString = recipe.toCooklang();

// Workaround: Keep the original source if you need it
const originalSource = source;
const recipe = parser.parse(source);
// Use originalSource when needed
```

#### `getImageURL()` Function

The helper function for constructing image URLs is removed.

```typescript
// Old (NO LONGER AVAILABLE)
import { getImageURL } from '@cooklang/cooklang-ts';
const url = getImageURL('Baked Potato', { extension: 'jpg', step: 2 });

// Workaround: Implement your own helper
function getImageURL(name: string, options?: { step?: number; extension?: 'png' | 'jpg' }) {
  const ext = options?.extension || 'png';
  const step = options?.step ? `.${options.step}` : '';
  return `${name}${step}.${ext}`;
}
```

#### Shopping List

The shopping list feature is not yet exposed in the WASM bindings:

```typescript
// Old (NO LONGER AVAILABLE)
recipe.shoppingList

// Workaround: Build your own shopping list from ingredients
const shoppingList = recipe.ingredients.reduce((acc, ing) => {
  // Your custom logic here
  return acc;
}, {});
```

## Value Extraction

The new package provides helper functions for working with quantity values:

```typescript
import { getNumericValue, extractNumericRange } from '@cooklang/cooklang';

const ingredient = recipe.ingredients[0];
const value = ingredient.quantity?.value;

// Get a single numeric value (for ranges, returns start)
const numeric = getNumericValue(value); // 2.5

// Get range values
const range = extractNumericRange(value); // { start: 2, end: 3 }
```

## Complete Example

### Before (v1.x)

```typescript
import { Recipe, Parser, getImageURL } from '@cooklang/cooklang-ts';

const source = `
>> servings: 4
Add @salt and @pepper to taste.
`;

const recipe = new Recipe(source);

console.log(recipe.metadata.servings); // "4"
console.log(recipe.ingredients[0].name); // "salt"

// Convert back to Cooklang
const cooklangString = recipe.toCooklang();

// Get image URL
const imageUrl = getImageURL('My Recipe', { step: 1 });
```

### After (v0.17+)

```typescript
import { Parser } from '@cooklang/cooklang';

const source = `
>> servings: 4
Add @salt and @pepper to taste.
`;

const parser = new Parser();
const recipe = parser.parse(source);

console.log(recipe.metadata.servings); // "4"
console.log(recipe.ingredients[0].name); // "salt"

// toCooklang() not available - keep original source if needed
const originalSource = source;

// getImageURL() not available - use custom helper
function getImageURL(name: string, options?: { step?: number; extension?: 'png' | 'jpg' }) {
  const ext = options?.extension || 'png';
  const step = options?.step ? `.${options.step}` : '';
  return `${name}${step}.${ext}`;
}

const imageUrl = getImageURL('My Recipe', { step: 1 });
```

## Type Definitions

The new package includes full TypeScript type definitions. Import types as needed:

```typescript
import type {
  Recipe,
  Ingredient,
  Cookware,
  Timer,
  Section,
  Content,
  Step,
  Item,
  Value,
  Quantity
} from '@cooklang/cooklang';
```

## Testing Your Migration

After migrating, ensure you:

1. **Run your test suite** - All tests should pass with the new API
2. **Check recipe parsing** - Verify recipes parse correctly
3. **Validate data access** - Ensure you can access all needed data from parsed recipes
4. **Test edge cases** - Complex recipes, special characters, etc.

## Getting Help

If you encounter issues during migration:

- Check the [README](./README.md) for API reference
- Open an issue on [GitHub](https://github.com/cooklang/cooklang-rs/issues)
- Review the [Cooklang specification](https://cooklang.org/docs/spec/)

## Version Numbering

Don't be alarmed by the 0.x version number! This package:

- Tracks the Rust core version (currently 0.17.x)
- Is production-ready and actively maintained
- Will bump to 1.0 when the Rust core does

The version number reflects synchronization with the Rust parser, not stability.
