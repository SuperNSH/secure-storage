//! The docs name every platform and what it survives.

use std::path::Path;

#[test]
fn docs_platform_table_covers_every_platform() {
    let decisions = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/decisions.md");
    let text = std::fs::read_to_string(&decisions).expect("docs/decisions.md is readable");
    let section = text
        .split("## What each platform protects, and what survives")
        .nth(1)
        .expect("the platform table has its section");
    for platform in ["Apple", "Android", "Windows", "Linux", "Web"] {
        let row = format!("| {platform} ");
        assert!(
            section.contains(&row),
            "the table has no row for {platform}"
        );
    }
    assert!(
        section.contains("Survives uninstall"),
        "the table names what survives uninstall"
    );
}
