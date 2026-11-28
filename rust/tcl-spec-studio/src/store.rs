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
//! `docs/design/registry/spec-packs.md` asks for exactly three things from
//! the studio's model layer, and this module is all three:
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
//! - Carry-forward reaches one scope down: a hook body hanging off a
//!   **subcommand** is matched to the freshly rendered `subcommand NAME { … }`
//!   of the same name and merged word by word, exactly as a command-level
//!   hook is. A subcommand the edit renamed or removed has nowhere to carry
//!   its body forward into, and a **sub-subcommand**'s own hook is one scope
//!   further than this reaches; both cases are detected and reported through
//!   [`Write::dropped`], so a surface can tell the author what an edit will
//!   cost before it costs it.
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

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use serde_json::{Value, json};
#[cfg(test)]
use tcl_lexer::SourceSegmentationPurpose;
use tcl_registry::CommandRegistry;
use tcl_spectcl::Tier;
use tcl_spectcl::loader;
use tcl_spectcl::loader::{FileExtension, Notice, Pack};
use tcl_spectcl::pack::{MergedPack, PackSet};

use crate::draft::{self, Draft};
use crate::render_spectcl;

mod patch_packs;
mod resolution;
mod source_spans;
mod write_back;

pub use patch_packs::{Programmed, StandingOverride};
pub use resolution::{Origin, Resolution, fields_set_of, standing_overrides_json};
#[cfg(test)]
use source_spans::segments;
use source_spans::{SourceIndex, with_override_flags};
pub use source_spans::{command_span, pack_body};
pub use write_back::{Write, WriteBack};

// Built-ins — the immutable model

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

/// The user's definitions, backed by one `.tclspec` document.
///
/// The text is the store. Everything else on this struct is derived from it by
/// [`loader::evaluate_pack_with`] and recomputed whenever the text changes, so
/// there is no second copy of the truth to drift.
pub struct PackStore {
    source: String,
    pack: Pack,
    /// Lazily built CST index for this exact `source`. A store's source is
    /// immutable in place — every edit constructs a replacement store — so
    /// the index never needs invalidating. Keeping it lazy avoids charging
    /// callers that only browse the evaluated snapshot for CST work they do
    /// not need.
    source_index: OnceLock<SourceIndex>,
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
    /// gate (`tcl-spectcl-it/tests/golden_packs.rs`), the fast-path gate
    /// (`tcl-spectcl-it/tests/eval_loader.rs`) and the studio's own round-trip
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
            source_index: OnceLock::new(),
            tier,
            drafts,
            patch: None,
        }
    }

    fn source_index(&self) -> &SourceIndex {
        self.source_index
            .get_or_init(|| SourceIndex::new(&self.source))
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
    /// A **hypothetical**, deliberately: a workspace pack the editor trusts
    /// is `Provenance::WorkspaceTrusted` (§6.4 keys the untrusted class on
    /// the editor's Workspace Trust state, which reaches discovery on
    /// `DiscoveryOptions::workspace_trust`), so one that overrides a shipped
    /// command loads for its author. This answers the question the author
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
            expanded: self.source_index().command(name).is_none(),
        })
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
        let mut load_errors: Vec<_> = self
            .pack
            .load_error
            .iter()
            .flat_map(|error| {
                packs[0]
                    .files
                    .iter()
                    .map(|path| (path.clone(), error.clone()))
            })
            .collect();
        if let Some(patch) = &self.patch {
            let loaded = merged(&self.patch_name(), &patch.pack, Tier::StudioOverride);
            load_errors.extend(patch.pack.load_error.iter().flat_map(|error| {
                loaded
                    .files
                    .iter()
                    .map(|path| (path.clone(), error.clone()))
            }));
            packs.push(loaded);
        }
        PackSet {
            packs,
            notices: Vec::new(),
            load_errors,
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
}

/// One loaded pack as the one-file [`MergedPack`] the installer takes.
///
/// The studio edits packs **in a browser**, where there is no file to
/// discover, so the merged shape is assembled here rather than by
/// `tcl_spectcl::pack::load` — same commands, same declarations, at the tier
/// the pack really layers from.
fn merged(name: &str, pack: &Pack, tier: Tier) -> MergedPack {
    // The studio evaluates its buffer trusted — the author's own file, as its
    // `EvalOptions` say — whichever tier it layers from.
    let trust = tcl_dialect::model::WorkspaceTrust::Trusted;
    // The stamp rejection rule, as a load at `tier` applies it: the installed
    // world drops every codegen-axis stamp that load would, while the
    // document, and every draft seeded from it, keeps the rows as written
    // ([`PackStore::stamp_refusals`] reports them).
    let provenance = tcl_spectcl::PackEnvironmentTier::of(tier, trust).provenance();
    let mut commands = pack.commands.clone();
    for command in &mut commands {
        tcl_spectcl::stamps::admit_codegen_stamps(
            command,
            provenance,
            tcl_spectcl::stamps::shipped(),
        );
    }
    MergedPack {
        name: name.to_owned(),
        dsl_version: pack.dsl_version.clone(),
        tier,
        trust,
        files: vec![std::path::PathBuf::from(format!("{name}.tclspec"))],
        display_name: pack.display_name.clone(),
        file_extensions: pack.file_extensions.clone(),
        ambient_packages: pack.ambient_packages.clone(),
        special_vars: pack.special_vars.clone(),
        environments: pack.environments.clone(),
        dialects: pack.dialects.clone(),
        surface_rosters: pack.surface_rosters.clone(),
        commands,
    }
}

/// What a load of `pack` at `tier` refuses under the stamp rejection rule
/// (`tcl_spectcl::stamps`), as `(line, why)` on each declaring command's row
/// — the same warning the load publishes on the pack file.
fn stamp_refusals_at(pack: &Pack, tier: Tier) -> Vec<(u32, String)> {
    let provenance =
        tcl_spectcl::PackEnvironmentTier::of(tier, tcl_dialect::model::WorkspaceTrust::Trusted)
            .provenance();
    pack.commands
        .iter()
        .flat_map(|command| {
            tcl_spectcl::stamps::stamp_refusals(
                command.spec,
                provenance,
                None,
                tcl_spectcl::stamps::shipped(),
            )
            .into_iter()
            .map(move |refusal| (command.line, refusal.message()))
        })
        .collect()
}

/// `(line, why)` rows as a surface reads them.
fn line_reasons_json(rows: &[(u32, String)]) -> Value {
    Value::Array(
        rows.iter()
            .map(|(line, why)| json!({ "line": line, "reason": why }))
            .collect(),
    )
}

#[cfg(test)]
mod tests;
