use std::collections::HashMap;

use cooklang::aisle::Category as OriginalAisleCategory;

/// An ingredient with its name and aliases for aisle categorization
#[derive(uniffi::Record, Debug, Clone)]
pub struct AisleIngredient {
    pub name: String,
    pub aliases: Vec<String>,
}

/// Maps ingredient names to their category names for quick lookup
pub type AisleReverseCategory = HashMap<String, String>;

/// A shopping aisle category containing related ingredients
#[derive(uniffi::Record, Debug, Clone)]
pub struct AisleCategory {
    pub name: String,
    pub ingredients: Vec<AisleIngredient>,
}

/// Configuration for organizing ingredients into shopping aisles
#[derive(uniffi::Object, Debug, Clone)]
pub struct AisleConf {
    pub categories: Vec<AisleCategory>, // cache for quick category search
    pub cache: AisleReverseCategory,
    pub common_names: HashMap<String, String>, // lowercase name/alias -> common name
}

#[uniffi::export]
impl AisleConf {
    /// Returns the category name for a given ingredient
    ///
    /// # Arguments
    /// * `ingredient_name` - The name of the ingredient to categorize
    ///
    /// # Returns
    /// The category name if the ingredient is found, None otherwise
    pub fn category_for(&self, ingredient_name: String) -> Option<String> {
        self.cache.get(&ingredient_name).cloned()
    }

    /// Returns all categories in the order they appear in the aisle configuration file
    pub fn categories(&self) -> Vec<AisleCategory> {
        self.categories.clone()
    }

    /// Returns the common name for an ingredient using aisle configuration
    ///
    /// Performs case-insensitive lookup against ingredient names and aliases.
    /// Returns the original name if not found in the configuration.
    pub fn common_name_for(&self, ingredient_name: String) -> String {
        self.common_names
            .get(&ingredient_name.to_lowercase())
            .cloned()
            .unwrap_or(ingredient_name)
    }
}

pub fn into_category(original: &OriginalAisleCategory) -> AisleCategory {
    let mut ingredients: Vec<AisleIngredient> = Vec::new();

    original.ingredients.iter().for_each(|i| {
        let mut it = i.names.iter();

        let name = it.next().unwrap().to_string();
        let aliases: Vec<String> = it.map(|v| v.to_string()).collect();

        ingredients.push(AisleIngredient { name, aliases });
    });

    AisleCategory {
        name: original.name.to_string(),
        ingredients,
    }
}
