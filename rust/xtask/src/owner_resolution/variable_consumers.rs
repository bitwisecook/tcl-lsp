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

//! Consumer checks for the variable owners: variable tables, variable callbacks, array searches and the variable command grammars.

use super::read;
use anyhow::Result;
use std::path::Path;

/// Native variable inventories preserve entry and declared-slot order; ABI
/// selection and physical ledgers must remain in their shared purpose owners.
pub(super) fn validate_variable_table_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    let info = read(root, "rust/tcl-cmd-core/src/info.rs")?;
    for (start, end, checked) in [
        (
            "fn jim_namespace_variables<",
            "/// `info vars",
            "vars_in_bytes_checked(",
        ),
        ("pub fn vars<", "/// `info locals", "filter_frame_names("),
        ("pub fn locals<", "/// `info globals", "filter_frame_names("),
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
    let frame_filter = info
        .split_once("fn filter_frame_names<")
        .and_then(|(_, tail)| tail.split_once("fn selected_variable_inventory_policy<"))
        .map(|(body, _)| body);
    if frame_filter.is_none_or(|body| {
        !body.contains("var_names_bytes_checked(")
            || !body.contains("var_name_pattern_inputs_bytes_checked(")
            || !body.contains("match_name_pattern(purpose,")
            || [".sort(", ".sort_unstable(", ".dedup("]
                .iter()
                .any(|bypass| body.contains(bypass))
    }) {
        problems.push("frame variable inventory bypasses original checked declarations or pattern-purpose ownership".into());
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

/// Direct callbacks use actual cell rows and the selected trace lifetime protocol.
pub(super) fn validate_native_variable_callback_consumers(root: &Path) -> Result<Vec<String>> {
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
pub(super) fn validate_array_search_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for path in [
        "rust/tcl-vm/src/cmd_array.rs",
        "runtime/rust/src/cmd_array.rs",
    ] {
        let source = read(root, path)?;
        let source = source.split("#[cfg(test)]").next().unwrap_or(&source);
        if !source.contains("tcl_cmd_core::native_array_search::dispatch_bytes(") {
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

pub(super) fn validate_variable_command_grammars(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for path in ["runtime/rust/src/cmd_var.rs", "rust/tcl-vm/src/command.rs"] {
        let source = read(root, path)?;
        if !source.contains("native_variable_declaration_protocol()")
            || !source.contains("grammar.arguments(")
        {
            problems.push(format!(
                "variable declaration in `{path}` bypasses its shared original argument grammar"
            ));
        }
    }
    let array = read(root, "rust/tcl-cmd-core/src/array.rs")?;
    let names_mode = array
        .split_once("fn names_mode<")
        .and_then(|(_, tail)| tail.split_once("fn names<"))
        .map(|(body, _)| body);
    if names_mode.is_none_or(|body| {
        let compact: String = body.split_whitespace().collect();
        !compact.contains(
            "crate::prefix::OptionTable::abbreviating(\"option\",&MODES).index_of_original(ops,mode)",
        )
    }) {
        problems.push("array names mode bypasses shared original option lookup".into());
    }
    let names = array
        .split_once("fn names<")
        .and_then(|(_, tail)| tail.split_once("fn get<"))
        .map(|(body, _)| body);
    if names.is_none_or(|body| {
        let compact: String = body.split_whitespace().collect();
        ![
            "NativeArrayNamesProtocol::from_name_policy(policy)",
            "array_key_bytes_checked_at(",
            "names_mode(ops,&rest[1])?",
            "names_mode(ops,original_mode)?",
            "NativeNameGlobPurpose::ArrayNamesSearch",
            "tcl_core_types::c_string_extent(",
        ]
        .iter()
        .all(|door| compact.contains(door))
            || [
                "string_match_bytes(",
                "from_utf8_lossy(",
                ".sort(",
                ".sort_unstable(",
                ".dedup(",
            ]
            .iter()
            .any(|bypass| compact.contains(bypass))
    }) {
        problems.push(
            "array names bypasses shared original mode, pattern or key inventory ownership".into(),
        );
    }
    Ok(problems)
}

pub(super) fn variable_inventory_is_owned(section: &str, checked: &str) -> bool {
    section.contains(checked)
        && (section.contains("filter_ordered_names(") || section.contains("filter_frame_names("))
        && ![
            ".sort(",
            ".sort_unstable(",
            ".dedup(",
            "finish_unqualified_bytes(",
        ]
        .iter()
        .any(|bypass| section.contains(bypass))
}
