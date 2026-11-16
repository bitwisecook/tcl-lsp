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

//! `analyse_proc_body_isolated`: one procedure or method body analysed at offset 0 in a reconstructed scope with seeded formals and frame state.

use super::super::state::Analyser;
use super::{BodyFragment, DeferredBody};
use tcl_lexer::Token;

/// Seed an isolated body's throwaway result with the ensemble facts the shell
/// attached ([`Analyser::attach_ensemble_facts`]) — the `subcommand → target`
/// maps, so an `<ensemble> <sub> …` call inside the body records the same
/// existence-probed invocation the whole-file walk does (provenance included,
/// so the body's dispatch words carry the same `-map`/`-subcommands` tag rename
/// gates on), **and** the `-prefixes 0` opt-out that decides
/// whether an abbreviation of one of those subcommands may match at all.
///
/// The counterpart of [`Analyser::graft_proc_body`]'s merge-back, and the one
/// site that reads `DeferredBody`'s ensemble pair, so the two facts cannot
/// drift apart.
fn seed_ensemble_facts(a: &mut super::super::state::Analyser, db: &DeferredBody) {
    for (ensemble, subs) in &db.ensemble_targets {
        a.result
            .ensemble_subcommand_targets
            .entry(ensemble.clone())
            .or_default()
            .extend(subs.iter().cloned());
    }
    a.result
        .prefixless_ensembles
        .extend(db.prefixless_ensembles.iter().cloned());
}

/// Analyse one `proc` **or method** body as an isolated unit at **offset 0** — a
/// pure function of `(body_text, namespace, scope_name, params, is_method,
/// seeded_variables)`, so a shifted-but-unedited body is a cache hit
/// (offset-invariant).  The enclosing namespace + params are reconstructed
/// (a `Method` scope with instance variables pre-bound for methods, a `Proc`
/// scope otherwise); params / instance vars get a placeholder definition span
/// (the aggregator keeps the shell's real one).  Spans/lines are relative to the
/// body start; [`Analyser::graft_proc_body`] rebases them by the body's real
/// position.
#[must_use]
pub fn analyse_proc_body_isolated<S: std::hash::BuildHasher>(
    db: &DeferredBody,
    dialect: &str,
    disabled: &std::collections::HashSet<String, S>,
    non_ascii: super::super::state::NonAsciiMode,
    declared_commands: Option<tcl_registry::model::DeclaredSurface>,
    workspace_class_factories: Option<std::sync::Arc<super::super::types::ClassFactoryIndex>>,
    workspace_declared_commands: Option<std::sync::Arc<tcl_registry::model::DeclaredSurface>>,
) -> BodyFragment {
    // Rebuild into the default-hasher set `Analyser` stores.
    let disabled: std::collections::HashSet<String> = disabled.iter().cloned().collect();
    // The workspace factory oracle travels with the body: a `Meta create …`
    // inside a proc body is classified by the whole-file walk, so it must be
    // classified identically here or the two strategies diverge on a class
    // (the same trap the ensemble-call-site and global-cell paths fall into).
    // Gated by
    // `the_per_item_walk_agrees_with_the_whole_file_walk_cross_file`.
    let mut a = Analyser::with_disabled_diagnostics(disabled)
        .with_non_ascii_mode(non_ascii)
        .with_workspace_class_factories(workspace_class_factories)
        .with_workspace_declared_commands(workspace_declared_commands);
    if let Some(input) = &db.resolved_input {
        a = a.with_resolved_input(input.for_nested_source());
    }
    a.resolve_walk_environment(dialect);
    a.result.body_lexer_config = Some(a.lexer_config());
    a.result.lexical_declaration_advice = a.selected_logical_declaration_advice();
    a.result.resolved_input = Some(a.resolved_analysis_input());
    a.declared_commands = declared_commands;
    // Offset 0: the body content is the whole source; a synthetic `Str` body
    // token spans it with `content_offset = 0` (no `{` to skip).
    a.retain_walk_source(&db.body_text);
    // The isolated pass has no whole-file view, so the constant
    // command-substitution fold may trust only the snapshot
    // the shell attached — and must distrust EVERYTHING when none was
    // (lowering the body-only `a.source` would miss a `rename` elsewhere
    // in the file).
    a.command_trust = Some(std::sync::Arc::new(db.command_trust.as_ref().map_or_else(
        crate::command_binding::ModuleCommandMutations::distrust_all,
        |snap| snap.to_mutations(),
    )));
    // Source spans are `u32`; a proc body longer than 4 GiB is not
    // representable (and never occurs), so clamp to `u32::MAX`.
    let body_len = u32::try_from(db.body_text.len()).unwrap_or(u32::MAX);
    let body_tok = Token::new(tcl_lexer::TokenType::Str, tcl_lexer::Span::new(0, body_len));
    a.registry = Some(a.profile_registry());
    a.line_offsets = Some(super::super::state::compute_line_offsets(&a.source));
    // Capture qualified (`::`/`static::`) reads that miss the (empty) enclosing
    // global scope, so the graft can replay them on the shell's real globals.
    a.capture_global_reads = Some(Vec::new());
    // …and the write-side twin: a fixed global cell this body *defines*
    // (`upvar ::ns::cell local`) must reach the shell's real global scope,
    // not the throwaway root this isolated walk builds.
    a.capture_global_defs = Some(Vec::new());
    seed_ensemble_facts(&mut a, db);
    // Capture object-instance creations: the isolated body's `all_classes` is
    // empty, so the class can't be resolved here — the graft replays them.
    a.pending_instances = Some(Vec::new());
    a.pending_bareword_dispatch_sites = Some(Vec::new());
    // A method body is analysed in a `Method` scope (so `in_method` dispatch
    // recording fires) under the class's defining namespace, with the class
    // instance variables pre-bound; a proc body in a `Proc` scope.
    let kind = if db.is_method {
        super::super::types::ScopeKind::Method
    } else {
        super::super::types::ScopeKind::Proc
    };
    let proc_path = reconstruct_proc_scope(
        &mut a.result.global_scope,
        &db.namespace,
        &db.scope_name,
        kind,
    );
    seed_isolated_frame_state(&mut a, db, &proc_path);
    a.analyse_body(&db.body_text, body_tok, &proc_path);
    let proc_scope = super::super::scope::scope_at_mut(&mut a.result.global_scope, &proc_path)
        .expect("reconstructed proc scope")
        .clone();
    BodyFragment {
        result: a.result,
        proc_scope,
        ensembles: a.ensemble_namespaces,
        pending_arity: a.pending_arity,
        pending_option_conflicts: a.pending_option_conflicts,
        pending_user_call_arity: a.pending_user_call_arity,
        pending_ctor_arity: a.pending_ctor_arity,
        pending_next_arity: a.pending_next_arity,
        var_sites: a.var_command_sites,
        cmd_sites: a.cmd_command_sites,
        global_reads: a.capture_global_reads.unwrap_or_default(),
        global_defs: a.capture_global_defs.unwrap_or_default(),
        disabled_commands: a.pending_disabled_commands,
        private_ns_calls: a.pending_w143,
        w304: a.pending_w304,
        instances: a.pending_instances.unwrap_or_default(),
        bareword_dispatch_sites: a.pending_bareword_dispatch_sites.unwrap_or_default(),
        walk_aliases: a.command_aliases,
        walk_renames: a.renamed_commands,
        walk_alias_offsets: a.alias_offsets,
        walk_rename_offsets: a.rename_offsets,
        walk_deleted: a.deleted_commands,
        const_dispatches: a.pending_const_dispatches,
        instance_class_sites: a.pending_instance_class_sites,
        widget_sites: a.widget_dispatch_sites,
        proven_sites: a.proven_sites,
        ensemble_offsets: a.ensemble_record_offsets,
        loop_candidates: a.loop_candidates,
        minted_synthetics: a.minted_synthetic_names,
    }
}

/// Build `global -> namespace* -> proc` scopes mirroring the proc's lexical
/// context (so nested defs qualify identically), returning the proc scope path.
/// Build a `namespace`-rooted scope chain under `root` and push a `kind` scope
/// named `scope_name` at its end, returning the path to that scope.
///
/// Used both by the per-item isolated re-analysis (to reconstruct a proc /
/// method context) and by `handle_apply_command` (to root an `apply` lambda
/// scope at the lambda's namespace rather than the caller's), so the inline and
/// per-item walks build byte-identical structure.
pub(crate) fn reconstruct_proc_scope(
    root: &mut super::super::types::Scope,
    namespace: &str,
    scope_name: &str,
    kind: super::super::types::ScopeKind,
) -> Vec<usize> {
    use super::super::types::Scope;
    use super::super::types::ScopeKind;
    // `namespace` is a constructed key: split it by the construction-inverse
    // rule so a legitimately colon-named segment survives — a
    // char-pattern trim + `split("::")` would collapse it.
    let comps = crate::naming::key_segments(namespace);
    let mut path: Vec<usize> = Vec::new();
    for comp in comps {
        let parent = super::super::scope::scope_at_mut(root, &path).expect("scope path");
        let i = parent.children.len();
        parent
            .children
            .push(Scope::new(ScopeKind::Namespace, &comp));
        path.push(i);
    }
    let parent = super::super::scope::scope_at_mut(root, &path).expect("scope path");
    let pi = parent.children.len();
    parent.children.push(Scope::new(kind, scope_name));
    path.push(pi);
    path
}

fn seed_isolated_formal_bindings(a: &mut Analyser, db: &DeferredBody, proc_path: &[usize]) {
    let placeholder = tcl_lexer::Span::new(0, 0);
    let dummy = Token::new(tcl_lexer::TokenType::Str, placeholder);
    // Re-binding params / instance variables into the isolated scope is purely
    // structural (so body references resolve). The shell walk already emitted
    // W215 for these declarations byte-identically to the full `analyse` path,
    // and the synthetic `dummy` token's kind would otherwise flip the W215
    // `braced` reachability heuristic — so suppress W215 across the rebind to
    // avoid a spurious duplicate. The body walk below re-enables it so genuine
    // in-body declarations are still checked.
    a.structural_rebind = true;
    for p in &db.params {
        a.define_var(&p.name, dummy, proc_path, false, Some(placeholder));
    }
    // Class instance variables — visible in every method body (skip ones that a
    // formal parameter already shadows, matching `walk_method_body`).
    for var in &db.seeded_variables {
        let base =
            tcl_syntax::naming::split_element_ref(var).map_or(var.as_str(), |(root, _)| root);
        if base.is_empty() || db.params.iter().any(|p| p.name == base) {
            continue;
        }
        // The isolated body only needs the binding to exist so `$v` reads
        // resolve; the authoritative `definition_span` comes from the shell
        // walk and is preserved by the graft (`merge_one_var`), so a
        // `placeholder` here is discarded for this shell-owned key.
        a.define_var(base, dummy, proc_path, false, Some(placeholder));
    }
    a.structural_rebind = false;
}

fn seed_isolated_frame_state(a: &mut Analyser, db: &DeferredBody, proc_path: &[usize]) {
    // A `TclOO` method's isolated scope resolves bare commands globally, like
    // its whole-file twin (`Scope::oo_global_resolution`).
    if db.oo_global_resolution
        && let Some(scope) =
            super::super::scope::scope_at_mut(&mut a.result.global_scope, proc_path)
    {
        scope.oo_global_resolution = true;
        // Every `DeferredBody` is a real method body — `walk_class_init_body`
        // walks the class-level `initialise` frame inline and never defers —
        // so the isolated scope is a method frame too (`Scope::oo_method_frame`).
        scope.oo_method_frame = true;
        // And the instance-side defining-class fact travels with the body,
        // so `[self class]` folds identically here and on
        // the whole-file walk.
        scope.oo_defining_class.clone_from(&db.oo_defining_class);
    }
    seed_isolated_formal_bindings(a, db, proc_path);
    // Restore the enclosing safe-interpreter visibility context so a hidden
    // Retain the original conditional receipt. Exact source/input checks are
    // performed by its emitter; a body-only foreign projection cannot recapture
    // an original child visibility context from nominal names.
    if let Some(context) = &db.safe_interp_ctx {
        a.safe_interp_stack.push(context.clone());
    }
}
