// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Declared Logical formal bindings, separate from entered cells and values.

use super::OriginalSourceScriptBody;
use super::source_structure::{
    OriginalRegistrySource, OriginalRegistryWords, original_logical_registry_words_for_command,
};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::realm::CommandBindingRealm;
use std::sync::Arc;
use tcl_lexer::{ExecutablePart, NativeScriptCommandWords, NativeWord, SourceImage, Span};
use tcl_registry::{ArgRole, CommandBindingDefinitionKind, CommandBindingTransition, Traits};

const MAX_COMMANDS: usize = 2048;

/// One original scalar read in its declared Logical procedure scope. The
/// complete word retains the executable arena that issued the name extent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalFormalReference {
    word: NativeWord,
    name: Span,
}
impl OriginalLogicalFormalReference {
    /// Complete original executable word that owns this scalar-name extent.
    #[must_use]
    pub const fn original_word(&self) -> &NativeWord {
        &self.word
    }
    /// Actual variable-name extent, excluding substitution delimiters.
    #[must_use]
    pub const fn name_span(&self) -> Span {
        self.name
    }
}

/// One unique declared formal, with exact list-child and original read spans.
/// This is lexical binding identity, never a current cell or a quiet value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalFormalBinding {
    name: String,
    declaration: Span,
    references: Vec<OriginalLogicalFormalReference>,
}
impl OriginalLogicalFormalBinding {
    /// Unchanged Unicode scalar name selected by the original formal-list grammar.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Exact original first field of the formal specifier.
    #[must_use]
    pub const fn declaration_span(&self) -> Span {
        self.declaration
    }
    /// All retained scalar reads of this declared binding.
    #[must_use]
    pub fn references(&self) -> &[OriginalLogicalFormalReference] {
        &self.references
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingClosure {
    Unavailable,
    Closed,
}

/// Original Logical procedure scope, formals and reads. Source correspondence
/// remains independent of alpha-renaming closure and the caller's isolated
/// authoring policy. No Native naming, activation, observer or Normal follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalProcedureBindings {
    declaration: Arc<OriginalRegistryWords>,
    parameters: NativeWord,
    body: Arc<OriginalSourceScriptBody>,
    input: ResolvedAnalysisInput,
    formals: Vec<OriginalLogicalFormalBinding>,
    closure: BindingClosure,
}
impl OriginalLogicalProcedureBindings {
    /// Genuine original body identifies this lexical scope independently of a
    /// reporting procedure name or a physical frame allocation.
    #[must_use]
    pub fn body(&self) -> &OriginalSourceScriptBody {
        &self.body
    }
    /// Unique fixed scalar bindings. A final variadic `args` remains unchanged.
    #[must_use]
    pub fn formals(&self) -> &[OriginalLogicalFormalBinding] {
        &self.formals
    }
    /// Complete current source and full retained editing-input correspondence.
    #[must_use]
    pub fn matches_source(&self, source: &str, input: &ResolvedAnalysisInput) -> bool {
        let image = SourceImage::document(source);
        self.input == *input
            && self
                .declaration
                .matches_source(&image, input.lexer_config())
            && self.body.matches_source(&image, input.lexer_config())
            && self.parameters.image() == &image
            && self.parameters.config() == input.lexer_config()
    }
    /// Binding-name lookup has complete original coverage in the bounded
    /// Logical authoring model. Editing additionally requires an isolated
    /// caller policy; this does not certify Native callback-free value reads.
    #[must_use]
    pub const fn alpha_binding_lookup_closed(&self) -> bool {
        matches!(self.closure, BindingClosure::Closed)
    }
    /// Same declared binding at a formal field or an authentic scalar read.
    #[must_use]
    pub fn formal_at(&self, offset: u32) -> Option<&OriginalLogicalFormalBinding> {
        self.formals.iter().find(|formal| {
            (formal.declaration.start() <= offset && offset < formal.declaration.end())
                || formal.references.iter().any(|reference| {
                    reference.name.start() <= offset && offset < reference.name.end()
                })
        })
    }
}

/// Retain root Logical procedure bindings from the actual whole analysis.
/// Unknown source operations preserve readonly declarations, but make alpha
/// closure unavailable. Native, hosted, foreign input and cooked geometry do
/// not borrow this separate authoring purpose.
#[must_use]
pub fn original_logical_procedure_bindings(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<OriginalLogicalProcedureBindings>> {
    // naming.minifier.logical-formal-binding-alpha
    // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
    let input = analysis.resolved_input.as_ref()?;
    let image = SourceImage::document(source);
    if !input.has_logical_source_name_context()
        || !analysis.matches_original_source_image(&image, input.lexer_config())
    {
        return None;
    }
    let realm = analysis.retained_command_realm()?;
    if realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()
        != Some(input)
    {
        return None;
    }
    let plan = tcl_lexer::native_script_words_in(
        image,
        Span::new(0, u32::try_from(source.len()).ok()?),
        input.lexer_config(),
    )
    .ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() > MAX_COMMANDS {
        return None;
    }
    let context = input.context_registry();
    let mut inventory = Vec::new();
    let mut root_closed = true;
    for command in &plan.commands {
        let Some(words) = original_logical_registry_words_for_command(realm, input, command) else {
            root_closed = false;
            continue;
        };
        if let Some(mut procedure) = procedure_bindings(input, &words) {
            retain_reads(realm, &mut procedure)?;
            inventory.push(procedure);
        } else {
            root_closed &= command.words.iter().all(static_word)
                && words.with_source_schema(&context, binding_effects_closed) == Some(true);
        }
        root_closed &= source_applicability_closed(&words);
    }
    if !root_closed {
        for procedure in &mut inventory {
            procedure.closure = BindingClosure::Unavailable;
        }
    }
    Some(inventory)
}

fn source_applicability_closed(words: &OriginalRegistryWords) -> bool {
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return false;
    };
    advice.uncertain_operations().is_empty()
        && !advice.obligations().contains(
            &crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
        )
}

fn procedure_bindings(
    input: &ResolvedAnalysisInput,
    words: &OriginalRegistryWords,
) -> Option<OriginalLogicalProcedureBindings> {
    let context = input.context_registry();
    let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    if advice.logical_source_input() != Some(input) || !advice.lineage().is_empty() {
        return None;
    }
    let parameter = words
        .with_source_schema(&context, |schema| {
            let definitions = schema.state_transitions();
            let mut definitions =
                definitions
                    .command_bindings()
                    .filter_map(|transition| match transition {
                        CommandBindingTransition::Define {
                            kind: CommandBindingDefinitionKind::Procedure,
                            ..
                        } => Some(()),
                        _ => None,
                    });
            if definitions.next().is_none()
                || definitions.next().is_some()
                || !schema.semantics.traits.contains(Traits::DEFERS_BODY)
                || schema.semantics.frame_effect.is_some()
                || schema.facts().arity_accepts_frozen_arguments() != Some(true)
            {
                return None;
            }
            let (roles, complete) = schema.authored_source_argument_roles();
            let mut parameters = roles
                .into_iter()
                .filter(|(_, role)| *role == ArgRole::ParamList);
            let (ordinal, _) = parameters.next()?;
            if !complete || parameters.next().is_some() {
                return None;
            }
            schema
                .semantics
                .argument_offset
                .checked_add(usize::from(ordinal))
        })
        .flatten()?;
    let parameters = words.operands().get(parameter)?.as_ref()?.word()?.clone();
    let formals = declared_formals(&parameters)?;
    let mut bodies = words.source_script_bodies(&context).into_iter();
    let body = bodies.next()?;
    if bodies.next().is_some() {
        return None;
    }
    Some(OriginalLogicalProcedureBindings {
        declaration: Arc::new(words.clone()),
        parameters,
        body: Arc::new(body),
        input: input.clone(),
        formals,
        closure: BindingClosure::Closed,
    })
}

fn declared_formals(word: &NativeWord) -> Option<Vec<OriginalLogicalFormalBinding>> {
    // The bounded Logical contract uses the selected strict Tcl list grammar.
    // Native/lenient acceptance and decoded intermediate offsets stay separate.
    if word.config().list_parse != tcl_dialect::ListParse::Strict || word.group().expand {
        return None;
    }
    let content = word.content_span().ok()?;
    let raw = word.image().bytes().get(content.as_range())?;
    if tcl_syntax::word_rules::original_static_word_unicode_value(word).as_deref() != Some(raw) {
        return None;
    }
    let text = std::str::from_utf8(raw).ok()?;
    let parameters = tcl_syntax::formal_params::parse_formal_parameters(text).ok()?;
    let mut formals = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut next = 0;
    for (ordinal, parameter) in parameters.iter().enumerate() {
        let element =
            tcl_syntax::list::find_element_with_syntax(text, next, word.config().list_parse)
                .ok()??;
        next = element.next;
        if !element.literal {
            return None;
        }
        let specifier = text.get(element.value.clone())?;
        let name =
            tcl_syntax::list::find_element_with_syntax(specifier, 0, word.config().list_parse)
                .ok()??;
        if !name.literal
            || specifier.get(name.value.clone())? != parameter.name
            || parameter.name.as_bytes().contains(&0)
            || !seen.insert(parameter.name.as_str())
        {
            return None;
        }
        if ordinal + 1 == parameters.len() && parameter.name == "args" {
            continue;
        }
        let start = content
            .start()
            .checked_add(u32::try_from(element.value.start.checked_add(name.value.start)?).ok()?)?;
        let end = start.checked_add(u32::try_from(parameter.name.len()).ok()?)?;
        formals.push(OriginalLogicalFormalBinding {
            name: parameter.name.clone(),
            declaration: Span::new(start, end),
            references: Vec::new(),
        });
    }
    Some(formals)
}

fn retain_reads(
    realm: &CommandBindingRealm,
    procedure: &mut OriginalLogicalProcedureBindings,
) -> Option<()> {
    let image = procedure.parameters.image().clone();
    let config = procedure.input.lexer_config();
    let context = procedure.input.context_registry();
    let plan =
        tcl_lexer::native_script_words_in(image, procedure.body.content_span(), config).ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() > MAX_COMMANDS {
        return None;
    }
    let mut incoming_closed = true;
    for command in &plan.commands {
        let words = original_logical_registry_words_for_command(realm, &procedure.input, command);
        let closed = words.as_ref().is_some_and(|words| {
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                return false;
            };
            advice
                .logical_source_body()
                .is_some_and(|body| body == procedure.body.as_ref())
                && advice.lineage().is_empty()
                && words.with_source_schema(&context, binding_effects_closed) == Some(true)
        });
        // Arguments precede the current handler. Earlier unknown binding
        // effects still prevent the subsequent read from joining this formal.
        let references_closed =
            incoming_closed && retain_command_references(command, &mut procedure.formals);
        incoming_closed &= closed && references_closed;
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_LOGICAL_FORMALS").is_some() {
            eprintln!(
                "LOGICAL_FORMAL_COMMAND offset={} schema_closed={} references_closed={}",
                command.span.start(),
                closed,
                references_closed
            );
        }
        if !closed || !references_closed {
            procedure.closure = BindingClosure::Unavailable;
        }
    }
    Some(())
}

fn retain_command_references(
    command: &NativeScriptCommandWords,
    formals: &mut [OriginalLogicalFormalBinding],
) -> bool {
    let mut complete = command.words.first().is_some_and(static_word);
    let mut references = Vec::new();
    for word in command.words.iter().skip(1) {
        if word.group().expand {
            complete = false;
        }
        for component in word.executable_parts().all_parts() {
            match &component.part {
                ExecutablePart::Text(_) => {}
                ExecutablePart::Variable { name, index: None } => {
                    let Some(raw) = word.image().bytes().get(name.as_range()) else {
                        complete = false;
                        continue;
                    };
                    let Some(ordinal) = formals
                        .iter()
                        .position(|formal| formal.name.as_bytes() == raw)
                    else {
                        complete = false;
                        continue;
                    };
                    references.push((
                        ordinal,
                        OriginalLogicalFormalReference {
                            word: word.clone(),
                            name: *name,
                        },
                    ));
                }
                _ => complete = false,
            }
        }
    }
    if complete {
        for (ordinal, reference) in references {
            formals[ordinal].references.push(reference);
        }
    }
    complete
}

fn static_word(word: &NativeWord) -> bool {
    !word.group().expand
        && tcl_syntax::word_rules::original_static_word_unicode_value(word).is_some()
}

fn binding_effects_closed(schema: &tcl_registry::ResolvedInvocation<'_, '_>) -> bool {
    let facts = schema.facts();
    let (roles, complete) = schema.authored_source_argument_roles();
    let barriers = tcl_registry::FRAME_REACH_TRAITS
        | Traits::INTROSPECTS_BY_NAME
        | Traits::TARGETS_VARIABLE_BY_NAME
        | Traits::REFLECTS_COMMAND_NAMES
        | Traits::CREATES_SCOPE_ALIAS
        | Traits::CREATES_DYNAMIC_BARRIER
        | Traits::DEFERS_BODY;
    complete
        && !roles.iter().any(|(_, role)| {
            matches!(
                role,
                ArgRole::Body
                    | ArgRole::Expr
                    | ArgRole::LambdaLiteral
                    | ArgRole::VarRead
                    | ArgRole::VarWrite
            )
        })
        && facts.arity_accepts_frozen_arguments() == Some(true)
        && !facts.traits.intersects(barriers)
        && facts.frame_effect.is_none()
        && (schema.authored_source_result_preserves_variable_bindings()
            || closed_binding_effects(&facts))
}

fn closed_binding_effects(facts: &tcl_registry::InvocationFacts) -> bool {
    !facts.effects.requires_world_barrier()
        && facts.effects.accesses().iter().all(|access| {
            matches!(
                access.domain,
                tcl_registry::world_effect::WorldStateDomain::InterpreterResult
                    | tcl_registry::world_effect::WorldStateDomain::CompletionState
                    | tcl_registry::world_effect::WorldStateDomain::LegacyExternal(
                        tcl_registry::side_effects::SideEffectTarget::EventControl
                    )
            )
        })
        && facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| transitions.facts().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyse(source: &str) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    #[test]
    fn logical_formal_bindings_retain_general_fields_and_original_reads() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        let source = "proc p {longleft {longright fixed}} {list $longleft ${longright} $longleft; return \"$longright/$longleft\"}\n";
        let analysis = analyse(source);
        let records = original_logical_procedure_bindings(source, &analysis).unwrap();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert!(record.matches_source(source, analysis.resolved_input.as_ref().unwrap()));
        assert!(record.alpha_binding_lookup_closed());
        assert_eq!(
            record
                .formals()
                .iter()
                .map(OriginalLogicalFormalBinding::name)
                .collect::<Vec<_>>(),
            ["longleft", "longright"]
        );
        assert_eq!(record.formals()[0].references().len(), 3);
        assert_eq!(record.formals()[1].references().len(), 2);
        for formal in record.formals() {
            assert_eq!(
                source.get(formal.declaration_span().as_range()),
                Some(formal.name())
            );
            for reference in formal.references() {
                assert_eq!(
                    source.get(reference.name_span().as_range()),
                    Some(formal.name())
                );
                assert_eq!(reference.word.image(), record.parameters.image());
                assert_eq!(
                    record.formal_at(reference.name_span().start()),
                    Some(formal)
                );
            }
        }
        let changed = format!("{source}# different source\n");
        assert!(original_logical_procedure_bindings(&changed, &analysis).is_none());
        let input = analysis.resolved_input.as_ref().unwrap();
        let foreign = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::new(
                input.context_registry().with_command_store(
                    input
                        .context_registry()
                        .commands()
                        .snapshot()
                        .shared_registry(),
                ),
            ),
            input.lexer_config(),
        );
        assert!(!record.matches_source(source, &foreign));
    }

    #[test]
    fn logical_unicode_formals_retain_literal_fields_and_original_scalar_reads() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        let source = "proc identité {naïve {東京 défaut} {é😀 fixed}} {list ${naïve} ${東京} \"${é😀}/${naïve}\"; return ${東京}}\n";
        let analysis = analyse(source);
        let records = original_logical_procedure_bindings(source, &analysis).unwrap();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert!(record.matches_source(source, analysis.resolved_input.as_ref().unwrap()));
        assert!(record.alpha_binding_lookup_closed());
        assert_eq!(
            record
                .formals()
                .iter()
                .map(OriginalLogicalFormalBinding::name)
                .collect::<Vec<_>>(),
            ["naïve", "東京", "é😀"]
        );
        assert_eq!(
            record
                .formals()
                .iter()
                .map(|formal| formal.references().len())
                .collect::<Vec<_>>(),
            [2, 2, 1]
        );
        for formal in record.formals() {
            assert_eq!(
                source.get(formal.declaration_span().as_range()),
                Some(formal.name())
            );
            assert_eq!(
                record.formal_at(formal.declaration_span().start()),
                Some(formal)
            );
            for reference in formal.references() {
                assert_eq!(
                    source.get(reference.name_span().as_range()),
                    Some(formal.name())
                );
                assert_eq!(
                    reference.original_word().image(),
                    &SourceImage::document(source)
                );
                assert_eq!(
                    record.formal_at(reference.name_span().start()),
                    Some(formal)
                );
            }
        }
        assert!(source.contains("{東京 défaut}"));
    }

    #[test]
    fn logical_unicode_formals_refuse_cooked_units_and_withdrawn_source_owners() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        for source in [
            r#"proc p "\u00e9" {return ${é}}"#,
            r#"proc p "\uD800" {return ${�}}"#,
            r#"proc p {é é} {return ${é}}"#,
            "proc p {é\0} {return ${é}}",
        ] {
            let analysis = analyse(source);
            assert!(
                original_logical_procedure_bindings(source, &analysis)
                    .unwrap()
                    .is_empty(),
                "{source:?}"
            );
        }
        let source = "proc p {é} {return ${é}}";
        let mut analysis = analyse(source);
        assert_eq!(
            original_logical_procedure_bindings(source, &analysis)
                .unwrap()
                .len(),
            1
        );
        assert!(
            original_logical_procedure_bindings("proc p {é} {return ${other}}", &analysis)
                .is_none()
        );
        let input = analysis.resolved_input.as_ref().unwrap().clone();
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        analysis.resolved_input = Some(ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        ));
        assert!(original_logical_procedure_bindings(source, &analysis).is_none());
        let context = input.context_registry();
        analysis.resolved_input = Some(ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::new(context.with_command_store(context.commands().snapshot().shared_registry())),
            input.lexer_config(),
        ));
        assert!(original_logical_procedure_bindings(source, &analysis).is_none());
        analysis.resolved_input = None;
        assert!(original_logical_procedure_bindings(source, &analysis).is_none());
    }

    #[test]
    fn logical_formal_alpha_closure_keeps_observers_aliases_and_unknowns_outside() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        for body in [
            "mystery $longleft",
            "info locals; return $longleft",
            "global longleft; return $longleft",
            "upvar 1 external longleft; return $longleft",
            "trace add variable longleft read hook; return $longleft",
            "set longleft changed; return $longleft",
            "return $longleft(index)",
            "return [list $longleft]",
            "if {$longleft} {return $longright}",
        ] {
            let source = format!("proc p {{longleft longright}} {{{body}}}");
            let analysis = analyse(&source);
            let records = original_logical_procedure_bindings(&source, &analysis).unwrap();
            assert_eq!(records.len(), 1, "{body}");
            assert!(!records[0].alpha_binding_lookup_closed(), "{body}");
        }
        let source = "proc p {longleft longright} {mystery $longleft; return $longleft}";
        let records = original_logical_procedure_bindings(source, &analyse(source)).unwrap();
        assert_eq!(records[0].formals()[0].references().len(), 1);
        assert_eq!(
            source.get(
                records[0].formals()[0].references()[0]
                    .name_span()
                    .as_range()
            ),
            Some("longleft")
        );
        let source = "proc p {longleft longright} {global longleft; return $longleft}";
        let records = original_logical_procedure_bindings(source, &analyse(source)).unwrap();
        assert!(records[0].formals()[0].references().is_empty());
        let source = "mystery; proc p {longleft longright} {return $longleft}";
        assert!(
            !original_logical_procedure_bindings(source, &analyse(source)).unwrap()[0]
                .alpha_binding_lookup_closed()
        );
        let source = "proc p {longleft longleft} {return $longleft}";
        assert!(
            original_logical_procedure_bindings(source, &analyse(source))
                .unwrap()
                .is_empty()
        );
        let source = r#"proc p {longleft} "return\u0020$longleft""#;
        assert!(
            original_logical_procedure_bindings(source, &analyse(source))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn logical_formal_inventory_keeps_variadic_and_native_domains_separate() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        let source = "proc p {longleft args} {return $longleft}";
        let records = original_logical_procedure_bindings(source, &analyse(source)).unwrap();
        assert_eq!(
            records[0]
                .formals()
                .iter()
                .map(OriginalLogicalFormalBinding::name)
                .collect::<Vec<_>>(),
            ["longleft"]
        );
        for environment in ["tcl8.4", "tcl8.6", "tcl9.1", "jim", "f5-irules"] {
            let owner = tcl_registry::model::ingress::resolve_environment(environment);
            let input = ResolvedAnalysisInput::new(
                owner.analyser_profile(),
                owner.unit_profile(),
                owner.default_context_registry(),
                tcl_lexer::LexerConfig::for_file_grammar(owner.analyser_profile().grammar),
            );
            let analysis = crate::analyser::Analyser::new()
                .with_resolved_input(input)
                .analyse(source, environment);
            assert!(
                original_logical_procedure_bindings(source, &analysis).is_none(),
                "{environment}"
            );
        }
    }
}
