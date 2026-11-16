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

//! Cross-document edits for a renamed procedure or class.

use super::{
    AnalysisResult, Backend, DocumentSourceResult, FromStr, Position, Range, RenameContext,
    TextEdit, Uri, WorkspaceRenameCaller, WorkspaceSourcePlanToken, core_rename,
};

impl Backend {
    /// Add cross-document rename edits for the proc / class at
    /// `pos` into `changes`.  Resolves the symbol, asks the core
    /// rename provider for the namespace-aware sibling-document
    /// edit intents (call sites + definition sites), converts
    /// each byte span to a range against its target document,
    /// and merges into the per-URI edit map (deduped).
    /// Returns `true` when the rename must be **aborted wholesale**: a
    /// sibling document holds an indirect dispatch of this symbol whose
    /// contributing constants are not all source-writable — no edit set can
    /// keep that dispatch alive, so not even
    /// the in-document edits may apply.
    ///
    /// A multi-seeded declaration is an explicit
    /// **multi-symbol rename**: the one physical token names every
    /// source-site identity, so the edit set is the union over all of
    /// them — each view's callers rewritten — keeping every runtime
    /// identity consistent with the edited declaration.
    pub(in super::super) async fn add_cross_document_rename_edits(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
        changes: &mut std::collections::HashMap<Uri, Vec<TextEdit>>,
    ) -> DocumentSourceResult<bool> {
        let (rehoming_guard, symbols) = self
            .resolved_symbol_index_snapshot_for_binding(uri, source, analysis, pos)
            .await?;
        let intents = {
            let index = self.workspace_index.read().await;
            if symbols.iter().any(|q| index.rename_blocked(q)) {
                return Ok(true);
            }
            let mut intents: Vec<core_rename::WorkspaceTextEdit> = Vec::new();
            for qualified in &symbols {
                intents.extend(core_rename::cross_document_symbol_edits(
                    qualified,
                    new_name,
                    &index,
                    uri.as_str(),
                ));
            }
            intents
        };
        drop(rehoming_guard);
        self.merge_rename_intents(intents, changes).await?;
        Ok(false)
    }

    /// Consumer-document rename: the cursor's command has no local
    /// definition, so the in-document rename had nothing to resolve against —
    /// not a *rejection* when the workspace (or a library file the autoload
    /// tier merges on demand) defines the symbol.  Resolve through the
    /// workspace oracle and build the whole edit set from the index, the
    /// current document's own call sites included.  Adds nothing when the new
    /// name would shadow an existing workspace command (the same collision
    /// discipline as the in-document rename).
    pub(in super::super) async fn add_workspace_resolved_rename_edits(
        &self,
        caller: WorkspaceRenameCaller<'_>,
        new_name: &str,
        changes: &mut std::collections::HashMap<Uri, Vec<TextEdit>>,
        source_plan: &mut WorkspaceSourcePlanToken,
    ) -> DocumentSourceResult<()> {
        let WorkspaceRenameCaller {
            uri,
            source,
            analysis,
            position: pos,
        } = caller;
        let (rehoming_guard, symbols) = self
            .resolved_symbol_index_snapshot_for_binding(uri, source, analysis, pos)
            .await?;
        if symbols.is_empty() {
            return Ok(());
        }
        // The open document's own call sites are edited from the index here (its
        // in-document rename found nothing local to resolve against), and only
        // its *focused* analysis carries the resolution candidates a
        // cross-document match needs.  The background workspace scan can leave
        // this file indexed without that focused walk, dropping the consumer's
        // own edit while the library declaration (matched by name) still lands
        // — a partial rename.  Commit the live focused analysis into the index
        // before reading it, so the current document's edits are deterministic
        // rather than dependent on scan timing.
        {
            let mut index = self.workspace_index.write().await;
            // This focused consumer replacement is an intentional input to
            // the workspace-resolved plan. Admit it only if no unrelated index
            // or receipt mutation occurred since planning began, then bind the
            // plan to the resulting exact index generation.
            if index.generation() != source_plan.index_generation
                || index.source_inventory_revision() != source_plan.inventory_revision
            {
                return Ok(());
            }
            index.replace_document(uri.as_str(), analysis);
            source_plan.index_generation = index.generation();
            source_plan.inventory_revision = index.source_inventory_revision();
        }
        let intents = {
            let index = self.workspace_index.read().await;
            // Every identity of a multi-seeded declaration must accept the
            // rename (no collision, no unwritable indirect dispatch) — a
            // partial multi-symbol edit would leave the views inconsistent,
            // so one refusal aborts them all.
            let mut intents: Vec<core_rename::WorkspaceTextEdit> = Vec::new();
            for qualified in &symbols {
                let Some(symbol_intents) =
                    core_rename::workspace_symbol_rename_edits(qualified, new_name, &index)
                else {
                    return Ok(());
                };
                intents.extend(symbol_intents);
            }
            intents
        };
        drop(rehoming_guard);
        self.merge_rename_intents(intents, changes).await?;

        Ok(())
    }

    /// Extend `changes` with cross-document rename edits — or resolve
    /// through the workspace oracle when the in-document rename found
    /// nothing local to resolve against (the consumer-document shape).
    /// Extracted from [`LanguageServer::rename`] to keep it within the line
    /// budget.  Returns `true` when the caller must abort the whole rename
    /// (`Ok(None)`): a sibling document dispatches this symbol through a
    /// value whose provenance is not fully writable.
    ///
    /// Gated on the same safety checks as the in-document path
    /// (`is_safe_symbol_name`, no built-in shadow) so a cross-doc rename
    /// can't produce an unsafe edit set — and skipped entirely when the
    /// in-document rename was rejected (collision, unrenameable cursor;
    /// adding cross-document edits for a locally-rejected rename would leak
    /// a partial, inconsistent edit set into sibling documents).
    pub(super) async fn extend_rename_with_cross_document_edits(
        &self,
        ctx: RenameContext<'_>,
        local_rejected: bool,
        changes: &mut std::collections::HashMap<Uri, Vec<TextEdit>>,
        source_plan: &mut WorkspaceSourcePlanToken,
    ) -> DocumentSourceResult<bool> {
        let new_name_safe = core_rename::is_safe_symbol_name(ctx.new_name)
            && !core_rename::is_builtin_command_name(ctx.new_name, ctx.registry);
        if !new_name_safe {
            return Ok(false);
        }
        if !local_rejected {
            return self
                .add_cross_document_rename_edits(
                    ctx.uri,
                    ctx.source,
                    ctx.analysis,
                    ctx.pos,
                    ctx.new_name,
                    changes,
                )
                .await;
        }
        self.add_workspace_resolved_rename_edits(
            WorkspaceRenameCaller {
                uri: ctx.uri,
                source: ctx.source,
                analysis: ctx.analysis,
                position: ctx.pos,
            },
            ctx.new_name,
            changes,
            source_plan,
        )
        .await?;
        Ok(false)
    }

    /// Resolve each byte-span edit intent to an LSP range against its target
    /// document's source (open buffer or on-disk fallback) and merge it into
    /// the per-URI edit map, deduping identical ranges.
    async fn merge_rename_intents(
        &self,
        intents: Vec<core_rename::WorkspaceTextEdit>,
        changes: &mut std::collections::HashMap<Uri, Vec<TextEdit>>,
    ) -> DocumentSourceResult<()> {
        let _: () = for intent in intents {
            let Ok(parsed) = Uri::from_str(&intent.uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await? else {
                continue;
            };
            let line_index = target_doc.line_index.clone();
            let start = line_index.position_at_utf16(intent.span.start(), &target_doc.text);
            let end = line_index.position_at_utf16(intent.span.end(), &target_doc.text);
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character.get(),
                    },
                    end: Position {
                        line: end.line,
                        character: end.character.get(),
                    },
                },
                new_text: intent.new_text,
            };
            let bucket = changes.entry(parsed).or_default();
            if !bucket.iter().any(|e| e.range == edit.range) {
                bucket.push(edit);
            }
        };
        Ok(())
    }
}
