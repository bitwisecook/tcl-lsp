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
cargo test -p tcl-syntax --test command_resolution_conformance --test variable_resolution_conformance --test namespace_op_conformance --test execution_resolution_conformance --test body_execution_conformance
cargo test -p tcl-compiler --test resolution_rewrite_oracles --test native_arithmetic_conformance --test codegen_integration
cargo test -p tcl-vm --test native_compilation_conformance --test cross_interp_alias_e2e --test legacy_variable_traces_e2e --test command_traces_e2e --test step_trace_inline_e2e --test variable_name_resolution_e2e --test variable_trace_semantics_e2e --test element_recovery_traces_e2e
cargo test -p tcl-registry --test dialect_oracle --test package_protocol_oracle
cargo test -p tcl-dialect --test package_version_oracle
cargo test --manifest-path runtime/rust/Cargo.toml --test array_trace_oracle
