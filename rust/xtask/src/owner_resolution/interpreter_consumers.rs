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

//! Consumer checks for the interpreter's shared owners: bootstrap, packages, lookups, instruction names, error headers, the compiler pass, list storage, coroutines, string trim, name policy and the F5 string predicates.

use super::read;
use anyhow::Result;
use std::path::Path;

pub(super) fn validate_f5_string_predicate_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, owners) in [
        (
            "rust/tcl-compiler/src/tcl_expr_eval.rs",
            &[
                "f5_predicates:Option<",
                "provider.predicate(op)",
                "predicate.evaluate(left,right)",
            ][..],
        ),
        (
            "rust/tcl-vm/src/expr.rs",
            &[
                "authored_f5_string_predicate_provider(",
                ".predicate(op)",
                ".evaluate(&subject,&operand)",
            ][..],
        ),
        (
            "rust/tcl-vm/src/exec.rs",
            &[
                "authored_f5_string_predicate_provider(",
                "expr::irule_binary(provider,op,&a,&b)",
            ][..],
        ),
        (
            "runtime/rust/src/expr/native_values.rs",
            &[
                ".f5_string_predicate_provider()",
                "provider.predicate(operator)",
                "predicate.evaluate(&left,&right)",
            ][..],
        ),
        (
            "runtime/rust/src/builtins.rs",
            &[
                "authored_f5_string_predicate_provider(",
                "self.interp.expression_evaluation_policy().as_ref()",
            ][..],
        ),
        (
            "runtime/rust/src/interp/native_body_artifact/native_control.rs",
            &[
                "authored_f5_string_predicate_provider(",
                "self.artifact.stamp.expression_policy.as_ref()",
            ][..],
        ),
    ] {
        let source = read(root, path)?;
        let compact: String = source.split_whitespace().collect();
        if path == "rust/tcl-vm/src/exec.rs"
            && compact.contains("AuthoredF5StringPredicateProvider::F5Trunk")
        {
            problems.push(
                "F5 predicate opcode constructs a provider without the installed policy".into(),
            );
        }
        for owner in owners {
            if !compact.contains(owner) {
                problems.push(format!(
                    "F5 string predicate consumer `{path}` omits shared typed owner `{owner}`"
                ));
            }
        }
    }
    Ok(problems)
}

/// Jim lookup receipts retain native lifetimes separately from byte-name lookup.
#[allow(clippy::too_many_lines)]
pub(super) fn validate_jim_original_lookup_consumers(root: &Path) -> Result<Vec<String>> {
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
pub(super) fn validate_native_bootstrap_consumers(root: &Path) -> Result<Vec<String>> {
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
pub(super) fn validate_package_consumers(root: &Path) -> Result<Vec<String>> {
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
            "native_index_from_original_checked(",
            "retains_version_object",
        ] {
            // The VM's private version owner lives beside the package state.
            if owner == "retains_version_object" && path.starts_with("rust/") {
                continue;
            }
            let owner =
                if owner == "native_index_from_original_checked(" && path.starts_with("runtime/") {
                    "native_index_from_original("
                } else {
                    owner
                };
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

pub(super) fn validate_native_instruction_name_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, doors) in [
        (
            "rust/tcl-vm/src/interp.rs",
            [
                "native_return_options_application(purpose)",
                "NativeReturnOptionsApplicationProtocol::inner_context_name",
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

pub(super) fn validate_private_error_header_consumers(root: &Path) -> Result<Vec<String>> {
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
        "NativeReturnOptions::from_original_using(",
        "process_original_c_return_options",
        "capture_original_c_syntax_options",
        "reset_original_c_compiler_result",
        "native_scalar_probe_with_stage_check(",
        "set_original_c_error_code_using(",
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

/// Compiler replay consumes actual environment and opcode owners; earlier
/// literal arrays remain chronological native factories, not live cache pins.
pub(super) fn validate_native_compiler_pass_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, required) in [
        (
            "rust/tcl-vm/src/interp.rs",
            vec![
                "capture_native_compiler_pass_environment(",
                ".capture_with_ordinary_entry_observation(",
                "self.command_owned_procedure_namespace(&proc)",
            ],
        ),
        (
            "rust/tcl-codegen/src/codegen/native_compiler_pass.rs",
            vec![
                "native_compiler_replays(",
                "ctx.instructions",
                "hazard(instruction.op)",
            ],
        ),
        (
            "rust/tcl-codegen/src/codegen/emitter/generate.rs",
            vec!["capture_first_pass(ctx)", "omit_command_markers(ctx)"],
        ),
        (
            "rust/tcl-codegen/src/codegen/emitter/mod.rs",
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
pub(super) fn validate_native_list_storage_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, required) in [
        (
            "rust/tcl-vm/src/value/native_list_storage.rs",
            "NativeListRangeSelection",
        ),
        (
            "runtime/rust/src/list/native_list_storage.rs",
            "NativeListRangeSelection",
        ),
        ("rust/tcl-vm/src/native_list_backing.rs", "replace_layout("),
        (
            "runtime/rust/src/list/native_list_storage.rs",
            "replace_layout(",
        ),
        ("rust/tcl-vm/src/value.rs", "items.replace_native("),
        (
            "runtime/rust/src/list.rs",
            "replace_prepared_native_elements_using(",
        ),
        (
            "rust/tcl-vm/src/native_list_backing.rs",
            "fn check_generation(",
        ),
        ("runtime/rust/src/list.rs", "fn checked_generation("),
    ] {
        let source = read(root, path)?;
        let compact: String = source.split_whitespace().collect();
        if !source.contains(required)
            || (required == "NativeListRangeSelection" && !compact.contains("selection.action("))
        {
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

pub(super) fn validate_native_coroutine_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for (path, required) in [
        (
            "rust/tcl-registry/src/native_instruction_plan.rs",
            "native_coroutine_compilation::compile_native_coroutine(",
        ),
        (
            "rust/tcl-codegen/src/codegen/statements.rs",
            "native_coroutine_tasks(",
        ),
        (
            "rust/tcl-codegen/src/codegen/native_coroutine.rs",
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
pub(super) fn validate_native_string_trim_consumers(root: &Path) -> Result<Vec<String>> {
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
        "rust/tcl-codegen/src/codegen/statements/native_string.rs",
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

pub(super) fn compiled_trim_operand_order_is_owned(source: &str) -> bool {
    let Some(section) = source
        .split_once("pub fn compiled_trim<")
        .map(|(_, section)| {
            section
                .split("pub(in super::super) fn command_trim<")
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

pub(super) fn validate_execution_name_policy_consumers(root: &Path) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    let dialect = read(
        root,
        "rust/tcl-native-model/src/native_string_materialization.rs",
    )?;
    let name_issuer = dialect
        .split_once("pub fn native_name_protocol(")
        .and_then(|(_, body)| body.split_once("pub fn authored_name_policy("))
        .map(|(body, _)| body);
    if name_issuer.is_none_or(|body| {
        body.contains("native_string_protocol(") || body.contains("NativeStringProtocol")
    }) {
        problems.push("native naming issuer depends on producer-string protocol".to_owned());
    }
    let snapshot = read(root, "rust/tcl-runtime-api/src/native_compilation.rs")?;
    if !snapshot.contains("pub fn execution_name_policy(")
        || !snapshot.contains("self.name_protocol")
        || !snapshot.contains("self.execution_name_policy()?.native_recipe()")
    {
        problems
            .push("native entry does not retain independent naming purpose admission".to_owned());
    }
    for path in [
        "rust/tcl-codegen/src/codegen/values.rs",
        "rust/tcl-codegen/src/codegen/native_control.rs",
        "rust/tcl-codegen/src/codegen/statements.rs",
        "rust/tcl-compiler/src/command_binding/native_variable_tables.rs",
    ] {
        let source = read(root, path)?;
        if source.contains("entry.name_protocol") || !source.contains("entry.command_name_policy()")
        {
            problems.push(format!(
                "native compiler `{path}` bypasses independent naming admission"
            ));
        }
    }
    let observed = read(root, "rust/tcl-registry/src/f5/naming.rs")?;
    for required in [
        "build == BigIpBuild::MEASURED_21_1_0_1",
        "context == BigIpExecutionContext::TmmIRule",
        "event == BigIpNameEvent::HttpRequest",
    ] {
        if !observed.contains(required) {
            problems.push(format!(
                "observed naming issuer loses exact context: `{required}`"
            ));
        }
    }
    let class_queries = read(root, "rust/tcl-compiler/src/analyser/class_hierarchy.rs")?;
    let written_query = class_queries
        .split_once("pub fn resolve_written_class_name_in_context(")
        .or_else(|| class_queries.split_once("pub fn resolve_written_class_name_in_context<"));
    if written_query.is_none()
        || !class_queries.contains("resolve_class_lookup(&lookup, classes)")
        || !class_queries.contains("source_name_ambiguous")
    {
        problems.push(
            "source class queries lose retained lookup geometry or collision withdrawal".to_owned(),
        );
    }
    Ok(problems)
}
