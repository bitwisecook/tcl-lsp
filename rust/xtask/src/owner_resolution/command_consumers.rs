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

//! Consumer checks for the command owners: command objects, namespace objects and command guards.

use super::read;
use anyhow::Result;
use std::path::Path;

/// Named command projections have an original-object purpose: buffer lookup
/// cannot substitute for reached cache effects. Check the concrete integration
/// categories alongside the public owner manifest.
pub(super) fn validate_command_object_consumers(root: &Path) -> Result<Vec<String>> {
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
        .and_then(|(_, tail)| tail.split_once("fn ensemble_config_list("))
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

pub(super) fn validate_namespace_object_consumers(root: &Path) -> Result<Vec<String>> {
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
        "obj::native_namespace_name::retire_using(",
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

pub(super) fn validate_command_guard_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, manager, owner) in [
        (
            "rust/tcl-vm/src/interp.rs",
            "OwnedGuardManager<CommandTokenIdentity>",
            "command_token_identity(",
        ),
        (
            "runtime/rust/src/interp.rs",
            "OwnedGuardManager<u64, Option<crate::namespace::CommandGuardLookup>>",
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

pub(super) fn command_guard_consumer_is_owned(source: &str, manager: &str, owner: &str) -> bool {
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

pub(super) fn original_handler_lookup_is_owned(section: &str) -> bool {
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

pub(super) fn ensemble_byte_serialization_is_owned(section: &str) -> bool {
    section.contains("native_string_materialization(")
        && section.contains("new_list_obj_native(")
        && section.contains("native_object_bytes(")
        && !["from_utf8_lossy(", "join_list(", "new_list_obj("]
            .iter()
            .any(|bypass| section.contains(bypass))
}

pub(super) fn command_object_projection_is_owned(section: &str) -> bool {
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
