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

//! Programmed documents and the patch pack laid over one: why a document is a program, standing overrides and the patch's own rendering.

use super::source_spans::{stated_verbatim, with_override_flags};
use super::write_back::{Write, WriteBack};
use super::{AUTHORING_TIER, PackStore, Patch, stamp_refusals_at};
use crate::draft::Draft;
use crate::{draft, render_spectcl};
use serde_json::Value;
use std::collections::BTreeSet;
use tcl_spectcl::export::Registration;
use tcl_spectcl::{Tier, export, loader};

/// Why a document is a **program** rather than a canonical pack (E-R11), and
/// therefore why the studio will not rewrite it (E-R12).
///
/// The three answers are the three things the studio can *see* from a
/// snapshot and the bytes that produced it. They are checked in this order,
/// so the reported reason is the most specific one that holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Programmed {
    /// The pack queried `available?` while it registered (E-R1), so its
    /// surface is one analysis target's answer. Writing that answer back as
    /// source would freeze one target's world into the file.
    TargetDependent,
    /// A command in the snapshot has no `command NAME { … }` statement in
    /// this document — a loop, a `proc`, or a computed name registered it.
    /// There is no byte range to splice.
    Expanded,
    /// The `speclib` body holds a top-level statement that is not one of the
    /// registration calls the snapshot recorded — a `proc`, a `set`, a
    /// `foreach`, an `if`. Every declaration may still be literal, but a
    /// re-render would delete the program around them.
    NonCanonicalStatement,
}

impl Programmed {
    /// The word the report uses.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::TargetDependent => "target-dependent",
            Self::Expanded => "expanded",
            Self::NonCanonicalStatement => "non-canonical-statement",
        }
    }

    /// One sentence a surface can show beside a read-only source pane.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::TargetDependent => {
                "this pack queried `available?` while registering, so what you \
                 see is one analysis target's answer (design E-R1)"
            }
            Self::Expanded => {
                "this pack registers commands from a program, so some \
                 declarations have no statement of their own to edit"
            }
            Self::NonCanonicalStatement => {
                "this pack's body holds statements that are not registration \
                 calls, so rewriting it would delete the program"
            }
        }
    }
}

/// One command a patch pack is currently overriding: what a UI labels a
/// patched row with, and what `spec check` reports so a patch cannot rot
/// silently (design E §15.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingOverride {
    /// The command the patch redefines.
    pub command: String,
    /// The `speclib` name of the patch pack holding the override.
    pub patch_pack: String,
    /// The `speclib` name of the base pack being overridden.
    pub base_pack: String,
    /// Where the base pack declared it — the line inside the program, for a
    /// declaration the program produced.
    pub base_line: Option<u32>,
    /// Whether the base declaration is a program's output rather than a
    /// statement in the base document.
    pub base_expanded: bool,
}

impl Patch {
    /// Load `source` as the patch, at the authoring tier.
    ///
    /// The **authoring** tier, deliberately, for the reason
    /// [`AUTHORING_TIER`] gives: a patch pack's whole job is to declare
    /// `-override`, and evaluating it as untrusted would discard it the
    /// moment the overridden name happened to be a compiled one. The gate is
    /// not lost — [`PackStore::patch_untrusted_tier_refusal`] reports what a
    /// real Spec Studio override tier would refuse — and
    /// [`PackStore::pack_set`] still installs the patch at
    /// [`Tier::StudioOverride`].
    pub(super) fn from_source(source: String) -> Self {
        let pack = loader::evaluate_pack_with(
            &source,
            &loader::EvalOptions {
                tier: AUTHORING_TIER,
                ..loader::EvalOptions::default()
            },
        );
        let drafts = pack
            .commands
            .iter()
            .map(|c| (c.spec.name.to_owned(), draft::from_command_spec(c.spec)))
            .collect();
        Self {
            source,
            pack,
            drafts,
        }
    }
}

impl PackStore {
    // ── E-R12: programmed documents and their patch packs ──────────────

    /// Why this document is a **program** rather than a canonical pack, or
    /// `None` when it is canonical and edits it in place as they always did.
    ///
    /// Three questions, most specific first, all answerable from the snapshot
    /// and the bytes that produced it:
    ///
    /// 1. did the pack query `available?` while registering
    ///    ([`Pack::target_dependent`], E-R1)?
    /// 2. is any browsable command an **expansion** — no `command NAME { … }`
    ///    statement of its own in this document ([`Self::declaration_site`])?
    /// 3. does the `speclib` body hold a top-level statement that is not one
    ///    of the registration calls the snapshot recorded?
    ///
    /// The third is what catches the pack whose declarations happen to all be
    /// literal but which wraps them in a `proc`, a `set`, or a data table: no
    /// declaration is an expansion, so (2) is silent, and yet the re-render
    /// floor would still delete the program. Comparing the document's own
    /// top-level statements against [`Pack::registrations`] pairwise — same
    /// count, same head word, same first argument — is the strongest test the
    /// record supports without the loader publishing a per-statement
    /// canonicality verdict of its own.
    #[must_use]
    pub fn programmed(&self) -> Option<Programmed> {
        if self.pack.target_dependent {
            return Some(Programmed::TargetDependent);
        }
        if self
            .drafts
            .iter()
            .any(|(name, _)| self.source_index().command(name).is_none())
        {
            return Some(Programmed::Expanded);
        }
        (!self.straight_line()).then_some(Programmed::NonCanonicalStatement)
    }

    /// Whether every top-level statement the document states is one of the
    /// registration calls the snapshot recorded, in the same order.
    ///
    /// True for every canonical pack — the loader records the file's own
    /// statements — and false the moment a statement runs rather than
    /// registers.
    pub(super) fn straight_line(&self) -> bool {
        if self.source_index().pack_body.is_none() {
            // Not a pack at all: nothing was registered, and there is nothing
            // programmed about a document that declares nothing.
            return self.pack.registrations.is_empty();
        }
        let statements = &self.source_index().document_statements;
        statements.len() == self.pack.registrations.len()
            && statements
                .iter()
                .zip(&self.pack.registrations)
                .all(|(statement, registration)| {
                    let paired = statement.words.first().map(String::as_str)
                        == Some(registration.word())
                        && statement.words.get(1).map_or("", String::as_str) == registration.arg(1);
                    paired && (statement.inside_pack || stated_verbatim(statement, registration))
                })
    }

    /// The document's own top-level statements, in the order the loader reads
    /// them: the `speclib` call stands for the statements its body holds, and
    /// a statement the file states *outside* that call keeps its place in the
    /// list.
    ///
    /// The registration record covers the whole file rather than the block
    /// alone, so the comparison in [`Self::straight_line`] has to be made
    /// against the whole file too. Reading only the body counted a word left
    /// outside the block as a statement the program had computed, and judged
    /// an ordinary document a program: every edit after it then went silently
    /// to a patch pack instead of into the document (E-R12).
    ///
    /// The discrimination survives the widening. A statement that *runs* is
    /// recorded as nothing at all, so the counts still disagree and the
    /// document is still a program; text the loader reads and drops — a
    /// half-typed `command`, an unknown property — is recorded like any other
    /// statement and pairs off. A statement outside the block that registers
    /// is held to [`stated_verbatim`] as well, because outside is exactly
    /// where the head-and-first-argument comparison is too weak to tell a
    /// declaration from the program that computed it.
    /// The canonical patch pack standing over this document, when form edits
    /// have landed against a programmed one.
    #[must_use]
    pub fn patch_source(&self) -> Option<&str> {
        self.patch.as_ref().map(|patch| patch.source.as_str())
    }

    /// The `speclib` name the patch pack would carry — derived from the base
    /// pack's, so a surface can name it before one exists.
    #[must_use]
    pub fn patch_name(&self) -> String {
        let base = if self.pack.name.is_empty() {
            "pack"
        } else {
            &self.pack.name
        };
        format!("{base}-studio-overrides")
    }

    /// The draft the patch pack declares for `name`, if it overrides it.
    #[must_use]
    pub fn patch_draft(&self, name: &str) -> Option<&Draft> {
        self.patch.as_ref().and_then(|patch| {
            patch
                .drafts
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, d)| d)
        })
    }

    /// The draft that is actually **live** for `name`: the patch's when one
    /// stands over it, the document's otherwise.
    ///
    /// Identical to [`Self::draft`] for every canonical pack, which never has
    /// a patch. This is what a form should show after an edit to a programmed
    /// pack, because it is what the registry answers with.
    #[must_use]
    pub fn effective_draft(&self, name: &str) -> Option<&Draft> {
        self.patch_draft(name).or_else(|| self.draft(name))
    }

    /// Every command a patch pack is currently overriding, in the patch's own
    /// declaration order — the surface a UI labels patched rows from, and the
    /// one `spec check` reports so an override cannot rot unnoticed.
    #[must_use]
    pub fn standing_overrides(&self) -> Vec<StandingOverride> {
        let Some(patch) = &self.patch else {
            return Vec::new();
        };
        patch
            .drafts
            .iter()
            .map(|(name, _)| {
                let site = self.declaration_site(name);
                StandingOverride {
                    command: name.clone(),
                    patch_pack: patch.pack.name.clone(),
                    base_pack: self.name().to_owned(),
                    base_line: site.as_ref().map(|site| site.line),
                    base_expanded: site.is_some_and(|site| site.expanded),
                }
            })
            .collect()
    }

    /// What the real [`Tier::StudioOverride`] tier would refuse the patch pack
    /// for, as `(line, why)` — E-R2 asked of the patch the way
    /// [`Self::untrusted_tier_refusal`] asks it of the document.
    ///
    /// `Some` exactly when the patch overrides a **compiled** command name: a
    /// studio author may write that, and a real override-tier load would
    /// discard the pack for it, so the warning is reported rather than
    /// applied.
    #[must_use]
    pub fn patch_untrusted_tier_refusal(&self) -> Option<(u32, String)> {
        self.patch
            .as_ref()
            .and_then(|patch| tcl_spectcl::provenance_violation(&patch.pack, Tier::StudioOverride))
    }

    /// The codegen-axis stamps a workspace load of this document would drop,
    /// as `(line, why)` on each declaring command's row: the stamp rejection
    /// rule (`tcl_spectcl::stamps`) refuses every one outside a bundled pack.
    /// The installed world ([`Self::pack_set`]) already drops them; the
    /// document keeps the rows as written, so this is where an author learns
    /// they will not change emitted code.
    #[must_use]
    pub fn stamp_refusals(&self) -> Vec<(u32, String)> {
        stamp_refusals_at(&self.pack, Tier::Workspace)
    }

    /// [`Self::stamp_refusals`] for the standing patch, at the override tier
    /// it layers from.
    #[must_use]
    pub fn patch_stamp_refusals(&self) -> Vec<(u32, String)> {
        self.patch
            .as_ref()
            .map_or_default(|patch| stamp_refusals_at(&patch.pack, Tier::StudioOverride))
    }

    /// Install `patch` as this document's standing patch pack, replacing any
    /// it already had. Blank text clears it.
    ///
    /// Returns whether a patch stands afterwards.
    pub fn adopt_patch(&mut self, patch: &str) -> bool {
        if patch.trim().is_empty() {
            self.patch = None;
            return false;
        }
        let loaded = Patch::from_source(patch.to_owned());
        if loaded.drafts.is_empty() {
            self.patch = None;
            return false;
        }
        self.patch = Some(loaded);
        true
    }

    /// Take `name` back out of the patch pack, restoring the base
    /// declaration. Dropping the last override drops the patch entirely.
    ///
    /// Returns `false` when no patch overrides `name`.
    pub fn remove_override(&mut self, name: &str) -> bool {
        if self.patch_draft(name).is_none() {
            return false;
        }
        let keep: Vec<(String, Draft)> = self.patch.as_ref().map_or_default(|patch| {
            patch
                .drafts
                .iter()
                .filter(|(key, _)| key != name)
                .cloned()
                .collect()
        });
        self.patch = self.render_patch(&keep);
        true
    }

    /// Render `commands` as a canonical patch pack, or `None` when there are
    /// none left to render.
    ///
    /// Canonical **by construction**: the drafts go through the ordinary
    /// whole-pack renderer (one call, so shared value tables are named once
    /// rather than colliding between per-command renders), that text is
    /// loaded, the base pack's `default` context rows are put in front of the
    /// registrations it recorded, and the whole record is spelled back out by
    /// `tcl_spectcl::export::export_pack` — the same expansion `tcl spec
    /// export` and `spectcl_expand` produce. A patch is therefore a
    /// straight-line pack whatever the document it patches looks like.
    ///
    /// Every declaration is written `-override`, whatever the form asked for:
    /// a patch that does not override is a patch that does not apply, because
    /// `installs_over` admits a later pack's claim on a held name only when it
    /// says so.
    fn render_patch(&self, commands: &[(String, Draft)]) -> Option<Patch> {
        if commands.is_empty() {
            return None;
        }
        let name = self.patch_name();
        let drafts: Vec<Draft> = commands.iter().map(|(_, draft)| draft.clone()).collect();
        let flags: BTreeSet<&str> = commands.iter().map(|(key, _)| key.as_str()).collect();
        let seed = with_override_flags(
            &render_spectcl::render_pack_with_version(&drafts, &name, self.render_version()),
            &flags,
        );
        let mut seeded = loader::evaluate_pack(&seed);
        // The minimal available context: the base pack's own `default` rows,
        // verbatim from its registration record. Without them the patch's
        // commands would inherit a different availability from the ones they
        // replace — the patch would change more than the author edited.
        let context: Vec<Registration> = self
            .pack
            .registrations
            .iter()
            .filter(|reg| reg.word() == "default")
            .cloned()
            .collect();
        seeded.registrations = context.into_iter().chain(seeded.registrations).collect();
        seeded.name = name;
        self.render_version().clone_into(&mut seeded.dsl_version);
        Some(Patch::from_source(export::export_pack(&seeded)))
    }

    /// Write `draft` back as a patch-pack override of `name` (E-R12): the
    /// author's source is not touched, and the edit lands in the
    /// `StudioOverride` tier instead.
    ///
    /// The form's `-override` tick is not consulted here, and cannot be: a
    /// patch declaration that does not override is inert (see
    /// [`Self::render_patch`]). What the tick decides for a canonical pack —
    /// whether the pack claims a *shipped* name — a patch decides by what it
    /// patches.
    pub(super) fn patch_command(&mut self, name: &str, draft: &Draft) -> Write {
        let written = draft
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(name)
            .to_owned();
        let mut commands: Vec<(String, Draft)> = self
            .patch
            .as_ref()
            .map_or_default(|patch| patch.drafts.clone());
        // Editing a command's `name` field renames the override, so the old
        // entry goes as well as the new one arriving.
        commands.retain(|(key, _)| key != name && key != &written);
        commands.push((written.clone(), draft.clone()));
        self.patch = self.render_patch(&commands);
        let dropped = if self.patch_draft(&written).is_some() {
            Vec::new()
        } else {
            vec![format!(
                "command {written} (the patch pack did not load it back)"
            )]
        };
        Write {
            how: WriteBack::Patched,
            dropped,
            upgraded_from: None,
        }
    }
}
