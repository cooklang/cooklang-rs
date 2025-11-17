# Shopping List Synonym Grouping Research

## Overview

This document describes the current implementation status and required changes to support ingredient synonym grouping in shopping lists, as specified in the Cooklang aisle configuration spec.

## Aisle Configuration Specification

The aisle.conf format supports defining ingredient synonyms using the `|` separator:

```
[produce]
potatoes

[dairy]
milk
butter

[deli]
chicken

[canned goods]
tuna|chicken of the sea
```

In this example, "tuna" is the canonical name, and "chicken of the sea" is a synonym. When creating shopping lists, ingredients using either name should be grouped together under the canonical name "tuna".

## Current Implementation Status

### ✅ Core Library Support (Fully Implemented)

The Rust core library (`src/aisle.rs` and `src/ingredient_list.rs`) **already fully supports** synonym grouping:

#### 1. Aisle Configuration Parsing (`src/aisle.rs`)

- **`Ingredient` struct** (lines 45-50): Stores `names: Vec<&str>` where the first name is canonical
- **Parsing** (lines 186-209): Correctly parses `|` separated synonyms
- **`ingredients_info()` method** (lines 77-97): Creates a HashMap where:
  - Every ingredient name (including all synonyms) maps to `IngredientInfo`
  - `IngredientInfo.common_name` always points to the first (canonical) name
  - Example: Both "tuna" and "chicken of the sea" → `common_name: "tuna"`

**Test evidence**: Lines 478-491 include a test `synonym_lookup` that verifies both names map to the same canonical name.

#### 2. Shopping List Categorization (`src/ingredient_list.rs`)

- **`IngredientList::categorize()` method** (lines 348-364): Correctly groups by canonical name
- **Line 358**: Uses `info.common_name.to_string()` as the key when inserting ingredients
- **Result**: Ingredients with different names but matching synonyms are automatically merged under the canonical name

**Example flow**:
```rust
// If shopping list has:
// - "tuna" with 200g
// - "chicken of the sea" with 100g

list.categorize(&aisle_conf);

// Result in CategorizedIngredientList:
// [canned goods]
//   "tuna": 300g  // Both quantities merged under canonical name
```

### ❌ Bindings API Gap (Missing Feature)

The UniFFI bindings (`bindings/src/lib.rs` and `bindings/src/aisle.rs`) used by mobile apps and external tools **do not expose** the synonym grouping functionality:

#### Current Bindings API

1. **`parse_aisle_config()`** (bindings/src/lib.rs:101-145)
   - Parses aisle configuration
   - Builds `AisleReverseCategory` cache mapping ingredient names to category names
   - **Problem**: Only maps to category, not to canonical name

2. **`AisleConf::category_for()`** (bindings/src/aisle.rs:38-40)
   - Returns category name for an ingredient
   - **Problem**: Doesn't return the canonical name

3. **`combine_ingredients()`** (bindings/src/lib.rs:155-158)
   - Combines ingredients by their recipe names
   - **Problem**: Doesn't normalize to canonical names

#### Example Code Problem

The example in `bindings/README.md` (lines 380-398) shows the issue:

```rust
let all_recipe_ingredients_combined = combine_ingredients(recipe.ingredients);
all_recipe_ingredients_combined.iter().for_each(|(name, grouped_quantity)| {
    let category = config.category_for(name).unwrap_or("Other");
    let mut entry = result.get(category).or_default();
    let mut ingredient_quantity = entry.get(name).or_default();  // ← Uses original name!
    ingredient_quantity.merge(grouped_quantity);
});
```

**Result**: If recipes use both "tuna" and "chicken of the sea", they create separate shopping list entries instead of being merged.

## Required Code Changes

### For Tools Using Core Library Directly (e.g., Rust CLI tools)

**Status**: ✅ **No changes needed**

Tools written in Rust that use the core library can already use synonym grouping:

```rust
use cooklang::{CooklangParser, aisle, ingredient_list::IngredientList};

// Parse recipes and combine ingredients
let mut shopping_list = IngredientList::new();
shopping_list.add_recipe(&recipe1, &converter, false);
shopping_list.add_recipe(&recipe2, &converter, false);

// Parse aisle configuration
let aisle_conf = aisle::parse(&aisle_config_text)?;

// Categorize - automatically groups synonyms under canonical names
let categorized = shopping_list.categorize(&aisle_conf);

// Iterate over categories and ingredients
for (category_name, ingredient_list) in categorized.iter() {
    println!("[{}]", category_name);
    for (ingredient_name, quantity) in ingredient_list.iter() {
        // ingredient_name is already the canonical name
        println!("  {}: {}", ingredient_name, quantity);
    }
}
```

### For Tools Using Bindings (Mobile Apps, External Tools)

**Status**: ⚠️ **Changes Required**

Three approaches, in order of recommendation:

#### Option 1: Add `canonical_name_for()` Method (Recommended)

Add a new method to expose the canonical name mapping:

**File**: `bindings/src/aisle.rs`

```rust
#[uniffi::export]
impl AisleConf {
    /// Returns the category name for a given ingredient
    pub fn category_for(&self, ingredient_name: String) -> Option<String> {
        self.cache.get(&ingredient_name).cloned()
    }

    /// Returns the canonical name for a given ingredient
    ///
    /// # Arguments
    /// * `ingredient_name` - The name of the ingredient (can be a synonym)
    ///
    /// # Returns
    /// The canonical name if the ingredient is found, None otherwise
    ///
    /// # Example
    /// ```
    /// let canonical = config.canonical_name_for("chicken of the sea");
    /// // Returns: Some("tuna")
    /// ```
    pub fn canonical_name_for(&self, ingredient_name: String) -> Option<String> {
        // Find the category first
        let category_name = self.cache.get(&ingredient_name)?;

        // Find the category
        let category = self.categories.iter()
            .find(|cat| &cat.name == category_name)?;

        // Find the ingredient and return its canonical name (first name)
        for ingredient in &category.ingredients {
            if ingredient.name == ingredient_name {
                return Some(ingredient.name.clone());
            }
            if ingredient.aliases.contains(&ingredient_name) {
                return Some(ingredient.name.clone());
            }
        }

        None
    }
}
```

**File**: `bindings/src/lib.rs`

Update `parse_aisle_config()` to also cache canonical names:

```rust
#[derive(uniffi::Object, Debug, Clone)]
pub struct AisleConf {
    pub categories: Vec<AisleCategory>,
    pub cache: AisleReverseCategory,  // ingredient -> category
    pub canonical_cache: HashMap<String, String>,  // ingredient -> canonical name
}

#[uniffi::export]
pub fn parse_aisle_config(input: String) -> Arc<AisleConf> {
    let mut categories: Vec<AisleCategory> = Vec::new();
    let mut cache: AisleReverseCategory = AisleReverseCategory::default();
    let mut canonical_cache: HashMap<String, String> = HashMap::default();

    let result = parse_lenient(&input);
    let parsed = match result.into_result() {
        Ok((parsed, warnings)) => {
            if warnings.has_warnings() {
                for diag in warnings.iter() {
                    eprintln!("Warning: {}", diag);
                }
            }
            parsed
        }
        Err(report) => {
            for diag in report.iter() {
                eprintln!("Error: {}", diag);
            }
            Default::default()
        }
    };

    parsed.categories.iter().for_each(|c| {
        let category = into_category(c);

        category.ingredients.iter().for_each(|i| {
            // Cache category
            cache.insert(i.name.clone(), category.name.clone());

            // Cache canonical name for the name itself
            canonical_cache.insert(i.name.clone(), i.name.clone());

            // Cache aliases
            i.aliases.iter().for_each(|a| {
                cache.insert(a.to_string(), category.name.clone());
                // Cache canonical name for each alias
                canonical_cache.insert(a.to_string(), i.name.clone());
            });
        });

        categories.push(category);
    });

    let config = AisleConf {
        categories,
        cache,
        canonical_cache,
    };

    Arc::new(config)
}
```

**Updated Usage Example** (`bindings/README.md`):

```rust
let recipe = parse_recipe(text, 1.0);
let config = parse_aisle_config(text);
let mut result = HashMap<String, HashMap<String, GroupedQuantity>>::new();

let all_recipe_ingredients_combined = combine_ingredients(recipe.ingredients);
all_recipe_ingredients_combined.iter().for_each(|(name, grouped_quantity)| {
    // Get canonical name (falls back to original name if not in config)
    let canonical_name = config.canonical_name_for(name).unwrap_or(name.clone());

    // Get category name
    let category = config.category_for(canonical_name).unwrap_or("Other");

    // Use canonical name when storing
    let mut entry = result.entry(category).or_default();
    let mut ingredient_quantity = entry.entry(canonical_name).or_default();
    ingredient_quantity.merge(grouped_quantity);
});
```

#### Option 2: Expose `categorize()` Method

Add a binding for the existing `categorize()` method:

**Pros**:
- Simpler for API consumers
- Directly reuses core library logic

**Cons**:
- Requires exposing `CategorizedIngredientList` type
- Less flexible for custom grouping logic

**Implementation**: Create bindings for `IngredientList::categorize()` and `CategorizedIngredientList`.

#### Option 3: Add `combine_and_categorize()` Helper

Add a high-level helper that does everything in one call:

**Pros**:
- Easiest for API consumers
- One-line solution

**Cons**:
- Less flexible
- More complex binding implementation

## Testing Requirements

### New Tests for Bindings

Add tests to verify synonym grouping works through the bindings API:

**File**: `bindings/tests/integration_test.rs` (or similar)

```rust
#[test]
fn test_synonym_grouping_in_shopping_list() {
    let aisle_config = r#"
[canned goods]
tuna|chicken of the sea
"#;

    let recipe1 = parse_recipe("@tuna{200%g}", 1.0);
    let recipe2 = parse_recipe("@chicken of the sea{100%g}", 1.0);

    let config = parse_aisle_config(aisle_config.to_string());

    // Verify canonical name mapping
    assert_eq!(
        config.canonical_name_for("tuna".to_string()),
        Some("tuna".to_string())
    );
    assert_eq!(
        config.canonical_name_for("chicken of the sea".to_string()),
        Some("tuna".to_string())
    );

    // Verify shopping list grouping
    // (implementation depends on chosen option)
}
```

### Documentation Updates

Update the following files:

1. **`bindings/README.md`**:
   - Update shopping list example to show synonym grouping
   - Add example with synonyms

2. **API documentation**:
   - Document new methods with clear examples
   - Explain canonical name concept

3. **Migration guide**:
   - If changing existing behavior, provide migration guide for existing API consumers

## Summary

### Current Status

- ✅ **Core Rust library**: Fully supports synonym grouping via `IngredientList::categorize()`
- ❌ **Bindings API**: Missing canonical name exposure

### Required Changes

1. **For Rust tools**: No changes needed, feature already works
2. **For bindings**:
   - Add `canonical_name_for()` method (Option 1 - Recommended)
   - Add canonical name cache in `AisleConf` struct
   - Update example code in README
   - Add integration tests

### Implementation Priority

1. Add `canonical_cache` to bindings `AisleConf` (bindings/src/aisle.rs)
2. Populate cache in `parse_aisle_config()` (bindings/src/lib.rs)
3. Add `canonical_name_for()` method (bindings/src/aisle.rs)
4. Update README example (bindings/README.md)
5. Add tests for synonym grouping through bindings

### Files to Modify

| File | Changes |
|------|---------|
| `bindings/src/aisle.rs` | Add `canonical_cache` field, add `canonical_name_for()` method |
| `bindings/src/lib.rs` | Update `parse_aisle_config()` to populate canonical cache |
| `bindings/README.md` | Update shopping list example to use canonical names |
| `bindings/tests/` | Add integration test for synonym grouping |
| `bindings/cooklang.udl` | Add `canonical_name_for` to UniFFI interface definition |

## Conclusion

The core library already implements synonym grouping correctly. The only missing piece is exposing this functionality through the bindings API so that mobile apps and external tools (like cookcli) can use it. The recommended approach is to add a `canonical_name_for()` method with an internal cache for O(1) lookups.
