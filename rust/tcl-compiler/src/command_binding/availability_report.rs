// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Complete binding equality for the separate availability-report purpose.

use super::SourceInvocationBinding;

impl SourceInvocationBinding {
    pub(crate) fn authenticated_availability_report_eq(
        &self,
        other: &Self,
        output: &crate::analyser::authenticated_report::ReportOutputCorrespondence<'_>,
    ) -> Option<()> {
        if !output.analysis_owners(
            self.original_analysis_owner.as_ref()?,
            other.original_analysis_owner.as_ref()?,
        ) {
            return None;
        }
        // Every other typed field remains exact. A future field must fail this
        // exhaustive pattern rather than silently becoming an ignored facet.
        macro_rules! same_fields {
            ($($field:ident),+ $(,)?) => {
                let Self { original_analysis_owner: _, $($field: _),+ } = self;
                $(if self.$field != other.$field { return None; })+
            };
        }
        same_fields!(
            compiled_candidates,
            compiled_named_candidates,
            may_use_live_dispatch,
            compiled_execution_residual,
            native_compilation_admission,
            native_compiler_admission,
            original_compiler_preserves_names,
            native_structured_preparation,
            native_switch_preparation,
            runtime_reachability,
            entered_execution_observer,
            native_inline_rejection,
            native_handler_envelope,
            invocation_variable_reads,
            lookup_state,
            compiler_lookup_state,
            original_logical_source_advice,
            original_source_operand_attachment,
            original_authored_source_body_scope,
            original_native_source_command,
            declaration_layout_observations,
            declaration_flow_inventory,
            original_logical_declaration_targets,
            compiler_policy,
            original_compiler_words,
            original_written_words,
            original_argument_syntax,
            conditional_expression_evaluations,
            lookup_namespace,
            lookup_namespace_key,
            lookup_word,
            dispatch_site,
            provider_state_taint,
            declared_command,
            catalogue_command,
            variable_context,
            original_argument_completion,
            normal_variable_continuation,
            original_variable_operands,
            normal_result_observations,
            rhs_read_store_observations,
            object_callback_effects,
            variable_frame,
            existing_namespace_cells,
            namespace_cell_presence,
            known_namespaces,
            evaluated_argument_values,
            evaluated_argument_words,
            frozen_written_words,
            frozen_written_names,
            frozen_head_object,
            method_prefix_arguments,
            native_operand_layout,
            targets,
            may_be_absent,
            unknown,
        );
        Some(())
    }
}
