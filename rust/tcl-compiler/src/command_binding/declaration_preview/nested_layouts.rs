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

//! Layouts retained for the commands nested in a declared body, and when parents agree on them.

use super::super::declaration_layout::{
    DeclarationLayoutIssuer, DeclarationLayoutObservation, OriginalDiagnosticFrameEntry,
    original_declaration_layouts,
};
use super::super::{SourceCommandBindings, SourceLookupSnapshot};
use super::declared_body::declared_body;
use super::procedure_header::declaration_tokens;
use super::{
    DeclaredProcedureBody, NestedLayoutCommand, NestedLayoutSyntax, OriginalDeclarationLayout,
};
use crate::command_binding::{ExecutedScriptWords, SourceOriginLexical};
use crate::registry_invocation::{EffectiveInvocationWord, effective_invocation_word};
use std::collections::HashMap;
use std::sync::Arc;
use tcl_names::command::CommandAllocationSite;
use tcl_names::origin::ExecutedScriptSource;

/// One lexical body under its complete immutable parent lookup worlds.
/// The registry is fixed for this collector. Parent command words are replaced
/// by genuine child words before any semantic query, so they are not a body
/// traversal input. Entry ownership, issuer, realm, state, namespace and lexer
/// config remain in structural equality; hashes alone never select a visit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct NestedDeclarationLayoutKey {
    pub(super) source: ExecutedScriptSource,
    pub(super) parents: Vec<DeclarationLayoutObservation>,
}

impl NestedDeclarationLayoutKey {
    pub(super) fn new(
        source: &ExecutedScriptSource,
        parents: &[DeclarationLayoutObservation],
    ) -> Self {
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

/// Hashes select lookup buckets only. Complete sealed source/parent equality
/// decides each visit; derived memo cells stay in values, never hash-map keys.
#[derive(Default)]
pub(super) struct NestedDeclarationLayoutVisits {
    buckets: HashMap<u64, Vec<NestedDeclarationLayoutKey>>,
}

impl NestedDeclarationLayoutVisits {
    pub(super) fn insert(&mut self, key: NestedDeclarationLayoutKey) -> bool {
        use std::hash::{BuildHasher, Hash, Hasher};
        // The bucket reads the body and each parent's cheap fields only.
        // Hashing a parent's lookup world reads the whole command world, and
        // the equality in the bucket compares those worlds in any case.
        let mut hasher = self.buckets.hasher().build_hasher();
        key.source.hash(&mut hasher);
        for parent in &key.parents {
            parent.issuer.hash(&mut hasher);
            parent.namespace.hash(&mut hasher);
            parent.config.hash(&mut hasher);
        }
        self.insert_in_bucket(hasher.finish(), key)
    }

    pub(super) fn insert_in_bucket(
        &mut self,
        fingerprint: u64,
        key: NestedDeclarationLayoutKey,
    ) -> bool {
        let bucket = self.buckets.entry(fingerprint).or_default();
        if bucket.contains(&key) {
            return false;
        }
        bucket.push(key);
        true
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.buckets.values().map(Vec::len).sum()
    }
}

/// Pure original source geometry. Lookup worlds remain separate traversal inputs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct NestedLayoutSyntaxKey {
    pub(super) source: ExecutedScriptSource,
    pub(super) config: tcl_lexer::LexerConfig,
}

impl NestedLayoutCommand {
    pub(super) fn new(site: CommandAllocationSite, tokens: Box<crate::ir::CommandTokens>) -> Self {
        let words = Arc::from(tokens.words());
        Self {
            site,
            tokens,
            words,
        }
    }
}

impl NestedLayoutSyntax {
    pub(super) fn commands(
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

    pub(super) fn parse(
        source: &ExecutedScriptSource,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<[NestedLayoutCommand]>> {
        let segments = nested_layout_segments(source, config)?;
        Some(
            segments
                .iter()
                .take_while(|segment| !segment.is_partial)
                .map(|segment| {
                    NestedLayoutCommand::new(
                        CommandAllocationSite {
                            source: Arc::clone(&source.origin),
                            offset: segment.span.start(),
                        },
                        super::super::source_command_tokens_boxed(
                            &source.text,
                            source.base(),
                            config,
                            segment,
                        ),
                    )
                })
                .collect::<Vec<_>>()
                .into(),
        )
    }
}

/// Local parser purpose for an authentic body value, including materialised
/// scripts at offset zero. Parent token/metadata/cache configurations remain
/// complete and unchanged; the lexical view supplies no selected invocation.
fn nested_layout_segments(
    source: &ExecutedScriptSource,
    parent_config: tcl_lexer::LexerConfig,
) -> Option<Vec<tcl_source::segment::SegmentedCommand>> {
    let local_config = tcl_lexer::SourceSegmentationPurpose::NestedScript
        .config(parent_config)
        .normalized();
    let tokens = tcl_lexer::Lexer::with_source_image(&source.text, local_config)
        .tokenise_all()
        .ok()?;
    let commands = tcl_lexer::group_commands_bytes(&tokens, source.text.bytes(), local_config);
    let cut =
        tcl_lexer::first_parse_cut_image_in_checked(&commands, &tokens, &source.text, local_config)
            .ok()?;
    let prefix = if let Some(cut) = cut {
        // The ordinal belongs to original lexical groups. Select their complete
        // byte prefix before F5 lookahead can merge a malformed following clause.
        let command_start = usize::try_from(commands.get(cut.command)?.span.start()).ok()?;
        tcl_lexer::SourceImage::from_bytes(
            source.text.bytes().get(..command_start)?,
            source.text.channel(),
        )
    } else {
        source.text.clone()
    };
    source
        .base()
        .checked_add(u32::try_from(prefix.bytes().len()).ok()?)?;
    tcl_source::segment::segment_commands_image_with_offset_and_config(
        &prefix,
        source.base(),
        local_config,
    )
}

impl OriginalDeclarationLayout {
    pub(super) fn select(
        site: &CommandAllocationSite,
        tokens: &crate::ir::CommandTokens,
        snapshot: &Arc<SourceLookupSnapshot>,
        namespace: &tcl_names::namespace::SourceNamespaceKey,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Self> {
        let advice = super::super::original_site_operand_layout_advice(
            site, tokens, snapshot, namespace, config,
        )?;
        let flow =
            crate::registry_invocation::declaration_invocation_flow(registry, tokens, &advice)?;
        Some(Self { advice, flow })
    }

    pub(super) fn retains_successor(&self) -> bool {
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

pub(super) struct ParentDeclarationLayout<'a> {
    parent: &'a DeclarationLayoutObservation,
    pub(super) original: Option<OriginalDeclarationLayout>,
}

pub(super) fn parent_declaration_layouts<'a>(
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
    pub(super) fn new() -> Self {
        Self {
            started: tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
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
    /// Preserve only original lexical layouts when the declaration's runtime
    /// installation is unknown. No dispatch, body entry or effect inventory is
    /// populated by this separate collector.
    pub(in super::super) fn retain_uninstalled_declaration_body_layouts(
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
                if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
                    eprintln!(
                        "ORIGINAL_DECLARATION offset={} stage=tokens-unavailable",
                        site.offset
                    );
                }
                continue;
            };
            let Some(body) = declared_body(&site, &tokens, &observation, registry) else {
                #[cfg(debug_assertions)]
                if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
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

    pub(super) fn retain_nested_declared_layouts(
        &mut self,
        registry: &tcl_registry::CommandRegistry,
    ) -> usize {
        let mut visited = NestedDeclarationLayoutVisits::default();
        self.retain_nested_declared_layouts_with(registry, |source, parents| {
            visited.insert(NestedDeclarationLayoutKey::new(source, parents))
        })
    }

    pub(super) fn retain_nested_declared_layouts_with(
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
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some() {
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

    pub(super) fn retain_nested_declared_layouts_using(
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
        let share = ScopeAliasOwnershipShare::for_roots(&roots);
        let mut pending = Vec::new();
        for (site, observations) in roots {
            let Some(tokens) = declaration_tokens(&site, &observations[0]) else {
                continue;
            };
            self.retain_original_argument_layouts(
                &site,
                &tokens,
                &observations,
                registry,
                &mut pending,
                &share,
            );
            pending.extend(unanimous_nested_layouts(
                &site,
                &tokens,
                &observations,
                registry,
                &share,
            ));
        }
        #[cfg(any(test, debug_assertions))]
        trace.phase(
            "nested-seeds",
            0,
            pending.len(),
            self.declaration_layouts.len(),
        );
        // Source binding records nothing past the shared nest cap, so a body
        // below it has no consumer for its layouts.
        let mut depths = vec![1_u32; pending.len()];
        let mut visits: usize = 0;
        while let Some((source, parents)) = pending.pop() {
            let depth = depths.pop().unwrap_or(1);
            if tcl_source::limits::MAX_SOURCE_NEST_DEPTH.exceeded(depth)
                || !visit(&source, &parents)
            {
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
                let retains_successor = self.retain_nested_layout_command(
                    command,
                    &parents,
                    registry,
                    &mut pending,
                    &share,
                );
                depths.resize(pending.len(), depth + 1);
                if !retains_successor {
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
        share: &ScopeAliasOwnershipShare,
    ) -> bool {
        let tokens = command.tokens.as_ref();
        let site = &command.site;
        // A body the walker entered already holds the observation of its own
        // execution point; a parent's earlier snapshot never replaces it.
        if self.declaration_layouts.get(site).is_some_and(|rows| {
            original_declaration_layouts(rows).is_some_and(|mut rows| {
                rows.any(|row| parents.iter().all(|parent| parent.entry != row.entry))
            })
        }) {
            return true;
        }
        let observations: Vec<_> = parents
            .iter()
            .map(|parent| DeclarationLayoutObservation {
                words: Arc::clone(&command.words),
                ..parent.clone()
            })
            .collect();
        let retained = self.declaration_layouts.entry(site.clone()).or_default();
        for observation in &observations {
            if !retained.contains(observation) {
                retained.push(observation.clone());
            }
        }
        self.retain_original_argument_layouts(
            site,
            tokens,
            &observations,
            registry,
            pending,
            share,
        );
        let originals = parent_declaration_layouts(site, tokens, &observations, registry);
        pending.extend(unanimous_nested_layouts_from_parents(
            site, &originals, registry, share,
        ));
        originals.iter().all(|row| {
            row.original
                .as_ref()
                .is_some_and(OriginalDeclarationLayout::retains_successor)
        })
    }

    fn original_argument_layout_parents_are_current(
        &self,
        site: &CommandAllocationSite,
        tokens: &crate::ir::CommandTokens,
        parents: &[DeclarationLayoutObservation],
        registry: &tcl_registry::CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        !(input
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != registry.snapshot().semantic_key()
            || self.lexer_config != Some(config)
            || self.root_origin.as_ref() != Some(&site.source)
            || parents.is_empty()
            || parents.iter().any(|parent| {
                parent.issuer != DeclarationLayoutIssuer::OriginalDeclaration
                    || parent.config != config
                    || parent.words.as_ref() != tokens.words()
                    || !parent.entry.owns_source(&site.source, site.offset)
                    || !parent
                        .entry
                        .owns_original_context(&parent.snapshot.state.source_variables)
                    || parent.snapshot.state.variable_frame != *parent.entry.frame()
                    || parent.snapshot.state.logical_source_name_advice_input() != Some(input)
                    || parent
                        .snapshot
                        .state
                        .baseline
                        .metadata_context
                        .source_analysis_input()
                        != Some(input)
                    || parent.snapshot.state.baseline.registry_snapshot
                        != Some(registry.snapshot().semantic_key())
                    || parent.snapshot.state.baseline.native_entry.is_some()
                    || parent
                        .snapshot
                        .state
                        .baseline
                        .execution_name_policy
                        .is_some()
                    || parent
                        .snapshot
                        .state
                        .baseline
                        .hosted_execution_context
                        .is_some()
            })
            || site
                .source
                .original_lexical_command(tokens.words(), site.offset, config)
                .is_none())
    }

    /// Original argv children in the same unentered Logical declaration world.
    /// Literal prefixes do not change lookup; an unknown getter, substitution
    /// result or callback ends this projection before any later child. Native
    /// entries retain their independent before-argv and actual dispatch owners.
    pub(super) fn retain_original_argument_layouts(
        &mut self,
        site: &CommandAllocationSite,
        tokens: &crate::ir::CommandTokens,
        parents: &[DeclarationLayoutObservation],
        registry: &tcl_registry::CommandRegistry,
        pending: &mut Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)>,
        share: &ScopeAliasOwnershipShare,
    ) {
        use tcl_source::words::Provenance;
        use tcl_source::words::WordExpr;
        let Some(input) = parents
            .first()
            .and_then(|parent| parent.snapshot.state.logical_source_name_advice_input())
            .cloned()
        else {
            return;
        };
        let config = input.lexer_config();
        if !self.original_argument_layout_parents_are_current(
            site, tokens, parents, registry, &input, config,
        ) {
            return;
        }
        let Some(calls) = crate::word_subst::checked_argument_lifted_calls(tokens, config) else {
            return;
        };
        for word in tokens.words() {
            let WordExpr::CommandSubstitution { source, .. } = word else {
                if matches!(
                    word,
                    WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
                ) {
                    continue;
                }
                return;
            };
            if source.provenance != Provenance::Source {
                return;
            }
            // The complete canonical collector supplies the child vector. A
            // nested or ambiguous command prefix needs its own original world.
            let children = calls
                .iter()
                .filter(|call| {
                    source.span.start() <= call.span.start()
                        && call.span.start() <= source.span.end()
                })
                .collect::<Vec<_>>();
            let [child] = children.as_slice() else {
                return;
            };
            let Some(child) = child.tokens.as_ref() else {
                return;
            };
            let Some(head) = child.words().first() else {
                return;
            };
            let offset = head.source().span.start();
            if child.synthetic.is_some()
                || head.source().provenance != Provenance::Source
                || parents.iter().any(|parent| {
                    child.words().iter().any(|word| {
                        !parent
                            .entry
                            .owns_source(&site.source, word.source().span.start())
                            || !parent
                                .entry
                                .owns_source(&site.source, word.source().span.end())
                    })
                })
                || site
                    .source
                    .original_lexical_command(child.words(), offset, config)
                    .is_none()
            {
                return;
            }
            let command = NestedLayoutCommand::new(
                CommandAllocationSite {
                    source: Arc::clone(&site.source),
                    offset,
                },
                Box::new(child.clone()),
            );
            if !self.retain_nested_layout_command(&command, parents, registry, pending, share) {
                return;
            }
        }
    }

    fn retain_declared_body_layouts(
        &mut self,
        body: DeclaredProcedureBody,
        declaration: &DeclarationLayoutObservation,
        registry: &tcl_registry::CommandRegistry,
    ) {
        self.retain_declared_body_layouts_from_snapshot(
            body,
            &declaration.snapshot,
            declaration.config,
            registry,
        );
    }

    pub(super) fn retain_declared_body_layouts_from_snapshot(
        &mut self,
        body: DeclaredProcedureBody,
        snapshot: &SourceLookupSnapshot,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let Some(segments) = nested_layout_segments(&body.source, config) else {
            return;
        };
        let mut state = snapshot.state.clone();
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
        let snapshot = Arc::new(SourceLookupSnapshot::in_realm(state, snapshot.realm));
        let entry = Arc::new(OriginalDiagnosticFrameEntry::DeclaredProcedure(Arc::new(
            body,
        )));
        for segment in segments {
            if segment.is_partial {
                break;
            }
            let tokens = super::super::source_command_tokens_boxed(
                &entry.source().text,
                entry.source().base(),
                config,
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
                config,
                words: Arc::from(tokens.words()),
            };
            if !observations.contains(&observation) {
                observations.push(observation);
            }
            if !retains_successor_layout(&site, &tokens, &snapshot, &namespace, config, registry) {
                break;
            }
        }
    }
}

pub(super) fn unanimous_nested_layouts(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    parents: &[DeclarationLayoutObservation],
    registry: &tcl_registry::CommandRegistry,
    share: &ScopeAliasOwnershipShare,
) -> Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)> {
    let originals = parent_declaration_layouts(site, tokens, parents, registry);
    unanimous_nested_layouts_from_parents(site, &originals, registry, share)
}

pub(super) fn unanimous_nested_layouts_from_parents(
    site: &CommandAllocationSite,
    originals: &[ParentDeclarationLayout<'_>],
    registry: &tcl_registry::CommandRegistry,
    share: &ScopeAliasOwnershipShare,
) -> Vec<(ExecutedScriptSource, Vec<DeclarationLayoutObservation>)> {
    let mut agreed: Option<Vec<ExecutedScriptSource>> = None;
    for row in originals {
        let sources = row.original.as_ref().map_or_else(Vec::new, |original| {
            nested_conditional_layouts(site, row.parent, original, registry, share)
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

/// The scope-alias names a parent body may write, shared by its sites while
/// no command-table, namespace or alias transition separates them. A root
/// body's span starts at its first site in source order; a nested body's
/// observations already share their parent's entry snapshot.
#[derive(Default)]
pub(super) struct ScopeAliasOwnershipShare {
    span_starts: HashMap<usize, Arc<SourceLookupSnapshot>>,
    results: std::cell::RefCell<
        HashMap<ScopeAliasOwnershipKey, Option<crate::ir_helpers::VariableWriteEffects>>,
    >,
}

#[derive(PartialEq, Eq, Hash)]
struct ScopeAliasOwnershipKey {
    snapshot: usize,
    source: ExecutedScriptSource,
    namespace: tcl_names::namespace::SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
}

type RootBodyKey<'a> = (
    &'a ExecutedScriptSource,
    &'a tcl_names::namespace::SourceNamespaceKey,
    tcl_lexer::LexerConfig,
);

impl ScopeAliasOwnershipShare {
    pub(super) fn for_roots(
        roots: &[(CommandAllocationSite, Vec<DeclarationLayoutObservation>)],
    ) -> Self {
        let mut bodies: HashMap<RootBodyKey<'_>, Vec<(u32, &Arc<SourceLookupSnapshot>)>> =
            HashMap::new();
        for (site, observations) in roots {
            for observation in observations {
                bodies
                    .entry((
                        observation.entry.source(),
                        &observation.namespace,
                        observation.config,
                    ))
                    .or_default()
                    .push((site.offset, &observation.snapshot));
            }
        }
        let mut span_starts = HashMap::new();
        for mut sites in bodies.into_values() {
            sites.sort_by_key(|(offset, _)| *offset);
            let mut start: Option<&Arc<SourceLookupSnapshot>> = None;
            for (_, snapshot) in sites {
                if !start.is_some_and(|start| {
                    Arc::ptr_eq(start, snapshot)
                        || start.state.same_command_world_tables(&snapshot.state)
                }) {
                    start = Some(snapshot);
                }
                span_starts
                    .entry(Arc::as_ptr(snapshot) as usize)
                    .or_insert_with(|| Arc::clone(start.expect("span start")));
            }
        }
        Self {
            span_starts,
            results: std::cell::RefCell::default(),
        }
    }

    fn scope_alias_ownership(
        &self,
        parent: &DeclarationLayoutObservation,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::ir_helpers::VariableWriteEffects> {
        let start = self
            .span_starts
            .get(&(Arc::as_ptr(&parent.snapshot) as usize))
            .unwrap_or(&parent.snapshot);
        let key = ScopeAliasOwnershipKey {
            snapshot: Arc::as_ptr(start) as usize,
            source: parent.entry.source().clone(),
            namespace: parent.namespace.clone(),
            config: parent.config,
        };
        if let Some(known) = self.results.borrow().get(&key) {
            return known.clone();
        }
        let observation = DeclarationLayoutObservation {
            snapshot: Arc::clone(start),
            ..parent.clone()
        };
        let effects = scope_alias_body_ownership(&observation, registry);
        self.results.borrow_mut().insert(key, effects.clone());
        effects
    }
}

fn scope_alias_body_ownership(
    parent: &DeclarationLayoutObservation,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::ir_helpers::VariableWriteEffects> {
    parent.source_body_name_ownership(registry, crate::script_binds::Ownership::ScopeAliases)
}

fn nested_conditional_layouts(
    site: &CommandAllocationSite,
    parent: &DeclarationLayoutObservation,
    original: &OriginalDeclarationLayout,
    registry: &tcl_registry::CommandRegistry,
    share: &ScopeAliasOwnershipShare,
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
                share,
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
    advice: &super::super::OriginalCompilationLookupAdvice,
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
    let tcl_names::origin::ExecutedScriptMapping::Contiguous { base } = source.mapping else {
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
    advice: &super::super::OriginalCompilationLookupAdvice,
    registry: &tcl_registry::CommandRegistry,
    share: &ScopeAliasOwnershipShare,
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
                    || super::super::declaration_layout::local_read_scope_is_excluded(context, name)
                    || share
                        .scope_alias_ownership(parent, registry)
                        .is_none_or(|effects| {
                            effects.opaque
                                || effects.names.iter().any(|candidate| candidate == name)
                        })
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

pub(in super::super) fn retains_successor_layout(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &tcl_names::namespace::SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    OriginalDeclarationLayout::select(site, tokens, snapshot, namespace, config, registry)
        .as_ref()
        .is_some_and(OriginalDeclarationLayout::retains_successor)
}
