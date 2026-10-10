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

//! Original procedure-body layouts independent of command installation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::declaration_layout::{
    DeclarationLayoutIssuer, DeclarationLayoutObservation, OriginalDiagnosticFrameEntry,
    original_declaration_layouts,
};
use super::{
    CommandAllocationSite, ExecutedScriptSource, SourceCommandBindings, SourceLookupSnapshot,
};
use crate::registry_invocation::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target, resolve_registry_words_in_realm,
};

/// A positioned conditional declaration recipe, never an implementation allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredProcedureBody {
    declaration: CommandAllocationSite,
    candidate: super::SourceCommandTarget,
    slot: super::SourceCommandKey,
    pub(super) source: ExecutedScriptSource,
    pub(super) namespace: super::SourceNamespaceKey,
    pub(super) parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    pub(super) original_parameters: Option<super::formal_topology::OriginalFormalTopology>,
    pub(super) frame: crate::var_resolve::VariableExecutionFrame,
}

/// One lexical body under its complete immutable parent lookup worlds.
/// The registry is fixed for this collector. Parent command words are replaced
/// by genuine child words before any semantic query, so they are not a body
/// traversal input. Entry ownership, issuer, realm, state, namespace and lexer
/// config remain in structural equality; hashes alone never select a visit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NestedDeclarationLayoutKey {
    source: ExecutedScriptSource,
    parents: Vec<DeclarationLayoutObservation>,
}

impl NestedDeclarationLayoutKey {
    fn new(source: &ExecutedScriptSource, parents: &[DeclarationLayoutObservation]) -> Self {
        let parents = parents
            .iter()
            .map(|parent| DeclarationLayoutObservation {
                words: Arc::from([]),
                ..parent.clone()
            })
            .collect();
        Self {
            source: source.clone(),
            parents,
        }
    }
}

/// Pure original source geometry. Lookup worlds remain separate traversal inputs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NestedLayoutSyntaxKey {
    source: ExecutedScriptSource,
    config: tcl_lexer::LexerConfig,
}

struct NestedLayoutCommand {
    site: CommandAllocationSite,
    tokens: Box<crate::ir::CommandTokens>,
}

/// A collector-local source tape, with no selected invocation or read/store facts.
#[derive(Default)]
struct NestedLayoutSyntax {
    commands: HashMap<NestedLayoutSyntaxKey, Option<Arc<[NestedLayoutCommand]>>>,
    #[cfg(test)]
    parsed_scripts: usize,
}

impl NestedLayoutSyntax {
    fn commands(
        &mut self,
        source: &ExecutedScriptSource,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<[NestedLayoutCommand]>> {
        let key = NestedLayoutSyntaxKey {
            source: source.clone(),
            config,
        };
        if let Some(commands) = self.commands.get(&key) {
            return commands.clone();
        }
        #[cfg(test)]
        {
            self.parsed_scripts += 1;
        }
        let commands = Self::parse(source, config);
        self.commands.insert(key, commands.clone());
        commands
    }

    fn parse(
        source: &ExecutedScriptSource,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<[NestedLayoutCommand]>> {
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &source.text,
            source.base(),
            config,
        )?;
        Some(
            segments
                .iter()
                .take_while(|segment| !segment.is_partial)
                .map(|segment| NestedLayoutCommand {
                    site: CommandAllocationSite {
                        source: Arc::clone(&source.origin),
                        offset: segment.span.start(),
                    },
                    tokens: super::source_command_tokens_boxed(
                        &source.text,
                        source.base(),
                        config,
                        segment,
                    ),
                })
                .collect::<Vec<_>>()
                .into(),
        )
    }
}

/// One original command layout in its complete immutable lookup world.
/// Children and continuation share this derivation; it grants no body entry.
struct OriginalDeclarationLayout {
    advice: super::OriginalCompilationLookupAdvice,
    flow: crate::registry_invocation::DeclarationInvocationFlow,
}

impl OriginalDeclarationLayout {
    fn select(
        site: &CommandAllocationSite,
        tokens: &crate::ir::CommandTokens,
        snapshot: &Arc<SourceLookupSnapshot>,
        namespace: &super::SourceNamespaceKey,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Self> {
        let advice =
            super::original_site_operand_layout_advice(site, tokens, snapshot, namespace, config)?;
        let flow =
            crate::registry_invocation::declaration_invocation_flow(registry, tokens, &advice)?;
        Some(Self { advice, flow })
    }

    fn retains_successor(&self) -> bool {
        self.flow.effects.lookup_stable
            && !self.flow.effects.unknown_writes
            && matches!(
                self.flow.flow,
                tcl_registry::script_body_flow::ScriptBodyFlow::None
            )
            && self.flow.effective.words.iter().all(|word| {
                effective_invocation_word(
                    word,
                    self.advice.dialect().lexer_grammar.escapes,
                    self.advice.dialect().word_values,
                )
                .literal_bytes()
                .is_some()
            })
    }
}

struct ParentDeclarationLayout<'a> {
    parent: &'a DeclarationLayoutObservation,
    original: Option<OriginalDeclarationLayout>,
}

fn parent_declaration_layouts<'a>(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    parents: &'a [DeclarationLayoutObservation],
    registry: &tcl_registry::CommandRegistry,
) -> Vec<ParentDeclarationLayout<'a>> {
    parents
        .iter()
        .map(|parent| ParentDeclarationLayout {
            parent,
            original: OriginalDeclarationLayout::select(
                site,
                tokens,
                &parent.snapshot,
                &parent.namespace,
                parent.config,
                registry,
            ),
        })
        .collect()
}

#[cfg(any(test, debug_assertions))]
struct NestedLayoutTrace {
    started: Option<std::time::Instant>,
}

#[cfg(any(test, debug_assertions))]
impl NestedLayoutTrace {
    fn new() -> Self {
        Self {
            started: std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
                .is_some()
                .then(std::time::Instant::now),
        }
    }

    fn phase(&self, stage: &str, visits: usize, pending: usize, layouts: usize) {
        if let Some(started) = self.started {
            eprintln!(
                "SOURCE_LAYOUT_PHASE stage={stage} ms={} visits={visits} pending={pending} layouts={layouts}",
                started.elapsed().as_millis()
            );
        }
    }
}

impl SourceCommandBindings {
    /// Unanimous conditional procedure body at its genuine declaration.
    /// The lexical recipe is independent of installation and entered calls.
    pub(super) fn original_declared_procedure_at(
        &self,
        site: &CommandAllocationSite,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<DeclaredProcedureBody> {
        let observations = original_declaration_layouts(self.declaration_layouts.get(site)?)?;
        let mut agreed = None;
        for observation in observations {
            let tokens = declaration_tokens(site, observation)?;
            let body = declared_body(site, &tokens, observation, registry)?;
            if agreed.as_ref().is_some_and(|previous| previous != &body) {
                return None;
            }
            agreed = Some(body);
        }
        agreed
    }

    /// Unanimous original procedure `ParamList` syntax at its actual declaration
    /// site. This does not select an installation or an entered body.
    pub(crate) fn original_procedure_formals_at(
        &self,
        site: &CommandAllocationSite,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::signature_scan::formal_parameters::SignatureSourceFormalParameters> {
        let observations = original_declaration_layouts(self.declaration_layouts.get(site)?)?;
        let mut agreed = None;
        for observation in observations {
            let tokens = declaration_tokens(site, observation)?;
            let body = declared_body(site, &tokens, observation, registry)?;
            let topology = body.original_parameters?;
            if agreed.as_ref().is_some_and(|old| old != &topology) {
                return None;
            }
            agreed = Some(topology);
        }
        Some(crate::signature_scan::formal_parameters::SignatureSourceFormalParameters::from_original_topology(&agreed?))
    }

    /// Preserve only original lexical layouts when the declaration's runtime
    /// installation is unknown. No dispatch, body entry or effect inventory is
    /// populated by this separate collector.
    pub(super) fn retain_uninstalled_declaration_body_layouts(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) {
        #[cfg(any(test, debug_assertions))]
        let trace = NestedLayoutTrace::new();
        let roots: Vec<_> = self
            .declaration_layouts
            .iter()
            .filter_map(|(site, observations)| {
                let observation = original_declaration_layouts(observations)?.next()?;
                matches!(
                    observation.entry.as_ref(),
                    OriginalDiagnosticFrameEntry::RootScript { .. }
                        | OriginalDiagnosticFrameEntry::ScriptActivation { .. }
                )
                .then(|| (site.clone(), observation.clone()))
            })
            .collect();
        #[cfg(any(test, debug_assertions))]
        trace.phase(
            "uninstalled-roots",
            0,
            roots.len(),
            self.declaration_layouts.len(),
        );
        for (site, observation) in roots {
            let Some(tokens) = declaration_tokens(&site, &observation) else {
                #[cfg(debug_assertions)]
                if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
                    eprintln!(
                        "ORIGINAL_DECLARATION offset={} stage=tokens-unavailable",
                        site.offset
                    );
                }
                continue;
            };
            let Some(body) = declared_body(&site, &tokens, &observation, registry) else {
                #[cfg(debug_assertions)]
                if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
                    eprintln!(
                        "ORIGINAL_DECLARATION offset={} stage=body-unavailable",
                        site.offset
                    );
                }
                continue;
            };
            // An independently retained installed declaration has its own
            // collector and ownership; never join it with this lexical recipe.
            if self
                .conditional_body_entry_at(&body.source.origin, body.source.base())
                .is_some()
            {
                continue;
            }
            self.retain_declared_body_layouts(body, &observation, registry);
        }
        #[cfg(any(test, debug_assertions))]
        trace.phase("uninstalled-complete", 0, 0, self.declaration_layouts.len());
        self.retain_nested_declared_layouts(registry);
        #[cfg(any(test, debug_assertions))]
        trace.phase("nested-complete", 0, 0, self.declaration_layouts.len());
    }

    fn retain_nested_declared_layouts(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) -> usize {
        let mut visited = HashSet::new();
        self.retain_nested_declared_layouts_with(registry, |source, parents| {
            visited.insert(NestedDeclarationLayoutKey::new(source, parents))
        })
    }

    fn retain_nested_declared_layouts_with(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
        visit: impl FnMut(&ExecutedScriptSource, &[DeclarationLayoutObservation]) -> bool,
    ) -> usize {
        let mut syntax = NestedLayoutSyntax::default();
        let visits =
            self.retain_nested_declared_layouts_using(registry, visit, |source, config| {
                syntax.commands(source, config)
            });
        #[cfg(any(test, debug_assertions))]
        if std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some() {
            let commands: usize = syntax
                .commands
                .values()
                .filter_map(Option::as_ref)
                .map(|commands| commands.len())
                .sum();
            eprintln!(
                "SOURCE_LAYOUT_SYNTAX_SUMMARY visits={visits} inputs={} parsed_commands={commands}",
                syntax.commands.len()
            );
        }
        visits
    }

    fn retain_nested_declared_layouts_using(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
        mut visit: impl FnMut(&ExecutedScriptSource, &[DeclarationLayoutObservation]) -> bool,
        mut parse: impl FnMut(
            &ExecutedScriptSource,
            tcl_lexer::LexerConfig,
        ) -> Option<Arc<[NestedLayoutCommand]>>,
    ) -> usize {
        #[cfg(any(test, debug_assertions))]
        let trace = NestedLayoutTrace::new();
        let roots: Vec<_> = self
            .declaration_layouts
            .iter()
            .filter_map(|(site, observations)| {
                let originals = original_declaration_layouts(observations)?;
                Some((site.clone(), originals.cloned().collect::<Vec<_>>()))
            })
            .collect();
        #[cfg(any(test, debug_assertions))]
        trace.phase(
            "nested-roots",
            0,
            roots.len(),
            self.declaration_layouts.len(),
        );
        let mut pending = Vec::new();
        for (site, observations) in roots {
            let Some(tokens) = declaration_tokens(&site, &observations[0]) else {
                continue;
            };
            pending.extend(unanimous_nested_layouts(
                &site,
                &tokens,
                &observations,
                registry,
            ));
        }
        #[cfg(any(test, debug_assertions))]
        trace.phase(
            "nested-seeds",
            0,
            pending.len(),
            self.declaration_layouts.len(),
        );
        let mut visits: usize = 0;
        while let Some((source, parents)) = pending.pop() {
            if !visit(&source, &parents) {
                continue;
            }
            visits += 1;
            #[cfg(any(test, debug_assertions))]
            if visits.is_multiple_of(64) {
                trace.phase(
                    "nested-visits",
                    visits,
                    pending.len(),
                    self.declaration_layouts.len(),
                );
            }
            let Some(commands) = parse(&source, parents[0].config) else {
                continue;
            };
            for command in commands.iter() {
                if !self.retain_nested_layout_command(command, &parents, registry, &mut pending) {
                    break;
                }
            }
        }
        #[cfg(any(test, debug_assertions))]
        trace.phase(
            "nested-drained",
            visits,
            pending.len(),
            self.declaration_layouts.len(),
        );
        visits
    }

    fn retain_nested_layout_command(
        &mut self,
        command: &NestedLayoutCommand,
        parents: &[DeclarationLayoutObservation],
        registry: &tcl_registry::CommandRegistry,
        pending: &mut Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)>,
    ) -> bool {
        let tokens = command.tokens.as_ref();
        let site = &command.site;
        let observations: Vec<_> = parents
            .iter()
            .map(|parent| DeclarationLayoutObservation {
                words: Arc::from(tokens.words()),
                ..parent.clone()
            })
            .collect();
        let retained = self.declaration_layouts.entry(site.clone()).or_default();
        for observation in &observations {
            if !retained.contains(observation) {
                retained.push(observation.clone());
            }
        }
        let originals = parent_declaration_layouts(site, tokens, &observations, registry);
        pending.extend(unanimous_nested_layouts_from_parents(
            site, &originals, registry,
        ));
        originals.iter().all(|row| {
            row.original
                .as_ref()
                .is_some_and(OriginalDeclarationLayout::retains_successor)
        })
    }

    fn retain_declared_body_layouts(
        &mut self,
        body: DeclaredProcedureBody,
        declaration: &DeclarationLayoutObservation,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let Some(segments) = crate::segmenter::segment_commands_image_with_offset_and_config(
            &body.source.text,
            body.source.base(),
            declaration.config,
        ) else {
            return;
        };
        let mut state = declaration.snapshot.state.clone();
        state.variable_frame = body.frame.clone();
        state.source_variables = Arc::new(
            state
                .source_variables
                .in_frame(&body.frame)
                .with_namespace_identity(body.namespace.clone()),
        );
        if let Some(topology) = &body.original_parameters {
            topology.seed_unknown(Arc::make_mut(&mut state.source_variables), registry);
        } else {
            for parameter in &body.parameters {
                Arc::make_mut(&mut state.source_variables)
                    .bind_unknown_incoming(&parameter.name, registry);
            }
        }
        state.current_source_origin = Some(Arc::clone(&body.source.origin));
        let namespace = body.namespace.clone();
        let snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state,
            declaration.snapshot.realm,
        ));
        let entry = Arc::new(OriginalDiagnosticFrameEntry::DeclaredProcedure(Arc::new(
            body,
        )));
        for segment in segments {
            if segment.is_partial {
                break;
            }
            let tokens = super::source_command_tokens_boxed(
                &entry.source().text,
                entry.source().base(),
                declaration.config,
                &segment,
            );
            let site = CommandAllocationSite {
                source: Arc::clone(&entry.source().origin),
                offset: segment.span.start(),
            };
            let observations = self.declaration_layouts.entry(site.clone()).or_default();
            let observation = DeclarationLayoutObservation {
                issuer: DeclarationLayoutIssuer::OriginalDeclaration,
                entry: Arc::clone(&entry),
                snapshot: Arc::clone(&snapshot),
                namespace: namespace.clone(),
                config: declaration.config,
                words: Arc::from(tokens.words()),
            };
            if !observations.contains(&observation) {
                observations.push(observation);
            }
            if !retains_successor_layout(
                &site,
                &tokens,
                &snapshot,
                &namespace,
                declaration.config,
                registry,
            ) {
                break;
            }
        }
    }
}

fn unanimous_nested_layouts(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    parents: &[DeclarationLayoutObservation],
    registry: &tcl_registry::CommandRegistry,
) -> Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)> {
    let originals = parent_declaration_layouts(site, tokens, parents, registry);
    unanimous_nested_layouts_from_parents(site, &originals, registry)
}

fn unanimous_nested_layouts_from_parents(
    site: &CommandAllocationSite,
    originals: &[ParentDeclarationLayout<'_>],
    registry: &tcl_registry::CommandRegistry,
) -> Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)> {
    let mut agreed: Option<Vec<ExecutedScriptSource>> = None;
    for row in originals {
        let sources = row.original.as_ref().map_or_else(Vec::new, |original| {
            nested_conditional_layouts(site, row.parent, original, registry)
        });
        if agreed.as_ref().is_some_and(|previous| previous != &sources) {
            return Vec::new();
        }
        agreed = Some(sources);
    }
    let parents: Vec<_> = originals.iter().map(|row| row.parent.clone()).collect();
    agreed
        .unwrap_or_default()
        .into_iter()
        .map(|source| (source, parents.clone()))
        .collect()
}

fn nested_conditional_layouts(
    site: &CommandAllocationSite,
    parent: &DeclarationLayoutObservation,
    original: &OriginalDeclarationLayout,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<ExecutedScriptSource> {
    let advice = &original.advice;
    let selected = &original.flow;
    let tcl_registry::script_body_flow::ScriptBodyFlow::Conditional(branches) = &selected.flow
    else {
        return Vec::new();
    };
    // Substitutions in argv precede every condition and may change its lookup.
    if selected.effective.words.iter().any(|word| {
        effective_invocation_word(word, parent.config.escapes, advice.dialect().word_values)
            .literal_bytes()
            .is_none()
    }) {
        return Vec::new();
    }
    let mut children = Vec::new();
    for &(condition, body) in branches {
        if condition.is_some_and(|argument| {
            !condition_retains_original_lookup(
                site,
                argument,
                &selected.effective,
                parent,
                advice,
                registry,
            )
        }) {
            break;
        }
        if let Some(source) = original_nested_body(site, body, &selected.effective, parent, advice)
        {
            children.push(source);
        }
    }
    children
}

fn original_nested_body(
    site: &CommandAllocationSite,
    argument: usize,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    parent: &DeclarationLayoutObservation,
    advice: &super::OriginalCompilationLookupAdvice,
) -> Option<ExecutedScriptSource> {
    let written = effective.written_argument(argument)?;
    let word = effective.words.get(argument + 1)?;
    let EffectiveInvocationWord::Literal(value) =
        effective_invocation_word(word, parent.config.escapes, advice.dialect().word_values)
    else {
        return None;
    };
    let source =
        ExecutedScriptSource::from_word(site.clone(), written, word, &value, parent.config);
    let super::ExecutedScriptMapping::Contiguous { base } = source.mapping else {
        return None;
    };
    let end = base.checked_add(u32::try_from(source.text.len()).ok()?)?;
    (source.origin == site.source
        && base > site.offset
        && parent.entry.owns_source(&source.origin, base)
        && end
            .checked_sub(1)
            .is_some_and(|last| parent.entry.owns_source(&source.origin, last)))
    .then_some(source)
}

fn condition_retains_original_lookup(
    site: &CommandAllocationSite,
    argument: usize,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    parent: &DeclarationLayoutObservation,
    advice: &super::OriginalCompilationLookupAdvice,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    use tcl_syntax::expr::ast::ExprNode;
    let Some(source) = original_nested_body(site, argument, effective, parent, advice) else {
        return false;
    };
    let Ok(text) = source.text.try_text() else {
        return false;
    };
    let tcl_syntax::expr::parser::CheckedExprParse::Parsed(expression) =
        tcl_syntax::expr::parser::parse_expr_checked_with_context(
            text,
            &advice.dialect().expression_parse_context(None),
        )
    else {
        return false;
    };
    let context = advice.original_variable_context();
    if context.dynamic_traces {
        return false;
    }
    let mut pending = vec![&expression];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } => {}
            ExprNode::Var { text, name, .. } => {
                let reference =
                    tcl_lexer::word_parts::whole_var_ref(text.as_bytes(), parent.config);
                if !matches!(reference, Ok(Some(reference)) if reference.index.is_none())
                    || tcl_syntax::naming::is_qualified(name.as_bytes())
                    || super::declaration_layout::local_read_scope_is_excluded(context, name)
                    || crate::script_binds::script_image_binds_name(
                        &parent.entry.source().text,
                        name,
                        crate::script_binds::Ownership::ScopeAliases,
                        registry,
                        parent.config,
                    )
                {
                    return false;
                }
            }
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]);
            }
            _ => return false,
        }
    }
    true
}

pub(super) fn declaration_tokens(
    site: &CommandAllocationSite,
    observation: &DeclarationLayoutObservation,
) -> Option<crate::ir::CommandTokens> {
    let text = site.source.try_text().ok()?;
    let first = tcl_lexer::word_span_at(text, observation.words.first()?.source().span);
    let last = tcl_lexer::word_span_at(text, observation.words.last()?.source().span);
    if first.start() != site.offset || first.start() > last.end() {
        return None;
    }
    let image = tcl_lexer::SourceImage::from_bytes(
        text.as_bytes()
            .get(first.start() as usize..last.end() as usize)?,
        site.source.source_image().channel(),
    );
    let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
        &image,
        first.start(),
        observation.config,
    )?;
    let [segment] = segments.as_slice() else {
        return None;
    };
    let tokens =
        super::source_command_tokens_boxed(&image, first.start(), observation.config, segment);
    (tokens.words() == observation.words.as_ref()).then_some(*tokens)
}

fn declared_body(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
) -> Option<DeclaredProcedureBody> {
    let advice = super::original_site_operand_layout_advice(
        site,
        tokens,
        &observation.snapshot,
        &observation.namespace,
        observation.config,
    );
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
        eprintln!(
            "ORIGINAL_DECLARATION offset={} stage=body-advice available={}",
            site.offset,
            advice.is_some()
        );
    }
    let advice = advice?;
    let dialect = advice.dialect();
    let mut agreed = None;
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm(registry, None, &words, Some(dialect), advice.realm())
                .ok()?
        else {
            return None;
        };
        let recipe = procedure_body_recipe(
            site,
            tokens,
            observation,
            &facts,
            target,
            &ProcedureOperands {
                effective: &effective,
                values: &values,
                dialect,
            },
        );
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
            eprintln!(
                "ORIGINAL_DECLARATION offset={} stage=procedure-recipe available={} arity={:?} roles={}",
                site.offset,
                recipe.is_some(),
                facts.arity_accepts_frozen_arguments(),
                facts.arg_roles_complete
            );
        }
        let recipe = recipe?;
        if agreed.as_ref().is_some_and(|old| old != &recipe) {
            return None;
        }
        agreed = Some(recipe);
    }
    agreed
}

struct ProcedureOperands<'a> {
    effective: &'a crate::registry_invocation::EffectiveCommandWords,
    values: &'a [EffectiveInvocationWord],
    dialect: tcl_registry::InvocationDialect,
}

/// A readonly source operand follows its actual effective ordinal. It does
/// not acquire installation, entered-frame or Normal authority here.
fn original_declaration_operand_input(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    candidate: &super::SourceCommandTarget,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    index: usize,
) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
    use crate::registry_invocation::InvocationWordOrigin;
    use crate::signature_scan::scope::{
        SignatureSourceNameInput, SignatureSourceNameKey, SignatureSourceNameValue,
    };
    let variables = &observation.snapshot.state.source_variables;
    let policy = variables.execution_name_policy?.native_recipe()?;
    let origin = effective.origins.get(index)?;
    if let InvocationWordOrigin::BindingPrefix(prefix) = origin {
        let input = candidate.original_prepended_name_inputs()?.get(*prefix)?;
        return (input.policy() == policy && input.is_current(variables)).then(|| input.clone());
    }
    let native = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        observation.config,
    )?;
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&observation.config);
    match origin {
        InvocationWordOrigin::Written(written) => {
            SignatureSourceNameKey::from_original_native_word(native.get(*written)?, rules, policy)
                .map(SignatureSourceNameInput::OriginalWord)
        }
        InvocationWordOrigin::ExpandedElement { written, element } => {
            let parent = SignatureSourceNameInput::OriginalValue(
                SignatureSourceNameValue::from_original_static_word(
                    native.get(*written)?,
                    rules,
                    policy,
                )?,
            );
            let children = parent.original_list_elements()?;
            let projected = effective.origins.iter().filter_map(|origin| match origin {
                InvocationWordOrigin::ExpandedElement {
                    written: parent,
                    element,
                } if parent == written => Some(*element),
                _ => None,
            });
            projected.eq(0..children.len()).then_some(())?;
            children.get(*element).cloned()
        }
        InvocationWordOrigin::ResolvedHead | InvocationWordOrigin::BindingPrefix(_) => None,
    }
}

fn procedure_body_recipe(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    facts: &tcl_registry::InvocationFacts,
    candidate: &super::SourceCommandTarget,
    operands: &ProcedureOperands<'_>,
) -> Option<DeclaredProcedureBody> {
    use tcl_registry::{CommandBindingDefinitionKind, CommandBindingTransition};
    let ProcedureOperands {
        effective,
        values,
        dialect,
    } = *operands;
    if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    let name_argument = facts
        .state_transitions
        .declared()?
        .command_bindings()
        .find_map(|transition| {
            if let CommandBindingTransition::Define {
                name,
                kind: CommandBindingDefinitionKind::Procedure,
            } = transition
            {
                name.argument_index()
            } else {
                None
            }
        })?;
    let name = original_declaration_operand_input(
        site,
        tokens,
        observation,
        candidate,
        effective,
        name_argument.checked_add(1)?,
    )?;
    if !observation.entry.owns_source(&site.source, site.offset)
        || !observation
            .entry
            .owns_original_context(&observation.snapshot.state.source_variables)
        || observation.snapshot.state.variable_frame != *observation.entry.frame()
        || observation.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
    {
        return None;
    }
    let key = observation
        .snapshot
        .state
        .original_conditional_procedure_key(&observation.namespace, &name)?;
    let namespace = key.holder().into_owned();
    let words: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let arguments =
        tcl_registry::InvocationArguments::structured(words.get(1..)?).with_dialect(dialect);
    let original_parameters =
        original_procedure_parameter_topology(site, tokens, observation, facts, effective, dialect);
    let parameters = original_parameters
        .as_ref()
        .map(super::formal_topology::OriginalFormalTopology::advisory_parameters)
        .or_else(|| super::native_procedure_parameters(facts, arguments))?;
    let body_index = facts.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::Body)
            .then_some(facts.argument_offset + usize::from(*index) + 1)
    })?;
    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
        effective.origins.get(body_index)?
    else {
        return None;
    };
    let EffectiveInvocationWord::Literal(value) = values.get(body_index)? else {
        return None;
    };
    let source = ExecutedScriptSource::from_word(
        site.clone(),
        written.checked_sub(1)?,
        tokens.words().get(*written)?,
        value,
        observation.config,
    );
    if source.origin != site.source {
        return None;
    }
    let frame = declared_procedure_frame(site, &namespace);
    Some(DeclaredProcedureBody {
        declaration: site.clone(),
        candidate: candidate.clone(),
        slot: key,
        source,
        namespace,
        parameters,
        original_parameters,
        frame,
    })
}

fn declared_procedure_frame(
    site: &CommandAllocationSite,
    namespace: &super::SourceNamespaceKey,
) -> crate::var_resolve::VariableExecutionFrame {
    crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: namespace.display().unwrap_or_default(),
        identity: super::source_activation_name(
            Some(&site.source),
            "declared-procedure-layout",
            site.offset,
        ),
    }
    .with_namespace_identity(namespace.clone())
}

fn original_procedure_parameter_topology(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    observation: &DeclarationLayoutObservation,
    facts: &tcl_registry::InvocationFacts,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    dialect: tcl_registry::InvocationDialect,
) -> Option<super::formal_topology::OriginalFormalTopology> {
    facts
        .arg_roles
        .iter()
        .find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::ParamList)
                .then_some(facts.argument_offset + usize::from(*index))
        })
        .and_then(|argument| effective.written_argument(argument))
        .and_then(|written| {
            super::formal_topology::capture(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                written + 1,
                observation.config,
                dialect,
                observation
                    .snapshot
                    .state
                    .source_variables
                    .execution_name_policy?
                    .native_recipe()?,
            )
        })
}

fn retains_successor_layout(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &super::SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    OriginalDeclarationLayout::select(site, tokens, snapshot, namespace, config, registry)
        .as_ref()
        .is_some_and(OriginalDeclarationLayout::retains_successor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory(source: &str, command: &str) -> (SourceCommandBindings, crate::ir::CommandTokens) {
        inventory_in_world(source, command, false)
    }

    fn inventory_in_world(
        source: &str,
        command: &str,
        unknown_entry: bool,
    ) -> (SourceCommandBindings, crate::ir::CommandTokens) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl8.6")).unwrap(),
        );
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                unknown_entry,
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find(command).unwrap()).unwrap();
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(command, offset, config)
                .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        (bindings, tokens)
    }

    #[test]
    fn original_registry_advice_uses_uncalled_body_layout_without_normal_receipt() {
        // Implementation contract: naming.source.original-registry-header-advice
        // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, command, expected) in [
            (
                "proc p {arg} {set evenWhenWritingCode}",
                "set evenWhenWritingCode",
                "set",
            ),
            ("proc p {} \"puts hi\"", "puts hi", "puts"),
        ] {
            let (_, tokens) = inventory(source, command);
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(
                binding.original_normal_result(&tokens).is_none(),
                "{source}"
            );
            assert!(
                !binding.original_invocation_completes_normally(&tokens),
                "{source}"
            );
            let transfer_available =
                crate::registry_invocation::normal_transfer_invocation(registry, None, &tokens)
                    .is_some();
            let advice = crate::registry_invocation::original_registry_invocation_assistance(
                registry, None, &tokens,
            )
            .expect("the original body layout supplies conditional metadata");
            assert_eq!(
                advice.unanimous_command_words().unwrap().command(),
                expected
            );
            assert_eq!(
                crate::registry_invocation::normal_transfer_invocation(registry, None, &tokens,)
                    .is_some(),
                transfer_available,
                "{source}"
            );
            assert!(
                binding.original_normal_result(&tokens).is_none(),
                "{source}"
            );
            assert!(
                !binding.original_invocation_completes_normally(&tokens),
                "{source}"
            );
            let mut synthetic = tokens.clone();
            synthetic.source_binding = None;
            assert!(
                crate::registry_invocation::original_registry_invocation_assistance(
                    registry, None, &synthetic,
                )
                .is_none()
            );
        }
    }

    #[test]
    fn original_registry_advice_retains_body_shadowing_and_original_word_currency() {
        // Implementation contract: naming.source.original-registry-header-advice
        // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (_, tokens) = inventory(
            "proc set args {return CUSTOM}; proc p {} {set value}",
            "set value",
        );
        assert!(
            crate::registry_invocation::original_registry_invocation_assistance(
                registry, None, &tokens,
            )
            .is_none_or(|advice| advice.unanimous_command_words().is_none())
        );
        let (_, mut changed) = inventory("proc p {} {puts hi}", "puts hi");
        changed.argv_texts[0] = "reporting-counterfactual".to_owned();
        let readonly = crate::registry_invocation::original_registry_invocation_assistance(
            registry, None, &changed,
        )
        .unwrap();
        assert_eq!(
            readonly.unanimous_command_words().unwrap().command(),
            "puts"
        );
        let crate::ir::WordExpr::Literal { text, .. } = &mut changed.word_exprs[0] else {
            panic!("a bare literal original head");
        };
        *text = "set".to_owned();
        assert!(
            crate::registry_invocation::original_registry_invocation_assistance(
                registry, None, &changed,
            )
            .is_none_or(|advice| advice.unanimous_command_words().is_none())
        );
    }

    #[test]
    fn original_declaration_procedure_name_keeps_native_operand_identity() {
        // Implementation contract: naming.source.original-declaration-name-publication (docs/design/analysis/name-resolution-proofs/original-declaration-name-publication.md).
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let owner = tcl_registry::model::ingress::static_context_for(engine);
            let registry = owner.commands();
            let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
            let source = r"proc p\uD800 {value} {set local $value}";
            let bindings = SourceCommandBindings::analyse(source, config, registry);
            let site = CommandAllocationSite {
                source: Arc::clone(bindings.source_origin().unwrap()),
                offset: 0,
            };
            let observations =
                original_declaration_layouts(bindings.declaration_layouts.get(&site).unwrap())
                    .unwrap();
            let observation = observations.clone().next().unwrap();
            let tokens = declaration_tokens(&site, observation).unwrap();
            let recipe = declared_body(&site, &tokens, observation, registry)
                .expect("original native name is not a reporting string");
            let native = crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                config,
            )
            .unwrap();
            let key =
                crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                    &native[1],
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    observation
                        .snapshot
                        .state
                        .source_variables
                        .execution_name_policy
                        .unwrap()
                        .native_recipe()
                        .unwrap(),
                )
                .unwrap();
            let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key);
            assert_eq!(
                recipe.slot,
                observation
                    .snapshot
                    .state
                    .original_publication_key(
                        &observation.namespace,
                        &input,
                        super::super::namespace_slots::PublicationPurpose::Procedure
                    )
                    .unwrap()
            );
            assert_eq!(recipe.source.origin, site.source);
            assert!(recipe.original_parameters.is_some());
            let dynamic = r"proc $unknown {value} {set local $value}";
            let bindings = SourceCommandBindings::analyse(dynamic, config, registry);
            let site = CommandAllocationSite {
                source: Arc::clone(bindings.source_origin().unwrap()),
                offset: 0,
            };
            if let Some(observation) = bindings
                .declaration_layouts
                .get(&site)
                .and_then(|rows| original_declaration_layouts(rows))
                .and_then(|mut rows| rows.next())
            {
                let tokens = declaration_tokens(&site, observation).unwrap();
                assert!(declared_body(&site, &tokens, observation, registry).is_none());
            }
        }
    }

    #[test]
    fn conditional_declaration_geometry_survives_unknown_table_without_installation() {
        // Implementation contract: naming.source.original-declaration-name-publication
        // docs/design/analysis/name-resolution-proofs/original-declaration-name-publication.md
        let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (bindings, tokens) = inventory_in_world(
            source,
            "proc wrap {arg} {upvar 1 other local; return $local}",
            true,
        );
        let site = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap();
        let observation =
            original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
                .unwrap()
                .next()
                .unwrap();
        assert!(observation.snapshot.state.has_opaque_domain());
        let recipe = declared_body(site, &tokens, observation, registry)
            .expect("genuine declaration geometry is independent of current table existence");
        assert_eq!(recipe.namespace.display().as_deref(), Some("::"));
        assert_eq!(
            recipe
                .original_parameters
                .as_ref()
                .unwrap()
                .advisory_parameters()[0]
                .name,
            "arg"
        );
        assert!(
            observation
                .snapshot
                .state
                .installed_procedure_targets(&recipe.slot, site.offset)
                .is_empty()
        );
        assert!(!bindings.has_actual_procedure_entry(&recipe.source));
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .proved_execution_target()
                .is_none()
        );
        let mut foreign = observation.clone();
        foreign.snapshot = Arc::new(SourceLookupSnapshot::new(
            super::super::ModuleCommandBindings::initial(registry),
        ));
        assert!(declared_body(site, &tokens, &foreign, registry).is_none());
    }

    #[test]
    fn nested_layout_visits_process_each_exact_source_world_once() {
        // Implementation contract: naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Compare seven genuine nested conditionals with the uncached
        // worklist. Counts describe lexical segmentation, not body execution
        // or elapsed time; all retained semantic observations must agree.
        let source = "if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {if {1} {puts done}}}}}}}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (mut cached, _) = inventory(source, "puts done");
        let mut uncached = cached.clone();
        let cached_visits = cached.retain_nested_declared_layouts(registry);
        let uncached_visits = uncached.retain_nested_declared_layouts_with(registry, |_, _| true);
        assert_eq!(cached_visits, 7);
        assert_eq!(uncached_visits, 28);
        assert_eq!(cached, uncached);
    }

    #[test]
    fn exact_parent_layout_reuse_keeps_guarded_children_and_successor_decisions() {
        // Implementation contract: naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Genuine original observations compare one shared projection with
        // independently rederived children and continuation. No elapsed-time,
        // execution, installed implementation or Native frame claim follows.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, command, child_count, successor) in [
            ("if {1} {puts done}", "if {1} {puts done}", 1, false),
            ("list done; list later", "list done", 0, true),
            (
                "set local $incoming; puts later",
                "set local $incoming",
                0,
                false,
            ),
            (
                "rename puts saved; puts later",
                "rename puts saved",
                0,
                false,
            ),
            (
                "if {[rename puts saved; expr 1]} {puts done}",
                "if {[rename puts saved; expr 1]} {puts done}",
                0,
                false,
            ),
            (
                "proc wrap {condition} {if {$condition} {puts done}}",
                "if {$condition} {puts done}",
                1,
                false,
            ),
        ] {
            let (bindings, tokens) = inventory(source, command);
            let site = tokens
                .source_binding
                .as_ref()
                .unwrap()
                .invocation_site()
                .unwrap();
            let parents: Vec<_> =
                original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
                    .unwrap()
                    .cloned()
                    .collect();
            assert!(!parents.is_empty(), "{source}");
            let originals = parent_declaration_layouts(site, &tokens, &parents, registry);
            let children = unanimous_nested_layouts_from_parents(site, &originals, registry);
            assert_eq!(
                children,
                unanimous_nested_layouts(site, &tokens, &parents, registry),
                "{source}"
            );
            assert_eq!(children.len(), child_count, "{source}");
            let shared = originals.iter().all(|row| {
                row.original
                    .as_ref()
                    .is_some_and(OriginalDeclarationLayout::retains_successor)
            });
            let repeated = parents.iter().all(|parent| {
                retains_successor_layout(
                    site,
                    &tokens,
                    &parent.snapshot,
                    &parent.namespace,
                    parent.config,
                    registry,
                )
            });
            assert_eq!(shared, repeated, "{source}");
            assert_eq!(shared, successor, "{source}");
        }
    }

    #[test]
    fn exact_parent_layout_reuse_refuses_lost_original_source_or_frame() {
        // Implementation contract: naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // These changed-premise API controls cannot manufacture a provider
        // execution result from an original layout's reporting fields.
        let source = "if {1} {puts done}";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (bindings, tokens) = inventory(source, source);
        let site = tokens
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap();
        let parents: Vec<_> =
            original_declaration_layouts(bindings.declaration_layouts.get(site).unwrap())
                .unwrap()
                .cloned()
                .collect();
        let original = &parents[0].snapshot.state;
        let mut lost_source = original.clone();
        lost_source.current_source_origin = None;
        let mut lost_frame = original.clone();
        lost_frame.variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
        let mut lost_dialect = original.clone();
        Arc::make_mut(&mut lost_dialect.source_variables).invocation_dialect = None;
        for state in [lost_source, lost_frame, lost_dialect] {
            let mut changed = parents.clone();
            changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
                state,
                changed[0].snapshot.realm,
            ));
            let rows = parent_declaration_layouts(site, &tokens, &changed, registry);
            assert!(rows[0].original.is_none());
            assert!(unanimous_nested_layouts_from_parents(site, &rows, registry).is_empty());
            assert!(!rows.iter().all(|row| {
                row.original
                    .as_ref()
                    .is_some_and(OriginalDeclarationLayout::retains_successor)
            }));
        }
    }

    fn nested_layout_key_fixture() -> (ExecutedScriptSource, Vec<DeclarationLayoutObservation>) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (bindings, _) = inventory("if {1} {puts done}", "puts done");
        let (site, rows) = bindings.declaration_layouts.iter().next().unwrap();
        let parents: Vec<_> = original_declaration_layouts(rows)
            .unwrap()
            .cloned()
            .collect();
        let tokens = declaration_tokens(site, &parents[0]).unwrap();
        unanimous_nested_layouts(site, &tokens, &parents, registry).remove(0)
    }

    #[test]
    fn nested_layout_syntax_shares_only_exact_original_geometry() {
        // naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Pure lexical sharing, independently of any Native lookup/frame/header claim.
        let (source, parents) = nested_layout_key_fixture();
        let config = parents[0].config;
        let mut syntax = NestedLayoutSyntax::default();
        let first = syntax.commands(&source, config).unwrap();
        let second = syntax.commands(&source, config).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(syntax.parsed_scripts, 1);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].site.source, source.origin);
        assert_eq!(first[0].site.offset, source.base());
        assert_eq!(first[0].tokens.argv_texts, ["puts", "done"]);
        assert!(first[0].tokens.source_binding.is_none());
        assert!(first[0].tokens.variable_accesses.is_empty());
        assert!(first[0].tokens.nested_bindings.is_empty());

        let mut changed_config = config;
        changed_config.strict_quoting = !config.strict_quoting;
        let changed = syntax.commands(&source, changed_config).unwrap();
        assert!(!Arc::ptr_eq(&first, &changed));
        let materialised = ExecutedScriptSource::materialised(
            CommandAllocationSite {
                source: Arc::clone(&source.origin),
                offset: source.base(),
            },
            vec![0],
            source.try_text().unwrap(),
        );
        let changed = syntax.commands(&materialised, config).unwrap();
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(changed[0].site.source, materialised.origin);
        assert_eq!(changed[0].site.offset, 0);
        assert_eq!(changed[0].tokens.argv[0].start(), 0);
        let mut changed_channel = source.clone();
        changed_channel.text = tcl_lexer::SourceImage::document(source.try_text().unwrap());
        let changed = syntax.commands(&changed_channel, config).unwrap();
        assert!(!Arc::ptr_eq(&first, &changed));
        assert_eq!(syntax.parsed_scripts, 4);
    }

    #[test]
    fn nested_layout_syntax_reuse_keeps_original_owner_refusal() {
        // naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // A retained pure tape cannot fill a missing source or frame owner.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (source, parents) = nested_layout_key_fixture();
        let mut syntax = NestedLayoutSyntax::default();
        let tape = syntax.commands(&source, parents[0].config).unwrap();
        let command = &tape[0];
        let rows = parent_declaration_layouts(&command.site, &command.tokens, &parents, registry);
        assert!(rows[0].original.is_some());
        let original = &parents[0].snapshot.state;
        let mut lost_source = original.clone();
        lost_source.current_source_origin = None;
        let mut lost_frame = original.clone();
        lost_frame.variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
        for state in [lost_source, lost_frame] {
            let mut changed = parents.clone();
            changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
                state,
                changed[0].snapshot.realm,
            ));
            let rows =
                parent_declaration_layouts(&command.site, &command.tokens, &changed, registry);
            assert!(rows[0].original.is_none());
        }
        assert_eq!(syntax.parsed_scripts, 1);
    }

    #[test]
    fn nested_layout_syntax_memo_matches_original_parser_world_inventory() {
        // naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Compare complete retained layouts; neither path claims entered execution.
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for source in [
            "if {1} {if {1} {if {1} {puts done}}}",
            "proc p {condition} {if {$condition} {if {1} {puts done}}}",
            "proc p {condition} {if {[rename puts saved; expr 1]} {puts done}}",
        ] {
            let (mut cached, _) = inventory(source, "puts done");
            let mut original = cached.clone();
            let cached_visits = cached.retain_nested_declared_layouts(registry);
            let mut visited = HashSet::new();
            let original_visits = original.retain_nested_declared_layouts_using(
                registry,
                |source, parents| visited.insert(NestedDeclarationLayoutKey::new(source, parents)),
                NestedLayoutSyntax::parse,
            );
            assert_eq!(cached_visits, original_visits, "{source}");
            assert_eq!(cached, original, "{source}");
        }
    }

    #[test]
    fn nested_layout_visit_key_keeps_complete_lookup_and_owner_facets() {
        // Implementation contract: naming.source.original-declaration-layout-visits
        // docs/design/analysis/name-resolution-proofs/source-original-declaration-layout-visits.md
        // Changing a semantic facet is a distinct source-contract question;
        // these key controls do not manufacture an entered Native frame.
        let (source, parents) = nested_layout_key_fixture();
        let mut visited = HashSet::new();
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &parents)));
        let mut changed = parents.clone();
        changed[0].words = Arc::from([]);
        assert!(!visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].issuer = DeclarationLayoutIssuer::EnteredActivation;
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].namespace = super::super::SourceNamespaceKey::authored("::other");
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].config.expand_syntax = !changed[0].config.expand_syntax;
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            changed[0].snapshot.state.clone(),
            tcl_dialect::model::InvocationRealm::InterpreterRuntime,
        ));
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        let mut state = changed[0].snapshot.state.clone();
        state.mark_opaque_binding_mutation();
        changed[0].snapshot = Arc::new(SourceLookupSnapshot::in_realm(
            state,
            changed[0].snapshot.realm,
        ));
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let mut changed = parents.clone();
        changed[0].entry = Arc::new(OriginalDiagnosticFrameEntry::ScriptActivation {
            source: changed[0].entry.source().clone(),
            frame: changed[0].entry.frame().clone(),
            namespace: changed[0].namespace.clone(),
            config: changed[0].config,
        });
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&source, &changed)));
        let materialised = ExecutedScriptSource::materialised(
            CommandAllocationSite {
                source: Arc::clone(&source.origin),
                offset: source.base(),
            },
            vec![0],
            source.try_text().unwrap(),
        );
        assert!(visited.insert(NestedDeclarationLayoutKey::new(&materialised, &parents)));
        assert_eq!(visited.len(), 8);
    }

    #[test]
    fn nested_declared_layout_keeps_original_scope_without_entered_authority() {
        let source = "proc wrap {condition} {if {$condition} {info exists condition}}";
        let (bindings, tokens) = inventory_in_world(source, "info exists condition", true);
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("unchanged scalar condition retains original nested grammar");
        assert!(!layout.closed_lookup());
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let site = binding.invocation_site().unwrap();
        let lexical_body = bindings
            .conditional_body_entry_at(&site.source, site.offset)
            .expect("independent unchanged declaration body recipe");
        assert!(lexical_body.owns_source(&site.source, site.offset));
        assert!(lexical_body.matches_parameters(&["condition"]));
        assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
        let originals = original_declaration_layouts(
            binding.declaration_layout_observations.as_deref().unwrap(),
        )
        .unwrap();
        assert!(originals.clone().all(|observation| {
            observation.entry.owns_source(&site.source, site.offset)
                && observation
                    .entry
                    .owns_original_context(&observation.snapshot.state.source_variables)
        }));
    }

    #[test]
    fn nested_declared_layout_stops_before_condition_callbacks_or_aliases() {
        for body in [
            "if {[rename info saved; expr 1]} {info exists condition}",
            "if {$array(index)} {info exists condition}",
            "if {[expr {1}]} {info exists condition}",
            "upvar 1 outer condition; if {$condition} {info exists condition}",
            "trace add variable condition read replaceInfo; if {$condition} {info exists condition}",
        ] {
            let source = format!("proc wrap {{condition}} {{{body}}}");
            let (_, tokens) = inventory_in_world(&source, "info exists condition", true);
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_operand_layout_advice(&tokens)
                    .is_none(),
                "condition lookup is unavailable after: {body}"
            );
        }
    }

    #[test]
    fn uninstalled_body_layout_retains_original_candidate_without_runtime_authority() {
        let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
        let (bindings, tokens) = inventory_in_world(source, "upvar 1 other local", true);
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("conditional original grammar");
        assert!(!layout.closed_lookup());
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let initial = super::super::ModuleCommandBindings::initial(registry);
        let original = super::super::source_binding(
            &initial,
            "upvar",
            &super::super::SourceNamespaceKey::authored("::"),
        );
        assert!(layout.targets().contains(original.proved_target().unwrap()));
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let (_, original_words, original_offset) = binding
            .original_compiler_source(&tokens)
            .expect("unchanged lexical vector is geometry, not compilation admission");
        assert_eq!(original_words, tokens.words());
        assert_eq!(
            original_offset,
            u32::try_from(source.find("upvar").unwrap()).unwrap()
        );
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
        let site = binding.invocation_site().unwrap();
        let lexical_body = bindings
            .conditional_body_entry_at(&site.source, site.offset)
            .expect("independent unchanged declaration body recipe");
        assert!(lexical_body.owns_source(&site.source, site.offset));
        assert!(lexical_body.matches_parameters(&["arg"]));
        assert!(!bindings.has_actual_procedure_entry_at(&site.source, site.offset));
        let observations = binding.declaration_layout_observations.as_deref().unwrap();
        let entry = &original_declaration_layouts(observations)
            .unwrap()
            .next()
            .unwrap()
            .entry;
        let declaration = match entry.as_ref() {
            OriginalDiagnosticFrameEntry::Body(body) => {
                assert_eq!(body.as_ref(), lexical_body.as_ref());
                &body.allocation().site
            }
            OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => &body.declaration,
            _ => panic!("independent procedure declaration owner"),
        };
        assert_eq!(entry.parameters()[0].name, "arg");
        assert_eq!(
            declaration.offset,
            u32::try_from(source.find("proc wrap").unwrap()).unwrap()
        );
        assert_eq!(
            entry.source().try_text().unwrap(),
            "upvar 1 other local; return $local"
        );
        assert_eq!(
            layout
                .snapshot
                .state
                .source_variables
                .namespace_identity
                .as_ref(),
            lexical_body.namespace_context()
        );
        assert!(entry.owns_original_context(&layout.snapshot.state.source_variables));
        let mut changed = tokens.clone();
        changed.word_exprs.pop();
        assert!(
            binding
                .declaration_operand_layout_advice(&changed)
                .is_none()
        );
    }

    #[test]
    fn abrupt_prefix_retains_declaration_body_geometry_without_installation() {
        let source = "rename $old decl\nproc wrap {arg} {upvar 1 other local; return $local}";
        let (bindings, tokens) = inventory(source, "upvar 1 other local");
        let binding = tokens.source_binding.as_ref().unwrap();
        let layout = binding
            .declaration_operand_layout_advice(&tokens)
            .expect("unchanged abrupt table retains original grammar");
        assert!(
            layout.closed_lookup(),
            "the skipped rename did not change the borrowed table"
        );
        assert!(binding.unknown);
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        let site = binding.invocation_site().unwrap();
        assert!(
            bindings
                .conditional_body_entry_at(&site.source, site.offset)
                .is_none()
        );
        assert!(bindings.entered_scripts.keys().all(
            |parent| parent.offset != u32::try_from(source.find("proc wrap").unwrap()).unwrap()
        ));
    }

    #[test]
    fn public_declaration_consumer_retains_original_authored_body_namespace() {
        let source = "rename $old decl\nproc wrap {} {\nupvar 1 other local\nreturn $local\n}\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let realm = crate::realm::document_realm_bindings(source, profile, &registry);
        let offset = u32::try_from(source.find("upvar").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            "upvar 1 other local",
            offset,
            config,
        )
        .remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        realm.stamp_original_tokens(&mut tokens);
        let assistance = realm
            .original_declaration_assistance(&tokens, &registry)
            .expect("same original authored namespace and lexical declaration owner");
        assert_eq!(
            assistance.declarations,
            vec![crate::registry_invocation::DeclarationArgument {
                argument: 2,
                name: "local".to_owned(),
            },]
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.proved_execution_target().is_none());
        let (_, original_words, original_offset) = binding
            .original_compiler_source(&tokens)
            .expect("conditional declaration retains only its original vector");
        assert_eq!(original_words, tokens.words());
        assert_eq!(original_offset, offset);
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn uninstalled_body_layout_stops_at_original_lookup_changes() {
        for prefix in [
            "rename upvar replacement",
            "proc upvar args {}",
            "unknown_call",
        ] {
            let source =
                format!("rename $old decl\nproc wrap {{}} {{{prefix}; upvar 1 other local}}");
            let (_, tokens) = inventory(&source, "upvar 1 other local");
            assert!(
                tokens
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .declaration_operand_layout_advice(&tokens)
                    .is_none(),
                "{prefix}"
            );
        }
    }
}
