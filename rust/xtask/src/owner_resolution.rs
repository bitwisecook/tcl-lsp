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

//! The source-of-truth check for the shared semantic-owner contract.
//!
//! The contract deliberately owns this manifest: a Rust table here would be
//! easy to update while leaving the human-facing map stale. The check parses
//! the small, marked Markdown table, then verifies every source path, entry
//! point, and named Makefile gate. It also makes sure the existing owner
//! headings are represented by a manifest row.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};

mod source_internal;

mod command_consumers;
mod drift_gates;
mod entry_declarations;
mod internal_manifest;
mod interpreter_consumers;
mod manifest;
mod name_delimiters;
mod variable_consumers;

#[cfg(test)]
use command_consumers::{
    command_guard_consumer_is_owned, command_object_projection_is_owned,
    ensemble_byte_serialization_is_owned, original_handler_lookup_is_owned,
};
use command_consumers::{
    validate_command_guard_consumers, validate_command_object_consumers,
    validate_namespace_object_consumers,
};
use drift_gates::{make_gate_command, naming_proof_gate_is_current, xtask_dispatches};
#[cfg(test)]
use entry_declarations::entry_is_declared;
use entry_declarations::{file_declares, use_declares};
#[cfg(test)]
use internal_manifest::{InternalModule, internal_entry_is_accessible, read_internal_module};
use internal_manifest::{
    file_attrs_preserve_routes, route_attrs_are_plain, type_attrs_are_plain,
    validate_internal_manifest,
};
#[cfg(test)]
use interpreter_consumers::compiled_trim_operand_order_is_owned;
use interpreter_consumers::{
    validate_execution_name_policy_consumers, validate_f5_string_predicate_consumers,
    validate_jim_original_lookup_consumers, validate_native_bootstrap_consumers,
    validate_native_compiler_pass_consumers, validate_native_coroutine_consumers,
    validate_native_instruction_name_consumers, validate_native_list_storage_consumers,
    validate_native_string_trim_consumers, validate_package_consumers,
    validate_private_error_header_consumers,
};
#[cfg(test)]
use manifest::parse_internal_manifest;
use manifest::{
    declared_owner_bullets, owner_headings, parse_manifest, single_code_token, split_row,
};
#[cfg(test)]
use name_delimiters::{
    NAME_DELIMITER_OTHER_TEXT, counted_name_local_recipes, counts_name_delimiters,
    name_delimiter_recipes, name_join_template, parse_name_delimiter_ledger,
    render_name_delimiter_ledger,
};
use name_delimiters::{validate_counted_name_facade_consumers, validate_workspace_name_delimiters};
#[cfg(test)]
use variable_consumers::variable_inventory_is_owned;
use variable_consumers::{
    validate_array_search_consumers, validate_native_variable_callback_consumers,
    validate_variable_command_grammars, validate_variable_table_consumers,
};

const CONTRACT_PATH: &str = "docs/design/contracts/shared-utility-contracts-rust.md";
const MAKEFILE_PATH: &str = "Makefile";
const XTASK_MAIN_PATH: &str = "rust/xtask/src/main.rs";
const START_MARKER: &str = "<!-- owner-resolution-manifest -->";
const END_MARKER: &str = "<!-- end-owner-resolution-manifest -->";
const INTERNAL_START: &str = "<!-- owner-resolution-internal-manifest -->";
const INTERNAL_END: &str = "<!-- end-owner-resolution-internal-manifest -->";

#[derive(Debug, PartialEq, Eq)]
struct InternalOwnerRow {
    surface: String,
    source: String,
    crate_name: String,
    module: String,
    entry: String,
    consumer_source: String,
    consumer_module: String,
    consumer_entry: String,
    receiver: String,
}

#[derive(Debug, PartialEq, Eq)]
struct OwnerRow {
    surface: String,
    source_paths: Vec<String>,
    entry_points: Vec<String>,
    axis: String,
    drift_gate: Option<String>,
}

/// Verify the owner manifest and its registered drift gates.
pub fn run() -> Result<ExitCode> {
    let root = crate::util::repo_root();
    let contract = read(&root, CONTRACT_PATH)?;
    let makefile = read(&root, MAKEFILE_PATH)?;
    let xtask_main = read(&root, XTASK_MAIN_PATH)?;

    let mut problems = validate_manifest(&root, &contract, &makefile, &xtask_main);
    problems.extend(validate_command_object_consumers(&root)?);
    problems.extend(validate_namespace_object_consumers(&root)?);
    problems.extend(validate_command_guard_consumers(&root)?);
    problems.extend(validate_variable_table_consumers(&root)?);
    problems.extend(validate_array_search_consumers(&root)?);
    problems.extend(validate_variable_command_grammars(&root)?);
    problems.extend(validate_native_bootstrap_consumers(&root)?);
    problems.extend(validate_native_variable_callback_consumers(&root)?);
    problems.extend(validate_private_error_header_consumers(&root)?);
    problems.extend(validate_native_instruction_name_consumers(&root)?);
    problems.extend(validate_jim_original_lookup_consumers(&root)?);
    problems.extend(validate_package_consumers(&root)?);
    problems.extend(validate_native_compiler_pass_consumers(&root)?);
    problems.extend(validate_native_coroutine_consumers(&root)?);
    problems.extend(validate_native_string_trim_consumers(&root)?);
    problems.extend(validate_native_list_storage_consumers(&root)?);
    problems.extend(validate_execution_name_policy_consumers(&root)?);
    problems.extend(validate_counted_name_facade_consumers(&root)?);
    problems.extend(validate_workspace_name_delimiters(&root)?);
    problems.extend(validate_f5_string_predicate_consumers(&root)?);
    if problems.is_empty() {
        let owner_count = parse_manifest(&contract).map_or_default(|rows| rows.len());
        println!(
            "owner-resolution: OK ({owner_count} owner row(s) resolve to live source and gates)"
        );
        return Ok(ExitCode::SUCCESS);
    }

    problems.sort();
    eprintln!("owner-resolution: contract drift detected:");
    for problem in problems {
        eprintln!("  - {problem}");
    }
    Ok(ExitCode::FAILURE)
}

/// The text a contract path stands for. A Rust path names a module: the
/// file, with the production text of its child modules placed ahead of its
/// own test modules, so a check written against a file still holds once the
/// file is split into `file/<module>.rs`.
fn read(root: &Path, relative: &str) -> Result<String> {
    let mut text = read_file(root, relative)?;
    if !is_rust_file(Path::new(relative)) {
        return Ok(text);
    }
    let children = child_module_files(root, relative);
    if children.is_empty() {
        return Ok(text);
    }
    let mut below = String::new();
    for child in &children {
        let child_text = read(root, child)?;
        below.push('\n');
        below.push_str(production_part(&child_text));
    }
    let at = production_part(&text).len();
    text.insert_str(at, &below);
    Ok(text)
}

/// One file's own text.
fn read_file(root: &Path, relative: &str) -> Result<String> {
    std::fs::read_to_string(root.join(relative))
        .with_context(|| format!("read owner-resolution input `{relative}`"))
}

fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}

/// `text` up to its first test-module declaration.
fn production_part(text: &str) -> &str {
    let lines: Vec<&str> = text.lines().collect();
    let Some(first_test_line) = crate::util::test_module_start(&lines) else {
        return text;
    };
    let offset: usize = text
        .split_inclusive('\n')
        .take(first_test_line)
        .map(str::len)
        .sum();
    &text[..offset]
}

/// The files of the modules directly below `relative` that are not test
/// code, sorted. A `mod.rs`, `lib.rs` or `main.rs` owns its directory; any
/// other file `a.rs` owns `a/`.
fn child_module_files(root: &Path, relative: &str) -> Vec<String> {
    let path = Path::new(relative);
    let Some(parent) = path.parent() else {
        return Vec::new();
    };
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    let is_root = matches!(stem, "mod" | "lib" | "main");
    let directory = if is_root {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    };
    let Ok(entries) = std::fs::read_dir(root.join(&directory)) else {
        return Vec::new();
    };
    let mut children = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let child = if entry.path().is_dir() {
            // A directory with its own `mod.rs`; one beside `<name>.rs` is that file's.
            if name == "bin" || !entry.path().join("mod.rs").is_file() {
                continue;
            }
            directory.join(&name).join("mod.rs")
        } else if is_rust_file(Path::new(&name))
            && !(is_root && matches!(name.as_str(), "mod.rs" | "lib.rs" | "main.rs" | "build.rs"))
        {
            directory.join(&name)
        } else {
            continue;
        };
        let child = child.to_string_lossy().replace('\\', "/");
        if !crate::util::test_sources::is_test_module_file(&child) {
            children.push(child);
        }
    }
    children.sort();
    children
}

fn validate_manifest(root: &Path, contract: &str, makefile: &str, xtask_main: &str) -> Vec<String> {
    let (source_routes, mut problems) = source_internal::validate(root, contract);
    let rows = match parse_manifest(contract) {
        Ok(rows) => rows,
        Err(problem) => {
            problems.push(problem);
            return problems;
        }
    };

    let owner_headings = owner_headings(contract);
    let mut manifest_owners = BTreeSet::new();
    let mut parsed_sources = std::collections::HashMap::<String, Option<syn::File>>::new();
    for row in &rows {
        for path in &row.source_paths {
            if let Some(owner) = owner_crate(path) {
                manifest_owners.insert(owner.to_owned());
            }
        }
    }
    for owner in owner_headings {
        if !manifest_owners.contains(&owner) {
            problems.push(format!(
                "owner heading `{owner}` has no row in the owner-resolution manifest"
            ));
        }
    }
    for (owner, declared) in declared_owner_bullets(contract) {
        let declared_name = declared.rsplit("::").next().unwrap_or(&declared);
        let covered = rows.iter().any(|row| {
            row.source_paths.iter().any(|path| {
                path.ends_with(&format!("/{declared_name}.rs"))
                    || path.contains(&format!("/{declared_name}/"))
            }) || row
                .entry_points
                .iter()
                .any(|entry| entry == &declared || entry == declared_name)
        });
        if !covered {
            problems.push(format!(
                "declared owner bullet `{owner}::{declared}` has no manifest row"
            ));
        }
    }

    validate_manifest_rows(
        root,
        makefile,
        xtask_main,
        rows,
        &source_routes,
        &mut parsed_sources,
        &mut problems,
    );
    problems.extend(validate_internal_manifest(root, contract));
    problems
}

fn validate_manifest_rows(
    root: &Path,
    makefile: &str,
    xtask_main: &str,
    rows: Vec<OwnerRow>,
    source_routes: &BTreeSet<(String, String, String)>,
    parsed_sources: &mut std::collections::HashMap<String, Option<syn::File>>,
    problems: &mut Vec<String>,
) {
    for row in rows {
        if row.surface.is_empty() {
            problems.push("owner row has an empty surface name".to_owned());
        }
        if row.source_paths.is_empty() {
            problems.push(format!("owner `{}` has no source path", row.surface));
        }
        if row.entry_points.is_empty() {
            problems.push(format!("owner `{}` has no public entry point", row.surface));
        }
        if row.axis.is_empty() {
            problems.push(format!(
                "owner `{}` has no dialect/release axis",
                row.surface
            ));
        }

        let mut source_texts = Vec::new();
        for source in &row.source_paths {
            let path = root.join(source);
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    parsed_sources
                        .entry(source.clone())
                        .or_insert_with(|| syn::parse_file(&text).ok());
                    source_texts.push(source.as_str());
                }
                Err(error) => problems.push(format!(
                    "owner `{}` source `{source}` is missing or unreadable: {error}",
                    row.surface
                )),
            }
        }
        for entry in &row.entry_points {
            let resolved_source = source_texts.iter().find(|source| {
                parsed_sources
                    .get(**source)
                    .and_then(Option::as_ref)
                    .is_some_and(|file| {
                        let module = std::path::Path::new(source)
                            .file_stem()
                            .and_then(|stem| stem.to_str());
                        let entry = entry
                            .split_once("::")
                            .filter(|(prefix, _)| Some(*prefix) == module)
                            .map_or(entry.as_str(), |(_, rest)| rest);
                        file_declares(file, entry)
                            || source_routes.contains(&(
                                row.surface.clone(),
                                source.to_string(),
                                entry.to_owned(),
                            ))
                    })
            });
            if resolved_source.is_none() {
                problems.push(format!(
                    "owner `{}` entry point `{entry}` has no public declaration in its source paths",
                    row.surface,
                ));
            }
        }

        if let Some(gate) = row.drift_gate {
            if gate == "check-name-resolution-proofs" {
                if !naming_proof_gate_is_current(root, makefile) {
                    problems.push(format!(
                        "owner `{}` names missing or altered naming-proof Makefile drift gate `{gate}`",
                        row.surface
                    ));
                }
                continue;
            }
            let Some(dispatch_name) = make_gate_command(makefile, &gate) else {
                problems.push(format!(
                    "owner `{}` names missing or non-xtask Makefile drift gate `{gate}`",
                    row.surface
                ));
                continue;
            };
            if !xtask_dispatches(xtask_main, &dispatch_name) {
                problems.push(format!(
                    "owner `{}` gate `{gate}` invokes `{dispatch_name}` but xtask dispatch has no matching command",
                    row.surface,
                ));
            }
        }
    }
}

fn owner_crate(path: &str) -> Option<&str> {
    let path = path.strip_prefix("rust/")?;
    path.split('/').next()
}

#[cfg(test)]
mod tests;
