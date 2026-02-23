//! Cooklang pantry configuration parser
//!
//! This module provides parsing for pantry inventory files in TOML format.
//! Items can have optional attributes like bought date, expiry date, and quantity.
//!
//! ## Format
//!
//! ```toml
//! [freezer]
//! # Simple item with just quantity
//! cranberries = "500%g"
//!
//! # Item with attributes
//! spinach = { bought = "05.05.2024", expire = "05.05.2025", quantity = "1%kg" }
//!
//! [fridge]
//! milk = { expire = "10.05.2024", quantity = "1%L" }
//! ```
//!
//! This module is only available with the `pantry` [feature](crate::_features).
//!
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    error::{CowStr, Label, RichError, SourceDiag, SourceReport, Stage},
    span::Span,
    PassResult,
};

/// Represents a pantry configuration file
///
/// This type also implements [`Serialize`] and [`Deserialize`], so if you don't
/// like the TOML pantry format you can swap it with any [`serde`]
/// format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PantryConf {
    /// Map of sections to their items (BTreeMap for consistent ordering)
    #[serde(flatten)]
    pub sections: BTreeMap<String, Vec<PantryItem>>,

    /// Index for fast ingredient lookups (lowercase name -> (section, index))
    /// Using BTreeMap for better cache locality and predictable iteration
    #[serde(skip)]
    ingredient_index: BTreeMap<String, Vec<(String, usize)>>,
}

/// A pantry item with optional attributes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PantryItem {
    /// Simple item (just a string name)
    Simple(String),
    /// Item with attributes
    WithAttributes(ItemWithAttributes),
}

/// An item with attributes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemWithAttributes {
    /// Name of the item
    #[serde(rename = "name")]
    pub name: String,
    /// Date when the item was bought (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bought: Option<String>,
    /// Expiry date of the item (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<String>,
    /// Quantity of the item (optional) - stored as string but can be parsed as cooklang Quantity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    /// Low stock threshold quantity (optional) - e.g. "500%ml", "2%kg"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<String>,
}

/// Parse a quantity string to extract numeric value and unit
/// Handles formats like "500%ml", "2%kg", "5", etc.
fn parse_quantity(quantity: &str) -> Option<(f64, String)> {
    // Remove % if present and split into parts
    let cleaned = quantity.replace('%', " ");
    let parts: Vec<&str> = cleaned.split_whitespace().collect();

    if let Some(number_str) = parts.first() {
        if let Ok(value) = number_str.parse::<f64>() {
            let unit = if parts.len() > 1 {
                parts[1].to_lowercase()
            } else {
                String::new() // No unit means it's just a count
            };
            return Some((value, unit));
        }
    }
    None
}

impl PantryItem {
    /// Get the name of the item
    pub fn name(&self) -> &str {
        match self {
            PantryItem::Simple(name) => name,
            PantryItem::WithAttributes(item) => &item.name,
        }
    }

    /// Parse the quantity and return as (value, unit)
    /// Returns None if no quantity or if parsing fails
    pub fn parsed_quantity(&self) -> Option<(f64, String)> {
        self.quantity().and_then(parse_quantity)
    }

    /// Get the bought date if available
    pub fn bought(&self) -> Option<&str> {
        match self {
            PantryItem::Simple(_) => None,
            PantryItem::WithAttributes(item) => item.bought.as_deref(),
        }
    }

    /// Get the expiry date if available
    pub fn expire(&self) -> Option<&str> {
        match self {
            PantryItem::Simple(_) => None,
            PantryItem::WithAttributes(item) => item.expire.as_deref(),
        }
    }

    /// Get the quantity as a string if available
    ///
    /// The quantity should be in cooklang format like "1%kg" or "500%ml"
    pub fn quantity(&self) -> Option<&str> {
        match self {
            PantryItem::Simple(_) => None,
            PantryItem::WithAttributes(item) => item.quantity.as_deref(),
        }
    }

    /// Check if the item is low on stock
    /// Compares current quantity with the low threshold if both are set and have the same unit
    pub fn is_low(&self) -> bool {
        match self {
            PantryItem::Simple(_) => false,
            PantryItem::WithAttributes(item) => {
                if let (Some(quantity), Some(low_threshold)) = (&item.quantity, &item.low) {
                    // Parse both quantities with their units
                    if let (
                        Some((current_val, current_unit)),
                        Some((threshold_val, threshold_unit)),
                    ) = (parse_quantity(quantity), parse_quantity(low_threshold))
                    {
                        // Only compare if units match
                        if current_unit == threshold_unit {
                            return current_val <= threshold_val;
                        }
                    }
                }
                false
            }
        }
    }

    /// Get the low stock threshold if explicitly set
    pub fn low(&self) -> Option<&str> {
        match self {
            PantryItem::Simple(_) => None,
            PantryItem::WithAttributes(item) => item.low.as_deref(),
        }
    }
}

impl PantryConf {
    /// Rebuild the ingredient index after manual modifications to sections
    ///
    /// Call this if you modify the `sections` field directly.
    pub fn rebuild_index(&mut self) {
        self.ingredient_index.clear();
        for (section_name, items) in &self.sections {
            for (idx, item) in items.iter().enumerate() {
                let lowercase_name = item.name().to_lowercase();
                self.ingredient_index
                    .entry(lowercase_name)
                    .or_insert_with(Vec::new)
                    .push((section_name.clone(), idx));
            }
        }
    }

    /// Returns all items across all sections
    pub fn all_items(&self) -> impl Iterator<Item = &PantryItem> {
        self.sections.values().flat_map(|items| items.iter())
    }

    /// Returns all items in a specific section
    pub fn section_items(&self, section: &str) -> Option<&[PantryItem]> {
        self.sections.get(section).map(|v| v.as_slice())
    }

    /// Returns a map of item names to their sections
    pub fn items_by_section(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        for (section, items) in &self.sections {
            for item in items {
                map.insert(item.name(), section.as_str());
            }
        }
        map
    }

    /// Check if an ingredient is in the pantry
    ///
    /// This performs a case-insensitive search using the pre-built index.
    /// O(1) lookup time.
    pub fn has_ingredient(&self, ingredient_name: &str) -> bool {
        let search_name = ingredient_name.to_lowercase();
        self.ingredient_index.contains_key(&search_name)
    }

    /// Find an ingredient in the pantry
    ///
    /// This performs a case-insensitive search using the pre-built index.
    /// Returns the first matching item along with its section name if found.
    /// O(1) lookup time.
    pub fn find_ingredient(&self, ingredient_name: &str) -> Option<(&str, &PantryItem)> {
        let search_name = ingredient_name.to_lowercase();

        if let Some(locations) = self.ingredient_index.get(&search_name) {
            if let Some((section_name, idx)) = locations.first() {
                if let Some(items) = self.sections.get(section_name) {
                    if let Some(item) = items.get(*idx) {
                        return Some((section_name.as_str(), item));
                    }
                }
            }
        }
        None
    }

    /// Find all ingredients matching a name (case-insensitive)
    ///
    /// Returns all matching items across all sections.
    /// O(1) lookup time for finding locations, O(m) for retrieving m matches.
    pub fn find_all_ingredients(&self, ingredient_name: &str) -> Vec<(&str, &PantryItem)> {
        let search_name = ingredient_name.to_lowercase();
        let mut results = Vec::new();

        if let Some(locations) = self.ingredient_index.get(&search_name) {
            for (section_name, idx) in locations {
                if let Some(items) = self.sections.get(section_name) {
                    if let Some(item) = items.get(*idx) {
                        results.push((section_name.as_str(), item));
                    }
                }
            }
        }
        results
    }

    /// Check if a cooklang recipe ingredient is available in the pantry
    ///
    /// This takes a cooklang Ingredient and checks if it's in the pantry.
    /// The search is case-insensitive and uses the ingredient's display name.
    #[cfg(feature = "aisle")]
    pub fn has_recipe_ingredient(&self, ingredient: &crate::Ingredient) -> bool {
        self.has_ingredient(&ingredient.name)
    }

    /// Get all items that are expired based on a given date
    ///
    /// Date should be in the same format as stored (e.g., "DD.MM.YYYY")
    pub fn expired_items(&self, current_date: &str) -> Vec<(&str, &PantryItem)> {
        let mut expired = Vec::new();

        for (section, items) in &self.sections {
            for item in items {
                if let Some(expire_date) = item.expire() {
                    // Simple string comparison - assumes dates are in comparable format
                    // For production, you'd want proper date parsing
                    if expire_date < current_date {
                        expired.push((section.as_str(), item));
                    }
                }
            }
        }
        expired
    }

    /// Get all items with quantities below a threshold
    ///
    /// This is a simple helper that returns items where quantity exists
    /// In a real implementation, you'd parse and compare quantities properly
    pub fn low_stock_items(&self) -> Vec<(&str, &PantryItem)> {
        let mut low_stock = Vec::new();

        for (section, items) in &self.sections {
            for item in items {
                // For now, just collect items with any quantity
                // In practice, you'd parse the quantity and check against thresholds
                if item.quantity().is_some() {
                    low_stock.push((section.as_str(), item));
                }
            }
        }
        low_stock
    }
}

/// Core parsing logic that can either return errors or collect warnings
fn parse_core(
    input: &str,
    lenient: bool,
    mut report: Option<&mut SourceReport>,
) -> Result<PantryConf, PantryConfError> {
    // Parse as generic TOML value first
    let toml_value: toml::Value = toml::from_str(input).map_err(|e| PantryConfError::Parse {
        message: format!("TOML parse error: {}", e),
    })?;

    let toml_table = toml_value
        .as_table()
        .ok_or_else(|| PantryConfError::Parse {
            message: "Expected TOML table at root".to_string(),
        })?;

    let mut sections = BTreeMap::new();
    let mut general_items = Vec::new(); // For top-level items

    for (section_name, section_value) in toml_table {
        let mut items = Vec::new();

        // A section can be:
        // 1. A string: "item_name"
        // 2. A table (for section tables [section_name]): process each key-value pair
        // 3. An array of items (strings or tables)

        match section_value {
            toml::Value::String(quantity) => {
                // Top-level string item: key is the name, value is the quantity
                // This should go into the "general" section
                general_items.push(PantryItem::WithAttributes(ItemWithAttributes {
                    name: section_name.clone(),
                    bought: None,
                    expire: None,
                    quantity: Some(quantity.clone()),
                    low: None,
                }));
                continue; // Skip to next item
            }
            toml::Value::Table(section_table) => {
                // This is a section table like [freezer]
                // Each key-value pair in the table is an item
                for (item_key, item_value) in section_table {
                    match item_value {
                        toml::Value::String(quantity) => {
                            // String value: key is the name, string is the quantity
                            items.push(PantryItem::WithAttributes(ItemWithAttributes {
                                name: item_key.clone(),
                                bought: None,
                                expire: None,
                                quantity: Some(quantity.clone()),
                                low: None,
                            }));
                        }
                        toml::Value::Table(attrs) => {
                            // Item with attributes: key is the name, table contains attributes
                            let mut item_table = attrs.clone();
                            // Parse the attributes table but use the key as the name
                            let bought = item_table.remove("bought").and_then(|val| {
                                if let toml::Value::String(s) = val {
                                    Some(s)
                                } else {
                                    None
                                }
                            });
                            let expire = item_table.remove("expire").and_then(|val| {
                                if let toml::Value::String(s) = val {
                                    Some(s)
                                } else {
                                    None
                                }
                            });
                            let quantity = item_table.remove("quantity").and_then(|val| {
                                if let toml::Value::String(s) = val {
                                    Some(s)
                                } else {
                                    None
                                }
                            });
                            let low = item_table.remove("low").and_then(|val| {
                                if let toml::Value::String(s) = val {
                                    Some(s)
                                } else {
                                    None
                                }
                            });

                            // Warn about unknown attributes
                            if !item_table.is_empty() && lenient {
                                if let Some(report) = report.as_mut() {
                                    for key in item_table.keys() {
                                        let warning = SourceDiag::warning(
                                            format!("Unknown field '{}' in item '{}'", key, item_key),
                                            (Span::new(0, 0), Some("valid attributes are: bought, expire, quantity, low".into())),
                                            Stage::Parse,
                                        );
                                        report.push(warning);
                                    }
                                }
                            }

                            items.push(PantryItem::WithAttributes(ItemWithAttributes {
                                name: item_key.clone(),
                                bought,
                                expire,
                                quantity,
                                low,
                            }));
                        }
                        _ => {
                            let msg = format!(
                                "Invalid value type for item '{}' in section '{}'",
                                item_key, section_name
                            );
                            if lenient {
                                if let Some(report) = report.as_mut() {
                                    let warning = SourceDiag::warning(
                                        msg.clone(),
                                        (Span::new(0, 0), Some("expected string or table".into())),
                                        Stage::Parse,
                                    );
                                    report.push(warning);
                                }
                            } else {
                                return Err(PantryConfError::Parse { message: msg });
                            }
                        }
                    }
                }
            }
            toml::Value::Array(array) => {
                // Array of items
                for (idx, item_value) in array.iter().enumerate() {
                    match item_value {
                        toml::Value::String(name) => {
                            items.push(PantryItem::Simple(name.clone()));
                        }
                        toml::Value::Table(table) => {
                            items.push(parse_item_from_table(
                                table.clone(),
                                section_name,
                                lenient,
                                report.as_deref_mut(),
                            )?);
                        }
                        _ => {
                            let msg = format!(
                                "Invalid item type at index {} in section '{}'",
                                idx, section_name
                            );
                            if lenient {
                                if let Some(report) = report.as_mut() {
                                    let warning = SourceDiag::warning(
                                        msg.clone(),
                                        (Span::new(0, 0), Some("expected string or table".into())),
                                        Stage::Parse,
                                    );
                                    report.push(warning);
                                }
                            } else {
                                return Err(PantryConfError::Parse { message: msg });
                            }
                        }
                    }
                }
            }
            _ => {
                let msg = format!("Invalid section type for '{}'", section_name);
                if lenient {
                    if let Some(report) = report.as_mut() {
                        let warning = SourceDiag::warning(
                            msg.clone(),
                            (
                                Span::new(0, 0),
                                Some("expected string, table, or array".into()),
                            ),
                            Stage::Parse,
                        );
                        report.push(warning);
                    }
                } else {
                    return Err(PantryConfError::Parse { message: msg });
                }
            }
        }

        if !items.is_empty() {
            sections.insert(section_name.clone(), items);
        }
    }

    // Add top-level items to "general" section if any exist
    if !general_items.is_empty() {
        sections.insert("general".to_string(), general_items);
    }

    // Build the index for fast lookups
    let mut ingredient_index = BTreeMap::new();
    for (section_name, items) in &sections {
        for (idx, item) in items.iter().enumerate() {
            let lowercase_name = item.name().to_lowercase();
            ingredient_index
                .entry(lowercase_name)
                .or_insert_with(Vec::new)
                .push((section_name.clone(), idx));
        }
    }

    Ok(PantryConf {
        sections,
        ingredient_index,
    })
}

fn parse_item_from_table(
    mut table: toml::map::Map<String, toml::Value>,
    section_name: &str,
    lenient: bool,
    mut report: Option<&mut SourceReport>,
) -> Result<PantryItem, PantryConfError> {
    // Extract known attributes first
    let bought = table.remove("bought").and_then(|val| {
        if let toml::Value::String(s) = val {
            Some(s)
        } else {
            None
        }
    });
    let expire = table.remove("expire").and_then(|val| {
        if let toml::Value::String(s) = val {
            Some(s)
        } else {
            None
        }
    });
    let quantity = table.remove("quantity").and_then(|val| {
        if let toml::Value::String(s) = val {
            Some(s)
        } else {
            None
        }
    });
    let low = table.remove("low").and_then(|val| {
        if let toml::Value::String(s) = val {
            Some(s)
        } else {
            None
        }
    });

    // Look for a "name" field
    let name = if let Some(val) = table.remove("name") {
        if let toml::Value::String(s) = val {
            Some(s)
        } else {
            None
        }
    } else {
        // If no "name" field, the first remaining string field's key is the name
        // This allows syntax like: { ice = "ice", ... } where "ice" is the name
        let mut found_name = None;
        let mut found_key = None;

        // Find the first string value - its key is the item name
        for (key, value) in table.iter() {
            if let toml::Value::String(_) = value {
                found_name = Some(key.clone());
                found_key = Some(key.clone());
                break;
            }
        }

        if let Some(key) = found_key {
            table.remove(&key);
        }
        found_name
    };

    let name = name.ok_or_else(|| PantryConfError::Parse {
        message: format!("Item in section '{}' missing name field", section_name),
    })?;

    // Warn about remaining fields if lenient
    if !table.is_empty() && lenient {
        if let Some(report) = report.as_mut() {
            for key in table.keys() {
                let warning = SourceDiag::warning(
                    format!("Unknown field '{}' in item '{}'", key, name),
                    (Span::new(0, 0), Some("item should have only one name field plus optional bought, expire, quantity, low".into())),
                    Stage::Parse,
                );
                report.push(warning);
            }
        }
    }

    Ok(PantryItem::WithAttributes(ItemWithAttributes {
        name,
        bought,
        expire,
        quantity,
        low,
    }))
}

/// Parse a [`PantryConf`] from TOML format
pub fn parse(input: &str) -> Result<PantryConf, PantryConfError> {
    parse_core(input, false, None)
}

/// Parse pantry configuration with lenient handling
///
/// This function returns a [`PassResult`] which includes both the parsed configuration
/// and any warnings that occurred during parsing.
///
/// # Examples
///
/// ```
/// let pantry_conf = r#"
/// [freezer]
/// cranberries = "500%g"
/// spinach = { bought = "05.05.2024", expire = "05.05.2025", quantity = "1%kg" }
/// "#;
///
/// let result = cooklang::pantry::parse_lenient(pantry_conf);
/// let (parsed, warnings) = result.into_result().unwrap();
///
/// assert_eq!(parsed.sections.len(), 1);
/// assert_eq!(parsed.sections["freezer"].len(), 2);
/// ```
pub fn parse_lenient(input: &str) -> PassResult<PantryConf> {
    let mut report = SourceReport::empty();

    match parse_core(input, true, Some(&mut report)) {
        Ok(conf) => PassResult::new(Some(conf), report),
        Err(e) => {
            // Convert error to diagnostic and add to report
            let diag = SourceDiag::error(e.to_string(), (Span::new(0, 0), None), Stage::Parse);
            report.push(diag);
            PassResult::new(None, report)
        }
    }
}

/// Write a [`PantryConf`] in TOML format
pub fn write(conf: &PantryConf, mut write: impl std::io::Write) -> std::io::Result<()> {
    let toml_string = toml::to_string_pretty(conf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    write.write_all(toml_string.as_bytes())
}

/// Error generated by [`parse`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PantryConfError {
    #[error("Error parsing input: {message}")]
    Parse { message: String },
}

impl RichError for PantryConfError {
    fn labels(&self) -> Cow<'_, [Label]> {
        use crate::error::label;
        match self {
            PantryConfError::Parse { .. } => vec![label!(Span::new(0, 0))],
        }
        .into()
    }

    fn hints(&self) -> Cow<'_, [CowStr]> {
        match self {
            PantryConfError::Parse { .. } => {
                vec!["Check TOML syntax".into()]
            }
        }
        .into()
    }

    fn severity(&self) -> crate::error::Severity {
        crate::error::Severity::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_pantry() {
        let input = r#"
[freezer]
cranberries = "500%g"
spinach = { bought = "05.05.2024", expire = "05.05.2025", quantity = "1%kg" }

[fridge]
milk = { expire = "10.05.2024", quantity = "1%L" }
"#;
        let p = parse(input).unwrap();
        assert_eq!(p.sections.len(), 2);

        let freezer = &p.sections["freezer"];
        assert_eq!(freezer.len(), 2);

        // Find items by name since order isn't guaranteed
        let cranberries = freezer.iter().find(|i| i.name() == "cranberries").unwrap();
        assert_eq!(cranberries.quantity(), Some("500%g"));
        assert!(cranberries.bought().is_none());
        assert!(cranberries.expire().is_none());

        let spinach = freezer.iter().find(|i| i.name() == "spinach").unwrap();
        assert_eq!(spinach.bought(), Some("05.05.2024"));
        assert_eq!(spinach.expire(), Some("05.05.2025"));
        assert_eq!(spinach.quantity(), Some("1%kg"));

        let fridge = &p.sections["fridge"];
        assert_eq!(fridge.len(), 1);
        let milk = fridge.iter().find(|i| i.name() == "milk").unwrap();
        assert_eq!(milk.bought(), None);
        assert_eq!(milk.expire(), Some("10.05.2024"));
        assert_eq!(milk.quantity(), Some("1%L"));
    }

    #[test]
    fn string_as_quantity() {
        let input = r#"
[pantry]
rice = "5%kg"
pasta = "1%kg"
flour = "2%kg"
"#;
        let p = parse(input).unwrap();
        assert_eq!(p.sections.len(), 1);

        let pantry = &p.sections["pantry"];
        assert_eq!(pantry.len(), 3);

        for item in pantry {
            assert!(item.quantity().is_some());
            assert!(item.bought().is_none());
            assert!(item.expire().is_none());
        }

        let rice = pantry.iter().find(|i| i.name() == "rice").unwrap();
        assert_eq!(rice.quantity(), Some("5%kg"));
    }

    #[test]
    fn simple_items() {
        let input = r#"
pantry = ["rice", "pasta", "flour"]
"#;
        let p = parse(input).unwrap();
        assert_eq!(p.sections.len(), 1);

        let pantry = &p.sections["pantry"];
        assert_eq!(pantry.len(), 3);
        assert_eq!(pantry[0].name(), "rice");
        assert_eq!(pantry[1].name(), "pasta");
        assert_eq!(pantry[2].name(), "flour");
    }

    #[test]
    fn mixed_items() {
        let input = r#"
cupboard = ["salt", { pepper = "pepper", bought = "01.01.2024" }, "sugar"]
"#;
        let p = parse(input).unwrap();
        assert_eq!(p.sections.len(), 1);

        let cupboard = &p.sections["cupboard"];
        assert_eq!(cupboard.len(), 3);
        assert_eq!(cupboard[0].name(), "salt");
        assert_eq!(cupboard[1].name(), "pepper");
        assert_eq!(cupboard[1].bought(), Some("01.01.2024"));
        assert_eq!(cupboard[2].name(), "sugar");
    }

    #[test]
    fn empty_file() {
        let p = parse("").unwrap();
        assert!(p.sections.is_empty());
    }

    #[test]
    fn empty_section() {
        let input = r#"
empty = []
"#;
        let p = parse(input).unwrap();
        // Empty sections are not added to the result
        assert_eq!(p.sections.len(), 0);
    }

    #[test]
    fn top_level_items() {
        // Test with top-level items (no section)
        // Top-level items should go into "general" section
        let input = r#"
paprika = "1%jar"
salt = "500%g"

[freezer]
spinach = "200%g"
"#;
        let p = parse(input).unwrap();

        // Should have "general" and "freezer" sections
        assert_eq!(p.sections.len(), 2);
        assert!(p.sections.contains_key("general"));
        assert!(p.sections.contains_key("freezer"));

        // The "general" section should contain paprika and salt
        let general_items = &p.sections["general"];
        assert_eq!(general_items.len(), 2);

        let paprika = general_items
            .iter()
            .find(|i| i.name() == "paprika")
            .unwrap();
        assert_eq!(paprika.quantity(), Some("1%jar"));

        let salt = general_items.iter().find(|i| i.name() == "salt").unwrap();
        assert_eq!(salt.quantity(), Some("500%g"));

        // The freezer section should contain spinach
        let freezer_items = &p.sections["freezer"];
        assert_eq!(freezer_items.len(), 1);
        assert_eq!(freezer_items[0].name(), "spinach");
        assert_eq!(freezer_items[0].quantity(), Some("200%g"));
    }

    #[test]
    fn optional_sections() {
        // Test with only one section
        let input = r#"
[freezer]
ice = "1%kg"
"#;
        let p = parse(input).unwrap();
        assert_eq!(p.sections.len(), 1);
        assert!(p.sections.contains_key("freezer"));
        assert!(!p.sections.contains_key("fridge"));
        assert!(!p.sections.contains_key("pantry"));

        // Test with no sections at all
        let input2 = "";
        let p2 = parse(input2).unwrap();
        assert_eq!(p2.sections.len(), 0);

        // Test with multiple sections, some empty
        let input3 = r#"
[freezer]

[pantry]
rice = "5%kg"
"#;
        let p3 = parse(input3).unwrap();
        assert_eq!(p3.sections.len(), 1); // Only pantry, since freezer is empty
        assert!(p3.sections.contains_key("pantry"));
        assert!(!p3.sections.contains_key("freezer")); // Empty section not included
    }

    #[test]
    fn items_by_section() {
        let input = r#"
freezer = ["ice cream", "frozen peas"]
fridge = ["milk", "cheese"]
"#;
        let p = parse(input).unwrap();
        let map = p.items_by_section();

        assert_eq!(map.get("ice cream"), Some(&"freezer"));
        assert_eq!(map.get("frozen peas"), Some(&"freezer"));
        assert_eq!(map.get("milk"), Some(&"fridge"));
        assert_eq!(map.get("cheese"), Some(&"fridge"));
    }

    #[test]
    fn parse_lenient_with_unknown_attrs() {
        let input = r#"
[freezer]
ice = { color = "white", texture = "solid" }
"#;
        let result = parse_lenient(input);
        let (parsed, warnings) = result.into_result().unwrap();

        // Should parse successfully with warnings about unknown attributes
        assert_eq!(parsed.sections.len(), 1);
        assert_eq!(parsed.sections["freezer"].len(), 1);
        assert_eq!(parsed.sections["freezer"][0].name(), "ice");

        // Should have warnings about unknown attributes
        assert!(warnings.has_warnings());
        let warning_count = warnings.iter().count();
        assert_eq!(warning_count, 2); // color and texture
    }

    #[test]
    fn test_has_ingredient() {
        let input = r#"
[freezer]
spinach = "1%kg"
ice_cream = "2%L"

[pantry]
Rice = "5%kg"
pasta = "1%kg"
"#;
        let p = parse(input).unwrap();

        // Case-insensitive search
        assert!(p.has_ingredient("spinach"));
        assert!(p.has_ingredient("Spinach"));
        assert!(p.has_ingredient("SPINACH"));
        assert!(p.has_ingredient("rice")); // Note: stored as "Rice"
        assert!(p.has_ingredient("RICE"));
        assert!(p.has_ingredient("pasta"));

        // Not found
        assert!(!p.has_ingredient("chicken"));
        assert!(!p.has_ingredient("milk"));
    }

    #[test]
    fn test_find_ingredient() {
        let input = r#"
[freezer]
spinach = { bought = "01.01.2024", expire = "01.02.2024", quantity = "1%kg" }

[pantry]
rice = "5%kg"
"#;
        let p = parse(input).unwrap();

        // Find spinach
        let result = p.find_ingredient("spinach");
        assert!(result.is_some());
        let (section, item) = result.unwrap();
        assert_eq!(section, "freezer");
        assert_eq!(item.name(), "spinach");
        assert_eq!(item.quantity(), Some("1%kg"));

        // Find rice (case-insensitive)
        let result = p.find_ingredient("RICE");
        assert!(result.is_some());
        let (section, item) = result.unwrap();
        assert_eq!(section, "pantry");
        assert_eq!(item.name(), "rice");
    }

    #[test]
    fn test_expired_items() {
        let input = r#"
[fridge]
milk = { expire = "10.01.2024", quantity = "1%L" }
cheese = { expire = "20.01.2024" }
yogurt = { expire = "05.01.2024" }

[pantry]
rice = "5%kg"
"#;
        let p = parse(input).unwrap();

        // Check items expired before 15.01.2024
        let expired = p.expired_items("15.01.2024");
        assert_eq!(expired.len(), 2); // milk and yogurt

        // Find the expired items
        let names: Vec<&str> = expired.iter().map(|(_, item)| item.name()).collect();
        assert!(names.contains(&"milk"));
        assert!(names.contains(&"yogurt"));
        assert!(!names.contains(&"cheese"));
    }

    #[test]
    fn test_index_performance() {
        // Create a large pantry to test performance
        let mut sections = BTreeMap::new();

        // Add 100 sections with 100 items each = 10,000 items
        for section_num in 0..100 {
            let section_name = format!("section_{}", section_num);
            let mut items = Vec::new();
            for item_num in 0..100 {
                items.push(PantryItem::WithAttributes(ItemWithAttributes {
                    name: format!("item_{}_{}", section_num, item_num),
                    bought: None,
                    expire: None,
                    quantity: Some("1%kg".to_string()),
                    low: None,
                }));
            }
            sections.insert(section_name, items);
        }

        // Build index
        let mut ingredient_index = BTreeMap::new();
        for (section_name, items) in &sections {
            for (idx, item) in items.iter().enumerate() {
                let lowercase_name = item.name().to_lowercase();
                ingredient_index
                    .entry(lowercase_name)
                    .or_insert_with(Vec::new)
                    .push((section_name.clone(), idx));
            }
        }

        let pantry = PantryConf {
            sections,
            ingredient_index,
        };

        // Test that lookups are fast (O(1))
        assert!(pantry.has_ingredient("item_50_50"));
        assert!(pantry.has_ingredient("ITEM_99_99")); // case insensitive
        assert!(!pantry.has_ingredient("nonexistent"));

        // Find specific item
        let result = pantry.find_ingredient("item_75_25");
        assert!(result.is_some());
        let (section, item) = result.unwrap();
        assert_eq!(section, "section_75");
        assert_eq!(item.name(), "item_75_25");
    }

    #[test]
    fn test_rebuild_index() {
        let mut pantry = PantryConf::default();

        // Manually add items
        pantry.sections.insert(
            "test".to_string(),
            vec![PantryItem::Simple("rice".to_string())],
        );

        // Index is empty since we didn't parse
        assert!(!pantry.has_ingredient("rice"));

        // Rebuild index
        pantry.rebuild_index();

        // Now it should find it
        assert!(pantry.has_ingredient("rice"));
        assert!(pantry.has_ingredient("RICE"));
    }

    #[test]
    fn roundtrip() {
        let input = r#"cupboard = [
    "salt",
    { name = "pepper", bought = "01.01.2024" },
    "sugar"
]

freezer = [
    "cranberries",
    { name = "spinach", bought = "05.05.2024", expire = "05.05.2025", quantity = "1%kg" }
]

fridge = [
    { name = "milk", expire = "10.05.2024", quantity = "1%L" }
]
"#;
        let p = parse(input).unwrap();
        let mut buffer = Vec::new();
        write(&p, &mut buffer).unwrap();
        let serialized = String::from_utf8(buffer).unwrap();
        let p2 = parse(&serialized).unwrap();
        assert_eq!(p, p2);
    }
}
