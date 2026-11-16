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

//! Writing a draft back into the document: `set_command`, `remove_command`, the splice attempts, what is carried forward and the re-render floor.

use super::PackStore;
use super::source_spans::{
    find_command, find_subcommand, is_native_placeholder, pack_body, reclaim, segments,
    trimmed_span, with_override_flags,
};
use crate::draft::Draft;
use crate::{draft, render_spectcl};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use tcl_lexer::SourceSegmentationPurpose;
use tcl_spectcl::loader;

// The pack store — the DSL text is the model

/// How a write-back reached the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteBack {
    /// One `command` statement's byte range was replaced; every other byte,
    /// comments and blank lines included, is the author's own.
    Spliced,
    /// The whole document was re-rendered from drafts — correct, and no
    /// comment or layout choice outside the rendered shape survives.
    Rerendered,
    /// The edited draft uses the current vocabulary, so an older pack header
    /// was raised before the otherwise-targeted command write.
    VocabularyUpgraded,
    /// The document is a **program** (E-R12), so it was not written to at
    /// all: the edit became a canonical patch pack in the
    /// [`Tier::StudioOverride`] tier, layered over the base by
    /// [`PackStore::pack_set`] and reported by
    /// [`PackStore::standing_overrides`].
    Patched,
}

impl WriteBack {
    /// The word the report uses.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Spliced => "spliced",
            Self::Rerendered => "rerendered",
            Self::VocabularyUpgraded => "vocabulary-upgraded",
            Self::Patched => "patched",
        }
    }
}

/// What a write-back did, and what it could not keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    /// How the new document was reached.
    pub how: WriteBack,
    /// Property words the declaration had before the edit and does not have
    /// after it, as `"arg_role_resolver"` or `"subcommand is / const_fold"`.
    ///
    /// **This is the honest half of the write-back.** A draft cannot recover
    /// every declaration — a hook body becomes a function pointer, and the
    /// draft records only "set, expression not recovered" — so re-rendering a
    /// command from its draft would silently delete the author's Tcl.
    /// [`PackStore::set_command`] carries such statements forward from the old
    /// text verbatim wherever it can (see `carry_forward`), and everything it
    /// still could not keep is named here rather than lost quietly.
    pub dropped: Vec<String>,
    /// Previous declared vocabulary when this edit had to raise the pack to
    /// the renderer's current vocabulary.
    pub upgraded_from: Option<String>,
}

impl PackStore {
    /// Write `draft` back as the definition of `name`.
    ///
    /// A command already in the document is replaced **where it stands**; one
    /// that is not is appended to the pack body. `name` is the declaration
    /// being written *over*, so renaming a command — the author editing the
    /// `name` field — is an ordinary edit: the new name lands in the old one's
    /// place rather than at the end of the file.
    ///
    /// A **programmed** document (E-R12) is never written to: the edit lands
    /// as a canonical patch pack in the `StudioOverride` tier instead, and
    /// the reply says [`WriteBack::Patched`].
    ///
    /// Returns how the document was reached — see [`WriteBack`], and this
    /// module's header for what "spliced" does and does not preserve.
    pub fn set_command(&mut self, name: &str, draft: &Draft, overrides: bool) -> Write {
        if self.programmed().is_some() {
            return self.patch_command(name, draft);
        }
        let upgraded_from = self
            .draft_requires_vocabulary_upgrade(draft)
            .then(|| self.upgrade_vocabulary_for_edit())
            .flatten();
        let existed = self.draft(name).is_some();
        let before = self.declared_properties(name);
        let block = self.carry_forward(name, &self.render_block(draft, overrides));

        let how = if let Some(text) = self.write_attempt(name, &block, existed) {
            let candidate = Self::from_source(text);
            if self.accepts(&candidate, name, &block, overrides, existed) {
                *self = candidate;
                Some(WriteBack::Spliced)
            } else {
                None
            }
        } else {
            None
        };
        let mut how = how.unwrap_or_else(|| {
            self.rerender_with(name, draft, overrides);
            WriteBack::Rerendered
        });
        if upgraded_from.is_some() && how == WriteBack::Spliced {
            how = WriteBack::VocabularyUpgraded;
        }

        let written = draft.get("name").and_then(Value::as_str).unwrap_or(name);
        let after = self.declared_properties(written);
        // An absent property or replacement native placeholder loses its declaration.
        let dropped = before
            .iter()
            .filter(|(word, was_placeholder)| {
                !**was_placeholder && after.get(*word).is_none_or(|now| *now)
            })
            .map(|(word, _)| word.clone())
            .collect();
        Write {
            how,
            dropped,
            upgraded_from,
        }
    }

    /// Drop `name` from the document.
    ///
    /// Returns `false` when the document does not declare it.
    ///
    /// On a **programmed** document the source is never touched (E-R12), so
    /// this can only take back a standing patch override —
    /// [`Self::remove_override`] — and answers `false` for a command the
    /// program itself declares.
    pub fn remove_command(&mut self, name: &str) -> bool {
        if self.programmed().is_some() {
            return self.remove_override(name);
        }
        if self.draft(name).is_none() {
            return false;
        }
        if let Some(span) = self
            .source_index()
            .command(name)
            .map(|command| command.statement.clone())
        {
            let mut text = self.source.clone();
            text.replace_range(trimmed_span(&text, span), "");
            let candidate = Self::from_source(text);
            let expected: Vec<&str> = self
                .drafts
                .iter()
                .map(|(key, _)| key.as_str())
                .filter(|key| *key != name)
                .collect();
            let got: Vec<&str> = candidate.drafts.iter().map(|(k, _)| k.as_str()).collect();
            if got == expected {
                *self = candidate;
                return true;
            }
        }
        // Fall back to a full re-render without the command.
        let keep: Vec<(String, Draft)> = self
            .drafts
            .iter()
            .filter(|(key, _)| key != name)
            .cloned()
            .collect();
        let flags: BTreeSet<&str> = self
            .pack
            .commands
            .iter()
            .filter(|c| c.overrides_shipped && c.spec.name != name)
            .map(|c| c.spec.name)
            .collect();
        let drafts: Vec<Draft> = keep.iter().map(|(_, d)| d.clone()).collect();
        let text = with_override_flags(
            &render_spectcl::render_pack_with_version(&drafts, self.name(), self.render_version()),
            &flags,
        );
        *self = Self::from_source(text);
        true
    }

    // ── write-back internals ───────────────────────────────────────────

    fn draft_requires_vocabulary_upgrade(&self, draft: &Draft) -> bool {
        let previous = self.dsl_version();
        if previous.is_empty()
            || !tcl_registry::version::compare(previous, render_spectcl::DSL_VERSION).is_lt()
        {
            return false;
        }
        let probe = render_spectcl::render_pack_with_version(
            std::slice::from_ref(draft),
            "vocabulary-probe",
            previous,
        );
        loader::evaluate_pack(&probe)
            .notices
            .iter()
            .any(|notice| notice.newer_vocabulary.is_some())
    }

    /// Raise an older pack header before writing a draft rendered with the
    /// current vocabulary. This changes only the version word; comments,
    /// layout, metadata, and existing declarations remain byte-for-byte.
    fn upgrade_vocabulary_for_edit(&mut self) -> Option<String> {
        let previous = self.dsl_version().to_owned();
        if previous.is_empty()
            || !tcl_registry::version::compare(&previous, render_spectcl::DSL_VERSION).is_lt()
        {
            return None;
        }
        let version_span = segments(&self.source, SourceSegmentationPurpose::WholeInput)
            .into_iter()
            .find(|statement| statement.words.first().map(String::as_str) == Some("speclib"))?
            .spans
            .get(2)?
            .inner
            .clone();
        let mut source = self.source.clone();
        source.replace_range(version_span, render_spectcl::DSL_VERSION);
        *self = Self::from_source(source);
        Some(previous)
    }

    /// The pack-body text for one command: whatever a one-command pack would
    /// put inside its `speclib` braces, which is the command block plus any
    /// value tables it needs.
    fn render_block(&self, draft: &Draft, overrides: bool) -> String {
        // In the *document's* vocabulary, not the renderer's newest: a block
        // spliced into a pack that declares 1.1 must be a 1.1 block, or the
        // document ends up with words newer than its own header — the
        // inconsistency the loader reports per site. A draft that
        // genuinely needs newer vocabulary has already raised the header by
        // the time this runs (`draft_requires_vocabulary_upgrade`).
        let one = render_spectcl::render_pack_with_version(
            std::slice::from_ref(draft),
            self.name(),
            self.render_version(),
        );
        let name = draft
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let one = if overrides {
            with_override_flags(&one, &BTreeSet::from([name]))
        } else {
            one
        };
        pack_body(&one)
            .map(|range| one[range].trim_matches('\n').to_owned())
            .unwrap_or(one)
    }

    /// Put back the statements the renderer had nothing to render from.
    ///
    /// A draft is not a superset of a declaration. `arg_role_resolver { … }`
    /// loads into a **function pointer**, and drafting that back gives only
    /// "this field is set, the expression could not be recovered" — at which
    /// point the renderer writes the only thing it can, a
    /// `arg_role_resolver -native <command>::<field>` placeholder naming a
    /// native hook that does not exist. Left alone, that turns a form edit into
    /// a silent deletion of the author's Tcl *dressed up as a valid
    /// declaration*, which is worse than an outright loss.
    ///
    /// The text, however, still has the body, and the text is the store. So a
    /// top-level statement is taken from the author's own bytes, verbatim,
    /// whenever the freshly rendered block either
    ///
    /// - does not mention that property word at all, or
    /// - mentions it only as a `-native` placeholder the original did not use.
    ///
    /// A hook hanging off a **subcommand** is carried forward too, one level
    /// down: each of the old declaration's `subcommand NAME { … }` blocks is
    /// matched to the freshly rendered one of the same name, and the same
    /// reclaim runs inside it, before the command-level pass above runs over
    /// the (now subcommand-patched) result. A subcommand renamed or removed
    /// by the edit has nowhere to carry its body forward *into*, so it is
    /// left alone here — `declared_properties` still reports the loss.
    /// Deeper than that — a sub-subcommand's own hook — is beyond this pass.
    pub(super) fn carry_forward(&self, name: &str, block: &str) -> String {
        let Some(old_body) = self
            .source_index()
            .command(name)
            .map(|command| command.body.clone())
        else {
            return block.to_owned();
        };
        let old_body_text = self.source[old_body.clone()].to_owned();
        let old_statements = segments(&old_body_text, SourceSegmentationPurpose::NestedScript);
        // The rendered block is a pack *body*: the command statement plus any
        // value tables it needed, with no `speclib` wrapper.
        if find_command(block, Some(0..block.len()), name).is_none() {
            return block.to_owned();
        }

        let mut out = block.to_owned();
        for statement in &old_statements {
            if statement.words.first().map(String::as_str) != Some("subcommand") {
                continue;
            }
            let (Some(sub_name), Some(span)) = (statement.words.get(1), statement.spans.last())
            else {
                continue;
            };
            // `span.inner` is relative to `old_body_text`, the slice `segments`
            // read it from.
            let sub_body_text = old_body_text[span.inner.clone()].to_owned();
            let sub_statements = segments(&sub_body_text, SourceSegmentationPurpose::NestedScript);
            let Some((_, new_command_body)) = find_command(&out, Some(0..out.len()), name) else {
                continue;
            };
            let Some((_, new_sub_body)) = find_subcommand(&out, new_command_body, sub_name) else {
                continue;
            };
            if let Some(patched) =
                reclaim(&sub_body_text, &sub_statements, &out, new_sub_body.clone())
            {
                out.replace_range(new_sub_body, &patched);
            }
        }

        let Some((_, new_body)) = find_command(&out, Some(0..out.len()), name) else {
            return out;
        };
        match reclaim(&old_body_text, &old_statements, &out, new_body.clone()) {
            Some(patched) => {
                out.replace_range(new_body, &patched);
                out
            }
            None => out,
        }
    }

    /// Every property the declaration of `name` states, one level into its
    /// subcommands, mapped to whether it is a real declaration or a `-native`
    /// placeholder.
    ///
    /// This is the inventory a write-back is judged against: a property that
    /// *vanishes*, and a property that *degrades from a body to a placeholder*,
    /// are both losses and both have to be reported.
    fn declared_properties(&self, name: &str) -> BTreeMap<String, bool> {
        let mut out = BTreeMap::new();
        let Some(body) = self
            .source_index()
            .command(name)
            .map(|command| command.body.clone())
        else {
            return out;
        };
        for statement in segments(
            &self.source[body.clone()],
            SourceSegmentationPurpose::NestedScript,
        ) {
            let Some(word) = statement.words.first() else {
                continue;
            };
            out.insert(word.clone(), is_native_placeholder(&statement));
            if word != "subcommand" {
                continue;
            }
            let (Some(sub), Some(span)) = (statement.words.get(1), statement.spans.last()) else {
                continue;
            };
            let inner = body.start + span.inner.start..body.start + span.inner.end;
            for nested in segments(&self.source[inner], SourceSegmentationPurpose::NestedScript) {
                if let Some(word) = nested.words.first() {
                    out.insert(
                        format!("subcommand {sub} / {word}"),
                        is_native_placeholder(&nested),
                    );
                }
            }
        }
        out
    }

    /// Splice `block` into the document, replacing `name`'s statement or
    /// appending at the end of the pack body. `None` when the document has no
    /// place to put it (no `speclib` body at all).
    fn write_attempt(&self, name: &str, block: &str, existed: bool) -> Option<String> {
        let mut text = self.source.clone();
        if existed {
            let span = self.source_index().command(name)?.statement.clone();
            text.replace_range(span, block);
        } else {
            let body = self.source_index().pack_body.clone()?;
            let at = self.source[..body.end].trim_end().len();
            text.insert_str(at, &format!("\n\n{block}\n"));
        }
        Some(text)
    }

    /// Whether `candidate` means exactly what the edit asked for.
    ///
    /// Two claims, and both are checked by *loading text and comparing drafts*
    /// — never by comparing a draft against a render, so the renderer's own
    /// documented gaps never enter into it:
    ///
    /// - the new declaration means what the block that was written means, on
    ///   its own, out of context (so a value table whose generated name
    ///   collided with one already in the file is caught), and
    /// - every other command in the document still means exactly what it meant.
    fn accepts(
        &self,
        candidate: &Self,
        name: &str,
        block: &str,
        overrides: bool,
        existed: bool,
    ) -> bool {
        let Some((written, expected)) = Self::block_meaning(self.name(), block) else {
            return false;
        };
        let written = written.as_str();
        let mut want: Vec<&str> = self
            .drafts
            .iter()
            .map(|(k, _)| if k == name { written } else { k.as_str() })
            .collect();
        if !existed {
            want.push(written);
        }
        let got: Vec<&str> = candidate.drafts.iter().map(|(k, _)| k.as_str()).collect();
        if got != want {
            return false;
        }
        if candidate.overrides_shipped(written) != overrides {
            return false;
        }
        for (key, before) in &self.drafts {
            if key == name {
                continue;
            }
            if candidate.draft(key) != Some(before) {
                return false;
            }
            if candidate.overrides_shipped(key) != self.overrides_shipped(key) {
                return false;
            }
        }
        candidate.draft(written) == Some(&expected)
    }

    /// What a pack body means on its own: the command it declares, and that
    /// command's draft.
    ///
    /// Wrapping the block in a minimal `speclib` and loading it is the only
    /// honest answer to "what did I just write" — it is the same loader the
    /// server runs, reading the same bytes the file will hold.
    fn block_meaning(pack_name: &str, block: &str) -> Option<(String, Draft)> {
        let wrapped = format!(
            "speclib {} {} {{\n{block}\n}}\n",
            render_spectcl::header_word(pack_name),
            render_spectcl::DSL_VERSION
        );
        let pack = loader::evaluate_pack(&wrapped);
        let command = pack.commands.first()?;
        Some((
            command.spec.name.to_owned(),
            draft::from_command_spec(command.spec),
        ))
    }

    /// The re-render floor: rebuild the whole document from drafts, with
    /// `name` set to `draft`.
    fn rerender_with(&mut self, name: &str, draft: &Draft, overrides: bool) {
        let mut drafts: Vec<Draft> = Vec::with_capacity(self.drafts.len() + 1);
        let mut flags: BTreeSet<String> = self
            .pack
            .commands
            .iter()
            .filter(|c| c.overrides_shipped)
            .map(|c| c.spec.name.to_owned())
            .collect();
        let mut replaced = false;
        for (key, existing) in &self.drafts {
            if key == name {
                drafts.push(draft.clone());
                replaced = true;
            } else {
                drafts.push(existing.clone());
            }
        }
        if !replaced {
            drafts.push(draft.clone());
        }
        if overrides {
            flags.insert(name.to_owned());
        } else {
            flags.remove(name);
        }
        let flags: BTreeSet<&str> = flags.iter().map(String::as_str).collect();
        let pack_name = if self.name().is_empty() {
            "pack"
        } else {
            self.name()
        };
        let text = with_override_flags(
            &render_spectcl::render_pack_with_version(&drafts, pack_name, self.render_version()),
            &flags,
        );
        *self = Self::from_source(text);
    }
}
