#[cfg(feature = "pantry")]
#[test]
fn test_pantry_integration() {
    use cooklang::pantry;
    
    let input = r#"
[freezer]
ice_cream = "1%L"
frozen_peas = "500%g"
spinach = { bought = "05.05.2024", expire = "05.06.2024", quantity = "1%kg" }

[fridge]
milk = { expire = "10.05.2024", quantity = "2%L" }
cheese = { expire = "15.05.2024" }

[pantry]
rice = "5%kg"
pasta = "1%kg"
flour = "5%kg"
"#;

    let pantry_conf = pantry::parse(input).unwrap();
    
    // Check sections
    assert_eq!(pantry_conf.sections.len(), 3);
    
    // Check freezer items (order not guaranteed due to HashMap)
    let freezer = &pantry_conf.sections["freezer"];
    assert_eq!(freezer.len(), 3);
    
    // Find items by name since order isn't guaranteed
    let ice_cream = freezer.iter().find(|i| i.name() == "ice_cream").unwrap();
    assert_eq!(ice_cream.quantity(), Some("1%L"));
    
    let frozen_peas = freezer.iter().find(|i| i.name() == "frozen_peas").unwrap();
    assert_eq!(frozen_peas.quantity(), Some("500%g"));
    
    let spinach = freezer.iter().find(|i| i.name() == "spinach").unwrap();
    assert_eq!(spinach.bought(), Some("05.05.2024"));
    assert_eq!(spinach.expire(), Some("05.06.2024"));
    assert_eq!(spinach.quantity(), Some("1%kg"));
    
    // Check fridge items
    let fridge = &pantry_conf.sections["fridge"];
    assert_eq!(fridge.len(), 2);
    
    let milk = fridge.iter().find(|i| i.name() == "milk").unwrap();
    assert_eq!(milk.expire(), Some("10.05.2024"));
    assert_eq!(milk.quantity(), Some("2%L"));
    
    let cheese = fridge.iter().find(|i| i.name() == "cheese").unwrap();
    assert_eq!(cheese.expire(), Some("15.05.2024"));
    assert!(cheese.quantity().is_none());
    
    // Check pantry items
    let pantry_section = &pantry_conf.sections["pantry"];
    assert_eq!(pantry_section.len(), 3);
    
    let rice = pantry_section.iter().find(|i| i.name() == "rice").unwrap();
    assert_eq!(rice.quantity(), Some("5%kg"));
    
    let pasta = pantry_section.iter().find(|i| i.name() == "pasta").unwrap();
    assert_eq!(pasta.quantity(), Some("1%kg"));
    
    let flour = pantry_section.iter().find(|i| i.name() == "flour").unwrap();
    assert_eq!(flour.quantity(), Some("5%kg"));
    
    // Test items_by_section method
    let items_map = pantry_conf.items_by_section();
    assert_eq!(items_map.get("ice_cream"), Some(&"freezer"));
    assert_eq!(items_map.get("milk"), Some(&"fridge"));
    assert_eq!(items_map.get("rice"), Some(&"pantry"));
    
    // Test all_items method
    let all_items: Vec<_> = pantry_conf.all_items().collect();
    assert_eq!(all_items.len(), 8);
}