// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exhaustive full-output policy. Authority-bearing facets stay strict unless
//! a distinct actual producer correspondence is explicitly supplied below.
use super::body_correspondence::ReportBodyCorrespondence;
use super::{AnalysisReportComparison, AnalysisReportUnavailable};
use crate::analyser::{AnalysisResult, DiagnosticSubject};
use tcl_lexer::SourceImage;
use tcl_registry::RegistrySemanticKey;

fn subjects(
    a: &DiagnosticSubject,
    b: &DiagnosticSubject,
    output: &super::ReportOutputCorrespondence<'_>,
) -> Option<bool> {
    // Every variant is explicit. Distinct unjoined authority stays unavailable.
    match a {
        DiagnosticSubject::PrivateTclNamespace(a) => match b {
            DiagnosticSubject::PrivateTclNamespace(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::UserSourceArity(a) => match b {
            DiagnosticSubject::UserSourceArity(b) => a.authenticated_report_eq(b, output),
            _ => None,
        },
        DiagnosticSubject::CallbackSourceArity(a) => match b {
            DiagnosticSubject::CallbackSourceArity(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::ObjectSourceArity(a) => match b {
            DiagnosticSubject::ObjectSourceArity(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::CommandAvailability(a) => match b {
            DiagnosticSubject::CommandAvailability(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::DeclaredSource(a) => match b {
            DiagnosticSubject::DeclaredSource(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::RegisteredInstanceSource(a) => match b {
            DiagnosticSubject::RegisteredInstanceSource(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::RegistrySource(a) => match b {
            DiagnosticSubject::RegistrySource(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::ConditionalInterpreterVisibility(a) => match b {
            DiagnosticSubject::ConditionalInterpreterVisibility(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::RequiredPackage(a) => match b {
            DiagnosticSubject::RequiredPackage(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::UnresolvedCommand(a) => match b {
            DiagnosticSubject::UnresolvedCommand(b) if a == b => Some(true),
            _ => None,
        },
        DiagnosticSubject::UnresolvedMathFunction(a) => match b {
            DiagnosticSubject::UnresolvedMathFunction(b) if a == b => Some(true),
            _ => None,
        },
    }
}

pub(super) fn compare(
    a: &AnalysisResult,
    b: &AnalysisResult,
    bodies: &ReportBodyCorrespondence<'_>,
    image: &SourceImage,
    registry: &RegistrySemanticKey,
    output: &super::ReportOutputCorrespondence<'_>,
) -> AnalysisReportComparison {
    // Exhaustive inventory, including every non-diagnostic report field.
    let AnalysisResult {
        optimiser_source_origin: _,
        analysis_context_unavailable: _,
        command_realm: _,
        original_command_world: _,
        original_source_region: _,
        lexical_declaration_advice: _,
        resolved_input: _,
        original_vendor_source_names: _,
        original_vendor_variable_advice: _,
        original_vendor_variable_bodies: _,
        global_scope: _,
        all_procs: _,
        original_procedure_metadata: _,
        original_vendor_procedure_metadata: _,
        original_vendor_class_metadata: _,
        original_symbol_metadata: _,
        original_vendor_symbol_metadata: _,
        proc_declaration_sites: _,
        superseded_procs: _,
        all_classes: _,
        logical_procedure_declarations: _,
        logical_class_declarations: _,
        original_class_metadata: _,
        original_class_configuration_metadata: _,
        original_object_configuration_metadata: _,
        superseded_classes: _,
        class_body_spans: _,
        all_variables: _,
        all_defined_symbols: _,
        diagnostics: _,
        command_invocations: _,
        package_requires: _,
        package_provides: _,
        package_ifneededs: _,
        package_prefer_latest: _,
        has_dynamic_providers: _,
        source_targets: _,
        path_constant_assignments: _,
        command_aliases: _,
        alias_offsets: _,
        renamed_commands: _,
        rename_offsets: _,
        ensemble_subcommand_targets: _,
        prefixless_ensembles: _,
        namespace_imports: _,
        namespace_forgets: _,
        original_namespace_patterns: _,
        destroyed_commands: _,
        namespace_exports: _,
        original_namespace_exports: _,
        original_namespace_export_unknowns: _,
        namespace_paths: _,
        namespace_path_computed: _,
        auto_path_entries: _,
        qualified_var_refs: _,
        original_variable_symbols: _,
        original_variable_write_advice: _,
        original_variable_symbol_conflicts: _,
        original_variable_roots: _,
        original_variable_alias_sites: _,
        original_variable_name_unknowns: _,
        original_variable_alias_obligations: _,
        original_variable_alias_operands: _,
        original_variable_alias_source_operands: _,
        original_variable_alias_receipts: _,
        namespace_refs: _,
        namespace_name_unknowns: _,
        dynamic_variable_names: _,
        namespace_overrides: _,
        stub_commands: _,
        stub_expr_defs: _,
        regex_patterns: _,
        suppressed_lines: _,
        unknown_proc_info: _,
        instance_classes: _,
        object_handle_facts: _,
        created_instance_commands: _,
        instance_command_bindings: _,
        ambiguous_instance_names: _,
        object_methods: _,
        object_member_state: _,
        unresolved_command_sites: _,
        scoped_command_regions: _,
        original_scoped_bodies: _,
        original_conditional_registry_metadata: _,
        scoped_sibling_defs: _,
        hierarchy_cache: _,
        dialect: _,
        body_lexer_config: _,
        library_versions: _,
    } = a;
    // Strict typed equality can prove these facets unchanged. If it cannot,
    // there is no implicit replacement of a nested issuer or authority.
    macro_rules! require_exact_owners {
        ($($field:ident),* $(,)?) => { $(
            if a.$field != b.$field {
                return AnalysisReportComparison::Unavailable(
                    AnalysisReportUnavailable::CompleteFieldCorrespondenceUnavailable);
            }
        )* };
    }
    require_exact_owners!(
        optimiser_source_origin,
        analysis_context_unavailable,
        command_realm,
        original_command_world,
        original_source_region,
        lexical_declaration_advice,
        resolved_input,
        original_vendor_source_names,
        original_vendor_variable_advice,
        original_vendor_variable_bodies,
        global_scope,
        all_procs,
        original_procedure_metadata,
        original_vendor_procedure_metadata,
        original_vendor_class_metadata,
        original_symbol_metadata,
        original_vendor_symbol_metadata,
        proc_declaration_sites,
        superseded_procs,
        all_classes,
        logical_procedure_declarations,
        logical_class_declarations,
        original_class_metadata,
        original_class_configuration_metadata,
        original_object_configuration_metadata,
        superseded_classes,
        class_body_spans,
        all_variables,
        all_defined_symbols,
        command_invocations,
        package_requires,
        package_provides,
        package_ifneededs,
        package_prefer_latest,
        source_targets,
        path_constant_assignments,
        command_aliases,
        alias_offsets,
        renamed_commands,
        rename_offsets,
        ensemble_subcommand_targets,
        prefixless_ensembles,
        namespace_imports,
        namespace_forgets,
        original_namespace_patterns,
        destroyed_commands,
        namespace_exports,
        original_namespace_exports,
        original_namespace_export_unknowns,
        namespace_paths,
        namespace_path_computed,
        auto_path_entries,
        qualified_var_refs,
        original_variable_symbols,
        original_variable_write_advice,
        original_variable_symbol_conflicts,
        original_variable_roots,
        original_variable_alias_sites,
        original_variable_name_unknowns,
        original_variable_alias_obligations,
        original_variable_alias_operands,
        original_variable_alias_source_operands,
        original_variable_alias_receipts,
        namespace_refs,
        namespace_name_unknowns,
        dynamic_variable_names,
        namespace_overrides,
        stub_commands,
        stub_expr_defs,
        regex_patterns,
        unknown_proc_info,
        instance_classes,
        created_instance_commands,
        instance_command_bindings,
        ambiguous_instance_names,
        object_methods,
        object_member_state,
        unresolved_command_sites,
        scoped_command_regions,
        original_scoped_bodies,
        original_conditional_registry_metadata,
        scoped_sibling_defs,
        hierarchy_cache,
    );
    // Authenticate every typed body/candidate reference before any Difference.
    let Some(input) = a.resolved_input.as_ref() else {
        return AnalysisReportComparison::Unavailable(
            AnalysisReportUnavailable::CurrentOwnerUnavailable,
        );
    };
    let Some(object_facts_equal) = a.object_handle_facts.authenticated_full_report_eq(
        &b.object_handle_facts,
        bodies,
        image,
        input,
        registry,
        output,
    ) else {
        return AnalysisReportComparison::Unavailable(
            AnalysisReportUnavailable::CompleteFieldCorrespondenceUnavailable,
        );
    };
    if a.diagnostics.len() != b.diagnostics.len() {
        return AnalysisReportComparison::Unavailable(
            AnalysisReportUnavailable::CompleteFieldCorrespondenceUnavailable,
        );
    }
    let mut diagnostics_equal = true;
    for (a, b) in a.diagnostics.iter().zip(&b.diagnostics) {
        // A new Diagnostic facet must also be given an explicit policy.
        let crate::analyser::Diagnostic {
            subject: _,
            code: _,
            span: _,
            message: _,
            severity: _,
            fixes: _,
        } = a;
        let subject_equal = match (&a.subject, &b.subject) {
            (None, None) => true,
            (Some(a), Some(b)) => match subjects(a, b, output) {
                Some(equal) => equal,
                None => {
                    return AnalysisReportComparison::Unavailable(
                        AnalysisReportUnavailable::CompleteFieldCorrespondenceUnavailable,
                    );
                }
            },
            _ => {
                return AnalysisReportComparison::Unavailable(
                    AnalysisReportUnavailable::CompleteFieldCorrespondenceUnavailable,
                );
            }
        };
        diagnostics_equal &= subject_equal
            && a.code == b.code
            && a.span == b.span
            && a.message == b.message
            && a.severity == b.severity
            && a.fixes == b.fixes;
    }
    // Only plain value facets and fully authenticated compared data may issue
    // Difference. Missing correspondence above is always terminal.
    if a.has_dynamic_providers != b.has_dynamic_providers {
        return AnalysisReportComparison::Different {
            field: "has_dynamic_providers",
        };
    }
    if a.suppressed_lines != b.suppressed_lines {
        return AnalysisReportComparison::Different {
            field: "suppressed_lines",
        };
    }
    if a.dialect != b.dialect {
        return AnalysisReportComparison::Different { field: "dialect" };
    }
    if a.body_lexer_config != b.body_lexer_config {
        return AnalysisReportComparison::Different {
            field: "body_lexer_config",
        };
    }
    if a.library_versions != b.library_versions {
        return AnalysisReportComparison::Different {
            field: "library_versions",
        };
    }
    if !diagnostics_equal {
        return AnalysisReportComparison::Different {
            field: "diagnostics",
        };
    }
    if !object_facts_equal {
        return AnalysisReportComparison::Different {
            field: "object_handle_facts",
        };
    }
    AnalysisReportComparison::Equal
}
