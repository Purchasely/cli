#[path = "../src/inventory.rs"]
mod inventory;

use std::collections::BTreeSet;

#[test]
fn inventory_file_parses_and_has_unique_ids() {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../../contract/inventory.json")).unwrap();
    let parsed =
        inventory::Inventory::from_vitest(&serde_json::Value::Array(vec![]), "unused").unwrap();
    assert!(parsed.ids.is_empty());
    let ids = value["ids"].as_array().unwrap();
    let unique = ids.iter().collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), unique.len());
    assert_eq!(value["base"], "c656c8fbd341f1eb5b793d2d584789b372fb4824");
}

#[test]
fn unmapped_ids_are_counted_and_exclusions_are_mapped() {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../../contract/inventory.json")).unwrap();
    let inv: inventory::Inventory = serde_json::from_value(value).unwrap();
    let mapped = BTreeSet::from([inv.ids[0].clone()]);
    assert_eq!(inv.unmapped_count(&mapped), inv.ids.len() - 1);
}
