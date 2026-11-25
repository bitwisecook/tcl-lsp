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

//! The compact tier: short names for procedure-local variables, parameters and isolated procedures.

use super::symbol_map::SymbolMap;
use super::{MinifyEnv, SourceEdit, apply_edits, lexical_metadata, original};
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;
use tcl_compiler::analyser::{AnalysisResult, Scope, ScopeKind};
use tcl_lexer::{LexerConfig, Span};
use tcl_registry::{ArgRole, Traits};
use tcl_syntax::naming::AuthoredCompactNameSequence;

/// Display label only: `::` for the root, then `parent::child`.
fn child_scope_label(parent_label: &str, child_name: &str) -> String {
    tcl_syntax::naming::rooted_member_key(parent_label, child_name)
}

/// Reborrow the actual deepest source scope and render its separate label.
pub(super) fn source_scope_at_offset<'a>(
    scope: &'a Scope,
    offset: u32,
    prefix: &str,
    include_global: bool,
) -> Option<(&'a Scope, String)> {
    for child in &scope.children {
        let label = child_scope_label(prefix, &child.name);
        if let Some(body) = child.body_span
            && body.start() <= offset
            && offset <= body.end()
        {
            if let Some(deeper) = source_scope_at_offset(child, offset, &label, include_global) {
                return Some(deeper);
            }
            return Some((child, label));
        }
    }
    match scope.kind {
        ScopeKind::Proc => Some((scope, prefix.to_owned())),
        ScopeKind::Global if include_global => Some((scope, prefix.to_owned())),
        _ => None,
    }
}

/// Report collisions are rendered after edits have been selected. These keys
/// never select a source scope or decide whether an edit is authorised.
pub(super) fn record_variable_report(
    symbol_map: &mut SymbolMap,
    label: &str,
    mapping: BTreeMap<String, String>,
) {
    let mut report_label = label.to_owned();
    let mut occurrence = 2;
    while symbol_map.variables.contains_key(&report_label) {
        report_label = format!("{label} [source {occurrence}]");
        occurrence += 1;
    }
    symbol_map.variables.insert(report_label, mapping);
}

/// Where renaming is unsafe, as proven by registry-declared
/// observability facts over the script's command invocations.
#[derive(Debug, Default)]
pub(super) struct RenameBarriers<'a> {
    /// Actual borrowed source scopes containing a local rename barrier.
    /// Equality is allocation correspondence within this retained analysis.
    pub(super) scopes: Vec<&'a Scope>,
    /// Variable renaming is barred in **every** scope: a caller-frame
    /// command (`upvar` / `uplevel`) or a cross-proc parameter
    /// introspection (`info args PROC`) can observe any frame.
    all_variable_scopes: bool,
    /// Variable renaming is barred in the global scope specifically —
    /// a scope-alias command (`global` / `variable`) or a global
    /// introspection links / enumerates global cells by name.
    pub(super) global_variables: bool,
    /// Procedure renaming is barred: a command-name-reflecting
    /// command or a computed command name is present, so proc names
    /// are observable data.
    pub(super) procs: bool,
}

impl RenameBarriers<'_> {
    pub(super) fn allows_scope(&self, scope: &Scope) -> bool {
        !self.all_variable_scopes
            && !self
                .scopes
                .iter()
                .any(|barred| std::ptr::eq(*barred, scope))
            && (scope.kind != ScopeKind::Global || !self.global_variables)
    }
}

/// Observability comes from the same selected complete-word metadata.
/// Missing table/selector coverage is a barrier, rather than an empty trait.
pub(super) fn find_rename_barriers<'a>(
    analysis: &'a AnalysisResult,
    env: MinifyEnv<'_>,
    commands: &[tcl_lexer::NativeScriptCommandWords],
    include_global: bool,
) -> RenameBarriers<'a> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let mut out = RenameBarriers::default();
    for command in commands {
        let traits = lexical_metadata::with_editable_command_schema(env, command, |schema| {
            let spec = env.context.context().resolve_spec_in_realm(
                env.registry,
                schema.canonical_command,
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )?;
            if !spec.subcommands.is_empty() && schema.subcommand.resolved().is_none() {
                return None;
            }
            Some(schema.semantics.traits)
        })
        .flatten();
        let Some(traits) = traits else {
            out.all_variable_scopes = true;
            out.procs = true;
            continue;
        };
        apply_rename_traits(
            &mut out,
            traits,
            source_scope_at_offset(
                &analysis.global_scope,
                command.span.start(),
                "::",
                include_global,
            )
            .map(|(scope, _)| scope),
        );
    }
    out
}

pub(super) fn apply_rename_traits<'a>(
    out: &mut RenameBarriers<'a>,
    traits: Traits,
    scope: Option<&'a Scope>,
) {
    let reflects_vars =
        traits.intersects(Traits::INTROSPECTS_BY_NAME | Traits::TARGETS_VARIABLE_BY_NAME);
    let reflects_commands = traits.contains(Traits::REFLECTS_COMMAND_NAMES);
    if (reflects_vars || traits.contains(Traits::CREATES_DYNAMIC_BARRIER))
        && let Some(scope) = scope
        && !out.scopes.iter().any(|barred| std::ptr::eq(*barred, scope))
    {
        out.scopes.push(scope);
    }
    out.global_variables |= reflects_vars || traits.contains(Traits::CREATES_SCOPE_ALIAS);
    out.all_variable_scopes |= traits
        .intersects(Traits::ALIASES_CALLER_FRAME | Traits::EVALUATES_IN_SHIFTED_FRAME)
        || (reflects_vars && reflects_commands);
    out.procs |= reflects_commands;
}

/// Apply only the original formal-name fields already checked for this header.
/// Defaults, list wrappers and unchanged rest parameters remain outside edits.
pub(super) fn rename_params_in_list(
    formals: &[tcl_compiler::signature_scan::formal_parameters::OriginalSourceFormalName],
    var_map: &BTreeMap<String, String>,
    edits: &mut Vec<SourceEdit>,
) {
    for formal in formals {
        let (Ok(name), Some(span)) = (std::str::from_utf8(formal.name()), formal.source_span())
        else {
            continue;
        };
        if let Some(short) = var_map.get(name) {
            let range = span.as_range();
            edits.push(SourceEdit {
                span: range,
                text: short.clone(),
            });
        }
    }
}

/// Byte-span slice of `source`.
pub(super) fn slice(source: &str, span: Span) -> &str {
    let (s, e) = (span.start() as usize, span.end() as usize);
    if s <= e && e <= source.len() {
        &source[s..e]
    } else {
        ""
    }
}

/// Written execution heads selected by the shared current procedure reader.
pub(super) fn current_proc_call_sites(
    source: &str,
    procedure: &tcl_compiler::analyser::ProcDef,
    analysis: &AnalysisResult,
) -> Vec<Span> {
    if !analysis.allows_lexical_declaration_advice()
        || crate::original_context::CurrentSourceContext::capture(source, analysis).is_none()
        || !analysis
            .all_procs
            .values()
            .any(|current| std::ptr::eq(current, procedure))
    {
        return Vec::new();
    }
    let mut seen = FxHashSet::default();
    analysis
        .command_invocations
        .iter()
        .filter(|invocation| {
            !invocation.indirect
                && invocation.lookup.is_execution_site()
                && crate::references::invocation_references_proc(
                    analysis,
                    invocation,
                    &procedure.qualified_name,
                    procedure,
                    source,
                )
                && seen.insert((invocation.range.start(), invocation.range.end()))
        })
        .map(|invocation| invocation.range)
        .collect()
}

/// Constructed presentation rows retained independently from source rewriting.
#[cfg(test)]
pub(super) fn find_proc_call_sites(
    name: &str,
    qualified_name: &str,
    analysis: &AnalysisResult,
) -> Vec<Span> {
    let qn_no_prefix =
        tcl_syntax::naming::unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
    let mut out = Vec::new();
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    for inv in &analysis.command_invocations {
        // An indirect site (a constant `$cmd` head) carries no written name
        // at its span — the minifier must not rewrite it.
        if inv.indirect {
            continue;
        }
        let matches = match &inv.resolved_qualified_name {
            Some(resolved) => resolved == qualified_name,
            None => inv.name == name || inv.name == qualified_name || inv.name == qn_no_prefix,
        };
        if matches && seen.insert((inv.range.start(), inv.range.end())) {
            out.push(inv.range);
        }
    }
    out
}

/// Compact proc-local (and, when `isolated`, global) variable and
/// parameter names — plus, under `isolated` only, proc names —
/// returning `(renamed_source, symbol_map)`.
///
/// Array member keys are **never** compacted: `arr(member)` is Tcl
/// data observable through `array get` / `array names` / traces /
/// serialisation, not a private compiler symbol.
pub(super) fn compact_names(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    isolated: bool,
) -> (String, SymbolMap) {
    let analysis = original::analysis_with_input(source, input);
    let context = input.context_registry();
    let Some(identities) = analysis.retained_command_realm() else {
        return (source.to_owned(), SymbolMap::default());
    };
    let env = MinifyEnv {
        original_source: source,
        analysis: &analysis,
        input,
        dialect: input.unit_profile(),
        config: input.lexer_config(),
        registry: context.commands(),
        context: &context,
        identities,
    };
    let Some(census) = lexical_metadata::command_census(source, env) else {
        return (source.to_owned(), SymbolMap::default());
    };
    let commands = lexical_metadata::CommandCensus::observed_commands(&census);
    let mut symbol_map = SymbolMap::default();
    let mut edits: Vec<SourceEdit> = Vec::new();

    let mut barriers = find_rename_barriers(&analysis, env, commands, isolated);
    if !lexical_metadata::CommandCensus::is_complete(&census, source, &analysis) {
        barriers.all_variable_scopes = true;
        barriers.procs = true;
    }
    let rmw_targets = rmw_target_var_names(env, commands);
    let builtin_names: FxHashSet<&str> = context
        .command_names()
        .filter(|name| {
            context
                .context()
                .resolve_spec_in_realm(
                    context.commands(),
                    name,
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .is_some()
        })
        .collect();

    let references =
        crate::source_structure::OriginalVariableReferences::capture(source, &analysis);
    let mut renamed_scopes = Vec::new();
    let scope_ctx = ScopeCtx {
        references: references.as_ref(),
        config: input.lexer_config(),
        source,
        analysis: &analysis,
        isolated,
        barriers: &barriers,
        rmw_targets: &rmw_targets,
    };
    process_scope(
        scope_ctx,
        &analysis.global_scope,
        "::",
        &mut ScopeOut {
            symbol_map: &mut symbol_map,
            edits: &mut edits,
            renamed_scopes: &mut renamed_scopes,
        },
    );

    // Proc renaming.  A proc name is a *public command identity* —
    // observable via `info procs` / `info commands`, `rename`,
    // `namespace export`, `unknown`, traces, and callable by code
    // outside this script — so it is renamed only when the caller
    // asserts a closed world (`isolated`) AND no command-name
    // reflection or computed command name is present.
    if isolated && !barriers.procs {
        compact_isolated_procedure_names(
            source,
            input,
            &analysis,
            &builtin_names,
            &mut symbol_map,
            &mut edits,
        );
    }

    compact_logical_formals(
        source,
        input,
        &analysis,
        isolated,
        &renamed_scopes,
        &mut symbol_map,
        &mut edits,
    );
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    edits.dedup();
    apply_edits(source, &edits).map_or_else(
        |_| (source.to_owned(), SymbolMap::default()),
        |result| (result, symbol_map),
    )
}

fn compact_isolated_procedure_names(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    analysis: &AnalysisResult,
    builtin_names: &FxHashSet<&str>,
    symbol_map: &mut SymbolMap,
    edits: &mut Vec<SourceEdit>,
) {
    let mut proc_gen = AuthoredCompactNameSequence::new(input.lexer_config());
    let mut used_proc_names: FxHashSet<String> = FxHashSet::default();
    let mut proc_keys: Vec<&String> = analysis.all_procs.keys().collect();
    proc_keys.sort();
    for qname in proc_keys {
        let proc_def = &analysis.all_procs[qname];
        let name = &proc_def.name;
        if name.len() <= 1 || tcl_syntax::naming::is_qualified(name.as_bytes()) {
            continue;
        }
        // A proc that overrides a registry-known command (`proc unknown
        // …`, an `auto_*` replacement, a shadowed builtin) is invoked by
        // the interpreter or library through that exact spelling — its
        // name is load-bearing.
        if builtin_names.contains(name.as_str()) {
            continue;
        }
        let Some(short) = proc_gen.next_avoiding_observed(
            |name| builtin_names.contains(name) || used_proc_names.contains(name),
            None,
        ) else {
            continue;
        };
        let short = short.into_name();
        if short.len() >= name.len() {
            continue;
        }
        used_proc_names.insert(short.clone());

        let r = proc_def.name_span;
        let actual = slice(source, r);
        let def_span = r.as_range();
        if actual == *name {
            edits.push(SourceEdit {
                span: def_span.clone(),
                text: short.clone(),
            });
        }
        for call in current_proc_call_sites(source, proc_def, analysis) {
            let call_text = slice(source, call);
            if call.as_range() != def_span && call_text == *name {
                edits.push(SourceEdit {
                    span: call.as_range(),
                    text: short.clone(),
                });
            }
        }
        symbol_map.procs.insert(name.clone(), short);
    }
}

/// The isolated Logical formal purpose has its own exact binding inventory.
/// General unknown-mutation keyword and variable barriers remain unchanged.
pub(super) fn compact_logical_formals(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    analysis: &AnalysisResult,
    isolated: bool,
    renamed_scopes: &[&Scope],
    symbol_map: &mut SymbolMap,
    edits: &mut Vec<SourceEdit>,
) {
    // naming.minifier.logical-formal-binding-alpha
    // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
    if !isolated {
        return;
    }
    let Some(inventory) =
        tcl_compiler::registry_invocation::logical_formals::original_logical_procedure_bindings(
            source, analysis,
        )
    else {
        return;
    };
    for procedure in inventory {
        if !procedure.matches_source(source, input) || !procedure.alpha_binding_lookup_closed() {
            continue;
        }
        let selected_scope = source_scope_at_offset(
            &analysis.global_scope,
            procedure.body().content_span().start(),
            "::",
            false,
        )
        .filter(|(scope, _)| {
            scope.kind == ScopeKind::Proc
                && scope.body_span == Some(procedure.body().content_span())
        });
        if selected_scope.as_ref().is_some_and(|(scope, _)| {
            renamed_scopes
                .iter()
                .any(|renamed| std::ptr::eq(*renamed, *scope))
        }) {
            continue;
        }
        let label = selected_scope.map_or_else(
            || format!("logical@{}", procedure.body().content_span().start()),
            |(_, label)| label,
        );
        let existing = procedure
            .formals()
            .iter()
            .map(|formal| formal.name().to_owned())
            .chain(std::iter::once("args".to_owned()))
            .collect::<FxHashSet<_>>();
        let mut names = AuthoredCompactNameSequence::new(input.lexer_config());
        let mut mapping = BTreeMap::new();
        let mut claimed = FxHashSet::default();
        for formal in procedure.formals() {
            if formal.name().len() <= 1 {
                continue;
            }
            let Some(short) = names.next_avoiding_observed(
                |name| existing.contains(name) || claimed.contains(name),
                Some(1000),
            ) else {
                continue;
            };
            let short = short.into_name();
            if short.len() >= formal.name().len() {
                continue;
            }
            claimed.insert(short.clone());
            for span in std::iter::once(formal.declaration_span())
                .chain(formal.references().iter().map(tcl_compiler::registry_invocation::logical_formals::OriginalLogicalFormalReference::name_span))
            {
                let range = span.as_range();
                edits.push(SourceEdit { span: range, text: short.clone() });
            }
            mapping.insert(formal.name().to_owned(), short);
        }
        if !mapping.is_empty() {
            record_variable_report(symbol_map, &label, mapping);
        }
    }
}

/// Read-only context for the recursive scope rename walk: the document
/// `source`, the analyser result, the `isolated` flag, and the precomputed
/// rename-barrier / read-modify-write-target sets.
#[derive(Clone, Copy)]
struct ScopeCtx<'a> {
    pub(super) references: Option<&'a crate::source_structure::OriginalVariableReferences<'a>>,
    pub(super) config: LexerConfig,
    pub(super) source: &'a str,
    pub(super) analysis: &'a AnalysisResult,
    pub(super) isolated: bool,
    barriers: &'a RenameBarriers<'a>,
    rmw_targets: &'a FxHashSet<String>,
}

/// Mutable outputs accumulated by [`process_scope`]: the symbol map and the
/// edit list.
struct ScopeOut<'a, 'scope> {
    pub(super) symbol_map: &'a mut SymbolMap,
    edits: &'a mut Vec<SourceEdit>,
    renamed_scopes: &'a mut Vec<&'scope Scope>,
}

/// Exact name extents within current original reads, plus unchanged bare role
/// operands. Missing correspondence prevents a partial variable rename.
pub(super) fn variable_reference_name_spans(
    source: &str,
    geometry: &crate::source_structure::OriginalVariableReferences<'_>,
    name: &str,
    references: &[tcl_lexer::Span],
) -> Option<Vec<tcl_lexer::Span>> {
    let mut names = Vec::new();
    for &span in references {
        if let Some(original) = geometry.reference_at(span.start()) {
            if original.root != name
                || (original.token_span != span && original.whole_span != span)
                || source.get(original.name_span.as_range()) != Some(name)
            {
                return None;
            }
            names.push(original.name_span);
        } else if source.get(span.as_range()) == Some(name) {
            names.push(span);
        } else {
            return None;
        }
    }
    names.sort_by_key(|span| (span.start(), span.end()));
    names.dedup();
    Some(names)
}

/// Recursively rename variables (and params) in a scope, mirroring
/// `_process_scope`.
fn process_scope<'a>(
    ctx: ScopeCtx<'a>,
    scope: &'a Scope,
    scope_label: &str,
    out: &mut ScopeOut<'_, 'a>,
) {
    let ScopeCtx {
        references,
        config,
        source,
        analysis,
        isolated,
        barriers,
        rmw_targets,
    } = ctx;
    let rename_scope = (scope.kind == ScopeKind::Proc
        || (isolated && scope.kind == ScopeKind::Global))
        && barriers.allows_scope(scope);

    if rename_scope {
        // Identify the proc this scope belongs to by its body span — unique per
        // proc — not a namespace-blind `pd.name == scope.name` scan.  The scope
        // name is the proc name *as written* (a bare `dup` inside two different
        // namespaces), so the simple-name scan matched an arbitrary same-named
        // proc in `HashMap` order and handed `rename_params_in_list` the wrong
        // declaration's parameter region, corrupting the output on a collision
        // (renaming one proc's `$use` sites while leaving its parameter — or a
        // colliding local — under the other proc's name).  The body span keys
        // straight to this proc regardless of name collisions.
        let proc_def = if scope.kind == ScopeKind::Proc {
            scope
                .body_span
                .and_then(|bs| analysis.all_procs.values().find(|pd| pd.body_span == bs))
        } else {
            None
        };
        let param_names: FxHashSet<&str> =
            proc_def.map_or_default(|pd| pd.params.iter().map(|p| p.name.as_str()).collect());
        // Obtain all formal geometry before adding any parameter-body edits.
        // An unavailable header or cooked name cannot leave a partial rename.
        let formal_sources = proc_def.and_then(|procedure| {
            lexical_metadata::parameter_name_sources(source, analysis, procedure)
        });

        let mut var_gen = AuthoredCompactNameSequence::new(config);
        let existing: FxHashSet<String> = scope.variables.keys().cloned().collect();
        let mut var_map: BTreeMap<String, String> = BTreeMap::new();

        let mut var_names: Vec<&String> = scope.variables.keys().collect();
        var_names.sort();
        for var_name in var_names {
            let var_def = &scope.variables[var_name];
            if var_name.len() <= 1
                || var_name.contains('\0')
                || tcl_syntax::naming::is_qualified(var_name.as_bytes())
            {
                continue;
            }
            // A local that aliases a namespace / global cell (`global v`,
            // `variable v`, `namespace upvar …`) shares that cell's public
            // name — renaming the local spelling would detach it from the
            // cell.  (Scopes containing the aliasing command are already
            // barred; this guards the same variable observed from a scope
            // that is not.)
            if var_def.link_target.is_some() {
                continue;
            }
            // Skip variables that are the bare write-target of a mutating
            // command (`incr` / `append` / `lappend`): the analyser records
            // those as definitions, not reads, so `VarDef.references` (reads
            // only) misses the target argument. Renaming the `set` / `$var`
            // sites but not the `incr var` site would mutate a different
            // variable than is read, corrupting the program. Keeping the name
            // unchanged everywhere is semantics-preserving (at the cost of less
            // compaction for that one name).
            if rmw_targets.contains(var_name.as_str()) {
                continue;
            }
            let is_param = param_names.contains(var_name.as_str());
            if is_param
                && (tcl_syntax::formal_params::is_variadic_formal_name(var_name)
                    || formal_sources.as_ref().is_none_or(|formals| {
                        !formals
                            .iter()
                            .any(|formal| formal.name() == var_name.as_bytes())
                    }))
            {
                continue;
            }

            if !is_param && slice(source, var_def.definition_span) != *var_name {
                continue;
            }
            let Some(reference_names) = references.and_then(|geometry| {
                variable_reference_name_spans(source, geometry, var_name, &var_def.references)
            }) else {
                continue;
            };
            let claimed: FxHashSet<String> = var_map.values().cloned().collect();
            let Some(short) = var_gen.next_avoiding_observed(
                |name| existing.contains(name) || claimed.contains(name),
                Some(1000),
            ) else {
                continue;
            };
            let short = short.into_name();
            if short.len() >= var_name.len() {
                continue;
            }
            let declaration = (!is_param).then_some(var_def.definition_span);
            for span in declaration.into_iter().chain(reference_names) {
                out.edits.push(SourceEdit {
                    span: span.as_range(),
                    text: short.clone(),
                });
            }
            var_map.insert(var_name.clone(), short);
        }

        if let Some(formals) = formal_sources.as_deref()
            && !var_map.is_empty()
        {
            rename_params_in_list(formals, &var_map, out.edits);
        }
        if !var_map.is_empty() {
            out.renamed_scopes.push(scope);
            record_variable_report(out.symbol_map, scope_label, var_map);
        }
    }

    for child in &scope.children {
        let label = child_scope_label(scope_label, &child.name);
        process_scope(ctx, child, &label, out);
    }
}

/// Variable names that are the bare target of a read-modify-write
/// command (`incr` / `append` / `lappend` / `lset` — the registry's
/// `rmw_first_arg_variable` set; a whole-value `set` is rename-safe) or
/// of a variable-destroying command (`unset` —
/// [`Traits::DESTROYS_VARIABLE`]). The analyser records these as
/// definitions, not reads, so [`AnalysisResult`]'s `VarDef.references`
/// (reads plus re-definition sites) can miss the target argument. The
/// name compaction excludes them so it cannot rename the `set` / `$var`
/// sites while leaving the `incr var` / `unset var` target untouched
/// (which would corrupt the program).
fn rmw_target_var_names(
    env: MinifyEnv<'_>,
    commands: &[tcl_lexer::NativeScriptCommandWords],
) -> FxHashSet<String> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let mut names = FxHashSet::default();
    for command in commands {
        lexical_metadata::with_command_schema(env, command, |schema| {
            if !schema
                .semantics
                .traits
                .contains(Traits::FIRST_ARG_VARNAME | Traits::READS_BEFORE_WRITE)
                && !schema.semantics.traits.contains(Traits::DESTROYS_VARIABLE)
            {
                return;
            }
            let (roles, complete) = schema.authored_source_argument_roles();
            if !complete {
                return;
            }
            for (index, role) in roles {
                if role != ArgRole::VarWrite {
                    continue;
                }
                if let Some(name) = schema
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))
                    .and_then(|index| index.checked_add(1))
                    .and_then(|index| command.words.get(index))
                    .and_then(lexical_metadata::ascii_value)
                {
                    names.insert(name);
                }
            }
        });
    }
    names
}
