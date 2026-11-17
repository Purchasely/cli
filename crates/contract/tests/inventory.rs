//! The test-ID inventory (plan 2.4): every vitest ID of the base SHA is mapped to a case, to a Rust test
//! through a `unit` field, or to an exclusion with a reason.

use contract::inventory::{Inventory, Mapping, parse_test_list};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A vitest ID with `${...}` in its name is the template of a loop; the recorded cases use the expanded
/// names. The template is mapped when each expanded name of its table is a recorded case.
const TEMPLATES: &[(&str, &[&str])] = &[
    (
        // test/errors.test.ts: the 10 rows of `cases`.
        "errors::error mapping > ${name} exits ${code}",
        &[
            "errors::error mapping > UNAUTHORIZED exits 3",
            "errors::error mapping > 401 without code exits 3",
            "errors::error mapping > FORBIDDEN exits 4",
            "errors::error mapping > FEATURE_NOT_ENABLED exits 4",
            "errors::error mapping > VALIDATION_FAILED exits 5",
            "errors::error mapping > other 422 exits 5",
            "errors::error mapping > NOT_FOUND exits 6",
            "errors::error mapping > 404 without code exits 6",
            "errors::error mapping > 500 exits 1",
            "errors::error mapping > unknown code exits 1",
        ],
    ),
    (
        // test/update.test.ts: the 4 rows of `cases` under "opt-outs".
        "update::opt-outs > ${name} records no exec and writes no state",
        &[
            "update::opt-outs > CI records no exec and writes no state",
            "update::opt-outs > PURCHASELY_DISABLE_AUTOUPDATE records no exec and writes no state",
            "update::opt-outs > PURCHASELY_OFFLINE records no exec and writes no state",
            "update::opt-outs > config auto_update false records no exec and writes no state",
        ],
    ),
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn inventory() -> Inventory {
    let text = std::fs::read_to_string(repo().join("contract/inventory.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn case_files() -> Vec<Value> {
    let mut files = Vec::new();
    for dir in ["contract/cases", "contract/bin"] {
        let mut paths = std::fs::read_dir(repo().join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            files.push(value);
        }
    }
    files
}

/// The mapped IDs: the case names, plus each template whose expanded rows are all recorded.
fn mapped(mapping: &Mapping) -> BTreeSet<String> {
    let mut mapped = mapping.tests.clone();
    for (template, rows) in TEMPLATES {
        if rows.iter().all(|row| mapping.recorded.contains(*row)) {
            mapped.insert((*template).to_owned());
        }
    }
    mapped
}

/// The file that the CI step writes before `cargo test --workspace`. `CONTRACT_TEST_LIST` overrides it;
/// the ignored list is the same path with `-ignored` before the extension.
fn read_list(ignored: bool) -> BTreeSet<String> {
    let path = std::env::var_os("CONTRACT_TEST_LIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo().join("target/test-list.txt"));
    let path = if ignored {
        path.with_file_name(format!(
            "{}-ignored.txt",
            path.file_stem().unwrap().to_string_lossy()
        ))
    } else {
        path
    };
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e}. Write it first: cargo test --workspace -- --list{} > {} 2>&1",
            path.display(),
            if ignored { " --ignored" } else { "" },
            path.display()
        )
    });
    parse_test_list(&text)
}

#[test]
fn inventory_file_parses_and_has_unique_ids() {
    let inv = inventory();
    let unique = inv.ids.iter().collect::<BTreeSet<_>>();
    assert_eq!(inv.ids.len(), unique.len());
    assert_eq!(inv.base, "c656c8fbd341f1eb5b793d2d584789b372fb4824");
    for exclusion in &inv.exclusions {
        assert!(inv.ids.contains(&exclusion.id), "{}", exclusion.id);
        assert!(!exclusion.reason.trim().is_empty(), "{}", exclusion.id);
    }
}

#[test]
fn each_template_row_is_a_recorded_case() {
    let inv = inventory();
    let mapping = Mapping::from_files(&case_files()).unwrap();
    for (template, rows) in TEMPLATES {
        assert!(inv.ids.iter().any(|id| id == template), "{template}");
        for row in *rows {
            assert!(mapping.recorded.contains(*row), "{template}: no case {row}");
        }
    }
}

#[test]
fn each_unit_is_a_listed_test_that_is_not_ignored() {
    let mapping = Mapping::from_files(&case_files()).unwrap();
    let (listed, ignored) = (read_list(false), read_list(true));
    assert!(!listed.is_empty(), "the test list is empty");
    let bad = mapping
        .units
        .iter()
        .filter_map(|(test, unit)| {
            Inventory::validate_unit(unit, &listed, &ignored)
                .err()
                .map(|e| format!("{test}: {e}"))
        })
        .collect::<Vec<_>>();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn no_id_is_unmapped() {
    let inv = inventory();
    let mapping = Mapping::from_files(&case_files()).unwrap();
    let unmapped = inv.unmapped(&mapped(&mapping));
    println!(
        "inventory: {} unmapped of {}",
        unmapped.len(),
        inv.ids.len()
    );
    for id in &unmapped {
        println!("unmapped: {id}");
    }
    // The cut-over (P13) deletes the TS tests: every ID needs its case, its Rust test or its exclusion.
    assert!(
        unmapped.is_empty(),
        "{} unmapped inventory IDs",
        unmapped.len()
    );
}
