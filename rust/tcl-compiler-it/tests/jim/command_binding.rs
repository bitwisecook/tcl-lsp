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

//! The bodies run here are in `tcl_compiler::command_binding` behind `test-support`.

mod called_static_layouts {
    mod tests {
        spectcl_seeded_tests!(tcl_compiler::command_binding::spectcl_test_bodies::called_static_layouts:
            called_static_body_keeps_a_separate_original_declaration_layout,
            called_static_layout_collection_does_not_publish_results_or_entered_inventories,
            static_layout_collection_does_not_reinterpret_other_callable_purposes,
        );
    }
}

mod declaration_preview {
    mod static_read_child {
        mod tests {
            spectcl_seeded_tests!(tcl_compiler::command_binding::spectcl_test_bodies::declaration_preview_static_read_child:
                original_nested_named_static_read_keeps_original_positive_expectations,
                original_nested_named_read_refuses_alias_rebinding_callee_and_opaque_children,
                original_nested_named_read_refuses_equal_foreign_and_stale_source_owners,
                original_nested_read_rejects_a_child_with_a_foreign_frame,
                original_child_body_keeps_its_issued_frame_distinct_from_header_layout,
                original_child_body_frame_refuses_missing_reissued_and_foreign_rows,
                original_child_body_frame_refuses_different_allocation_and_missing_formals,
                known_child_projection_uses_live_producer_and_separate_body_scope,
                known_child_projection_refuses_consistently_replaced_producer_fields,
                known_child_projection_refuses_foreign_and_retired_source_owner,
            );
        }
    }
}

mod jim_stock_class {
    mod reference_prefix {
        mod tests {
            spectcl_seeded_tests!(tcl_compiler::command_binding::spectcl_test_bodies::jim_stock_class_reference_prefix:
                global_incoming_values_cannot_donate_reference_freshness_through_the_memo,
            );
        }
    }

    mod source_advice {
        mod controls {
            spectcl_seeded_tests!(tcl_compiler::command_binding::spectcl_test_bodies::jim_stock_class_source_advice:
                stock_source_effects_need_the_actual_analysis_owner_and_complete_publications,
                stock_source_effects_keep_unknown_operations_and_changed_dependencies_terminal,
            );
        }
    }

    mod tests {
        spectcl_seeded_tests!(tcl_compiler::command_binding::spectcl_test_bodies::jim_stock_class:
            empty_base_class_retains_every_stock_publication_without_native_entry,
            earlier_unknown_and_changed_dependencies_do_not_recover_a_stock_world,
            fresh_stock_method_retains_original_source_publication_without_entry,
            stock_method_publication_keeps_missing_and_replaced_owners_opaque,
            one_stock_parent_retains_alias_overwrite_allocations_and_variable_owners,
            former_single_base_method_refusal_has_its_original_publication_owner,
            extra_parent_commands_and_changed_parent_helpers_keep_inheritance_opaque,
        );
    }
}

mod jim_wrapper_body {
    mod scope_body {
        mod tests {
            spectcl_seeded_tests!(tcl_compiler::command_binding::spectcl_test_bodies::jim_wrapper_body_scope_body:
                original_single_parent_definition_prefix_issues_first_entry_absence,
                prior_body_effects_substitutions_and_generated_heads_remain_unknown,
                unknown_history_overrides_and_conflicting_links_do_not_issue_absence,
            );
        }
    }

    mod tests {
        spectcl_seeded_tests!(tcl_compiler::command_binding::spectcl_test_bodies::jim_wrapper_body:
            jim_wrapper_body_keeps_a_distinct_opaque_future_world,
            jim_wrapper_namespace_follows_both_published_procedure_keys,
            jim_wrapper_does_not_borrow_computed_or_recooked_body_geometry,
        );
    }
}

mod source_transition_advice {
    mod source_jim_member {
        mod tests {
            spectcl_seeded_tests!(tcl_compiler::command_binding::spectcl_test_bodies::source_transition_advice_source_jim_member:
                own_jim_member_keeps_before_and_after_factory_custody,
                own_jim_member_refuses_foreign_and_corrupt_recipes,
                own_jim_member_refuses_rebound_class_and_replaced_initialisers,
            );
        }
    }
}
