// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Genuine declaration contracts enter the unchanged source-policy owner.

use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::{
    SourceAnalysisEntry, SourceCommandBindings, SourceDeclaredCommandContracts,
};

pub(crate) fn document_realm_with_declared_contracts(
    source: &str,
    input: &ResolvedAnalysisInput,
    entry: Option<&SourceAnalysisEntry>,
    declarations: &SourceDeclaredCommandContracts,
) -> super::CommandBindingRealm {
    // naming.source.original-declared-command-word-contract
    // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
    let context = input.context_registry();
    let realm = if let Some(entry) = entry {
        let options = declarations.with_options(entry.options());
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            input.lexer_config(),
            context.commands(),
            options,
        );
        super::realm_from_source_bindings(bindings, context.commands())
    } else if input.has_hosted_source_name_context() {
        super::document_vendor_realm_with_resolved_input(source, input, Some(declarations))
    } else if input.has_logical_source_name_context() {
        super::document_lexical_realm_with_resolved_input(source, input, Some(declarations))
    } else {
        super::document_realm_bindings_with_resolved_input(source, input, Some(declarations))
    };
    realm.with_resolved_analysis_input(input.clone())
}
