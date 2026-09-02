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

//! Dialect detection heuristics (and re-exports of the dialect vocabulary).
//!
//! The dialect *types* — `SpecSurface`, [`KNOWN_DIALECTS`], the
//! `DialectProfile` catalog — live in the foundational `tcl-dialect` crate
//! (dialect-profile-model.md §3) so layers below the registry (tcl-lexer,
//! tcl-syntax) consume the same source of truth. They are re-exported here
//! for the registry's own convenience and for backwards compatibility.
//!
//! What genuinely lives here is dialect *detection*: the directive /
//! shebang / content-signature / version-guard heuristics, which tokenise
//! source text and therefore need `tcl_lexer` — they sit above the lexer,
//! unlike the vocabulary itself.

pub use tcl_dialect::{KNOWN_DIALECTS, available_dialects};

/// Number of leading lines scanned for a `# tcl-dialect:` directive.
pub const DIALECT_DIRECTIVE_SCAN_LINES: usize = 5;

/// Map a bare Tcl `<major.minor>` version to its canonical dialect name.
fn tcl_version_dialect(ver: &str) -> Option<&'static str> {
    Some(match ver {
        "8.4" => "tcl8.4",
        "8.5" => "tcl8.5",
        "8.6" => "tcl8.6",
        "9.0" => "tcl9.0",
        "9.1" => "tcl9.1",
        _ => return None,
    })
}

/// `true` when the ASCII byte is a `\w` word character.
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Whether `haystack` contains `word` delimited by `\b` boundaries (ASCII).
fn has_word(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut i = 0;
    while let Some(off) = haystack[i..].find(word) {
        let start = i + off;
        let end = start + word.len();
        let before = start == 0 || !is_word_byte(bytes[start - 1]);
        let after = end == bytes.len() || !is_word_byte(bytes[end]);
        if before && after {
            return true;
        }
        i = start + 1;
    }
    false
}

/// The interpreter names a Tcl shebang may spell, each of which may carry a
/// `<x.y>` version suffix: the Tcl shell and the Tk shell.
///
/// `wish` is here because a `#!/usr/bin/wish8.6` script is a Tcl 8.6 script by
/// the same reasoning `tclsh8.6` is — Tk is a *library* in this model, not a
/// dialect profile, so the shell name only ever contributes the version
/// (issue #1625). A bare `#!/usr/bin/wish` therefore still falls through to
/// the content tiers, exactly as a bare `tclsh` does.
const SHEBANG_TCL_SHELLS: &[&str] = &["tclsh", "wish"];

/// Extract `<x.y>` from a `…\b(tclsh|wish)<x.y>\b…` shebang (input already
/// lowercased).
fn shebang_tclsh_version(lower: &str) -> Option<String> {
    SHEBANG_TCL_SHELLS
        .iter()
        .find_map(|shell| shebang_shell_version(lower, shell))
}

/// [`shebang_tclsh_version`] for one interpreter name.
fn shebang_shell_version(lower: &str, shell: &str) -> Option<String> {
    let bytes = lower.as_bytes();
    let mut i = 0;
    while let Some(off) = lower[i..].find(shell) {
        let start = i + off;
        let before = start == 0 || !is_word_byte(bytes[start - 1]);
        let mut j = start + shell.len();
        let d1 = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if before && j > d1 && j < bytes.len() && bytes[j] == b'.' {
            j += 1;
            let d2 = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let after = j == bytes.len() || !is_word_byte(bytes[j]);
            if j > d2 && after {
                return Some(lower[d1..j].to_string());
            }
        }
        i = start + 1;
    }
    None
}

/// Extract a leading `<major>.<minor>` version from `s` — one or more digits, a
/// `.`, and one or more digits. Trailing content (a patchlevel `.z`, a `-`
/// range suffix, whitespace) is ignored, so `"9.0"`, `"9.0.3"`, and `"8.5-9.0"`
/// all yield the leading `major.minor`. Returns `None` when `s` doesn't start
/// with a `major.minor` pair.
fn extract_major_minor(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut j = 0;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j == 0 || j >= bytes.len() || bytes[j] != b'.' {
        return None;
    }
    j += 1;
    let d2 = j;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j == d2 {
        return None;
    }
    Some(s[..j].to_string())
}

/// Maximum nesting depth `scan_tokens_for_tcl_version` descends into command
/// substitutions / braced words. The version guard is always near the top of a
/// file at shallow nesting (`if { [package vsatisfies …] }` is depth 2), so a
/// small bound keeps the scan cheap while covering every real idiom.
const VERSION_SCAN_DEPTH: u8 = 8;

/// One word of a tokenised command: its lexer kind and delimiter-stripped text.
struct ScanWord {
    kind: tcl_lexer::TokenType,
    text: String,
}

/// Tokenise `script` and search its *command structure* for a Tcl version
/// requirement, descending into command substitutions (`[…]`) and braced words
/// (`{…}`) so a guard nested inside `if { … }` is found too. Recognises both:
///
/// - `package require ?-exact? Tcl <x.y>` — the direct minimum-version require;
/// - `package vsatisfies [package require Tcl] <x.y>` — the idiomatic runtime
///   guard (Tcl 9's own `tclshrc` uses exactly this).
///
/// Working from tokens rather than raw text means a commented-out or
/// string-literal `package require` never matches (the lexer classes those as
/// `Comment` / a single quoted word), and bracket / brace nesting is handled
/// structurally instead of by fragile delimiter counting. Returns the first
/// `<major.minor>` found in reading order, or `None`.
fn scan_tokens_for_tcl_version(script: &str, depth: u8) -> Option<String> {
    if depth >= VERSION_SCAN_DEPTH {
        return None;
    }
    let tokens = tcl_lexer::Lexer::new(script).tokenise_all().ok()?;
    let sm = tcl_lexer::SourceMap::new(script);
    let mut cmd: Vec<ScanWord> = Vec::new();
    for tok in tokens {
        match tok.kind {
            // Command boundary: match the accumulated words, then reset.
            tcl_lexer::TokenType::Eol | tcl_lexer::TokenType::Eof => {
                if let Some(v) = match_version_command(&cmd, depth) {
                    return Some(v);
                }
                cmd.clear();
            }
            // Non-word tokens carry no command word.
            tcl_lexer::TokenType::Sep
            | tcl_lexer::TokenType::Comment
            | tcl_lexer::TokenType::Expand => {}
            _ => cmd.push(ScanWord {
                kind: tok.kind,
                text: sm.token_text(tok).to_string(),
            }),
        }
    }
    match_version_command(&cmd, depth)
}

/// Whether a command evaluates its *braced / quoted* word arguments as script or
/// expression bodies — the gate for whether the version scan may recurse into
/// them. Command substitutions (`[…]`) are always genuine scripts and are
/// recursed into regardless of the command; this gate applies only to braced /
/// quoted words, so inert data such as `set msg {package require Tcl 8.4}` is
/// never mis-tokenised as a script and used to select a bogus dialect. The name
/// is matched with any leading `::` stripped so fully-qualified builtins
/// (`::if`, `::namespace`) are recognised too.
fn is_script_body_command(name: &str) -> bool {
    matches!(
        name.trim_start_matches("::"),
        "if" | "while"
            | "for"
            | "foreach"
            | "catch"
            | "try"
            | "eval"
            | "namespace"
            | "uplevel"
            | "apply"
            | "expr"
            | "subst"
            | "proc"
            | "time"
            | "coroutine"
    )
}

/// Match one tokenised command against the two Tcl-version idioms, first
/// descending into any command-substitution / script-body argument so a guard
/// nested inside this command (e.g. `if { [package vsatisfies …] }`) is found.
fn match_version_command(cmd: &[ScanWord], depth: u8) -> Option<String> {
    // Recurse into command substitutions unconditionally (they are always real
    // scripts), but into braced / quoted words only when this command evaluates
    // them as script / expression bodies. The gate is applied at *every* nesting
    // level, so even `if {$c} {set msg {package require Tcl 8.4}}` leaves the
    // inner `set` data untouched. (Reading order.)
    let descend_braced = cmd.first().is_some_and(|w| is_script_body_command(&w.text));
    for w in cmd {
        let recurse = match w.kind {
            tcl_lexer::TokenType::Cmd => true,
            tcl_lexer::TokenType::Str => descend_braced,
            _ => false,
        };
        if recurse && let Some(v) = scan_tokens_for_tcl_version(&w.text, depth + 1) {
            return Some(v);
        }
    }
    // This command's own shape must start with `package`.
    if cmd.first().map(|w| w.text.as_str()) != Some("package") {
        return None;
    }
    match cmd.get(1).map(|w| w.text.as_str()) {
        // `package require ?-exact? Tcl <x.y>`. C Tcl 9 also registers its
        // core package under the lower-case `tcl` alias; that spelling was not
        // accepted by Tcl 8.x, so it is only a version signal from 9.0 onward.
        Some("require") => {
            let mut i = 2;
            if cmd.get(i).map(|w| w.text.as_str()) == Some("-exact") {
                i += 1;
            }
            let version = cmd.get(i + 1).and_then(|w| extract_major_minor(&w.text))?;
            if !is_tcl_core_package(cmd.get(i).map(|w| w.text.as_str()), &version) {
                return None;
            }
            Some(version)
        }
        // `package vsatisfies [package require Tcl] <x.y>` — the subject must be
        // a `[package require Tcl]` substitution so a `vsatisfies` over some
        // other package can't select a Tcl dialect.
        Some("vsatisfies") => {
            let subject = cmd.get(2)?;
            let version = cmd.get(3).and_then(|w| extract_major_minor(&w.text))?;
            if subject.kind != tcl_lexer::TokenType::Cmd
                || !is_package_require_tcl(&subject.text, &version)
            {
                return None;
            }
            Some(version)
        }
        _ => None,
    }
}

/// Whether `name` is C Tcl's core package for the requested minimum version.
///
/// `Tcl` is the package name in every supported C Tcl. Tcl 9 additionally
/// exposes the lower-case `tcl` alias (used by its shipped `init.tcl`), while
/// Tcl 8 rejects that alias. Do not use case-insensitive matching: `TCL` is
/// not a C Tcl core package spelling.
fn is_tcl_core_package(name: Option<&str>, version: &str) -> bool {
    name == Some("Tcl") || (name == Some("tcl") && version.starts_with("9."))
}

/// Whether `s` tokenises to exactly `package require Tcl` (or Tcl 9's lower
/// `tcl` alias) — the subject of a `vsatisfies` Tcl-version guard.
fn is_package_require_tcl(s: &str, requested_version: &str) -> bool {
    let Ok(tokens) = tcl_lexer::Lexer::new(s).tokenise_all() else {
        return false;
    };
    let sm = tcl_lexer::SourceMap::new(s);
    let words: Vec<&str> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.kind,
                tcl_lexer::TokenType::Esc
                    | tcl_lexer::TokenType::Str
                    | tcl_lexer::TokenType::Var
                    | tcl_lexer::TokenType::Cmd
            )
        })
        .map(|t| sm.token_text(*t))
        .collect();
    words.first().copied() == Some("package")
        && words.get(1).copied() == Some("require")
        && words.len() == 3
        && is_tcl_core_package(words.get(2).copied(), requested_version)
}

/// Return the dialect named by a `# tcl-dialect: <dialect>` comment directive in
/// the first [`DIALECT_DIRECTIVE_SCAN_LINES`] lines, or `None`. The directive
/// keyword is matched case-insensitively; the named dialect must be one of
/// [`KNOWN_DIALECTS`] (so an unknown name yields `None`).
#[must_use]
pub fn detect_dialect_directive(source: &str) -> Option<&'static str> {
    const KEY: &str = "tcl-dialect:";
    for line in source.lines().take(DIALECT_DIRECTIVE_SCAN_LINES) {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let rest = rest.trim_start();
        // `rest.get(..KEY.len())` (unlike `rest[..KEY.len()]`) returns `None`
        // rather than panicking when `KEY.len()` falls inside a multi-byte
        // char instead of on a boundary — a real case: a leading comment
        // with a non-ASCII byte (an em dash, a curly quote, …) before byte
        // offset 12 used to crash the server on `textDocument/didOpen`.
        let Some(prefix) = rest.get(..KEY.len()) else {
            continue;
        };
        if !prefix.eq_ignore_ascii_case(KEY) {
            continue;
        }
        let candidate = rest[KEY.len()..]
            .split_whitespace()
            .next()
            .unwrap_or_default();
        return KNOWN_DIALECTS.iter().copied().find(|&d| d == candidate);
    }
    None
}

/// Maximum bytes of a file inspected by [`detect_dialect`]. Detection reads
/// only the head of a document — enough to catch a directive / shebang /
/// signature line without scanning a large file.
pub const DETECT_SCAN_BYTES: usize = 8192;

/// Every filename extension that names a Tcl-family **source** file the
/// toolchain indexes and analyses.
///
/// The single source of truth for that set. The LSP server's workspace scan,
/// watched-file filter and rename filter read it; the `tcl` CLI's directory
/// discovery reads it; and `cargo xtask gen-vscode-package` generates the VS
/// Code extension's `workspaceContains` activation glob from it. Each of those
/// used to keep its own list and two of the three had drifted — the activation
/// glob named nine of the twelve (issue #1242).
///
/// Lower-case by convention; every consumer compares case-insensitively (a
/// glob consumer folds case per character, since `workspaceContains` matches
/// case-sensitively on Linux — issue #1215).
///
/// This is deliberately **not** the same question as
/// [`dialect_from_extension`], which maps an extension to a *dialect* and
/// covers vendor extensions (`.sdc`, `.do`, `.xdc`) that are Tcl but are not
/// project source we index.
pub const TCL_SOURCE_EXTENSIONS: &[&str] = &[
    "tcl", "tk", "itcl", "tm", "irul", "irule", "iapp", "iappimpl", "impl", "exp", "apl", "test",
    // The long spellings of the two extensions above that every editor
    // registers: `.irules` is owned by `f5-irules` and `.expect` by `expect`
    // in the profile catalog, so a file with either name opens as project
    // source in VS Code / JetBrains / Sublime / Zed. They were simply missed
    // when their short forms were listed, which left them registered by the
    // editors but never *indexed* — `is_tcl_source`, the watched-file glob,
    // the rename filter and the CLI directory walk all skipped them, so
    // cross-file references and rename silently missed those files until one
    // was opened (issue #1625).
    "irules", "expect",
    // `.tmsh` is an F5 tmsh *script* — Tcl the user writes and keeps beside
    // the rest of a project, in the same sense `.exp` is, and unlike the EDA
    // vendor suffixes below. It was omitted here, which left the
    // `workspaceContains` activation glob without it: a workspace whose only
    // Tcl files were `.tmsh` was never indexed (issue #1625).
    "tmsh",
    // SpecTcl packs (`spec-packs.md`): a `.tclspec` is one Tcl script, sits
    // beside the code it describes, and is indexed like any other source.
    "tclspec",
    // SslicTcl TLS declarations (#1543): a `.sslictcl` is one Tcl script that
    // is read and never evaluated, kept beside the deployment it describes,
    // and indexed like any other source.
    "sslictcl",
];

/// The `**/*.{…}` glob naming exactly [`TCL_SOURCE_EXTENSIONS`], written so it
/// matches **any casing** — `**/*.{[tT][cC][lL],…}`.
///
/// Glob consumers that have no case-insensitivity option match against the
/// platform file system: case-insensitively on Windows and macOS,
/// case-**sensitively** on Linux. That is true of LSP
/// `workspace/didChangeWatchedFiles` registrations (issue #1215) and of VS
/// Code's `workspaceContains` activation events (issue #1242) alike, so both
/// build their glob here.
///
/// Brace-expanding the casings (`{tcl,TCL}`) does not fix it — `Upper.Tcl` is
/// neither — but a per-character class does, exactly and with no extra
/// matches: `[]` ranges are part of the LSP `GlobPattern` grammar and of VS
/// Code's own glob matcher, so this stays one precise pattern rather than a
/// broad `**/*` filtered afterwards.
#[must_use]
pub fn tcl_source_glob_any_case() -> String {
    extension_glob_any_case(TCL_SOURCE_EXTENSIONS.iter().copied())
}

/// One literal spelled so a glob matches it in **any** casing —
/// `bigip.conf` → `[bB][iI][gG][iI][pP].[cC][oO][nN][fF]`.
///
/// The per-character-class trick [`tcl_source_glob_any_case`] documents,
/// factored out because the same problem appears wherever a *name* rather
/// than an extension has to be matched case-insensitively by a consumer with
/// no case-insensitivity option — VS Code's contributed `filenamePatterns`
/// being the case that motivated splitting it out (issue #1625).
#[must_use]
pub fn fold_case_in_glob(literal: &str) -> String {
    literal
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                format!("[{}{}]", c.to_ascii_lowercase(), c.to_ascii_uppercase())
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// The `**/*.{…}` any-casing glob over an arbitrary extension set.
///
/// [`tcl_source_glob_any_case`] is this over the static set; the LSP server's
/// pack-extension watcher registration is this over
/// [`pack_source_extensions`], which is why it takes an iterator rather than
/// reading the constant. Returns `None` for an empty set, since `**/*.{}`
/// matches nothing and registering it would be a silent no-op.
#[must_use]
pub fn extension_glob_any_case_opt<'a>(
    extensions: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let alternatives: Vec<String> = extensions.into_iter().map(fold_case_in_glob).collect();
    if alternatives.is_empty() {
        return None;
    }
    Some(format!("**/*.{{{}}}", alternatives.join(",")))
}

/// [`extension_glob_any_case_opt`] for a set known to be non-empty.
fn extension_glob_any_case<'a>(extensions: impl IntoIterator<Item = &'a str>) -> String {
    extension_glob_any_case_opt(extensions).unwrap_or_else(|| "**/*.{}".to_owned())
}

/// Extension-to-dialect routing declared by loaded `SpecTcl` packs
/// (`file_extension upf -dialect synopsys-eda-tcl`), published by the pack
/// merge so a pack — bundled or private — is the source of truth for its
/// own extensions. Inserted additively: a later registration for the same
/// extension wins, and nothing here ever removes the static fallback arms
/// below, which keep working for consumers that load no packs.
static PACK_EXTENSION_DIALECTS: std::sync::RwLock<
    Option<std::collections::HashMap<String, &'static str>>,
> = std::sync::RwLock::new(None);

/// Publish extension routing declared by loaded packs. Keys are lower-case
/// extensions without the leading dot; values must be canonical profile
/// names (the loader validates them against the profile catalogue).
///
/// **Snapshot semantics**: each call replaces the whole routing table with
/// the new generation, so a workspace-pack reload that dropped a
/// `file_extension` row retires that route immediately — an insert-merge
/// would pin the stale dialect until the process restarted. Callers
/// therefore always pass the *complete* pack set's pairs, which
/// `PackSet::extension_dialects` (the one producer) does by construction.
pub fn register_pack_extension_dialects(pairs: impl IntoIterator<Item = (String, &'static str)>) {
    let map: std::collections::HashMap<String, &'static str> = pairs.into_iter().collect();
    let mut guard = match PACK_EXTENSION_DIALECTS.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(map);
}

/// Every extension currently claimed by a loaded pack that the shipped
/// catalogue does **not** already account for, sorted.
///
/// The indexing counterpart of [`dialect_from_extension`]'s pack tier. A pack
/// claiming `.irulex` made an *opened* `.irulex` file analyse correctly from
/// the first release of that tier, but every predicate that decides which
/// files the toolchain reaches on its own — the LSP workspace scan, the
/// watched-file admission filter, the rename filter, the CLI directory walk —
/// read the static constant alone, so closed files stayed unindexed and
/// external edits never refreshed references or definitions (issue #1626,
/// review finding P1-3).
///
/// Two kinds of extension are filtered out, for different reasons.
/// [`TCL_SOURCE_EXTENSIONS`] entries, because a consumer unions this with that
/// set and returning them would make a caller building a glob emit each one
/// twice. And extensions the **profile catalogue** owns, because those are a
/// decision the catalogue has already taken: `.upf` is catalogued as Synopsys
/// Tcl *and* deliberately left out of the indexed set (it is Tcl, but it is
/// not project source we walk), so a bundled pack restating
/// `file_extension upf` must not quietly overturn that. The rule is the same
/// one the server's `pack_file_extensions` advertisement applies, which keeps
/// "what a pack adds" one answer rather than two.
///
/// A **snapshot**, not a subscription: the answer changes whenever a pack
/// reload republishes routing, so a consumer that caches a derived value
/// (a glob, a watcher registration) has to recompute it on reload rather than
/// once at startup.
#[must_use]
pub fn pack_source_extensions() -> Vec<String> {
    let guard = match PACK_EXTENSION_DIALECTS.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let Some(map) = guard.as_ref() else {
        return Vec::new();
    };
    let mut out: Vec<String> = map
        .keys()
        .filter(|ext| {
            !TCL_SOURCE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(ext))
                && catalog_extension_dialect(&ext.to_ascii_lowercase()).is_none()
        })
        .cloned()
        .collect();
    out.sort();
    out
}

/// Whether `ext` is one [`pack_source_extensions`] would list — asked per
/// **file**, so it allocates nothing.
///
/// The list form is for building a glob, which happens once per pack reload.
/// This is for `is_tcl_source`, which the workspace scan calls on every path
/// it walks; building and sorting a `Vec<String>` there would put an
/// allocation and a sort on the hot path of a scan that runs on every
/// configuration change.
///
/// Returns `false` immediately when no pack has registered anything, which is
/// the overwhelmingly common case and costs one lock-free-ish read.
#[must_use]
pub fn is_pack_source_extension(ext: &str) -> bool {
    let guard = match PACK_EXTENSION_DIALECTS.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let Some(map) = guard.as_ref() else {
        return false;
    };
    if map.is_empty() {
        return false;
    }
    // The map is keyed lower-case; only pay for a lowercased copy when the
    // caller's spelling is not already one.
    let lowered;
    let key = if ext.bytes().any(|b| b.is_ascii_uppercase()) {
        lowered = ext.to_ascii_lowercase();
        lowered.as_str()
    } else {
        ext
    };
    map.contains_key(key)
        && !TCL_SOURCE_EXTENSIONS.contains(&key)
        && catalog_extension_dialect(key).is_none()
}

/// The pack-declared dialect for `ext`, if any pack registered one.
fn pack_extension_dialect(ext: &str) -> Option<&'static str> {
    let guard = match PACK_EXTENSION_DIALECTS.read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.as_ref()?.get(ext).copied()
}

/// The dialect owning the whole basename `base` per the catalog's
/// `filenames` axis (`bigip.conf` → `f5-bigip`), or `None`.
///
/// A basename claim is the more specific of the two static tiers — the files
/// it names have no useful extension — so [`dialect_from_extension`] asks
/// this before the extension tier.
fn catalog_filename_dialect(base: &str) -> Option<&'static str> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<&'static str, &'static str>> =
        std::sync::OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = std::collections::HashMap::new();
        for profile in tcl_dialect::DialectProfile::all() {
            for name in profile.filenames {
                map.insert(*name, profile.name);
            }
        }
        map
    })
    .get(base)
    .copied()
}

/// The dialect owning `ext` per the [`tcl_dialect::DialectProfile`] catalog —
/// the `file_extensions` axis each profile declares (`xdc` →
/// `xilinx-eda-tcl`). Built once; the catalog's invariant tests guarantee
/// one owner per extension.
fn catalog_extension_dialect(ext: &str) -> Option<&'static str> {
    static MAP: std::sync::OnceLock<std::collections::HashMap<&'static str, &'static str>> =
        std::sync::OnceLock::new();
    MAP.get_or_init(|| {
        let mut map = std::collections::HashMap::new();
        for profile in tcl_dialect::DialectProfile::all() {
            for row in profile.file_extensions {
                map.insert(row.extension, profile.name);
            }
        }
        map
    })
    .get(ext)
    .copied()
}

/// The dialect implied by a filename extension, or `None` when the extension
/// is generic (`.tcl`) or unknown — in which case content heuristics decide.
///
/// Pack-declared routing ([`register_pack_extension_dialects`]) is consulted
/// first, so a loaded pack owns its extensions; the
/// [`tcl_dialect::DialectProfile`] catalog's per-profile `file_extensions`
/// are the no-packs fallback and the home of everything no pack declares.
/// Deliberate non-mappings stay deliberate: `.svrf` (Calibre rule decks) is
/// a declarative DSL, not Tcl, so it falls through to content/default; the
/// generic `.tcl` and HDL extensions let content decide.
#[must_use]
pub fn dialect_from_extension(filename: &str) -> Option<&'static str> {
    let base = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .to_ascii_lowercase();
    // Vendor filename *conventions* that are not a single trailing extension:
    // the Synopsys `.synopsys_{dc,pt}.setup` dotfiles and the Cadence
    // Genus→Innovus handoff scripts are near-certain vendor signals by name
    // alone (eda-library-packages.md).
    if base.ends_with(".synopsys_dc.setup") || base.ends_with(".synopsys_pt.setup") {
        return Some("synopsys-eda-tcl");
    }
    if base.ends_with(".invs_setup.tcl") || base.ends_with(".genus_setup.tcl") {
        return Some("cadence-eda-tcl");
    }
    // The catalog's whole-basename tier (`bigip.conf`), ahead of the
    // extension tier: a file claimed by name has no extension worth
    // claiming (issue #1625).
    if let Some(dialect) = catalog_filename_dialect(base.as_str()) {
        return Some(dialect);
    }
    let ext = base.rsplit('.').next()?;
    if let Some(dialect) = pack_extension_dialect(ext) {
        return Some(dialect);
    }
    catalog_extension_dialect(ext)
}

/// Truncate `source` to at most [`DETECT_SCAN_BYTES`] on a UTF-8 char boundary.
fn scan_head(source: &str) -> &str {
    if source.len() <= DETECT_SCAN_BYTES {
        return source;
    }
    let mut end = DETECT_SCAN_BYTES;
    while end > 0 && !source.is_char_boundary(end) {
        end -= 1;
    }
    &source[..end]
}

/// Whether `head` contains an iRules `when EVENT {` handler (the strongest
/// iRules signal). Mirrors `^\s*when\s+[A-Z][A-Z0-9_]{2,}\s*\{`.
fn has_irules_when(head: &str) -> bool {
    for line in head.lines() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("when") else {
            continue;
        };
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let name = rest.trim_start();
        let mut chars = name.chars();
        // `[A-Z]` then `[A-Z0-9_]{2,}`.
        if !matches!(chars.next(), Some(c) if c.is_ascii_uppercase()) {
            continue;
        }
        let ident: String = name
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        if ident.len() >= 3 && name[ident.len()..].trim_start().starts_with('{') {
            return true;
        }
    }
    false
}

/// Textual substrings that a plausible in-source dialect hint would contain:
/// the directive key, a shebang marker, the `package`/`vsatisfies`
/// version-guard vocabulary, and every [`CONTENT_SIGNATURES`] marker word
/// (plus `has_irules_when`'s `when` keyword).
///
/// A conservative *superset* filter for "could this text change what
/// [`detect_dialect`] returns" — it does not replicate the word-boundary or
/// tokenisation rules those heuristics apply, so it can false-positive (a
/// comment mentioning "package" re-triggers a detect that then confirms
/// nothing changed), but nothing that changes the detected dialect can
/// appear in a diff without containing one of these substrings. Intended for
/// a caller that wants to skip a full re-detect on an edit that plainly
/// cannot touch the hint (see the LSP server's `did_change`), not as a
/// detector in its own right.
pub fn dialect_hint_markers() -> impl Iterator<Item = &'static str> {
    const EXTRA: &[&str] = &["tcl-dialect:", "#!", "package", "vsatisfies", "when"];
    EXTRA.iter().copied().chain(
        CONTENT_SIGNATURES
            .iter()
            .flat_map(|&(_, markers)| markers.iter().copied()),
    )
}

/// Content signatures for the non-Tcl-core dialects, checked in priority order.
/// Each entry is `(dialect, &[marker words])`; a marker matches when it appears
/// as a whole word anywhere in the scanned head. Ordered most-specific first so
/// an EDA-tool script never falls through to a weaker signal.
const CONTENT_SIGNATURES: &[(&str, &[&str])] = &[
    // SslicTcl TLS declarations. The mandatory `sslictcl VERSION` header is
    // the document's first declaration and the word appears nowhere else in
    // the vocabulary, so it is the most specific signature here and the one
    // that catches a document saved under a `.tcl` name.
    ("sslictcl", &["sslictcl"]),
    // SpecTcl command packs. `speclib` is the DSL's one loader directive and
    // its only possible top-level word, so it is both the most specific
    // signature here and the one that catches a pack saved under a `.tcl`
    // name — the case the extension tier below cannot reach.
    ("spectcl", &["speclib"]),
    // F5 tmsh / iApp management scripts.
    (
        "f5-iapps",
        &["iapp::", "tmsh::create_app", "sys application template"],
    ),
    (
        "f5-tmsh",
        &["tmsh::", "tmsh create", "tmsh modify", "tmsh list"],
    ),
    // EDA-tool Tcl (synthesis / P&R / simulation). Markers are the vendors'
    // *proprietary* commands only — shared SDC verbs (create_clock,
    // set_input_delay, link_design, set_max_area, get_ports, …) are excluded,
    // as they appear in every vendor's constraint files and would misclassify
    // a portable `.sdc` (eda-library-packages.md; the July-2026 EDA study).
    (
        "xilinx-eda-tcl",
        &[
            "synth_design",
            "launch_runs",
            "create_bd_design",
            "write_bitstream",
            "create_project",
            "read_xdc",
        ],
    ),
    (
        "synopsys-eda-tcl",
        &[
            "compile_ultra",
            "dc_shell",
            "pt_shell",
            "icc2_shell",
            "fm_shell",
            "set_svf",
            "set_app_var",
        ],
    ),
    (
        "cadence-eda-tcl",
        &[
            "set_db",
            "get_db",
            "syn_generic",
            "place_opt_design",
            "innovus",
            "genus",
            "init_design",
        ],
    ),
    (
        "intel-quartus-eda-tcl",
        &[
            "quartus_",
            "::quartus::",
            "project_new",
            "set_global_assignment",
            "set_location_assignment",
            "execute_flow",
        ],
    ),
    (
        "mentor-eda-tcl",
        &["vsim", "vlog", "vcom", "vlib", "vmap", "vopt", "questa"],
    ),
    // Microchip (Microsemi) Libero SoC project/flow scripting. Markers are
    // Libero-proprietary command spellings only — the SmartTime SDC verbs are
    // excluded for the same portable-`.sdc` reason as the vendors above.
    (
        "microchip-libero-eda-tcl",
        &[
            "run_designer",
            "select_libero_design_device",
            "smartpower_report_power",
            "export_prog_job",
            "pin_fix_all",
            "configure_tool",
        ],
    ),
    // Expect automation.
    (
        "expect",
        &["spawn", "expect_before", "send_user", "interact"],
    ),
];

/// Whether `marker` appears in `haystack` at a word boundary on its **left**
/// (start-of-string or a non-word byte before it). A marker whose final
/// character is itself a word byte (`interact`, `spawn`, `vsim`) also
/// requires a word boundary on its **right** — `interactive` must not match
/// `interact`. A marker ending in a non-word byte (`tmsh::`, `iapp::`) is a
/// command-*prefix* form: the identifier that follows is expected, so no
/// right constraint applies. The one prefix marker ending in a word byte,
/// `quartus_` (`_` is a word byte), keeps prefix semantics by ending in `_`
/// — treat a trailing `_` as an explicit "identifier continues" marker.
fn contains_token(haystack: &str, marker: &str) -> bool {
    let hbytes = haystack.as_bytes();
    let whole_word = marker
        .as_bytes()
        .last()
        .is_some_and(|&b| is_word_byte(b) && b != b'_');
    let mut i = 0;
    while let Some(off) = haystack[i..].find(marker) {
        let start = i + off;
        let left_ok = start == 0 || !is_word_byte(hbytes[start - 1]);
        let end = start + marker.len();
        let right_ok = !whole_word || end >= hbytes.len() || !is_word_byte(hbytes[end]);
        if left_ok && right_ok {
            return true;
        }
        i = start + 1;
    }
    false
}

/// `source` with full-line comments removed (lines whose first non-blank
/// byte is `#`), so a dialect marker mentioned in prose — a header like
/// `# Source: user-reported interactive session` — can never flip the
/// detected dialect. Mid-line `;# …` tails are left in place: they are rare
/// in practice and a structural strip would need a full lex, which
/// [`scan_tokens_for_tcl_version`] already provides for the version guard.
fn strip_comment_lines(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Detect a dialect from a script's *content* signatures (iRules `when`,
/// F5 tmsh/iApp, EDA-tool commands, expect), over the scanned `head`.
fn detect_from_content(head: &str) -> Option<&'static str> {
    if has_irules_when(head) {
        return Some("f5-irules");
    }
    let code = strip_comment_lines(head);
    for (dialect, markers) in CONTENT_SIGNATURES {
        if markers.iter().any(|m| contains_token(&code, m)) {
            return Some(dialect);
        }
    }
    None
}

/// The dialect named by a `#!…` shebang on the first line (`expect`, or a
/// versioned `tclsh<x.y>` / `wish<x.y>`), or `None`.
fn shebang_dialect(source: &str) -> Option<&'static str> {
    let first = source.lines().next()?;
    if !first.starts_with("#!") {
        return None;
    }
    let lower = first.to_ascii_lowercase();
    if has_word(&lower, "expect") {
        return Some("expect");
    }
    shebang_tclsh_version(&lower).and_then(|ver| tcl_version_dialect(&ver))
}

/// The *content-borne* dialect signals, in the priority the project wants:
/// a `package require` / `vsatisfies` Tcl-version guard (tokenised, so comments
/// and string literals never match) first, then the command / `when`-clause
/// content signatures (iRules, F5 tmsh / iApp, EDA tools, expect). Returns the
/// dialect for the first signal found in `head`, or `None`.
///
/// Split out from [`detect_dialect`] so it can be applied to a cheap head
/// first and — only on a miss — the full source, catching a signal that a very
/// large script reveals only near its end without paying that cost on the
/// common (signal-near-the-top) path.
fn detect_content_signals(head: &str) -> Option<&'static str> {
    if let Some(ver) = scan_tokens_for_tcl_version(head, 0)
        && let Some(d) = tcl_version_dialect(&ver)
    {
        return Some(d);
    }
    detect_from_content(head)
}

/// The canonical dialect detector shared by the LSP, editors, CLI tooling, and
/// AI integrations. Given a document's `source` and optional `filename`,
/// returns a best-guess dialect (never fails — falls back to `default` when no
/// heuristic fires).
///
/// Heuristics are applied in this priority order, most-trusted first:
/// 1. an explicit `# tcl-dialect: <name>` directive (first
///    [`DIALECT_DIRECTIVE_SCAN_LINES`] lines);
/// 2. the `#!…` shebang (`expect`, `tclsh<x.y>`, `wish<x.y>`);
/// 3. a tokenised `package require ?-exact? Tcl <x.y>` or `package vsatisfies
///    [package require Tcl] <x.y>` version guard;
/// 4. content signatures — iRules `when EVENT {`, F5 `tmsh::` / iApp, EDA-tool
///    commands (Xilinx / Synopsys / Cadence / Quartus / Mentor), `expect`;
/// 5. the filename extension ([`dialect_from_extension`]) — a fallback, since a
///    `.tcl` file's *content* is a stronger signal than its name;
/// 6. the caller's `default` (the editor / LSP / XDG configuration).
///
/// The `BigIP` config-object tier (tier between content and extension in the
/// project's model) is applied by the `tcl-bigip` layer, which wraps this
/// detector — it isn't reachable from the registry crate.
///
/// Tiers 3–4 (the *content-borne* signals) are scanned streaming-style: the
/// cheap [`DETECT_SCAN_BYTES`]-byte head is tried first and, only if it is
/// inconclusive, the full source — so a signal that a very large script reveals
/// only near its end is still caught without paying that cost when a signal
/// sits near the top (the common case).
///
/// An explicit editor / workspace-folder dialect is applied by the caller
/// *above* this function and overrides everything here.
#[must_use]
pub fn detect_dialect(source: &str, filename: Option<&str>, default: &'static str) -> &'static str {
    // 1. Explicit directive always wins.
    if let Some(d) = detect_dialect_directive(source) {
        return d;
    }
    // 2. Shebang.
    if let Some(d) = shebang_dialect(source) {
        return d;
    }
    // 3-4. Content-borne signals (package/vsatisfies version, then content
    //       signatures). Try the cheap head first; on a miss, scan the whole
    //       source so a late signal in a huge script is still caught.
    let head = scan_head(source);
    if let Some(d) = detect_content_signals(head).or_else(|| {
        (source.len() > head.len())
            .then(|| detect_content_signals(source))
            .flatten()
    }) {
        return d;
    }
    // 5. Filename extension — only when the content gave nothing away.
    if let Some(d) = filename.and_then(dialect_from_extension) {
        return d;
    }
    // 6. Caller default (editor / LSP / XDG config).
    default
}

/// Detect a Tcl dialect from a script's *content* — used when no explicit
/// dialect is configured. Checks, in priority order: a `# tcl-dialect:`
/// directive (first [`DIALECT_DIRECTIVE_SCAN_LINES`] lines), a
/// `#!…tclsh<x.y>` / `#!…wish<x.y>` / `#!…expect` shebang (first line), then a
/// tokenised
/// `package require ?-exact? Tcl <x.y>` or `package vsatisfies [package require
/// Tcl] <x.y>` version guard over the first [`DETECT_SCAN_BYTES`] bytes.
/// Returns `None` when no hint is found.
///
/// (Conf-wrapped-iRules detection is an additional fallback; it depends on
/// the BIG-IP layer and is handled there.)
#[must_use]
pub fn detect_dialect_from_source(source: &str) -> Option<&'static str> {
    if let Some(d) = detect_dialect_directive(source) {
        return Some(d);
    }
    if let Some(first) = source.lines().next()
        && first.starts_with("#!")
    {
        let lower = first.to_ascii_lowercase();
        if has_word(&lower, "expect") {
            return Some("expect");
        }
        if let Some(ver) = shebang_tclsh_version(&lower)
            && let Some(d) = tcl_version_dialect(&ver)
        {
            return Some(d);
        }
    }
    let head = scan_head(source);
    let scan =
        |s: &str| scan_tokens_for_tcl_version(s, 0).and_then(|ver| tcl_version_dialect(&ver));
    // Head first (cheap); on a miss, the full source so a late guard in a huge
    // script is still caught.
    scan(head).or_else(|| (source.len() > head.len()).then(|| scan(source)).flatten())
}

#[cfg(test)]
mod detect_tests {
    use super::{detect_dialect, detect_dialect_directive};

    const DEF: &str = "tcl9.0";

    #[test]
    fn directive_wins() {
        assert_eq!(
            detect_dialect("# tcl-dialect: tcl8.5\nputs hi\n", None, DEF),
            "tcl8.5"
        );
    }

    #[test]
    fn multibyte_comment_before_directive_key_does_not_panic() {
        // A leading comment with a non-ASCII byte (an em dash here) landing
        // inside the `tcl-dialect:` key's byte length used to panic on the
        // `rest[..KEY.len()]` slice instead of just not matching.
        assert_eq!(
            detect_dialect_directive("# Issue #806 — report::defstyle\nputs hi\n"),
            None
        );
        // The directive itself still resolves once past any such comment.
        assert_eq!(
            detect_dialect_directive("# Issue #806 — report::defstyle\n# tcl-dialect: tcl8.5\n"),
            Some("tcl8.5")
        );
    }

    #[test]
    fn extension_is_the_fallback_for_generic_content() {
        // With no content signal, the extension decides.
        assert_eq!(
            detect_dialect("puts hi\n", Some("x.irule"), DEF),
            "f5-irules"
        );
        assert_eq!(
            detect_dialect("spawn ssh host\n", Some("y.exp"), DEF),
            "expect"
        );
        assert_eq!(
            detect_dialect("read_xdc c.xdc\n", Some("c.xdc"), DEF),
            "xilinx-eda-tcl"
        );
    }

    #[test]
    fn content_signals_beat_extension() {
        // A `package require Tcl` guard outranks the filename extension …
        assert_eq!(
            detect_dialect("package require Tcl 9.0\n", Some("x.exp"), DEF),
            "tcl9.0"
        );
        // … and so does a `when`-clause content signature.
        assert_eq!(
            detect_dialect("when HTTP_REQUEST {\n  pool web\n}\n", Some("x.tcl"), DEF),
            "f5-irules"
        );
    }

    #[test]
    fn late_signal_in_large_script_is_caught() {
        // A version guard past the head-scan window is still found by the
        // full-source fallback pass.
        let mut s = "set x 1\n".repeat(2000); // ~16 KB of filler > DETECT_SCAN_BYTES
        s.push_str("if {[package vsatisfies [package require Tcl] 9.0]} { x }\n");
        assert_eq!(detect_dialect(&s, None, DEF), "tcl9.0");
    }

    #[test]
    fn shebang_detected() {
        assert_eq!(
            detect_dialect("#!/usr/bin/expect -f\nspawn ssh\n", None, DEF),
            "expect"
        );
        assert_eq!(
            detect_dialect("#!/usr/bin/tclsh8.6\nputs hi\n", None, DEF),
            "tcl8.6"
        );
    }

    #[test]
    fn irules_when_detected() {
        assert_eq!(
            detect_dialect("when HTTP_REQUEST {\n  pool web\n}\n", None, DEF),
            "f5-irules"
        );
    }

    #[test]
    fn eda_and_f5_content_signatures() {
        assert_eq!(
            detect_dialect("synth_design -top foo\n", None, DEF),
            "xilinx-eda-tcl"
        );
        assert_eq!(
            detect_dialect("compile_ultra -gate_clock\n", None, DEF),
            "synopsys-eda-tcl"
        );
        assert_eq!(
            detect_dialect("set_db init_design\ninit_design\n", None, DEF),
            "cadence-eda-tcl"
        );
        assert_eq!(
            detect_dialect("tmsh::create ltm pool p\n", None, DEF),
            "f5-tmsh"
        );
    }

    #[test]
    fn expect_content_without_shebang() {
        assert_eq!(
            detect_dialect("spawn ssh host\nexpect_before timeout\n", None, DEF),
            "expect"
        );
    }

    #[test]
    fn plain_tcl_falls_back_to_default() {
        assert_eq!(detect_dialect("set x 1\nputs $x\n", None, DEF), "tcl9.0");
        assert_eq!(
            detect_dialect("package require Tcl 8.6\n", None, DEF),
            "tcl8.6"
        );
    }

    #[test]
    fn eda_extension_and_filename_conventions() {
        // High-confidence Tcl-syntax EDA file extensions.
        assert_eq!(
            detect_dialect("run -all\n", Some("sim.do"), DEF),
            "mentor-eda-tcl"
        );
        assert_eq!(
            detect_dialect("x\n", Some("top.qsf"), DEF),
            "intel-quartus-eda-tcl"
        );
        assert_eq!(
            detect_dialect("x\n", Some("design.globals"), DEF),
            "cadence-eda-tcl"
        );
        // Synopsys setup dotfile + Cadence handoff, by filename convention.
        assert_eq!(
            detect_dialect("set x 1\n", Some(".synopsys_dc.setup"), DEF),
            "synopsys-eda-tcl"
        );
        assert_eq!(
            detect_dialect("x\n", Some("/p/genus.invs_setup.tcl"), DEF),
            "cadence-eda-tcl"
        );
        // IEEE 1801 UPF power intent — cross-vendor Tcl, same representative
        // profile as `.sdc`.
        assert_eq!(
            detect_dialect("create_power_domain PD_top\n", Some("soc.upf"), DEF),
            "synopsys-eda-tcl"
        );
        // `.svrf` (Calibre rule decks) is NOT Tcl — falls to the caller default.
        assert_eq!(
            detect_dialect("LAYOUT PATH x\n", Some("drc.svrf"), DEF),
            DEF
        );
    }

    #[test]
    fn eda_proprietary_content_signatures() {
        // Cadence set_db/get_db + Genus/Innovus verbs.
        assert_eq!(
            detect_dialect("get_db insts -if {.is_macro}\n", None, DEF),
            "cadence-eda-tcl"
        );
        assert_eq!(
            detect_dialect("place_opt_design\n", None, DEF),
            "cadence-eda-tcl"
        );
        // Quartus package idiom + flow verbs.
        assert_eq!(
            detect_dialect("package require ::quartus::flow\n", None, DEF),
            "intel-quartus-eda-tcl"
        );
        assert_eq!(
            detect_dialect("execute_flow -compile\n", None, DEF),
            "intel-quartus-eda-tcl"
        );
        // Synopsys proprietary shell/app-var (not the shared-SDC verbs).
        assert_eq!(
            detect_dialect("set_svf -off\n", None, DEF),
            "synopsys-eda-tcl"
        );
        assert_eq!(
            detect_dialect("set_app_var target_library foo\n", None, DEF),
            "synopsys-eda-tcl"
        );
        // Xilinx IP integrator.
        assert_eq!(
            detect_dialect("create_bd_design system\n", None, DEF),
            "xilinx-eda-tcl"
        );
        // Mentor/Questa compile verbs.
        assert_eq!(
            detect_dialect("vlib work\nvmap work work\n", None, DEF),
            "mentor-eda-tcl"
        );
        // Microchip Libero flow verbs.
        assert_eq!(
            detect_dialect("open_project {a.prjx}\nrun_designer\n", None, DEF),
            "microchip-libero-eda-tcl"
        );
        assert_eq!(
            detect_dialect(
                "configure_tool -name {PLACEROUTE} -params {EFFORT_LEVEL:false}\n",
                None,
                DEF
            ),
            "microchip-libero-eda-tcl"
        );
    }

    /// Issue #1625: the Tk shell names a Tcl version exactly as `tclsh` does.
    /// Tk is modelled as a library, not a dialect, so a `wish` shebang
    /// contributes only its version — and a *bare* `wish` contributes nothing,
    /// falling through to the content tiers like a bare `tclsh`.
    #[test]
    fn a_wish_shebang_names_its_tcl_version() {
        assert_eq!(
            detect_dialect("#!/usr/bin/wish8.6\nbutton .b\n", None, DEF),
            "tcl8.6"
        );
        assert_eq!(
            detect_dialect("#!/usr/bin/env wish9.0\nbutton .b\n", None, DEF),
            "tcl9.0"
        );
        // Bare `wish`: no version, no opinion.
        assert_eq!(
            detect_dialect("#!/usr/bin/wish\nbutton .b\n", None, DEF),
            DEF
        );
        // The `tclsh` half is unchanged.
        assert_eq!(
            detect_dialect("#!/usr/bin/tclsh8.4\nputs hi\n", None, DEF),
            "tcl8.4"
        );
    }

    /// Issue #1625: the catalog's whole-basename axis routes the BIG-IP
    /// config files, which have no extension worth claiming — a bare `.conf`
    /// belongs to every unrelated config file on the machine.
    #[test]
    fn a_bigip_config_basename_routes_by_name() {
        use super::dialect_from_extension;
        assert_eq!(dialect_from_extension("bigip.conf"), Some("f5-bigip"));
        assert_eq!(
            dialect_from_extension("/config/BiGiP_base.CONF"),
            Some("f5-bigip")
        );
        assert_eq!(
            dialect_from_extension("C:\\tmp\\bigip_gtm.conf"),
            Some("f5-bigip")
        );
        // NEGATIVE control: the extension itself is still nobody's.
        assert_eq!(dialect_from_extension("httpd.conf"), None);
        assert_eq!(dialect_from_extension("bigip.conf.bak"), None);
    }

    /// Issue #1625: the long spellings every editor registers are indexed
    /// too, and so is the tmsh script extension — they were registered but
    /// never walked, so cross-file references silently missed them.
    #[test]
    fn the_long_extension_spellings_are_indexed_source() {
        use super::TCL_SOURCE_EXTENSIONS;
        for ext in ["irules", "expect", "tmsh", "irule", "exp"] {
            assert!(
                TCL_SOURCE_EXTENSIONS.contains(&ext),
                ".{ext} must be indexed as project source",
            );
        }
        // NEGATIVE control: the EDA vendor suffixes stay out — they are Tcl
        // but are not project source we walk.
        for ext in ["sdc", "xdc", "do", "qsf"] {
            assert!(
                !TCL_SOURCE_EXTENSIONS.contains(&ext),
                ".{ext} must not be indexed",
            );
        }
    }

    /// The two views of "what a pack adds" must agree.
    ///
    /// One builds the watcher glob, the other decides per file whether the
    /// scan indexes it. If they could disagree, a file would be indexed but
    /// not watched (so external edits never refresh it) or watched but not
    /// indexed (so its events are admitted and then dropped) — the exact
    /// half-wired state review finding P1-3 was about.
    ///
    /// Asserts nothing about *which* packs are loaded, so it is safe beside
    /// the process-global routing table other tests share.
    #[test]
    fn the_two_pack_source_views_agree() {
        use super::{is_pack_source_extension, pack_source_extensions};
        for ext in pack_source_extensions() {
            assert!(
                is_pack_source_extension(&ext),
                ".{ext} is listed as pack source but the per-file predicate says no",
            );
            // Case-folded, since the scan meets any spelling on disk.
            assert!(is_pack_source_extension(&ext.to_ascii_uppercase()));
        }
        // NEGATIVE control: neither view ever admits what the shipped
        // catalogue already accounts for — `.upf` is catalogued Synopsys Tcl
        // *and* deliberately not indexed, and a pack restating it must not
        // overturn that.
        for ext in ["tcl", "irule", "upf", "sdc", "xdc"] {
            assert!(
                !is_pack_source_extension(ext),
                ".{ext} is the catalogue's, not a pack's addition",
            );
            assert!(!pack_source_extensions().iter().any(|e| e == ext));
        }
    }

    #[test]
    fn shared_sdc_verbs_do_not_misclassify_as_a_vendor() {
        // A portable constraint file using only shared SDC verbs must not be
        // forced to a specific vendor by content — `link_design` / `set_max_area`
        // were dropped as Synopsys markers. With no filename it falls to the
        // caller default rather than a wrong vendor dialect.
        assert_eq!(
            detect_dialect(
                "create_clock -period 10 [get_ports clk]\nset_max_area 0\n",
                None,
                DEF
            ),
            DEF
        );
    }
}
