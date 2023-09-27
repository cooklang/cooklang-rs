
struct Ingredient {
    name: String,
    alias: Vec<String>
}

struct Category {
    name: String,
    ingredients: Ingredient
}

#[derive(uniffi::Object, Debug)]
pub struct AisleConf {
    categories: Vec<Category>
}

#[uniffi::export]
impl AisleConf {
    pub fn category_name_for(self, ingredient_name: String) -> String {

    }
}


