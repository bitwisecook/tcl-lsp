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

//! The self-tests of the source scanner and the manifest: modules, include macros, owners and duplicate rows.

use super::manifest::{read_manifest, target_identity, validate_manifest};
use super::source_scan::{
    check_source_inventory, collect_smoke_test_sources, collect_source_smoke_tests,
    inventory_entries, record_target_owner, record_target_source, scan_target_smoke_sources,
    source_include_macros, source_smoke_markers,
};
use super::{Fixture, ManifestRow, SmokeInventory, Target};
use anyhow::{Context, Result, bail};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

fn smoke_module_scanner_self_test() -> Result<()> {
    let module_fixture = Fixture::new()?;
    module_fixture.write("src/lib.rs", "pub mod smoke;\n")?;
    module_fixture.write("src/smoke.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &module_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([module_fixture.root.join("src/smoke.rs")]) {
        bail!("out-of-line smoke module scanner self-test failed");
    }
    let conditional_fixture = Fixture::new()?;
    conditional_fixture.write(
        "src/lib.rs",
        "#[cfg_attr(unix, path = \"platform_tests.rs\")]\n#[cfg_attr(windows, path = \"windows_tests.rs\")]\npub mod smoke;\n",
    )?;
    conditional_fixture.write("src/platform_tests.rs", "#[test]\nfn ordinary() {}\n")?;
    conditional_fixture.write("src/windows_tests.rs", "#[test]\nfn ordinary() {}\n")?;
    conditional_fixture.write("src/smoke.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &conditional_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found
        != BTreeSet::from([
            conditional_fixture.root.join("src/platform_tests.rs"),
            conditional_fixture.root.join("src/smoke.rs"),
            conditional_fixture.root.join("src/windows_tests.rs"),
        ])
    {
        bail!("conditionally attributed module-path scanner self-test failed");
    }
    let relative_fixture = Fixture::new()?;
    relative_fixture.write("src/lib.rs", "#[path = \"../smoke_tests.rs\"] mod smoke;\n")?;
    relative_fixture.write("smoke_tests.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &relative_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([relative_fixture.root.join("smoke_tests.rs")]) {
        bail!("path-attributed source outside crate directory self-test failed");
    }
    let logical_fixture = Fixture::new()?;
    logical_fixture.write("src/lib.rs", "#[path = \"alt.rs\"]\nmod smoke;\n")?;
    logical_fixture.write("src/alt.rs", "mod inner;\n")?;
    logical_fixture.write("src/inner.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &logical_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([logical_fixture.root.join("src/inner.rs")]) {
        bail!("path-attributed module logical-directory self-test failed");
    }
    nested_path_module_scanner_self_test()?;
    literal_include_scanner_self_test()?;
    inline_smoke_module_include_self_test()?;
    macro_rules_smoke_module_include_self_test()?;
    Ok(())
}

pub(super) fn inline_smoke_module_include_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.write(
        "src/lib.rs",
        "std::include!(\"outside.rs\");\nmod smoke { helper::include!(\"helper.rs\"); std::include!(\"inside.rs\"); core::include!(\"core_inside.rs\"); }\n",
    )?;
    fixture.write("src/outside.rs", "#[test]\nfn ordinary() {}\n")?;
    fixture.write("src/helper.rs", "#[test]\nfn ordinary() {}\n")?;
    fixture.write("src/inside.rs", "#[test]\nfn ordinary() {}\n")?;
    fixture.write("src/core_inside.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found
        != BTreeSet::from([
            fixture.root.join("src/core_inside.rs"),
            fixture.root.join("src/inside.rs"),
        ])
    {
        bail!("inline smoke-module include context self-test failed: {found:?}");
    }
    Ok(())
}

pub(super) fn macro_rules_smoke_module_include_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.write(
        "src/lib.rs",
        "macro_rules! generated { () => { include!(\"inside.rs\"); helper::include!(\"helper.rs\"); }; } mod smoke { generated!(); }\n",
    )?;
    fixture.write("src/inside.rs", "#[test]\nfn ordinary() {}\n")?;
    fixture.write("src/helper.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    if collect_source_smoke_tests(
        &fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )
    .is_ok()
    {
        bail!("unclassified macro-body include was not rejected");
    }

    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-smoke-target\nmacro_rules! generated { () => { include!(\"inside.rs\"); helper::include!(\"helper.rs\"); }; } mod smoke { generated!(); }\n",
    )?;
    found.clear();
    collect_source_smoke_tests(
        &fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([fixture.root.join("src/lib.rs")]) {
        bail!("smoke-classified macro-body include self-test failed: {found:?}");
    }

    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-no-smoke-include\nmacro_rules! generated { () => { include!(\"inside.rs\"); helper::include!(\"helper.rs\"); }; } mod smoke { generated!(); }\n",
    )?;
    found.clear();
    collect_source_smoke_tests(
        &fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if !found.is_empty() {
        bail!("no-smoke macro-body include selected a source: {found:?}");
    }

    fixture.write(
        "src/lib.rs",
        "macro_rules! top_level { () => { include!(\"inside.rs\"); }; } top_level!();\n",
    )?;
    if collect_source_smoke_tests(
        &fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut BTreeSet::new(),
    )
    .is_ok()
    {
        bail!("top-level macro invocation escaped classification");
    }

    fixture.write(
        "src/lib.rs",
        "macro_rules! unused { () => { include!(\"inside.rs\"); }; }\n",
    )?;
    if collect_source_smoke_tests(
        &fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut BTreeSet::new(),
    )
    .is_ok()
    {
        bail!("unused macro-body include escaped classification");
    }

    fixture.write(
        "src/lib.rs",
        "macro_rules! helper_only { () => { helper::include!(\"helper.rs\"); }; } helper_only!();\n",
    )?;
    found.clear();
    collect_source_smoke_tests(
        &fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if !found.is_empty() {
        bail!("custom namespaced include was traversed: {found:?}");
    }
    Ok(())
}

fn nested_path_module_scanner_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.write("src/lib.rs", "#[path = \"sub/alt.rs\"]\nmod smoke;\n")?;
    fixture.write("src/sub/alt.rs", "mod inner;\n")?;
    fixture.write("src/sub/inner.rs", "#[test]\nfn ordinary() {}\n")?;
    fixture.write(
        "src/inner.rs",
        "#[test]\nfn wrong_directory_must_not_be_selected() {}\n",
    )?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([fixture.root.join("src/sub/inner.rs")]) {
        bail!("nested path-attributed module directory self-test failed");
    }
    let inline_fixture = Fixture::new()?;
    inline_fixture.write(
        "src/lib.rs",
        "#[path = \"alt\"]\nmod smoke { mod inner; }\n",
    )?;
    inline_fixture.write("src/alt/inner.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &inline_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([inline_fixture.root.join("src/alt/inner.rs")]) {
        bail!("path-attributed inline-module directory self-test failed");
    }
    Ok(())
}

fn literal_include_scanner_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.write("src/lib.rs", "include!(\"sub/generated_tests.rs\");\n")?;
    fixture.write("src/main.rs", "fn main() {}\n")?;
    fixture.write(
        "src/sub/generated_tests.rs",
        "// tcl-lsp-smoke-target\n#[test]\nfn smoke_generated() {}\nmod smoke;\n",
    )?;
    fixture.write(
        "src/sub/smoke.rs",
        "#[test]\nfn ordinary_in_real_module() {}\n",
    )?;
    fixture.write(
        "src/smoke.rs",
        "#[test]\nfn ordinary_in_wrong_module() {}\n",
    )?;
    let targets = [
        Target {
            package_id: "included".to_owned(),
            package: "included".to_owned(),
            kind: "lib".to_owned(),
            name: "included".to_owned(),
            source: fixture.root.join("src/lib.rs"),
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        },
        Target {
            package_id: "included".to_owned(),
            package: "included".to_owned(),
            kind: "bin".to_owned(),
            name: "included".to_owned(),
            source: fixture.root.join("src/main.rs"),
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        },
    ];
    let mut inventory = SmokeInventory::default();
    inventory
        .sources
        .insert("src/sub/generated_tests.rs".to_owned());
    for target in &targets {
        scan_target_smoke_sources(&fixture.root, target, &mut inventory)?;
    }
    let included = inventory_entries(&fixture.root, &inventory, &targets);
    let expected = BTreeSet::from([
        (
            "src/sub/generated_tests.rs".to_owned(),
            target_identity(&targets[0]),
        ),
        ("src/sub/smoke.rs".to_owned(), target_identity(&targets[0])),
    ]);
    if included != expected {
        bail!("literal include target ownership self-test failed: {included:?}");
    }

    let macro_fixture = Fixture::new()?;
    macro_fixture.write(
        "src/lib.rs",
        "// tcl-lsp-smoke-target\nmacro_rules! generated { () => { include!(\"sub/generated_tests.rs\"); }; }\ngenerated!();\n",
    )?;
    macro_fixture.write(
        "src/sub/generated_tests.rs",
        "#[test]\nfn smoke_generated_in_macro() {}\n",
    )?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &macro_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([macro_fixture.root.join("src/lib.rs")]) {
        bail!("macro-body literal include self-test failed: {found:?}");
    }
    Ok(())
}

pub(super) fn smoke_scanner_self_test() -> Result<()> {
    for source in [
        "mod smoke { #[test] fn ordinary() {} }",
        "pub(crate) mod smoke_tests { #[test] fn ordinary() {} }",
        "#[test] extern \"C\" fn smoke_probe() {}",
        "#[test] pub unsafe extern \"C-unwind\" fn smoke_hook() {}",
        "#[test] pub(crate) const unsafe extern fn smoke_qualified() {}",
        "#[test]\npub(crate)\nunsafe\nextern \"C\"\nfn smoke_split() {}",
        "#[test]\nfn r#smoke_raw() {}",
        "#[cfg_attr(unix, test)]\nfn smoke_conditional() {}",
        "#[cfg_attr(unix, cfg_attr(unix, test))]\nfn smoke_nested_conditional() {}",
    ] {
        let parsed = syn::parse_file(source).context("parsing smoke scanner self-test")?;
        let fake_source = Path::new("/repo/src/lib.rs");
        let mut found = BTreeSet::new();
        collect_smoke_test_sources(
            fake_source,
            Path::new("/repo/src"),
            &parsed.items,
            false,
            &mut HashSet::new(),
            &mut found,
        )?;
        if found != BTreeSet::from([fake_source.to_path_buf()]) {
            bail!("smoke declaration scanner missed: {source}");
        }
    }
    for source in [
        "fn smoke_detector() {}",
        "mod smoke { fn ordinary() {} }",
        "#[test] fn ordinary() {}",
    ] {
        let parsed = syn::parse_file(source).context("parsing smoke scanner negative self-test")?;
        let mut found = BTreeSet::new();
        collect_smoke_test_sources(
            Path::new("/repo/src/lib.rs"),
            Path::new("/repo/src"),
            &parsed.items,
            false,
            &mut HashSet::new(),
            &mut found,
        )?;
        if !found.is_empty() {
            bail!("smoke declaration scanner false positive: {source}");
        }
    }
    smoke_module_scanner_self_test()?;
    namespaced_include_scanner_self_test()?;
    if !source_smoke_markers("// tcl-lsp-smoke-target")?.target
        || source_smoke_markers("fn tcl_lsp_smoke() {}")?.target
        || source_smoke_markers("const TEXT: &str = r#\"\n// tcl-lsp-smoke-target\n\"#;\n")?.target
        || source_smoke_markers("/*\n// tcl-lsp-smoke-target\n*/\n")?.target
        || source_smoke_markers("// tcl-lsp-smoke-target-extra").is_ok()
        || source_smoke_markers("let value = 1; // tcl-lsp-smoke-target").is_ok()
        || source_smoke_markers("// tcl-lsp-smoke-target\n// tcl-lsp-no-smoke-include\n").is_ok()
    {
        bail!("smoke marker scanner self-test failed");
    }
    include_macro_scanner_self_test()
}

fn include_macro_scanner_self_test() -> Result<()> {
    let ignored = source_include_macros(
        "const TEXT: &str = r#\"include!(concat!(env!(\\\"OUT_DIR\\\"), \\\"/tests.rs\\\"))\"#;\n/* include!(concat!(env!(\"OUT_DIR\"), \"/tests.rs\")); */\n",
    )?;
    if !ignored.literal_paths.is_empty() || ignored.non_literal_count != 0 {
        bail!("include scanner inspected a string or block comment");
    }
    let fixture = Fixture::new()?;
    let source = fixture.root.join("src/lib.rs");
    fixture.write(
        "src/lib.rs",
        "include!(concat!(env!(\"OUT_DIR\"), \"/tests.rs\"));\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("non-literal include without a smoke marker was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "macro_rules! generated { () => { include!(concat!(env!(\"OUT_DIR\"), \"/tests.rs\")); }; }\ngenerated!();\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("macro-body non-literal include without a marker was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "const DATA: &str = include!(concat!(env!(\"OUT_DIR\"), \"/data.rs\"));\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("expression-position non-literal include without a marker was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-no-smoke-include\ninclude!(concat!(env!(\"OUT_DIR\"), \"/data.rs\"));\n",
    )?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut found)?;
    if !found.is_empty() {
        bail!("non-smoke generated include marker selected a smoke source");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-no-smoke-include\ninclude!(concat!(env!(\"OUT_DIR\"), \"/one.rs\"));\ninclude!(concat!(env!(\"OUT_DIR\"), \"/two.rs\"));\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("one no-smoke marker classified multiple non-literal includes");
    }
    fixture.write("src/lib.rs", "// tcl-lsp-no-smoke-include\n")?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("stale no-smoke marker without an include was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-smoke-target\ninclude!(concat!(env!(\"OUT_DIR\"), \"/tests.rs\"));\ninclude!(concat!(env!(\"OUT_DIR\"), \"/more_tests.rs\"));\n",
    )?;
    found.clear();
    collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut found)?;
    if found != BTreeSet::from([source.clone()]) {
        bail!("marked non-literal include source was not selected");
    }
    fixture.write("src/lib.rs", "include!(\"generated_tests.rs\");\n")?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("missing literal include without a smoke marker was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-no-smoke-include\ninclude!(\"generated_tests.rs\");\n",
    )?;
    found.clear();
    collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut found)?;
    if !found.is_empty() {
        bail!("non-smoke missing literal include selected a smoke source");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-smoke-target\ninclude!(\"generated_tests.rs\");\n",
    )?;
    found.clear();
    collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut found)?;
    if found != BTreeSet::from([source.clone()]) {
        bail!("marked missing literal include source was not selected");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-no-smoke-include\ninclude!(\"one.rs\");\ninclude!(\"two.rs\");\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("one no-smoke marker classified multiple missing literal includes");
    }
    Ok(())
}

pub(super) fn namespaced_include_scanner_self_test() -> Result<()> {
    let namespaced = source_include_macros(
        "std::include!(\"std.rs\"); ::std::include!(\"absolute.rs\"); core::include!(\"core.rs\"); ::core::include!(\"absolute_core.rs\"); helper::include!(\"helper.rs\"); $crate::include!(\"crate.rs\"); helper::std::include!(\"nested.rs\"); helper::core::include!(\"nested_core.rs\");",
    )?;
    if namespaced.literal_paths != ["std.rs", "absolute.rs", "core.rs", "absolute_core.rs"] {
        bail!(
            "include scanner accepted the wrong namespace: {:?}",
            namespaced.literal_paths
        );
    }
    Ok(())
}

pub(super) fn multi_owner_inventory_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    let root = &fixture.root;
    fixture.write("shared.rs", "#[test]\nfn smoke_shared_source() {}\n")?;
    fixture.write("o/Cargo.toml", "")?;
    fixture.write("p/Cargo.toml", "")?;
    fixture.write(
        "smoke-targets.tsv",
        "shared.rs\to\tlib\tcommon\nshared.rs\tp\tlib\tcommon\n",
    )?;
    let source = root.join("shared.rs");
    let targets: Vec<Target> = ["o", "p"]
        .into_iter()
        .map(|package| Target {
            package_id: package.to_owned(),
            package: package.to_owned(),
            kind: "lib".to_owned(),
            name: "common".to_owned(),
            source: source.clone(),
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        })
        .collect();
    let mut inventory = SmokeInventory::default();
    record_target_source(&mut inventory, "shared.rs".to_owned(), &targets[0]);
    record_target_owner(&mut inventory, "shared.rs".to_owned(), &targets[1]);
    let rows: Vec<ManifestRow> = targets
        .iter()
        .map(|target| ManifestRow {
            source_text: "shared.rs".to_owned(),
            package: target.package.clone(),
            kind: target.kind.clone(),
            target_name: target.name.clone(),
        })
        .collect();
    if check_source_inventory(root, &rows[..1], &inventory, &targets).is_ok() {
        bail!("multi-owner source accepted a missing target row");
    }
    check_source_inventory(root, &rows, &inventory, &targets)
        .context("multi-owner source rejected complete target rows")?;
    let package_roots = HashMap::from([
        ("o".to_owned(), root.join("o")),
        ("p".to_owned(), root.join("p")),
    ]);
    let validated = validate_manifest(
        root,
        &root.join("smoke-targets.tsv"),
        &package_roots,
        &targets,
        &inventory,
    )?;
    if validated.len() != 2 {
        bail!("Cargo-owned source outside package directories was rejected");
    }

    fixture.write(
        "smoke-targets.tsv",
        "shared.rs\to\tlib\tcommon\nshared.rs\to\tbin\tcommon-bin\n",
    )?;
    let same_package_targets = vec![
        targets[0].clone(),
        Target {
            package_id: "o".to_owned(),
            package: "o".to_owned(),
            kind: "bin".to_owned(),
            name: "common-bin".to_owned(),
            source,
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        },
    ];
    let mut same_package_inventory = SmokeInventory::default();
    for target in &same_package_targets {
        record_target_source(&mut same_package_inventory, "shared.rs".to_owned(), target);
    }
    let validated = validate_manifest(
        root,
        &root.join("smoke-targets.tsv"),
        &package_roots,
        &same_package_targets,
        &same_package_inventory,
    )?;
    if validated.len() != 2 {
        bail!("same-package shared Cargo source rejected exact target rows");
    }
    Ok(())
}

pub(super) fn duplicate_manifest_row_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    let manifest = fixture.root.join("smoke-targets.tsv");
    fixture.write(
        "smoke-targets.tsv",
        "shared.rs\texample\tlib\texample\nshared.rs\texample\tlib\texample\n",
    )?;
    if read_manifest(&manifest).is_ok() {
        bail!("duplicate smoke target manifest row was accepted");
    }
    Ok(())
}
