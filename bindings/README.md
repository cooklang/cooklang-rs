# Bindings

This repository exports UniFFI difined bindings that can be used to call Rust Cooklang parser code from languages other than Rust: Kotlin, Swift, Ruby, Python and [some other languages](https://mozilla.github.io/uniffi-rs/#third-party-foreign-language-bindings).

## UniFFI

[UniFFI](https://mozilla.github.io/uniffi-rs/Overview.html) is a brilliant way to define a cross-language interface and associated tools. Rust compiles a C-compatible library with UniFFI metadata baked. Based on this metadata UniFFI compiler can create snippets of code in foreign language that mirrors exposed Rust API.

This particular library employes new-ish [procedural macroses](https://mozilla.github.io/uniffi-rs/proc_macro/index.html) to define exported methods and data-types.

## Exposed API

This library exports methods:

```rust
    // full parsing, returns full recipe object with meta
    parse_recipe(input: String, scaling_factor: f64) -> CooklangRecipe;
    // fast metadata parsing, recipe text is not parsed
    parse_metadata(input: String, scaling_factor: f64) -> CooklangMetadata;
    // parse aisle config to use in shopping list
    parse_aisle_config(input: String) -> Arc<AisleConfig>;

    // dereferences component reference to component
    // usage example:
    // let ingredient = deref_component(recipe, Item::IngredientRef { index: 0 });
    deref_component(recipe: &CooklangRecipe, item: Item) -> Component;
    // dereferences ingredient reference to ingredient
    // usage example:
    // let ingredient = deref_ingredient(recipe, 0);
    deref_ingredient(recipe: &CooklangRecipe, index: u32) -> Ingredient;
    // dereferences cookware reference to cookware
    // usage example:
    // let cookware = deref_cookware(recipe, 0);
    deref_cookware(recipe: &CooklangRecipe, index: u32) -> Cookware;
    // dereferences timer reference to timer
    // usage example:
    // let timer = deref_timer(recipe, 0);
    deref_timer(recipe: &CooklangRecipe, index: u32) -> Timer;

    // combines ingredient lists into one
    // usage example:
    // let all_recipe_ingredients_combined = combine_ingredients(recipe.ingredients);
    // if multiple recipes need to be combined, combine their ingredients lists and pass them to this method
    combine_ingredients(ingredients: Vec<Ingredient>) -> IngredientList;
    // combines ingredient lists into one
    // usage example:
    // let combined_ingredients_from_section1 = combine_ingredients_selected(recipe.ingredients, section1.ingredient_refs);
    // let combined_ingredients_from_step1 = combine_ingredients_selected(recipe.ingredients, step1.ingredient_refs);
    combine_ingredients_selected(ingredients: Vec<Ingredient>, indices: Vec<u32>) -> IngredientList;

    // Metadata helper functions - convenient accessors for common recipe metadata fields
    // Returns the servings from metadata (e.g., "4" or "2-3 portions")
    metadata_servings(recipe: &CooklangRecipe) -> Option<Servings>;

    // Returns the recipe title
    metadata_title(recipe: &CooklangRecipe) -> Option<String>;

    // Returns the recipe description
    metadata_description(recipe: &CooklangRecipe) -> Option<String>;

    // Returns the recipe tags as a list
    metadata_tags(recipe: &CooklangRecipe) -> Option<Vec<String>>;

    // Returns the recipe author with optional URL
    metadata_author(recipe: &CooklangRecipe) -> Option<NameAndUrl>;

    // Returns the recipe source with optional URL
    metadata_source(recipe: &CooklangRecipe) -> Option<NameAndUrl>;

    // Returns the recipe time (total or prep+cook)
    metadata_time(recipe: &CooklangRecipe) -> Option<RecipeTime>;

    // Get a custom metadata value by key
    metadata_get(recipe: &CooklangRecipe, key: String) -> Option<String>;

    // Get a standard metadata value using StdKey enum
    metadata_get_std(recipe: &CooklangRecipe, key: StdKey) -> Option<String>;

    // Get all non-standard (custom) metadata keys
    // Returns a list of all metadata keys that are not part of the Cooklang standard
    // Useful for discovering custom metadata fields in recipes
    // usage example:
    // let custom_keys = metadata_custom_keys(&recipe); // Returns ["nutrition", "rating", "custom-field"]
    metadata_custom_keys(recipe: &CooklangRecipe) -> Vec<String>;

    // Format a Value to a display string with proper fraction handling
    // Converts decimals to fractions where appropriate (e.g., 0.666667 -> "2/3", 1.5 -> "1 1/2")
    // Handles floating point precision issues from scaling (e.g., 0.89999999 -> "0.9")
    // Returns None for Empty values
    // usage example:
    // let formatted = format_value(&Value::Number { value: 0.666667 }); // Returns Some("2/3")
    format_value(value: &Value) -> Option<String>;

    // Parse a string into a Value
    // Supports fractions (e.g., "1/2" -> 0.5), mixed numbers (e.g., "1 1/2" -> 1.5)
    // Supports ranges (e.g., "1/2 - 3/4" -> Range{0.5, 0.75})
    // Falls back to Text if not parseable as number or range
    // usage example:
    // let value = parse_value("2/3".to_string()); // Returns Value::Number { value: 0.666667 }
    parse_value(s: String) -> Value;

    // Format an Amount to a display string with units
    // Combines formatted quantity with units (e.g., "2/3 cups", "1 1/2 tsp")
    // usage example:
    // let formatted = format_amount(&amount); // Returns "2/3 cups" for 0.666667 cups
    format_amount(amount: &Amount) -> String;
```

### Exposed data structures

```rust
    /// A recipe is a collection of sections, each containing blocks of content.
    struct CooklangRecipe {
        /// Recipe metadata like title, source, etc.
        metadata: CooklangMetadata,
        /// List of recipe sections, each containing blocks of content, like steps, notes, etc.
        sections: Vec<Section>,
        /// List of all ingredients used in the recipe in order of use. Not quantity combined.
        ingredients: Vec<Ingredient>,
        /// List of all cookware used in the recipe.
        cookware: Vec<Cookware>,
        /// List of all timers used in the recipe.
        timers: Vec<Timer>,
    }

    /// Represents a distinct section of a recipe, optionally with a title
    struct Section {
        /// Optional section title (e.g., "Dough", "Topping", etc.)
        title: Option<String>,
        /// List of content blocks in this section. Each block can be a step or a note.
        blocks: Vec<Block>,
        /// Indices of  ingredients used in this section.
        ingredient_refs: Vec<u32>,
        /// Indices of cookware used in this section.
        cookware_refs: Vec<u32>,
        /// Indices of timers used in this section.
        timer_refs: Vec<u32>,
    }

    /// A block can either be a cooking step or a note
    enum Block {
        /// A cooking instruction step
        Step(Step),
        /// An informational note
        Note(BlockNote),
    }

    /// Represents a single cooking instruction step
    struct Step {
        /// List of items that make up this step (text and references)
        items: Vec<Item>,
        /// Indices of ingredients used in this step
        ingredient_refs: Vec<u32>,
        /// Indices of cookware used in this step
        cookware_refs: Vec<u32>,
        /// Indices of timers used in this step
        timer_refs: Vec<u32>,
    }

    /// A text note within the recipe
    struct BlockNote {
        /// The content of the note
        text: String,
    }

    /// Represents an ingredient in the recipe
    struct Ingredient {
        /// Name of the ingredient
        name: String,
        /// Optional quantity and units
        amount: Option<Amount>,
        /// Optional descriptor instructions (e.g., "chopped", "diced")
        descriptor: Option<String>,
    }

    /// Represents a piece of cookware used in the recipe
    struct Cookware {
        name: String,
        amount: Option<Amount>,
    }

    /// Represents a timer in the recipe
    struct Timer {
        /// Optional timer name (e.g., "boiling", "baking", etc.)
        name: Option<String>,
        amount: Option<Amount>,
    }

    /// Represents an item in the recipe
    enum Item {
        /// A text item
        Text { value: String },
        /// An ingredient reference index
        IngredientRef { index: u32 },
        /// A cookware reference index
        CookwareRef { index: u32 },
        /// A timer reference index
        TimerRef { index: u32 },
    }

    /// Represents a quantity in the recipe
    struct Amount {
        /// Quantity value
        quantity: Value,
        /// Optional units
        units: Option<String>,
    }

    /// Represents a value in the recipe
    enum Value {
        Number { value: f64 },
        Range { start: f64, end: f64 },
        Text { value: String },
        Empty,
    }

    /// Represents the metadata of the recipe
    type CooklangMetadata = HashMap<String, String>;
    /// Represents a list of ingredients that are grouped by name and quantity
    type IngredientList = HashMap<String, GroupedQuantity>;
    /// Represents a grouped quantity for multiple unit types
    // \
    //  |- <litre,Number> => 1.2
    //  |- <litre,Text> => half
    //  |- <,Text> => pinch
    //  |- <,Empty> => Some
    type GroupedQuantity = HashMap<GroupedQuantityKey, Value>;

    /// Represents a grouped quantity key
    struct GroupedQuantityKey {
        /// Name of the grouped quantity
        name: String,
        /// Type of the grouped quantity
        unit_type: QuantityType,
    }

    /// Represents the type of the grouped quantity
    enum QuantityType {
        Number,
        Range,
        Text,
        Empty,
    }

    /// Represents recipe servings
    enum Servings {
        /// Numeric servings (e.g., 4)
        Number { value: u32 },
        /// Text servings (e.g., "2-3 portions")
        Text { value: String },
    }

    /// Represents a name with optional URL
    struct NameAndUrl {
        /// Optional name (e.g., "John Doe")
        name: Option<String>,
        /// Optional URL (e.g., "https://johndoe.com")
        url: Option<String>,
    }

    /// Represents recipe time
    enum RecipeTime {
        /// Total time in minutes
        Total { minutes: u32 },
        /// Separate prep and cook times
        Composed {
            prep_time: Option<u32>,
            cook_time: Option<u32>,
        },
    }

    /// Standard metadata keys
    enum StdKey {
        Title,
        Description,
        Tags,
        Author,
        Source,
        Course,
        Time,
        PrepTime,
        CookTime,
        Servings,
        Difficulty,
        Cuisine,
        Diet,
        Images,
        Locale,
    }
```


### Metadata Access Example

The metadata helper functions provide convenient access to common recipe metadata fields:

```rust
let recipe = parse_recipe(recipe_text, 1.0);

// Access common metadata fields
if let Some(title) = metadata_title(&recipe) {
    println!("Recipe: {}", title);
}

if let Some(servings) = metadata_servings(&recipe) {
    match servings {
        Servings::Number { value } => println!("Serves: {}", value),
        Servings::Text { value } => println!("Serves: {}", value),
    }
}

if let Some(tags) = metadata_tags(&recipe) {
    println!("Tags: {}", tags.join(", "));
}

if let Some(author) = metadata_author(&recipe) {
    if let Some(name) = author.name {
        print!("Author: {}", name);
    }
    if let Some(url) = author.url {
        print!(" ({})", url);
    }
}

// Access custom metadata
if let Some(difficulty) = metadata_get(&recipe, "difficulty".to_string()) {
    println!("Difficulty: {}", difficulty);
}

// Or use standard keys
if let Some(cuisine) = metadata_get_std(&recipe, StdKey::Cuisine) {
    println!("Cuisine: {}", cuisine);
}

// Discover all custom metadata fields
let custom_keys = metadata_custom_keys(&recipe);
for key in custom_keys {
    if let Some(value) = metadata_get(&recipe, key.clone()) {
        println!("Custom field '{}': {}", key, value);
    }
}
```

### Formatting Values Example (Android/Kotlin)

Instead of implementing value formatting in Kotlin (which can produce values like "0.66666666 cups"), use the provided formatting functions:

```kotlin
// Before (problematic Kotlin code):
// internal fun Value.asString(): String? {
//     return when (this) {
//         Value.Empty -> null
//         is Value.Number -> value.toFormattedString() // Would show "0.66666666"
//         ...
//     }
// }

// After (using the bindings):
fun formatIngredientAmount(ingredient: Ingredient): String? {
    return ingredient.amount?.let { formatAmount(it) }
}

// Or format just the value:
fun formatValue(value: Value): String? {
    return formatValue(value)
}

// Examples:
// formatValue(Value.Number(0.666667)) -> "2/3"
// formatValue(Value.Number(0.5)) -> "1/2"
// formatValue(Value.Number(1.5)) -> "1 1/2"
// formatValue(Value.Number(0.89999999)) -> "0.9" (handles scaling precision issues)
// formatAmount(Amount(Value.Number(0.666667), "cups")) -> "2/3 cups"
```

### Shopping list usage example

Not all categories from AisleConfig are referenced in a shopping list. There could be "Other" category if not defined in the config.

```rust
    // parse
    let recipe = parse_recipe(text, 1.0);
    let config = parse_aisle_config(text);
    // object which we'll use for rendering
    let mut result = HashMap<String, HashMap<String,GroupedQuantity>>::New();
    // iterate over each recipe ingredients and fill results into result object.
    let all_recipe_ingredients_combined = combine_ingredients(recipe.ingredients);
    all_recipe_ingredients_combined.iter().for_each(|(name, grouped_quantity)| {
        // Get category name for current ingredient
        let category = config.category_for(name).unwrap_or("Other");
        // Get list of ingredients for that category
        let mut entry = result.get(category).or_default();
        // Get quantity object for that ingredient
        let mut ingredient_quantity = entry.get(name).or_default();
        // Add extra quantity to it
        ingredient_quantity.merge(grouped_quantity);
    });
```


## Building for Android

### Prepare

Install `rustup` https://www.rust-lang.org/tools/install.

Then add Android targets.

    rustup target add aarch64-linux-android
    rustup target add armv7-linux-androideabi
    rustup target add i686-linux-android
    rustup target add x86_64-linux-android

Install Android NDK https://developer.android.com/studio/projects/install-ndk#default-version.

Add ndk linkers to the PATH variable. Example for ~/.zshrc:

    export PATH=$PATH:/Users/dubadub/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/

### Build

Build library:

    cargo build --lib --target=x86_64-linux-android --release

Biuld foreight language bindings (this will output Kotlin code into `./out` dir:

    cargo run --features="uniffi/cli"  \
      --bin uniffi-bindgen generate \
      --library target/x86_64-linux-android/release/libcooklang.so \
      --language kotlin \
      --out-dir out

See example of a Gradle config [here](https://github.com/cooklang/cooklang-android/blob/main/app/build.gradle#L77-L132) with all required tasks.



## Building for iOS

### Prepare

Install `rustup` https://www.rust-lang.org/tools/install.

Then add iOS targets.

    rustup target add aarch64-apple-ios
    rustup target add x86_64-apple-ios

Install iOS SDK https://developer.apple.com/xcode/resources/.

### Build

Build library:

    cargo build --lib --target=x86_64-apple-ios --release

Biuld foreight language bindings (this will output Swift code into `./out` dir:

    cargo run --features="uniffi/cli"  \
      --bin uniffi-bindgen generate \
      --config uniffi.toml \
      --library ../target/x86_64-apple-ios/release/libcooklang_bindings.a \
      --language swift \
      --out-dir out

See example of a Xcode project [here](https://github.com/cooklang/cooklang-ios/blob/main/Cooklang.xcodeproj).

Combine into universal library:

    mkdir -p ../target/universal/release
    lipo -create -output ../target/universal/release/libcooklang_bindings.a \
      ../target/x86_64-apple-ios/release/libcooklang_bindings.a \
      ../target/aarch64-apple-ios/release/libcooklang_bindings.a



    xcodebuild -create-xcframework \
       -library ../target/aarch64-apple-ios/release/libcooklang_bindings.a \
       -library ../target/x86_64-apple-ios/release/libcooklang_bindings.a \
       -output CooklangParserFFI.xcframework
