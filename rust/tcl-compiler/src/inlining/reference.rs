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

//! Reference bodies — the definitions a pack's `runtime_backing` names for a
//! command, brought into a module so calls to the command can be inlined
//! (`docs/design/compiler/registry-consumer-contracts.md` § *Four rungs of
//! codegen meeting `.tclspec`*, rung 3).
//!
//! A definition is lowered on its own, as the text a runtime would `eval` to
//! create the procedure, so nothing the module around it says (its aliases, its
//! namespace imports) can change what it means. Its spans are then shifted past
//! the end of the module's source and the text appended there, so the module and
//! the definition share one text for every span ([`ReferenceBodies`]). The
//! definitions enter the module's procedure table under the rooted name the
//! definition creates; the inliner decides, as it does for a procedure the module
//! defines, which of the calls it may replace, and the module drops them again.

use std::collections::BTreeSet;

use tcl_lexer::{LexerConfig, Span};
use tcl_registry::CommandRegistry;
use tcl_runtime_api::BackingKind;

use crate::ir::{Module, Procedure, ReferenceImport, Statement};

/// Append and import the definition of every command a pack declares
/// `TclBody`-backed that `module` calls, returning the rooted names of the
/// procedures added to [`Module::procedures`].
///
/// A command is a candidate only when the registry holds its text and the pack
/// that declared it, so there are facts for a site to claim, and when it is the
/// registry's live answer for its name. A name the module defines itself is
/// never imported, so the module's own definition keeps precedence. A text that
/// is not exactly one `proc` definition of the command it backs is passed over,
/// as is one that would put a span past what an offset can hold.
pub(super) fn import(
    module: &mut Module,
    registry: &CommandRegistry,
    config: LexerConfig,
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> BTreeSet<crate::ir::ProcedureId> {
    let mut imported = BTreeSet::new();
    // Read the original callers before an appended body changes the image.
    let called = called_descriptors(module, registry);
    for (spec, text) in registry.reference_bodies() {
        if !called.contains(spec.name) {
            continue;
        }
        let Some(facts) = crate::site_claims::reference_body_facts(registry, spec) else {
            continue;
        };
        let Some(definition) = lower_definition(text, spec, registry, config, profile) else {
            continue;
        };
        let qualified = definition.qualified_name.clone();
        if module.procedures.has_display(&qualified)
            || module.redefined_procedures.contains(&qualified)
        {
            continue;
        }
        let Some(proc) = append(module, text, definition) else {
            continue;
        };
        module.reference_bodies.imports.push(ReferenceImport {
            name: qualified.clone(),
            parameters: proc.params_raw.clone(),
            body: proc.body_source.clone().unwrap_or_default(),
            backing: spec.runtime_backing.kind(),
            facts,
        });
        let id = module.retain_procedure(proc);
        imported.insert(id);
    }
    imported
}

/// Closed conditional Registry descriptor labels from the original module.
/// These are import candidates, not entered command or pack/admission proofs.
fn called_descriptors(module: &Module, registry: &CommandRegistry) -> BTreeSet<String> {
    if crate::registry_invocation::InvocationMetadataContext::for_module(registry, module).is_none()
    {
        return BTreeSet::new();
    }
    let mut labels = BTreeSet::new();
    let mut note = |statement: &Statement| {
        if !matches!(statement, Statement::Call { .. }) {
            return;
        }
        let Some(tokens) = statement.tokens() else {
            return;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return;
        };
        let Some(metadata) =
            binding.original_argument_metadata_for_module(tokens, module, registry)
        else {
            return;
        };
        let Some(advice) =
            crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                registry, Some(metadata), tokens,
            )
        else {
            return;
        };
        if let Some(words) = advice.unanimous_command_words() {
            labels.insert(words.command().to_owned());
        }
    };
    crate::ir::for_each_statement(&module.top_level, &mut note);
    for proc in module.procedures.values() {
        crate::ir::for_each_statement(&proc.body, &mut note);
    }
    labels
}

/// Lower the actual body and join its produced declaration key through the
/// Registry's descriptor lookup to this exact backing row. No descriptor
/// punctuation is treated as a constructed-key or Native naming contract.
fn lower_definition(
    text: &str,
    expected: &tcl_registry::CommandSpec,
    registry: &CommandRegistry,
    config: LexerConfig,
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<Procedure> {
    let mut lowered = crate::lowering::lower_script_module_for_bytecode(
        text, "", registry, config, profile, false,
    );
    if lowered.top_level.statements.len() != 1
        || lowered.procedures.len() != 1
        || !lowered.redefined_procedures.is_empty()
    {
        return None;
    }
    let id = *lowered.procedures.keys().next()?;
    let proc = lowered.procedures.remove(&id)?;
    let selected = registry.get(&proc.qualified_name)?;
    if !std::ptr::eq(selected, expected) {
        return None;
    }
    // Without the body's text there is nothing for the runtime to compare.
    proc.body_source.is_some().then_some(proc)
}

/// Append `text` to the module's source and shift `proc` to where it now stands.
fn append(module: &mut Module, text: &str, mut proc: Procedure) -> Option<Procedure> {
    let start = module.source.len() + 1;
    let end = start.checked_add(text.len())?;
    if u32::try_from(end).is_err() {
        return None;
    }
    let delta = i64::try_from(start).ok()?;
    crate::lattice_rebase::rebase_script(&mut proc.body, delta);
    proc.span = Span::new(
        proc.span.start().saturating_add(u32::try_from(start).ok()?),
        proc.span.end().saturating_add(u32::try_from(start).ok()?),
    );
    proc.body_offset = proc.body_offset.saturating_add(u32::try_from(start).ok()?);
    module
        .reference_bodies
        .appendix_start
        .get_or_insert(module.source.len());
    let mut source = module.source.bytes().to_vec();
    source.push(b'\n');
    source.extend_from_slice(text.as_bytes());
    module.source = tcl_lexer::SourceImage::from_bytes(source, module.source.channel());
    Some(proc)
}

/// The claim a function's procedure binding makes when it came from a
/// reference body: the binding itself, the backing the definition was declared
/// with, and the pack facts behind the declaration. `None` for a binding that
/// names no reference body — a procedure the module defines.
pub(crate) fn claim_for(
    imports: &[ReferenceImport],
    binding: &tcl_runtime_api::ProcedureBindingIdentity,
) -> Option<tcl_runtime_api::SiteClaim> {
    let import = imports.iter().find(|import| {
        import.name == binding.name
            && import.parameters == binding.parameters
            && import.body == binding.body
    })?;
    debug_assert_eq!(import.backing, BackingKind::TclBody);
    Some(tcl_runtime_api::SiteClaim::ReferenceBody {
        procedure: binding.clone(),
        backing: import.backing,
        facts: import.facts.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logical_module(source: &str, registry: &CommandRegistry) -> Module {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl9.0")
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            LexerConfig::for_file_grammar(profile.grammar),
        );
        let entry =
            crate::command_binding::SourceAnalysisEntry::for_logical_source(&input).unwrap();
        crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            &input,
        )
        .ir_module
    }

    #[test]
    fn original_reference_candidates_use_selected_labels_instead_of_source_tails() {

        // Conditional original source import candidates; no pack or Native admission.
        let registry = CommandRegistry::build_default();
        let selected = logical_module(
            "interp alias {} originalAlias {} string length; originalAlias VALUE",
            &registry,
        );
        assert!(called_descriptors(&selected, &registry).contains("string"));
        assert!(!called_descriptors(&selected, &registry).contains("originalAlias"));
        for source in [
            "other::string length VALUE",
            "namespace eval other {proc string args {}; string length VALUE}",
        ] {
            let module = logical_module(source, &registry);
            assert!(
                !called_descriptors(&module, &registry).contains("string"),
                "{source}"
            );
        }
        let mut missing = selected.clone();
        missing.source_metadata_input = None;
        assert!(called_descriptors(&missing, &registry).is_empty());
        let mut changed = selected.clone();
        changed.source = tcl_lexer::SourceImage::document("puts CHANGED");
        assert!(called_descriptors(&changed, &registry).is_empty());
        let mut foreign = CommandRegistry::build_default();
        foreign.insert(registry.get("string").unwrap().clone());
        assert!(called_descriptors(&selected, &foreign).is_empty());
    }

    #[test]
    fn reference_definition_uses_its_produced_key_and_exact_registry_row() {

        // Actual authored definition/source only; no body import or execution is granted.
        let registry = CommandRegistry::build_default();
        let selected = registry.get("string").unwrap();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let declaration = lower_definition(
            "proc string {x} {return $x}",
            selected,
            &registry,
            config,
            Some(profile),
        )
        .unwrap();
        assert_eq!(declaration.qualified_name, "::string");
        assert!(
            lower_definition(
                "proc other::string {x} {return $x}",
                selected,
                &registry,
                config,
                Some(profile),
            )
            .is_none()
        );
        assert!(
            lower_definition(
                "proc string {x} {return $x}; proc another {} {}",
                selected,
                &registry,
                config,
                Some(profile),
            )
            .is_none()
        );
        let mut foreign = CommandRegistry::build_default();
        foreign.insert(selected.clone());
        assert!(
            lower_definition(
                "proc string {x} {return $x}",
                foreign.get("string").unwrap(),
                &registry,
                config,
                Some(profile),
            )
            .is_none()
        );
    }
}
