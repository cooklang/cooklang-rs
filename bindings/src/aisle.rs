use std::collections::HashMap;

use cooklang::aisle::Category as OriginalAisleCategory;

#[derive(uniffi::Record, Debug, Clone)]
pub struct AisleIngredient {
    pub name: String,
    pub aliases: Vec<String>,
}

pub type AisleReverseCategory = HashMap<String, String>;

#[derive(uniffi::Record, Debug, Clone)]
pub struct AisleCategory {
    pub name: String,
    pub ingredients: Vec<AisleIngredient>,
}

#[derive(uniffi::Object, Debug, Clone)]
pub struct AisleConf {
    pub categories: Vec<AisleCategory>, // cache for quick category search
    pub cache: AisleReverseCategory,
}

#[uniffi::export]
impl AisleConf {
    pub fn category_for(&self, ingredient_name: String) -> Option<String> {
        self.cache.get(&ingredient_name).cloned()
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
