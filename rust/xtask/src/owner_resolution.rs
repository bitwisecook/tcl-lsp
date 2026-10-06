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

const CONTRACT_PATH: &str = "docs/design/contracts/shared-utility-contracts-rust.md";
const MAKEFILE_PATH: &str = "Makefile";
const XTASK_MAIN_PATH: &str = "rust/xtask/src/main.rs";
const START_MARKER: &str = "<!-- owner-resolution-manifest -->";
const END_MARKER: &str = "<!-- end-owner-resolution-manifest -->";

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
    if problems.is_empty() {
        let owner_count = parse_manifest(&contract)
            .map(|rows| rows.len())
            .unwrap_or_default();
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

fn read(root: &Path, relative: &str) -> Result<String> {
    std::fs::read_to_string(root.join(relative))
        .with_context(|| format!("read owner-resolution input `{relative}`"))
}

fn validate_manifest(root: &Path, contract: &str, makefile: &str, xtask_main: &str) -> Vec<String> {
    let mut problems = Vec::new();
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
    problems
}

/// Native variable inventories preserve entry and declared-slot order; ABI
/// selection and physical ledgers must remain in their shared purpose owners.
fn validate_variable_table_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    let info = read(root, "rust/tcl-cmd-core/src/info.rs")?;
    for (start, end, checked) in [
        (
            "fn jim_namespace_variables<",
            "/// `info vars",
            "vars_in_bytes_checked(",
        ),
        (
            "pub fn vars<",
            "/// `info locals",
            "var_names_bytes_checked(",
        ),
        (
            "pub fn locals<",
            "/// `info globals",
            "var_names_bytes_checked(",
        ),
        (
            "pub fn globals<",
            "/// `info consts",
            "vars_in_bytes_checked(",
        ),
        (
            "fn qualified_variable_listing_bytes<",
            "/// Filter an already",
            "vars_in_bytes_checked(",
        ),
    ] {
        let section = info
            .split_once(start)
            .and_then(|(_, tail)| tail.split_once(end))
            .map(|(body, _)| body);
        if section.is_none_or(|section| !variable_inventory_is_owned(section, checked)) {
            problems.push(format!("variable inventory `{start}` bypasses checked owned order or sorts/deduplicates native declarations"));
        }
    }
    for path in ["rust/tcl-vm/src/interp.rs", "runtime/rust/src/interp.rs"] {
        let source = read(root, path)?;
        if !source.contains("supported_backend_hash_abi(")
            || !source.contains("native_variable_table_protocol(")
            || !source.contains("authored_variable_table_protocol(")
        {
            problems.push(format!(
                "variable table issuer in `{path}` omits independent ABI/native/authored selection"
            ));
        }
    }
    for path in ["rust/tcl-vm/src/vars.rs", "runtime/rust/src/frame.rs"] {
        let source = read(root, path)?;
        if !source.contains("NativeEntryLedger") || !source.contains("select_recipe(") {
            problems.push(format!(
                "variable storage in `{path}` bypasses the persistent physical entry owner"
            ));
        }
    }
    Ok(problems)
}

/// Jim lookup receipts retain native lifetimes separately from byte-name lookup.
fn validate_jim_original_lookup_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, doors) in [
        (
            "rust/tcl-vm/src/interp/jim_teardown.rs",
            &[
                "teardown_jim_interpreter",
                "invoke_jim_frame_defers",
                "release_object",
                "advance_jim_procedure_epoch",
            ][..],
        ),
        (
            "runtime/rust/src/interp/jim_teardown.rs",
            &[
                "teardown_jim_interpreter",
                "invoke_jim_frame_defers",
                "release_object",
                "retire_jim_procedure_epoch",
            ][..],
        ),
        (
            "rust/tcl-vm/src/interp/jim_local.rs",
            &[
                "native_jim_local_protocol()",
                "select_jim_previous_node",
                "jim_local_commands",
                "clean_jim_local_commands",
                "advance_jim_procedure_epoch",
            ][..],
        ),
        (
            "rust/tcl-vm/src/interp/native_jim_lookup.rs",
            &[
                "native_jim_lookup_protocol()",
                "with_jim_command_cache",
                "with_jim_current_namespace",
                "current_jim_variable_cell",
                "validate_jim_cell",
                "retain_original_jim_key",
            ][..],
        ),
        (
            "rust/tcl-vm/src/value/native_jim_lookup.rs",
            &["require_jim_lookup_origin", "WeakJimVariableCell"][..],
        ),
        (
            "rust/tcl-vm/src/exec/native_jim_script.rs",
            &["read_original_named_variable"][..],
        ),
        (
            "rust/tcl-vm/src/command.rs",
            &[
                "read_original_named_variable",
                "store_original_named_variable",
            ][..],
        ),
        (
            "rust/tcl-vm/src/interp/native_command_names.rs",
            &["native_jim_command_from_original_at"][..],
        ),
        (
            "runtime/rust/src/interp/jim_local.rs",
            &[
                "native_jim_local_protocol()",
                "retain_jim_local_command",
                "enter_jim_upcall",
                "clean_jim_local_key",
            ][..],
        ),
        (
            "runtime/rust/src/namespace/jim_local.rs",
            &[
                "jim_previous_commands",
                "follows_previous",
                "advance_jim_procedure_epoch",
                "take_slot",
            ][..],
        ),
        (
            "runtime/rust/src/interp/native_jim_lookup.rs",
            &[
                "native_jim_lookup_protocol()",
                "with_jim_command_cache",
                "native_jim_frame_id",
                "original_jim_variable_cell",
                "retain_original_jim_variable_key",
            ][..],
        ),
        (
            "runtime/rust/src/cmd_var.rs",
            &[
                "read_original_named_variable",
                "store_original_named_variable",
            ][..],
        ),
        (
            "runtime/rust/src/interp/native_command_names.rs",
            &["resolve_original_jim_command"][..],
        ),
    ] {
        let source = read(root, path)?;
        for door in doors {
            if !source.contains(door) {
                problems.push(format!("original Jim lookup in `{path}` bypasses `{door}`"));
            }
        }
    }
    Ok(problems)
}

/// Native core root birth order comes from the selected producer, not a visible-name replay.
fn validate_native_bootstrap_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    let hooks = read(root, "runtime/rust/src/interp/native_error_variables.rs")?;
    if !hooks.contains("native_error_variable_protocol()")
        || !hooks.contains("protocol.read(")
        || !hooks.contains("protocol.reset_order()")
        || !hooks.contains("home.binding_id?")
    {
        problems.push(
            "native error-variable callbacks bypass selected flag/object/cell lifetime owner"
                .into(),
        );
    }

    let vm_hooks = read(root, "rust/tcl-vm/src/interp/native_error_variables.rs")?;
    if !vm_hooks.contains("native_error_variable_protocol()")
        || !vm_hooks.contains("protocol.read(")
        || !vm_hooks.contains("protocol.reset_order()")
        || !vm_hooks.contains("native_error_cells.get(&id)")
        || !vm_hooks.contains("save_native_error_trace_state")
        || !vm_hooks.contains("append_counted_bytes")
        || !hooks.contains("save_native_error_trace_state")
        || !hooks.contains("append_counted_bytes")
    {
        problems.push(
            "hidden error objects bypass original-object read/reset/append/trace ownership".into(),
        );
    }

    for path in ["rust/tcl-vm/src/interp.rs", "runtime/rust/src/interp.rs"] {
        let source = read(root, path)?;
        if !source.contains("native_bootstrap_protocol()")
            || !source.contains("NativeBootstrapPurpose::CreateInterpreter")
            || !source.contains(".allocations(")
            || !source.contains("bootstrap_native_core(")
            || !source.contains("with_native_core(")
        {
            problems.push(format!(
                "native root bootstrap in `{path}` bypasses selected producer/allocation order"
            ));
        }
    }
    Ok(problems)
}

/// Private interpreter snapshots retain original headers rather than rebuilding children.
fn validate_package_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for path in [
        "rust/tcl-vm/src/cmd_package.rs",
        "runtime/rust/src/cmd_package.rs",
    ] {
        let source = read(root, path)?;
        let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
        for owner in [
            "native_package_protocol()",
            "c_members()",
            "native_index_from_original(",
            "retains_version_object",
        ] {
            // The VM's private version owner lives beside the package state.
            if owner == "retains_version_object" && path.starts_with("rust/") {
                continue;
            }
            if !production.contains(owner) {
                problems.push(format!(
                    "package consumer `{path}` omits shared original owner `{owner}`"
                ));
            }
        }
        for bypass in [
            ".runtime_version()",
            "from_utf8_lossy",
            ".package_protocol",
            "OptionTable::",
        ] {
            if production.contains(bypass) {
                problems.push(format!("package consumer `{path}` bypasses native byte/original selection with `{bypass}`"));
            }
        }
    }
    for path in [
        "rust/tcl-vm/src/interp/native_index_lookup.rs",
        "runtime/rust/src/interp/native_index_lookup.rs",
    ] {
        let source = read(root, path)?;
        let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
        // The high-level diagnostic wrapper also gets bytes on failure. The
        // actual primary door must authenticate and probe before its getter.
        let door = production
            .split("fn native_index_from_original_with_flags")
            .nth(1)
            .unwrap_or("");
        if !door.contains("cached_index_with_flags(")
            || !door.contains("let bytes =")
            || door.find("cached_index_with_flags(") > door.find("let bytes =")
        {
            problems.push(format!(
                "original Index getter `{path}` does not check authenticated cache before updater"
            ));
        }
    }
    let vm = read(root, "rust/tcl-vm/src/interp/native_package_files.rs")?;
    if !vm.contains("SharedPackageFileListMutation")
        || !vm.contains("native_list_append_elements(")
        || !vm.contains("take_package_file_scope")
    {
        problems.push(
            "VM package inventory lost original List ownership or native source scope".into(),
        );
    }
    Ok(problems)
}

fn validate_native_instruction_name_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, doors) in [
        (
            "rust/tcl-vm/src/interp.rs",
            [
                "native_return_options_application(purpose)",
                "inner_context_name()",
            ],
        ),
        (
            "runtime/rust/src/interp/native_return_instruction.rs",
            [
                "native_return_options_application(purpose)",
                "inner_context_name()",
            ],
        ),
        (
            "rust/tcl-vm/src/interp/native_error_stack.rs",
            [
                "new_native_instruction_name(name)",
                "native_list_replace_elements(",
            ],
        ),
        (
            "runtime/rust/src/interp/native_error_headers.rs",
            [
                "native_instruction_name::fresh(name)",
                "replace_elements_native(",
            ],
        ),
    ] {
        let source = read(root, path)?;
        if !doors.iter().all(|door| source.contains(door)) {
            problems.push(format!(
                "instruction error context in `{path}` bypasses selected name or original List ownership"
            ));
        }
    }
    let descriptor = read(root, "runtime/rust/src/obj/native_instruction_name.rs")?;
    if !descriptor.contains("NativeInstructionName::for_return(")
        || !descriptor.contains("free_int_rep_proc: None")
        || !descriptor.contains("dup_int_rep_proc: None")
        || !descriptor.contains("name.string_bytes()")
    {
        problems
            .push("Runtime instname descriptor bypasses shared opcode/updater ownership".into());
    }
    Ok(problems)
}

fn validate_private_error_header_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for path in ["rust/tcl-vm/src/interp.rs", "runtime/rust/src/interp.rs"] {
        let source = read(root, path)?;
        if !source.contains("native_error_objects_protocol()")
            || !source.contains("NativeErrorStack")
        {
            problems.push(format!(
                "private error headers in `{path}` bypass selected physical ownership"
            ));
        }
    }
    let vm = read(root, "rust/tcl-vm/src/interp/native_error_stack.rs")?;
    if !vm.contains("header: Option<Value>")
        || !vm.contains("ErrorStack<()>")
        || !vm.contains("native_list_replace_elements(")
    {
        problems.push(
            "VM private error stack duplicates semantic child ownership or bypasses header COW"
                .into(),
        );
    }
    let runtime = read(root, "runtime/rust/src/interp/native_error_headers.rs")?;
    if !runtime.contains("header: Option<obj::Owned>")
        || !runtime.contains("ErrorStack<()>")
        || !runtime.contains("new_dict_obj_native(")
        || !runtime.contains("replace_elements_native(")
    {
        problems
            .push("Runtime private error state bypasses actual List/Dict header ownership".into());
    }
    let instructions = read(root, "runtime/rust/src/interp/native_return_instruction.rs")?;
    for door in [
        "native_return_options_application(purpose)",
        "retains_merged_header()",
        "NativeReturnOptions::from_original(",
        "process_original_c_return_options",
        "capture_original_c_syntax_options",
        "reset_original_c_compiler_result",
        "native_scalar_probe(",
        "retain_native_error_option(false, error_code)",
    ] {
        if !instructions.contains(door) {
            problems.push(format!("original return instructions bypass `{door}`"));
        }
    }
    if !runtime.contains("is_original(") || !runtime.contains("duplicate_native_backing(") {
        problems.push(
            "original return operands bypass same-header retention or pre-extraction stack COW"
                .into(),
        );
    }
    let oo = read(root, "runtime/rust/src/cmd_oo.rs")?;
    if !oo.contains("original_argv: Option<Vec<*mut TclObj>>")
        || !oo.contains("original_argv: original_argv.as_deref()")
        || !oo.contains("oo_invoke_with_original(")
    {
        problems.push("Runtime TclOO CALL children bypass original invocation ownership".into());
    }
    let literals = read(root, "runtime/rust/src/interp/native_literal_pool.rs")?;
    if !literals.contains("source_literal_action(")
        || !literals.contains("Weak<RefCell<NativeLiteralWorld<obj::Owned>>>")
        || !literals.contains("retire_registered_members(")
        || !literals.contains("registered_c84_long(")
    {
        problems
            .push("Runtime executable literals bypass actual registration/source cleanup".into());
    }
    Ok(problems)
}

/// Direct callbacks use actual cell rows and the selected trace lifetime protocol.
fn validate_native_variable_callback_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for path in [
        "rust/tcl-vm/src/interp/native_variable_observers.rs",
        "runtime/rust/src/interp/native_variable_observers.rs",
    ] {
        let source = read(root, path)?;
        for door in [
            "native_variable_trace_protocol()",
            "same_registration(",
            "add_native_variable_observer",
            "remove_native_variable_observer",
        ] {
            if !source.contains(door) {
                problems.push(format!(
                    "direct native variable callback in `{path}` bypasses `{door}`"
                ));
            }
        }
    }
    for (path, door) in [
        (
            "rust/tcl-vm/src/interp.rs",
            "native_procedure_body_creation_protocol()",
        ),
        (
            "runtime/rust/src/interp/native_procedure_body.rs",
            "native_procedure_body_creation_protocol()",
        ),
    ] {
        if !read(root, path)?.contains(door) {
            problems.push(format!(
                "original chosen procedure body in `{path}` bypasses `{door}`"
            ));
        }
    }
    let pool = read(root, "rust/tcl-vm/src/literal_pool.rs")?;
    for door in [
        "source_literal_action(",
        "finalize_original_source_pool(",
        "CopySourceString",
        "RetainSourceCycle",
    ] {
        if !pool.contains(door) {
            problems.push(format!("original source literal array bypasses `{door}`"));
        }
    }
    Ok(problems)
}

/// Original array cursors and handle conversion remain in distinct shared owners.
fn validate_array_search_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for path in [
        "rust/tcl-vm/src/cmd_array.rs",
        "runtime/rust/src/cmd_array.rs",
    ] {
        let source = read(root, path)?;
        let source = source.split("#[cfg(test)]").next().unwrap_or(&source);
        if !source.contains("tcl_cmd_core::native_array_search::dispatch(") {
            problems.push(format!(
                "array search in `{path}` bypasses the shared original-object handler"
            ));
        }
        if ["strtoul(", "parse::<i32>", "parse::<u64>"]
            .iter()
            .any(|bypass| source.contains(bypass))
        {
            problems.push(format!(
                "array search in `{path}` duplicates native handle parsing"
            ));
        }
    }
    for path in [
        "rust/tcl-vm/src/interp.rs",
        "runtime/rust/src/state_traits.rs",
    ] {
        let source = read(root, path)?;
        if !source.contains("native_array_search_protocol(")
            || !source.contains("supported_backend_array_search_abi()?")
        {
            problems.push(format!("array search in `{path}` omits actual engine or independent unsigned-long ABI selection"));
        }
    }
    for path in ["rust/tcl-vm/src/vars.rs", "runtime/rust/src/frame.rs"] {
        let source = read(root, path)?;
        if !source.contains("NativeArraySearchChain") || !source.contains("array_searches.clear()")
        {
            problems.push(format!("array search in `{path}` omits original-cell chain ownership or mutation retirement"));
        }
    }
    Ok(problems)
}

fn variable_inventory_is_owned(section: &str, checked: &str) -> bool {
    section.contains(checked)
        && section.contains("filter_ordered_names(")
        && ![
            ".sort(",
            ".sort_unstable(",
            ".dedup(",
            "finish_unqualified_bytes(",
        ]
        .iter()
        .any(|bypass| section.contains(bypass))
}

/// Named command projections have an original-object purpose: buffer lookup
/// cannot substitute for reached cache effects. Check the concrete integration
/// categories alongside the public owner manifest.
fn validate_command_object_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, start, end) in [
        (
            "rust/tcl-vm/src/cmd_namespace.rs",
            "\"which\" => {",
            "\"export\" => {",
        ),
        (
            "rust/tcl-vm/src/exec.rs",
            "Op::RESOLVE_CMD => {",
            "Op::CLOCK_READ => {",
        ),
    ] {
        let source = read(root, path)?;
        let section = source
            .split_once(start)
            .and_then(|(_, tail)| tail.split_once(end))
            .map(|(section, _)| section);
        if section.is_none_or(|section| !command_object_projection_is_owned(section)) {
            problems.push(format!("original command-object projection in `{path}` bypasses its shared getter/name owner"));
        }
    }
    let dispatch = read(root, "rust/tcl-vm/src/exec.rs")?;
    let handler = dispatch
        .split_once("fn invoke_missing_command_value(")
        .and_then(|(_, tail)| tail.split_once("pub(crate) fn invoke_command_value_at("))
        .map(|(section, _)| section);
    if handler.is_none_or(|section| !original_handler_lookup_is_owned(section)) {
        problems.push("missing-command handlers must resolve their original command object through the shared getter".into());
    }
    let namespace = read(root, "runtime/rust/src/cmd_namespace.rs")?;
    let configuration = namespace
        .split_once("fn join_words(")
        .and_then(|(_, tail)| tail.split_once("fn ensemble_config_dict("))
        .map(|(section, _)| section);
    if configuration.is_none_or(|section| !ensemble_byte_serialization_is_owned(section)) {
        problems.push("namespace ensemble configuration must serialize original bytes through a selected native List".into());
    }
    let shim = read(root, "rust/tcl-cshim/src/obj.rs")?;
    if !shim.contains("Rep::CommandName(cache) => R::CommandName(cache.clone())") {
        problems.push(
            "callback command-name cache export must retain its opaque original receipt".into(),
        );
    }
    let bridge = read(root, "rust/tcl-engine-tclvm/src/lib.rs")?;
    if !bridge.contains("downcast_ref::<VmCommandNameCacheReceipt>()")
        || !bridge.contains("recover_native_command_name_cache(")
    {
        problems.push(
            "callback command-name cache recovery must authenticate its private engine receipt"
                .into(),
        );
    }
    Ok(problems)
}

fn validate_namespace_object_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    let getter = read(root, "rust/tcl-vm/src/interp/native_namespace_names.rs")?;
    for entry in [
        "original.native_namespace_name_cache()",
        "cache.is_current(",
        "actual.same_token(target)",
        "original.retire_native_namespace_name_cache()",
    ] {
        if !getter.contains(entry) {
            problems.push(format!("namespace object lookup must use original cache, selected lifecycle and closed ledger identity: missing `{entry}`"));
        }
    }
    let namespace = read(root, "rust/tcl-vm/src/cmd_namespace.rs")?;
    for entry in [
        "namespace::current_original",
        "namespace::parent_original",
        "namespace::children_tokens_checked",
        "namespace::children_original",
        "namespace::delete_original",
        "vm.namespace_object_lookup(original)",
    ] {
        if !namespace.contains(entry) {
            problems.push(format!("VM namespace consumers must retain original objects and selected physical tokens: missing `{entry}`"));
        }
    }
    let runtime_getter = read(root, "runtime/rust/src/interp/native_namespace_names.rs")?;
    for entry in [
        "obj::native_namespace_name::cache(original)",
        "cache.is_current(",
        "owns_namespace_name_token(target)",
        "obj::native_namespace_name::retire(original)",
    ] {
        if !runtime_getter.contains(entry) {
            problems.push(format!("Runtime namespace object lookup must authenticate the original cache against the actual lifecycle and closed ledger: missing `{entry}`"));
        }
    }
    let runtime_namespace = read(root, "runtime/rust/src/cmd_namespace.rs")?;
    for entry in [
        "namespace::current_original",
        "namespace::parent_original",
        "namespace::children_tokens_checked",
        "namespace::children_original",
        "namespace::namespace_objects_original",
        "namespace::delete_original",
        "interp.native_namespace_object_lookup(original)",
    ] {
        if !runtime_namespace.contains(entry) {
            problems.push(format!("Runtime namespace consumers must retain original objects and use the shared physical producer and deletion owners: missing `{entry}`"));
        }
    }
    let shared_namespace = read(root, "rust/tcl-cmd-core/src/namespace.rs")?;
    for entry in [
        "namespace_children_lookup",
        "find_namespace_child_bytes_checked",
        "let tokens = children_tokens_checked(ops, ns, pattern)?",
    ] {
        if !shared_namespace.contains(entry) {
            problems.push(format!("namespace children reporting and original-object consumers must share the release-specific physical query: missing `{entry}`"));
        }
    }
    for path in [
        "rust/tcl-vm/src/interp.rs",
        "runtime/rust/src/state_traits.rs",
    ] {
        if !read(root, path)?.contains("fn find_namespace_child_bytes_checked(") {
            problems.push(format!(
                "namespace children exact lookup requires an actual child-table door in {path}"
            ));
        }
    }
    let shim = read(root, "rust/tcl-cshim/src/obj.rs")?;
    if !shim.contains("Rep::NamespaceName(cache) => R::NamespaceName(cache.clone())") {
        problems.push(
            "callback namespace-name cache export must retain its opaque original receipt".into(),
        );
    }
    let bridge = read(root, "rust/tcl-engine-tclvm/src/lib.rs")?;
    if !bridge.contains("downcast_ref::<VmNamespaceNameCacheReceipt>()")
        || !bridge.contains("recover_native_namespace_name_cache(")
    {
        problems.push(
            "callback namespace-name cache recovery must authenticate its private engine receipt"
                .into(),
        );
    }
    Ok(problems)
}

fn validate_command_guard_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, manager, owner) in [
        (
            "rust/tcl-vm/src/interp.rs",
            "OwnedGuardManager<CommandTokenIdentity>",
            "command_token_identity(",
        ),
        (
            "runtime/rust/src/interp.rs",
            "OwnedGuardManager<u64>",
            ".resolve_generation(",
        ),
    ] {
        let source = read(root, path)?;
        if !command_guard_consumer_is_owned(&source, manager, owner) {
            problems.push(format!("runtime-command-guard-owner: `{path}` must retain the shared owner manager and actual command allocation lookup"));
        }
    }
    Ok(problems)
}

fn command_guard_consumer_is_owned(source: &str, manager: &str, owner: &str) -> bool {
    source.contains(manager)
        && source.contains(owner)
        && ![
            "HashMap<GuardToken",
            "BTreeMap<GuardToken",
            "HashMap<u64, GuardToken",
            "BTreeMap<u64, GuardToken",
        ]
        .iter()
        .any(|duplicate| source.contains(duplicate))
}

fn original_handler_lookup_is_owned(section: &str) -> bool {
    section.contains("lookup_original_command_at(")
        && ![
            "lookup_command_bytes_checked(",
            "lookup_command_in_namespace(",
            "resolve_command_fqn(",
            ".to_str()",
        ]
        .iter()
        .any(|bypass| section.contains(bypass))
}

fn ensemble_byte_serialization_is_owned(section: &str) -> bool {
    section.contains("native_string_materialization(")
        && section.contains("new_list_obj_native(")
        && section.contains("native_object_bytes(")
        && !["from_utf8_lossy(", "join_list(", "new_list_obj("]
            .iter()
            .any(|bypass| section.contains(bypass))
}

fn command_object_projection_is_owned(section: &str) -> bool {
    section.contains("native_namespace_command_name(")
        && ![
            "resolve_command_fqn(",
            "resolve_command_bytes_checked(",
            "lookup_command_bytes_checked(",
            "which_command_bytes(",
            "which_command_bytes_checked(",
            "origin_bytes_checked(",
            ".to_str()",
        ]
        .iter()
        .any(|bypass| section.contains(bypass))
}

/// Compiler replay consumes actual environment and opcode owners; earlier
/// literal arrays remain chronological native factories, not live cache pins.
fn validate_native_compiler_pass_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, required) in [
        (
            "rust/tcl-vm/src/interp.rs",
            vec![
                "capture_native_compiler_pass_environment(",
                "self.native_compiler_pass_owner.capture(",
                "self.command_owned_procedure_namespace(&proc)",
            ],
        ),
        (
            "rust/tcl-compiler/src/codegen/native_compiler_pass.rs",
            vec![
                "native_compiler_replays(",
                "ctx.instructions",
                "hazard(instruction.op)",
            ],
        ),
        (
            "rust/tcl-compiler/src/codegen/emitter/generate.rs",
            vec!["capture_first_pass(ctx)", "omit_command_markers(ctx)"],
        ),
        (
            "rust/tcl-compiler/src/codegen/emitter/mod.rs",
            vec![
                "native_compiler_pass::replay_environment(",
                "replay.lvt = std::mem::take(&mut asm.lvt)",
                "retain_discarded_native_pass(first_literals)",
                "retain_compiler_replay_environment(environment)",
            ],
        ),
        (
            "rust/tcl-vm/src/interp/native_command_names.rs",
            vec![
                "table.discarded_native_passes()",
                "create_native_literal_table_pool(",
                "drop(discarded)",
                "table.compiler_replay_environment()",
                "capture_native_compiler_pass_environment(None)",
            ],
        ),
        (
            "runtime/rust/src/interp/native_body_artifact/native_compiler_pass.rs",
            vec![
                "native_compiler_replays(",
                "materialize_body_literals(original)",
                "drop(first_pass)",
            ],
        ),
    ] {
        let source = read(root, path)?;
        for door in required {
            if !source.contains(door) {
                problems.push(format!(
                    "native compiler replay in `{path}` bypasses `{door}`"
                ));
            }
        }
    }
    Ok(problems)
}

/// Original List storage consumers share range and replacement geometry.
fn validate_native_list_storage_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, required) in [
        (
            "rust/tcl-vm/src/value/native_list_storage.rs",
            "range_action(",
        ),
        (
            "runtime/rust/src/list/native_list_storage.rs",
            "range_action(",
        ),
        ("rust/tcl-vm/src/native_list_backing.rs", "replace_layout("),
        (
            "runtime/rust/src/list/native_list_storage.rs",
            "replace_layout(",
        ),
        ("rust/tcl-vm/src/value.rs", "items.replace_native("),
        (
            "runtime/rust/src/list.rs",
            "replace_prepared_native_elements(",
        ),
        (
            "rust/tcl-vm/src/native_list_backing.rs",
            "fn check_generation(",
        ),
        ("runtime/rust/src/list.rs", "fn checked_generation("),
    ] {
        if !read(root, path)?.contains(required) {
            problems.push(format!(
                "native List storage in `{path}` bypasses original owner `{required}`"
            ));
        }
    }
    let execution = read(root, "rust/tcl-vm/src/exec.rs")?;
    for required in [
        "instr.native_list_range",
        "l.native_list_range(range, protocol)",
        "l.native_list_range_validate(protocol)",
    ] {
        if !execution.contains(required) {
            problems.push(format!(
                "native List range execution loses original coordinate/getter owner `{required}`"
            ));
        }
    }
    Ok(problems)
}

fn validate_native_coroutine_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, required) in [
        (
            "rust/tcl-registry/src/native_instruction_plan.rs",
            "native_coroutine_compilation::compile_native_coroutine(",
        ),
        (
            "rust/tcl-compiler/src/codegen/statements.rs",
            "native_coroutine_tasks(",
        ),
        (
            "rust/tcl-compiler/src/codegen/native_coroutine.rs",
            "NativeCoroutineStep::",
        ),
        (
            "rust/tcl-compiler/src/command_binding/compiled_preflight/original_preparation.rs",
            "NativeInstructionPlan::Coroutine(",
        ),
        (
            "runtime/rust/src/interp/native_body_artifact/native_coroutine.rs",
            "NativeCoroutineStep::",
        ),
        ("rust/tcl-vm/src/exec.rs", "request_yieldto_original("),
        (
            "rust/tcl-vm/src/cmd_coro.rs",
            "Frame::new_original_invocation(",
        ),
        ("runtime/rust/src/cmd_coro.rs", "OriginalHandoff"),
    ] {
        if !read(root, path)?.contains(required) {
            problems.push(format!("original coroutine consumer `{path}` lacks shared recipe or original-object transport `{required}`"));
        }
    }
    let source = read(root, "rust/tcl-vm/src/cmd_coro.rs")?;
    if let Some((_, creation)) = source.split_once("fn cmd_coroutine(")
        && creation
            .split("fn coro_resume(")
            .next()
            .unwrap_or(creation)
            .contains("compile_script_cached")
    {
        problems.push(
            "coroutine creation reconstructs source instead of retaining original argv".to_owned(),
        );
    }
    Ok(problems)
}

/// Original trim instructions share operand conversion/cut/result owners.
fn validate_native_string_trim_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for path in [
        "rust/tcl-vm/src/exec.rs",
        "runtime/rust/src/interp/native_body_artifact/native_string_trim.rs",
    ] {
        if !read(root, path)?.contains("tcl_cmd_core::string::compiled_trim(") {
            problems.push(format!(
                "native trim in `{path}` bypasses shared original operand/result ownership"
            ));
        }
    }
    for path in [
        "rust/tcl-compiler/src/codegen/statements/native_string.rs",
        "runtime/rust/src/interp/native_body_artifact/native_string_trim.rs",
    ] {
        if !read(root, path)?.contains("native_string_trim_compilation::default_trim_set(") {
            problems.push(format!(
                "native trim in `{path}` bypasses selected default character operand"
            ));
        }
    }
    let source = read(root, "rust/tcl-cmd-core/src/string/native_trim.rs")?;
    if !compiled_trim_operand_order_is_owned(&source) {
        problems.push(
            "shared native trim loses character-before-subject original getter order".to_owned(),
        );
    }
    Ok(problems)
}

fn compiled_trim_operand_order_is_owned(source: &str) -> bool {
    let Some(section) = source
        .split_once("pub fn compiled_trim<")
        .map(|(_, section)| {
            section
                .split("pub(super) fn command_trim<")
                .next()
                .unwrap_or(section)
        })
    else {
        return false;
    };
    let characters = section.find("ops.native_concat_string_bytes(characters)");
    let subject = section.find("ops.native_concat_string_bytes(subject)");
    matches!((characters, subject), (Some(characters), Some(subject)) if characters < subject)
}

fn parse_manifest(markdown: &str) -> Result<Vec<OwnerRow>, String> {
    let start = markdown
        .find(START_MARKER)
        .ok_or_else(|| format!("missing `{START_MARKER}` in {CONTRACT_PATH}"))?;
    let body = &markdown[start + START_MARKER.len()..];
    let end = body
        .find(END_MARKER)
        .ok_or_else(|| format!("missing `{END_MARKER}` in {CONTRACT_PATH}"))?;

    let mut rows = Vec::new();
    for line in body[..end].lines() {
        let cells = split_row(line);
        if cells.is_empty() || cells[0] == "Surface" || cells.iter().all(|cell| *cell == "---") {
            continue;
        }
        if cells.len() != 5 {
            return Err(format!(
                "owner manifest row must have five columns: `{line}`"
            ));
        }
        let source_paths = code_tokens(cells[1]);
        let entry_points = code_tokens(cells[2]);
        let drift_gate = if cells[4] == "none" {
            None
        } else {
            let gates = code_tokens(cells[4]);
            if gates.len() != 1 {
                return Err(format!(
                    "owner manifest drift gate must be `none` or one backticked Make target: `{line}`"
                ));
            }
            gates.into_iter().next()
        };
        rows.push(OwnerRow {
            surface: cells[0].to_owned(),
            source_paths,
            entry_points,
            axis: cells[3].to_owned(),
            drift_gate,
        });
    }
    if rows.is_empty() {
        return Err(format!("owner manifest in {CONTRACT_PATH} has no rows"));
    }
    Ok(rows)
}

fn split_row(line: &str) -> Vec<&str> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return Vec::new();
    }
    trimmed[1..trimmed.len() - 1]
        .split('|')
        .map(str::trim)
        .collect()
}

fn code_tokens(cell: &str) -> Vec<String> {
    cell.split('`')
        .enumerate()
        .filter_map(|(index, part)| (index % 2 == 1).then_some(part.trim().to_owned()))
        .filter(|part| !part.is_empty())
        .collect()
}

fn owner_headings(markdown: &str) -> BTreeSet<String> {
    let Some(start) = markdown.find("## Owners") else {
        return BTreeSet::new();
    };
    let tail = &markdown[start..];
    let end = tail.find("\n## Decision rules").unwrap_or(tail.len());
    tail[..end]
        .lines()
        .filter_map(|line| line.strip_prefix("### `"))
        .filter_map(|line| line.split('`').next())
        .map(str::to_owned)
        .collect()
}

fn declared_owner_bullets(markdown: &str) -> Vec<(String, String)> {
    let Some(start) = markdown.find("## Owners") else {
        return Vec::new();
    };
    let tail = &markdown[start..];
    let end = tail.find("\n## Decision rules").unwrap_or(tail.len());
    let mut owner = String::new();
    let mut out = Vec::new();
    for line in tail[..end].lines() {
        if let Some(heading) = line.strip_prefix("### `") {
            heading
                .split('`')
                .next()
                .unwrap_or_default()
                .clone_into(&mut owner);
        } else if let Some(bullet) = line.trim_start().strip_prefix("- `")
            && let Some(name) = bullet.split('`').next()
        {
            out.push((owner.clone(), name.to_owned()));
        }
    }
    out
}

#[cfg(test)]
fn entry_is_declared(source: &str, entry: &str) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    file_declares(&file, entry)
}

fn file_declares(file: &syn::File, entry: &str) -> bool {
    let (owner, ident) = entry
        .rsplit_once("::")
        .map_or((None, entry), |(owner, ident)| {
            (Some(owner.rsplit("::").next().unwrap_or(owner)), ident)
        });
    items_declare(&file.items, owner, ident)
}

fn exported(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

fn fields_declare(fields: &syn::Fields, ident: &str) -> bool {
    fields
        .iter()
        .any(|field| exported(&field.vis) && field.ident.as_ref().is_some_and(|name| name == ident))
}

fn items_declare(items: &[syn::Item], owner: Option<&str>, ident: &str) -> bool {
    items.iter().any(|item| match item {
        syn::Item::Fn(item) => owner.is_none() && exported(&item.vis) && item.sig.ident == ident,
        syn::Item::Struct(item) => {
            exported(&item.vis)
                && if let Some(owner) = owner {
                    item.ident == owner && fields_declare(&item.fields, ident)
                } else {
                    item.ident == ident || fields_declare(&item.fields, ident)
                }
        }
        syn::Item::Enum(item) => {
            exported(&item.vis)
                && if let Some(owner) = owner {
                    item.ident == owner
                        && item.variants.iter().any(|variant| variant.ident == ident)
                } else {
                    item.ident == ident
                }
        }
        syn::Item::Trait(item) => {
            exported(&item.vis)
                && if let Some(owner) = owner {
                    item.ident == owner && item.items.iter().any(|member| {
                        matches!(member, syn::TraitItem::Fn(method) if method.sig.ident == ident)
                    })
                } else {
                    item.ident == ident
                }
        }
        syn::Item::Impl(item) => {
            let matches_owner = owner.is_none_or(|owner| {
                matches!(item.self_ty.as_ref(), syn::Type::Path(path)
                    if path.path.segments.last().is_some_and(|segment| segment.ident == owner))
            });
            let locally_private = match item.self_ty.as_ref() {
                syn::Type::Path(path) => path.path.segments.last().is_some_and(|segment| {
                    items.iter().any(|declaration| match declaration {
                        syn::Item::Struct(value) => {
                            value.ident == segment.ident && !exported(&value.vis)
                        }
                        syn::Item::Enum(value) => {
                            value.ident == segment.ident && !exported(&value.vis)
                        }
                        _ => false,
                    })
                }),
                _ => false,
            };
            matches_owner
                && !locally_private
                && item.items.iter().any(|member| match member {
                    syn::ImplItem::Fn(method) => exported(&method.vis) && method.sig.ident == ident,
                    syn::ImplItem::Const(value) => exported(&value.vis) && value.ident == ident,
                    syn::ImplItem::Type(value) => exported(&value.vis) && value.ident == ident,
                    _ => false,
                })
        }
        syn::Item::Const(item) => owner.is_none() && exported(&item.vis) && item.ident == ident,
        syn::Item::Static(item) => owner.is_none() && exported(&item.vis) && item.ident == ident,
        syn::Item::Type(item) => owner.is_none() && exported(&item.vis) && item.ident == ident,
        syn::Item::Use(item) => exported(&item.vis) && use_declares(&item.tree, ident),
        syn::Item::Mod(item) => item.content.as_ref().is_some_and(|(_, items)| {
            if owner.is_some_and(|owner| item.ident == owner) {
                exported(&item.vis) && items_declare(items, None, ident)
            } else {
                items_declare(items, owner, ident)
            }
        }),
        _ => false,
    })
}

fn use_declares(tree: &syn::UseTree, ident: &str) -> bool {
    match tree {
        syn::UseTree::Name(item) => item.ident == ident,
        syn::UseTree::Rename(item) => item.rename == ident,
        syn::UseTree::Path(item) => use_declares(&item.tree, ident),
        syn::UseTree::Group(item) => item.items.iter().any(|item| use_declares(item, ident)),
        syn::UseTree::Glob(_) => false,
    }
}

fn xtask_dispatches(main: &str, command: &str) -> bool {
    let variant = match command {
        "resolution-drift" => "ResolutionDrift",
        "segmentation-drift" => "SegmentationDrift",
        "number-drift" => "NumberDrift",
        "audit-option-dialects" => "AuditOptionDialects",
        "command-backing" => "WasmBacking",
        "diag-tables" => "DiagTables",
        "gen-ai-diagnostics" => "GenAiDiagnostics",
        "gen-editor-extensions" => "GenEditorExtensions",
        "owner-resolution" => "OwnerResolution",
        "sslictcl-data" => "SslictclData",
        _ => return false,
    };
    let prefix = format!("Command::{variant}");
    main.lines().any(|line| {
        let trimmed = line.trim_start();
        if !trimmed.starts_with(&prefix) {
            return false;
        }
        // A single-line arm carries its `=>`; an arm whose struct pattern
        // binds fields opens a brace and puts the `=>` several lines later.
        trimmed.contains("=>") || trimmed.ends_with('{')
    })
}

fn owner_crate(path: &str) -> Option<&str> {
    let path = path.strip_prefix("rust/")?;
    path.split('/').next()
}

fn make_gate_command(makefile: &str, target: &str) -> Option<String> {
    direct_gate_command(makefile, target).or_else(|| {
        // A gate may be an alias: a `.PHONY` target whose whole body is one
        // prerequisite that carries the recipe (`xtask-sslictcl-data:
        // check-source-data`). The manifest should be free to name the gate a
        // reader would run, so follow the prerequisites one level rather than
        // forcing the contract to name the inner target.
        gate_prerequisites(makefile, target)
            .into_iter()
            .find_map(|prerequisite| direct_gate_command(makefile, &prerequisite))
    })
}

/// The `cargo xtask <verb>` this target's own recipe runs, if any.
fn direct_gate_command(makefile: &str, target: &str) -> Option<String> {
    let target_prefix = format!("{target}:");
    let mut in_recipe = false;
    for line in makefile.lines() {
        if !in_recipe {
            if line.trim_start().starts_with(&target_prefix) {
                in_recipe = true;
            }
            continue;
        }
        if !line.starts_with('\t') {
            if !line.trim().is_empty() {
                break;
            }
            continue;
        }
        let Some(command) = line.split_once("cargo xtask ").map(|(_, rest)| rest) else {
            continue;
        };
        return command.split_whitespace().next().map(str::to_owned);
    }
    None
}

/// The prerequisites named on `target`'s rule line, ignoring a trailing
/// `## help` comment. Empty when the target has no rule (or only `.PHONY`
/// declarations mention it, which never carry a `:` in that position).
fn gate_prerequisites(makefile: &str, target: &str) -> Vec<String> {
    let target_prefix = format!("{target}:");
    for line in makefile.lines() {
        if line.starts_with(&target_prefix) {
            let rest = &line[target_prefix.len()..];
            let rest = rest.split_once("##").map_or(rest, |(head, _)| head);
            return rest.split_whitespace().map(str::to_owned).collect();
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn a_multi_line_dispatch_arm_still_counts() {
        let main = "        Command::SslictclData {\n            operation,\n        } => sslictcl_data::run(&operation),\n";
        assert!(xtask_dispatches(main, "sslictcl-data"));
        assert!(!xtask_dispatches(main, "owner-resolution"));
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
            .filter(|line| !line.starts_with("| glob matching |"))
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
            "`rust/tcl-syntax/src/list.rs`",
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
            "`rust/tcl-syntax/src/list.rs`",
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
}
