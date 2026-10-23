#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Dedicated complete reference suite. Selection and validation stay in
# tcl-test-support; a missing or invalid interpreter is a test failure.
set -euo pipefail
export TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1
export TCL_LSP_REQUIRE_JIM_ORACLE=1
cargo test -p tcl-test-support
cargo test -p tcl-syntax --test command_resolution_conformance --test variable_resolution_conformance --test namespace_op_conformance --test jim_name_resolution_conformance --test execution_resolution_conformance --test body_execution_conformance
cargo test -p tcl-compiler --test resolution_rewrite_oracles --test native_arithmetic_conformance --test codegen_integration
cargo test -p tcl-vm --test native_compilation_conformance --test cross_interp_alias_e2e --test legacy_variable_traces_e2e --test command_traces_e2e --test step_trace_inline_e2e --test variable_name_resolution_e2e --test variable_trace_semantics_e2e --test element_recovery_traces_e2e
cargo test -p tcl-registry --test dialect_oracle --test package_protocol_oracle
cargo test -p tcl-dialect --test package_version_oracle
cargo test --manifest-path runtime/rust/Cargo.toml --test array_trace_oracle

# Explicit integration targets above do not run library fixture consumers.
# Each owner selection must execute at least one test; a stale filter fails.
run_naming_owner() {
    local owner="$1" filter="$2" log
    shift 2
    log=$(mktemp "/tmp/tcl-naming-${owner}.XXXXXX.log")
    if ! cargo test "$@" "$filter" -- --nocapture --test-threads=1 | tee "$log"; then
        return 1
    fi
    if ! rg -q '^test result: ok\. [1-9][0-9]* passed;' "$log"; then
        printf 'Naming owner %s filter %s executed no passing tests; see %s\n' "$owner" "$filter" "$log" >&2
        return 1
    fi
}

run_naming_owner tcl-core-types 'name_bytes::' -p tcl-core-types --lib
run_naming_owner tcl-core-types 'native_hash_order::' -p tcl-core-types --lib
run_naming_owner tcl-cmd-core 'namespace::' -p tcl-cmd-core --lib
run_naming_owner tcl-cmd-core 'trace::' -p tcl-cmd-core --lib
run_naming_owner tcl-syntax 'naming::' -p tcl-syntax --lib
run_naming_owner tcl-syntax 'native_variable_name::' -p tcl-syntax --lib
run_naming_owner tcl-syntax 'native_namespace_name::' -p tcl-syntax --lib
run_naming_owner tcl-syntax 'native_jim_lookup::' -p tcl-syntax --lib
run_naming_owner tcl-syntax 'native_variable_words::' -p tcl-syntax --lib
run_naming_owner tcl-syntax 'formal_params::' -p tcl-syntax --lib
run_naming_owner tcl-registry 'native_procedure::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_namespace_' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_upvar_compilation::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_info_exists_compilation::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_instruction_plan::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_mathop_compilation::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_list_operations_compilation::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_dictionary_scope_compilation::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_property_lookup::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_tcloo_method_cache::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_variable_table::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_variable_trace::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_package::' -p tcl-registry --lib
run_naming_owner tcl-registry 'native_string_materialization::' -p tcl-registry --lib
run_naming_owner tcl-compiler 'auto_path_eval::path_constants::' -p tcl-compiler --lib
run_naming_owner tcl-compiler 'command_binding::runtime_entry::' -p tcl-compiler --lib
run_naming_owner tcl-compiler 'codegen::statements::native_namespace_upvar::' -p tcl-compiler --lib
run_naming_owner tcl-compiler 'var_resolve::' -p tcl-compiler --lib
run_naming_owner tcl-compiler 'variable_bindings::' -p tcl-compiler --lib
run_naming_owner tcl-compiler 'allocated_instance::' -p tcl-compiler --lib
run_naming_owner tcl-vm 'command::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::native_command_names::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::native_namespace_names::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::native_variable_names::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::execution_name_policy::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::native_variable_observers::' -p tcl-vm --lib
run_naming_owner tcl-vm 'cmd_mathop::' -p tcl-vm --lib
run_naming_owner tcl-vm 'exec::native_list_operations_tests::' -p tcl-vm --lib
run_naming_owner tcl-vm 'cmd_dict::native_rmw_fixture_tests::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::native_jim_lookup::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::native_jim_links::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::native_rename::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::native_upvar_info_exists_tests::' -p tcl-vm --lib
run_naming_owner tcl-vm 'interp::native_compiled_locals::' -p tcl-vm --lib
run_naming_owner tcl-vm 'exec::native_uplevel_tests::' -p tcl-vm --lib
run_naming_owner tcl-vm 'cmd_namespace::' -p tcl-vm --lib
run_naming_owner tcl-vm 'cmd_package::' -p tcl-vm --lib
run_naming_owner tcl-vm 'cmd_trace::' -p tcl-vm --lib
run_naming_owner tcl-vm 'cmd_oo::' -p tcl-vm --lib
run_naming_owner tcl-vm 'cmd_info::native_commands_tests::' -p tcl-vm --lib
run_naming_owner tcl-vm 'value::native_' -p tcl-vm --lib
run_naming_owner tcl-runtime 'cmd_proc::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'interp::native_command_names::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'interp::native_namespace_names::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'interp::native_variable_names::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'interp::execution_name_policy::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'cmd_mathop::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'interp::native_jim_links::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'interp::native_procedure_relocation' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'interp::native_body_artifact::namespace_tests::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'interp::native_body_artifact::native_upvar_info_exists_tests::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'cmd_namespace::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'cmd_package::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'cmd_trace::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'cmd_oo::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'frame::' --manifest-path runtime/rust/Cargo.toml --lib
run_naming_owner tcl-runtime 'obj::native_' --manifest-path runtime/rust/Cargo.toml --lib
