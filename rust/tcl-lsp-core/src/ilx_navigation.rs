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

//! iRulesLX cross-language navigation: an `ILX::call` / `ILX::notify` method
//! word ↔ the `ILXServer.addMethod` registration that implements it
//! (issue #1707).
//!
//! [`tcl_irules::ilx`] finds the two halves inside one file each; this module
//! is what connects them across the workspace, and it is the only place that
//! decides *which* extension source an iRule's `ILX::init PLUGIN EXTENSION`
//! refers to.
//!
//! # The workspace layout this reads
//!
//! VERIFIED against F5's tmsh `ilx workspace` reference (fetched 2026-08-30),
//! which documents the on-box layout as
//!
//! ```text
//! /var/ilx/workspaces/<partition>/<workspace>/extensions/<extension>/…
//! /var/ilx/workspaces/<partition>/<workspace>/rules/<rule>.tcl
//! ```
//!
//! and the entry point as "node will look in package.json for a main field
//! that identifies the main entry point of the plugin.  If the main field is
//! not present node will look for the file index.js."
//!
//! So an *extension* name is established by the source layout: it is the
//! directory under `extensions/`.  A **plugin** name is not: a plugin is
//! created from a workspace (`create ilx plugin P from-workspace W`) and the
//! two names need not match.  The rule this module applies, and the only one:
//!
//! > the `PLUGIN` word of `ILX::init` must equal the **name of the workspace
//! > directory** holding `extensions/EXTENSION`.
//!
//! Candidate workspace directories are looked for along the document's own
//! ancestors only — the enclosing workspace of a rule in `…/W/rules/x.tcl`,
//! and a `…/<ancestor>/PLUGIN/extensions/…` sibling — never by scanning the
//! whole workspace.  If the name does not match, or two distinct directories
//! match, nothing resolves: an unknown or ambiguous mapping abstains rather
//! than guessing (issue #1707 criterion 4).  A plugin deliberately named
//! differently from its workspace is therefore **not** navigable yet; that is
//! the "documented workspace/config mapping" half of criterion 2, and it is
//! left for a follow-up rather than approximated here.
//!
//! # Everything else abstains
//!
//! A dynamic handle, a computed method name, a missing or unreadable
//! JavaScript source, a method the extension does not register, and a method
//! registered twice all resolve to an [`IlxTarget`] variant that carries *why*
//! — hover says so in one line, and go-to-definition offers nothing.  No
//! diagnostic is emitted from any of this; an unresolved method is not an
//! error, because the extension's method table is only known when the
//! JavaScript is in the workspace.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tcl_irules::ilx::{
    IlxExtension, IlxMethodCall, IlxMethodRegistration, extension_entry_file,
    extension_registrations, ilx_method_calls,
};

/// The two per-file models this module joins, re-exported so a consumer names
/// one crate rather than two for one feature.
pub use tcl_irules::ilx::{
    IlxExtension as Extension, IlxMethodCall as MethodCall,
    IlxMethodRegistration as MethodRegistration,
};
use tcl_lexer::{LineIndex, Span, Utf16Col};
use tcl_registry::CommandRegistry;

use crate::definition::{LspRange, span_to_range};

/// The dialect whose registry carries the ILX descriptors.
///
/// A JavaScript extension source has no dialect of its own, so the reverse
/// direction (registration → iRule call sites) has to name the one that owns
/// the relation.  Keeping the name here rather than in the server means the
/// server never spells a dialect for this feature.
pub const EXTENSION_RULE_DIALECT: &str = "f5-irules";

/// How far up the document's ancestors the workspace search walks.
///
/// A rule sits two levels below its workspace (`W/rules/x.tcl`), so this is
/// generous; the bound exists so a document at the root of a deep tree cannot
/// turn one navigation request into an unbounded run of `stat` calls.
const MAX_ANCESTORS: usize = 24;

/// The directory that holds an ILX workspace's extensions.
const EXTENSIONS_DIR: &str = "extensions";

/// The directory that holds an ILX workspace's iRules.
const RULES_DIR: &str = "rules";

/// How deep a *declared* extra rule directory is walked.
const MAX_RULE_DEPTH: usize = 8;

/// How many directories one request may `read_dir` across all of its declared
/// extra rule roots.
const MAX_RULE_DIRS: usize = 512;

/// The document a request is about: where it is, and the text the editor has.
///
/// The text is carried rather than re-read because an open buffer is routinely
/// ahead of the file on disk, and a reference list computed from stale bytes
/// points at the wrong columns.
#[derive(Clone, Copy)]
pub struct IlxDocument<'a> {
    /// The document's filesystem path.
    pub path: &'a Path,
    /// The document's current text.
    pub text: &'a str,
}

/// The text the editor currently holds for a document, when it holds one.
///
/// The server's own cross-document providers reach a sibling file through
/// `read_document`, which answers from the open-document map first and only
/// then from disk. This relation reads *other* files too — sibling rules, and
/// the extension's JavaScript — so it needs the same precedence, or a
/// find-references over an edited-but-unsaved rule silently reports the text
/// that was last written to disk (issue #1707 review).
///
/// A trait rather than a map so the caller keeps ownership of its document
/// store and this crate stays free of the server's types; the native server
/// implements it over the very map `read_document` consults.
///
/// `Send + Sync` for the same reason [`crate::vfs::SourceStore`] is: the native
/// server holds one across an `await`, so a non-thread-safe implementation
/// would make the request future itself non-`Send`.
pub trait OpenDocuments: Send + Sync {
    /// The editor's current text for `path`, or `None` when it holds none.
    fn text(&self, path: &Path) -> Option<Arc<str>>;
}

/// Where every *other* file a request touches is read from.
///
/// Separate from [`IlxContext`] because reading a file needs no registry: the
/// gate that decides whether a document is an extension entry point at all
/// ([`is_extension_entry`]) is pure filesystem, and asking it for a dialect it
/// has no use for would push the caller into choosing one before it knows
/// which end of the relation it is on.
#[derive(Clone, Copy)]
pub struct IlxFiles<'a> {
    /// Where closed files are read from (see [`crate::vfs`]).
    pub store: &'a dyn crate::vfs::SourceStore,
    /// What the editor holds for files that are open, consulted first.
    ///
    /// `None` is the "no editor" case — the CLI, a unit test — where the store
    /// is the whole truth.
    pub open: Option<&'a dyn OpenDocuments>,
}

impl<'a> IlxFiles<'a> {
    /// Files read from `store` alone.
    #[must_use]
    pub const fn new(store: &'a dyn crate::vfs::SourceStore) -> Self {
        Self { store, open: None }
    }

    /// The same files, with the editor's unsaved buffers taking precedence.
    #[must_use]
    pub const fn with_open_documents(self, open: &'a dyn OpenDocuments) -> Self {
        Self {
            open: Some(open),
            ..self
        }
    }

    /// One file's current text: the editor's copy when it has one, else the
    /// store's.
    fn read(self, path: &Path) -> Option<String> {
        if let Some(open) = self.open
            && let Some(text) = open.text(path)
        {
            return Some(text.to_string());
        }
        self.store.read_to_string(path).ok()
    }

    /// Whether `path` names a readable file — an unsaved buffer counts, so a
    /// just-created `index.js` is an entry point before its first save.
    fn is_file(self, path: &Path) -> bool {
        if let Some(open) = self.open
            && open.text(path).is_some()
        {
            return true;
        }
        self.store.metadata(path).is_ok_and(|meta| !meta.is_dir)
    }

    /// Whether `path` is a directory.  Only the store can answer: a directory
    /// is never an open document.
    fn is_dir(self, path: &Path) -> bool {
        self.store.is_dir(path)
    }
}

/// One **configured** plugin, as the user declared it.
///
/// The source layout establishes an extension name but never a plugin name — a
/// plugin is created *from* a workspace (`create ilx plugin P from-workspace
/// W`) and the two need not match. Without a declaration the only association
/// available is the directory-name convention, and a plugin named otherwise is
/// simply not navigable; guessing from the extension name alone would pick the
/// wrong file in a workspace holding two plugins, which is the guess issue
/// #1707 criterion 4 forbids. So the user says it instead
/// (`.tcl-lsp.ini [iruleslx.plugins]`, or the `tclLsp.iruleslx` settings key).
///
/// A declaration is **authoritative**: once a plugin name is configured, the
/// convention is not consulted for it at all, so a mapping can also *correct* a
/// directory that happens to collide with a plugin name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlxPluginRoot {
    /// The `PLUGIN` word of `ILX::init`.
    pub plugin: String,
    /// The ILX workspace directory holding `extensions/` (and usually
    /// `rules/`), as an absolute path.
    pub workspace: PathBuf,
    /// Extra directories searched — recursively — for iRules that call this
    /// plugin, beyond its workspace's own `rules/`.
    ///
    /// The deployed layout keeps every rule in `rules/`, but a repository
    /// routinely keeps its iRules somewhere else entirely and builds the
    /// workspace at release time. Those callers are unreachable from the
    /// layout alone, and finding them by scanning the tree is the cost this
    /// model refuses — so, again, the user names them.
    pub extra_rule_dirs: Vec<PathBuf>,
}

/// What a request may consult beyond the document in front of it.
#[derive(Clone, Copy)]
pub struct IlxContext<'a> {
    /// The dialect registry — the descriptors, and the dialect gate.
    pub registry: &'a CommandRegistry,
    /// Where the other files this request reads come from.
    pub files: IlxFiles<'a>,
    /// The user's declared plugin associations (see [`IlxPluginRoot`]).
    /// Empty is the unconfigured case, where the directory-name convention is
    /// the whole story.
    pub plugins: &'a [IlxPluginRoot],
}

impl<'a> IlxContext<'a> {
    /// A context that reads every other file from `store`, with no configured
    /// plugin associations.
    #[must_use]
    pub const fn new(
        registry: &'a CommandRegistry,
        store: &'a dyn crate::vfs::SourceStore,
    ) -> Self {
        Self {
            registry,
            files: IlxFiles::new(store),
            plugins: &[],
        }
    }

    /// This context, with the editor's unsaved buffers taking precedence over
    /// the store.
    #[must_use]
    pub const fn with_open_documents(self, open: &'a dyn OpenDocuments) -> Self {
        Self {
            files: self.files.with_open_documents(open),
            ..self
        }
    }

    /// This context, with the user's declared plugin associations.
    #[must_use]
    pub const fn with_plugins(self, plugins: &'a [IlxPluginRoot]) -> Self {
        Self { plugins, ..self }
    }

    /// The configured entries declaring `plugin`, in configuration order.
    fn declared(self, plugin: &str) -> impl Iterator<Item = &'a IlxPluginRoot> {
        self.plugins.iter().filter(move |p| p.plugin == plugin)
    }

    /// The extra rule directories declared for whichever plugin(s) `workspace`
    /// was configured as.
    fn extra_rules_for_workspace(self, workspace: &Path) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        for entry in self.plugins.iter().filter(|p| p.workspace == workspace) {
            for dir in &entry.extra_rule_dirs {
                push_unique(&mut out, dir.clone());
            }
        }
        out
    }
}

/// One resolved location in some file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlxLocation {
    /// The file the location is in.
    pub path: PathBuf,
    /// The range within it, in LSP UTF-16 coordinates.
    pub range: LspRange,
    /// Which end of the relation this location is.
    pub kind: IlxSite,
}

/// Which end of the relation a location sits at.
///
/// Carried so a caller can honour `ReferenceContext.includeDeclaration`: the
/// `addMethod` registration *is* the declaration of an ILX method, and a client
/// that asked for uses only must not be handed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IlxSite {
    /// An `ILXServer.addMethod` registration — the method's declaration.
    Registration,
    /// An `ILX::call` / `ILX::notify` word — a use.
    Call,
}

/// What a method word resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IlxTarget {
    /// Exactly one registration implements the method.
    Resolved(IlxLocation),
    /// The extension registers this name more than once — an explicitly
    /// scoped ambiguity, not a target.
    Ambiguous {
        /// The extension source the duplicate registrations live in.
        path: PathBuf,
        /// How many registrations of the name it holds.
        count: usize,
    },
    /// Nothing resolved, and why.
    Unresolved(IlxUnresolved),
}

/// Why a method word did not resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IlxUnresolved {
    /// The handle is not a literal `ILX::init PLUGIN EXTENSION`.
    HandleNotStatic,
    /// No workspace directory named for the plugin holds the extension.
    ExtensionNotFound,
    /// More than one candidate directory matched the plugin/extension pair.
    ExtensionAmbiguous,
    /// The extension source was found but could not be read as text.
    ExtensionUnreadable,
    /// The extension source registers no method of that name.
    MethodNotRegistered,
}

impl IlxUnresolved {
    /// One line for hover, phrased as what is *not* known.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::HandleNotStatic => {
                "the handle is not a literal `ILX::init PLUGIN EXTENSION`, so the extension is unknown"
            }
            Self::ExtensionNotFound => {
                "no workspace directory named for the plugin holds this extension"
            }
            Self::ExtensionAmbiguous => {
                "more than one workspace directory matches this plugin and extension"
            }
            Self::ExtensionUnreadable => "the extension's entry point could not be read",
            Self::MethodNotRegistered => "the extension registers no method of this name",
        }
    }
}

// ---------------------------------------------------------------------------
// Tcl side: from a call site to the registration
// ---------------------------------------------------------------------------

/// The `ILX::call` / `ILX::notify` site whose **method word** the cursor sits
/// on, if any.
///
/// Anchored on the method word alone: a cursor on the command name, the
/// handle, or an argument is not this relation, and answering for them would
/// hijack the ordinary providers.
#[must_use]
pub fn method_call_at(
    doc: IlxDocument<'_>,
    ctx: IlxContext<'_>,
    line: u32,
    character: u32,
) -> Option<IlxMethodCall> {
    let index = LineIndex::new(doc.text);
    let offset = index.offset_at_utf16(line, Utf16Col::new(character), doc.text);
    ilx_method_calls(doc.text, ctx.registry)
        .into_iter()
        .find(|call| call.method_span.start() <= offset && offset <= call.method_span.end())
}

/// Resolve `call`'s method word to the `addMethod` registration implementing
/// it.
#[must_use]
pub fn definition(doc: IlxDocument<'_>, ctx: IlxContext<'_>, call: &IlxMethodCall) -> IlxTarget {
    let Some(target) = call.target.as_ref() else {
        return IlxTarget::Unresolved(IlxUnresolved::HandleNotStatic);
    };
    let site = match locate_extension(doc.path, target, ctx) {
        Ok(site) => site,
        Err(reason) => return IlxTarget::Unresolved(reason),
    };
    let Some(source) = ctx.files.read(&site.entry) else {
        return IlxTarget::Unresolved(IlxUnresolved::ExtensionUnreadable);
    };
    let matches: Vec<IlxMethodRegistration> = extension_registrations(&source)
        .into_iter()
        .filter(|registration| registration.name == call.method)
        .collect();
    match matches.len() {
        0 => IlxTarget::Unresolved(IlxUnresolved::MethodNotRegistered),
        1 => IlxTarget::Resolved(location(
            &site.entry,
            &source,
            matches[0].name_span,
            IlxSite::Registration,
        )),
        count => IlxTarget::Ambiguous {
            path: site.entry,
            count,
        },
    }
}

/// The hover body for a method word: what it names, where it is implemented,
/// and how the call reaches it.
///
/// `ILX::call` and `ILX::notify` share the method target but are *not* the same
/// operation, so the dispatch line always says which one this is — the
/// "synchronous versus best-effort notification" distinction issue #1707
/// criterion 5 asks to keep visible.
#[must_use]
pub fn hover_markdown(call: &IlxMethodCall, target: &IlxTarget) -> String {
    use std::fmt::Write as _;

    let mut out = format!("**iRulesLX method** `{}`\n\n", call.method);
    match &call.target {
        Some(extension) => {
            let _ = write!(
                out,
                "Extension `{}` of plugin `{}`.\n\n",
                extension.extension, extension.plugin
            );
        }
        None => out.push_str("Extension unknown.\n\n"),
    }
    match target {
        IlxTarget::Resolved(location) => {
            let _ = write!(
                out,
                "Implemented by `ILXServer.addMethod` in `{}`.\n\n",
                location.path.display()
            );
        }
        IlxTarget::Ambiguous { path, count } => {
            let _ = write!(
                out,
                "`{}` registers this name {count} times — no single definition \
                 to navigate to.\n\n",
                path.display()
            );
        }
        IlxTarget::Unresolved(reason) => {
            let _ = write!(out, "Not resolved: {}.\n\n", reason.label());
        }
    }
    let _ = write!(
        out,
        "Reached by `{}` — {}.",
        call.command,
        call.dispatch.label()
    );
    out
}

/// Every site of `call`'s method: its registration(s) in the extension, and
/// every literal `ILX::call` / `ILX::notify` of the same method on the same
/// extension in this document and in the associated workspace's `rules/`.
#[must_use]
pub fn references(
    doc: IlxDocument<'_>,
    ctx: IlxContext<'_>,
    call: &IlxMethodCall,
) -> Vec<IlxLocation> {
    let Some(target) = call.target.as_ref() else {
        // With no extension the method name is scoped to nothing, so the only
        // honest answer is this document's own equally-unscoped sites: matching
        // by name alone across the workspace is exactly the global uniqueness
        // this model refuses (issue #1707 criterion 1).
        return call_sites_in(doc.path, doc.text, ctx, None, &call.method);
    };
    let Ok(site) = locate_extension(doc.path, target, ctx) else {
        return call_sites_in(doc.path, doc.text, ctx, Some(target), &call.method);
    };
    let mut out = registration_locations(&site, ctx, &call.method);
    out.extend(workspace_call_sites(&site, doc, ctx, target, &call.method));
    out
}

/// The registration locations for `method` in `site`'s entry point.
fn registration_locations(
    site: &ExtensionSite,
    ctx: IlxContext<'_>,
    method: &str,
) -> Vec<IlxLocation> {
    let Some(source) = ctx.files.read(&site.entry) else {
        return Vec::new();
    };
    extension_registrations(&source)
        .into_iter()
        .filter(|registration| registration.name == method)
        .map(|registration| {
            location(
                &site.entry,
                &source,
                registration.name_span,
                IlxSite::Registration,
            )
        })
        .collect()
}

/// Every call site of `method` on `target`: the open document's own, plus each
/// rule file in the workspace's `rules/` directory.
fn workspace_call_sites(
    site: &ExtensionSite,
    doc: IlxDocument<'_>,
    ctx: IlxContext<'_>,
    target: &IlxExtension,
    method: &str,
) -> Vec<IlxLocation> {
    let mut out = call_sites_in(doc.path, doc.text, ctx, Some(target), method);
    for rule in rule_files(&site.workspace, &site.extra_rules, ctx.files.store) {
        if rule == doc.path {
            continue;
        }
        let Some(text) = ctx.files.read(&rule) else {
            continue;
        };
        out.extend(call_sites_in(&rule, &text, ctx, Some(target), method));
    }
    out
}

/// The literal call sites of `method` on `target` in one document's text.
///
/// `target` `None` restricts the match to sites whose own handle is unknown,
/// which is what keeps a name-only match from leaking across extensions.
fn call_sites_in(
    path: &Path,
    text: &str,
    ctx: IlxContext<'_>,
    target: Option<&IlxExtension>,
    method: &str,
) -> Vec<IlxLocation> {
    ilx_method_calls(text, ctx.registry)
        .into_iter()
        .filter(|call| call.method == method && call.target.as_ref() == target)
        .map(|call| location(path, text, call.method_span, IlxSite::Call))
        .collect()
}

// ---------------------------------------------------------------------------
// JavaScript side: from a registration back to the iRules
// ---------------------------------------------------------------------------

/// The `addMethod` registration whose **name literal** the cursor sits on.
#[must_use]
pub fn registration_at(
    doc: IlxDocument<'_>,
    line: u32,
    character: u32,
) -> Option<IlxMethodRegistration> {
    let index = LineIndex::new(doc.text);
    let offset = index.offset_at_utf16(line, Utf16Col::new(character), doc.text);
    extension_registrations(doc.text)
        .into_iter()
        .find(|reg| reg.name_span.start() <= offset && offset <= reg.name_span.end())
}

/// Every site of a registered method: the registration itself, and every
/// literal `ILX::call` / `ILX::notify` of it in the workspace's `rules/`.
///
/// The document's own text is used for the registration's location, so an
/// unsaved edit still lands on the right column.
#[must_use]
pub fn references_from_registration(
    doc: IlxDocument<'_>,
    ctx: IlxContext<'_>,
    registration: &IlxMethodRegistration,
) -> Vec<IlxLocation> {
    let mut out = vec![location(
        doc.path,
        doc.text,
        registration.name_span,
        IlxSite::Registration,
    )];
    let Some((workspace, extension)) = extension_of_entry(doc.path) else {
        return out;
    };
    // Which `PLUGIN` word reaches this workspace. A declaration wins over the
    // directory-name convention for the same reason it does on the Tcl side —
    // the user has said what the plugin is called — and a workspace declared
    // under more than one plugin name is searched for all of them.
    let declared: Vec<String> = ctx
        .plugins
        .iter()
        .filter(|entry| entry.workspace == workspace)
        .map(|entry| entry.plugin.clone())
        .collect();
    let plugins = if declared.is_empty() {
        directory_name(&workspace).into_iter().collect()
    } else {
        declared
    };
    let targets: Vec<IlxExtension> = plugins
        .into_iter()
        .map(|plugin| IlxExtension {
            plugin,
            extension: extension.clone(),
        })
        .collect();
    let extra = ctx.extra_rules_for_workspace(&workspace);
    for rule in rule_files(&workspace, &extra, ctx.files.store) {
        let Some(text) = ctx.files.read(&rule) else {
            continue;
        };
        for target in &targets {
            out.extend(call_sites_in(
                &rule,
                &text,
                ctx,
                Some(target),
                &registration.name,
            ));
        }
    }
    out
}

/// Whether `path` is the resolved entry point of an ILX extension — the gate
/// that decides whether a non-Tcl document is worth looking at at all.
#[must_use]
pub fn is_extension_entry(path: &Path, files: IlxFiles<'_>) -> bool {
    let Some((workspace, extension)) = extension_of_entry(path) else {
        return false;
    };
    let dir = workspace.join(EXTENSIONS_DIR).join(&extension);
    entry_file(&dir, files).is_some_and(|entry| entry == path)
}

/// Split `…/<workspace>/extensions/<extension>/<entry…>` into the workspace
/// directory and the extension name.
///
/// Returns `None` for any path that is not under an `extensions/<name>/`
/// directory, which is every ordinary file in a project.
fn extension_of_entry(path: &Path) -> Option<(PathBuf, String)> {
    let mut dir = path.parent()?;
    // Walk up to the directory whose own parent is `extensions/`; a nested
    // entry point (`package.json`'s `main: "lib/server.js"`) sits deeper.
    for _ in 0..MAX_ANCESTORS {
        let parent = dir.parent()?;
        if directory_name(parent).is_some_and(|name| name == EXTENSIONS_DIR) {
            let workspace = parent.parent()?.to_path_buf();
            return Some((workspace, directory_name(dir)?));
        }
        dir = parent;
    }
    None
}

// ---------------------------------------------------------------------------
// Extension discovery
// ---------------------------------------------------------------------------

/// One located extension: where its sources are, and which workspace holds it.
struct ExtensionSite {
    /// The ILX workspace directory (the one holding `extensions/` and
    /// `rules/`).
    workspace: PathBuf,
    /// The resolved entry point (`index.js`, or `package.json`'s `main`).
    entry: PathBuf,
    /// Extra directories the configuration named as holding callers of this
    /// plugin — see [`IlxPluginRoot::extra_rule_dirs`]. Empty unless declared.
    extra_rules: Vec<PathBuf>,
}

/// Find the extension `target` names, relative to `document`.
///
/// See the module docs for the association rule.  The result is a hard
/// abstention on anything but exactly one match.
fn locate_extension(
    document: &Path,
    target: &IlxExtension,
    ctx: IlxContext<'_>,
) -> Result<ExtensionSite, IlxUnresolved> {
    let mut found: Vec<ExtensionSite> = Vec::new();
    for (workspace, extra_rules) in candidate_workspaces(document, target, ctx) {
        let dir = workspace.join(EXTENSIONS_DIR).join(&target.extension);
        if !ctx.files.is_dir(&dir) {
            continue;
        }
        let Some(entry) = entry_file(&dir, ctx.files) else {
            continue;
        };
        if found.iter().any(|site| site.entry == entry) {
            continue;
        }
        found.push(ExtensionSite {
            workspace,
            entry,
            extra_rules,
        });
    }
    match found.len() {
        0 => Err(IlxUnresolved::ExtensionNotFound),
        1 => Ok(found.remove(0)),
        _ => Err(IlxUnresolved::ExtensionAmbiguous),
    }
}

/// The workspace directories that may hold `target`, each with the extra rule
/// directories declared alongside it.
///
/// A configured association *replaces* the convention for that plugin name
/// rather than adding to it: the user has said where the plugin is, so a
/// directory that merely shares its name is not a second candidate (which
/// would make the declaration produce an ambiguity instead of an answer).
fn candidate_workspaces(
    document: &Path,
    target: &IlxExtension,
    ctx: IlxContext<'_>,
) -> Vec<(PathBuf, Vec<PathBuf>)> {
    let declared: Vec<(PathBuf, Vec<PathBuf>)> = ctx
        .declared(&target.plugin)
        .map(|entry| (entry.workspace.clone(), entry.extra_rule_dirs.clone()))
        .collect();
    if !declared.is_empty() {
        return declared;
    }
    // The convention: `PLUGIN` names the workspace directory, looked for along
    // the document's own ancestors only — never by scanning the tree.
    let mut workspaces: Vec<PathBuf> = Vec::new();
    let mut dir = document.parent();
    for _ in 0..MAX_ANCESTORS {
        let Some(current) = dir else { break };
        // The ancestor *is* the plugin's workspace…
        if directory_name(current).is_some_and(|name| name == target.plugin) {
            push_unique(&mut workspaces, current.to_path_buf());
        }
        // …or holds it as a sibling directory.
        push_unique(&mut workspaces, current.join(&target.plugin));
        dir = current.parent();
    }
    workspaces
        .into_iter()
        .map(|workspace| (workspace, Vec::new()))
        .collect()
}

/// The entry point of the extension directory `dir`.
///
/// `package.json`'s `main` when it names a readable file, else `index.js`,
/// else nothing — the fallback order node itself documents, with the extra
/// requirement that the file actually exist so a stale `main` cannot point
/// navigation at a path that is not there.
fn entry_file(dir: &Path, files: IlxFiles<'_>) -> Option<PathBuf> {
    let manifest = files.read(&dir.join("package.json"));
    let declared = dir.join(extension_entry_file(manifest.as_deref()));
    if files.is_file(&declared) {
        return Some(declared);
    }
    let fallback = dir.join("index.js");
    files.is_file(&fallback).then_some(fallback)
}

/// The rule files to search for callers: the immediate children of the
/// workspace's `rules/` directory, plus every Tcl source under each **declared**
/// extra directory.
///
/// The `rules/` half is deliberately not recursive and is not the whole
/// workspace: the documented layout puts every rule directly in `rules/`, and a
/// deeper walk there would turn one find-references into a tree scan of a
/// directory the user never pointed at.
///
/// A declared directory is different — the user named it precisely because
/// their iRules do not live in `rules/` — so it *is* walked, bounded by
/// [`MAX_RULE_DEPTH`] and [`MAX_RULE_DIRS`] so a mapping that accidentally
/// names a repository root still costs a bounded number of `read_dir` calls
/// rather than an unbounded one.
fn rule_files(
    workspace: &Path,
    extra: &[PathBuf],
    store: &dyn crate::vfs::SourceStore,
) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = store
        .read_dir(&workspace.join(RULES_DIR))
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.is_file)
        .map(|entry| entry.path)
        .filter(|path| is_tcl_source(path))
        .collect();
    let mut budget = MAX_RULE_DIRS;
    for root in extra {
        collect_tcl_sources(root, store, 0, &mut budget, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

/// Depth-first walk of `dir` for Tcl sources, bounded in both depth and total
/// directories visited (`budget`, shared across the roots of one request).
fn collect_tcl_sources(
    dir: &Path,
    store: &dyn crate::vfs::SourceStore,
    depth: usize,
    budget: &mut usize,
    out: &mut Vec<PathBuf>,
) {
    if depth > MAX_RULE_DEPTH || *budget == 0 {
        return;
    }
    *budget -= 1;
    let Ok(entries) = store.read_dir(dir) else {
        return;
    };
    for entry in entries {
        if entry.is_file {
            if is_tcl_source(&entry.path) {
                out.push(entry.path);
            }
        } else if entry.is_dir {
            collect_tcl_sources(&entry.path, store, depth + 1, budget, out);
        }
    }
}

/// Whether `path` carries one of the Tcl-family source extensions the server
/// indexes — the same list the workspace scan filters on.
fn is_tcl_source(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            tcl_registry::dialects::TCL_SOURCE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(ext))
        })
}

/// A directory's own name, as a `String`.
fn directory_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

/// A span in `text` as a location in `path`.
fn location(path: &Path, text: &str, span: Span, kind: IlxSite) -> IlxLocation {
    let index = LineIndex::new(text);
    IlxLocation {
        path: path.to_path_buf(),
        range: span_to_range(text, &index, span),
        kind,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IlxContext, IlxDocument, IlxPluginRoot, IlxSite, IlxTarget, IlxUnresolved, definition,
        is_extension_entry, method_call_at, references, references_from_registration,
        registration_at,
    };
    use crate::vfs::MemoryStore;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tcl_dialect::model::{Family, SurfaceLayer};
    use tcl_registry::CommandRegistry;

    const RULE: &str = concat!(
        "when HTTP_REQUEST {\n",
        "    set handle [ILX::init my_plugin my_extension]\n",
        "    set reply [ILX::call $handle my_js_function [HTTP::uri]]\n",
        "    ILX::notify $handle my_js_function logged\n",
        "}\n",
    );

    const EXTENSION: &str = concat!(
        "var f5 = require('f5-nodejs');\n",
        "var ilx = new f5.ILXServer();\n",
        "ilx.addMethod('my_js_function', function (req, res) {\n",
        "    res.reply('ok');\n",
        "});\n",
        "ilx.listen();\n",
    );

    fn registry() -> CommandRegistry {
        let mut registry = CommandRegistry::build_default();
        registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        registry
    }

    /// A store laid out the way BIG-IP lays an ILX workspace out.
    fn workspace_store() -> MemoryStore {
        let store = MemoryStore::new();
        store.upsert("/w/my_plugin/rules/rule1.tcl", RULE.as_bytes().to_vec());
        store.upsert(
            "/w/my_plugin/extensions/my_extension/index.js",
            EXTENSION.as_bytes().to_vec(),
        );
        store
    }

    #[test]
    fn a_literal_call_resolves_to_the_registration() {
        let store = workspace_store();
        let registry = registry();
        let doc = IlxDocument {
            path: Path::new("/w/my_plugin/rules/rule1.tcl"),
            text: RULE,
        };
        let ctx = IlxContext::new(&registry, &store);
        // The `my_js_function` word of the `ILX::call` line.
        let call = method_call_at(doc, ctx, 2, 34).expect("a method word under the cursor");
        assert_eq!(call.method, "my_js_function");
        let IlxTarget::Resolved(location) = definition(doc, ctx, &call) else {
            panic!("expected a resolved registration");
        };
        assert_eq!(
            location.path,
            Path::new("/w/my_plugin/extensions/my_extension/index.js")
        );
        assert_eq!(location.range.start_line, 2);
    }

    #[test]
    fn references_reach_both_call_sites_and_the_registration() {
        let store = workspace_store();
        let registry = registry();
        let doc = IlxDocument {
            path: Path::new("/w/my_plugin/rules/rule1.tcl"),
            text: RULE,
        };
        let ctx = IlxContext::new(&registry, &store);
        let call = method_call_at(doc, ctx, 2, 34).expect("a method word under the cursor");
        let found = references(doc, ctx, &call);
        assert_eq!(found.len(), 3, "{found:?}");
        assert_eq!(
            found
                .iter()
                .filter(|l| l.path.ends_with("index.js"))
                .count(),
            1
        );
        assert_eq!(
            found
                .iter()
                .filter(|l| l.path.ends_with("rule1.tcl"))
                .count(),
            2,
            "the ILX::call and the ILX::notify are both sites"
        );
        // The registration is the declaration, and is labelled as such so a
        // caller can drop it for `includeDeclaration: false`.
        assert_eq!(
            found
                .iter()
                .filter(|l| l.kind == IlxSite::Registration)
                .map(|l| l.path.clone())
                .collect::<Vec<_>>(),
            vec![PathBuf::from(
                "/w/my_plugin/extensions/my_extension/index.js"
            )]
        );
    }

    #[test]
    fn references_from_the_registration_reach_the_rules() {
        let store = workspace_store();
        let registry = registry();
        let js = Path::new("/w/my_plugin/extensions/my_extension/index.js");
        let doc = IlxDocument {
            path: js,
            text: EXTENSION,
        };
        let ctx = IlxContext::new(&registry, &store);
        let registration = registration_at(doc, 2, 16).expect("a registration under the cursor");
        assert_eq!(registration.name, "my_js_function");
        let found = references_from_registration(doc, ctx, &registration);
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(is_extension_entry(js, ctx.files));
    }

    /// A plugin whose name is *not* its workspace directory: the layout alone
    /// cannot associate the two, so the declaration is the only thing that can.
    const RENAMED_RULE: &str = concat!(
        "when HTTP_REQUEST {\n",
        "    set handle [ILX::init prod_plugin my_extension]\n",
        "    set reply [ILX::call $handle my_js_function [HTTP::uri]]\n",
        "}\n",
    );

    /// The same workspace as [`workspace_store`], but under a directory whose
    /// name is not the plugin's — `create ilx plugin prod_plugin
    /// from-workspace ws_alpha`.
    fn renamed_plugin_store() -> MemoryStore {
        let store = MemoryStore::new();
        store.upsert(
            "/w/ws_alpha/rules/rule1.tcl",
            RENAMED_RULE.as_bytes().to_vec(),
        );
        store.upsert(
            "/w/ws_alpha/extensions/my_extension/index.js",
            EXTENSION.as_bytes().to_vec(),
        );
        store
    }

    fn declared(plugin: &str, workspace: &str, extra: &[&str]) -> Vec<IlxPluginRoot> {
        vec![IlxPluginRoot {
            plugin: plugin.to_owned(),
            workspace: PathBuf::from(workspace),
            extra_rule_dirs: extra.iter().map(PathBuf::from).collect(),
        }]
    }

    #[test]
    fn a_plugin_named_unlike_its_directory_needs_a_declaration() {
        let store = renamed_plugin_store();
        let registry = registry();
        let doc = IlxDocument {
            path: Path::new("/w/ws_alpha/rules/rule1.tcl"),
            text: RENAMED_RULE,
        };
        // Undeclared: the directory-name convention has nothing to match, and
        // guessing from the extension name alone is what criterion 4 forbids.
        let bare = IlxContext::new(&registry, &store);
        let call = method_call_at(doc, bare, 2, 34).expect("a method word");
        assert!(matches!(
            definition(doc, bare, &call),
            IlxTarget::Unresolved(IlxUnresolved::ExtensionNotFound)
        ));
        // Declared: the user has said where `prod_plugin` lives.
        let roots = declared("prod_plugin", "/w/ws_alpha", &[]);
        let ctx = bare.with_plugins(&roots);
        let IlxTarget::Resolved(location) = definition(doc, ctx, &call) else {
            panic!("expected the declared workspace to resolve");
        };
        assert_eq!(
            location.path,
            Path::new("/w/ws_alpha/extensions/my_extension/index.js")
        );
    }

    #[test]
    fn a_declaration_replaces_the_convention_rather_than_competing_with_it() {
        // Both a directory literally named `my_plugin` *and* a declaration
        // pointing elsewhere. Treating the declaration as one more candidate
        // would make it an ambiguity — an answer strictly worse than the one
        // the user asked for — so it replaces the convention outright.
        let store = MemoryStore::new();
        store.upsert("/w/my_plugin/rules/rule1.tcl", RULE.as_bytes().to_vec());
        store.upsert(
            "/w/my_plugin/extensions/my_extension/index.js",
            EXTENSION.as_bytes().to_vec(),
        );
        store.upsert(
            "/w/ws_beta/extensions/my_extension/index.js",
            EXTENSION.as_bytes().to_vec(),
        );
        let registry = registry();
        let doc = IlxDocument {
            path: Path::new("/w/my_plugin/rules/rule1.tcl"),
            text: RULE,
        };
        let roots = declared("my_plugin", "/w/ws_beta", &[]);
        let ctx = IlxContext::new(&registry, &store).with_plugins(&roots);
        let call = method_call_at(doc, ctx, 2, 34).expect("a method word");
        let IlxTarget::Resolved(location) = definition(doc, ctx, &call) else {
            panic!("expected the declared workspace to win outright");
        };
        assert_eq!(
            location.path,
            Path::new("/w/ws_beta/extensions/my_extension/index.js")
        );
    }

    #[test]
    fn declared_rule_directories_are_searched_recursively() {
        // A repository that keeps its iRules outside the workspace it builds:
        // `rules/` holds nothing, and the caller is two directories down in a
        // declared tree.
        let store = MemoryStore::new();
        store.upsert(
            "/w/ws_alpha/extensions/my_extension/index.js",
            EXTENSION.as_bytes().to_vec(),
        );
        store.upsert(
            "/repo/irules/http/rule1.tcl",
            RENAMED_RULE.as_bytes().to_vec(),
        );
        let registry = registry();
        let doc = IlxDocument {
            path: Path::new("/w/ws_alpha/extensions/my_extension/index.js"),
            text: EXTENSION,
        };
        let roots = declared("prod_plugin", "/w/ws_alpha", &["/repo/irules"]);
        let ctx = IlxContext::new(&registry, &store).with_plugins(&roots);
        let registration = registration_at(doc, 2, 16).expect("the addMethod name");
        let found = references_from_registration(doc, ctx, &registration);
        assert_eq!(
            found
                .iter()
                .filter(|l| l.path == Path::new("/repo/irules/http/rule1.tcl"))
                .count(),
            1,
            "the declared tree's caller must be found: {found:?}"
        );
        // Undeclared, the same request finds only the registration itself —
        // the workspace's own `rules/` is empty and nothing else is searched.
        let bare = IlxContext::new(&registry, &store);
        assert_eq!(
            references_from_registration(doc, bare, &registration).len(),
            1
        );
    }

    #[test]
    fn a_plugin_word_that_names_no_workspace_abstains() {
        let store = workspace_store();
        let registry = registry();
        let text = RULE.replace("my_plugin my_extension", "other_plugin my_extension");
        let doc = IlxDocument {
            path: Path::new("/w/my_plugin/rules/rule1.tcl"),
            text: &text,
        };
        let ctx = IlxContext::new(&registry, &store);
        let call = method_call_at(doc, ctx, 2, 34).expect("the method word is still literal");
        assert_eq!(
            definition(doc, ctx, &call),
            IlxTarget::Unresolved(IlxUnresolved::ExtensionNotFound)
        );
    }

    #[test]
    fn a_dynamic_handle_abstains_with_a_reason() {
        let store = workspace_store();
        let registry = registry();
        let text = RULE.replace("ILX::init my_plugin", "ILX::init $plugin");
        let doc = IlxDocument {
            path: Path::new("/w/my_plugin/rules/rule1.tcl"),
            text: &text,
        };
        let ctx = IlxContext::new(&registry, &store);
        let call = method_call_at(doc, ctx, 2, 34).expect("the method word is still literal");
        assert_eq!(
            definition(doc, ctx, &call),
            IlxTarget::Unresolved(IlxUnresolved::HandleNotStatic)
        );
    }

    #[test]
    fn a_duplicate_registration_is_an_ambiguity_not_a_target() {
        let store = workspace_store();
        store.upsert(
            "/w/my_plugin/extensions/my_extension/index.js",
            format!("{EXTENSION}ilx.addMethod('my_js_function', other);\n").into_bytes(),
        );
        let registry = registry();
        let doc = IlxDocument {
            path: Path::new("/w/my_plugin/rules/rule1.tcl"),
            text: RULE,
        };
        let ctx = IlxContext::new(&registry, &store);
        let call = method_call_at(doc, ctx, 2, 34).expect("a method word under the cursor");
        assert!(
            matches!(definition(doc, ctx, &call), IlxTarget::Ambiguous { count, .. } if count == 2)
        );
    }

    #[test]
    fn a_package_main_entry_point_is_followed() {
        let store = MemoryStore::new();
        store.upsert("/w/p/rules/r.tcl", RULE.as_bytes().to_vec());
        store.upsert(
            "/w/p/extensions/my_extension/package.json",
            br#"{"main": "lib/server.js"}"#.to_vec(),
        );
        store.upsert(
            "/w/p/extensions/my_extension/lib/server.js",
            EXTENSION.as_bytes().to_vec(),
        );
        let registry = registry();
        let text = RULE.replace("my_plugin", "p");
        let doc = IlxDocument {
            path: Path::new("/w/p/rules/r.tcl"),
            text: &text,
        };
        let ctx = IlxContext::new(&registry, &store);
        let call = method_call_at(doc, ctx, 2, 34).expect("a method word under the cursor");
        let IlxTarget::Resolved(location) = definition(doc, ctx, &call) else {
            panic!("expected a resolved registration");
        };
        assert_eq!(
            location.path,
            Path::new("/w/p/extensions/my_extension/lib/server.js")
        );
        assert!(is_extension_entry(
            Path::new("/w/p/extensions/my_extension/lib/server.js"),
            ctx.files
        ));
    }

    /// An editor holding unsaved text for some files.
    #[derive(Default)]
    struct OpenBuffers(Vec<(PathBuf, Arc<str>)>);

    impl OpenBuffers {
        fn holding(path: &str, text: &str) -> Self {
            Self(vec![(PathBuf::from(path), Arc::from(text))])
        }
    }

    impl super::OpenDocuments for OpenBuffers {
        fn text(&self, path: &Path) -> Option<Arc<str>> {
            self.0
                .iter()
                .find(|(held, _)| held == path)
                .map(|(_, text)| Arc::clone(text))
        }
    }

    #[test]
    fn an_unsaved_buffer_wins_over_the_file_on_disk() {
        // The reader is one seam, so proving it on the JavaScript half proves
        // it for the sibling rules too: a registration typed but not yet saved
        // must resolve, and one deleted in the editor must stop resolving,
        // even though the disk still says otherwise (issue #1707 review).
        let store = workspace_store();
        let registry = registry();
        let js = "/w/my_plugin/extensions/my_extension/index.js";
        let doc = IlxDocument {
            path: Path::new("/w/my_plugin/rules/rule1.tcl"),
            text: RULE,
        };

        let typed = OpenBuffers::holding(
            js,
            "var f5 = require('f5-nodejs');\nvar ilx = new f5.ILXServer();\n\n\n\nilx.addMethod('my_js_function', cb);\n",
        );
        let ctx = IlxContext::new(&registry, &store).with_open_documents(&typed);
        let call = method_call_at(doc, ctx, 2, 34).expect("a method word under the cursor");
        let IlxTarget::Resolved(location) = definition(doc, ctx, &call) else {
            panic!("the editor's copy of the extension must be what is read");
        };
        assert_eq!(
            location.range.start_line, 5,
            "the range must come from the unsaved text, not the saved file"
        );

        // The same seam in the other direction: an editor that has *removed*
        // the registration must stop resolving it, however stale the disk is.
        let emptied = OpenBuffers::holding(js, "var f5 = require('f5-nodejs');\n");
        let ctx = IlxContext::new(&registry, &store).with_open_documents(&emptied);
        assert_eq!(
            definition(doc, ctx, &call),
            IlxTarget::Unresolved(IlxUnresolved::MethodNotRegistered)
        );
    }

    #[test]
    fn plain_tcl_finds_no_method_word() {
        // Criterion 5 at the provider seam: the same source, resolved against a
        // stock Tcl registry, has no ILX relation at all.
        let store = workspace_store();
        let registry = CommandRegistry::build_default();
        let doc = IlxDocument {
            path: Path::new("/w/my_plugin/rules/rule1.tcl"),
            text: RULE,
        };
        let ctx = IlxContext::new(&registry, &store);
        assert!(method_call_at(doc, ctx, 2, 34).is_none());
    }
}
