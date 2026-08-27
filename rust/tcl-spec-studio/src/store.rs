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

//! The studio's models, with no UI anywhere near them.
//!
//! `docs/design/spec-packs.md`'s "Phase 2: the studio becomes the DSL's IDE"
//! asks for exactly three things in this layer, and this module is all three:
//!
//! - **[`Builtins`]** — the immutable registry the wasm ships. Reference
//!   material, never edited, one per dialect profile.
//! - **[`PackStore`]** — the user's definitions, whose **authoritative form is
//!   the `.tclspec` text**. Drafts are *derived* from that text by the real
//!   loader, never held alongside it, so the DSL pane and the form are two
//!   projections of one document rather than two stores to keep in sync.
//!   The derivation runs the **evaluation loader** (design E §15.2): a pack
//!   may be a program, so what the store browses is the *snapshot* it
//!   registered. A canonical pack — every pack shipped today — is unchanged
//!   by that, and [`PackStore::declaration_site`] says which declarations
//!   are literal text and which a loop produced.
//! - **[`Resolution`]** — one facade over the pair, applying the shipped
//!   collision policy (`shipped wins unless the pack says -override`, the rule
//!   `tcl_spectcl::pack::installs_over` encodes) so every surface queries the
//!   same merged world a real editor would see.
//!
//! ## Write-back: targeted splice, verified, with a re-render floor
//!
//! A form edit has to become DSL text. [`PackStore::set_command`] does that in
//! two tiers:
//!
//! 1. **Splice.** Render *just the edited command* as a one-command pack, take
//!    that pack's body, and replace the byte range the CST reports for the
//!    existing `command NAME { … }` statement. Every other byte of the
//!    document — the author's banner comment, their blank lines, the comments
//!    inside every *other* command — survives untouched.
//! 2. **Carry forward what a draft cannot say.** A draft is *not* a superset of
//!    a declaration: `arg_role_resolver { … }` loads into a function pointer,
//!    and drafting that back yields only "set, expression not recovered". A
//!    block rendered from the draft would therefore silently delete the
//!    author's Tcl. So every top-level property word the old declaration used
//!    and the new block does not is copied across **verbatim from the author's
//!    own bytes**, and whatever still could not be kept is named in
//!    [`Write::dropped`] rather than lost quietly.
//! 3. **Verify, or fall back.** The spliced text is loaded again and compared
//!    against what the written block means on its own, out of context, and
//!    every other command must draft to exactly what it drafted to before. If
//!    anything disagrees — a value table whose generated name collided, a
//!    pack-level construct the splice disturbed — the whole document is
//!    re-rendered instead, which is always correct and never preserves layout.
//!
//! **The known limitations, stated plainly:**
//!
//! - The splice is *statement* granular, not *field* granular. Comments and
//!   layout **inside the command being edited** are re-rendered from the draft
//!   and therefore lost; comments and layout everywhere else are preserved.
//!   Field-granular surgery — editing the one `arity` row and leaving its
//!   neighbouring comment alone — is the next slice, and needs the loader to
//!   publish per-statement spans it currently keeps `pub(crate)`.
//! - Carry-forward is *top level* only. A hook body hanging off a
//!   **subcommand** is not carried, because the renderer does emit a
//!   `subcommand` statement and the two would have to be merged word by word.
//!   That case is detected and reported through [`Write::dropped`], so a
//!   surface can tell the author what an edit will cost before it costs it.
//!
//! ## A programmed pack is never rewritten — it is patched (E-R12)
//!
//! Everything above describes editing a **canonical** document, and that is
//! the only kind of document it may describe: a splice replaces the byte range
//! of a `command NAME { … }` statement, and the re-render floor rebuilds the
//! whole file from drafts. Neither is admissible against a *program*. The
//! statement a form edit wants to replace may not exist (a `foreach` wrote the
//! declaration), and a re-render would delete the program itself — the
//! `proc`s, the data table, the loop — replacing it with its own expansion.
//!
//! So [`PackStore::programmed`] classifies the document first, and a form edit
//! against a programmed one takes a different route entirely
//! ([`WriteBack::Patched`]): the edited command is rendered as a **canonical
//! patch pack** — `speclib <base>-studio-overrides`, the base pack's
//! `default` context rows, and the edited commands as `-override`
//! declarations, all spelled by `tcl_spectcl::export::export_pack` so the
//! patch is canonical by construction — and that pack is layered over the
//! base in the [`Tier::StudioOverride`] tier through [`PackStore::pack_set`].
//! The author's own source is not touched by a single byte.
//!
//! The layering is the shipped collision policy, not a studio invention: the
//! patch pack sits **after** the base in the [`PackSet`], so
//! `tcl_spectcl::pack::installs_over` admits its commands exactly because
//! they declare `-override`, and the registry answers with the last spec
//! registered for the name. [`PackStore::standing_overrides`] is the
//! queryable surface a UI labels from — which command, from which patch,
//! against which base pack and base declaration — so a patch cannot rot
//! silently, and [`PackStore::remove_override`] takes one back out and
//! restores the base.
//!
//! ## A note on cost
//!
//! The loader leaks its specs (`Box::leak`), because a
//! loaded pack is meant to live as long as the server process. In a long-lived
//! browser page that makes every reload a small permanent allocation, so
//! callers should debounce keystrokes and reuse a [`PackStore`] rather than
//! rebuild one per query — which is what the wasm facade's cached store does.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde_json::{Value, json};
use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_lexer::{LexerConfig, SourceMap};
use tcl_registry::CommandRegistry;
use tcl_spectcl::Tier;
use tcl_spectcl::export::{self, Registration};
use tcl_spectcl::loader::{self, FileExtension, HookOwner, HookSource, Notice, Pack};
use tcl_spectcl::pack::{MergedPack, PackSet};

use crate::draft::{self, Draft};
use crate::render_spectcl;

// ---------------------------------------------------------------------------
// Built-ins — the immutable model
// ---------------------------------------------------------------------------

/// The shipped command registry for one dialect: reference material the studio
/// reads and never writes.
#[derive(Clone, Copy)]
pub struct Builtins {
    dialect: &'static str,
    registry: &'static CommandRegistry,
}

impl Builtins {
    /// The built-ins for `dialect`, resolved the way every other registry
    /// lookup in the server resolves one.
    #[must_use]
    pub fn for_dialect(dialect: &str) -> Self {
        // The catalogue holds the `&'static str` the caller means, alias
        // spellings canonicalised onto it; a name with no catalogue entry
        // at all resolves to the dialect the picker starts on, and keeping
        // a `'static` name avoids an allocation per query. Both halves go
        // through the one ingress seam (`crate::environment`).
        let dialect = crate::environment::catalogue_dialect_or_default(dialect);
        Self {
            dialect,
            registry: crate::environment::store_for_dialect(dialect),
        }
    }

    /// The dialect these built-ins were resolved for.
    #[must_use]
    pub fn dialect(self) -> &'static str {
        self.dialect
    }

    /// Whether the shipped registry defines `name`.
    #[must_use]
    pub fn contains(self, name: &str) -> bool {
        self.registry.get(name).is_some()
    }

    /// A draft seeded from the shipped spec for `name`.
    #[must_use]
    pub fn draft(self, name: &str) -> Option<Draft> {
        let spec = self.registry.get(name)?;
        let mut d = draft::from_command_spec(spec);
        d.insert(draft::SOURCE_DIALECT_KEY.to_owned(), json!(self.dialect));
        Some(d)
    }
}

// ---------------------------------------------------------------------------
// The pack store — the DSL text is the model
// ---------------------------------------------------------------------------

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

/// Where one browsable command was declared, and how it got there.
///
/// The data plumbing behind an "expanded from" label: design E lets a pack
/// register commands from a loop, and a surface that shows the *snapshot*
/// has to be able to say which declarations are literal text the author can
/// edit and which are a program's output that they cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationSite {
    /// The line the `command` statement was made on. For an expanded
    /// command that is the line **inside the program** that registered it.
    pub line: u32,
    /// The file, when the pack came through a merge that knows one. Always
    /// `None` for a document open in the studio, which has no file.
    pub file: Option<std::path::PathBuf>,
    /// Whether the declaration is a program's output rather than a statement
    /// in this document.
    pub expanded: bool,
}

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

/// The canonical patch pack laid over a programmed document.
///
/// Its `source` is the whole truth, exactly as the base document's is: the
/// pack and the drafts beside it are derived from those bytes by the same
/// loader, so there is no second copy to drift.
struct Patch {
    source: String,
    pack: Pack,
    drafts: Vec<(String, Draft)>,
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
    fn from_source(source: String) -> Self {
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

/// The user's definitions, backed by one `.tclspec` document.
///
/// The text is the store. Everything else on this struct is derived from it by
/// [`loader::evaluate_pack_with`] and recomputed whenever the text changes, so
/// there is no second copy of the truth to drift.
pub struct PackStore {
    source: String,
    pack: Pack,
    /// The provenance tier the document is *evaluated* under (E-R2).
    tier: Tier,
    /// One draft per declared command, in declaration order — derived, and the
    /// only thing the form is ever handed.
    drafts: Vec<(String, Draft)>,
    /// The `StudioOverride`-tier patch pack, when form edits have landed
    /// against a programmed document (E-R12). `None` for every canonical
    /// pack, which is every pack shipped today.
    patch: Option<Patch>,
}

/// The tier a document open in the studio evaluates under.
///
/// **Trusted, deliberately.** E-R2 gates what a pack loaded *from* an
/// untrusted tier may register — it is a question about where a file was
/// discovered, and the answer for the document under the author's own cursor
/// is "they wrote it". Gating the editing buffer would make the studio unable
/// to author the one thing E-R2 is about: a `-override` on a compiled name is
/// a first-class studio operation ([`PackStore::set_command`] takes it as an
/// argument), and evaluating the buffer as untrusted would discard the whole
/// pack the moment the author ticked that box.
///
/// The gate is not lost, only moved to where it belongs: [`PackStore::pack_set`]
/// still installs at [`Tier::Workspace`], and
/// [`PackStore::untrusted_tier_refusal`] reports — without failing anything —
/// what a workspace or Spec Studio load would refuse, so a surface can warn
/// before the pack silently fails to load in an editor.
const AUTHORING_TIER: Tier = Tier::Bundled;

impl PackStore {
    /// Load `source` as the store's document, at the authoring tier.
    ///
    /// Never fails: an unparsable or empty document loads as a pack with no
    /// commands and the loader's notices explaining why.
    #[must_use]
    pub fn from_source(source: impl Into<String>) -> Self {
        Self::from_source_at_tier(source, AUTHORING_TIER)
    }

    /// [`PackStore::from_source`] at an explicit provenance tier.
    ///
    /// The document is **evaluated** (design E §1), not walked: a pack may be
    /// a program, and the studio browses the *snapshot* it registered rather
    /// than the text that registered it. For a canonical pack — every pack
    /// shipped today — nothing observable changes, which is what the golden
    /// gate (`tcl-spectcl/tests/golden_packs.rs`), the fast-path gate
    /// (`tcl-spectcl/tests/eval_loader.rs`) and the studio's own round-trip
    /// suite hold this to.
    #[must_use]
    pub fn from_source_at_tier(source: impl Into<String>, tier: Tier) -> Self {
        let source = source.into();
        let pack = loader::evaluate_pack_with(
            &source,
            &loader::EvalOptions {
                tier,
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
            tier,
            drafts,
            patch: None,
        }
    }

    /// [`PackStore::from_source`] with a patch pack already standing over it.
    ///
    /// The studio's state is two documents once a programmed pack has been
    /// edited — the author's source and the canonical patch — and a surface
    /// that persists them (a browser reload, a saved workspace) restores the
    /// pair through this rather than losing the overrides. An empty or
    /// unparsable patch restores nothing, which is the same answer as never
    /// having had one.
    #[must_use]
    pub fn from_source_with_patch(source: impl Into<String>, patch: &str) -> Self {
        let mut store = Self::from_source(source);
        store.adopt_patch(patch);
        store
    }

    /// The provenance tier this document evaluates under.
    #[must_use]
    pub const fn tier(&self) -> Tier {
        self.tier
    }

    /// Whether the document queried `available?` while it registered (E-R1).
    ///
    /// A target-dependent pack's snapshot is one *target's* answer rather
    /// than the pack's: the commands below are what this analysis target
    /// would see, and another would see a different set. The store surfaces
    /// it so a surface can say so; nothing here changes because of it.
    #[must_use]
    pub const fn target_dependent(&self) -> bool {
        self.pack.target_dependent
    }

    /// What an untrusted tier would refuse this document for, as
    /// `(line, why)` — E-R2 asked of the snapshot rather than of the load.
    ///
    /// A **hypothetical**, deliberately: `Tier::Workspace` is
    /// `Provenance::WorkspaceTrusted` today (§6.4 keys the untrusted class on
    /// the editor's Workspace Trust state, which nothing on the discovery
    /// path is told — redesign §11.1 O9), so a workspace pack that overrides
    /// a shipped command still loads. This answers the question the author
    /// wants answered anyway: *would* an untrusted workspace refuse this?
    ///
    /// `None` for every pack that touches nothing reserved, which is nearly
    /// all of them.
    #[must_use]
    pub fn untrusted_tier_refusal(&self) -> Option<(u32, String)> {
        tcl_spectcl::provenance_violation(&self.pack, Tier::Workspace)
    }

    /// Where a command was declared: the line of its `command` statement, the
    /// file when one is known, and whether it was **expanded** from a program
    /// rather than written out.
    ///
    /// The expansion flag is derived, not stored: a literal declaration has a
    /// `command NAME { … }` statement in this very document, and one a loop
    /// registered does not. That is the fact a surface needs to label a
    /// browsable command "expanded from `bench-command`, line 12" — and the
    /// reason such a command must not be edited in place (E-R12: a programmed
    /// pack is never rewritten by the studio).
    #[must_use]
    pub fn declaration_site(&self, name: &str) -> Option<DeclarationSite> {
        let command = self.pack.command(name)?;
        Some(DeclarationSite {
            line: command.line,
            file: (!command.file.as_os_str().is_empty()).then(|| command.file.clone()),
            expanded: command_span(&self.source, name).is_none(),
        })
    }

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
            .any(|(name, _)| command_span(&self.source, name).is_none())
        {
            return Some(Programmed::Expanded);
        }
        (!self.straight_line()).then_some(Programmed::NonCanonicalStatement)
    }

    /// Whether every top-level statement of the `speclib` body is one of the
    /// registration calls the snapshot recorded, in the same order.
    ///
    /// True for every canonical pack — the loader records the file's own
    /// statements — and false the moment a statement runs rather than
    /// registers.
    fn straight_line(&self) -> bool {
        let Some(body) = pack_body(&self.source) else {
            // Not a pack at all: nothing was registered, and there is nothing
            // programmed about a document that declares nothing.
            return self.pack.registrations.is_empty();
        };
        let statements = segments(&self.source[body]);
        statements.len() == self.pack.registrations.len()
            && statements
                .iter()
                .zip(&self.pack.registrations)
                .all(|(statement, registration)| {
                    statement.words.first().map(String::as_str) == Some(registration.word())
                        && statement.words.get(1).map_or("", String::as_str) == registration.arg(1)
                })
    }

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
        let keep: Vec<(String, Draft)> = self
            .patch
            .as_ref()
            .map(|patch| {
                patch
                    .drafts
                    .iter()
                    .filter(|(key, _)| key != name)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
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
    fn patch_command(&mut self, name: &str, draft: &Draft) -> Write {
        let written = draft
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(name)
            .to_owned();
        let mut commands: Vec<(String, Draft)> = self
            .patch
            .as_ref()
            .map(|patch| patch.drafts.clone())
            .unwrap_or_default();
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

    /// An empty pack document named `pack_name` — the "start a new library"
    /// state, already valid DSL.
    #[must_use]
    pub fn empty(pack_name: &str) -> Self {
        Self::from_source(render_spectcl::render_pack(&[], pack_name))
    }

    /// The authoritative document.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The `speclib` name the document declares.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.pack.name
    }

    /// The DSL vocabulary version the document declares.
    #[must_use]
    pub fn dsl_version(&self) -> &str {
        &self.pack.dsl_version
    }

    /// Version used when rendering this store back to DSL.  A malformed or
    /// non-pack source has no declared vocabulary; a newly materialised pack
    /// must use the renderer's current vocabulary rather than emitting an
    /// invalid empty `speclib NAME { … }` header.
    #[must_use]
    fn render_version(&self) -> &str {
        if self.pack.dsl_version.is_empty() {
            render_spectcl::DSL_VERSION
        } else {
            &self.pack.dsl_version
        }
    }

    /// The human-readable name the document declares (`display_name {IEEE
    /// 1801 UPF}`), if any — what a surface calls the library rather than the
    /// `speclib` word a script types.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.pack.display_name.as_deref()
    }

    /// The file extensions the document declares its language is written
    /// under, in declaration order.
    #[must_use]
    pub fn file_extensions(&self) -> &[FileExtension] {
        &self.pack.file_extensions
    }

    /// Everything the loader dropped on the way in.
    #[must_use]
    pub fn notices(&self) -> &[Notice] {
        &self.pack.notices
    }

    /// The declared commands, in declaration order, with their derived drafts.
    #[must_use]
    pub fn commands(&self) -> &[(String, Draft)] {
        &self.drafts
    }

    /// The derived draft for `name`.
    #[must_use]
    pub fn draft(&self, name: &str) -> Option<&Draft> {
        self.drafts
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, d)| d)
    }

    /// Whether the declaration for `name` claimed a shipped name with
    /// `-override`.
    #[must_use]
    pub fn overrides_shipped(&self, name: &str) -> bool {
        self.pack.command(name).is_some_and(|c| c.overrides_shipped)
    }

    /// The notices whose context names `name`.
    #[must_use]
    pub fn notices_for(&self, name: &str) -> Vec<&Notice> {
        let prefix = format!("command {name}");
        self.pack
            .notices
            .iter()
            .filter(|n| n.context == prefix || n.context.starts_with(&format!("{prefix} /")))
            .collect()
    }

    /// This document as the one-file [`PackSet`] the installer takes.
    ///
    /// The studio edits a pack **in a browser**, where there is no file to
    /// discover, so the set is assembled here rather than by
    /// `tcl_spectcl::pack::load` — same `MergedPack`, same commands, at the
    /// workspace tier a private pack really loads from. Everything downstream
    /// (installation, the collision policy, hook specialisation) is then the
    /// shipped code path, unchanged.
    ///
    /// Load notices are the store's own to report, so the set carries none.
    ///
    /// A standing patch pack (E-R12) is the set's **second** pack, at
    /// [`Tier::StudioOverride`] and *after* the base — which is the whole of
    /// the layering. `tcl_spectcl::install::install_into` inserts in pack
    /// order and `tcl_spectcl::pack::installs_over` admits a later claim only
    /// when it says `-override`, which every patch declaration does, so the
    /// patch wins by the shipped policy rather than by anything here.
    #[must_use]
    pub fn pack_set(&self) -> PackSet {
        let name = if self.pack.name.is_empty() {
            "pack".to_owned()
        } else {
            self.pack.name.clone()
        };
        let mut packs = vec![merged(&name, &self.pack, Tier::Workspace)];
        if let Some(patch) = &self.patch {
            packs.push(merged(
                &self.patch_name(),
                &patch.pack,
                Tier::StudioOverride,
            ));
        }
        PackSet {
            packs,
            notices: Vec::new(),
            key: self.overlay_key(),
        }
    }

    /// The identity this document installs under in the registry cache.
    ///
    /// Content-derived, exactly as `tcl_spectcl::pack::load`'s key is: the same
    /// document resolves to the same installed registry however many times it
    /// is asked for, and an edit is a new entry. `0` is reserved by the
    /// installer for "no packs", so a document that hashes there is nudged.
    #[must_use]
    pub fn overlay_key(&self) -> u64 {
        if self.pack.commands.is_empty() && self.patch.is_none() {
            return 0;
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.source.hash(&mut hasher);
        // The patch is part of the installed world, so it is part of that
        // world's identity: adopting or dropping one is a new registry.
        self.patch_source().hash(&mut hasher);
        match hasher.finish() {
            0 => 1,
            key => key,
        }
    }

    /// The whole document, re-rendered from the derived drafts.
    ///
    /// This is what "canonical" means for a pack: the exact text the renderer
    /// would emit for these commands. It is offered as an explicit action, not
    /// applied behind the author's back.
    #[must_use]
    pub fn canonical(&self) -> String {
        let drafts: Vec<Draft> = self.drafts.iter().map(|(_, d)| d.clone()).collect();
        let flags: BTreeSet<&str> = self
            .pack
            .commands
            .iter()
            .filter(|c| c.overrides_shipped)
            .map(|c| c.spec.name)
            .collect();
        with_override_flags(
            &render_spectcl::render_pack_with_version(&drafts, self.name(), self.render_version()),
            &flags,
        )
    }

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
        // A loss is a property that vanished, or one that used to be a real
        // declaration and is now only a `-native` placeholder.
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
        if let Some(span) = command_span(&self.source, name) {
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
        loader::evaluate_pack(&probe).notices.iter().any(|notice| {
            notice.message.contains(" is SpecTcl ")
                && notice
                    .message
                    .contains(" vocabulary, but this pack declares vocabulary ")
        })
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
        let version_span = segments(&self.source)
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
        // inconsistency the loader reports per site (#1627). A draft that
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
    /// Anything nested deeper than that — a hook hanging off a subcommand — is
    /// beyond this pass, and is reported through [`Write::dropped`] instead of
    /// being lost in silence.
    fn carry_forward(&self, name: &str, block: &str) -> String {
        let Some((_, old_body)) = find_command(&self.source, pack_body(&self.source), name) else {
            return block.to_owned();
        };
        let old = &self.source[old_body];
        let old_statements = segments(old);
        // The rendered block is a pack *body*: the command statement plus any
        // value tables it needed, with no `speclib` wrapper.
        let Some((_, new_body)) = find_command(block, Some(0..block.len()), name) else {
            return block.to_owned();
        };
        let written = segments(&block[new_body.clone()]);

        let mut reclaim: BTreeSet<&str> = BTreeSet::new();
        for statement in &old_statements {
            let Some(word) = statement.words.first() else {
                continue;
            };
            let rendered = written.iter().find(|s| s.words.first() == Some(word));
            let lost = match rendered {
                None => true,
                Some(rendered) => {
                    is_native_placeholder(rendered) && !is_native_placeholder(statement)
                }
            };
            if lost {
                reclaim.insert(word.as_str());
            }
        }
        if reclaim.is_empty() {
            return block.to_owned();
        }

        // Drop the placeholders, back to front so the earlier spans stay valid.
        let mut body_text = block[new_body.clone()].to_owned();
        for statement in written.iter().rev() {
            if statement
                .words
                .first()
                .is_some_and(|word| reclaim.contains(word.as_str()))
            {
                body_text.replace_range(trimmed_span(&body_text, statement.span.clone()), "");
            }
        }
        // Then append the author's own, verbatim.
        for statement in &old_statements {
            if statement
                .words
                .first()
                .is_some_and(|word| reclaim.contains(word.as_str()))
            {
                body_text.push_str("\n    ");
                body_text.push_str(old[statement.span.clone()].trim_end());
                body_text.push('\n');
            }
        }

        let mut out = block.to_owned();
        out.replace_range(new_body, &body_text);
        out
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
        let Some((_, body)) = find_command(&self.source, pack_body(&self.source), name) else {
            return out;
        };
        for statement in segments(&self.source[body.clone()]) {
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
            for nested in segments(&self.source[inner]) {
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
            let span = command_span(&self.source, name)?;
            text.replace_range(span, block);
        } else {
            let body = pack_body(&self.source)?;
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

/// One loaded pack as the one-file [`MergedPack`] the installer takes.
///
/// The studio edits packs **in a browser**, where there is no file to
/// discover, so the merged shape is assembled here rather than by
/// `tcl_spectcl::pack::load` — same commands, same declarations, at the tier
/// the pack really layers from.
fn merged(name: &str, pack: &Pack, tier: Tier) -> MergedPack {
    MergedPack {
        name: name.to_owned(),
        dsl_version: pack.dsl_version.clone(),
        tier,
        files: vec![std::path::PathBuf::from(format!("{name}.tclspec"))],
        display_name: pack.display_name.clone(),
        file_extensions: pack.file_extensions.clone(),
        ambient_packages: pack.ambient_packages.clone(),
        environments: pack.environments.clone(),
        dialects: pack.dialects.clone(),
        commands: pack.commands.clone(),
    }
}

// ---------------------------------------------------------------------------
// The resolution facade
// ---------------------------------------------------------------------------

/// Where a name's *effective* definition comes from once the collision policy
/// has been applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Only the shipped registry defines it.
    Builtin,
    /// Only the pack defines it.
    Pack,
    /// Both define it and the pack said `-override`; the pack's wins.
    Override,
    /// Both define it and the pack did not say `-override`; the shipped one
    /// wins and the pack's declaration is inert.
    Shadowed,
}

impl Origin {
    /// The word the report uses.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Pack => "pack",
            Self::Override => "override",
            Self::Shadowed => "shadowed",
        }
    }

    /// Whether the pack's declaration is the one an editor would use.
    #[must_use]
    pub fn pack_wins(self) -> bool {
        matches!(self, Self::Pack | Self::Override)
    }
}

/// The one merged world every surface queries: built-ins with the pack laid
/// over them, collision policy applied.
pub struct Resolution<'a> {
    builtins: Builtins,
    store: &'a PackStore,
}

impl<'a> Resolution<'a> {
    /// Merge `store` over `builtins`.
    #[must_use]
    pub fn new(builtins: Builtins, store: &'a PackStore) -> Self {
        Self { builtins, store }
    }

    /// The built-ins half.
    #[must_use]
    pub fn builtins(&self) -> Builtins {
        self.builtins
    }

    /// The user-definitions half.
    #[must_use]
    pub fn store(&self) -> &PackStore {
        self.store
    }

    /// The **live registry** this merged world resolves to: the shipped one
    /// for the dialect with the pack installed over it.
    ///
    /// This is the seam that makes the Test tab honest. Rather than the studio
    /// merging two drafts and calling that "what an editor would see", the
    /// pack goes through `tcl_spectcl::install::registry_with_packs` — the very
    /// function the language server calls at workspace init — so the collision
    /// policy, the vendor package gate, and hook specialisation are all the
    /// shipped behaviour rather than a re-implementation of it.
    ///
    /// Cached on `(profile, key)` inside the registry cache, so repeated
    /// queries against an unchanged document cost a lookup. An edit is a new
    /// key and therefore a new registry: callers should debounce, which is why
    /// the Test tab re-analyses on a settle timer rather than per keystroke.
    ///
    /// An owning handle: an edit supersedes the previous key, and the
    /// generation behind it is freed once the last reader lets go.
    #[must_use]
    pub fn registry(&self) -> Arc<CommandRegistry> {
        tcl_spectcl::install::registry_with_packs(
            crate::environment::profile_for_dialect(self.builtins.dialect()),
            &self.store.pack_set(),
        )
    }

    /// The pack overlay identity [`Self::registry`] installed under — the value
    /// `Analyser::with_pack_overlay` takes. `0` when the pack declares nothing.
    #[must_use]
    pub fn overlay_key(&self) -> u64 {
        self.store.overlay_key()
    }

    /// Where `name` resolves.
    ///
    /// This is `tcl_spectcl::pack::installs_over`'s rule — *shipped wins unless
    /// the pack says `-override`* — restated as a four-way answer so a surface
    /// can show the author which of the two definitions is live.
    #[must_use]
    pub fn origin(&self, name: &str) -> Option<Origin> {
        let in_pack = self.store.draft(name).is_some();
        let in_builtins = self.builtins.contains(name);
        match (in_pack, in_builtins) {
            (false, false) => None,
            (false, true) => Some(Origin::Builtin),
            (true, false) => Some(Origin::Pack),
            (true, true) if self.store.overrides_shipped(name) => Some(Origin::Override),
            (true, true) => Some(Origin::Shadowed),
        }
    }

    /// The merged view of `name`: which definition is live, and both of them
    /// when there are two.
    #[must_use]
    pub fn view(&self, name: &str) -> Option<Value> {
        let origin = self.origin(name)?;
        // The **effective** pack draft: a standing patch override is what the
        // registry answers with, so it is what the form must show (E-R12).
        let pack = self.store.effective_draft(name);
        let builtin = self.builtins.draft(name);
        let effective = if origin.pack_wins() {
            pack.cloned()
        } else {
            builtin.clone()
        };
        Some(json!({
            "name": name,
            "origin": origin.key(),
            "editable": origin.pack_wins() || pack.is_some(),
            "dialect": self.builtins.dialect(),
            // Prefixed: in a *command* view every unprefixed key is the
            // command's, and `pack` is already this command's pack draft.
            "pack_display_name": self.store.display_name(),
            "pack_file_extensions": file_extensions_json(self.store.file_extensions()),
            "override": self.store.overrides_shipped(name),
            // Whether what is shown comes from a `StudioOverride` patch pack
            // rather than from the document itself.
            "patched": self.store.patch_draft(name).is_some(),
            "effective": effective,
            "pack": pack,
            "builtin": builtin,
            "notices": notices_json(&self.store.notices_for(name)),
        }))
    }

    /// One row per pack command, with the state a sidebar shows at a glance.
    #[must_use]
    pub fn pack_index(&self) -> Vec<Value> {
        let defaults = draft::default_command_draft();
        self.store
            .commands()
            .iter()
            .map(|(name, d)| {
                let origin = self.origin(name).unwrap_or(Origin::Pack);
                let notices = self.store.notices_for(name);
                let site = self.store.declaration_site(name);
                json!({
                    "name": name,
                    "origin": origin.key(),
                    "override": self.store.overrides_shipped(name),
                    // Where the declaration is, and whether the author can
                    // edit it at all: an expanded command is a program's
                    // output (E-R12 — the studio never rewrites a programmed
                    // pack), so a surface labels it rather than opening it.
                    "declared_at": site.as_ref().map(|site| json!({
                        "line": site.line,
                        "file": site.file.as_ref().map(|f| f.display().to_string()),
                        "expanded": site.expanded,
                    })),
                    // A standing patch override (E-R12): the row a surface
                    // labels "patched" and offers a "revert" on.
                    "patched": self.store.patch_draft(name).is_some(),
                    "summary": summary_of(d),
                    "fields_set": fields_set(d, &defaults).len(),
                    "subcommands": count_of(d, "subcommands"),
                    "options": count_of(d, "options"),
                    "notices": notices.len(),
                    "unrenderable": d
                        .get(draft::UNRENDERABLE_KEY)
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len),
                })
            })
            .collect()
    }

    /// Every name where the pack and the shipped registry both speak.
    #[must_use]
    pub fn collisions(&self) -> Vec<Value> {
        self.store
            .commands()
            .iter()
            .filter_map(|(name, _)| {
                let origin = self.origin(name)?;
                if !matches!(origin, Origin::Override | Origin::Shadowed) {
                    return None;
                }
                Some(json!({
                    "name": name,
                    "override": matches!(origin, Origin::Override),
                    "effect": if matches!(origin, Origin::Override) {
                        "pack-spec-wins"
                    } else {
                        "shipped-spec-wins"
                    },
                    "reason": if matches!(origin, Origin::Override) {
                        format!("`{name}` replaces the shipped command of that name (`-override`)")
                    } else {
                        format!(
                            "`{name}` is already a shipped command; the shipped spec wins \
                             (declare it `-override` to replace it)"
                        )
                    },
                }))
            })
            .collect()
    }

    /// The whole-store view a sidebar and a status bar are built from.
    #[must_use]
    pub fn store_view(&self) -> Value {
        let collisions = self.collisions();
        let shadowed = collisions
            .iter()
            .filter(|c| c["effect"] == "shipped-spec-wins")
            .count();
        json!({
            "pack": self.store.name(),
            "display_name": self.store.display_name(),
            "file_extensions": file_extensions_json(self.store.file_extensions()),
            "dsl_version": self.store.dsl_version(),
            "dialect": self.builtins.dialect(),
            // Two facts only the evaluation loader can report: whether this
            // surface is one analysis target's answer (E-R1), and what an
            // untrusted tier would refuse the document for (E-R2).
            "target_dependent": self.store.target_dependent(),
            "untrusted_tier_refusal": self
                .store
                .untrusted_tier_refusal()
                .map(|(line, why)| json!({ "line": line, "reason": why })),
            // E-R12: whether this document is a program the studio must not
            // rewrite, and what patch pack currently stands over it.
            "programmed": self.store.programmed().map(|why| json!({
                "why": why.key(),
                "reason": why.reason(),
            })),
            "patch": self.store.patch_source().map(|source| json!({
                "pack": self.store.patch_name(),
                "tier": Tier::StudioOverride.label(),
                "source": source,
                "untrusted_tier_refusal": self
                    .store
                    .patch_untrusted_tier_refusal()
                    .map(|(line, why)| json!({ "line": line, "reason": why })),
            })),
            "standing_overrides": standing_overrides_json(&self.store.standing_overrides()),
            "commands": self.pack_index(),
            "notices": notices_json(&self.store.notices().iter().collect::<Vec<_>>()),
            "collisions": collisions,
            "summary": {
                "commands": self.store.commands().len(),
                "notices": self.store.notices().len(),
                "collisions": collisions.len(),
                "shadowed_commands": shadowed,
                "standing_overrides": self.store.standing_overrides().len(),
                "bytes": self.store.source().len(),
            },
        })
    }

    /// The structured validation report — the studio's twin of `tcl spec
    /// check` and the MCP server's `spectcl_check`.
    ///
    /// Same three questions: what did the pack declare, what did the loader
    /// drop, and what collides with a shipped spec.
    #[must_use]
    pub fn validate(&self) -> Value {
        let defaults = draft::default_command_draft();
        let sub_defaults = draft::default_subcommand_draft();
        let commands: Vec<Value> = self
            .store
            .commands()
            .iter()
            .map(|(name, d)| {
                let subcommands: Vec<Value> = d
                    .get("subcommands")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_object)
                    .map(|sub| {
                        json!({
                            "name": sub.get("name").cloned().unwrap_or(Value::Null),
                            "fields_set": fields_set(sub, &sub_defaults),
                        })
                    })
                    .collect();
                let hooks: Vec<Value> = self
                    .store
                    .pack
                    .command(name)
                    .map(|c| c.hooks.iter().map(hook_json).collect())
                    .unwrap_or_default();
                json!({
                    "name": name,
                    "origin": self.origin(name).map_or("pack", Origin::key),
                    "override": self.store.overrides_shipped(name),
                    "fields_set": fields_set(d, &defaults),
                    "unrenderable": d.get(draft::UNRENDERABLE_KEY).cloned().unwrap_or(Value::Null),
                    "subcommands": subcommands,
                    "clause_grammar": self
                        .store
                        .pack
                        .command(name)
                        .is_some_and(|c| c.clause_grammar.is_some()),
                    "hooks": hooks,
                })
            })
            .collect();

        let mut report = self.store_view();
        if let Some(map) = report.as_object_mut() {
            map.insert("commands".to_owned(), Value::Array(commands));
            let hooks: usize = self.store.pack.commands.iter().map(|c| c.hooks.len()).sum();
            if let Some(summary) = map.get_mut("summary").and_then(Value::as_object_mut) {
                summary.insert("hooks".to_owned(), json!(hooks));
            }
        }
        report
    }
}

// ---------------------------------------------------------------------------
// JSON helpers
// ---------------------------------------------------------------------------

/// The pack's `file_extension` rows as a surface reads them: what the file
/// type is called and where a file of it routes, minus the declaring line
/// (which only the DSL pane's notices need).
/// The standing patch overrides as a surface reads them: which command, from
/// which patch, over which base declaration (design E §15.2 — "`spec check`
/// reports standing overrides so they cannot rot silently").
#[must_use]
pub fn standing_overrides_json(rows: &[StandingOverride]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                json!({
                    "command": row.command,
                    "patch_pack": row.patch_pack,
                    "base_pack": row.base_pack,
                    "base_line": row.base_line,
                    "base_expanded": row.base_expanded,
                })
            })
            .collect(),
    )
}

fn file_extensions_json(rows: &[FileExtension]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                json!({
                    "extension": row.extension,
                    "display_name": row.display_name,
                    "dialect": row.dialect,
                })
            })
            .collect(),
    )
}

fn notices_json(notices: &[&Notice]) -> Value {
    Value::Array(
        notices
            .iter()
            .map(|n| json!({ "line": n.line, "context": n.context, "reason": n.message }))
            .collect(),
    )
}

/// The draft keys `d` says something about — [`fields_set`] against a
/// brand-new command's defaults, for callers outside this module.
#[must_use]
pub fn fields_set_of(d: &Draft) -> Vec<String> {
    fields_set(d, &draft::default_command_draft())
}

/// The draft keys this declaration actually said something about, sorted.
///
/// Two kinds, because the draft model has two ways of saying "set": a key
/// whose *value* differs from a brand-new command's, and a key listed under
/// [`draft::UNRENDERABLE_KEY`] — precisely the fields that *are* set but whose
/// defining Rust expression could not be recovered, every hook among them.
/// `name` is excluded because it always differs and is reported separately.
fn fields_set(d: &Draft, defaults: &Draft) -> Vec<String> {
    let mut set: BTreeSet<String> = d
        .iter()
        .filter(|(key, _)| key.as_str() != "name" && key.as_str() != draft::UNRENDERABLE_KEY)
        .filter(|(key, value)| defaults.get(key.as_str()) != Some(*value))
        .map(|(key, _)| key.to_owned())
        .collect();
    set.extend(
        d.get(draft::UNRENDERABLE_KEY)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned),
    );
    set.into_iter().collect()
}

/// The hover summary a sidebar row shows, if the draft has one.
fn summary_of(d: &Draft) -> String {
    d.get("hover")
        .and_then(Value::as_object)
        .and_then(|h| h.get("summary"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn count_of(d: &Draft, key: &str) -> usize {
    d.get(key).and_then(Value::as_array).map_or(0, Vec::len)
}

/// The schema spelling of a hook family — the word an author writes.
fn family_key(family: loader::HookFamily) -> &'static str {
    use loader::HookFamily as F;
    match family {
        F::ArgRoleResolver => "arg_role_resolver",
        F::CommandPrefixResolver => "command_prefix_resolver",
        F::ScriptTimingResolver => "script_timing_resolver",
        F::ConstFold => "const_fold",
        F::ConstFoldVersioned => "const_fold_versioned",
        F::TaintSinkGate => "taint_sink_gate",
        F::ContextGate => "context_gate",
        F::LiteralArgumentValidator => "literal_argument_validator",
        F::ClauseShapeCheck => "clause_shape_check",
        F::Constraints => "constraints",
        F::OptionArity => "-arity-hook",
    }
}

fn hook_json(hook: &loader::HookDecl) -> Value {
    let (kind, detail) = match &hook.source {
        HookSource::Body { params, .. } => ("body", params.join(" ")),
        HookSource::Native { id } => ("native", id.clone()),
        HookSource::Derived { keyword } => ("derived", keyword.clone()),
    };
    let owner = match &hook.owner {
        HookOwner::Command => "command".to_owned(),
        HookOwner::Subcommand(name) => format!("subcommand {name}"),
        HookOwner::Option { subcommand, option } => match subcommand {
            Some(sub) => format!("subcommand {sub} option {option}"),
            None => format!("option {option}"),
        },
    };
    json!({
        "owner": owner,
        "field": hook.field,
        "family": family_key(hook.family),
        "source": kind,
        "detail": detail,
    })
}

// ---------------------------------------------------------------------------
// The CST side: finding a statement's bytes
// ---------------------------------------------------------------------------

/// The byte range of the `speclib` body's **contents** — everything between
/// its braces.
///
/// `None` when the document has no `speclib` statement with a braced body,
/// which is the "not a pack yet" case every caller has to handle anyway.
#[must_use]
pub fn pack_body(source: &str) -> Option<std::ops::Range<usize>> {
    for statement in segments(source) {
        if statement.words.first().map(String::as_str) != Some("speclib") {
            continue;
        }
        // `speclib <name> <version> { … }` — the body is the fourth word.
        let word = statement.spans.get(3)?;
        return Some(word.inner.clone());
    }
    None
}

/// The byte range of the `command NAME …` statement inside the pack body.
///
/// The whole statement, from its first word to its closing brace — which is
/// what a splice replaces.
#[must_use]
pub fn command_span(source: &str, name: &str) -> Option<std::ops::Range<usize>> {
    find_command(source, pack_body(source), name).map(|(statement, _)| statement)
}

/// The `command NAME …` statement inside `region`, as `(whole statement, body
/// contents)`, both in `source`'s own byte offsets.
///
/// `region` is the pack body for a whole document, or the whole string for a
/// bare pack body such as a freshly rendered block — the two callers of this,
/// and the reason it takes the region rather than deriving it.
fn find_command(
    source: &str,
    region: Option<std::ops::Range<usize>>,
    name: &str,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let region = region?;
    let base = region.start;
    for statement in segments(&source[region]) {
        if statement.words.first().map(String::as_str) != Some("command") {
            continue;
        }
        if statement.words.get(1).map(String::as_str) != Some(name) {
            continue;
        }
        // `command NAME { … }`, or `command NAME -override { … }`: the body is
        // the last word, and it has to actually be a braced one.
        let word = statement.spans.last()?;
        if statement.words.len() < 3
            || source.as_bytes().get(base + word.outer.start) != Some(&b'{')
        {
            return None;
        }
        return Some((
            base + statement.span.start..base + statement.span.end,
            base + word.inner.start..base + word.inner.end,
        ));
    }
    None
}

/// One statement, with the byte ranges a splice needs.
struct Segmented {
    words: Vec<String>,
    span: std::ops::Range<usize>,
    spans: Vec<WordSpan>,
}

/// A word's byte ranges: `outer` includes any `{ }` / `" "` delimiters,
/// `inner` is the contents the loader reads as the word's text.
struct WordSpan {
    outer: std::ops::Range<usize>,
    inner: std::ops::Range<usize>,
}

/// Segment `source` into statements, the same way the loader does — same
/// lexer, same segmenter, so what this sees and what the loader reads are the
/// same statements.
fn segments(source: &str) -> Vec<Segmented> {
    let source_map = SourceMap::new(source);
    let (document, _warnings) = build_document(source, LexerConfig::default());
    segments_from_document(document, &source_map)
        .into_iter()
        .map(|segment| {
            let spans = segment
                .argv
                .iter()
                .map(|token| {
                    let start = token.span.start() as usize;
                    let end = token.span.end() as usize;
                    let lead = usize::from(token.content_offset);
                    let inner = if lead > 0 && end > start + lead {
                        start + lead..end - 1
                    } else {
                        start..end
                    };
                    WordSpan {
                        outer: start..end,
                        inner,
                    }
                })
                .collect();
            Segmented {
                words: segment.texts.clone(),
                span: segment.span.start() as usize..segment.span.end() as usize,
                spans,
            }
        })
        .collect()
}

/// Whether a statement is the renderer's `field -native ID` stand-in rather
/// than a real declaration.
///
/// A genuinely native hook is written that way too, which is why the *pair* of
/// before-and-after matters and not this predicate alone: a body degrading into
/// a placeholder is a loss, a placeholder staying a placeholder is not.
fn is_native_placeholder(statement: &Segmented) -> bool {
    statement.words.get(1).map(String::as_str) == Some("-native")
}

/// Widen `span` to swallow the blank line a removed statement leaves behind.
fn trimmed_span(source: &str, span: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let mut end = span.end;
    let bytes = source.as_bytes();
    while end < bytes.len() && (bytes[end] == b'\n' || bytes[end] == b' ' || bytes[end] == b'\t') {
        end += 1;
    }
    let mut start = span.start;
    while start > 0 && (bytes[start - 1] == b' ' || bytes[start - 1] == b'\t') {
        start -= 1;
    }
    start..end
}

/// Re-write every `command <name> {` header in `text` whose name is in
/// `flagged` to carry `-override`.
///
/// The renderer has no spelling for `-override` — it renders a *draft*, and
/// the flag lives on the declaration rather than on `CommandSpec` — so this
/// is where a re-render puts it back. Header lines are matched whole and at
/// column 0, which is where the renderer puts them and where a command body's
/// own (four-space indented) content never is.
fn with_override_flags(text: &str, flagged: &BTreeSet<&str>) -> String {
    if flagged.is_empty() {
        return text.to_owned();
    }
    let headers: BTreeMap<String, String> = flagged
        .iter()
        .map(|name| {
            let word = render_spectcl::header_word(name);
            (
                format!("command {word} {{"),
                format!("command {word} -override {{"),
            )
        })
        .collect();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut out = String::with_capacity(text.len() + flagged.len() * 10);
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match headers.get(line) {
            Some(replacement) if !seen.contains(line) => {
                seen.insert(line);
                out.push_str(replacement);
            }
            _ => out.push_str(line),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACK: &str = r"# A hand-written banner the studio must never eat.

speclib demo 1.0 {

# Why greet exists, in the author's own words.
command greet {
    arity 1..2
    hover {
        summary {Say hello.}
    }
}

# And a second one, with its own comment.
command farewell {
    arity 1
}

}
";

    #[test]
    fn the_document_is_the_store_and_drafts_are_derived_from_it() {
        let store = PackStore::from_source(PACK);
        assert_eq!(store.name(), "demo");
        assert_eq!(store.dsl_version(), "1.0");
        let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["greet", "farewell"]);
        assert_eq!(store.source(), PACK, "the source is kept verbatim");
        assert_eq!(
            store.draft("greet").and_then(|d| d.get("name")),
            Some(&json!("greet"))
        );
    }

    #[test]
    fn the_views_carry_what_the_pack_calls_itself_and_its_files() {
        let source = "speclib upf 1.1 {\ndisplay_name {IEEE 1801 UPF}\n\
file_extension upf -name {Unified Power Format} -dialect synopsys-eda-tcl\n\
command add_parameter {\narity 1..\n}\n}\n";
        let store = PackStore::from_source(source);
        let resolution = Resolution::new(Builtins::for_dialect("synopsys-eda-tcl"), &store);

        let view = resolution.store_view();
        assert_eq!(view["display_name"], json!("IEEE 1801 UPF"));
        assert_eq!(
            view["file_extensions"],
            json!([{
                "extension": "upf",
                "display_name": "Unified Power Format",
                "dialect": "synopsys-eda-tcl",
            }])
        );

        let command = resolution
            .view("add_parameter")
            .expect("the pack's command");
        assert_eq!(command["pack_display_name"], json!("IEEE 1801 UPF"));
        assert_eq!(command["pack_file_extensions"], view["file_extensions"]);
    }

    #[test]
    fn a_pack_that_names_neither_reports_neither() {
        let store = PackStore::from_source(PACK);
        let view = Resolution::new(Builtins::for_dialect("tcl9.0"), &store).store_view();
        assert_eq!(view["display_name"], Value::Null);
        assert_eq!(view["file_extensions"], json!([]));
    }

    #[test]
    fn a_command_span_covers_exactly_its_statement() {
        let span = command_span(PACK, "farewell").expect("farewell is declared");
        assert_eq!(&PACK[span], "command farewell {\n    arity 1\n}");
    }

    #[test]
    fn the_pack_body_span_is_the_contents_of_the_speclib_braces() {
        let body = pack_body(PACK).expect("a speclib body");
        let text = &PACK[body];
        assert!(
            text.trim_start().starts_with("# Why greet exists"),
            "{text}"
        );
        assert!(text.trim_end().ends_with('}'), "{text}");
        assert!(!text.contains("speclib demo"));
    }

    #[test]
    fn a_form_edit_splices_and_leaves_every_other_byte_alone() {
        let mut store = PackStore::from_source(PACK);
        let mut edited = store.draft("greet").expect("greet").clone();
        edited.insert("return_type".to_owned(), json!("String"));

        assert_eq!(
            store.set_command("greet", &edited, false).how,
            WriteBack::Spliced
        );

        let text = store.source();
        assert!(
            text.contains("# A hand-written banner the studio must never eat."),
            "{text}"
        );
        assert!(
            text.contains("# And a second one, with its own comment."),
            "{text}"
        );
        assert!(
            text.contains("command farewell {\n    arity 1\n}"),
            "{text}"
        );
        assert!(text.contains("return_type"), "{text}");
        assert_eq!(
            store.draft("greet").and_then(|d| d.get("return_type")),
            Some(&json!("String"))
        );
        // Untouched neighbours keep their drafts byte for byte.
        assert_eq!(
            store.draft("farewell").and_then(|d| d.get("arity")),
            PackStore::from_source(PACK)
                .draft("farewell")
                .and_then(|d| d.get("arity"))
        );
    }

    #[test]
    fn the_comment_inside_the_edited_command_is_the_documented_casualty() {
        // The banner above the command survives (it is not part of the
        // statement); a comment *inside* the braces does not. That is the
        // limitation this module's header states, pinned as a test so it
        // cannot regress into a surprise.
        let source = "speclib demo 1.0 {\ncommand greet {\n    # inside\n    arity 1\n}\n}\n";
        let mut store = PackStore::from_source(source);
        let mut edited = store.draft("greet").expect("greet").clone();
        edited.insert("return_type".to_owned(), json!("String"));
        store.set_command("greet", &edited, false);
        assert!(!store.source().contains("# inside"), "{}", store.source());
    }

    /// The bug this exists to stop: a form edit deleting the author's Tcl.
    ///
    /// A hook body loads into a function pointer, so the draft the form holds
    /// records only "set, expression not recovered" — render that back and the
    /// body is gone. The write-back has to carry the original statement across
    /// verbatim.
    #[test]
    fn a_form_edit_does_not_delete_a_hook_body_it_cannot_re_render() {
        let source = "speclib demo 1.0 {\n\
                      command greet {\n\
                      \x20   arity 1..2\n\
                      \x20   arg_role_resolver {words ctx} {\n\
                      \x20       role 1 Body\n\
                      \x20   }\n\
                      }\n}\n";
        let mut store = PackStore::from_source(source);
        let mut edited = store.draft("greet").expect("greet").clone();
        edited.insert("return_type".to_owned(), json!("String"));

        let write = store.set_command("greet", &edited, false);
        assert_eq!(write.how, WriteBack::Spliced);
        assert_eq!(write.dropped, Vec::<String>::new(), "{}", store.source());
        assert!(
            store.source().contains("arg_role_resolver {words ctx} {"),
            "the hook body must survive a form edit:\n{}",
            store.source()
        );
        assert!(
            store.source().contains("role 1 Body"),
            "the hook body's own text, verbatim:\n{}",
            store.source()
        );
        assert!(store.source().contains("return_type"), "{}", store.source());
        // And it is still a *live* hook after the round trip, not just text.
        assert!(
            store
                .draft("greet")
                .and_then(|d| d.get(draft::UNRENDERABLE_KEY))
                .and_then(Value::as_array)
                .is_some_and(|lost| lost.iter().any(|k| k == "arg_role_resolver")),
            "the reloaded command should carry the resolver again"
        );
    }

    /// What carry-forward cannot reach is *named*, not silently dropped.
    #[test]
    fn a_loss_carry_forward_cannot_reach_is_reported() {
        let source = "speclib demo 1.0 {\n\
                      command greet {\n\
                      \x20   subcommand hi {\n\
                      \x20       arity 1\n\
                      \x20       const_fold {words ctx} {\n\
                      \x20           result hello\n\
                      \x20       }\n\
                      \x20   }\n\
                      }\n}\n";
        let mut store = PackStore::from_source(source);
        let mut edited = store.draft("greet").expect("greet").clone();
        edited.insert("return_type".to_owned(), json!("String"));

        let write = store.set_command("greet", &edited, false);
        assert_eq!(
            write.dropped,
            vec!["subcommand hi / const_fold".to_owned()],
            "a nested hook body the splice cannot carry must be reported:\n{}",
            store.source()
        );
    }

    #[test]
    fn renaming_a_command_edits_it_where_it_stands() {
        let mut store = PackStore::from_source(PACK);
        let mut edited = store.draft("greet").expect("greet").clone();
        edited.insert("name".to_owned(), json!("salute"));

        assert_eq!(
            store.set_command("greet", &edited, false).how,
            WriteBack::Spliced
        );
        let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec!["salute", "farewell"],
            "a rename must not move the command to the end of the file"
        );
        assert!(
            store.source().contains("# Why greet exists"),
            "{}",
            store.source()
        );
    }

    #[test]
    fn a_new_command_is_appended_to_the_pack_body() {
        let mut store = PackStore::from_source(PACK);
        let mut fresh = draft::default_command_draft();
        fresh.insert("name".to_owned(), json!("wave"));
        fresh.insert("arity".to_owned(), json!({ "min": 1, "max": 1 }));

        store.set_command("wave", &fresh, false);
        let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["greet", "farewell", "wave"]);
        assert!(
            store.source().contains("# A hand-written banner"),
            "{}",
            store.source()
        );
    }

    #[test]
    fn removing_a_command_takes_its_statement_and_nothing_else() {
        let mut store = PackStore::from_source(PACK);
        assert!(store.remove_command("greet"));
        let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["farewell"]);
        assert!(store.source().contains("# A hand-written banner"));
        assert!(store.source().contains("# And a second one"));
        assert!(!store.remove_command("greet"), "already gone");
    }

    #[test]
    fn an_empty_store_is_valid_dsl_that_loads_back_as_itself() {
        let store = PackStore::empty("mylib");
        assert_eq!(store.name(), "mylib");
        assert!(store.commands().is_empty());
        assert!(store.source().contains(&format!(
            "speclib mylib {} {{",
            crate::render_spectcl::DSL_VERSION
        )));
    }

    #[test]
    fn selecting_a_shipped_command_and_canonicalising_preserves_the_pack_version() {
        // Browser repro: selecting `doctools::search` seeds it into a fresh
        // 1.1 pack, then Pack DSL → Re-render canonically used to rewrite the
        // header to the renderer's newest (1.2) vocabulary. Selection itself
        // is a normal `set_command` write, so exercise the shared store rather
        // than teaching the browser a version rule of its own.
        let mut store = PackStore::from_source("speclib mylib 1.1 {\n}\n");
        let selected = Builtins::for_dialect("tcl9.0")
            .draft("doctools::search")
            .expect("doctools::search is a browsable shipped command");
        store.set_command("doctools::search", &selected, true);
        assert!(store.source().starts_with("speclib mylib 1.1 {"));

        // The browser hydrates every tab from this one stored draft.  Keep
        // the DSL, generated Rust, and Tcl stub outputs coupled so selecting
        // a registry command cannot leave either generated tab blank.
        let hydrated = store
            .draft("doctools::search")
            .expect("selected command is retained in the pack");
        let rust = crate::render_rs::render(hydrated);
        let stub = crate::render_stub::render(
            std::slice::from_ref(hydrated),
            crate::render_stub::Mode::Inline,
            "tcl9.0",
        );
        assert!(
            store
                .source()
                .contains("command doctools::search -override"),
            "DSL tab must contain the selected command:\n{}",
            store.source()
        );
        assert!(
            !rust.trim().is_empty() && rust.contains("doctools::search"),
            "Rust tab must render the selected command:\n{rust}"
        );
        assert!(
            !stub.trim().is_empty() && stub.contains("doctools::search"),
            "Tcl stub tab must render the selected command:\n{stub}"
        );

        let canonical = store.canonical();
        assert!(
            canonical.contains("\nspeclib mylib 1.1 {\n"),
            "canonical rendering must preserve the loaded header:\n{canonical}"
        );
        let round_tripped = PackStore::from_source(canonical);
        assert_eq!(round_tripped.dsl_version(), "1.1");
        assert!(round_tripped.draft("doctools::search").is_some());
        assert!(round_tripped.overrides_shipped("doctools::search"));
    }

    #[test]
    fn adding_newer_registry_data_upgrades_the_header_before_the_first_render() {
        let mut store = PackStore::from_source("speclib mylib 1.1 {\n}\n");
        let selected = Builtins::for_dialect("tcl9.0")
            .draft("ttk::treeview")
            .expect("ttk::treeview is a browsable shipped command");
        let write = store.set_command("ttk::treeview", &selected, true);

        assert_eq!(write.how, WriteBack::VocabularyUpgraded);
        assert_eq!(write.upgraded_from.as_deref(), Some("1.1"));
        assert_eq!(store.dsl_version(), render_spectcl::DSL_VERSION);
        assert!(store.source().contains("command ttk::treeview -override"));
        assert!(
            store.notices().iter().all(|notice| !notice
                .message
                .contains("vocabulary, but this pack declares vocabulary")),
            "an upgraded document must not contain newer words under an older header: {:?}",
            store.notices()
        );
        assert_eq!(
            PackStore::from_source(store.canonical()).dsl_version(),
            render_spectcl::DSL_VERSION,
            "canonical re-render is stable after the edit-time upgrade"
        );
    }

    #[test]
    fn override_survives_a_write_back() {
        let source = "speclib demo 1.0 {\ncommand lsort -override {\n    arity 1\n}\n}\n";
        let mut store = PackStore::from_source(source);
        assert!(store.overrides_shipped("lsort"));
        let mut edited = store.draft("lsort").expect("lsort").clone();
        edited.insert("return_type".to_owned(), json!("List"));
        store.set_command("lsort", &edited, true);
        assert!(
            store.overrides_shipped("lsort"),
            "-override must survive a form edit:\n{}",
            store.source()
        );
    }

    #[test]
    fn the_resolution_facade_applies_the_shipped_collision_policy() {
        let source = "speclib demo 1.0 {\n\
                      command lsort {\n    arity 1\n}\n\
                      command uplift -override {\n    arity 1\n}\n\
                      command mything {\n    arity 1\n}\n}\n";
        let store = PackStore::from_source(source);
        let builtins = Builtins::for_dialect("tcl9.0");
        let merged = Resolution::new(builtins, &store);

        assert_eq!(merged.origin("lsort"), Some(Origin::Shadowed));
        assert_eq!(merged.origin("mything"), Some(Origin::Pack));
        assert_eq!(merged.origin("lappend"), Some(Origin::Builtin));
        assert_eq!(merged.origin("not::a::command"), None);
        // `uplift` is not a shipped Tcl command, so `-override` is harmless.
        assert_eq!(merged.origin("uplift"), Some(Origin::Pack));

        // Shadowed means the shipped spec is what the editor would use.
        let view = merged.view("lsort").expect("a view for lsort");
        assert_eq!(view["origin"], json!("shadowed"));
        assert_eq!(view["effective"], view["builtin"]);
        assert_ne!(view["pack"], Value::Null);

        let view = merged.view("mything").expect("a view for mything");
        assert_eq!(view["effective"], view["pack"]);
        assert_eq!(view["builtin"], Value::Null);
    }

    #[test]
    fn the_store_view_counts_what_a_sidebar_shows() {
        let store = PackStore::from_source(PACK);
        let merged = Resolution::new(Builtins::for_dialect("tcl9.0"), &store);
        let view = merged.store_view();
        assert_eq!(view["pack"], json!("demo"));
        assert_eq!(view["summary"]["commands"], json!(2));
        let rows = view["commands"].as_array().expect("rows");
        assert_eq!(rows[0]["name"], json!("greet"));
        assert_eq!(rows[0]["summary"], json!("Say hello."));
        assert!(
            rows[0]["fields_set"].as_u64().is_some_and(|n| n >= 2),
            "{:?}",
            rows[0]
        );
    }

    #[test]
    fn validation_reports_what_the_loader_dropped() {
        let source = "speclib demo 1.0 {\ncommand greet {\n    arity 1\n    nonsense yes\n}\n}\n";
        let store = PackStore::from_source(source);
        let merged = Resolution::new(Builtins::for_dialect("tcl9.0"), &store);
        let report = merged.validate();
        let notices = report["notices"].as_array().expect("notices");
        assert_eq!(notices.len(), 1, "{report}");
        assert_eq!(notices[0]["context"], json!("command greet"));
        assert!(
            notices[0]["reason"]
                .as_str()
                .is_some_and(|r| r.contains("nonsense")),
            "{report}"
        );
        assert_eq!(report["summary"]["notices"], json!(1));
        assert_eq!(report["commands"][0]["name"], json!("greet"));
    }

    #[test]
    fn a_document_that_is_not_a_pack_loads_as_an_empty_store_with_a_notice() {
        let store = PackStore::from_source("this is not a pack\n");
        assert!(store.commands().is_empty());
        assert!(!store.notices().is_empty());
        // And it can still be written to — the write-back falls through to the
        // re-render floor, which produces a real pack.
        let mut store = store;
        let mut fresh = draft::default_command_draft();
        fresh.insert("name".to_owned(), json!("wave"));
        assert_eq!(
            store.set_command("wave", &fresh, false).how,
            WriteBack::Rerendered
        );
        assert_eq!(store.commands().len(), 1);
    }

    #[test]
    fn canonical_rendering_round_trips_the_declared_commands() {
        let store = PackStore::from_source(PACK);
        let canonical = PackStore::from_source(store.canonical());
        let before: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
        let after: Vec<&str> = canonical
            .commands()
            .iter()
            .map(|(n, _)| n.as_str())
            .collect();
        assert_eq!(before, after);
        for (name, d) in store.commands() {
            assert_eq!(canonical.draft(name), Some(d), "{name} changed meaning");
        }
    }

    // ── The evaluation loader underneath (design E §15.2) ─────────────

    /// A canonical document is observably what it always was: the store
    /// browses the same commands, in the same order, with the same drafts.
    #[test]
    fn a_canonical_document_is_unchanged_by_evaluation() {
        let store = PackStore::from_source(PACK);
        assert_eq!(store.name(), "demo");
        let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["greet", "farewell"]);
        assert!(!store.target_dependent());
        assert_eq!(store.untrusted_tier_refusal(), None);
        // Every declaration is a statement in this document, so none is an
        // expansion.
        for (name, _) in store.commands() {
            let site = store.declaration_site(name).expect("a declaration site");
            assert!(!site.expanded, "{name}: {site:?}");
            assert!(site.line > 0, "{name}: {site:?}");
            assert_eq!(site.file, None);
        }
    }

    /// A *programmed* document browses its snapshot: the loop's output is
    /// browsable, and each derived command is marked as an expansion with
    /// the line inside the program that registered it.
    #[test]
    fn a_programmed_document_browses_its_expansion_with_provenance() {
        let store = PackStore::from_source(
            "speclib fleet 2.0 {\n    \
             proc fleet-command {name} {\n        \
             command fleet::$name {\n            arity 1\n        }\n    }\n    \
             foreach name {alpha beta} { fleet-command $name }\n}\n",
        );
        let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["fleet::alpha", "fleet::beta"]);
        for name in names {
            let site = store.declaration_site(name).expect("a declaration site");
            assert!(site.expanded, "{name}: {site:?}");
            // The line points *into the program* — the registering statement,
            // not a `command` statement in the file, because there is none.
            assert!(site.line > 1, "{name}: {site:?}");
            assert!(
                crate::store::command_span(store.source(), name).is_none(),
                "{name}: an expanded command has no statement of its own"
            );
        }
    }

    /// `available?` makes the browsed surface one target's answer, and the
    /// store says so.
    #[test]
    fn a_target_dependent_document_is_flagged() {
        let store = PackStore::from_source(
            "speclib trap 2.0 {\n    default available {tcl 8.6-}\n    \
             command base { arity 1 }\n    if {[available? {tcl 8.6-}]} {\n        \
             command extra { arity 1 }\n    }\n}\n",
        );
        assert!(store.target_dependent());
        assert_eq!(store.commands().len(), 2);
    }

    // ── E-R12: patch-pack editing ──────────────────────────────────────

    /// The programmed document every patch test edits: one data table, one
    /// helper `proc`, one `foreach`, and nothing a splice could reach.
    const PROGRAMMED: &str = "speclib fleet 2.0 {\n    \
         proc fleet-command {name arity} {\n        \
         command fleet::$name {\n            arity $arity\n        }\n    }\n    \
         foreach {name arity} {alpha 1 beta 2} { fleet-command $name $arity }\n}\n";

    /// A canonical pack is canonical, and the studio keeps editing it in
    /// place — the property the round-trip suite rests on.
    #[test]
    fn a_canonical_document_is_not_programmed() {
        let store = PackStore::from_source(PACK);
        assert_eq!(store.programmed(), None);
        assert_eq!(store.patch_source(), None);
        assert!(store.standing_overrides().is_empty());
    }

    /// A pack whose declarations are all literal but which wraps them in a
    /// program is still a program: no declaration is an expansion, so only
    /// the straight-line check catches it — and it must, because the
    /// re-render floor would delete the `proc`.
    #[test]
    fn a_literal_pack_around_a_program_is_still_programmed() {
        let store = PackStore::from_source(
            "speclib demo 2.0 {\n    set width 3\n    \
             command greet {\n        arity $width\n    }\n}\n",
        );
        assert_eq!(
            store.programmed(),
            Some(Programmed::NonCanonicalStatement),
            "{:#?}",
            store.notices()
        );
    }

    /// A form edit against a programmed pack leaves the source alone and
    /// lands as a canonical patch pack in the `StudioOverride` tier.
    #[test]
    fn an_edit_to_a_programmed_pack_becomes_a_studio_override_patch() {
        let mut store = PackStore::from_source(PROGRAMMED);
        assert_eq!(store.programmed(), Some(Programmed::Expanded));
        let before = store.source().to_owned();

        let mut edited = store.draft("fleet::alpha").expect("a draft").clone();
        edited["arity"] = json!({ "min": 2, "max": 4 });
        let write = store.set_command("fleet::alpha", &edited, true);

        assert_eq!(write.how, WriteBack::Patched);
        assert!(write.dropped.is_empty(), "{:?}", write.dropped);
        assert_eq!(store.source(), before, "the program was rewritten");

        let patch = store.patch_source().expect("a patch pack");
        assert!(
            patch.starts_with("speclib fleet-studio-overrides 2.0 {"),
            "{patch}"
        );
        assert!(
            patch.contains("command fleet::alpha -override {"),
            "{patch}"
        );
        // Canonical by construction: it reloads as itself.
        assert_eq!(
            PackStore::from_source(patch).source(),
            patch,
            "the patch is not canonical"
        );
        // The base declaration is untouched, and the patch is what is live.
        assert_eq!(
            store
                .draft("fleet::alpha")
                .and_then(|d| d["arity"].get("min")),
            Some(&json!(1))
        );
        assert_eq!(
            store
                .effective_draft("fleet::alpha")
                .and_then(|d| d["arity"].get("min")),
            Some(&json!(2))
        );
    }

    /// The patch layers over the base by the **shipped** collision policy:
    /// second pack in the set, `-override` on every declaration, so the
    /// registry the Test tab queries answers with the patched spec.
    #[test]
    fn the_patch_layers_over_the_base_under_the_collision_policy() {
        let mut store = PackStore::from_source(PROGRAMMED);
        let mut edited = store.draft("fleet::alpha").expect("a draft").clone();
        edited["arity"] = json!({ "min": 2, "max": 4 });
        store.set_command("fleet::alpha", &edited, true);

        let set = store.pack_set();
        assert_eq!(set.packs.len(), 2);
        assert_eq!(set.packs[0].tier, Tier::Workspace);
        assert_eq!(set.packs[1].tier, Tier::StudioOverride);
        assert_eq!(set.packs[1].name, "fleet-studio-overrides");
        assert!(
            set.packs[1].commands.iter().all(|c| c.overrides_shipped),
            "a patch declaration that does not say -override cannot win"
        );

        let registry = Resolution::new(Builtins::for_dialect("tcl"), &store).registry();
        let spec = registry.get("fleet::alpha").expect("the patched command");
        assert_eq!(spec.arity.min, 2);
        // And the command the patch does not mention still comes from the base.
        assert_eq!(
            registry
                .get("fleet::beta")
                .expect("the base command")
                .arity
                .min,
            2
        );
    }

    /// The standing-overrides report names the patch, the base, and where the
    /// base declaration came from — the data a UI labels a patched row with.
    #[test]
    fn the_standing_overrides_report_lists_the_patch() {
        let mut store = PackStore::from_source(PROGRAMMED);
        let edited = store.draft("fleet::beta").expect("a draft").clone();
        store.set_command("fleet::beta", &edited, true);

        let standing = store.standing_overrides();
        assert_eq!(standing.len(), 1);
        assert_eq!(standing[0].command, "fleet::beta");
        assert_eq!(standing[0].patch_pack, "fleet-studio-overrides");
        assert_eq!(standing[0].base_pack, "fleet");
        assert!(standing[0].base_expanded);
        assert!(standing[0].base_line.is_some_and(|line| line > 1));

        let view = Resolution::new(Builtins::for_dialect("tcl"), &store).store_view();
        assert_eq!(view["standing_overrides"][0]["command"], "fleet::beta");
        assert_eq!(view["programmed"]["why"], "expanded");
        assert_eq!(view["patch"]["pack"], "fleet-studio-overrides");
        assert_eq!(view["summary"]["standing_overrides"], 1);
    }

    /// Removing the patch restores the base declaration — the promise that
    /// makes layering safe to try.
    #[test]
    fn removing_the_patch_restores_the_base() {
        let mut store = PackStore::from_source(PROGRAMMED);
        let mut edited = store.draft("fleet::alpha").expect("a draft").clone();
        edited["arity"] = json!({ "min": 2, "max": 4 });
        store.set_command("fleet::alpha", &edited, true);
        let key_with_patch = store.overlay_key();

        assert!(store.remove_override("fleet::alpha"));
        assert_eq!(store.patch_source(), None);
        assert!(store.standing_overrides().is_empty());
        assert_eq!(store.pack_set().packs.len(), 1);
        assert_ne!(store.overlay_key(), key_with_patch);
        assert_eq!(
            store
                .effective_draft("fleet::alpha")
                .and_then(|d| d["arity"].get("min")),
            Some(&json!(1))
        );
        assert!(!store.remove_override("fleet::alpha"), "already gone");
    }

    /// Two edits share one patch pack, and a patch survives a round trip
    /// through its own text — which is how a browser reload restores it.
    #[test]
    fn a_patch_accumulates_and_round_trips_through_its_own_text() {
        let mut store = PackStore::from_source(PROGRAMMED);
        for name in ["fleet::alpha", "fleet::beta"] {
            let draft = store.draft(name).expect("a draft").clone();
            store.set_command(name, &draft, true);
        }
        assert_eq!(store.standing_overrides().len(), 2);
        let patch = store.patch_source().expect("a patch").to_owned();

        let restored = PackStore::from_source_with_patch(PROGRAMMED, &patch);
        assert_eq!(restored.patch_source(), Some(patch.as_str()));
        assert_eq!(restored.standing_overrides().len(), 2);
        assert_eq!(restored.overlay_key(), store.overlay_key());
    }

    /// The patch carries the base pack's `default` rows, so a patched command
    /// keeps the availability of the one it replaces rather than silently
    /// widening it — the "minimal available context" a patch pack needs.
    #[test]
    fn the_patch_carries_the_base_packs_available_context() {
        let mut store = PackStore::from_source(
            "speclib fleet 2.0 {\n    default available {tcl 8.6-}\n    \
             proc fleet-command {name} {\n        \
             command fleet::$name {\n            arity 1\n        }\n    }\n    \
             foreach name {alpha beta} { fleet-command $name }\n}\n",
        );
        let base = store
            .draft("fleet::alpha")
            .expect("a draft")
            .get("available")
            .cloned();
        store.set_command(
            "fleet::alpha",
            &store.draft("fleet::alpha").expect("a draft").clone(),
            true,
        );
        let patch = store.patch_source().expect("a patch pack");
        assert!(patch.contains("default available"), "{patch}");
        assert_eq!(
            store
                .patch_draft("fleet::alpha")
                .expect("the patched draft")
                .get("available")
                .cloned(),
            base,
            "the patch changed more than the author edited"
        );
    }

    /// A patch that claims a compiled name loads for its author and would be
    /// refused by a real `StudioOverride` load — reported, not applied, the
    /// same posture the document itself gets.
    #[test]
    fn a_patch_over_a_compiled_name_reports_its_override_tier_refusal() {
        let mut store = PackStore::from_source(
            "speclib trap 2.0 {\n    set names lsort\n    \
             foreach n $names { command $n -override {\n        arity 1..\n    } }\n}\n",
        );
        assert!(store.programmed().is_some());
        let draft = store.draft("lsort").expect("a draft").clone();
        assert_eq!(
            store.set_command("lsort", &draft, true).how,
            WriteBack::Patched
        );
        let (_, why) = store
            .patch_untrusted_tier_refusal()
            .expect("an override-tier refusal");
        assert!(why.contains("design E-R2"), "{why}");
    }

    /// The authoring buffer is trusted — a studio author may write an
    /// `-override` on a compiled name — and the refusal a workspace load
    /// would raise is reported instead of applied.
    #[test]
    fn an_override_is_authorable_and_its_workspace_refusal_is_reported() {
        let store = PackStore::from_source(
            "speclib demo 2.0 {\n    command lsort -override {\n        arity 1..\n    }\n}\n",
        );
        assert_eq!(store.tier(), Tier::Bundled);
        assert_eq!(store.commands().len(), 1, "{:#?}", store.notices());
        assert!(store.overrides_shipped("lsort"));
        let (line, why) = store
            .untrusted_tier_refusal()
            .expect("a workspace tier would refuse this");
        assert_eq!(line, 2);
        assert!(why.contains("design E-R2"), "{why}");

        // A *workspace* pack is `Provenance::WorkspaceTrusted` (redesign
        // §6.4 keys the untrusted class on the editor's Workspace Trust
        // state, not on where the file was found), so it still loads and
        // still overrides — the report above is the warning about the day
        // it does not.
        let workspace = PackStore::from_source_at_tier(store.source(), Tier::Workspace);
        assert_eq!(workspace.commands().len(), 1);

        // The live Spec Studio override tier *is* untrusted, and refuses it,
        // which is what makes the report meaningful rather than theoretical.
        let override_tier = PackStore::from_source_at_tier(store.source(), Tier::StudioOverride);
        assert!(override_tier.commands().is_empty());
    }
}
