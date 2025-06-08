use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SimpleSection {
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SimpleRecipe {
    pub sections: Vec<SimpleSection>,
}

#[derive(Serialize, Deserialize)]
pub struct ParsedRecipe {
    pub recipe: SimpleRecipe,
    pub report: String,
}
