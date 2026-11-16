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

//! Whole-file analysis: the checked document input, the per-item incremental path and each procedure body's memoised isolated analysis.

use super::compilation_units::{compilation_unit_for_config, lexer_cfg_key};
use super::{AnalyserConfig, OverlayEpoch, SourceFile, TclDb};
use std::collections::HashSet;
use std::sync::Arc;
use tcl_compiler::analyser::per_item::{BodyFragment, DeferredBody, analyse_proc_body_isolated};
use tcl_compiler::analyser::{Analyser, AnalysisResult, NonAsciiMode};
use tcl_compiler::signature_scan::types::ParamDef;
use tcl_registry::model::{DeclaredSurface, OverlayMiss};

pub(super) fn document_analyser_without_factory_oracle(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Analyser {
    let disabled = config.disabled_diagnostics(db).iter().cloned().collect();
    let extra = config.extra_commands(db).iter().cloned().collect();
    Analyser::with_disabled_diagnostics(disabled)
        .with_non_ascii_mode(config.non_ascii_mode(db))
        .with_pack_overlay(config.spec_pack_key(db))
        .with_extra_commands(extra)
        .with_bigip_version(config.bigip_version(db).clone())
        .with_declared_targets(config.targets(db).clone())
        .with_package_provides(config.package_provides(db).clone())
        .with_script_encoding(config.script_encoding(db))
        .with_file_path(file.path(db).clone())
        .with_workspace_subclass_methods(file.workspace_subclass_methods(db).clone())
        .with_workspace_declared_commands(file.workspace_declared_commands(db).clone())
        .with_workspace_source_modules(file.workspace_source_modules(db).clone())
}

pub(super) fn document_analyser(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Analyser {
    document_analyser_without_factory_oracle(db, file, config)
        .with_workspace_class_factories(file.workspace_class_factories(db).clone())
}

/// [`document_analysis_input`] for a consumer that walks the source. The input
/// carries the document's workspace modules without comparing on them, so the
/// consumer also depends on the field itself.
pub(crate) fn walked_document_input(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Result<Arc<tcl_compiler::analyser::ResolvedAnalysisInput>, OverlayMiss> {
    let _modules = file.workspace_source_modules(db);
    document_analysis_input(db, file, config)
}

/// The checked document input shared by analysis and compilation. This query
/// resolves context only; it performs no source walk and depends on no CU query.
///
/// # Errors
/// An unavailable overlay remains a typed miss until its epoch changes.
#[salsa::tracked(returns(clone))]
pub fn document_analysis_input(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Result<Arc<tcl_compiler::analyser::ResolvedAnalysisInput>, OverlayMiss> {
    if config.spec_pack_key(db) != 0 {
        let _epoch = OverlayEpoch::try_get(db).map_or(0, |epoch| epoch.generation(db));
    }
    document_analyser_without_factory_oracle(db, file, config)
        .prepare_analysis_input(file.dialect(db))
        .map(Arc::new)
}

fn unavailable_document_analysis(dialect: &str, miss: OverlayMiss) -> Arc<AnalysisResult> {
    let mut result = AnalysisResult::default();
    dialect.clone_into(&mut result.dialect);
    result.analysis_context_unavailable = Some(miss);
    Arc::new(result)
}

/// Whole-file analysis, behind an `Arc` so reads bump a refcount rather than
/// deep-clone.
///
/// Wraps [`Analyser::analyse`] unchanged, uncancellable and with no per-item
/// memoisation.  Every production feature provider reads
/// [`file_analysis_incremental`] instead: this coarse query has no
/// interior salsa cancellation checkpoint, so a caller holding a read blocks a
/// concurrent edit's `set_text` until the whole walk finishes. `file_analysis`
/// itself stays live as the differential-fuzzer / corpus-gate ground truth
/// [`file_analysis_incremental`] is proven byte-identical against.
///
/// One deliberate asymmetry: this query lets the analyser build its **own**
/// compilation unit, so it never sees [`SourceFile::external_call_sites`],
/// while [`file_analysis_incremental`] feeds it the shared
/// [`compilation_unit`] (which does). The two therefore agree exactly when
/// the file has no cross-file view — which is every gate and fuzzer input,
/// since [`SourceFile::new`] leaves the field `None`. Production reads only
/// the incremental query.
///
/// `config.disabled_diagnostics` is the analyser's production-time skip —
/// rule 2's permitted saving in `docs/design/compiler/diagnostic-policy.md`
/// § What the producers leave to the policy: the codes the document's policy
/// turns off, which the analyser need not compute. It is never a presentation filter. The
/// surface that reads this analysis declares the same set to its report, so a
/// code left uncomputed is explained rather than read as clean; what the
/// document shows is the policy step's decision.
#[salsa::tracked(returns(clone))]
pub fn file_analysis(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<AnalysisResult> {
    let _sidecar_stubs_epoch = file.sidecar_stubs_epoch(db);
    let input = match walked_document_input(db, file, config) {
        Ok(input) => input,
        Err(miss) => return unavailable_document_analysis(file.dialect(db), miss),
    };
    Arc::new(
        document_analyser(db, file, config)
            .with_resolved_input((*input).clone())
            .analyse(file.text(db), file.dialect(db)),
    )
}

/// Interned identity of a single `proc` body's isolated analysis — the per-item
/// firewall's memoisation key.  **Offset-invariant**: it holds only what the
/// offset-0 analysis consumes (body text + enclosing namespace / name / params +
/// config), *not* the body's position — so a shifted-but-unedited proc has the
/// same key and reuses the cached [`item_body_analysis`] (the aggregator rebases
/// the offset-0 facts by the body's real span).
///
/// **Per-revision key — reclaimed by salsa's interned garbage collector.**
/// `body_text` changes on every keystroke inside the body, so each edit mints a
/// fresh id here.  What keeps that bounded is the collector reusing the LRU
/// tail's stale slots (`clear_memos` releasing the retained analysis `Arc`s),
/// which happens only for `Durability::LOW` slots interned inside a tracked
/// query.  See the crate docs' "The interned garbage collector is
/// load-bearing"; pinned by `tests/interned_gc.rs`.
#[salsa::interned]
pub struct ItemBodyKey<'db> {
    /// The proc / method body source, shared with the
    /// [`tcl_compiler::analyser::per_item::DeferredBody`] the aggregator built
    /// it from rather than copied out of it.  Interning is content-addressed, so
    /// the text must be *in* the key; what it must not be is a fresh `String`
    /// per proc per edit — on a 114-proc file that is 114 full body copies
    /// before a single memo is consulted, with the copies discarded the
    /// moment the intern hits.  `Arc<str>` also makes
    /// [`item_body_analysis`]'s rebuild of the `DeferredBody` a refcount bump.
    #[returns(ref)]
    pub body_text: Arc<str>,
    #[returns(ref)]
    pub namespace: String,
    #[returns(ref)]
    pub scope_name: String,
    #[returns(ref)]
    pub params: Vec<ParamDef>,
    /// `true` for a `TclOO` method body (isolated in a `Method` scope with
    /// instance variables pre-bound); `false` for a `proc`.
    #[returns(copy)]
    pub is_method: bool,
    /// Mirrors `Scope::oo_global_resolution`: `true` for a TclOO method
    /// body (bare commands resolve globally — object-namespace semantics),
    /// `false` for procs and snit / itcl members.
    #[returns(copy)]
    pub oo_global_resolution: bool,
    /// Variables pre-bound in the body: a method body's class instance
    /// variables, or a procedure's static variables.
    #[returns(ref)]
    pub seeded_variables: Vec<String>,
    /// The constant command-substitution fold context: the
    /// whole-file command-mutation trust snapshot the shell attached for a
    /// body with a fold candidate, paired with the instance-side `TclOO`
    /// defining class (`[self class]`'s answer) when the body is such a
    /// method — mirrors
    /// [`tcl_compiler::analyser::per_item::DeferredBody::command_trust`] /
    /// `oo_defining_class`. One field because the class fact is only ever
    /// consumed under a trust snapshot (no snapshot ⇒ the fold distrusts
    /// everything and never reads the class). Part of the key so a `rename`
    /// appearing (or disappearing) anywhere in the file re-analyses exactly
    /// the bodies whose folds it could affect; `None` (no candidate) keeps
    /// the key untouched by unrelated renames.
    #[returns(ref)]
    pub fold_ctx: Option<(
        Arc<tcl_compiler::command_binding::CommandTrustSnapshot>,
        Option<String>,
    )>,
    /// The body's enclosing-environment snapshot, several halves in one field
    /// (salsa interns the field tuple, whose `Hash` impl caps at 12
    /// elements — and this struct is already at that cap, which is why these
    /// travel packed rather than as fields of their own):
    ///
    /// - `.0` — the ensemble environment, itself a pair that must stay a
    ///   pair:
    ///   - `.0.0` — the ensemble `subcommand → target` maps visible to this
    ///     body, mirroring
    ///     [`tcl_compiler::analyser::per_item::DeferredBody::ensemble_targets`]
    ///     (already canonically sorted and filtered to ensembles the body
    ///     mentions, so an unrelated ensemble edit re-keys no body). Part of
    ///     the key so an `<ensemble> <sub> …` call inside the body
    ///     re-analyses when the mapping it resolves through changes.
    ///   - `.0.1` — the `-prefixes 0` opt-out for those same ensembles,
    ///     mirroring
    ///     [`tcl_compiler::analyser::per_item::DeferredBody::prefixless_ensembles`].
    ///     **Bound to `.0.0` by type on purpose.**
    ///     `AnalysisResult::resolve_ensemble_subcommand` reads the two
    ///     together — the map says which subcommands exist, this says
    ///     whether an abbreviation of one may match — so seeding the map
    ///     alone lets an isolated body resolve `g fo` against a
    ///     `-prefixes 0` ensemble and record a dispatch the whole-file walk
    ///     (and real Tcl) refuse. This query *resolves* the body rather than
    ///     replaying a recorded answer, so the salsa altitude needs the
    ///     opt-out exactly as much as the whole-file walk does; the nested
    ///     pair is what stops a future edit from cloning one without the
    ///     other.
    /// - `.1` — the complete original conditional child-visibility receipt,
    ///   mirroring
    ///   [`tcl_compiler::analyser::per_item::DeferredBody::safe_interp_ctx`].
    ///   Its hash retains full source/configuration, immutable editing input,
    ///   scoped visible slots and independent hidden-token relationships.
    ///   It never supplies selected callability or native hidden-table identity.
    /// - `.2` — the workspace **class factory** oracle
    ///   ([`SourceFile::workspace_class_factories`]). A
    ///   `Meta create …` inside a proc body is classified by the whole-file
    ///   walk from this index, so the isolated body pass must see the same
    ///   one or the two strategies disagree about whether a class exists.
    ///   Part of the key for the usual reason: a metaclass appearing or
    ///   changing shape elsewhere in the workspace must re-analyse the
    ///   bodies whose creation calls it decides.
    /// - `.4` — the procedures other documents declare to this one
    ///   ([`SourceFile::workspace_declared_commands`]), so a cross-file call
    ///   inside the body is read as the whole-file walk reads it.
    #[returns(ref)]
    pub body_env: (
        (
            Vec<(
                String,
                Vec<(
                    String,
                    tcl_compiler::analyser::types::EnsembleSubcommandTarget,
                )>,
            )>,
            Vec<String>,
        ),
        Option<tcl_compiler::analyser::SourceInterpreterVisibilitySnapshot>,
        Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
        Option<tcl_compiler::analyser::ResolvedAnalysisInput>,
        Option<Arc<DeclaredSurface>>,
    ),
    #[returns(ref)]
    pub dialect: String,
    #[returns(ref)]
    pub disabled: Vec<String>,
    #[returns(copy)]
    pub non_ascii: NonAsciiMode,
}

/// Memoised offset-0 isolated analysis of one `proc` body.  A body-only edit
/// changes only that body's [`ItemBodyKey`], so salsa reuses every other body's
/// result; an edit that merely *shifts* a body leaves its key unchanged.
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn item_body_analysis<'db>(db: &'db dyn TclDb, key: ItemBodyKey<'db>) -> Arc<BodyFragment> {
    // The isolated analysis works at offset 0 and ignores `body_tok` / scope
    // path (the aggregator supplies the real position when grafting), so a
    // placeholder token is fine.
    let (command_trust, oo_defining_class) = match key.fold_ctx(db) {
        Some((trust, class)) => (Some(Arc::clone(trust)), class.clone()),
        None => (None, None),
    };
    let body = DeferredBody {
        resolved_input: key.body_env(db).3.clone(),
        body_text: Arc::clone(key.body_text(db)),
        body_tok: tcl_lexer::Token::new(tcl_lexer::TokenType::Str, tcl_lexer::Span::new(0, 0)),
        scope_path: Vec::new(),
        is_method: key.is_method(db),
        oo_global_resolution: key.oo_global_resolution(db),
        namespace: key.namespace(db).clone(),
        scope_name: key.scope_name(db).clone(),
        params: key.params(db).clone(),
        seeded_variables: key.seeded_variables(db).clone(),
        command_trust,
        oo_defining_class,
        // The two ensemble halves are seeded together, never separately —
        // see `ItemBodyKey::body_env`.
        ensemble_targets: key.body_env(db).0.0.clone(),
        prefixless_ensembles: key.body_env(db).0.1.clone(),
        safe_interp_ctx: key.body_env(db).1.clone(),
    };
    let disabled: HashSet<String> = key.disabled(db).iter().cloned().collect();
    let no_declarations = tcl_registry::model::DeclaredSurface::new();
    Arc::new(analyse_proc_body_isolated(
        &body,
        key.dialect(db),
        &disabled,
        key.non_ascii(db),
        Some(no_declarations),
        key.body_env(db).2.clone(),
        key.body_env(db).4.clone(),
    ))
}

/// Incremental whole-file analysis: the per-item path with each `proc` body's
/// isolated analysis memoised via [`item_body_analysis`], so a body edit
/// recomputes one body + the cheap shell instead of the whole walk; the
/// CFG/SSA diagnostic tail's per-procedure lattices are likewise memoised via
/// the salsa-native [`function_lattice`] query (through
/// `memoised_compilation_unit`), so an unchanged procedure's lattice is reused
/// (and rebased) instead of rebuilt.  Byte-identical to [`file_analysis`] (and
/// `analyse`) — proven by the `per_item_corpus` gate over the shared
/// `analyse_per_item_with` orchestration.
///
/// `config.disabled_diagnostics` is the analyser's production-time skip —
/// rule 2's permitted saving in `docs/design/compiler/diagnostic-policy.md`
/// § What the producers leave to the policy: the codes the document's policy
/// turns off, which the analyser need not compute. It is never a presentation filter. The
/// surface that reads this analysis declares the same set to its report, so a
/// code left uncomputed is explained rather than read as clean; what the
/// document shows is the policy step's decision.
// LRU-capped: per-file key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 64, returns(clone))]
pub fn file_analysis_incremental(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<AnalysisResult> {
    let _sidecar_stubs_epoch = file.sidecar_stubs_epoch(db);
    let disabled_vec = config.disabled_diagnostics(db).clone();
    let non_ascii = config.non_ascii_mode(db);
    let dialect = file.dialect(db).clone();
    let text = file.text(db).clone();
    let workspace_class_factories = file.workspace_class_factories(db).clone();
    let workspace_declared_commands = file.workspace_declared_commands(db).clone();
    let input = match walked_document_input(db, file, config) {
        Ok(input) => input,
        Err(miss) => return unavailable_document_analysis(&dialect, miss),
    };
    let mut analyser = document_analyser(db, file, config).with_resolved_input((*input).clone());

    // Build the CFG/SSA tail's compilation unit with per-procedure lattices
    // memoised by `function_lattice`, and feed it through the analyser's
    // `cu_override` seam, via the shared [`compilation_unit`] query.  The
    // document's own environment grammar mirrors what
    // `emit_cfg_ssa_diagnostics` builds for itself, so the supplied unit is
    // the one it would otherwise build; routing through the tracked query lets
    // `compiler_check_diagnostics` reuse this exact build in the same edit,
    // for *every* environment, because both consumers intern the same
    // environment id.
    let cfg_key = lexer_cfg_key(db, &dialect);
    // The input was checked before either producer walked source. A missing
    // generation returns above; it cannot reopen standalone compilation.
    if let Some(unit) = compilation_unit_for_config(db, file, cfg_key, config) {
        analyser.set_cu_override(unit);
    }

    let mut body_fn = |body: &DeferredBody| -> BodyFragment {
        let key = ItemBodyKey::new(
            db,
            Arc::clone(&body.body_text),
            body.namespace.clone(),
            body.scope_name.clone(),
            body.params.clone(),
            body.is_method,
            body.oo_global_resolution,
            body.seeded_variables.clone(),
            body.command_trust
                .clone()
                .map(|trust| (trust, body.oo_defining_class.clone())),
            (
                (
                    body.ensemble_targets.clone(),
                    body.prefixless_ensembles.clone(),
                ),
                body.safe_interp_ctx.clone(),
                workspace_class_factories.clone(),
                body.resolved_input.clone(),
                workspace_declared_commands.clone(),
            ),
            dialect.clone(),
            disabled_vec.clone(),
            non_ascii,
        );
        // Owned for the same reason as the lattice memo: `graft_proc_body`
        // rebases the offset-0 fragment to the body's real span and then moves
        // its fields into the shell, so the fragment cannot be shared.
        (*item_body_analysis(db, key)).clone()
    };
    Arc::new(analyser.analyse_per_item_with(&text, &dialect, &mut body_fn))
}
