//! Port of the Python pytest suite `tests/test_bigip_irules_refs.py`
//! ("parser-backed iRules object reference extraction").
//!
//! Each `#[test]` mirrors one pytest function, asserting the same extracted
//! BIG-IP object references (command + name, and that `kinds` is exposed) from
//! the same iRule bodies. Structural — asserts directly against the
//! `tcl_irules` public API, no `tclsh`.
//!
//! Rust API used:
//!   - `tcl_irules::extract_irules_object_references(source, rule_module, registry)`
//!   - `tcl_irules::IrulesObjectReference { command, name, kinds, .. }`
//!
//! These correspond to the Python `extract_irules_object_references(source)`
//! returning objects with `.command`, `.name`, and `.kinds`.

use tcl_irules::{IrulesObjectReference, extract_irules_object_references};
use tcl_registry::CommandRegistry;

/// Extract object references against an iRules-aware registry, the way the
/// Python `extract_irules_object_references(source)` does (it resolves `pool` /
/// `snatpool` / `class` as BIG-IP dialect commands). `rule_module = None` so
/// every LTM/GTM reference is recognised.
fn refs(source: &str) -> Vec<IrulesObjectReference> {
    let mut registry = CommandRegistry::build_default();
    registry.load_irules();
    extract_irules_object_references(source, None, &registry)
}

/// Build the Python `by_name = {(ref.command, ref.name): ref.kinds ...}`
/// mapping. We keep the `kinds` value to mirror the pytest exactly (it reads
/// `ref.kinds`), even though the assertions check key membership.
fn by_command_and_name(source: &str) -> Vec<((String, String), Vec<&'static str>)> {
    refs(source)
        .into_iter()
        .map(|r| ((r.command, r.name), r.kinds))
        .collect()
}

fn has_command_name(
    by_name: &[((String, String), Vec<&'static str>)],
    command: &str,
    name: &str,
) -> bool {
    by_name.iter().any(|((c, n), _)| c == command && n == name)
}

/// pytest: `test_extract_irules_refs_for_pool_snatpool_and_datagroup`
#[test]
fn extract_irules_refs_for_pool_snatpool_and_datagroup() {
    let source = "\n\
        when HTTP_REQUEST {\n\
        \x20   if {[class match -- [HTTP::host] equals /Common/host_dg]} {\n\
        \x20       snatpool /Common/sp1\n\
        \x20       pool /Common/web_pool\n\
        \x20   }\n\
        }\n";

    let by_name = by_command_and_name(source);

    // assert ("class", "/Common/host_dg") in by_name
    assert!(
        has_command_name(&by_name, "class", "/Common/host_dg"),
        "{by_name:?}"
    );
    // assert ("snatpool", "/Common/sp1") in by_name
    assert!(
        has_command_name(&by_name, "snatpool", "/Common/sp1"),
        "{by_name:?}"
    );
    // assert ("pool", "/Common/web_pool") in by_name
    assert!(
        has_command_name(&by_name, "pool", "/Common/web_pool"),
        "{by_name:?}"
    );
}

/// pytest: `test_extract_irules_refs_nested_in_body_and_command_substitution`
#[test]
fn extract_irules_refs_nested_in_body_and_command_substitution() {
    let source = "\n\
        when HTTP_REQUEST {\n\
        \x20   set count [active_members /Common/app_pool]\n\
        \x20   LB::reselect pool /Common/fallback_pool\n\
        }\n";

    let names: Vec<String> = refs(source).into_iter().map(|r| r.name).collect();

    // assert "/Common/app_pool" in names
    assert!(names.contains(&"/Common/app_pool".to_owned()), "{names:?}");
    // assert "/Common/fallback_pool" in names
    assert!(
        names.contains(&"/Common/fallback_pool".to_owned()),
        "{names:?}"
    );
}
