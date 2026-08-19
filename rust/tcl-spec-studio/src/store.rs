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
//! ## A note on cost
//!
//! `tcl_spectcl::loader::load_pack` leaks its specs (`Box::leak`), because a
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
use tcl_dialect::DialectProfile;
use tcl_lexer::{LexerConfig, SourceMap};
use tcl_registry::CommandRegistry;
use tcl_registry::cache::registry_for_dialect;
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
        // spellings canonicalised onto it; a name with no profile at all
        // resolves to the dialect the picker starts on, and keeping a
        // `'static` name avoids an allocation per query.
        let dialect = DialectProfile::find(dialect).map_or("tcl9.0", |profile| profile.name);
        Self {
            dialect,
            registry: registry_for_dialect(dialect),
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
}

impl WriteBack {
    /// The word the report uses.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Spliced => "spliced",
            Self::Rerendered => "rerendered",
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
}

/// The user's definitions, backed by one `.tclspec` document.
///
/// The text is the store. Everything else on this struct is derived from it by
/// [`loader::load_pack`] and recomputed whenever the text changes, so there is
/// no second copy of the truth to drift.
pub struct PackStore {
    source: String,
    pack: Pack,
    /// One draft per declared command, in declaration order — derived, and the
    /// only thing the form is ever handed.
    drafts: Vec<(String, Draft)>,
}

impl PackStore {
    /// Load `source` as the store's document.
    ///
    /// Never fails: an unparsable or empty document loads as a pack with no
    /// commands and the loader's notices explaining why.
    #[must_use]
    pub fn from_source(source: impl Into<String>) -> Self {
        let source = source.into();
        let pack = loader::load_pack(&source);
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
    #[must_use]
    pub fn pack_set(&self) -> PackSet {
        let name = if self.pack.name.is_empty() {
            "pack".to_owned()
        } else {
            self.pack.name.clone()
        };
        PackSet {
            packs: vec![MergedPack {
                name: name.clone(),
                dsl_version: self.pack.dsl_version.clone(),
                tier: tcl_spectcl::Tier::Workspace,
                files: vec![std::path::PathBuf::from(format!("{name}.tclspec"))],
                display_name: self.pack.display_name.clone(),
                file_extensions: self.pack.file_extensions.clone(),
                commands: self.pack.commands.clone(),
            }],
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
        if self.pack.commands.is_empty() {
            return 0;
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.source.hash(&mut hasher);
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
        with_override_flags(&render_spectcl::render_pack(&drafts, self.name()), &flags)
    }

    /// Write `draft` back as the definition of `name`.
    ///
    /// A command already in the document is replaced **where it stands**; one
    /// that is not is appended to the pack body. `name` is the declaration
    /// being written *over*, so renaming a command — the author editing the
    /// `name` field — is an ordinary edit: the new name lands in the old one's
    /// place rather than at the end of the file.
    ///
    /// Returns how the document was reached — see [`WriteBack`], and this
    /// module's header for what "spliced" does and does not preserve.
    pub fn set_command(&mut self, name: &str, draft: &Draft, overrides: bool) -> Write {
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
        let how = how.unwrap_or_else(|| {
            self.rerender_with(name, draft, overrides);
            WriteBack::Rerendered
        });

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
        Write { how, dropped }
    }

    /// Drop `name` from the document.
    ///
    /// Returns `false` when the document does not declare it.
    pub fn remove_command(&mut self, name: &str) -> bool {
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
        let text = with_override_flags(&render_spectcl::render_pack(&drafts, self.name()), &flags);
        *self = Self::from_source(text);
        true
    }

    // ── write-back internals ───────────────────────────────────────────

    /// The pack-body text for one command: whatever a one-command pack would
    /// put inside its `speclib` braces, which is the command block plus any
    /// value tables it needs.
    fn render_block(&self, draft: &Draft, overrides: bool) -> String {
        let one = render_spectcl::render_pack(std::slice::from_ref(draft), self.name());
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
        let pack = loader::load_pack(&wrapped);
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
        let text = with_override_flags(&render_spectcl::render_pack(&drafts, pack_name), &flags);
        *self = Self::from_source(text);
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
            DialectProfile::by_name(self.builtins.dialect()),
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
        let pack = self.store.draft(name);
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
                json!({
                    "name": name,
                    "origin": origin.key(),
                    "override": self.store.overrides_shipped(name),
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
            "commands": self.pack_index(),
            "notices": notices_json(&self.store.notices().iter().collect::<Vec<_>>()),
            "collisions": collisions,
            "summary": {
                "commands": self.store.commands().len(),
                "notices": self.store.notices().len(),
                "collisions": collisions.len(),
                "shadowed_commands": shadowed,
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
        F::ConstFold => "const_fold",
        F::ConstFoldVersioned => "const_fold_versioned",
        F::TaintSinkGate => "taint_sink_gate",
        F::ContextGate => "context_gate",
        F::LiteralArgumentValidator => "literal_argument_validator",
        F::ClauseShapeCheck => "clause_shape_check",
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
}
