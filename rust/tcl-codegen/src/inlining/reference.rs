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

use std::collections::{BTreeSet, HashMap};

use tcl_lexer::{LexerConfig, Span};
use tcl_registry::CommandRegistry;
use tcl_runtime_api::BackingKind;

use tcl_compiler::ir::{CommandTokens, Module, Procedure, ReferenceImport, Statement};

/// Where a call stands in the module's own text: the span of its command word.
pub(super) type SiteKey = (u32, u32);

/// The imported definition each call site of the module answers to.
pub(super) type Sites = HashMap<SiteKey, tcl_compiler::ir::ProcedureId>;

/// The key of the call `tokens` spells, `None` for one with no written word.
pub(super) fn site_key(tokens: &CommandTokens) -> Option<SiteKey> {
    let head = tokens.argv.first()?;
    Some((head.start(), head.end()))
}

/// What [`import`] brought into a module.
#[derive(Default)]
pub(super) struct Imported {
    /// The procedures added to [`Module::procedures`].
    pub(super) ids: BTreeSet<tcl_compiler::ir::ProcedureId>,
    /// What each splices in, proved in the module it was lowered in.
    pub(super) inlinable: HashMap<tcl_compiler::ir::ProcedureId, super::InlineSpec>,
    /// The calls of the module each answers.
    pub(super) sites: Sites,
}

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
) -> Imported {
    let mut imported = Imported::default();
    // Read the original callers before an appended body changes the image.
    let called = called_descriptors(module, registry);
    for (spec, text) in registry.reference_bodies() {
        let Some(sites) = called.get(spec.name) else {
            continue;
        };
        let Some(facts) = tcl_compiler::site_claims::reference_body_facts(registry, spec) else {
            continue;
        };
        // The definition is read as the module was: the same release and
        // metadata, and none of what the module's own text declares.
        let Some((definition, inlinable, _)) = lower_definition(
            text,
            spec,
            registry,
            config,
            profile,
            module.source_metadata_input.as_ref(),
        ) else {
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
        let inlinable = inlinable.map(|spec| spec.moved_to(&proc));
        let id = module.retain_procedure(proc);
        imported.ids.insert(id);
        if let Some(spec) = inlinable {
            imported.inlinable.insert(id, spec);
            imported.sites.extend(sites.iter().map(|site| (*site, id)));
        }
    }
    imported
}

/// Closed conditional Registry descriptor labels from the original module.
/// These are import candidates, not entered command or pack/admission proofs.
fn called_descriptors(
    module: &Module,
    registry: &CommandRegistry,
) -> HashMap<String, Vec<SiteKey>> {
    if tcl_compiler::registry_invocation::InvocationMetadataContext::for_module(registry, module)
        .is_none()
    {
        return HashMap::new();
    }
    let mut labels: HashMap<String, Vec<SiteKey>> = HashMap::new();
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
            tcl_compiler::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                registry, Some(metadata), tokens,
            )
        else {
            return;
        };
        if let Some(words) = advice.unanimous_command_words() {
            let sites = labels.entry(words.command().to_owned()).or_default();
            // A site is the command as written: a call reached through a
            // prefix that adds words is not the definition's argument list.
            if words.effective().binding_prefix.is_empty()
                && words.effective().words.get(1..) == tokens.words().get(1..)
                && let Some(site) = site_key(tokens)
            {
                sites.push(site);
            }
        }
    };
    tcl_compiler::ir::for_each_statement(&module.top_level, &mut note);
    for proc in module.procedures.values() {
        tcl_compiler::ir::for_each_statement(&proc.body, &mut note);
    }
    labels
}

/// Lower the actual body and join its produced declaration key through the
/// Registry's descriptor lookup to this exact backing row. No descriptor
/// punctuation is treated as a constructed-key or Native naming contract.
/// A definition as a fresh interpreter of the release reads it: the procedure,
/// what it splices in, and whether its body may be run again from its start.
pub(super) type Definition = (
    Procedure,
    Option<super::InlineSpec>,
    Result<(), tcl_compiler::command_binding::ReExecutionDecline>,
);

pub(super) fn lower_definition(
    text: &str,
    expected: &tcl_registry::CommandSpec,
    registry: &CommandRegistry,
    config: LexerConfig,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    input: Option<&tcl_compiler::analyser::ResolvedAnalysisInput>,
) -> Option<Definition> {
    let mut lowered = match input {
        // A fresh interpreter of the caller's release evaluating the text.
        Some(input) => {
            let entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
                registry, input, config, profile,
            );
            let mut options = entry.options();
            options.compilation_scope = tcl_runtime_api::SourceCompilationScope::WholeModule;
            let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(registry, config)
                .with_dialect(profile)
                .for_bytecode_backend()
                .with_resolved_analysis_input(input.clone());
            lowerer.set_source_analysis_options(options);
            lowerer.lower_script_target(text, "");
            lowerer.finish_module(text)
        }
        None => tcl_compiler::lowering::lower_script_module_for_bytecode(
            text, "", registry, config, profile, false,
        ),
    };
    if lowered.top_level.statements.len() != 1
        || lowered.procedures.len() != 1
        || !lowered.redefined_procedures.is_empty()
    {
        return None;
    }
    let id = *lowered.procedures.keys().next()?;
    let inlinable = super::reference_inline_spec(&lowered, registry);
    let re_execution = tcl_compiler::command_binding::re_executable_body(
        &lowered,
        &lowered.procedures.get(&id)?.body,
        registry,
    );
    let proc = lowered.procedures.remove(&id)?;
    let selected = registry.get(&proc.qualified_name)?;
    if !std::ptr::eq(selected, expected) {
        return None;
    }
    // Without the body's text there is nothing for the runtime to compare.
    proc.body_source
        .is_some()
        .then_some((proc, inlinable, re_execution))
}

/// Append `text` to the module's source and shift `proc` to where it now stands.
fn append(module: &mut Module, text: &str, mut proc: Procedure) -> Option<Procedure> {
    let (start, image) = appended(module, text)?;
    tcl_compiler::lattice_rebase::rebase_script(&mut proc.body, i64::from(start));
    shift_definition(&mut proc, start);
    commit(module, image);
    Some(proc)
}

/// Where `text` starts once appended to the module's source, and the source
/// it makes; `None` when a span past it would not fit an offset.
pub(super) fn appended(module: &Module, text: &str) -> Option<(u32, tcl_lexer::SourceImage)> {
    let start = module.source.len() + 1;
    u32::try_from(start.checked_add(text.len())?).ok()?;
    let mut source = module.source.bytes().to_vec();
    source.push(b'\n');
    source.extend_from_slice(text.as_bytes());
    Some((
        u32::try_from(start).ok()?,
        tcl_lexer::SourceImage::from_bytes(source, module.source.channel()),
    ))
}

/// Shift a definition's own span and body offset, not its body, by `start`.
pub(super) fn shift_definition(proc: &mut Procedure, start: u32) {
    proc.span = Span::new(
        proc.span.start().saturating_add(start),
        proc.span.end().saturating_add(start),
    );
    proc.body_offset = proc.body_offset.saturating_add(start);
}

/// Make `image`, made by [`appended`], the module's source.
pub(super) fn commit(module: &mut Module, image: tcl_lexer::SourceImage) {
    module
        .reference_bodies
        .appendix_start
        .get_or_insert(module.source.len());
    module.source = image;
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
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            LexerConfig::for_file_grammar(profile.grammar),
        );
        let entry =
            tcl_compiler::command_binding::SourceAnalysisEntry::for_logical_source(&input).unwrap();
        tcl_compiler::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            tcl_compiler::compilation_unit::UnitBuildOptions {
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
        // naming.inlining.original-reference-descriptor-selection
        // docs/design/analysis/name-resolution-proofs/inlining-original-reference-descriptor-selection.md
        // Conditional original source import candidates; no pack or Native admission.
        let registry = CommandRegistry::build_default();
        let selected = logical_module(
            "interp alias {} originalAlias {} string length; originalAlias VALUE",
            &registry,
        );
        assert!(called_descriptors(&selected, &registry).contains_key("string"));
        assert!(!called_descriptors(&selected, &registry).contains_key("originalAlias"));
        for source in [
            "other::string length VALUE",
            "namespace eval other {proc string args {}; string length VALUE}",
        ] {
            let module = logical_module(source, &registry);
            assert!(
                !called_descriptors(&module, &registry).contains_key("string"),
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
        // naming.inlining.original-reference-descriptor-selection
        // docs/design/analysis/name-resolution-proofs/inlining-original-reference-descriptor-selection.md
        // Actual authored definition/source only; no body import or execution is granted.
        let registry = CommandRegistry::build_default();
        let selected = registry.get("string").unwrap();
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let (declaration, _, _) = lower_definition(
            "proc string {x} {return $x}",
            selected,
            &registry,
            config,
            Some(profile),
            None,
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
                None,
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
                None,
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
                None,
            )
            .is_none()
        );
    }

    #[test]
    fn a_definition_is_re_executable_only_where_its_declared_effects_stay_in_its_frame() {
        use tcl_compiler::command_binding::ReExecutionDecline as Decline;
        use tcl_compiler::math_function_binding::OpenExpression as Open;
        let registry = CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::compile_service::fresh_document_input(&registry, config, profile)
            .expect("a fresh reading of tcl9.0");
        // The definition takes a name none of the bodies calls.
        let selected = registry.get("format").unwrap();
        for (body, verdict) in [
            ("string toupper $x", Ok(())),
            ("lindex $x 0", Ok(())),
            ("set y $x; incr y; return $y", Ok(())),
            ("return \"$x-$x\"", Ok(())),
            ("puts $x", Err(Decline::Callback)),
            // Whether an `expr` runs more than its arithmetic is a fact of
            // its site: its operands are the frame's own, and it calls only a
            // function the registry declares closed.
            ("expr {$x * 2}", Ok(())),
            ("set y 2; expr {$x * $y}", Ok(())),
            ("return [expr {$x * 2}]", Ok(())),
            ("expr {abs($x)}", Ok(())),
            (
                "expr {[string length $x] * 2}",
                Err(Decline::Expression(Open::CommandSubstitution)),
            ),
            (
                "expr {nosuch($x)}",
                Err(Decline::Expression(Open::UnresolvedFunction)),
            ),
            // A variable another can name may hold a read trace.
            (
                "expr {$::g * 2}",
                Err(Decline::Expression(Open::ForeignOperand)),
            ),
            (
                "expr {$a(1) * 2}",
                Err(Decline::Expression(Open::ForeignOperand)),
            ),
            (
                "expr {\"$x\" eq {a}}",
                Err(Decline::Expression(Open::OpenRead)),
            ),
            // A structured `if` evaluates its conditions and runs its arms,
            // each answered at its own site.
            ("if {$x < 0} {set r neg} else {set r pos}", Ok(())),
            (
                "if {[string length $x]} {set r a} else {set r b}",
                Err(Decline::Expression(Open::CommandSubstitution)),
            ),
            (
                "if {$::g} {set r a} else {set r b}",
                Err(Decline::Expression(Open::ForeignOperand)),
            ),
            ("if {$x < 0} {puts $x}", Err(Decline::Callback)),
            // Unbraced, the words are substituted before `expr` parses them.
            ("expr $x", Err(Decline::Callback)),
            ("set ::g $x", Err(Decline::ForeignVariables)),
            // `global` declares a link: a state transition of the frame.
            ("global g; set g $x", Err(Decline::StateTransition)),
            // A completion other than normal is re-executable where nothing
            // else is declared: running `break` again breaks again.
            ("break", Ok(())),
            ("continue", Ok(())),
            (
                "return -code error -errorcode {VENDOR BAD} $x",
                Err(Decline::StateTransition),
            ),
            (
                "error $x {custom info} {VENDOR ERR}",
                Err(Decline::Callback),
            ),
            ("return -level 2 $x", Err(Decline::IncompleteRoles)),
        ] {
            let (_, _, re_execution) = lower_definition(
                &format!("proc format {{x}} {{{body}}}"),
                selected,
                &registry,
                config,
                Some(profile),
                Some(&input),
            )
            .unwrap_or_else(|| panic!("{body}"));
            assert_eq!(re_execution, verdict, "{body}");
        }
    }
}
