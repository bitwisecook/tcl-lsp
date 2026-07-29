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

//! Tcl package + auto-load resolution, aligned to C Tcl's own loading
//! machinery.
//!
//! # What C Tcl actually does (the reference this module mirrors)
//!
//! * **`package require Name ?ver?`** — when `Name` isn't already present, the
//!   `package unknown` handler [`tclPkgUnknown`] walks `$auto_path` (each
//!   directory *and its immediate subdirectories*, `*/pkgIndex.tcl`),
//!   `source`-ing every `pkgIndex.tcl`. Each runs
//!   `package ifneeded NAME VERSION SCRIPT`, registering the version→script
//!   map. `package require` then evaluates the best-matching version's script,
//!   which `load`s a shared library or `source`s Tcl files — *that* is what
//!   defines the package's commands. See `library/package.tcl:467`
//!   (`tclPkgUnknown`).
//!
//! * **auto-loading an unknown command** — `unknown` → `auto_load` consults the
//!   `auto_index` array, populated by [`auto_load_index`] from `tclIndex` files
//!   found on `$auto_path`. A `tclIndex` maps a (qualified) command name to the
//!   file that `source`s its definition. Name lookup is qualified through
//!   [`auto_qualify`]. See `library/init.tcl:353` (`auto_load`), `:422`
//!   (`auto_load_index`), `:488` (`auto_qualify`).
//!
//! # What this module provides
//!
//! [`parse_pkg_index`] and [`parse_tcl_index`] extract, respectively, the
//! `package ifneeded` source targets and the `auto_index` proc→file mappings
//! from index-file *content* (tokenised with the real Tcl lexer, so nested
//! `[file join $dir x]` / quoted / braced forms are handled structurally, not
//! by regex). [`auto_qualify`] is a faithful port of the name-qualification
//! rules. [`PackageResolver`] ties them together over a set of search paths
//! (an `auto_path`), resolving `package require` and auto-loaded command names
//! to the files C Tcl would load.
//!
//! The differential tests in this module verify the output against a real
//! `tclsh` (`auto_qualify`, and `pkg_mkIndex` / `auto_mkindex`-generated index
//! files) so the port cannot silently drift from C Tcl.
//!
//! [`tclPkgUnknown`]: https://www.tcl-lang.org/man/tcl/TclCmd/package.htm
//! [`auto_load_index`]: https://www.tcl-lang.org/man/tcl/TclCmd/library.htm

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tcl_lexer::{Lexer, Token, TokenType};

/// Metadata for one `package ifneeded` declaration discovered in a
/// `pkgIndex.tcl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInfo {
    /// Package name (the `NAME` in `package ifneeded NAME VERSION …`).
    pub name: String,
    /// Declared version.
    pub version: String,
    /// Absolute paths to the implementation files the `ifneeded` script
    /// would `source` / `load`.
    pub source_files: Vec<PathBuf>,
    /// The `pkgIndex.tcl` that declared this entry.
    pub pkg_index_path: PathBuf,
    /// What must hold for the interpreter to reach this declaration at all —
    /// empty for the ordinary unguarded index file.  See
    /// [`reachability`] for the guard shapes modelled.
    pub conditions: reachability::Conditions,
}

impl PackageInfo {
    /// Whether a Tcl release running this index registers the declaration.
    ///
    /// `target` is the release the workspace targets; `None` (an unversioned
    /// `tcl` dialect, or a non-Tcl one) leaves every version guard undecided,
    /// so a guarded entry reports [`reachability::Availability::Conditional`]
    /// rather than being guessed either way.
    #[must_use]
    pub fn availability(
        &self,
        target: Option<tcl_dialect::TclVersion>,
    ) -> reachability::Availability {
        self.conditions.availability(target)
    }
}

/// One `proc → file` mapping from a `tclIndex` auto-load index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoIndexEntry {
    /// The (possibly namespace-qualified) command name, e.g. `::http::geturl`.
    pub proc_name: String,
    /// Absolute path to the file that defines it.
    pub source_file: PathBuf,
}

// ---------------------------------------------------------------------------
// Tokeniser-based word/command walking (mirrors resolver.py helpers).
// ---------------------------------------------------------------------------

/// Tokenise `text` and group tokens into commands of words.
///
/// Each command is a `Vec` of words; each word is the run of adjacent
/// non-separator tokens between `Sep` / `Eol` boundaries (so `a$b[c]` is one
/// word). `{*}` expansion markers and comments are word/command boundaries,
/// matching `resolver.py::_walk_command_words`.
pub(super) fn walk_command_words(text: &str) -> Vec<Vec<Vec<Token>>> {
    let tokens = Lexer::new(text).tokenise_all().unwrap_or_default();
    let mut commands: Vec<Vec<Vec<Token>>> = Vec::new();
    let mut words: Vec<Vec<Token>> = Vec::new();
    let mut word: Vec<Token> = Vec::new();

    for tok in tokens {
        match tok.kind {
            TokenType::Comment | TokenType::Eof => {}
            TokenType::Eol => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                if !words.is_empty() {
                    commands.push(std::mem::take(&mut words));
                }
            }
            TokenType::Sep | TokenType::Expand => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            _ => word.push(tok),
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    if !words.is_empty() {
        commands.push(words);
    }
    commands
}

/// The verbatim source slice spanning `word` within `text`.
///
/// For a `[...]` command-substitution word the lexer's `Cmd` token excludes
/// the closing `]` from its span, so this returns `[list …` (no trailing
/// bracket) — exactly as `resolver.py::_word_raw` did.
pub(super) fn word_raw<'t>(text: &'t str, word: &[Token]) -> &'t str {
    let start = word[0].span.start() as usize;
    let end = word[word.len() - 1].span.end() as usize;
    text.get(start..end).unwrap_or("")
}

/// The inner script text of `word` if it is a single wrapper word
/// (`[...]` command substitution, `{...}` braced literal, or `"..."` quoted
/// word); `None` for a bare word. Mirrors `resolver.py::_word_unwrap`.
pub(super) fn word_unwrap(text: &str, word: &[Token]) -> Option<String> {
    if word.len() == 1 {
        let tok = word[0];
        if matches!(tok.kind, TokenType::Cmd | TokenType::Str) {
            let start = tok.span.start() as usize + tok.content_offset as usize;
            let end = tok.span.end() as usize;
            return text.get(start..end).map(str::to_owned);
        }
    }
    let open_off = word[0].span.start() as usize;
    if word[0].in_quote && text.as_bytes().get(open_off) == Some(&b'"') {
        // Locate the closing quote with a backslash-aware forward scan; the
        // inner script is the span between the two quotes.
        let bytes = text.as_bytes();
        let mut i = open_off + 1;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 2,
                b'"' => return text.get(open_off + 1..i).map(str::to_owned),
                _ => i += 1,
            }
        }
        return text.get(open_off + 1..).map(str::to_owned);
    }
    None
}

/// Extract the package-relative filename from a `source` argument word.
///
/// Accepts the two structural forms real index files use, via the tokeniser
/// (mirrors `resolver.py::_source_filename`):
///
/// * `[file join $dir X …]` — path components after `$dir` joined by `/`.
/// * `$dir/X` — the path tail after a bare `$dir/` prefix.
fn source_filename(text: &str, arg: &[Token]) -> Option<String> {
    let raw = word_raw(text, arg);

    // `[file join $dir X]` — a single command-substitution word.
    if arg.len() == 1 && arg[0].kind == TokenType::Cmd {
        let start = arg[0].span.start() as usize + arg[0].content_offset as usize;
        let end = arg[0].span.end() as usize;
        let inner = text.get(start..end)?;
        let cmds = walk_command_words(inner);
        if cmds.len() != 1 {
            return None;
        }
        let words = &cmds[0];
        if words.len() < 4 {
            return None;
        }
        if word_raw(inner, &words[0]) != "file" || word_raw(inner, &words[1]) != "join" {
            return None;
        }
        if !matches!(word_raw(inner, &words[2]), "$dir" | "${dir}") {
            return None;
        }
        // `file join $dir a b c` joins path *components* with the directory
        // separator, not a space: the tail is `a/b/c`, so
        // `[file join $dir src impl.tcl]` resolves `src/impl.tcl` (issue 177).
        // Each component's surrounding quotes/braces are stripped individually
        // so a quoted component with no separator survives intact.
        let tail = words[3..]
            .iter()
            .map(|w| {
                word_raw(inner, w)
                    .trim()
                    .trim_matches(|c| c == '"' || c == '{' || c == '}')
            })
            .collect::<Vec<_>>()
            .join("/");
        return Some(tail);
    }

    // `$dir/X` — bare variable substitution immediately followed by `/tail`.
    for prefix in ["$dir/", "${dir}/"] {
        if let Some(rest) = raw.strip_prefix(prefix) {
            return Some(rest.trim().trim_matches('"').to_owned());
        }
    }
    None
}

/// Cap on `[...]`/`{...}`/`"..."` wrapper-word nesting depth
/// [`collect_source_targets`] will descend into — pathologically deep
/// nesting in a `pkgIndex.tcl` `package ifneeded` body could otherwise
/// blow the native stack when the workspace indexer scans it. See
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
const MAX_SOURCE_TARGET_SCAN_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

/// Walk `commands` collecting `source <arg>` targets that exist under
/// `pkg_dir` (per `exists`), descending into `[...]` / `{...}` / `"..."`
/// wrapper words. Mirrors `resolver.py::_collect_source_targets`.
///
/// `depth` is the nesting level of this call (0 at the top); past
/// [`MAX_SOURCE_TARGET_SCAN_DEPTH`] this stops descending into wrapper
/// words rather than recursing further.
fn collect_source_targets(
    text: &str,
    commands: &[Vec<Vec<Token>>],
    pkg_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
    out: &mut Vec<PathBuf>,
    depth: u32,
) {
    if MAX_SOURCE_TARGET_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    for words in commands {
        for (i, word) in words.iter().enumerate() {
            if word_raw(text, word) == "source"
                && let Some(next) = words.get(i + 1)
                && let Some(filename) = source_filename(text, next)
            {
                let full = pkg_dir.join(&filename);
                if exists(&full) {
                    out.push(full);
                }
            }
        }
        for word in words {
            if let Some(inner) = word_unwrap(text, word) {
                collect_source_targets(
                    &inner,
                    &walk_command_words(&inner),
                    pkg_dir,
                    exists,
                    out,
                    depth + 1,
                );
            }
        }
    }
}

/// A package version token's accepted *format*: digits/dots with an optional
/// alpha/beta suffix (`1.0`, `2.1`, `1.0b1`). A narrow gate on an
/// already-tokenised word, matching `resolver.py`'s `_VERSION_RE`.
fn is_version_word(word: &str) -> bool {
    let (head, suffix) = match word.find(['a', 'b']) {
        Some(i) => (&word[..i], &word[i..]),
        None => (word, ""),
    };
    if head.is_empty() || !head.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return false;
    }
    if suffix.is_empty() {
        return true;
    }
    // `[ab]\d+`
    let mut it = suffix.bytes();
    let first = it.next();
    if !matches!(first, Some(b'a' | b'b')) {
        return false;
    }
    let digits = &suffix[1..];
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

// ---------------------------------------------------------------------------
// Public parsers.
// ---------------------------------------------------------------------------

/// Parse a `pkgIndex.tcl`'s `content`, returning a [`PackageInfo`] per
/// `package ifneeded` declaration whose script names (existing) source files.
///
/// `pkg_dir` is the directory the index lives in (`$dir` in the script);
/// `exists` decides whether a candidate implementation file is real (pass
/// `|p| p.is_file()` for the filesystem; a closure for tests). When an
/// `ifneeded` body names no source file, this falls back to every `*.tcl`
/// (other than `pkgIndex.tcl`) reported by `list_tcl_files` — matching C
/// Tcl-era `pkg_mkIndex` output, where the index always sources concrete
/// files but hand-written indices sometimes don't.
///
/// Declarations are found through the [`reachability`] scan, so a declaration
/// nested inside an `if` branch is found (the TEA "pick a Tcl 8 or Tcl 9
/// library" shape) and each one carries the guard conditions standing in
/// front of it. Nothing is filtered here: a caller that targets a specific
/// Tcl release asks [`PackageInfo::availability`], and one that does not
/// (go-to-definition, which is deliberately permissive) ignores the field.
pub fn parse_pkg_index(
    content: &str,
    pkg_dir: &Path,
    pkg_index_path: &Path,
    exists: &dyn Fn(&Path) -> bool,
    list_tcl_files: &dyn Fn(&Path) -> Vec<PathBuf>,
) -> Vec<PackageInfo> {
    // A `pkgIndex.tcl` is plain Tcl whatever dialect the workspace's own
    // documents are in — `tclPkgUnknown` sources it in the ordinary
    // interpreter — so the guard vocabulary (`package`, `if`, `return`) comes
    // from the core registry, not the document's.  The lookup is cached and
    // returns a `&'static`, so this costs nothing per file.
    let registry = tcl_registry::registry_for_dialect("tcl");
    let mut out = Vec::new();
    reachability::scan(content, registry, &mut |reached| {
        let reachability::Reached {
            text,
            words,
            conditions,
        } = reached;
        if words.len() < 5 {
            return;
        }
        let name = word_raw(text, &words[2]);
        let version = word_raw(text, &words[3]);
        if !is_version_word(version) {
            return;
        }
        let body = &words[4..];
        let mut source_files = Vec::new();
        collect_source_targets(
            text,
            &[body.to_vec()],
            pkg_dir,
            exists,
            &mut source_files,
            0,
        );
        if source_files.is_empty() {
            for f in list_tcl_files(pkg_dir) {
                if f.file_name().is_some_and(|n| n != "pkgIndex.tcl") {
                    source_files.push(f);
                }
            }
        }
        if !source_files.is_empty() {
            out.push(PackageInfo {
                name: name.to_owned(),
                version: version.to_owned(),
                source_files,
                pkg_index_path: pkg_index_path.to_owned(),
                conditions,
            });
        }
    });
    out
}

/// Parse a `tclIndex` auto-load index's `content` into proc→file mappings.
///
/// Handles both formats `auto_load_index` reads (`library/init.tcl:446`):
///
/// * **version 2.0** — `set auto_index(NAME) [list source [file join $dir F]]`
///   (also `::tcl::Pkg::source` and an `-encoding utf-8` flag), and
/// * **old format** — `# Tcl autoload index file: each line identifies a Tcl`
///   followed by `NAME FILE` lines.
///
/// `index_dir` is the directory the index lives in; `exists` gates referenced
/// files (pass `|p| p.is_file()`), matching `auto_index.py`.
pub fn parse_tcl_index(
    content: &str,
    index_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
) -> Vec<AutoIndexEntry> {
    let mut entries = Vec::new();
    let first_line = content.lines().next().unwrap_or("");
    let old_format = first_line == "# Tcl autoload index file: each line identifies a Tcl";

    if old_format {
        for line in content.lines().skip(1) {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            // `NAME FILE` — exactly two whitespace-separated words.
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 2 {
                continue;
            }
            push_auto_entry(parts[0], parts[1], index_dir, exists, &mut entries);
        }
        return entries;
    }

    // Version 2.0: structurally match each `set auto_index(NAME) [list … source
    // … [file join $dir FILE]]` command via the tokeniser.
    for words in walk_command_words(content) {
        if words.len() != 3 {
            continue;
        }
        if word_raw(content, &words[0]) != "set" {
            continue;
        }
        let Some(proc_name) = auto_index_key(word_raw(content, &words[1])) else {
            continue;
        };
        let Some(inner) = word_unwrap(content, &words[2]) else {
            continue;
        };
        // `[list ?::tcl::Pkg::?source ?-encoding utf-8? [file join $dir FILE]]`.
        let cmds = walk_command_words(&inner);
        if cmds.len() != 1 {
            continue;
        }
        let lw = &cmds[0];
        let Some(src_arg) = list_source_file_arg(&inner, lw) else {
            continue;
        };
        if let Some(filename) = source_filename(&inner, src_arg) {
            push_auto_entry(proc_name, &filename, index_dir, exists, &mut entries);
        }
    }
    entries
}

/// Extract `NAME` from an `auto_index(NAME)` array-element word.
fn auto_index_key(raw: &str) -> Option<&str> {
    let inner = raw.strip_prefix("auto_index(")?.strip_suffix(')')?;
    Some(inner.trim())
}

/// In a `[list …]` body, find the argument word that names the source file:
/// the word *after* a `source` / `::tcl::Pkg::source` word, skipping an
/// `-encoding <enc>` option pair.
fn list_source_file_arg<'w>(inner: &str, words: &'w [Vec<Token>]) -> Option<&'w Vec<Token>> {
    let mut i = 0;
    // Find the `source` verb.
    while i < words.len() {
        let w = word_raw(inner, &words[i]);
        if w == "source" || w == "::tcl::Pkg::source" {
            break;
        }
        i += 1;
    }
    i += 1; // step past the source verb
    // Optional `-encoding <enc>` pair.
    if words.get(i).map(|w| word_raw(inner, w)) == Some("-encoding") {
        i += 2;
    }
    words.get(i)
}

fn push_auto_entry(
    proc_name: &str,
    filename: &str,
    index_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
    out: &mut Vec<AutoIndexEntry>,
) {
    let filename = filename.trim().trim_matches('"');
    let full = index_dir.join(filename);
    if exists(&full) {
        out.push(AutoIndexEntry {
            proc_name: proc_name.trim().to_owned(),
            source_file: full,
        });
    }
}

/// The package names a file makes available: every literal `package require
/// NAME` (with optional `-exact`) plus every `package provide NAME`.
///
/// Used to chase the *transitive* closure of a `package require` — a package
/// whose implementation does `package require Tk` (transitively) makes `Tk`
/// available, exactly as C Tcl would once that package's `ifneeded` script
/// runs. A non-literal name (`package require $x`) is skipped: its provided
/// set is unknowable statically, and the caller treats that conservatively.
#[must_use]
pub fn package_requires_in(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    for words in walk_command_words(content) {
        if words.len() < 3 || word_raw(content, &words[0]) != "package" {
            continue;
        }
        let sub = word_raw(content, &words[1]);
        let name = match sub {
            "require" => {
                // `package require ?-exact? NAME ?version?`
                let idx = if word_raw(content, &words[2]) == "-exact" {
                    3
                } else {
                    2
                };
                words.get(idx).map(|w| word_raw(content, w))
            }
            "provide" => Some(word_raw(content, &words[2])),
            _ => None,
        };
        if let Some(name) = name
            && !name.is_empty()
            && !name.contains(['$', '[', '{'])
        {
            names.push(name.to_owned());
        }
    }
    names
}

// ---------------------------------------------------------------------------
// Name qualification (exact port of C Tcl's auto_qualify).
// ---------------------------------------------------------------------------

/// Faithful port of C Tcl's `auto_qualify` (`library/init.tcl:488`).
///
/// Returns the candidate fully-qualified names an auto-load lookup tries for
/// command `cmd` used in namespace `namespace` (a canonical namespace as from
/// `namespace current`, e.g. `::` or `::sub`). For historical reasons a global
/// command has *no* leading `::` in its `auto_index` key, so `::global` →
/// `global`. A relative name in a non-global namespace yields two candidates
/// (namespace-qualified, then global-relative).
#[must_use]
pub fn auto_qualify(cmd: &str, namespace: &str) -> Vec<String> {
    // `regsub -all {::+} $cmd :: cmd` — collapse runs of >=2 colons to `::`
    // and count how many such runs there were.
    let (cmd, n) = collapse_colons(cmd);

    if let Some(without_prefix) = cmd.strip_prefix("::").map(str::to_owned) {
        return if n > 1 {
            // (::foo::bar, *) -> ::foo::bar
            vec![cmd]
        } else {
            // (::global, *) -> global
            vec![without_prefix]
        };
    }

    if n == 0 {
        if namespace == "::" {
            // (nocolons, ::) -> nocolons
            vec![cmd]
        } else {
            // (nocolons, ::sub) -> ::sub::nocolons nocolons
            vec![format!("{namespace}::{cmd}"), cmd]
        }
    } else if namespace == "::" {
        // (foo::bar, ::) -> ::foo::bar
        vec![format!("::{cmd}")]
    } else {
        // (foo::bar, ::sub) -> ::sub::foo::bar ::foo::bar
        vec![format!("{namespace}::{cmd}"), format!("::{cmd}")]
    }
}

/// Collapse maximal runs of two-or-more `:` to `::`, returning the normalised
/// string and the number of runs collapsed (the `regsub -all {::+}` count).
fn collapse_colons(cmd: &str) -> (String, usize) {
    let bytes = cmd.as_bytes();
    let mut out = String::with_capacity(cmd.len());
    let mut n = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' {
            let mut j = i;
            while j < bytes.len() && bytes[j] == b':' {
                j += 1;
            }
            let run = j - i;
            if run >= 2 {
                out.push_str("::");
                n += 1;
            } else {
                out.push(':');
            }
            i = j;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    (out, n)
}

// ---------------------------------------------------------------------------
// PackageResolver — ties the parsers to a set of search paths (auto_path).
// ---------------------------------------------------------------------------

/// Resolves `package require` and auto-loaded command names to the files C
/// Tcl would load, by scanning `pkgIndex.tcl` / `tclIndex` files across a set
/// of search paths (an `auto_path`).
///
/// Search-path order follows Tcl's `auto_path` semantics: the **first**
/// provider discovered for a name wins (workspace-local copies should be
/// listed first so they shadow installed libraries). Later providers append
/// to the version list but never displace the head.
#[derive(Debug, Default)]
pub struct PackageResolver {
    packages: HashMap<String, Vec<PackageInfo>>,
    auto_index: HashMap<String, Vec<PathBuf>>,
    scanned_dirs: Vec<PathBuf>,
}

impl PackageResolver {
    /// A new, empty resolver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a parsed `pkgIndex.tcl`'s declarations (first provider wins).
    pub fn add_pkg_index(&mut self, infos: Vec<PackageInfo>) {
        for info in infos {
            self.packages
                .entry(info.name.clone())
                .or_default()
                .push(info);
        }
    }

    /// Record a parsed `tclIndex`'s declarations.
    pub fn add_tcl_index(&mut self, entries: Vec<AutoIndexEntry>) {
        for entry in entries {
            self.auto_index
                .entry(entry.proc_name)
                .or_default()
                .push(entry.source_file);
        }
    }

    /// Scan one directory and its **immediate subdirectories** for
    /// `pkgIndex.tcl` and `tclIndex` files, mirroring `tclPkgUnknown`'s
    /// `glob -directory $dir -join * pkgIndex.tcl` plus the `$dir/pkgIndex.tcl`
    /// case (`library/package.tcl:486-534`). Idempotent per directory.
    pub fn scan_path(&mut self, dir: &Path) {
        if !dir.is_dir() {
            return;
        }
        // The directory itself, then each immediate subdirectory.
        self.scan_single_dir(dir);
        if let Ok(read) = std::fs::read_dir(dir) {
            for entry in read.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.scan_single_dir(&path);
                }
            }
        }
    }

    /// Recursively scan a directory tree (bounded to `max_dirs` directories)
    /// for `pkgIndex.tcl` / `tclIndex` files.
    ///
    /// Unlike [`Self::scan_path`] — which follows C Tcl's `auto_path` rule of
    /// scanning a directory and its *immediate* subdirectories — this walks
    /// the whole tree, the right behaviour for an IDE workspace root where a
    /// project may nest its packages arbitrarily deep (a full recursive walk).
    /// The `max_dirs` cap keeps a huge tree from stalling the scan.
    pub fn scan_tree(&mut self, root: &Path, max_dirs: usize) {
        let mut stack = vec![root.to_path_buf()];
        let mut visited = 0usize;
        while let Some(dir) = stack.pop() {
            if visited >= max_dirs {
                break;
            }
            if !dir.is_dir() {
                continue;
            }
            visited += 1;
            self.scan_single_dir(&dir);
            if let Ok(read) = std::fs::read_dir(&dir) {
                for entry in read.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    }
                }
            }
        }
    }

    fn scan_single_dir(&mut self, dir: &Path) {
        let dir_buf = dir.to_path_buf();
        if self.scanned_dirs.contains(&dir_buf) {
            return;
        }
        self.scanned_dirs.push(dir_buf);
        let pkg_index = dir.join("pkgIndex.tcl");
        if pkg_index.is_file()
            && let Ok(content) = std::fs::read_to_string(&pkg_index)
        {
            let infos =
                parse_pkg_index(&content, dir, &pkg_index, &|p| p.is_file(), &list_tcl_files);
            self.add_pkg_index(infos);
        }
        // `tclIndex` is matched case-insensitively (`tclIndex` / `tclindex`).
        if let Ok(read) = std::fs::read_dir(dir) {
            for entry in read.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().eq_ignore_ascii_case("tclindex") {
                    let path = entry.path();
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let dir_owned = path.parent().unwrap_or(dir).to_path_buf();
                        self.add_tcl_index(parse_tcl_index(&content, &dir_owned, &|p| p.is_file()));
                    }
                }
            }
        }
    }

    /// Resolve a `package require NAME ?VERSION?` to its implementation files.
    ///
    /// With no version constraint, the first-discovered provider's files are
    /// returned (Tcl `auto_path`-order semantics). With a version, an entry
    /// whose version equals or is prefixed by the request is preferred. Empty
    /// when the package is unknown to the scanned paths.
    #[must_use]
    pub fn resolve(&self, name: &str, version: Option<&str>) -> Vec<PathBuf> {
        let Some(infos) = self.packages.get(name) else {
            return Vec::new();
        };
        if let Some(ver) = version {
            for info in infos {
                if info.version == ver || info.version.starts_with(ver) {
                    return info.source_files.clone();
                }
            }
            return Vec::new();
        }
        infos
            .first()
            .map(|i| i.source_files.clone())
            .unwrap_or_default()
    }

    /// Whether the scanned paths know how to provide `name`.
    #[must_use]
    pub fn provides(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }

    /// The transitive set of package names available once `roots` are
    /// required.
    ///
    /// Starts from `roots` (a document's own `package require` names) and, for
    /// each *resolvable* package, reads its implementation files (`read_file`)
    /// and folds in the packages they `require` / `provide`, to a fixpoint.
    /// This mirrors C Tcl evaluating each `ifneeded` script, which may itself
    /// `package require` further packages: e.g. a wrapper whose body does
    /// `package require Tk` makes `Tk` available transitively. The returned
    /// set includes `roots`. Unresolvable roots simply contribute only
    /// themselves (their provided set is unknowable here — the caller decides
    /// how to treat that).
    #[must_use]
    pub fn transitive_available_packages(
        &self,
        roots: &[String],
        read_file: &dyn Fn(&Path) -> Option<String>,
    ) -> HashSet<String> {
        let mut available: HashSet<String> = HashSet::new();
        let mut work: Vec<String> = roots.to_vec();
        while let Some(pkg) = work.pop() {
            if !available.insert(pkg.clone()) {
                continue;
            }
            for file in self.resolve(&pkg, None) {
                if let Some(content) = read_file(&file) {
                    for req in package_requires_in(&content) {
                        if !available.contains(&req) {
                            work.push(req);
                        }
                    }
                }
            }
        }
        available
    }

    /// Resolve an auto-loaded command `cmd` (used in `namespace`) to the files
    /// that would define it, trying each [`auto_qualify`] candidate in order —
    /// the same lookup `auto_load` performs against `auto_index`.
    #[must_use]
    pub fn resolve_auto_command(&self, cmd: &str, namespace: &str) -> Vec<PathBuf> {
        for candidate in auto_qualify(cmd, namespace) {
            if let Some(files) = self.auto_index.get(&candidate) {
                return files.clone();
            }
        }
        Vec::new()
    }

    /// Every package name known to the scanned paths.
    #[must_use]
    pub fn package_names(&self) -> Vec<&str> {
        self.packages.keys().map(String::as_str).collect()
    }

    /// Every auto-loadable command name known to the scanned paths.
    #[must_use]
    pub fn auto_command_names(&self) -> Vec<&str> {
        self.auto_index.keys().map(String::as_str).collect()
    }

    /// Whether the scanned `auto_path` can auto-load the bare command `cmd`
    /// (used in `namespace`) — i.e. some `tclIndex` on the path maps one of its
    /// [`auto_qualify`] candidates to a defining file.
    ///
    /// This is the package-database analogue of the built-in command registry:
    /// a command an installed library makes available through its auto-load
    /// index is as *resolvable* as a built-in, so the unknown-command (W123)
    /// check must treat it as known rather than flag it (issue #832). The
    /// decision is data-driven — it never matches on a specific command name.
    #[must_use]
    pub fn auto_loads_command(&self, cmd: &str, namespace: &str) -> bool {
        !self.resolve_auto_command(cmd, namespace).is_empty()
    }

    /// The union of command names defined by the implementation files of every
    /// package in `available`, each resolved to its source files (via
    /// [`Self::resolve`]) and then read + parsed by the caller-supplied
    /// `defined_commands` extractor.
    ///
    /// The extractor is injected (rather than parsing here) so the command-name
    /// discovery can run through the compiler's registry-driven symbol-definer
    /// machinery — the same `proc` / `oo::class` / `interp alias` set the
    /// analyser records — instead of a hand-rolled `proc`-name scan. Keeping it
    /// out of the resolver also keeps this crate's package database free of a
    /// compile dependency on the analyser and makes the method unit-testable
    /// with a stub extractor.
    ///
    /// Used by the W123 refinement for a command provided by a `pkgIndex.tcl`
    /// package with no `tclIndex` (so [`Self::auto_loads_command`] can't see it)
    /// that the document's requires — or the requires it inherits from a
    /// `source` ancestor / entry point — pull in.
    #[must_use]
    pub fn package_defined_commands(
        &self,
        available: &[String],
        target: Option<tcl_dialect::TclVersion>,
        defined_commands: &dyn Fn(&Path) -> Vec<String>,
    ) -> HashSet<String> {
        let mut out = HashSet::new();
        for pkg in available {
            for file in self.reachable_files(pkg, target) {
                out.extend(defined_commands(&file));
            }
        }
        out
    }

    /// The implementation files of `name` that a Tcl release `target` would
    /// actually load, skipping any `package ifneeded` its `pkgIndex.tcl`
    /// guards out ([`reachability`]).
    ///
    /// A declaration whose guards cannot be decided
    /// ([`reachability::Availability::Conditional`]) counts as loadable: the
    /// scan does not know it *won't* run, and treating "unsure" as "absent"
    /// would resurrect exactly the false unknown-command reports this
    /// modelling exists to remove (issue #923 idx 42).
    #[must_use]
    pub fn reachable_files(
        &self,
        name: &str,
        target: Option<tcl_dialect::TclVersion>,
    ) -> Vec<PathBuf> {
        let Some(infos) = self.packages.get(name) else {
            return Vec::new();
        };
        infos
            .iter()
            .filter(|i| i.availability(target) != reachability::Availability::Unavailable)
            .flat_map(|i| i.source_files.iter().cloned())
            .collect()
    }
}

/// List the `*.tcl` files directly in `dir` (the `pkg_mkIndex` fallback).
fn list_tcl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "tcl") {
                files.push(path);
            }
        }
    }
    files
}

pub mod reachability;

#[cfg(test)]
mod tests;
