// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Explicit path source advice from one complete retained analysis.

use super::{PathConstantAssignments, PathConstantValue, PathConstantWrite, Scope};
use crate::analyser::AnalysisResult;
use crate::registry_invocation::source_structure::{OriginalRegistryWords, source_registry_words};
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::model::ContextRegistry;

struct PathSourceContext<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    context: &'a ContextRegistry,
}

/// Inventory source writes under the actual image, availability and full
/// grammar. This is explicit conditional path advice, not observed values,
/// entered namespaces, native cells or successful command execution. An
/// unavailable naming recipe or stale retained input declines the inventory.
#[must_use]
pub fn constant_path_assignments_from_analysis(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<PathConstantAssignments> {
    let input = analysis.resolved_input.as_ref()?;
    let commands =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, input.lexer_config());
    let mut out = PathConstantAssignments::default();
    extend_path_constant_assignments_from_analysis_commands(
        source, analysis, &commands, None, &mut out,
    )
    .then_some(out)
}

/// Extend source advice under the actual complete analysis. The optional seed
/// is explicitly constructed source namespace geometry, never an entered
/// native namespace. Missing/stale premises withdraw prior batch facts.
pub(crate) fn extend_path_constant_assignments_from_analysis_commands(
    source: &str,
    analysis: &AnalysisResult,
    batch: &[crate::segmenter::SegmentedCommand],
    namespace: Option<&str>,
    out: &mut PathConstantAssignments,
) -> bool {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let Some((selected, root)) = prepare_inventory(source, analysis, namespace, out) else {
        out.clear();
        return false;
    };
    let pending = batch
        .iter()
        .rev()
        .cloned()
        .map(|command| (command, root.clone()))
        .collect();
    if selected.collect(pending, out).is_none() {
        out.clear();
        return false;
    }
    out.recorded_end = batch
        .iter()
        .map(|command| command.execution_span(source).end())
        .max()
        .unwrap_or(out.recorded_end)
        .max(out.recorded_end);
    true
}

fn prepare_inventory<'a>(
    source: &'a str,
    analysis: &'a AnalysisResult,
    namespace: Option<&str>,
    out: &mut PathConstantAssignments,
) -> Option<(PathSourceContext<'a>, Scope)> {
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = SourceImage::document(source);
    let realm = analysis.retained_command_realm_arc()?;
    let policy = super::authored_policy(input.analyser_profile())?;
    if analysis.body_lexer_config != Some(config)
        || !analysis.matches_original_source_image(&image, config)
    {
        return None;
    }
    let selected = PathSourceContext {
        source,
        analysis,
        context: input.borrowed_context_registry(),
    };
    let root = Scope {
        id: 0,
        start: 0,
        end: u32::MAX,
        namespace: namespace.unwrap_or_default().to_owned(),
        local: namespace.is_some()
            && policy.recipe() == tcl_syntax::naming::NativeNameProtocol::Jim084,
    };
    let same = out.source_input.as_ref() == Some(input)
        && out.source_image.as_ref() == Some(&image)
        && out
            .source_realm
            .as_ref()
            .is_some_and(|prior| std::sync::Arc::ptr_eq(prior, realm))
        && out.scopes.first() == Some(&root);
    if !same {
        let prefix = out.recorded_end;
        if prefix != 0
            && out
                .source_image
                .as_ref()
                .and_then(|prior| prior.bytes().get(..prefix as usize))
                != source.as_bytes().get(..prefix as usize)
        {
            return None;
        }
        *out = PathConstantAssignments {
            policy: Some(policy),
            source_config: Some(config),
            source_input: Some(input.clone()),
            source_image: Some(image),
            source_realm: Some(std::sync::Arc::clone(realm)),
            recorded_end: 0,
            ..PathConstantAssignments::default()
        };
        out.scopes.push(root.clone());
        if prefix != 0 {
            let text = source.get(..prefix as usize)?;
            selected.collect(commands(text, 0, config, &root), out)?;
            out.recorded_end = prefix;
        }
    }
    Some((selected, root))
}

fn commands(
    source: &str,
    base: u32,
    config: LexerConfig,
    scope: &Scope,
) -> Vec<(crate::segmenter::SegmentedCommand, Scope)> {
    crate::segmenter::segment_commands_with_offset_and_config(source, base, config)
        .into_iter()
        .rev()
        .map(|command| (command, scope.clone()))
        .collect()
}

fn operand(words: &OriginalRegistryWords, argument: usize) -> Option<&NativeWord> {
    words.operands().get(argument)?.as_ref()?.word()
}
fn name(words: &OriginalRegistryWords, argument: usize) -> Option<&str> {
    operand(words, argument)?;
    let bytes = words.arguments().get(argument)?.literal_bytes()?;
    if tcl_syntax::naming::split_element_ref_bytes(bytes).is_some() {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

impl PathSourceContext<'_> {
    fn collect(
        &self,
        mut pending: Vec<(crate::segmenter::SegmentedCommand, Scope)>,
        out: &mut PathConstantAssignments,
    ) -> Option<()> {
        while let Some((command, scope)) = pending.pop() {
            let at = command.span.start();
            let Some(words) = source_registry_words(self.source, self.analysis, &command) else {
                out.barriers.push(at);
                continue;
            };
            let Some((namespace, assignments, traits, lowering)) =
                words.with_source_schema(self.context, |schema| {
                    (
                        schema.authored_source_namespace_body_arguments(),
                        schema.authored_source_assignment_arguments(),
                        schema.semantics.traits,
                        schema.semantics.lowering_hook,
                    )
                })
            else {
                out.barriers.push(at);
                continue;
            };
            if let Some(layout) = namespace {
                if let Some(child) =
                    self.namespace_body(&words, &scope, layout, out.policy?.recipe())
                {
                    let text = self.source.get(child.start as usize..child.end as usize)?;
                    pending.extend(commands(
                        text,
                        child.start,
                        self.analysis.body_lexer_config?,
                        &child,
                    ));
                    out.scopes.push(child);
                } else {
                    out.barriers.push(at);
                }
                continue;
            }
            out.unavailable_scopes.extend(
                words
                    .source_script_bodies(self.context)
                    .into_iter()
                    .map(|body| {
                        let span = body.content_span();
                        (span.start(), span.end())
                    }),
            );
            if matches!(
                lowering,
                Some(
                    tcl_registry::hooks::LoweringHookId::Eval
                        | tcl_registry::hooks::LoweringHookId::Uplevel
                        | tcl_registry::hooks::LoweringHookId::Apply
                        | tcl_registry::hooks::LoweringHookId::NamespaceEval
                )
            ) {
                out.barriers.push(command.span.end());
            }
            let Some(pairs) = assignments else {
                out.barriers.push(at);
                continue;
            };
            let declares = traits.contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS);
            if pairs.is_empty() {
                self.poison_writes(&words, &scope, at, out);
            } else {
                for (name, value) in pairs {
                    self.assignment(&words, &scope, at, (name, value, declares), out);
                }
            }
        }
        Some(())
    }

    fn namespace_body(
        &self,
        words: &OriginalRegistryWords,
        scope: &Scope,
        (name_index, body_index): (usize, usize),
        protocol: tcl_syntax::naming::NativeNameProtocol,
    ) -> Option<Scope> {
        operand(words, name_index)?;
        let name = std::str::from_utf8(words.arguments().get(name_index)?.literal_bytes()?).ok()?;
        let body_word = operand(words, body_index)?;
        let body = words
            .source_script_bodies(self.context)
            .into_iter()
            .find(|body| body.original_container() == body_word)?;
        let content = body.content_span();
        if words.arguments().get(body_index)?.literal_bytes()?
            != self.source.as_bytes().get(content.as_range())?
        {
            return None;
        }
        let namespace = match protocol {
            tcl_syntax::naming::NativeNameProtocol::C(_) => {
                crate::naming::qualify_namespace(&scope.namespace, name)
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                super::jim_namespace(protocol, &scope.namespace, name)?
            }
        };
        Some(Scope {
            id: content.start(),
            start: content.start(),
            end: content.end(),
            namespace,
            local: protocol == tcl_syntax::naming::NativeNameProtocol::Jim084,
        })
    }

    fn assignment(
        &self,
        words: &OriginalRegistryWords,
        scope: &Scope,
        at: u32,
        (name_index, value_index, declares): (usize, Option<usize>, bool),
        out: &mut PathConstantAssignments,
    ) {
        let Some((home, name, alias)) =
            name(words, name_index).and_then(|name| super::target(out, scope, name, declares))
        else {
            out.barriers.push(at);
            return;
        };
        let value = value_index.map_or(PathConstantValue::Declared, |argument| {
            self.value(words, argument)
                .unwrap_or(PathConstantValue::Poisoned)
        });
        out.writes.push(PathConstantWrite {
            name,
            ns: scope.namespace.clone(),
            at,
            value,
            home,
            scope: scope.id,
            alias,
        });
    }

    fn value(&self, words: &OriginalRegistryWords, argument: usize) -> Option<PathConstantValue> {
        let word = operand(words, argument)?;
        if let Some(bytes) = words.arguments().get(argument)?.literal_bytes() {
            return Some(PathConstantValue::Raw {
                text: std::str::from_utf8(bytes).ok()?.to_owned(),
                literal: true,
            });
        }
        super::super::capture_source_path_expression(self.source, self.analysis, word)
            .map(PathConstantValue::Original)
    }

    fn poison_writes(
        &self,
        words: &OriginalRegistryWords,
        scope: &Scope,
        at: u32,
        out: &mut PathConstantAssignments,
    ) {
        for &(argument, role) in words.roles().unwrap_or_default() {
            if role == tcl_registry::ArgRole::VarWrite {
                self.assignment(words, scope, at, (argument, None, false), out);
                if let Some(write) = out.writes.last_mut().filter(|write| write.at == at) {
                    write.value = PathConstantValue::Poisoned;
                }
            }
        }
    }
}

/// Check original nested source schema correspondence before a caller uses
/// the standalone path-expression algebra. This does not prove its result or
/// execution: known replacements, missing source operands and stale images
/// cannot supply the algebra's command premises.
#[must_use]
pub fn path_source_word_is_available(
    source: &str,
    analysis: &AnalysisResult,
    word: &NativeWord,
) -> bool {
    super::super::capture_source_path_expression(source, analysis, word).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn retained_input(
        profile: &'static tcl_dialect::DialectProfile,
        config: LexerConfig,
    ) -> crate::analyser::ResolvedAnalysisInput {
        let generation = tcl_registry::model::ingress::context_for_profile(profile);
        let mut commands = generation.commands().project_for_profile(profile);
        commands.insert_ambient_package("example", "1.0");
        let actual =
            std::sync::Arc::new(generation.with_command_store(std::sync::Arc::new(commands)));
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, actual, config)
    }

    fn capture_analysis(
        source: &str,
        profile: &'static tcl_dialect::DialectProfile,
        config: LexerConfig,
    ) -> AnalysisResult {
        crate::analyser::Analyser::new()
            .with_resolved_input(retained_input(profile, config))
            .analyse(source, profile.name)
    }

    #[test]
    fn original_path_inventory_keeps_full_input_scalar_names_and_absolute_namespace_scopes() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            ..LexerConfig::for_profile(Some(profile))
        };
        let source = "set {path(open} /ROOT; set {literal$é} /UNICODE; namespace eval N {variable local /N; source $local/a.tcl}";
        let analysis = capture_analysis(source, profile, config);
        assert!(
            analysis
                .resolved_input
                .as_ref()
                .unwrap()
                .borrowed_context_registry()
                .context()
                .ambient_package("example")
        );
        let inventory = constant_path_assignments_from_analysis(source, &analysis).unwrap();
        assert_eq!(inventory.source_config, Some(config));
        assert!(inventory.iter().any(|write| write.name == "path(open"));
        assert!(inventory.iter().any(|write| write.name == "literal$é"));
        let span = inventory.namespace_body_spans().next().unwrap();
        assert_eq!(
            source.get(span.as_range()),
            Some("variable local /N; source $local/a.tcl")
        );
        assert!(
            inventory
                .iter()
                .any(|write| write.name == "::N::local" && write.at == span.start())
        );
        assert!(
            constant_path_assignments_from_analysis(&source.replace("/ROOT", "/FAIL"), &analysis)
                .is_none()
        );
        let mut stale = analysis.clone();
        stale.body_lexer_config = Some(LexerConfig::for_profile(Some(profile)));
        assert!(constant_path_assignments_from_analysis(source, &stale).is_none());
        assert!(
            constant_path_assignments_from_analysis(source, &AnalysisResult::default()).is_none()
        );
    }

    #[test]
    fn original_path_name_domains_keep_empty_scalar_and_parenthesised_namespace_literals() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig::for_profile(Some(profile));
        let source = "set {} /EMPTY; namespace eval {N(x)} {variable local /P}";
        let analysis = capture_analysis(source, profile, config);
        let inventory = constant_path_assignments_from_analysis(source, &analysis).unwrap();
        assert!(inventory.iter().any(|write| write.name.is_empty()));
        assert!(inventory.iter().any(|write| write.name == "::N(x)::local"));
        let span = inventory.namespace_body_spans().next().unwrap();
        assert_eq!(source.get(span.as_range()), Some("variable local /P"));
    }

    #[test]
    fn original_path_folding_keeps_selected_first_close_variable_grammar() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let source = "set a\\{b /RIGHT; set a /WRONG; set result ${a{b}/x";
        let config = LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            ..LexerConfig::for_profile(Some(profile))
        };
        let analysis = capture_analysis(source, profile, config);
        let inventory = constant_path_assignments_from_analysis(source, &analysis).unwrap();
        let constants = super::super::fold_constant_assignments(&inventory, None);
        assert_eq!(
            constants.get("result").map(String::as_str),
            Some("/RIGHT/x")
        );
        let unavailable = "proc file {args} {}; set result [file join /tmp sub]";
        let analysis = capture_analysis(unavailable, profile, config);
        let inventory = constant_path_assignments_from_analysis(unavailable, &analysis).unwrap();
        assert!(inventory.iter().all(|write| !matches!(write.value, PathConstantValue::Raw { ref text, literal: false } if text.contains("file join"))));
    }
    #[test]
    fn original_path_producer_full_chunked_and_restored_batches_keep_actual_input() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            ..LexerConfig::for_profile(Some(profile))
        };
        let input = retained_input(profile, config);
        let source = "set a\\{b /ROOT\nset {literal$é} /UNICODE\nset result ${a{b}/leaf\n";
        let full = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let chunks = commands.into_iter().map(|command| vec![command]).collect();
        let (chunked, mut snapshots) = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse_chunked(source, chunks, profile.name);
        assert_eq!(
            full.path_constant_assignments,
            chunked.path_constant_assignments
        );
        assert_eq!(snapshots[0].result.path_constant_assignments.len(), 1);
        assert_eq!(snapshots[1].result.path_constant_assignments.len(), 2);
        assert_eq!(chunked.path_constant_assignments.len(), 3);
        assert_eq!(
            chunked.path_constant_assignments.source_input.as_ref(),
            Some(&input)
        );
        let constants =
            super::super::fold_constant_assignments(&chunked.path_constant_assignments, None);
        assert_eq!(
            constants.get("result").map(String::as_str),
            Some("/ROOT/leaf")
        );

        let changed = source.replace("/leaf", "/next");
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(&changed, 0, config);
        let mut restored = crate::analyser::Analyser::new();
        restored.restore(snapshots.remove(1));
        let resumed = restored.analyse_commands(&changed, &commands[2..], profile.name, true);
        let fresh = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(&changed, profile.name);
        assert_eq!(
            resumed.path_constant_assignments,
            fresh.path_constant_assignments
        );
        assert_eq!(resumed.path_constant_assignments.len(), 3);
        let constants =
            super::super::fold_constant_assignments(&resumed.path_constant_assignments, None);
        assert_eq!(
            constants.get("result").map(String::as_str),
            Some("/ROOT/next")
        );
    }

    #[test]
    fn original_path_batches_reselect_current_realm_and_withdraw_stale_prefixes() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig::for_profile(Some(profile));
        let source = "set dir /ROOT; set tail /TAIL";
        let first = capture_analysis(source, profile, config);
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let mut out = PathConstantAssignments::default();
        assert!(extend_path_constant_assignments_from_analysis_commands(
            source,
            &first,
            &commands[..1],
            None,
            &mut out
        ));
        assert_eq!(out.len(), 1);
        let current = capture_analysis(source, profile, config);
        assert!(!std::sync::Arc::ptr_eq(
            first.retained_command_realm_arc().unwrap(),
            current.retained_command_realm_arc().unwrap()
        ));
        assert!(extend_path_constant_assignments_from_analysis_commands(
            source,
            &current,
            &commands[1..],
            None,
            &mut out
        ));
        assert_eq!(
            out,
            constant_path_assignments_from_analysis(source, &current).unwrap()
        );
        assert!(std::sync::Arc::ptr_eq(
            out.source_realm.as_ref().unwrap(),
            current.retained_command_realm_arc().unwrap()
        ));

        let mut stale = current.clone();
        stale.body_lexer_config = Some(LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            ..config
        });
        assert!(!extend_path_constant_assignments_from_analysis_commands(
            source,
            &stale,
            &[],
            None,
            &mut out
        ));
        assert!(out.naming_policy().is_none());
        assert!(out.is_empty());
        assert!(extend_path_constant_assignments_from_analysis_commands(
            source,
            &first,
            &commands[..1],
            None,
            &mut out
        ));
        assert!(!extend_path_constant_assignments_from_analysis_commands(
            source,
            &AnalysisResult::default(),
            &[],
            None,
            &mut out
        ));
        assert!(out.source_input.is_none());
        assert!(out.source_realm.is_none());
        assert!(out.is_empty());

        assert!(extend_path_constant_assignments_from_analysis_commands(
            source,
            &first,
            &commands[..1],
            None,
            &mut out
        ));
        let changed = source.replace("/ROOT", "/FAIL");
        let current = capture_analysis(&changed, profile, config);
        let commands =
            crate::segmenter::segment_commands_with_offset_and_config(&changed, 0, config);
        assert!(!extend_path_constant_assignments_from_analysis_commands(
            &changed,
            &current,
            &commands[1..],
            None,
            &mut out
        ));
        assert!(out.is_empty());
        assert!(out.naming_policy().is_none());
    }

    #[test]
    fn original_path_producer_keeps_explicit_source_seed_and_declines_replaced_setter() {
        // naming.navigation.retained-path-source-inventory
        // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig::for_profile(Some(profile));
        let source = "variable dir /ROOT; set dir /NEXT";
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(retained_input(profile, config))
            .analyse_with_source_namespace(source, profile.name, "::N");
        let inventory = &analysis.path_constant_assignments;
        assert_eq!(inventory.len(), 2);
        assert_eq!(inventory.scopes[0].namespace, "::N");
        assert!(inventory.iter().all(|write| write.name == "::N::dir"));
        assert_eq!(inventory[0].alias.as_deref(), Some("dir"));
        assert_eq!(inventory[1].alias, None);
        assert_eq!(
            inventory[1].value,
            PathConstantValue::Raw {
                text: "/NEXT".to_owned(),
                literal: true
            }
        );
        let unavailable = "proc set {args} {}; set dir /FAIL";
        let analysis = capture_analysis(unavailable, profile, config);
        assert!(analysis.path_constant_assignments.iter().all(
            |write| !matches!(&write.value, PathConstantValue::Raw { text, .. } if text == "/FAIL")
        ));
        assert_eq!(
            analysis.path_constant_assignments,
            constant_path_assignments_from_analysis(unavailable, &analysis).unwrap()
        );
    }
}
