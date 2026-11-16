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

//! Filling one document's record tables from its analysis, and clearing them.

use super::records::{
    WorkspaceCommandDeletion, WorkspaceCommandLink, WorkspaceCommandRetirement,
    WorkspaceDefinedSymbol, WorkspaceDiagnosticSourceContext, WorkspaceGlobImport,
    WorkspaceImportGate, WorkspaceMethod, WorkspaceNamespaceExport, WorkspaceNamespaceForget,
    WorkspaceNamespaceRef, WorkspacePackageIfneeded, WorkspacePackagePrefer,
    WorkspacePackageProvide, WorkspacePackageRequire, WorkspaceProc, WorkspaceSource,
    WorkspaceVariable, WorkspaceVariableRef,
};
use super::{
    DocumentRecords, WorkspaceClass, WorkspaceIndex, WorkspaceInvocation,
    WorkspaceLogicalCommandCall, WorkspaceVariableAlias,
};
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::ir::MethodKind;
use tcl_lexer::Span;
use tcl_registry::definer::DeclaredMemberVisibility;

/// Collect `items` into source order by the span `key` reports, so a
/// `HashMap`-valued analyser table lands in the index deterministically
/// instead of in the process's random hash order.
pub(super) fn sorted_by_span<'a, T, I, F>(items: I, key: F) -> Vec<&'a T>
where
    I: IntoIterator<Item = &'a T>,
    F: Fn(&T) -> Span,
{
    let mut out: Vec<&T> = items.into_iter().collect();
    out.sort_by_key(|item| {
        let span = key(item);
        (span.start(), span.end())
    });
    out
}

/// The names of a `HashSet`-valued analyser table, in a stable order.  See
/// [`sorted_by_span`] for why the source order matters.
fn sorted_names(names: &std::collections::HashSet<String>) -> Vec<String> {
    let mut out: Vec<String> = names.iter().cloned().collect();
    out.sort();
    out
}

pub(super) fn resolved_user_definition(
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
) -> Option<String> {
    if inv.resolved_user_definition {
        inv.resolved_qualified_name.clone()
    } else {
        None
    }
}

impl DocumentRecords {
    /// Drop every record, keeping each table's allocation.
    ///
    /// The capacity is deliberately retained: a re-index of the same document
    /// (the remove-then-add every publish performs) refills tables of very
    /// nearly the same size, so the slot's buffers are reused instead of being
    /// freed and regrown fourteen times per publish.  A slot whose document is
    /// gone for good keeps its capacity until another document reuses the slot
    /// — bounded by the workspace's peak document count, not by the number of
    /// removals.
    pub(super) fn clear(&mut self) {
        let Self {
            uri,
            original_source_views,
            revision: _,
            dialect,
            diagnostic_source,
            source_order_context,
            procs,
            classes,
            original_command_world,
            original_source_procedures,
            original_source_classes,
            original_source_input,
            logical_command_calls,
            logical_namespace_locations,
            logical_procedure_declarations,
            original_procs,
            original_classes,
            original_symbols,
            original_vendor_declarations,
            original_method_queries,
            original_variables,
            variables,
            variable_refs,
            variable_aliases,
            alias_source_coverage_complete,
            namespace_refs,
            invocations,
            sources,
            path_constant_assignments,
            package_requires,
            package_provides,
            package_ifneededs,
            package_prefers,
            command_links,
            glob_imports,
            original_namespace_patterns,
            namespace_exports,
            namespace_forgets,
            command_deletions,
            command_retirements,
            defined_symbols,
        } = self;
        uri.clear();
        original_source_views.clear();
        dialect.clear();
        *diagnostic_source = None;
        *source_order_context = None;
        path_constant_assignments.clear();
        procs.clear();
        classes.clear();
        *original_command_world = None;
        *original_source_procedures = None;
        *original_source_classes = None;
        *original_source_input = None;
        logical_command_calls.clear();
        logical_namespace_locations.clear();
        logical_procedure_declarations.clear();
        original_procs.clear();
        original_classes.clear();
        original_symbols.clear();
        original_vendor_declarations.clear();
        original_method_queries.clear();
        original_variables.clear();
        variables.clear();
        variable_refs.clear();
        variable_aliases.clear();
        *alias_source_coverage_complete = false;
        namespace_refs.clear();
        invocations.clear();
        sources.clear();
        package_requires.clear();
        package_provides.clear();
        package_ifneededs.clear();
        package_prefers.clear();
        command_links.clear();
        glob_imports.clear();
        original_namespace_patterns.clear();
        namespace_exports.clear();
        namespace_forgets.clear();
        command_deletions.clear();
        command_retirements.clear();
        defined_symbols.clear();
    }

    /// Lift one document's analysis into this record set.
    ///
    /// The caller ([`WorkspaceIndex::add_document`]) has already cleared the
    /// slot, so this only ever appends.
    pub(super) fn index_document(&mut self, uri: &str, analysis: &AnalysisResult) {
        let had_source_view = !self.uri.is_empty();
        let source_order_context = crate::source_graph::SourceOrderContext::capture(analysis);
        uri.clone_into(&mut self.uri);
        self.dialect.clone_from(&analysis.dialect);
        self.diagnostic_source = WorkspaceDiagnosticSourceContext::for_analysis(analysis);
        // Accumulated source-site views cannot replace an incompatible
        // original input/Realm with the latest reporting view.
        self.source_order_context =
            if !had_source_view || self.source_order_context == source_order_context {
                source_order_context
            } else {
                None
            };
        self.original_command_world = analysis.original_completed_command_world().cloned();
        self.original_source_input
            .clone_from(&analysis.resolved_input);
        self.original_source_classes = self
            .diagnostic_source
            .as_ref()
            .and_then(|context| context.image().try_text().ok())
            .and_then(|source| {
                tcl_compiler::registry_invocation::source_structure::source_class_publications(
                    source, analysis,
                )
            });
        self.original_source_procedures = self
            .diagnostic_source
            .as_ref()
            .and_then(|context| context.image().try_text().ok())
            .and_then(|source| {
                tcl_compiler::registry_invocation::source_structure::source_procedure_publications(
                    source, analysis,
                )
            });
        if let (Some(context), Some(input)) = (&self.diagnostic_source, &analysis.resolved_input)
            && input.has_logical_source_name_context()
            && let Ok(source) = context.image().try_text()
            && let Some(calls) = tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(source, analysis)
        {
            self.logical_command_calls.extend(calls.into_iter().filter_map(|candidates| {
                candidates.matches_source(context.image(), input).then(|| WorkspaceLogicalCommandCall {
                    uri: uri.to_owned(), input: input.clone(), source: context.clone(), candidates: Arc::new(candidates),
                })
            }));
        }
        self.logical_namespace_locations
            .extend(analysis.namespace_refs.iter().filter_map(|row| {
                crate::original_indexed_location::LogicalIndexedNamespaceLocation::capture(
                    uri, analysis, row,
                )
            }));
        self.original_namespace_patterns
            .extend(analysis.original_namespace_patterns().cloned());
        if let Some(context) = self.diagnostic_source.as_ref()
            && let Ok(source) = std::str::from_utf8(context.image().bytes())
        {
            self.original_vendor_declarations.extend(
                crate::vendor_declaration::retained_declarations(source, analysis)
                    .unwrap_or_default(),
            );
            self.original_symbols
                .extend(crate::workspace_symbols::original_symbols(
                    uri, source, analysis, context,
                ));
        }
        self.original_method_queries
            .extend(crate::method_symbol::source_queries(analysis));
        self.original_procs
            .extend(analysis.original_procedure_declarations().cloned());
        self.original_classes
            .extend(analysis.original_class_declarations().cloned());
        self.index_document_declarations(uri, analysis);
        self.index_document_invocations(uri, analysis);
        self.index_sources(uri, analysis);
        self.index_package_declarations(uri, analysis);
        self.index_namespace_exports(uri, analysis);
        self.index_import_lifecycle(uri, analysis);
        self.index_command_links(uri, analysis);
    }

    fn index_document_declarations(&mut self, uri: &str, analysis: &AnalysisResult) {
        // `all_procs` / `all_classes` / a class's `methods` are `HashMap`s, so
        // iterating them directly would order this document's index entries by
        // the process's random hash seed — and consumers that answer with the
        // *first* matching record (go-to-definition's dispatch entry, most
        // visibly) then answer differently run to run.  Source
        // order is the stable, meaningful order: it is also the order the
        // records take effect in when the file is sourced.
        for proc_def in sorted_by_span(analysis.all_procs.values(), |p| p.name_span) {
            if let Some(declaration) = analysis.logical_source_procedure_declaration(proc_def) {
                self.logical_procedure_declarations
                    .push(declaration.clone());
            }
            self.procs.push(WorkspaceProc {
                source_name: proc_def.source_name.clone(),
                uri: uri.to_owned(),
                name: proc_def.name.clone(),
                qualified_name: proc_def.qualified_name.clone(),
                param_count: proc_def.params.len(),
                arity: proc_def.arity(),
                name_span: proc_def.name_span,
                nested: analysis.offset_is_inside_any_definition_body(proc_def.name_span.start()),
            });
        }
        self.index_classes(uri, analysis);
        for sym in sorted_by_span(&analysis.all_defined_symbols, |s| s.name_span) {
            self.defined_symbols.push(WorkspaceDefinedSymbol {
                uri: uri.to_owned(),
                name: sym.name.clone(),
                qualified_name: sym.qualified_name.clone(),
                kind: sym.kind,
                name_span: sym.name_span,
            });
        }
        if let Some(context) = self.diagnostic_source.as_ref()
            && let Ok(source) = context.image().try_text()
        {
            self.original_variables.extend(
                analysis
                    .original_variable_symbols
                    .iter()
                    .filter(|occurrence| {
                        occurrence.symbol().is_namespace()
                            && crate::original_name_edit::original_input_matches_source(
                                source,
                                analysis,
                                occurrence.original_name_input(),
                                occurrence.span(),
                            )
                    })
                    .cloned(),
            );
        }
        self.index_variables(uri, analysis);
        self.index_namespace_refs(uri, analysis);
    }

    fn index_document_invocations(&mut self, uri: &str, analysis: &AnalysisResult) {
        let bodies = WorkspaceIndex::enclosing_body_spans(
            analysis,
            &analysis
                .command_invocations
                .iter()
                .map(|inv| inv.range.start())
                .collect::<Vec<_>>(),
        );
        for (inv, enclosing_body) in analysis.command_invocations.iter().zip(bodies) {
            self.invocations.push(WorkspaceInvocation {
                logical_source_domain: analysis.resolved_input.as_ref().map(
                    tcl_compiler::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
                ),
                original_name_input: inv.original_name_input.clone(),
                original_lookup: inv.original_lookup.clone(),
                lookup: inv.lookup,
                uri: uri.to_owned(),
                name: inv.name.clone(),
                resolution_candidates: inv.resolution_candidates.clone(),
                resolved_user_definition: resolved_user_definition(inv),
                resolved_definition: inv.resolved_definition.clone(),
                resolved_command_reference: inv.resolved_command_reference.clone(),
                range: inv.range,
                indirect: inv.indirect,
                rename_safe: inv.rename_safe,
                ensemble_dispatch: inv.ensemble_dispatch,
                enclosing_body,
            });
        }
    }

    fn index_package_declarations(&mut self, uri: &str, analysis: &AnalysisResult) {
        for pr in &analysis.package_requires {
            self.package_requires.push(WorkspacePackageRequire {
                original_name: pr.original_name.as_ref().map(|name| name.key().clone()),
                requirements: pr.requirements.clone(),
                exact: pr.exact,
                prefer: crate::package_resolver::package_prefer_at(
                    analysis,
                    pr.range.start(),
                    crate::package_resolver::PackagePrefer::default(),
                ),
                uri: uri.to_owned(),
                name: pr.name.clone(),
                at: pr.range.start(),
                enclosing_body: analysis.innermost_definition_body_span(pr.range.start()),
                conditional: pr.conditional,
            });
        }
        for pp in &analysis.package_provides {
            self.package_provides.push(WorkspacePackageProvide {
                original_name: pp.original_name.as_ref().map(|name| name.key().clone()),
                uri: uri.to_owned(),
                name: pp.name.clone(),
                at: pp.range.start(),
                conditional: pp.conditional,
            });
        }
        for pi in &analysis.package_ifneededs {
            self.package_ifneededs.push(WorkspacePackageIfneeded {
                original_name: pi.original_name.as_ref().map(|name| name.key().clone()),
                uri: uri.to_owned(),
                name: pi.name.clone(),
            });
        }
        // Only the unconditional raises: a `package prefer latest` inside an
        // `if` / `catch` / `try` may not run, and the cross-document tier
        // abstains toward the interpreter default exactly as the
        // single-document one does.
        for prefer in analysis
            .package_prefer_latest
            .iter()
            .filter(|p| !p.conditional)
        {
            self.package_prefers.push(WorkspacePackagePrefer {
                uri: uri.to_owned(),
                at: prefer.range.start(),
                enclosing_body: analysis.innermost_definition_body_span(prefer.range.start()),
            });
        }
    }

    fn index_namespace_exports(&mut self, uri: &str, analysis: &AnalysisResult) {
        for exp in analysis.original_namespace_exports() {
            self.namespace_exports.push(WorkspaceNamespaceExport {
                original: Some(exp.clone()),
                unknown: false,
                uri: uri.to_owned(),
                ns: exp.context().reporting_label(),
                pattern: exp
                    .pattern()
                    .map_or_else(String::new, tcl_syntax::native_string::resident_name_label),
                at: exp.span().start(),
                enclosing_body: analysis.original_definition_body_span(exp.span().start()),
                clears: exp.clears(),
            });
        }
        for span in analysis.original_namespace_export_unknowns() {
            self.namespace_exports.push(WorkspaceNamespaceExport {
                original: None,
                unknown: true,
                uri: uri.to_owned(),
                ns: String::new(),
                pattern: String::new(),
                at: span.start(),
                enclosing_body: analysis.original_definition_body_span(span.start()),
                clears: false,
            });
        }
        if analysis.allows_lexical_declaration_advice() {
            for exp in &analysis.namespace_exports {
                self.namespace_exports.push(WorkspaceNamespaceExport {
                    original: None,
                    unknown: false,
                    uri: uri.to_owned(),
                    ns: exp.ns.clone(),
                    pattern: exp.pattern.clone(),
                    at: exp.range.start(),
                    enclosing_body: analysis.innermost_definition_body_span(exp.range.start()),
                    clears: exp.clears,
                });
            }
        }
    }

    /// The document's `source` rows, plus the raw path-constant candidates
    /// the edge resolver folds them with — recorded together because they
    /// are consumed together (a computed row without its constants is
    /// unresolvable, constants without rows are inert).
    pub(super) fn index_sources(&mut self, uri: &str, analysis: &AnalysisResult) {
        for target in &analysis.source_targets {
            if let Some(original) = &target.original_project_source_load
                && !analysis.resolved_input.as_ref().is_some_and(|input| {
                    original.path_word().is_some_and(|word| {
                        analysis.matches_original_source_image(word.image(), input.lexer_config())
                            && original.matches_source(word.image(), input)
                    })
                })
            {
                continue;
            }
            if let Some(original) = &target.original_interpreter_source_load
                && !original.matches_analysis(analysis)
            {
                continue;
            }
            self.sources.push(WorkspaceSource {
                original_project_source_load: target.original_project_source_load.clone(),
                original_source_frontier: target
                    .original_project_source_load
                    .as_ref()
                    .map(|load| std::sync::Arc::new(load.frontier(analysis))),
                original_path_expression:
                    tcl_compiler::auto_path_eval::capture_source_target_path_expression(
                        analysis, target,
                    ),
                original_interpreter_source_load: target.original_interpreter_source_load.clone(),
                uri: uri.to_owned(),
                raw_path: target.raw_path.clone(),
                site_namespace: target.site_namespace.clone(),
                range: target.range,
                is_literal: target.is_literal,
                enclosing_body: analysis.innermost_definition_body_span(target.range.start()),
            });
        }
        self.path_constant_assignments
            .clone_from(&analysis.path_constant_assignments);
    }

    /// Lift one document's class definitions, each with its members flattened
    /// into one source-ordered [`WorkspaceMethod`] list (instance methods
    /// first, then the class-side ones).
    ///
    /// Split out of [`Self::index_document`] only for size; the source-order
    /// rule that walk applies here unchanged.
    fn index_classes(&mut self, uri: &str, analysis: &AnalysisResult) {
        let mut classes: Vec<_> = analysis
            .all_classes
            .values()
            .map(|class_def| {
                let mut originals = analysis.original_class_declarations().filter(|original| {
                    !class_def.source_name_ambiguous.is_observed()
                        && class_def.source_name.as_ref() == Some(original.name())
                        && class_def.name_span == original.metadata().name_span
                });
                let original = originals.next().filter(|_| originals.next().is_none());
                (class_def, original)
            })
            .collect();
        for original in analysis.original_class_declarations() {
            if !classes.iter().any(|(_, candidate)| {
                candidate.is_some_and(|candidate| {
                    candidate.original_occurrence() == original.original_occurrence()
                })
            }) {
                classes.push((original.metadata(), Some(original)));
            }
        }
        classes
            .sort_by_key(|(class_def, _)| (class_def.name_span.start(), class_def.name_span.end()));
        for (class_def, original) in classes {
            let methods: Vec<WorkspaceMethod> =
                sorted_by_span(class_def.methods.values(), |m| m.name_span)
                    .into_iter()
                    .map(|m| WorkspaceMethod {
                        name: m.name.clone(),
                        kind: m.kind.clone(),
                        exported: m.visibility == DeclaredMemberVisibility::Public.as_str(),
                        private: m.visibility == DeclaredMemberVisibility::Private.as_str(),
                        is_self_method: m.is_self_method,
                        name_span: m.name_span,
                    })
                    .chain(
                        sorted_by_span(class_def.class_methods.values(), |m| m.name_span)
                            .into_iter()
                            .map(|m| WorkspaceMethod {
                                name: m.name.clone(),
                                kind: MethodKind::ClassMethod.as_str().to_string(),
                                exported: m.visibility == DeclaredMemberVisibility::Public.as_str(),
                                private: m.visibility == DeclaredMemberVisibility::Private.as_str(),
                                is_self_method: m.is_self_method,
                                name_span: m.name_span,
                            }),
                    )
                    .collect();
            self.classes.push(WorkspaceClass {
                logical_declaration: analysis
                    .logical_source_class_declaration(class_def)
                    .cloned(),
                original_declaration: original
                    .map(|original| original.original_occurrence().clone()),
                original_members: original
                    .map(|original| original.metadata().original_members.clone())
                    .or_else(|| {
                        analysis
                            .allows_retained_logical_declaration_advice()
                            .then(|| class_def.original_members.clone())
                    }),
                original_relations: original
                    .map(|original| original.metadata().original_relations.clone()),
                source_name: class_def.source_name.clone(),
                source_name_ambiguous: class_def.source_name_ambiguous.is_observed(),
                relation_lookups: class_def.relation_lookups.clone(),
                uri: uri.to_owned(),
                name: class_def.name.clone(),
                qualified_name: class_def.qualified_name.clone(),
                name_span: class_def.name_span,
                superclasses: class_def.superclasses.clone(),
                mixins: class_def.mixins.clone(),
                methods,
                exports: sorted_names(&class_def.exports),
                unexports: sorted_names(&class_def.unexports),
                class_exports: sorted_names(&class_def.class_exports),
                class_unexports: sorted_names(&class_def.class_unexports),
                retracted_members: class_def.retracted_members.clone(),
                via_define: class_def.via_define,
                metaclass: class_def.metaclass.clone(),
                definer_family: crate::definition::class_definer_family(analysis, class_def),
                bare_word_construction: class_def
                    .class_command_fallback
                    .constructs_named_instance(),
                constructor_spans: class_def.constructors.iter().map(|c| c.name_span).collect(),
            });
        }
    }

    /// Lift a document's `namespace forget` events and command **destructions**
    /// — the removal half of the import edge's lifecycle.
    ///
    /// A destroying `rename OLD {}` / `interp alias {} NAME {}` kills every
    /// import alias pointing at `OLD`, because the alias holds the command
    /// object; a plain `rename OLD NEW` does not, which is why
    /// `AnalysisResult::destroyed_commands` (not `deleted_commands`) is the
    /// source here.
    fn index_import_lifecycle(&mut self, uri: &str, analysis: &AnalysisResult) {
        for fgt in &analysis.namespace_forgets {
            self.namespace_forgets.push(WorkspaceNamespaceForget {
                uri: uri.to_owned(),
                ns: fgt.ns.clone(),
                source_ns: fgt.source_ns.clone(),
                pattern: fgt.pattern.clone(),
                at: fgt.range.start(),
            });
        }
        // `HashMap`-valued, so sort into source order for the same reason
        // `sorted_by_span` exists: a consumer answering with the
        // first matching record must not answer differently run to run.
        let mut destroyed: Vec<(&String, &u32)> = analysis.destroyed_commands.iter().collect();
        destroyed.sort_unstable_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)));
        for (name, at) in destroyed {
            let qualified_name = name.clone();
            self.command_deletions.push(WorkspaceCommandDeletion {
                uri: uri.to_owned(),
                qualified_name: qualified_name.clone(),
                at: *at,
            });
            self.command_retirements.push(WorkspaceCommandRetirement {
                uri: uri.to_owned(),
                qualified_name,
                at: *at,
            });
        }
    }

    /// Index current original source symbols and conditional alias declarations.
    /// Compatibility labels are checked outward C projections only; they never
    /// reconstruct a source symbol or claim actual cells or alias completion.
    fn index_variables(&mut self, uri: &str, analysis: &AnalysisResult) {
        let Some(context) = self.diagnostic_source.as_ref() else {
            return;
        };
        let Ok(source) = context.image().try_text() else {
            return;
        };
        self.alias_source_coverage_complete =
            analysis.original_variable_alias_source_is_complete(context.image(), context.config());
        for occurrence in &analysis.original_variable_symbols {
            if !crate::original_name_edit::original_input_matches_source(
                source,
                analysis,
                occurrence.original_name_input(),
                occurrence.span(),
            ) {
                continue;
            }
            let Some(qualified_name) = occurrence.symbol().checked_c_namespace_key() else {
                continue;
            };
            if occurrence.is_declaration() {
                let Some(name) = occurrence
                    .symbol()
                    .rename_tail()
                    .and_then(|name| std::str::from_utf8(name).ok())
                else {
                    continue;
                };
                self.variables.push(WorkspaceVariable {
                    uri: uri.to_owned(),
                    name: name.to_owned(),
                    qualified_name,
                    name_span: occurrence.span(),
                });
            } else {
                self.variable_refs.push(WorkspaceVariableRef {
                    uri: uri.to_owned(),
                    qualified_name,
                    span: occurrence.span(),
                });
            }
        }
        if let Some(templates) =
            analysis.original_variable_alias_templates_in_source(context.image(), context.config())
        {
            self.variable_aliases.extend(
                templates
                    .into_iter()
                    .filter(tcl_compiler::signature_scan::variable_symbol::OriginalVariableAliasTemplate::is_declaration)
                    .map(|source| WorkspaceVariableAlias {
                        uri: uri.to_owned(),
                        source,
                    }),
            );
        }
    }

    /// Lift a document's namespace-name occurrences
    /// ([`tcl_compiler::analyser::AnalysisResult::namespace_refs`]) into the
    /// workspace table, so `namespace children ::tomato` in one file reaches
    /// the `namespace eval ::tomato { … }` in another.
    ///
    /// A straight copy: the analyser has already rooted relative spellings
    /// and dropped computed ones, so there is no second resolution rule here
    /// that could disagree with the in-document providers.
    fn index_namespace_refs(&mut self, uri: &str, analysis: &AnalysisResult) {
        for nref in &analysis.namespace_refs {
            self.namespace_refs.push(WorkspaceNamespaceRef {
                source: nref.clone(),
                uri: uri.to_owned(),
                qualified_name: nref.qualified_name.clone(),
                span: nref.span,
                declares: nref.declares,
            });
        }
    }

    /// Lift a document's `namespace import` / `interp alias` / `rename`
    /// records into flat [`WorkspaceCommandLink`] entries the cross-document
    /// reference walk can follow.  Each becomes `linked_qname → target_qname`:
    /// the new callable name and the command it ultimately runs.
    pub(super) fn index_command_links(&mut self, uri: &str, analysis: &AnalysisResult) {
        // `namespace import ::mod::helper` inside `::app` binds `::app::helper`
        // to the exporting `::mod::helper`.  A glob pattern names no single
        // command, so it introduces no fixed `WorkspaceCommandLink` — instead
        // it is indexed as a [`WorkspaceGlobImport`], consulted per-call by
        // [`Self::resolve_wildcard_import`] against whichever bare name the
        // invocation actually writes.
        for imp in &analysis.namespace_imports {
            let Some(source) = &imp.source else {
                continue;
            };
            if !tcl_syntax::glob::is_literal(&source.tail_pattern) {
                self.glob_imports.push(WorkspaceGlobImport {
                    original_pattern: analysis
                        .original_namespace_patterns()
                        .find(|pattern| {
                            pattern.span() == imp.range
                                && pattern.purpose()
                                    == tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern
                        })
                        .cloned(),
                    native_source: source.native_source.clone(),
                    uri: uri.to_owned(),
                    ns: imp.ns.clone(),
                    source_ns: source.namespace.clone(),
                    tail_pattern: source.tail_pattern.clone(),
                    at: imp.range.start(),
                    enclosing_body: analysis.innermost_definition_body_span(imp.range.start()),
                    forced: imp.forced,
                });
                continue;
            }
            let source_ns = &source.namespace;
            let tail = &source.tail_pattern;
            // An exact import is export-gated exactly like a glob one — real
            // Tcl silently installs nothing when the name is not exported at
            // the import's own position (oracle on
            // `WorkspaceCommandLink::import_gate`). One shape stays ungated:
            // a *conjectured* import, which is inferred from a tcllib
            // `<NS>::import <ALIAS>` wrapper rather than read off a real
            // `namespace import` word, so there is no export declaration for
            // it to have passed and gating it would drop the idiom entirely.
            //
            // A pattern rooted at the global namespace (`namespace import
            // ::p`) is **not** one of them, though it reads as an empty
            // source namespace, which makes it easy to skip on that basis.
            // Real Tcl treats it like any other unexported import: a silent
            // no-op leaving `info commands ::dst::*` empty (oracle 8.6.14 /
            // 9.0.4). `::` is its source namespace, the same spelling every
            // global-level `namespace export` record already carries.
            let import_gate =
                (!imp.conjectured)
                    .then_some(source_ns)
                    .map(|source_ns| WorkspaceImportGate {
                        original_pattern: analysis
                            .original_namespace_patterns()
                            .find(|pattern| {
                                pattern.span() == imp.range
                                && pattern.purpose()
                                    == tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern
                            })
                            .cloned(),
                        native_source: source.native_source.clone(),
                        source_ns: global_rooted(source_ns).to_owned(),
                        name: tail.clone(),
                        at: imp.range.start(),
                        enclosing_body: analysis.innermost_definition_body_span(imp.range.start()),
                        forced: imp.forced,
                    });
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: tcl_syntax::naming::qualify(&imp.ns, tail),
                linked_source_name: None,
                target_qname: source.constructed_pattern(),
                target_source_name: None,
                target_span: Some(imp.range),
                nested: analysis.offset_is_inside_any_definition_body(imp.range.start()),
                import_gate,
            });
        }
        self.index_alias_and_rename_links(uri, analysis);
    }

    fn index_alias_and_rename_links(&mut self, uri: &str, analysis: &AnalysisResult) {
        // `interp alias {} a {} ::mod::helper` binds `a` to `::mod::helper`;
        // the alias target resolves from `::` at call time, so root it there.
        // The `TARGET` word itself is already a first-class command invocation
        // (the registry marks it a command prefix), so it needs no
        // `target_span` here — the ordinary reference/rename path covers it;
        // this link only lets a call through the *alias name* resolve.
        for alias in analysis.command_aliases.values() {
            if alias.target.as_str().is_empty() {
                continue;
            }
            let nested = analysis
                .alias_offsets
                .get(&alias.qualified_name)
                .is_some_and(|&off| analysis.offset_is_inside_any_definition_body(off));
            let policy = analysis.resolved_profile().and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
            });
            let Some(target) = alias.target.reported_global_key(policy) else {
                continue;
            };
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: alias.qualified_name.clone(),
                linked_source_name: alias.source_name.clone(),
                target_qname: target.into_owned(),
                target_source_name: alias.target.selected_global_name(policy),
                target_span: None,
                nested,
                import_gate: None,
            });
        }
        // `rename OLD NEW` makes `NEW` run what `OLD` denoted.  The recorded
        // map is `NEW → OLD`, both already `::`-normalised.  `OLD`'s own
        // word is already a first-class command invocation — the ordinary
        // reference/rename path covers it — so, like
        // `interp alias`'s `TARGET` word above, it needs no `target_span`
        // here.
        for (new, old) in &analysis.renamed_commands {
            let Some(&at) = analysis.rename_offsets.get(new) else {
                continue;
            };
            let nested = analysis.offset_is_inside_any_definition_body(at);
            self.command_links.push(WorkspaceCommandLink {
                uri: uri.to_owned(),
                linked_qname: new.clone(),
                linked_source_name: None,
                target_qname: old.clone(),
                target_source_name: None,
                target_span: None,
                nested,
                import_gate: None,
            });
            // A rename written in a definition body runs only if that body is
            // called.  It introduces the same conditional link above but is
            // not evidence that the load-level name has gone away.
            if !nested {
                self.command_retirements.push(WorkspaceCommandRetirement {
                    uri: uri.to_owned(),
                    qualified_name: old.clone(),
                    at,
                });
            }
        }
    }
}

impl WorkspaceIndex {
    /// [`tcl_compiler::analyser::AnalysisResult::innermost_definition_body_span`]
    /// for a whole column of offsets at once, in the order they were given.
    ///
    /// That single-offset lookup scans every recorded proc and class body, so
    /// calling it once per invocation is `O(procs × invocations)` — far too
    /// dear to run once per indexed call site.
    /// It is not the cost the answer actually needs: definition bodies are
    /// syntactic, so they **nest properly** — no two of them partially overlap
    /// — which means one pass over the bodies in start order, keeping the
    /// currently-open chain on a stack, yields the innermost body of every
    /// offset in increasing order. Sorting both sides makes the whole column
    /// `O((P + I) log (P + I))` instead, small beside the analysis that
    /// produced them.
    ///
    /// Ties in `start` are ordered by *longer first*, so a body sharing its
    /// start with a nested one is pushed before its child and the stack top
    /// stays the innermost.
    pub(super) fn enclosing_body_spans(
        analysis: &AnalysisResult,
        offsets: &[u32],
    ) -> Vec<Option<Span>> {
        let mut bodies: Vec<Span> = analysis
            .all_procs
            .values()
            .map(|p| p.body_span)
            .chain(analysis.all_classes.values().map(|c| c.body_span))
            .collect();
        if bodies.is_empty() {
            return vec![None; offsets.len()];
        }
        bodies.sort_unstable_by_key(|s| (s.start(), std::cmp::Reverse(s.end())));
        let mut order: Vec<usize> = (0..offsets.len()).collect();
        order.sort_unstable_by_key(|&i| offsets[i]);
        let mut out = vec![None; offsets.len()];
        let mut open: Vec<Span> = Vec::new();
        let mut next = 0usize;
        for i in order {
            let off = offsets[i];
            while next < bodies.len() && bodies[next].start() <= off {
                open.push(bodies[next]);
                next += 1;
            }
            while open.last().is_some_and(|b| b.end() <= off) {
                open.pop();
            }
            out[i] = open.last().copied();
        }
        out
    }
}

/// The source namespace an import pattern names, reading the empty prefix a
/// global-rooted pattern (`::p`, `::*`) splits to as the global namespace it
/// actually is.
fn global_rooted(source_ns: &str) -> &str {
    if source_ns.is_empty() {
        "::"
    } else {
        source_ns
    }
}
