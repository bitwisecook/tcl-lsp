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

//! Analyser tests that need a crate above `tcl-compiler`: pack-declared
//! command resolution (`command_resolution`), and the analyser's own test
//! bodies that need Jim's core surface. Helpers copied from
//! `tcl-compiler/tests/it_4/analyser.rs`.

use tcl_compiler::analyser::{Analyser, Severity};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::run_all_checks;
use tcl_registry::model::ingress::static_context_for;

mod command_resolution;

/// Default dialect for reproducers that are not dialect-sensitive.
const D: &str = "tcl8.6";

/// Every diagnostic code the full pipeline surfaces for `src` under `dialect`,
/// mirroring the user-facing `tcl diag` path: the analyser pass plus the
/// `run_all_checks` compiler-checks pass (shimmer / taint / dead-store), with
/// optimisation codes excluded exactly as `diag` excludes them.
///
/// Copied verbatim from `src/analyser/diagnostics/fp/mod.rs::codes`.
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    for d in run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True if `code` appears anywhere in the full diagnostic set for `src`.
fn fires(src: &str, dialect: &str, code: &str) -> bool {
    codes(src, dialect).iter().any(|c| c == code)
}

/// Full `(code, message)` pairs from the *analyser pass only* — used when a
/// test pins a message substring or message-scoped count (the analyser
/// pass owns every code those tests inspect: W210/W211/W213/W214/W215/W216/
/// W123/E001/E002/E003/W001).
fn analyser_diags(src: &str, dialect: &str) -> Vec<(String, String, Severity)> {
    Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| (d.code.to_string(), d.message.clone(), d.severity))
        .collect()
}

mod commands {
    mod original_logical_jim_body_roles_tests {
        spectcl_seeded_tests!(tcl_compiler::analyser::commands::original_logical_jim_body_roles_tests:
            selected_logical_if_body_retains_original_jim_class_metadata,
            replaced_logical_handler_and_inert_data_do_not_donate_body_roles,
        );
    }
}

mod handlers {
    mod finite_foreach_source {
        mod controls {
            spectcl_seeded_tests!(tcl_compiler::analyser::handlers::finite_foreach_source_controls:
                literal_jim_iterations_keep_readonly_list_lineage_and_source_names,
                iteration_names_refuse_equal_foreign_owner_changed_source_and_namespace,
                iteration_plan_refuses_mutation_multiple_bindings_expansion_and_name_prefixes,
                reporting_scope_restores_after_unwind_and_preserves_body_depth_refusal,
            );
        }
    }

    mod original_proc_source_layout_tests {
        spectcl_seeded_tests!(tcl_compiler::analyser::handlers::original_proc_source_layout_tests:
            original_jim_source_layout_keeps_statics_and_actual_body_in_shared_registry,
            original_procedure_source_layout_refuses_foreign_values_tokens_and_expansion,
            original_procedure_source_layout_keeps_plain_and_rooted_jim_forms,
        );
    }
}
