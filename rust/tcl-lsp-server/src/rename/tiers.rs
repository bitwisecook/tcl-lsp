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

//! The order rename tiers are tried in, against one workspace source plan.

use super::{RenameContext, WorkspaceRenameCaller, WorkspaceSourcePlanToken, rename_refusal_error};
use crate::uri_norm::DocumentPath as _;
use crate::{
    AnalysisResult, Arc, Backend, CommandRegistry, ControlFlow, DocumentSourceResult,
    DocumentState, Position, RenameParams, TextEdit, Uri, WorkspaceEdit, core_definition,
    core_rename, core_rename_safety, jsonrpc, lift_lsp_range, vfs,
};

impl Backend {
    /// The two rename tiers that answer *before* the ordinary in-document /
    /// cross-document path, or `None` when neither applies.
    ///
    /// `Some(Err(..))` is a **refusal** carrying its own reason; `Some(Ok(..))`
    /// is a complete namespace-variable edit set.  Split out of
    /// [`Backend::rename`] so that handler stays inside the line budget.
    /// The in-document rename tiers, run on the blocking pool with the
    /// request's whole-program export view attached.
    ///
    /// `rename_in_program`, not `rename`: the in-document tiers carry their
    /// own safety gate for the cursor positions the workspace gate cannot
    /// resolve — an untracked `[$other X]` dispatch, an `export … X …`
    /// bareword — and a refusal from them must reach the editor as an error
    /// with its reason, exactly like the workspace gate's.  Flattening it to
    /// an empty edit set would present the one hazard the gate exists to stop
    /// as "nothing renameable here" and let the cross-document tier build
    /// edits for it.
    ///
    /// The export snapshot is what lets a rename started from a
    /// `-force`-shadowed call retarget to the definition that call actually
    /// reaches.
    ///
    /// Extracted from [`Self::rename`] to keep that entry point inside the
    /// line budget.
    async fn in_document_rename_edits(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        registry: &Arc<tcl_registry::CommandRegistry>,
        pos: Position,
        new_name: &str,
    ) -> jsonrpc::Result<Vec<core_rename::TextEdit>> {
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let analysis_for_worker = analysis.clone();
        let new_name_worker = new_name.to_owned();
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        // The worker owns a clone of the handle: it outlives this frame, and
        // the generation it names must outlive the worker.
        let registry_worker = Arc::clone(registry);
        crate::rt::spawn_blocking(move || {
            core_rename::rename_in_program(
                &text,
                tcl_lsp_core::profile_for_dialect(&dialect),
                pos.line,
                pos.character,
                &new_name_worker,
                &analysis_for_worker,
                core_definition::CallResolution {
                    registry: Some(&registry_worker),
                    program: Some(core_definition::ProgramExports {
                        uri: &uri_key,
                        oracle: exports.as_ref(),
                    }),
                },
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("rename worker panicked: {err}").into(),
            data: None,
        })?
        .map_err(|refusal| rename_refusal_error(&refusal))
    }

    pub(in super::super) async fn gated_rename_tiers(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
    ) -> DocumentSourceResult<Option<jsonrpc::Result<Option<WorkspaceEdit>>>> {
        // Namespace tier: the cursor names a namespace, so
        // rename it across the whole workspace or refuse with the reason.
        // Either answer is returned from here and never falls through: an
        // empty edit set would reach the ordinary and workspace-resolved
        // rename tiers, which resolve the cursor **word** as a command — so
        // `namespace children widget` beside a `proc widget` would rename the
        // proc instead.
        if Self::original_namespace_symbol(&doc.text, analysis, pos).is_some()
            || Self::namespace_cell(&doc.text, analysis, pos).is_some()
        {
            return Ok(Some(
                match self
                    .cross_document_namespace_rename(uri, doc, analysis, pos, new_name)
                    .await
                {
                    Err(refusal) => Err(rename_refusal_error(&refusal)),
                    Ok(changes) if changes.is_empty() => Ok(None),
                    Ok(changes) => Ok(Some(WorkspaceEdit {
                        changes: Some(changes),
                        document_changes: None,
                        change_annotations: None,
                    })),
                },
            ));
        }
        if Self::original_namespace_cursor_retained(&doc.text, analysis, pos) {
            return Ok(Some(Err(rename_refusal_error(
                &core_rename_safety::RenameRefusal {
                    reason: "cannot prove the original namespace selected by this occurrence"
                        .into(),
                    range: None,
                },
            ))));
        }
        if let std::ops::ControlFlow::Break(selected) =
            tcl_lsp_core::variable_symbol::select(&doc.text, analysis, pos.line, pos.character)
        {
            let Some(occurrence) = selected else {
                return Ok(Some(Err(rename_refusal_error(
                    &core_rename_safety::RenameRefusal {
                        reason: "cannot prove the original variable selected by this occurrence"
                            .into(),
                        range: None,
                    },
                ))));
            };
            return Ok(Some(
                match self
                    .cross_document_original_variable_rename(
                        uri,
                        doc,
                        analysis,
                        occurrence.symbol(),
                        new_name,
                    )
                    .await
                {
                    Err(refusal) => Err(rename_refusal_error(&refusal)),
                    Ok(changes) if changes.is_empty() => Ok(None),
                    Ok(changes) => Ok(Some(WorkspaceEdit {
                        changes: Some(changes),
                        document_changes: None,
                        change_annotations: None,
                    })),
                },
            ));
        }
        // Safety gate: when the cursor names a `TclOO` member whose dispatch
        // this rename cannot account for anywhere in the workspace, refuse
        // with a precise reason instead of emitting a partial edit set that
        // breaks the program once applied.
        if analysis.allows_lexical_declaration_advice()
            && core_rename::is_safe_symbol_name(new_name)
            && let Some(refusal) = self
                .method_rename_refusal(uri, doc, analysis, pos, new_name)
                .await?
        {
            return Ok(Some(Err(rename_refusal_error(&refusal))));
        }
        // Original variable selection above is terminal. A readonly Logical
        // coordinate without an original cursor symbol cannot issue edits in
        // sibling documents; the local fallback retains its own source gates.
        Ok(None)
    }

    /// Rebuild the indexed view before an absence-sensitive rename when the
    /// browser host has finished a newer complete census but this index still
    /// carries an older receipt. The scan is the only owner allowed to bind a
    /// new receipt to indexed bytes; a host marker alone never upgrades the
    /// previous index. A known incomplete server scan is left refused rather
    /// than retried on every rename.
    async fn refresh_stale_complete_host_inventory_for_rename(&self) -> DocumentSourceResult<()> {
        let provider = self.store.workspace_source_inventory_provider();
        if provider != vfs::WorkspaceSourceInventoryProvider::HostReported {
            return Ok(());
        }
        let folders = self.workspace_folder_urls().await;
        let Some(roots) = folders
            .iter()
            .map(|folder| folder.document_path().map(std::borrow::Cow::into_owned))
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(());
        };
        if roots.is_empty() {
            return Ok(());
        }
        let observations = roots
            .iter()
            .map(|root| self.store.workspace_source_inventory(root))
            .collect::<Vec<_>>();
        if observations.is_empty()
            || !observations
                .iter()
                .all(vfs::WorkspaceSourceInventoryObservation::admits_complete_inventory)
        {
            return Ok(());
        }
        let index = self.workspace_index.read().await;
        if index.source_inventory_scan_complete() == Some(false)
            && index.source_inventory_observations_match(provider, &observations)
        {
            return Ok(());
        }
        let already_current = index.source_inventory_is_current(provider, &observations)
            && index.source_inventory_matches_store(self.store.as_ref());
        drop(index);
        if !already_current {
            self.scan_workspace_folders().await?;
        }
        Ok(())
    }

    pub(super) async fn capture_workspace_source_plan(&self) -> WorkspaceSourcePlanToken {
        let index = self.workspace_index.read().await;
        WorkspaceSourcePlanToken {
            index_generation: index.generation(),
            inventory_revision: index.source_inventory_revision(),
        }
    }

    /// Validate the exact index/receipt generations used by the plan, then
    /// confirm that receipt still matches the live provider and active roots.
    pub(in super::super) async fn workspace_source_plan_is_current(
        &self,
        token: WorkspaceSourcePlanToken,
    ) -> bool {
        let provider = self.store.workspace_source_inventory_provider();
        // Hold the active-root owner through the index/store comparison so a
        // folder change cannot occur between reading the root set and
        // validating the exact index snapshot.
        let folders = self.workspace_folders.lock().await;
        let Some(mut current_roots) = folders
            .iter()
            .map(|folder| folder.document_path().map(std::borrow::Cow::into_owned))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        current_roots.sort();
        current_roots.dedup();
        let index = self.workspace_index.read().await;
        let mut indexed_roots = index.source_inventory_roots();
        indexed_roots.sort();
        indexed_roots.dedup();
        index.generation() == token.index_generation
            && index.source_inventory_revision() == token.inventory_revision
            && (provider == vfs::WorkspaceSourceInventoryProvider::Filesystem
                || current_roots == indexed_roots)
            && index.source_inventory_matches_store(self.store.as_ref())
    }
}

impl Backend {
    /// One whole rename against one workspace snapshot. `plan` receives the
    /// snapshot token once it is captured.
    pub(in super::super) async fn rename_attempt(
        &self,
        params: RenameParams,
        plan: &mut Option<WorkspaceSourcePlanToken>,
    ) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        let conditional_uri = params.text_document_position.text_document.uri.clone();
        let conditional_uri_generation = self.live_publication_uri_generation(&conditional_uri);
        if !self
            .feature_enabled("rename", &params.text_document_position.text_document.uri)
            .await
        {
            return Ok(None);
        }
        self.refresh_stale_complete_host_inventory_for_rename()
            .await?;
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        // Per-dialect cached registry for safety gating — proc renames
        // refuse to overwrite built-in command names.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !self
            .original_procedure_expression_coverage_is_editable(&uri, &doc.text, &analysis, pos)
            .await?
        {
            return Ok(None);
        }
        // Reconcile source-site views before capturing the workspace snapshot
        // used by rename planning. Any later rehoming/index mutation makes the
        // exact token fail at publication.
        self.refresh_source_rehoming().await?;
        let source_plan = self.capture_workspace_source_plan().await;
        *plan = Some(source_plan);
        if let ControlFlow::Break(result) = self
            .conditional_variable_rename_tier(
                WorkspaceRenameCaller {
                    uri: &uri,
                    source: &doc.text,
                    analysis: &analysis,
                    position: pos,
                },
                &doc,
                &new_name,
                source_plan,
                conditional_uri_generation,
            )
            .await?
        {
            return Ok(result);
        }
        // The gated tiers, before any ordinary edit is built: the `TclOO`
        // member safety gate and the workspace namespace-variable rename.
        if let std::ops::ControlFlow::Break(result) = self
            .guarded_rename_source_tiers(&uri, &doc, &analysis, pos, &new_name, source_plan)
            .await?
        {
            return result;
        }
        // Method rename → cross-file override-family path.
        if let Some(changes) = self
            .method_family_rename_tier(&uri, &doc, &analysis, pos, &new_name)
            .await?
        {
            if !self.workspace_source_plan_is_current(source_plan).await {
                return Ok(None);
            }
            return Ok(Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            }));
        }
        self.ordinary_rename_edit(
            &doc,
            &registry,
            RenameContext {
                uri: &uri,
                source: &doc.text,
                analysis: &analysis,
                pos,
                new_name: &new_name,
                registry: &registry,
            },
            source_plan,
        )
        .await
    }
}

impl Backend {
    fn lift_local_rename_edits(edits: Vec<core_rename::TextEdit>) -> Vec<TextEdit> {
        edits
            .into_iter()
            .map(|e| TextEdit {
                range: lift_lsp_range(e.range),
                new_text: e.new_text,
            })
            .collect()
    }

    async fn guarded_rename_source_tiers(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
        source_plan: WorkspaceSourcePlanToken,
    ) -> DocumentSourceResult<std::ops::ControlFlow<jsonrpc::Result<Option<WorkspaceEdit>>>> {
        if let Some(gated) = self
            .gated_rename_tiers(uri, doc, analysis, pos, new_name)
            .await?
        {
            if matches!(&gated, Ok(Some(_)))
                && !self.workspace_source_plan_is_current(source_plan).await
            {
                return Ok(std::ops::ControlFlow::Break(Ok(None)));
            }
            return Ok(std::ops::ControlFlow::Break(gated));
        }
        if let std::ops::ControlFlow::Break(result) = self
            .original_command_rename_tier(uri, doc, analysis, pos, new_name)
            .await?
        {
            if matches!(&result, Ok(Some(_)))
                && !self.workspace_source_plan_is_current(source_plan).await
            {
                return Ok(std::ops::ControlFlow::Break(Ok(None)));
            }
            return Ok(std::ops::ControlFlow::Break(result));
        }
        Ok(std::ops::ControlFlow::Continue(()))
    }
}

impl Backend {
    async fn ordinary_rename_edit(
        &self,
        doc: &DocumentState,
        registry: &Arc<CommandRegistry>,
        ctx: RenameContext<'_>,
        mut source_plan: WorkspaceSourcePlanToken,
    ) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        let RenameContext {
            uri,
            analysis,
            pos,
            new_name,
            ..
        } = ctx;
        let edits = self
            .in_document_rename_edits(uri, doc, analysis, registry, pos, new_name)
            .await?;
        let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
            std::collections::HashMap::new();
        // An empty in-document result means the rename was *rejected*
        // (collision with an existing symbol, an unsafe / built-in target
        // name) or the cursor isn't on a renameable symbol.  In every one
        // of those cases the rename must abort wholesale — adding
        // cross-document edits for a locally-rejected rename would leak a
        // partial, inconsistent edit set into sibling documents.
        let local_rejected = edits.is_empty();
        if !edits.is_empty() {
            let lifted = Self::lift_local_rename_edits(edits);
            changes.insert(uri.clone(), lifted);
        }
        // Cross-document rename: rewrite the symbol's call / definition
        // sites in sibling documents (or resolve through the workspace
        // oracle when the in-document rename found nothing local to
        // resolve against).  Aborts the whole rename when a sibling's
        // provenance isn't fully writable.
        // The consumer-document fallback can trigger an on-demand library
        // merge. Complete that selected lookup before binding the index token;
        // the actual workspace-resolved edit plan below must use this exact
        // snapshot. A subsequent index change is refused by the guarded
        // focused replacement or final publication check.
        if local_rejected
            && core_rename::is_safe_symbol_name(new_name)
            && !core_rename::is_builtin_command_name(new_name, registry)
        {
            let _ = self
                .resolve_workspace_symbols_for_binding(uri, &doc.text, analysis, pos)
                .await?;
            source_plan = self.capture_workspace_source_plan().await;
        }
        if self
            .extend_rename_with_cross_document_edits(
                ctx,
                local_rejected,
                &mut changes,
                &mut source_plan,
            )
            .await?
        {
            return Ok(None);
        }
        if changes.is_empty() {
            return Ok(None);
        }
        if !self.workspace_source_plan_is_current(source_plan).await {
            return Ok(None);
        }
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }))
    }
}
