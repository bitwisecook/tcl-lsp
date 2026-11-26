// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[test]
fn variable_inventory_guard_rejects_post_sort_and_unchecked_tables() {
    let body = "filter_ordered_names(ops.vars_in_bytes_checked(id)?, pattern)";
    assert!(super::variable_inventory_is_owned(
        body,
        "vars_in_bytes_checked("
    ));
    assert!(!super::variable_inventory_is_owned(
        "filter_ordered_names(ops.vars_in_bytes(id), pattern)",
        "vars_in_bytes_checked("
    ));
    assert!(!super::variable_inventory_is_owned(
        &format!("{body}; names.sort();"),
        "vars_in_bytes_checked("
    ));
    assert!(!super::variable_inventory_is_owned(
        &format!("{body}; names.dedup();"),
        "vars_in_bytes_checked("
    ));
}

#[test]
fn original_trim_guard_requires_selected_getter_order() {
    let original = "pub fn compiled_trim<O>() { let characters = ops.native_concat_string_bytes(characters)?; let bytes = ops.native_concat_string_bytes(subject)?; }";
    assert!(super::compiled_trim_operand_order_is_owned(original));
    let reversed = "pub fn compiled_trim<O>() { let bytes = ops.native_concat_string_bytes(subject)?; let characters = ops.native_concat_string_bytes(characters)?; }";
    assert!(!super::compiled_trim_operand_order_is_owned(reversed));
    assert!(!super::compiled_trim_operand_order_is_owned(
        "pub fn compiled_trim<O>() { ops.to_string(subject); }"
    ));
}

use super::*;

#[test]
fn parses_marked_manifest_rows() {
    let md = "<!-- owner-resolution-manifest -->\n| Surface | Owner source paths | Public entry points | Axis | Drift gate |\n| --- | --- | --- | --- | --- |\n| names | `rust/a/src/lib.rs`; `rust/b/src/lib.rs` | `one`; `two` | invariant | `xtask-example` |\n<!-- end-owner-resolution-manifest -->";
    let rows = parse_manifest(md).expect("manifest parses");
    assert_eq!(
        rows,
        vec![OwnerRow {
            surface: "names".to_owned(),
            source_paths: vec![
                "rust/a/src/lib.rs".to_owned(),
                "rust/b/src/lib.rs".to_owned()
            ],
            entry_points: vec!["one".to_owned(), "two".to_owned()],
            axis: "invariant".to_owned(),
            drift_gate: Some("xtask-example".to_owned()),
        }]
    );
}

fn internal_manifest_row(scope: &str) -> String {
    format!(
        "{INTERNAL_START}\n| entry | `rust/sample/src/storage.rs` | `sample` | `crate::storage` | `Ledger::revision` | {scope} | `rust/sample/src/reader.rs` | `crate::reader` | `Reader::synchronise` | `Reader::ledger` |\n{INTERNAL_END}"
    )
}

fn internal_fixture() -> (InternalOwnerRow, InternalModule, InternalModule) {
    let row = parse_internal_manifest(&internal_manifest_row("crate"))
        .expect("explicit crate row")
        .remove(0);
    let module = |source: &str| InternalModule {
        crate_root: "rust/sample".into(),
        crate_name: "sample".to_owned(),
        file: syn::parse_file(source).expect("fixture Rust"),
    };
    let owner = module(
        "pub struct Ledger; impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
    );
    let consumer = module(
        "use crate::storage::Ledger;
             struct Reader;
             impl Reader {
                 fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() }
                 fn synchronise(&self) { let revision = self.ledger().revision(); }
             }",
    );
    (row, owner, consumer)
}

#[test]
fn internal_owner_scopes_are_explicit_and_public_defaults_stay_strict() {
    let (row, owner, consumer) = internal_fixture();
    assert!(internal_entry_is_accessible(&row, &owner, &consumer));
    assert!(!file_declares(&owner.file, &row.entry));
    for scope in ["", "public", "private", "module", "unknown"] {
        assert!(parse_internal_manifest(&internal_manifest_row(scope)).is_err());
    }
    assert!(parse_internal_manifest(INTERNAL_START).is_err());
    assert!(parse_internal_manifest(INTERNAL_END).is_err());
    assert!(parse_internal_manifest(&format!("{INTERNAL_START}\n{INTERNAL_END}")).is_err());
}

#[test]
fn internal_owner_rejects_foreign_crates_and_private_sibling_methods() {
    let (row, mut owner, mut consumer) = internal_fixture();
    consumer.crate_root = "rust/foreign".into();
    assert!(!internal_entry_is_accessible(&row, &owner, &consumer));
    consumer.crate_root = owner.crate_root.clone();
    consumer.crate_name = "foreign".to_owned();
    assert!(!internal_entry_is_accessible(&row, &owner, &consumer));
    consumer.crate_name.clone_from(&owner.crate_name);
    owner.file = syn::parse_file(
        "pub struct Ledger; impl Ledger { fn revision(&self) -> Option<u64> { None } }",
    )
    .unwrap();
    assert!(!internal_entry_is_accessible(&row, &owner, &consumer));
    owner.file = syn::parse_file(
        "struct Ledger; impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
    )
    .unwrap();
    assert!(!internal_entry_is_accessible(&row, &owner, &consumer));
}

#[test]
fn internal_owner_requires_the_declared_typed_receiver_and_import_route() {
    let (mut row, owner, mut consumer) = internal_fixture();
    row.module = "crate::other".to_owned();
    assert!(!internal_entry_is_accessible(&row, &owner, &consumer));
    row.module = "crate::storage".to_owned();
    for source in [
        "use crate::storage::Other as Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; use crate::other::Other as Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Other> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> Alias<Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = other.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { /* self.ledger().revision() */ let revision = 0; } }",
    ] {
        consumer.file = syn::parse_file(source).expect("fixture Rust");
        assert!(
            !internal_entry_is_accessible(&row, &owner, &consumer),
            "{source}"
        );
    }
}

#[test]
fn internal_owner_rejects_conditionally_available_route_parts() {
    let (row, mut owner, mut consumer) = internal_fixture();
    for source in [
        "#![cfg(test)] pub struct Ledger; impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "#![cfg_attr(test, cfg(test))] pub struct Ledger; impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "#[cfg(test)] pub struct Ledger; impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "pub struct Ledger; #[cfg(test)] impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "pub struct Ledger; impl Ledger { #[cfg(test)] pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "#[cfg_attr(test, cfg(test))] pub struct Ledger; impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "pub struct Ledger; #[cfg_attr(test, cfg(test))] impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "pub struct Ledger; impl Ledger { #[cfg_attr(test, cfg(test))] pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "#[rewrite_route] pub struct Ledger; impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "pub struct Ledger; #[rewrite_route] impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
        "pub struct Ledger; impl Ledger { #[rewrite_route] pub(crate) fn revision(&self) -> Option<u64> { None } }",
    ] {
        owner.file = syn::parse_file(source).expect("fixed owner fixture");
        assert!(
            !internal_entry_is_accessible(&row, &owner, &consumer),
            "{source}"
        );
    }
    owner.file = syn::parse_file(
        "pub struct Ledger; impl Ledger { pub(crate) fn revision(&self) -> Option<u64> { None } }",
    )
    .unwrap();
    for source in [
        "#![cfg(test)] use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "#![cfg_attr(test, cfg(test))] use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "#[cfg(test)] use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; #[cfg(test)] struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; #[cfg(test)] impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { #[cfg(test)] fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } #[cfg(test)] fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { #[cfg(test)] let revision = self.ledger().revision(); } }",
        "#[cfg_attr(test, cfg(test))] use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; #[cfg_attr(test, cfg(test))] struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; #[cfg_attr(test, cfg(test))] impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { #[cfg_attr(test, cfg(test))] fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } #[cfg_attr(test, cfg(test))] fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { #[cfg_attr(test, cfg(test))] let revision = self.ledger().revision(); } }",
        "#[rewrite_route] use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; #[rewrite_route] struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; #[rewrite_route] impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { #[rewrite_route] fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } #[rewrite_route] fn synchronise(&self) { let revision = self.ledger().revision(); } }",
        "use crate::storage::Ledger; struct Reader; impl Reader { fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } fn synchronise(&self) { #[rewrite_route] let revision = self.ledger().revision(); } }",
    ] {
        consumer.file = syn::parse_file(source).expect("fixed consumer fixture");
        assert!(
            !internal_entry_is_accessible(&row, &owner, &consumer),
            "{source}"
        );
    }
    owner.file = syn::parse_file("#[doc = \"owner\"] #[derive(Debug)] pub struct Ledger; #[doc = \"members\"] impl Ledger { #[doc = \"revision\"] pub(crate) fn revision(&self) -> Option<u64> { None } }").unwrap();
    consumer.file = syn::parse_file("#[doc = \"import\"] use crate::storage::Ledger; #[derive(Debug)] struct Reader; #[doc = \"reader\"] impl Reader { #[doc = \"getter\"] fn ledger(&self) -> std::cell::Ref<'_, Ledger> { todo!() } #[doc = \"caller\"] fn synchronise(&self) { let revision = self.ledger().revision(); } }").unwrap();
    assert!(internal_entry_is_accessible(&row, &owner, &consumer));
}

#[test]
fn internal_owner_file_attributes_preserve_only_declarations() {
    for source in [
        "#![cfg(test)]",
        "#![cfg_attr(test, cfg(test))]",
        "#![cfg_attr(test, rewrite_route)]",
        "#![cfg_attr(test, allow(dead_code), cfg(test))]",
        r"#![recursion_limit(rewrite_route)]",
        r#"#![recursion_limit = include_str!("../limit")]"#,
        r#"#![recursion_limit = "unknown"]"#,
        r#"#![cfg_attr(test, recursion_limit = "256", cfg(test))]"#,
    ] {
        let file = syn::parse_file(source).expect("fixed file attributes");
        assert!(!file_attrs_preserve_routes(&file.attrs), "{source}");
    }
    let file = syn::parse_file(
            "#![doc = \"crate\"] #![deny(unused)] #![cfg_attr(target_arch = \"wasm32\", allow(dead_code, unused_imports))]",
        ).unwrap();
    assert!(file_attrs_preserve_routes(&file.attrs));
    for source in [
        r#"#![recursion_limit = "256"]"#,
        r#"#![cfg_attr(test, recursion_limit = "512", allow(dead_code))]"#,
    ] {
        let file = syn::parse_file(source).expect("literal expansion budget");
        assert!(file_attrs_preserve_routes(&file.attrs), "{source}");
    }
}

#[test]
fn internal_owner_current_namespace_revision_route_resolves() {
    let root = crate::util::repo_root();
    let contract = read(&root, CONTRACT_PATH).expect("owner contract");
    assert!(
        parse_internal_manifest(&contract)
            .unwrap()
            .iter()
            .any(|row| {
                row.source == "runtime/rust/src/namespace.rs"
                    && row.entry == "Namespaces::guard_lookup_environment_revision"
            })
    );
    assert_eq!(validate_internal_manifest(&root, &contract).len(), 0);
    assert!(read_internal_module(&root, "runtime/rust/src/namespace.rs", "crate::other").is_err());
    assert!(
        read_internal_module(
            &root,
            "runtime/rust/src/interp/stock_ensembles.rs",
            "crate::interp"
        )
        .is_err()
    );
}

#[test]
fn a_multi_line_dispatch_arm_still_counts() {
    let main = "        Command::SslictclData {\n            operation,\n        } => sslictcl_data::run(&operation),\n";
    assert!(xtask_dispatches(main, "sslictcl-data"));
    assert!(!xtask_dispatches(main, "owner-resolution"));
}

#[test]
fn the_editor_catalog_gate_resolves_to_its_dispatch_arm() {
    let main = "        Command::GenEditorCatalogs { check } => gen_editor_catalogs::run(check),\n";
    assert!(xtask_dispatches(main, "gen-editor-catalogs"));
    assert!(!xtask_dispatches(main, "gen-editor-extensions"));
}

#[test]
fn a_gate_alias_resolves_through_its_prerequisite() {
    let makefile = "xtask-alias: real-gate ## help text\n\nreal-gate:\n\tcd $(ROOT) && cargo xtask sslictcl-data check\n";
    assert_eq!(
        make_gate_command(makefile, "xtask-alias").as_deref(),
        Some("sslictcl-data"),
    );
    assert_eq!(
        make_gate_command(makefile, "real-gate").as_deref(),
        Some("sslictcl-data"),
    );
    assert_eq!(make_gate_command(makefile, "no-such-target"), None);
}

#[test]
fn the_backing_gate_resolves_through_its_own_binary() {
    let makefile =
        "xtask-command-backing:\n\tcd $(ROOT) && cargo xtask-backing command-backing --check\n";
    assert_eq!(
        make_gate_command(makefile, "xtask-command-backing").as_deref(),
        Some("command-backing"),
    );
    let root = crate::util::repo_root();
    let main = read(&root, XTASK_BACKING_MAIN_PATH).expect("xtask-backing main");
    assert!(xtask_dispatches(&main, "command-backing"));
}

#[test]
fn owner_headings_are_limited_to_owner_section() {
    let md = "## Owners\n### `tcl-syntax` — owner\n## Decision rules\n### `not-an-owner`";
    assert_eq!(
        owner_headings(md),
        BTreeSet::from(["tcl-syntax".to_owned()])
    );
}

#[test]
fn missing_declared_owner_row_is_reported() {
    let root = crate::util::repo_root();
    let contract = std::fs::read_to_string(root.join(CONTRACT_PATH)).expect("contract");
    let contract = contract
        .lines()
        .filter(|line| !(line.starts_with('|') && line.contains("`rust/tcl-syntax/src/glob.rs`")))
        .collect::<Vec<_>>()
        .join("\n");
    let makefile = std::fs::read_to_string(root.join(MAKEFILE_PATH)).expect("Makefile");
    let main = std::fs::read_to_string(root.join(XTASK_MAIN_PATH)).expect("xtask main");
    let problems = validate_manifest(&root, &contract, &makefile, &main);
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("tcl-syntax::glob"))
    );
}

#[test]
fn missing_source_is_reported() {
    let root = crate::util::repo_root();
    let contract = synthetic_manifest("`rust/tcl-syntax/src/moved.rs`", "`Moved`", "none");
    let makefile = std::fs::read_to_string(root.join(MAKEFILE_PATH)).expect("Makefile");
    let main = std::fs::read_to_string(root.join(XTASK_MAIN_PATH)).expect("xtask main");
    let problems = validate_manifest(&root, &contract, &makefile, &main);
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("missing or unreadable"))
    );
}

#[test]
fn missing_entry_point_is_reported() {
    let root = crate::util::repo_root();
    let contract = synthetic_manifest(
        "`rust/tcl-value-syntax/src/list.rs`",
        "`DefinitelyMissing`",
        "none",
    );
    let makefile = std::fs::read_to_string(root.join(MAKEFILE_PATH)).expect("Makefile");
    let main = std::fs::read_to_string(root.join(XTASK_MAIN_PATH)).expect("xtask main");
    let problems = validate_manifest(&root, &contract, &makefile, &main);
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("no public declaration"))
    );
}

#[test]
fn private_functions_do_not_satisfy_public_entry_points() {
    assert!(!entry_is_declared("fn hidden() {}", "hidden"));
    assert!(!entry_is_declared(
        "pub(crate) fn internal() {}",
        "internal"
    ));
    assert!(!entry_is_declared(
        "pub trait ValueOpsExtra {\n    fn dict_pairs_extra(&self);\n}",
        "ValueOps::dict_pairs"
    ));
    assert!(!entry_is_declared(
        "pub trait ValueOps {\n    fn dict_pairs_extra(&self);\n}",
        "ValueOps::dict_pairs"
    ));
    assert!(entry_is_declared(
        "pub trait Public {\n    fn member(&self);\n}",
        "Public::member"
    ));
}

#[test]
fn command_object_categories_reject_text_lookup_and_checked_snapshot_bypasses() {
    assert!(super::command_object_projection_is_owned(
        "vm.native_namespace_command_name(&original, true)"
    ));
    for bypass in [
        "resolve_command_bytes_checked(",
        "which_command_bytes_checked(",
        ".to_str()",
    ] {
        let source = format!("vm.native_namespace_command_name(&original, true); vm.{bypass}");
        assert!(!super::command_object_projection_is_owned(&source));
    }
    assert!(!super::command_object_projection_is_owned(
        "vm.origin_bytes_checked(bytes)"
    ));
}

#[test]
fn command_guard_category_requires_shared_manager_and_actual_owner() {
    let owned = "OwnedGuardManager<Allocation>; resolve_actual_owner(";
    assert!(super::command_guard_consumer_is_owned(
        owned,
        "OwnedGuardManager<Allocation>",
        "resolve_actual_owner("
    ));
    assert!(!super::command_guard_consumer_is_owned(
        "GuardManager; resolve_actual_owner(",
        "OwnedGuardManager<Allocation>",
        "resolve_actual_owner("
    ));
    assert!(!super::command_guard_consumer_is_owned(
        "OwnedGuardManager<Allocation>",
        "OwnedGuardManager<Allocation>",
        "resolve_actual_owner("
    ));
    assert!(!super::command_guard_consumer_is_owned(
        &format!("{owned}; HashMap<GuardToken, Allocation>"),
        "OwnedGuardManager<Allocation>",
        "resolve_actual_owner("
    ));
}

#[test]
fn handler_and_ensemble_categories_require_original_selected_owners() {
    let handler = "vm.lookup_original_command_at(context, &head)";
    assert!(super::original_handler_lookup_is_owned(handler));
    for bypass in ["lookup_command_bytes_checked(", ".to_str()"] {
        assert!(!super::original_handler_lookup_is_owned(&format!(
            "{handler}; {bypass}"
        )));
    }
    let list = "dialect.native_string_materialization(None); new_list_obj_native(items, protocol); native_object_bytes(list, protocol)";
    assert!(super::ensemble_byte_serialization_is_owned(list));
    for bypass in ["from_utf8_lossy(", "join_list(", "new_list_obj("] {
        assert!(!super::ensemble_byte_serialization_is_owned(&format!(
            "{list}; {bypass}"
        )));
    }
}

#[test]
fn associated_fields_and_variants_require_the_exact_public_owner() {
    let source = "pub enum Protocol { Selected }\n\
                      enum Private { Selected }\n\
                      pub struct Entry { pub location: u32, hidden: u32 }\n\
                      pub(crate) struct Internal { pub location: u32 }
\
                      impl Internal { pub fn getter(&self) {} }";
    assert!(entry_is_declared(source, "Protocol::Selected"));
    assert!(entry_is_declared(source, "Entry::location"));
    assert!(!entry_is_declared(source, "Private::Selected"));
    assert!(!entry_is_declared(source, "ProtocolExtra::Selected"));
    assert!(!entry_is_declared(source, "Entry::hidden"));
    assert!(!entry_is_declared(source, "Internal::location"));
    assert!(!entry_is_declared(source, "Internal::getter"));
}

#[test]
fn public_const_and_async_function_owners_are_recognised_without_prefix_collisions() {
    assert!(entry_is_declared(
        "pub const fn filesystem_cases() -> usize { 0 }",
        "filesystem_cases"
    ));
    assert!(entry_is_declared(
        "pub async fn resolve_async() {}",
        "resolve_async"
    ));
    assert!(!entry_is_declared(
        "pub const fn filesystem_cases_extra() -> usize { 0 }",
        "filesystem_cases"
    ));
    assert!(!entry_is_declared(
        "const fn filesystem_cases() -> usize { 0 }",
        "filesystem_cases"
    ));
}

#[test]
fn module_qualified_entries_require_the_actual_public_module() {
    let public = "pub mod bootstrap { pub struct Snapshot; pub fn snapshot() {} }";
    assert!(entry_is_declared(public, "bootstrap::Snapshot"));
    assert!(entry_is_declared(public, "bootstrap::snapshot"));
    assert!(!entry_is_declared(public, "unrelated::snapshot"));
    assert!(!entry_is_declared(
        "mod bootstrap { pub fn snapshot() {} }",
        "bootstrap::snapshot",
    ));
    assert!(!entry_is_declared(
        "pub mod bootstrap { fn snapshot() {} }",
        "bootstrap::snapshot",
    ));
}

#[test]
fn missing_make_gate_is_reported() {
    let root = crate::util::repo_root();
    let contract = synthetic_manifest(
        "`rust/tcl-value-syntax/src/list.rs`",
        "`split_list`",
        "`xtask-does-not-exist`",
    );
    let makefile = std::fs::read_to_string(root.join(MAKEFILE_PATH)).expect("Makefile");
    let main = std::fs::read_to_string(root.join(XTASK_MAIN_PATH)).expect("xtask main");
    let problems = validate_manifest(&root, &contract, &makefile, &main);
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("missing or non-xtask Makefile drift gate"))
    );
}

#[test]
fn legacy_naming_gate_resolves_the_tcl_evidence_check() {
    let root = crate::util::repo_root();
    let makefile = read(&root, MAKEFILE_PATH).expect("Makefile");
    let main = read(&root, XTASK_MAIN_PATH).expect("xtask main");
    assert!(naming_proof_gate_is_current(&root, &makefile));
    let contract = synthetic_manifest(
        "`rust/tcl-syntax/src/frame_level.rs`",
        "`parse_supplied_level`",
        "`check-name-resolution-proofs`",
    );
    assert_eq!(
        validate_manifest(&root, &contract, &makefile, &main),
        [] as [std::string::String; 0]
    );
    assert!(!naming_proof_gate_is_current(
        &root.join(MAKEFILE_PATH),
        &makefile
    ));
}

#[test]
fn legacy_naming_gate_rejects_missing_or_altered_evidence_checks() {
    let root = crate::util::repo_root();
    let actual = read(&root, MAKEFILE_PATH).expect("Makefile");
    for makefile in [
        String::new(),
        actual.replace(
            "check-name-resolution-proofs: check-tcl-evidence",
            "check-name-resolution-proofs:",
        ),
        actual.replace(
            "python3 scripts/dev/check-tcl-evidence.py",
            "python3 scripts/dev/other.py",
        ),
        actual.replace(
            "python3 -m unittest discover -s scripts/dev -p 'test_tcl_evidence*.py'",
            "true",
        ),
        format!("{actual}\ncheck-name-resolution-proofs: check-tcl-evidence\n"),
    ] {
        assert!(!naming_proof_gate_is_current(&root, &makefile));
    }
}

fn synthetic_manifest(paths: &str, entries: &str, gate: &str) -> String {
    format!(
        "## Owners\n### `tcl-syntax` — owner\n\n<!-- owner-resolution-manifest -->\n| Surface | Owner source paths | Public entry points | Dialect/release axis | Drift gate |\n| --- | --- | --- | --- | --- |\n| test | {paths} | {entries} | invariant | {gate} |\n<!-- end-owner-resolution-manifest -->\n\n## Decision rules"
    )
}

#[test]
fn resolves_make_targets_to_their_actual_xtask_commands() {
    let makefile = "xtask-resolution-drift:\n\tcd $(ROOT) && cargo xtask resolution-drift\n\nxtask-option-registry-drift:\n\tcd $(ROOT) && cargo xtask audit-option-dialects --check\n";
    assert_eq!(
        make_gate_command(makefile, "xtask-resolution-drift"),
        Some("resolution-drift".to_owned())
    );
    assert_eq!(
        make_gate_command(makefile, "xtask-option-registry-drift"),
        Some("audit-option-dialects".to_owned())
    );
}

#[test]
fn dispatch_check_ignores_comments_and_requires_match_arm() {
    assert!(!xtask_dispatches(
        "// Command::ResolutionDrift => resolution_drift::run(check),",
        "resolution-drift"
    ));
    assert!(xtask_dispatches(
        "Command::ResolutionDrift { check } => resolution_drift::run(check),",
        "resolution-drift"
    ));
}
#[test]
fn counted_name_facades_reject_local_recipes_and_keep_production_after_tests() {
    for source in [
        r#"fn helper(name:&str) { name.rsplit_once("::"); }"#,
        r#"fn helper(name:&str) { name.split(r"::"); }"#,
        r"fn helper(name:&[u8]) { name.split(|byte| *byte == 0); }",
        r"fn helper(name:&[u8]) { name.iter().position(|byte| *byte == b'\0'); }",
        r"fn helper(name:&[u8]) { String::from_utf8_lossy(name); }",
        r#"#[cfg(test)] mod tests { fn fixture(){ s.split("::"); } }
               fn later(name:&str){ name.split_once("::"); }"#,
    ] {
        assert!(!counted_name_local_recipes(source).is_empty(), "{source}");
    }
    let source = r##"
            // name.split("::");
            fn source_fixture(){ let fixture = r#"name.split("::")"#; }
            #[cfg(test)] mod tests { fn fixture(){ s.split("::"); } }
            fn production(){ original.matches_source(&image, config); }
        "##;
    assert_eq!(counted_name_local_recipes(source).len(), 0);
}

#[test]
fn name_delimiter_recipes_count_separator_methods_and_join_templates() {
    let source = r#"
            fn production(name: &str, ns: &str) -> String {
                let _ = name.split("::").count();
                let _ = name.as_bytes().starts_with(b"::");
                let _ = name.split(':').count();
                let _ = name.trim_start_matches(':');
                let _ = name.as_bytes().trim_ascii();
                let _ = name.contains("tcl::mathop");
                let _ = format!("no namespace named ::{ns}");
                format!("::{ns}::{name}")
            }
            // name.rfind("::");
            #[cfg(test)] mod tests { fn fixture(s: &str) { s.split("::"); } }
        "#;
    assert_eq!(name_delimiter_recipes(source), 4);
    for (template, joins) in [
        ("\"::{}\"", true),
        ("\"{}::{}\"", true),
        ("\"{ns}::{name}\"", true),
        ("\"{}\"", false),
        ("\"::\"", false),
        ("\"{}::{} is unknown\"", false),
    ] {
        assert_eq!(name_join_template(template), joins, "{template}");
    }
}

#[test]
fn name_delimiter_ledger_round_trips_and_skips_owner_and_tests() {
    let counts = std::collections::BTreeMap::from([
        ("rust/a/src/x.rs".to_owned(), 2),
        ("runtime/rust/src/y.rs".to_owned(), 1),
    ]);
    let ledger = render_name_delimiter_ledger(&counts);
    assert_eq!(parse_name_delimiter_ledger(&ledger), counts);
    for (path, _, text) in NAME_DELIMITER_OTHER_TEXT {
        assert!(ledger.contains(path) && ledger.contains(text), "{path}");
    }
    for (path, counted) in [
        ("rust/tcl-vm/src/exec.rs", true),
        ("rust/tcl-syntax/src/naming.rs", false),
        ("rust/tcl-syntax/src/naming/autoload.rs", false),
        ("rust/tcl-vm/tests/it_3/main.rs", false),
        ("rust/tcl-vm/src/exec_tests.rs", false),
        ("rust/xtask/src/owner_resolution.rs", false),
    ] {
        assert_eq!(counts_name_delimiters(path), counted, "{path}");
    }
}

#[test]
fn counted_name_facade_inventory_uses_shared_purpose_owners() {
    let root = crate::util::repo_root();
    assert_eq!(
        validate_counted_name_facade_consumers(&root).unwrap().len(),
        0
    );
}
