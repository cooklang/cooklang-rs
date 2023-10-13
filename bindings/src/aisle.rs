#[derive(uniffi::Record, Debug, Clone)]
pub struct AisleIngredient {
    pub name: String,
    pub aliases: Vec<String>,
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct AisleCategory {
    pub name: String,
    pub ingredients: Vec<AisleIngredient>,
}

#[derive(uniffi::Object, Debug, Clone)]
pub struct AisleConf {
    pub categories: Vec<AisleCategory>, // cache for quick category search
}

#[uniffi::export]
impl AisleConf {
    pub fn add_category(&self, _ingredient: AisleCategory) {
        todo!();
    }

    pub fn add_ingredient(&self, _category_name: String, _name: String, _aliases: Vec<String>) {
        todo!();
    }

    pub fn category_for(&self, _ingredient_name: String) -> String {
        todo!();
    }
}
