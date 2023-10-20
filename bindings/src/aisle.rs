use std::collections::HashMap;

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
