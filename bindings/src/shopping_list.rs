use cooklang::shopping_list::{
    self as sl, CheckEntry as OriginalCheckEntry, IngredientItem as OriginalIngredientItem,
    RecipeItem as OriginalRecipeItem, ShoppingList as OriginalShoppingList,
    ShoppingListItem as OriginalShoppingListItem,
};
use std::collections::HashSet;

/// Errors returned by shopping list binding functions.
///
/// UniFFI does not support returning plain `String` as an error across the FFI
/// boundary, so we wrap string messages in a typed error enum.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ShoppingListError {
    #[error("failed to parse shopping list: {message}")]
    Parse { message: String },
    #[error("failed to serialize shopping list: {message}")]
    Serialize { message: String },
}

// ---------------------------------------------------------------------------
// UniFFI wrapper types
// ---------------------------------------------------------------------------

/// A shopping list containing recipe references and free-hand ingredients
#[derive(uniffi::Record, Debug, Clone)]
pub struct ShoppingList {
    pub items: Vec<ShoppingListItem>,
}

/// An item in the shopping list
#[derive(uniffi::Enum, Debug, Clone)]
pub enum ShoppingListItem {
    /// A recipe reference with a path, optional multiplier, and children
    Recipe {
        path: String,
        multiplier: Option<f64>,
        children: Vec<ShoppingListItem>,
    },
    /// A free-hand ingredient with a name and optional quantity
    Ingredient { name: String, quantity: Option<String> },
}

/// An entry in the checked log
#[derive(uniffi::Enum, Debug, Clone)]
pub enum CheckEntry {
    /// Ingredient was checked (acquired)
    Checked { name: String },
    /// Ingredient was unchecked
    Unchecked { name: String },
}

// ---------------------------------------------------------------------------
// Conversions: original → binding
// ---------------------------------------------------------------------------

impl From<&OriginalShoppingList> for ShoppingList {
    fn from(list: &OriginalShoppingList) -> Self {
        ShoppingList {
            items: list.items.iter().map(ShoppingListItem::from).collect(),
        }
    }
}

impl From<&OriginalShoppingListItem> for ShoppingListItem {
    fn from(item: &OriginalShoppingListItem) -> Self {
        match item {
            OriginalShoppingListItem::Recipe(r) => ShoppingListItem::Recipe {
                path: r.path.clone(),
                multiplier: r.multiplier,
                children: r.children.iter().map(ShoppingListItem::from).collect(),
            },
            OriginalShoppingListItem::Ingredient(i) => ShoppingListItem::Ingredient {
                name: i.name.clone(),
                quantity: i.quantity.clone(),
            },
        }
    }
}

impl From<&OriginalCheckEntry> for CheckEntry {
    fn from(entry: &OriginalCheckEntry) -> Self {
        match entry {
            OriginalCheckEntry::Checked(name) => CheckEntry::Checked {
                name: name.clone(),
            },
            OriginalCheckEntry::Unchecked(name) => CheckEntry::Unchecked {
                name: name.clone(),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Conversions: binding → original
// ---------------------------------------------------------------------------

impl From<&ShoppingList> for OriginalShoppingList {
    fn from(list: &ShoppingList) -> Self {
        OriginalShoppingList {
            items: list.items.iter().map(OriginalShoppingListItem::from).collect(),
        }
    }
}

impl From<&ShoppingListItem> for OriginalShoppingListItem {
    fn from(item: &ShoppingListItem) -> Self {
        match item {
            ShoppingListItem::Recipe {
                path,
                multiplier,
                children,
            } => OriginalShoppingListItem::Recipe(OriginalRecipeItem {
                path: path.clone(),
                multiplier: *multiplier,
                children: children.iter().map(OriginalShoppingListItem::from).collect(),
            }),
            ShoppingListItem::Ingredient { name, quantity } => {
                OriginalShoppingListItem::Ingredient(OriginalIngredientItem {
                    name: name.clone(),
                    quantity: quantity.clone(),
                })
            }
        }
    }
}

impl From<&CheckEntry> for OriginalCheckEntry {
    fn from(entry: &CheckEntry) -> Self {
        match entry {
            CheckEntry::Checked { name } => OriginalCheckEntry::Checked(name.clone()),
            CheckEntry::Unchecked { name } => OriginalCheckEntry::Unchecked(name.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// Public helpers used by lib.rs exported functions
// ---------------------------------------------------------------------------

/// Parse a `.shopping-list` file into a ShoppingList.
pub fn parse_shopping_list_impl(input: &str) -> Result<ShoppingList, ShoppingListError> {
    sl::parse(input)
        .map(|list| ShoppingList::from(&list))
        .map_err(|e| ShoppingListError::Parse {
            message: e.to_string(),
        })
}

/// Serialize a ShoppingList back to the `.shopping-list` format.
pub fn write_shopping_list_impl(list: &ShoppingList) -> Result<String, ShoppingListError> {
    let original = OriginalShoppingList::from(list);
    let mut buf = Vec::new();
    sl::write(&original, &mut buf).map_err(|e| ShoppingListError::Serialize {
        message: e.to_string(),
    })?;
    String::from_utf8(buf).map_err(|e| ShoppingListError::Serialize {
        message: e.to_string(),
    })
}

/// Parse a `.shopping-checked` log file.
pub fn parse_checked_impl(input: &str) -> Vec<CheckEntry> {
    sl::parse_checked(input)
        .iter()
        .map(CheckEntry::from)
        .collect()
}

/// Replay a checked log and return the set of currently checked ingredient
/// names (lowercased).
pub fn checked_set_impl(entries: &[CheckEntry]) -> HashSet<String> {
    let original: Vec<OriginalCheckEntry> = entries.iter().map(OriginalCheckEntry::from).collect();
    sl::checked_set(&original)
}

/// Serialize a single check entry to string.
pub fn write_check_entry_impl(entry: &CheckEntry) -> Result<String, ShoppingListError> {
    let original = OriginalCheckEntry::from(entry);
    let mut buf = Vec::new();
    sl::write_check_entry(&original, &mut buf).map_err(|e| ShoppingListError::Serialize {
        message: e.to_string(),
    })?;
    String::from_utf8(buf).map_err(|e| ShoppingListError::Serialize {
        message: e.to_string(),
    })
}

/// Compact a checked log against the ingredient names currently in the
/// shopping list.
///
/// Callers should pass the fully-aggregated ingredient names the user
/// actually sees. A raw on-disk [`ShoppingList`] usually contains only
/// recipe references, not ingredients, so it cannot be used directly.
pub fn compact_checked_impl(
    entries: &[CheckEntry],
    current_ingredients: &[String],
) -> Vec<CheckEntry> {
    let original_entries: Vec<OriginalCheckEntry> =
        entries.iter().map(OriginalCheckEntry::from).collect();
    sl::compact_checked(
        &original_entries,
        current_ingredients.iter().map(String::as_str),
    )
    .iter()
    .map(CheckEntry::from)
    .collect()
}
