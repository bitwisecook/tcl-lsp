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

//! The bodies run here are in `tcl_compiler::analyser` behind `test-support`.

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
