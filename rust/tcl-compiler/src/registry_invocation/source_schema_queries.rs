// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Owned source-schema projections with independent invocation lifetimes.
//!
//! The original schema closures are higher-ranked over their borrowed argv.
//! These free function items retain that rank, unlike an associated method
//! selected from a lifetime-specialised `ResolvedInvocation` impl. They issue
//! exactly the original projection; source ownership and refusal stay with the
//! caller's existing `with_schema` or `with_source_schema` boundary.

use tcl_registry::ResolvedInvocation;

pub(crate) fn facts(schema: &ResolvedInvocation<'_, '_>) -> tcl_registry::InvocationFacts {
    schema.facts()
}

pub(crate) fn index_bounds(
    schema: &ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::SourceIndexBoundsInvocation> {
    schema.authored_source_index_bounds()
}

pub(crate) fn credential_arguments(
    schema: &ResolvedInvocation<'_, '_>,
) -> Vec<tcl_registry::AuthoredSourceCredentialArgument> {
    schema.authored_source_credential_arguments()
}

pub(crate) fn case_pattern_layout(
    schema: &ResolvedInvocation<'_, '_>,
) -> Option<(
    tcl_registry::CaseListSpec,
    tcl_registry::spec::CaseInvocation,
)> {
    schema.authored_source_case_pattern_layout()
}

pub(crate) fn substitutions_performed(
    schema: &ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::substitution::SubstitutionKinds> {
    schema.substitutions_performed()
}

pub(crate) fn channel_configuration(
    schema: &ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::AuthoredSourceChannelConfiguration> {
    schema.authored_source_channel_configuration()
}

pub(crate) fn lambda_call(
    schema: &ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::resolved_invocation::AuthoredSourceLambdaCall> {
    schema.authored_source_lambda_call()
}

pub(crate) fn package_query(
    schema: &ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::source_navigation::SourcePackageQueryInvocation> {
    schema.authored_source_package_query()
}

pub(crate) fn state_transitions(
    schema: &ResolvedInvocation<'_, '_>,
) -> tcl_registry::StateTransitions {
    schema.state_transitions()
}

pub(crate) fn clause_plan(
    schema: &ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::clause_grammar::ClausePlan> {
    schema.clause_plan()
}
