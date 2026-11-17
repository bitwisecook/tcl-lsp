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

//! Renames of namespaces and namespace variables across the workspace.

use super::{
    AnalysisResult, DocumentState, FromStr, Position, TextEdit, Uri, core_namespace_rename,
    core_namespace_symbol, core_rename, core_rename_safety, lift_lsp_range,
};
use crate::Backend;

/// The collision gate's refusal for renaming `cell` onto `new_cell`, or `None`
/// when nothing declares the target.
///
/// Split out to say *where* the collision is. "`::b` is already declared in
/// this workspace" is a claim the user cannot check: the workspace the gate
/// reads spans every scanned folder, not the files they have in mind, so a
/// refusal against a project whose only visible Tcl file plainly declares no
/// `b` leaves the user with nowhere to look. Naming the documents turns the
/// refusal into something falsifiable in one glance.
///
/// At most three are listed, with a count for the rest: the reason renders as a
/// single unwrapped line in some hosts, so the point
/// is to be checkable, not exhaustive — one name is usually the whole answer,
/// and the gate refuses on the first one regardless.
pub(in super::super) fn describe_variable_collision(
    cell: &str,
    new_cell: &str,
    colliding: &[String],
) -> Option<String> {
    const SHOWN: usize = 3;
    if colliding.is_empty() {
        return None;
    }
    let listed = colliding
        .iter()
        .take(SHOWN)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let rest = colliding.len().saturating_sub(SHOWN);
    let where_ = if rest == 0 {
        listed
    } else {
        format!("{listed} and {rest} more")
    };
    Some(format!(
        "cannot rename `{cell}`: `{new_cell}` is already declared in {where_}, \
         so renaming would merge two distinct namespace variables into one cell."
    ))
}

impl Backend {
    /// Rename the **namespace** the cursor names, across the whole workspace.
    ///
    /// A namespace is spelled in every file that reaches into it — a
    /// `namespace eval` block that reopens it, a `$::ns::v`, a `::ns::p` call,
    /// an import pattern — so a rename that stopped at one document would be
    /// exactly the partial edit set the safety bar forbids.  Each document's
    /// share is [`core_namespace_rename::namespace_rename_edits`], the same
    /// function the in-document tier uses, so the two cannot disagree about
    /// which occurrences exist.
    ///
    /// # Why every indexed document is scanned
    ///
    /// The gate refuses on an occurrence the analyser cannot attribute — a
    /// rooted name beneath the namespace sitting in a data word.  Such a word
    /// puts *nothing* in any index table, so no index query can find the
    /// document holding it; a candidate set derived from the index would scan
    /// only documents that already name the namespace in an attributed
    /// position, and the one file with the embedded callback string would be
    /// neither scanned nor rewritten.  Coverage therefore has to be the whole
    /// workspace, which is affordable because a namespace rename is a rare,
    /// deliberate refactor rather than a per-keystroke query.
    ///
    /// `Err` is a refusal: the target namespace already exists, or some
    /// document holds an occurrence this rename can neither rewrite nor rule
    /// out.
    pub(in super::super) async fn cross_document_namespace_rename(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
    ) -> Result<std::collections::HashMap<Uri, Vec<TextEdit>>, core_rename_safety::RenameRefusal>
    {
        if let Some(symbol) = Self::original_namespace_symbol(&doc.text, analysis, pos) {
            return self
                .cross_document_original_namespace_rename(uri, doc, analysis, &symbol, new_name)
                .await;
        }
        let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
            std::collections::HashMap::new();
        let Some(cell) = Self::namespace_cell(&doc.text, analysis, pos) else {
            return Ok(changes);
        };
        if !core_rename::is_safe_symbol_name(new_name) {
            return Ok(changes);
        }
        // Only the final segment moves, so the new namespace is the old one
        // with its tail replaced.
        // Lexical constructed metadata does not reconstruct a Native path.
        // Preserve its analytical holder, including literal colon segments.
        let (holder, _) = tcl_syntax::naming::key_holder_and_tail(&cell);
        let new_cell = tcl_syntax::naming::qualify(holder, new_name);
        let candidates: Vec<String> = {
            let _rehoming_guard = self.rehomed_index_guard().await?;
            let index = self.workspace_index.read().await;
            // Collision gate: renaming onto a namespace that already exists
            // merges two namespaces into one, silently moving every name in
            // this one into a namespace that already has its own contents.
            let occupied = !index
                .namespace_declarations_qualified(&new_cell, "")
                .is_empty()
                || !core_namespace_symbol::namespace_declaration_spans(analysis, &new_cell)
                    .is_empty();
            if occupied {
                return Err(core_rename_safety::RenameRefusal {
                    reason: format!(
                        "cannot rename `{cell}`: `{new_cell}` already exists in this \
                         workspace, and renaming would merge two distinct namespaces \
                         into one."
                    ),
                    range: None,
                });
            }
            let mut c = index.document_uris();
            c.push(uri.as_str().to_owned());
            c.sort();
            c.dedup();
            c
        };
        for u in candidates {
            let Ok(parsed) = Uri::from_str(&u) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await? else {
                continue;
            };
            let target_analysis = self
                .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                .await?;
            let edits = core_namespace_rename::namespace_rename_edits(
                &target_doc.text,
                tcl_lsp_core::profile_for_dialect(&target_doc.dialect),
                &target_analysis,
                &cell,
                new_name,
            )?;
            if edits.is_empty() {
                continue;
            }
            let bucket = changes.entry(parsed).or_default();
            for e in edits {
                let range = lift_lsp_range(e.range);
                if !bucket.iter().any(|x| x.range == range) {
                    bucket.push(TextEdit {
                        range,
                        new_text: e.new_text,
                    });
                }
            }
        }
        Ok(changes)
    }

    /// Every document uses the same original namespace edit planner. The
    /// proposed address is byte geometry; it grants no namespace existence or
    /// source completion. Incomplete document coverage refuses the edit set.
    pub(in super::super) async fn cross_document_original_namespace_rename(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        symbol: &core_namespace_symbol::OriginalNamespaceSymbol,
        new_name: &str,
    ) -> Result<std::collections::HashMap<Uri, Vec<TextEdit>>, core_rename_safety::RenameRefusal>
    {
        let proposed = analysis
            .body_lexer_config
            .and_then(|config| symbol.renamed(new_name, tcl_lexer::SourceChannel::Document, config))
            .ok_or_else(|| core_rename_safety::RenameRefusal {
                reason: "cannot prove the proposed original namespace spelling".into(),
                range: None,
            })?;
        let candidates = {
            let _rehoming_guard = self.rehomed_index_guard().await?;
            let index = self.workspace_index.read().await;
            let local_facts = core_namespace_symbol::original_namespace_facts(analysis, &proposed);
            let occupied = index
                .original_namespace_occurrences(&proposed, "")
                .any(|reference| reference.declares)
                || index
                    .original_namespace_descendant_declarations(&proposed, "")
                    .next()
                    .is_some()
                || local_facts.declarations > 0
                || local_facts.implicit_declarations > 0;
            if proposed != *symbol && occupied {
                return Err(core_rename_safety::RenameRefusal {
                    reason: "cannot rename onto another namespace: the target namespace already \
                             exists in this workspace"
                        .into(),
                    range: None,
                });
            }
            let mut candidates = index.document_uris();
            candidates.push(uri.as_str().to_owned());
            candidates.sort();
            candidates.dedup();
            candidates
        };
        let mut changes = std::collections::HashMap::new();
        for owner in candidates {
            let parsed = Uri::from_str(&owner).map_err(|_| core_rename_safety::RenameRefusal {
                reason: "cannot inspect every indexed namespace consumer".into(),
                range: None,
            })?;
            let (source, dialect, target_analysis) = if parsed == *uri {
                (doc.text.clone(), doc.dialect.clone(), None)
            } else {
                let target_doc = self.read_document(&parsed).await?.ok_or_else(|| {
                    core_rename_safety::RenameRefusal {
                        reason: format!("cannot inspect indexed namespace consumer `{owner}`"),
                        range: None,
                    }
                })?;
                let target_analysis = self
                    .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                    .await?;
                (target_doc.text, target_doc.dialect, Some(target_analysis))
            };
            let target_analysis = target_analysis.as_deref().unwrap_or(analysis);
            let edits = core_namespace_rename::original_namespace_rename_edits(
                &source,
                tcl_lsp_core::profile_for_dialect(&dialect),
                target_analysis,
                symbol,
                new_name,
            )?;
            if !edits.is_empty() {
                changes.insert(
                    parsed,
                    edits
                        .into_iter()
                        .map(|edit| TextEdit {
                            range: lift_lsp_range(edit.range),
                            new_text: edit.new_text,
                        })
                        .collect(),
                );
            }
        }
        Ok(changes)
    }

    /// Plan byte-identity variable edits through the shared Core owner. Namespace
    /// symbols require coverage of every current indexed document; procedure
    /// symbols remain in their sealed source frame. Unknown coverage, missing
    /// sources and occupied proposed symbols refuse the complete edit set.
    pub(in super::super) async fn cross_document_original_variable_rename(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        symbol: &tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
        new_name: &str,
    ) -> Result<std::collections::HashMap<Uri, Vec<TextEdit>>, core_rename_safety::RenameRefusal>
    {
        let refuse = |reason: &str| core_rename_safety::RenameRefusal {
            reason: reason.into(),
            range: None,
        };
        let units = tcl_syntax::backslash::native_source_literal_bytes(
            new_name.as_bytes(),
            tcl_lexer::SourceChannel::Document,
            symbol.policy().string_protocol(),
        )
        .map_err(|_| refuse("the replacement variable has no selected source channel recipe"))?;
        let proposed = symbol
            .renamed(&units)
            .ok_or_else(|| refuse("the replacement must remain one unqualified variable root"))?;
        let candidates = if symbol.is_namespace() {
            let _rehoming_guard = self.rehomed_index_guard().await?;
            let index = self.workspace_index.read().await;
            if proposed != *symbol {
                let occupied = index
                    .original_variable_occurrences(&proposed, "")
                    .collect::<Vec<_>>();
                if !occupied.is_empty() {
                    let mut declarations = occupied
                        .iter()
                        .filter(|(_, occurrence)| occurrence.is_declaration())
                        .map(|(owner, _)| (*owner).to_owned())
                        .collect::<Vec<_>>();
                    declarations.sort();
                    declarations.dedup();
                    let reason = symbol
                        .checked_c_namespace_key()
                        .zip(proposed.checked_c_namespace_key())
                        .and_then(|(selected, proposed)| {
                            describe_variable_collision(&selected, &proposed, &declarations)
                        })
                        .unwrap_or_else(|| {
                            "the replacement names another variable in this workspace".to_owned()
                        });
                    return Err(refuse(&reason));
                }
            }
            let mut candidates = index.document_uris();
            candidates.push(uri.as_str().to_owned());
            candidates.sort();
            candidates.dedup();
            candidates
        } else {
            vec![uri.as_str().to_owned()]
        };
        let mut changes = std::collections::HashMap::new();
        for owner in candidates {
            let parsed = Uri::from_str(&owner)
                .map_err(|_| refuse("cannot inspect every indexed variable consumer"))?;
            let (source, target_analysis) = if parsed == *uri {
                (doc.text.clone(), None)
            } else {
                let target_doc = self
                    .read_document(&parsed)
                    .await?
                    .ok_or_else(|| refuse("cannot inspect an indexed variable consumer"))?;
                let target_analysis = self
                    .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect)
                    .await?;
                (target_doc.text, Some(target_analysis))
            };
            let target_analysis = target_analysis.as_deref().unwrap_or(analysis);
            let edits = tcl_lsp_core::variable_symbol::original_variable_rename_edits(
                &source,
                target_analysis,
                symbol,
                new_name,
            )?;
            if !edits.is_empty() {
                changes.insert(
                    parsed,
                    edits
                        .into_iter()
                        .map(|edit| TextEdit {
                            range: lift_lsp_range(edit.range),
                            new_text: edit.new_text,
                        })
                        .collect(),
                );
            }
        }
        Ok(changes)
    }
}
