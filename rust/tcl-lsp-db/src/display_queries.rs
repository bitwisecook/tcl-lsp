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

//! The queries whose values the editor displays: the outline, folding ranges and semantic tokens, with the per-file and project token facts the tokens classify against.

use super::compilation_units::{document_compilation_unit_for, token_registry};
use super::document_analysis::file_analysis_incremental;
use super::{AnalyserConfig, Project, SourceFile, TclDb};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_compiler::analyser::{
    Analyser, AnalysisResult, ClassDef, ClassHierarchy, build_class_hierarchy,
};
use tcl_lsp_core::document_symbols::DocumentSymbol;
use tcl_lsp_core::folding::FoldingRange;
use tcl_lsp_core::semantic_tokens::{SemanticTokens, VarNameArgRoles};

/// Document outline — wraps `document_symbols_from_analysis`, reusing the
/// tracked [`file_analysis_incremental`] so the outline shares the per-item
/// memoised analysis with the push-diagnostics path in the same edit.
// `returns(clone)`, not `returns(ref)`: the caller must be able to move the
// result out of `Cancelled::catch` and project it into `lsp-types` *outside*
// the read, because that projection has no cancellation checkpoint and a read
// held across it blocks a concurrent `set_text`. See the crate docs' "Return
// modes".
#[salsa::tracked(returns(clone))]
pub fn document_symbols(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Vec<DocumentSymbol> {
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::document_symbols::document_symbols_from_analysis(file.text(db), &analysis)
}

/// Semantic tokens — wraps `semantic_tokens::full_with_cu`; reads the durable
/// registry.
///
/// This query demands the document's [`CompilationUnit`] so a `regexp` /
/// `regsub` pattern supplied through a provably-constant string variable
/// highlights its originating `set` literal as a regex (see
/// [`tcl_compiler::regex_source`]), and the whole-file analysis so a
/// `$obj method …` / `[dict get $objs $k] method …` dispatch resolves against
/// user classes and their `oo::configurable` properties, not only
/// registry ones. Using the coarse, non-incremental [`file_analysis`] for
/// this instead of [`file_analysis_incremental`] would cost every token
/// request a *third* independent whole-file analyser walk (on top of
/// the two the diagnostics path already shares via [`compilation_unit`]), and
/// that walk has no interior salsa cancellation checkpoint, so a concurrent
/// edit's `set_text` would block until it finishes.  Using
/// [`file_analysis_incremental`] here instead gets the same correctness
/// (identical `AnalysisResult` shape, proven byte-identical to `file_analysis`
/// by the `per_item_corpus` gate) while sharing the diagnostics path's
/// per-item memoisation and cancellation checkpoints: a token request that
/// lands after diagnostics have already analysed this revision is a cache
/// hit, and a cold request is preemptible by the next edit instead of running
/// an uninterruptible pass to completion.
// `returns(clone)`: same liveness rule as the other bare-value queries, and the
// payload is a flat `Vec<u32>` the handler hands to the JSON-RPC layer by value
// anyway, so a borrow would buy nothing even if it were safe.
#[salsa::tracked(returns(clone))]
pub fn semantic_tokens(db: &dyn TclDb, file: SourceFile, config: AnalyserConfig) -> SemanticTokens {
    let registry = token_registry(db, file.dialect(db), config.spec_pack_key(db));
    let cu = document_compilation_unit_for(db, file, config);
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::semantic_tokens::full_with_cu_and_analysis(
        file.text(db),
        tcl_lsp_core::profile_for_dialect(file.dialect(db)),
        &registry,
        cu.as_deref(),
        Some(&analysis),
    )
}

/// The cross-file facts the project-level token aggregates read from **one**
/// file: its class definitions and its inferred variable-name argument roles.
///
/// The structure-only per-file boundary retains original source-display
/// receipts as well as compatibility projections. Original-image changes
/// invalidate those receipts; unchanged aggregate display maps can backdate
/// independently after their original providers have been checked.
#[derive(Clone, Default, PartialEq)]
pub struct FileTokenFacts {
    /// Classes declared in this file, keyed by qualified name.
    pub classes: HashMap<String, ClassDef>,
    /// The file's user-proc parameter roles.
    pub proc_roles: VarNameArgRoles,
    /// Readonly named-instance reports with separate existing compatibility
    /// and conditional source-display modes. Conditional labels supply no
    /// receiver identity; actual project token selection uses `class_cards`.
    pub named_instances: tcl_lsp_core::semantic_tokens::NamedInstanceMap,
    /// Genuine original class/factory/instance source-display owners.
    pub class_cards:
        Option<tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards>,
}

/// The light (structure-only) per-file tier feeding [`project_class_index`] and
/// [`project_proc_var_index`].
///
/// These aggregates read *every* file in the project, so whatever per-file
/// query they call decides what an interactive `semanticTokens` request costs
/// on a large workspace.  Reading the deep tier
/// ([`file_analysis_incremental`]) instead would mean one whole-workspace
/// *deep* analysis — CFG/SSA units, per-body lattices, diagnostic emitters —
/// behind a token request for a single file: measured at ~19 s of CPU over an
/// 883-file tcllib checkout, which no request survives.  Every attempt would
/// be cancelled by the next `set_text`, so it would never memoise and every
/// subsequent request would pay it again, while the in-flight read would
/// block the writer that cancelled it (`didOpen`'s `set_text` measured at
/// 270–660 ms) — producing open-to-tokens latency spikes.
///
/// The structural facts these aggregates want do not need the deep tier: an
/// unopened workspace file only ever needs the lightweight state the scan
/// leaves it at.  `structure_only` builds the identical declaration structure
/// while skipping diagnostic emission and cross-feature recording — the bulk of
/// the cost — for the same 883 files in ~2.9 s, and the result is a projection
/// small enough to backdate.
///
/// This explicit standalone query uses the file's stated dialect. Actual
/// document consumers use [`file_token_facts_for_config`], whose checked input
/// retains pack roles, availability and source grammar on the same light tier.
#[salsa::tracked(returns(clone))]
pub fn file_token_facts(db: &dyn TclDb, file: SourceFile) -> Arc<FileTokenFacts> {
    let mut analyser = Analyser::new()
        .structure_only()
        .with_file_path(file.path(db).clone());
    let result = analyser.analyse(file.text(db), file.dialect(db));
    token_facts_from_analysis(result)
}

pub(super) fn token_facts_from_analysis(result: AnalysisResult) -> Arc<FileTokenFacts> {
    let mut named_instances =
        tcl_lsp_core::semantic_tokens::NamedInstanceMap::from_analysis(&result);
    let class_cards =
        tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards::capture(
            &result,
        );
    if let Some(cards) = &class_cards {
        named_instances.extend_conditional_source_cards(cards);
    }
    Arc::new(FileTokenFacts {
        proc_roles: VarNameArgRoles::from_analysis(&result),
        class_cards,
        classes: result.all_classes,
        named_instances,
    })
}

/// The project's workspace-merged class hierarchy: every file's `ClassDef`s
/// unioned into one cross-file MRO index, so a `$obj method` dispatch resolves
/// against a class defined in *another* file.
///
/// Aggregates [`file_token_facts`]`(f).classes` across `project`.  A body-only
/// edit backdates at the per-file firewall, so this does not even re-aggregate.
/// On a class-signature change the merged index changes and the affected tokens
/// recompute — the correct cross-file invalidation.
#[salsa::tracked(returns(clone))]
pub fn project_class_index(db: &dyn TclDb, project: Project) -> Arc<ClassHierarchy> {
    let mut providers = HashSet::new();
    merge_project_classes(
        project
            .files(db)
            .iter()
            .filter(|&&file| providers.insert(file))
            .map(|&file| file_token_facts(db, file)),
    )
}

pub(super) fn merge_project_classes(
    facts: impl IntoIterator<Item = Arc<FileTokenFacts>>,
) -> Arc<ClassHierarchy> {
    // `project.files(db)` is an unordered `Vec` with no stable identity, so a
    // "first definition wins" merge would make the winner for a duplicate
    // qualified class name depend on file-enumeration order — non-deterministic
    // cross-file resolution (and token output).  Instead, abstain: a qualified
    // name defined in two or more files is genuinely ambiguous (which `::Foo`
    // does `$obj method` mean?), so drop it from the merged index and fall back
    // to no cross-file resolution — order-independent and sound, matching this
    // feature's highlight-only / sound-by-abstention posture.
    let mut merged: HashMap<String, ClassDef> = HashMap::new();
    let mut ambiguous: HashSet<String> = HashSet::new();
    for facts in facts {
        for (name, class) in &facts.classes {
            if ambiguous.contains(name) {
                continue;
            }
            match merged.entry(name.clone()) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    // A second file defines the same qualified name — ambiguous.
                    e.remove();
                    ambiguous.insert(name.clone());
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(class.clone());
                }
            }
        }
    }
    Arc::new(build_class_hierarchy(merged))
}

/// The project's workspace-merged inferred variable-name argument roles: every
/// file's user-proc parameter roles — a parameter the analyser inferred to
/// alias a caller variable (`upvar $param` + write) — unioned into one
/// cross-file index, so a `myproc arr(key) …` call highlights its array-element
/// target even when `myproc` is defined in another file.
///
/// A proc name defined with *conflicting* roles across files is dropped as
/// ambiguous by [`VarNameArgRoles::merge`], so the merged index is
/// order-independent — matching the abstention posture of
/// [`project_class_index`].
///
/// Reads the same light [`file_token_facts`] firewall as [`project_class_index`].
#[salsa::tracked(returns(clone))]
pub fn project_proc_var_index(db: &dyn TclDb, project: Project) -> Arc<VarNameArgRoles> {
    let mut providers = HashSet::new();
    merge_project_proc_roles(
        project
            .files(db)
            .iter()
            .filter(|&&file| providers.insert(file))
            .map(|&file| file_token_facts(db, file)),
    )
}

fn merge_project_proc_roles(
    facts: impl IntoIterator<Item = Arc<FileTokenFacts>>,
) -> Arc<VarNameArgRoles> {
    let per_file = facts.into_iter().collect::<Vec<_>>();
    Arc::new(VarNameArgRoles::merge(
        per_file.iter().map(|facts| &facts.proc_roles),
    ))
}

/// Workspace named-instance display reports, preserving each row's existing
/// compatibility or conditional source-display mode. Conditional labels never
/// enter legacy classification; actual project receiver selection uses the
/// independently retained original `FileTokenFacts::class_cards`.
///
/// Conflicting class labels are omitted from this report in either file order.
/// Agreement between labels supplies no command or receiver identity.
///
/// Reads the same light [`file_token_facts`] firewall as [`project_class_index`].
#[salsa::tracked(returns(clone))]
pub fn project_named_instance_index(
    db: &dyn TclDb,
    project: Project,
) -> Arc<tcl_lsp_core::semantic_tokens::NamedInstanceMap> {
    let mut providers = HashSet::new();
    merge_project_named_instances(
        project
            .files(db)
            .iter()
            .filter(|&&file| providers.insert(file))
            .map(|&file| file_token_facts(db, file)),
    )
}

pub(super) fn merge_project_named_instances(
    facts: impl IntoIterator<Item = Arc<FileTokenFacts>>,
) -> Arc<tcl_lsp_core::semantic_tokens::NamedInstanceMap> {
    let mut merged = tcl_lsp_core::semantic_tokens::NamedInstanceMap::default();
    for facts in facts {
        merged.merge_reports(&facts.named_instances);
    }
    Arc::new(merged)
}

/// [`semantic_tokens`] resolved against the **workspace-merged** class index, so
/// a `$obj method …` dispatch on a class defined in another project file
/// resolves too.  The server calls this when a [`Project`] is available; the
/// bare [`semantic_tokens`] (local file only) is the fallback.
///
/// This compatibility query keeps standalone cross-file indexes. Actual
/// workspace document callers use [`semantic_tokens_project_for_inputs`] and
/// the driver's per-file configuration map.
// `returns(clone)` for the same reason as [`semantic_tokens`].
#[salsa::tracked(returns(clone))]
pub fn semantic_tokens_project(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
    config: AnalyserConfig,
) -> SemanticTokens {
    let registry = token_registry(db, file.dialect(db), config.spec_pack_key(db));
    let cu = document_compilation_unit_for(db, file, config);
    let classes = project_class_index(db, project);
    let mut seen = HashSet::new();
    let class_cards = project
        .files(db)
        .iter()
        .filter(|&&member| member != file && seen.insert(member))
        .map(|&file| file_token_facts(db, file).class_cards.clone())
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();
    let proc_roles = project_proc_var_index(db, project);
    let named_instances = project_named_instance_index(db, project);
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::semantic_tokens::full_with_cu_and_facts(
        file.text(db),
        tcl_lsp_core::profile_for_dialect(file.dialect(db)),
        &registry,
        cu.as_deref(),
        tcl_lsp_core::semantic_tokens::WorkspaceTokenFacts {
            classes: Some(&classes),
            project_classes: Some(&class_cards),
            proc_roles: Some(&proc_roles),
            named_instances: Some(&named_instances),
            analysis: Some(&analysis),
        },
    )
}

/// Folding ranges — wraps `folding::folding_ranges`; reads the durable registry.
// `returns(clone)` for the same liveness reason as [`document_symbols`]: this
// query has no interior cancellation checkpoint at all (it is a single
// straight-line segmenter walk), so the read region must stay as short as
// possible and the projection must happen outside it.
#[salsa::tracked(returns(clone))]
pub fn folding_ranges(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Vec<FoldingRange> {
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::folding::folding_ranges_with_analysis(file.text(db), &analysis)
}
