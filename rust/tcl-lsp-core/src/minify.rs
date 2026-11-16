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

//! Tcl code minifier.
//!
//! [`minify_with_analysis`] retains the complete current editing input. Native
//! syntax compaction retains original words and edits only separators in
//! authenticated original script regions. Expansions, escaping and variable
//! syntax retain their original bytes. Every selected region checks emitted
//! lexical correspondence independently; this makes no
//! runtime reflection or line-number equivalence claim. Native naming,
//! insertion, semantic and keyword passes require separate contracts and report
//! typed refusals while those contracts are unavailable.
//!
//! The following transformations belong to explicit lexical authoring advice.
//! Its **default tier** ([`minify_tcl`]) performs:
//!
//! 1. Stripping all comments.
//! 2. Collapsing inter-command whitespace to `;`.
//! 3. Collapsing intra-command whitespace to single spaces.
//! 4. Recursively minifying braced body arguments (and `[…]`
//!    command substitutions).
//! 5. Preserving string literals verbatim, dropping redundant
//!    double quotes when safe.
//! 6. Compressing whitespace inside `expr` bodies and applying
//!    AST-level shrinking (comparison inversion, De Morgan,
//!    double-negation) when it shortens the expression.
//! 7. Replacing `${var}` with `$var` when safe.
//! 8. Minifying the braced clause-list argument of registry
//!    `case_list` commands (`switch`, Expect's `expect`) with the
//!    Tcl **list** grammar — a braced case list is a list, not a
//!    script, so `#` is an ordinary pattern there, never a comment.
//!
//! Syntax compaction never introduces variables, writes, or any other
//! observable behaviour — its output is frame-transparent.  That rules
//! out `[subst $alias]` template deduplication, whose `set alias {…}`
//! preamble can clobber a live variable, fire traces, and change
//! `info vars`.
//!
//! Note: the expression tokeniser adds a catch-all so no character
//! is dropped — naively dropping unmatched characters (e.g. commas
//! in `atan2($a,$b)` and braces in `$x ni {a b}`) would corrupt
//! those expressions.
//!
//! The **`compact_names` tier** ([`minify_tcl_compact`]) renames
//! proc-local variables and parameters to short identifiers and
//! returns a [`SymbolMap`].  It relies on the analyser tracking
//! `$var` references inside `[…]` command substitutions and braced
//! `expr` bodies so a rename never rewrites a declaration without
//! its body references.  Renaming is fenced by the observability facts
//! of the document's command surface — the catalogue's, and the ones a
//! `# tcl-lsp: stub` declaration's flags state:
//!
//! * Scopes containing a dynamic-barrier command (`upvar`, `eval`,
//!   `trace`, a `-barrier` stub, … —
//!   [`Traits::CREATES_DYNAMIC_BARRIER`]) or a
//!   variable-name introspection ([`Traits::INTROSPECTS_BY_NAME`],
//!   e.g. `info locals` / `info vars` / `info exists`) are left
//!   untouched.
//! * A caller-frame command (`upvar` —
//!   [`Traits::ALIASES_CALLER_FRAME`], `uplevel` —
//!   [`Traits::EVALUATES_IN_SHIFTED_FRAME`]) anywhere blocks
//!   variable renaming in **every** scope: the observed frame is
//!   statically unknowable.
//! * **Procedure names are public identities** — `info procs`,
//!   `rename`, `namespace export`, external callers, and `unknown`
//!   can all observe or invoke them — so procs are renamed only
//!   under `isolated` (the caller asserts a self-contained,
//!   closed-world script), and even then only when no
//!   command-name-reflecting command
//!   ([`Traits::REFLECTS_COMMAND_NAMES`]) and no computed command
//!   name is present.
//! * Array member keys (`arr(member)`) are Tcl **data**, never
//!   compacted: `array get` / `array names` / serialisation observe
//!   them.
//!
//! `isolated` also compacts global-scope variables.
//!
//! The **`aggressive` tier** ([`minify_tcl_aggressive`]) applies the
//! compiler's optimiser rewrites, compacts names, aliases repeated
//! commands / arguments / quoted-string substrings (the last via a
//! suffix array), then minifies whitespace, returning a
//! [`MinifyResult`].  **Aggressive output is deliberately not
//! frame-transparent**: the aliasing phases inject `set alias …`
//! preambles, which create real Tcl variables (visible to `info
//! vars`, traces, and any pre-existing same-named variable).  The
//! alias generators avoid every name the compiler can see — the
//! compacted shorts, every variable name in any analysed scope or
//! SSA table, and every textual `$name` reference — but a name that
//! only exists in the *hosting interpreter* (a variable set before
//! the minified script is sourced) cannot be proven absent; use the
//! default or compact tier where frame transparency is required.
//!
//! [`unminify_error`] translates compacted names in an error
//! message back to the originals via a [`SymbolMap`] (round-tripped
//! through [`SymbolMap::format`] / [`SymbolMap::parse`]).  Both the
//! `tcl-lsp.minifyDocument` and `tcl-lsp.unminifyError`
//! `workspace/executeCommand` handlers are wired in the server.
//!
//! SCCP static-substring folding ([`fold_static_substrings`]) runs
//! between the optimiser rewrites and name compaction, and replaces
//! `$var` interpolations inside
//! quoted strings with the literal the compiler's SCCP pass proves
//! them to be (integer / string constants), taint-guarded.
//! `unminify_error` also remaps minified line references back to
//! original lines ([`remap_line_references`]) when both sources are
//! supplied.
//!
//! Static-substring folding does not do compile-time evaluation of
//! pure command substitutions (`[string …]` / `[format …]` /
//! `[expr …]`), boolean / float constants, or dead-`set`
//! elimination (leaving the now-unused `set` is harmless, just
//! larger).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

mod lexical_keywords;
mod lexical_metadata;
mod original;
pub use original::{MinifyOptions, MinifyRefusal, MinifyTier, minify_with_analysis};

use rustc_hash::FxHashSet;

use tcl_compiler::analyser::{AnalysisResult, Scope, ScopeKind};
use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::expr_ast::render_expr;
#[cfg(test)]
use tcl_compiler::ir::Statement;
#[cfg(test)]
use tcl_compiler::ssa::Version;
use tcl_compiler::taint::TaintColour;
#[cfg(test)]
use tcl_compiler::taint::TaintLattice;
use tcl_compiler::{BinOp, ExprNode, UnaryOp, parse_expr_for_profile};
#[cfg(test)]
use tcl_lexer::close_quote_offset;
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry, Traits};
use tcl_syntax::naming::AuthoredCompactNameSequence;

/// Depth cap for [`minify_body`]'s recursion over nested control-flow
/// bodies, `[…]` command substitutions, and `expr` bodies.
/// Threaded through every function on the path back to `minify_body`
/// (`render_command`, `reconstruct_arg`/`reconstruct_raw`,
/// `minify_switch_case_list`, `minify_lambda_literal`,
/// `compress_expr`/`tokenise_expr`).
///
/// Same reasoning and value as
/// [`crate::formatting::engine::MAX_FORMAT_DEPTH`] (this crate is also
/// reachable from a WASM host with no stack-size guarantee, via
/// `bigip-query-wasm`) — see that constant's doc comment.
const MAX_MINIFY_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(128);

/// One argument accumulated while parsing a command.
struct Arg {
    tokens: Vec<Token>,
    is_braced: bool,
    is_quoted: bool,
}

/// Map of original names to compacted names, grouped by scope
/// (only the fields the active tiers populate).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SymbolMap {
    /// Per-scope `{original_var: short}` reports, keyed by display label.
    /// Colliding labels are disambiguated after edit selection.
    pub variables: BTreeMap<String, BTreeMap<String, String>>,
    /// `{original_proc: short}`.
    pub procs: BTreeMap<String, String>,
    /// `{original_command: alias_var}` (aggressive tier).
    pub command_aliases: BTreeMap<String, String>,
    /// `{original_argument: alias_var}` (aggressive tier).
    pub argument_aliases: BTreeMap<String, String>,
    /// `{original_literal: alias_var}` (aggressive tier).
    pub string_aliases: BTreeMap<String, String>,
    /// `{original_dynamic_string: folded_static_value}` (aggressive
    /// tier, SCCP static-substring folding).
    pub static_folds: BTreeMap<String, String>,
}

/// Separate report labels, without a source scope, frame or variable identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopedSymbolMapLabel {
    /// Original transformation's display scope label.
    pub scope_label: String,
    /// Compacted spelling recorded by that transformation.
    pub compacted: String,
}

/// Failure of the lossless structured report, independently of legacy display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolMapReportError {
    /// No structured record is present; legacy display parsing remains possible.
    MissingRecord,
    /// A present structured record is malformed or has trailing non-report data.
    InvalidRecord,
    /// The record uses a schema this reader does not understand.
    UnsupportedVersion(u32),
}

const SYMBOL_MAP_RECORD: &str = "# tcl-lsp-symbol-map ";

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SymbolMapReport {
    version: u32,
    symbols: SymbolMap,
}

impl SymbolMap {
    /// Human-readable labels followed by the authoritative lossless report.
    #[must_use]
    pub fn format(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        if !self.procs.is_empty() {
            lines.push("# Procs".to_owned());
            for (original, short) in &self.procs {
                lines.push(format!("  {short} <- {original}"));
            }
        }
        for (scope_name, var_map) in &self.variables {
            lines.push(format!("# Variables in {scope_name}"));
            let mut entries: Vec<(&String, &String)> = var_map.iter().collect();
            entries.sort_by(|a, b| a.1.cmp(b.1));
            for (original, short) in entries {
                lines.push(format!("  {short} <- {original}"));
            }
        }
        if !self.command_aliases.is_empty() {
            lines.push("# Command aliases".to_owned());
            for (original, alias) in &self.command_aliases {
                lines.push(format!("  ${alias} <- {original}"));
            }
        }
        if !self.argument_aliases.is_empty() {
            lines.push("# Argument aliases".to_owned());
            for (original, alias) in &self.argument_aliases {
                lines.push(format!("  ${alias} <- {original}"));
            }
        }
        if !self.string_aliases.is_empty() {
            lines.push("# String literal aliases".to_owned());
            let mut entries: Vec<(&String, &String)> = self.string_aliases.iter().collect();
            entries.sort_by(|a, b| a.1.cmp(b.1));
            for (original, alias) in entries {
                lines.push(format!("  ${alias} <- {original:?}"));
            }
        }
        if !self.static_folds.is_empty() {
            lines.push("# Static substring folds (SCCP)".to_owned());
            for (original, folded) in &self.static_folds {
                lines.push(format!("  {folded:?} <- {original:?}"));
            }
        }
        let record = serde_json::json!({"version": 1, "symbols": self});
        lines.push(format!("{SYMBOL_MAP_RECORD}{record}"));
        lines.join("\n")
    }

    /// Exact separate scope/compacted report labels, retaining every original
    /// spelling when a compacted label is duplicated. This supplies no binding.
    #[must_use]
    pub fn scoped_reverse(&self) -> BTreeMap<ScopedSymbolMapLabel, Vec<String>> {
        let mut reversed: BTreeMap<ScopedSymbolMapLabel, Vec<String>> = BTreeMap::new();
        for (scope, variables) in &self.variables {
            for (original, compacted) in variables {
                reversed
                    .entry(ScopedSymbolMapLabel {
                        scope_label: scope.clone(),
                        compacted: compacted.clone(),
                    })
                    .or_default()
                    .push(original.clone());
            }
        }
        reversed
    }

    /// Approximate display labels. Bare ties keep the first row; ambiguous
    /// concatenated scope labels are omitted. This grants no name identity.
    #[must_use]
    pub fn reverse(&self) -> BTreeMap<String, String> {
        let mut reversed = self.bare_reverse();
        let mut scoped: BTreeMap<String, Option<String>> = BTreeMap::new();
        for (label, originals) in self.scoped_reverse() {
            let key = format!("{}:{}", label.scope_label, label.compacted);
            let value = match originals.as_slice() {
                [original] => Some(format!("{}:{original}", label.scope_label)),
                _ => None,
            };
            scoped
                .entry(key)
                .and_modify(|value| *value = None)
                .or_insert(value);
        }
        for (label, original) in scoped {
            if let Some(original) = original {
                reversed.entry(label).or_insert(original);
            }
        }
        reversed
    }

    fn bare_reverse(&self) -> BTreeMap<String, String> {
        let mut rev: BTreeMap<String, String> = BTreeMap::new();
        for (original, short) in &self.procs {
            rev.entry(short.clone()).or_insert_with(|| original.clone());
        }
        for var_map in self.variables.values() {
            for (original, short) in var_map {
                rev.entry(short.clone()).or_insert_with(|| original.clone());
            }
        }
        for aliases in [
            &self.command_aliases,
            &self.argument_aliases,
            &self.string_aliases,
        ] {
            for (original, alias) in aliases {
                rev.entry(alias.clone()).or_insert_with(|| original.clone());
            }
        }
        rev
    }

    /// Read the terminal structured record exactly. A malformed or unsupported
    /// record cannot fall back to the lossy display sections.
    ///
    /// # Errors
    /// Returns the missing, malformed or unsupported structured-record category.
    pub fn parse_structured(text: &str) -> Result<Self, SymbolMapReportError> {
        let record = text
            .lines()
            .rfind(|line| !line.trim().is_empty())
            .and_then(|line| line.strip_prefix(SYMBOL_MAP_RECORD))
            .ok_or_else(|| {
                if text.lines().any(|line| line.starts_with(SYMBOL_MAP_RECORD)) {
                    SymbolMapReportError::InvalidRecord
                } else {
                    SymbolMapReportError::MissingRecord
                }
            })?;
        let report: SymbolMapReport =
            serde_json::from_str(record).map_err(|_| SymbolMapReportError::InvalidRecord)?;
        if report.version != 1 {
            return Err(SymbolMapReportError::UnsupportedVersion(report.version));
        }
        Ok(report.symbols)
    }

    /// Parse the lossless [`Self::format`] report, or legacy variable/procedure
    /// display sections. Invalid structured records produce an empty report;
    /// [`Self::parse_structured`] exposes their exact refusal category.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        match Self::parse_structured(text) {
            Ok(report) => return report,
            Err(SymbolMapReportError::MissingRecord) => {}
            Err(_) => return Self::default(),
        }
        let mut sm = SymbolMap::default();
        let mut section = "";
        let mut section_name = String::new();
        for line in text.lines() {
            let stripped = line.trim();
            if stripped.is_empty() {
                continue;
            }
            if let Some(rest) = stripped.strip_prefix("# Variables in ") {
                section = "variables";
                section_name.clear();
                section_name.push_str(rest);
                continue;
            }
            if stripped.starts_with("# Procs") {
                section = "procs";
                continue;
            }
            if stripped.starts_with('#') {
                section = "";
                continue;
            }
            // Entry line: `short <- original`.
            let Some((short, original)) = stripped.split_once(" <- ") else {
                continue;
            };
            let (short, original) = (short.trim().to_owned(), original.trim().to_owned());
            // registry-axis-ok: irreducible — `section` is this parser's own
            // section tag, set a few lines up from *this format's own*
            // `# Procs` / `# Variables in …` headers (the `Self::format`
            // writer's own vocabulary for its symbol-map file), not a
            // dispatch on `info procs` / `info variables`; the spelling
            // coincides with `info`'s subcommand names only; until never
            match section {
                "procs" => {
                    sm.procs.insert(original, short);
                }
                "variables" => {
                    sm.variables
                        .entry(section_name.clone())
                        .or_default()
                        .insert(original, short);
                }
                _ => {}
            }
        }
        sm
    }
}

/// Translate a Tcl / iRule error message from minified names back
/// to the originals using `symbol_map`.  Replaces `$short` and
/// `"short"` occurrences; the source-correlated line remapping is
/// handled separately by [`remap_line_references`].
#[must_use]
pub fn unminify_error(error_message: &str, symbol_map: &SymbolMap) -> String {
    let rev = symbol_map.bare_reverse();
    if rev.is_empty() {
        return error_message.to_owned();
    }
    let bytes = error_message.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        let c = bytes[i];
        // `$short` variable reference.
        if c == b'$' {
            let start = i + 1;
            let mut j = start;
            while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j > start {
                let ident = &error_message[start..j];
                if let Some(orig) = rev.get(ident) {
                    out.push('$');
                    out.push_str(orig);
                    i = j;
                    continue;
                }
            }
            out.push('$');
            i += 1;
            continue;
        }
        // `"short"` quoted identifier.
        if c == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if j < n && bytes[j] == b'"' && j > start {
                let ident = &error_message[start..j];
                if let Some(orig) = rev.get(ident) {
                    out.push('"');
                    out.push_str(orig);
                    out.push('"');
                    i = j + 1;
                    continue;
                }
            }
            out.push('"');
            i += 1;
            continue;
        }
        let ch_len = utf8_len(c);
        out.push_str(&error_message[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Remap `line N` / `(procedure "X" line N)` references in `message`
/// from minified positions to approximate original lines, using the
/// proportional heuristic from `_remap_line_references`.  Single
/// pass (no double-application).
#[must_use]
pub fn remap_line_references(
    message: &str,
    minified_source: &str,
    original_source: &str,
) -> String {
    let min_commands = minified_source.matches(';').count() + 1;
    let orig_non_empty: Vec<usize> = original_source
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .map(|(i, _)| i + 1)
        .collect();
    if orig_non_empty.is_empty() {
        return message.to_owned();
    }
    let map_line = |line_no: usize| -> Option<usize> {
        // Reject `line_no == 0` as well as out-of-range high values: the
        // proportional map below computes `line_no - 1`, which underflows
        // at zero (`(procedure "f" line 0)` is untrusted error text) — a
        // debug panic / silent garbage line in release (F2b).
        if line_no == 0 || line_no > min_commands {
            return None;
        }
        // Integer proportional map: (line_no-1)/(min_commands-1) of the
        // non-empty original lines.
        let denom = min_commands.saturating_sub(1).max(1);
        let last = orig_non_empty.len() - 1;
        let idx = ((line_no - 1) * last / denom).min(last);
        Some(orig_non_empty[idx])
    };

    let bytes = message.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        // `(procedure "NAME" line N)` form.
        if message[i..].starts_with("(procedure \"")
            && let Some(rewritten) = try_remap_procline(&message[i..], &map_line)
        {
            out.push_str(&rewritten.0);
            i += rewritten.1;
            continue;
        }
        // Standalone `line N` (word-bounded).
        if message[i..].starts_with("line ")
            && (i == 0 || !is_word_byte(Some(bytes[i - 1])))
            && let Some((rewritten, consumed)) = try_remap_line(&message[i..], &map_line)
        {
            out.push_str(&rewritten);
            i += consumed;
            continue;
        }
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&message[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Parse a leading `line N` (digits, then a word boundary) and remap
/// it.  Returns `(replacement, bytes_consumed)`.
fn try_remap_line(s: &str, map_line: &impl Fn(usize) -> Option<usize>) -> Option<(String, usize)> {
    let rest = s.strip_prefix("line ")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let after = rest.as_bytes().get(digits.len()).copied();
    if is_word_byte(after) {
        return None;
    }
    let line_no: usize = digits.parse().ok()?;
    if line_no == 1 {
        // Line 1 of minified code = whole script; not useful.
        return None;
    }
    let orig = map_line(line_no)?;
    let consumed = "line ".len() + digits.len();
    Some((format!("line {orig} (minified line {line_no})"), consumed))
}

/// Parse a leading `(procedure "NAME" line N)` and remap the line.
fn try_remap_procline(
    s: &str,
    map_line: &impl Fn(usize) -> Option<usize>,
) -> Option<(String, usize)> {
    let rest = s.strip_prefix("(procedure \"")?;
    let name_end = rest.find('"')?;
    let name = &rest[..name_end];
    let tail = &rest[name_end + 1..];
    let tail = tail.strip_prefix(" line ")?;
    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let rest_after = &tail[digits.len()..];
    let close = rest_after.strip_prefix(')')?;
    let _ = close;
    let line_no: usize = digits.parse().ok()?;
    let orig = map_line(line_no)?;
    let consumed = s.len() - rest_after.len() + 1; // up to and including ')'
    Some((
        format!("(procedure \"{name}\" line {orig}, minified line {line_no})"),
        consumed,
    ))
}

/// Result and requested-pass refusals from minification.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinifyResult {
    /// The minified source.
    pub source: String,
    /// Compaction symbol map.
    pub symbol_map: SymbolMap,
    /// Number of retained naming or optimiser rewrites applied.
    pub optimisations_applied: usize,
    /// Length of the original source (bytes).
    pub original_length: usize,
    /// Requested transformations whose independent contracts are unavailable.
    /// Syntax compaction can succeed while a naming or insertion pass refuses.
    pub refusals: Vec<MinifyRefusal>,
}

impl MinifyResult {
    /// Length of the minified source.
    #[must_use]
    pub fn minified_length(&self) -> usize {
        self.source.len()
    }

    /// Percentage size reduction versus the original.
    #[must_use]
    pub fn savings_pct(&self) -> f64 {
        if self.original_length == 0 {
            return 0.0;
        }
        let min = f64::from(u32::try_from(self.source.len()).unwrap_or(u32::MAX));
        let orig = f64::from(u32::try_from(self.original_length).unwrap_or(u32::MAX));
        (1.0 - min / orig) * 100.0
    }
}

/// Minify under explicit driver profile/store inputs. Native output changes
/// only lexical separators and comments, keeping whole original words.
/// Use [`minify_with_profile`] or [`minify_with_analysis`] to retain refusals.
#[must_use]
pub fn minify_tcl(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> String {
    minify_with_profile(source, dialect, registry, MinifyOptions::default()).source
}

/// Request aggressive minification under explicit driver profile/store inputs.
/// Native analyses retain whole original words and report unavailable naming,
/// semantic, keyword and alias-insertion contracts in [`MinifyResult::refusals`].
/// Only explicit lexical authoring advice uses the compatibility passes, whose
/// inserted assignments are observable and are not frame-transparent.
#[must_use]
pub fn minify_tcl_aggressive(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    isolated: bool,
    registry: &CommandRegistry,
) -> MinifyResult {
    minify_tcl_aggressive_with(source, dialect, isolated, registry, true)
}

/// [`minify_tcl_aggressive`] with the keyword-abbreviation phase
/// switchable.
///
/// `abbreviations = false` is the CLI's `--no-abbreviations`: abbreviated
/// output is correct but harder to eyeball-diff, so the emitter can be turned
/// off without giving up the rest of the tier.
#[must_use]
pub fn minify_tcl_aggressive_with(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    isolated: bool,
    registry: &CommandRegistry,
    abbreviations: bool,
) -> MinifyResult {
    minify_with_profile(
        source,
        dialect,
        registry,
        MinifyOptions {
            tier: MinifyTier::Aggressive,
            isolated,
            abbreviations,
        },
    )
}

/// Aggressive authoring transformations, reached only after the retained
/// analysis explicitly selects lexical advice and the complete configuration
/// and availability generation match these compatibility passes. Native
/// planning never enters this path.
fn lexical_aggressive(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    isolated: bool,
    abbreviations: bool,
) -> Result<MinifyResult, MinifyRefusal> {
    let dialect = input.unit_profile();
    let context = input.context_registry();
    let registry = context.commands().as_ref();
    let original_length = source.len();

    // Apply the optimiser's semantic-preserving rewrites.
    let profile = dialect;
    let optimisations =
        tcl_compiler::optimiser::optimise_with_dialect(source, registry, Some(profile));
    let opt_count = optimisations.iter().filter(|o| !o.hint_only).count();
    let opt_edits: Vec<Edit> = optimisations
        .iter()
        .filter(|o| !o.hint_only)
        .map(|o| {
            (
                o.span.start() as usize,
                (o.span.end() - o.span.start()) as usize,
                o.replacement.clone(),
            )
        })
        .collect();
    let optimised = apply_edits(source, opt_edits);

    // Fold static substrings (SCCP-proven constants).
    let (folded, fold_count, static_folds) = fold_original_static_substrings(&optimised, input);

    // Compact names.
    let (renamed, mut symbol_map) = compact_names(&folded, input, isolated);
    symbol_map.static_folds = static_folds;

    // Keep observed collision candidates separate from the source/body gaps
    // that remain unresolved. This preserves the existing aggressive
    // authoring policy; it is not a completeness or absence certificate.
    let live_names = collect_live_names(&renamed, input);
    let _unresolved_collision_debt = &live_names.gaps;
    let mut claimed_names = collect_symbol_shorts(&symbol_map);
    claimed_names.extend(live_names.names.iter().cloned());
    let (renamed, cmd_aliases) = alias_repeated_commands(&renamed, input, &mut claimed_names);
    symbol_map.command_aliases = cmd_aliases;
    let (renamed, arg_aliases) = alias_repeated_arguments(&renamed, input, &mut claimed_names);
    symbol_map.argument_aliases = arg_aliases;
    let (renamed, str_aliases) = alias_string_literals(&renamed, input, &mut claimed_names);
    symbol_map.string_aliases = str_aliases;

    // Emit unique-prefix keyword abbreviations.
    let (renamed, abbrev_count) = if abbreviations {
        abbreviate_keywords(&renamed, input)
    } else {
        (renamed, 0)
    };

    // Rewritten source retains the same full editing input before any further
    // script/body selection; no reporting label rebuilds its context.
    let current = original::analysis_with_input(&renamed, input);
    let minified = lexical_default(&renamed, &current)?;

    Ok(MinifyResult {
        source: minified,
        symbol_map,
        optimisations_applied: opt_count + fold_count + abbrev_count,
        original_length,
        refusals: Vec::new(),
    })
}

/// Request name compaction under explicit driver profile/store inputs.
/// Native naming passes require separate complete-coverage/observer contracts;
/// until available this returns syntax compaction and an empty symbol map.
/// Use [`minify_with_profile`] or [`minify_with_analysis`] to retain the refusal.
/// Explicit lexical advice preserves the compatibility rename pass.
#[must_use]
pub fn minify_tcl_compact(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    isolated: bool,
    registry: &CommandRegistry,
) -> (String, SymbolMap) {
    let result = minify_with_profile(
        source,
        dialect,
        registry,
        MinifyOptions {
            tier: MinifyTier::Compact,
            isolated,
            ..MinifyOptions::default()
        },
    );
    (result.source, result.symbol_map)
}

fn lexical_compact(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    isolated: bool,
) -> Result<(String, SymbolMap), MinifyRefusal> {
    // Implementation contract: naming.minifier.lexical-authoring-passes
    // docs/design/analysis/name-resolution-proofs/minifier-lexical-authoring-passes.md
    let (renamed, symbol_map) = compact_names(source, input, isolated);
    let current = original::analysis_with_input(&renamed, input);
    let minified = lexical_default(&renamed, &current)?;
    Ok((minified, symbol_map))
}

/// Analyse an explicitly supplied profile and command store once, then use the
/// retained-analysis planner. Custom editor inputs should call
/// [`minify_with_analysis`] directly. The display label is not re-resolved.
#[must_use]
pub fn minify_with_profile(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    options: MinifyOptions,
) -> MinifyResult {
    let analysis = original::analysis_for_profile(source, dialect, registry);
    minify_with_analysis(source, &analysis, options)
}

fn lexical_default(source: &str, analysis: &AnalysisResult) -> Result<String, MinifyRefusal> {
    // naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    let input = analysis
        .resolved_input
        .as_ref()
        .ok_or(MinifyRefusal::MissingAnalysisContext)?;
    let context = input.context_registry();
    let identities = analysis
        .retained_command_realm()
        .ok_or(MinifyRefusal::MissingAnalysisContext)?;
    Ok(minify_body(
        source,
        MinifyEnv {
            original_source: source,
            analysis,
            input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            registry: context.commands(),
            context: &context,
            identities,
        },
        0,
        tcl_lexer::SourceSegmentationPurpose::WholeInput,
    ))
}

/// The document-wide context every step of the minify recursion carries
/// unchanged.
///
/// Bundled rather than passed as separate parameters because the recursion is
/// deep (`minify_body` → `render_command` → `reconstruct_raw` → `minify_body`)
/// and one of its steps was already at the argument limit.
#[derive(Clone, Copy)]
struct MinifyEnv<'a> {
    /// Complete unchanged source backing every borrowed rendering region.
    original_source: &'a str,
    /// Actual analysis owning this complete source, input and Realm.
    analysis: &'a AnalysisResult,
    /// Same immutable complete driver input sealed by the retained Realm.
    input: &'a tcl_compiler::analyser::ResolvedAnalysisInput,
    /// The document's dialect, for the lexer and the abbreviation tables.
    dialect: &'static tcl_dialect::DialectProfile,
    /// Complete actual body grammar, already checked for compatibility support.
    config: LexerConfig,
    /// Actual full availability generation; no profile-default re-selection.
    context: &'a tcl_registry::model::ContextRegistry,
    /// The registry the argument roles and clause-list shapes come from.
    registry: &'a CommandRegistry,
    /// The document's statically proven command-identity facts
    /// ([`tcl_compiler::realm`]), so a body / lambda / expression /
    /// clause-list argument is recognised by the command a head *is* rather
    /// than the one it is spelled as.
    ///
    /// Read *unpositioned*: this recursion re-minifies each nested body from
    /// its own slice, segmented at offset 0, so no document-absolute offset
    /// exists at the point of the query.
    identities: &'a tcl_compiler::realm::CommandBindingRealm,
}

impl MinifyEnv<'_> {
    /// Only an actual subslice of the complete held input retains coordinates.
    /// Decoded or generated text cannot recapture a source-body occurrence.
    fn original_region(self, source: &str) -> Option<Span> {
        let start =
            (source.as_ptr() as usize).checked_sub(self.original_source.as_ptr() as usize)?;
        let end = start.checked_add(source.len())?;
        (self.original_source.get(start..end)? == source).then_some(())?;
        Some(Span::new(
            u32::try_from(start).ok()?,
            u32::try_from(end).ok()?,
        ))
    }
}

/// Minify a Tcl script body (top-level or inside braces). `depth` is this
/// body's nesting level — see [`MAX_MINIFY_DEPTH`].
fn minify_body(
    source: &str,
    env: MinifyEnv<'_>,
    depth: u32,
    purpose: tcl_lexer::SourceSegmentationPurpose,
) -> String {
    let local_config = purpose.config(env.config);
    let dialect = env.dialect;
    // Native-stack safety net — see `MAX_MINIFY_DEPTH`'s doc comment.
    // Past the cap, leave this (deeply nested) body unminified rather than
    // recursing further, matching the give-up-gracefully fallback just below
    // for an unparseable body.
    if MAX_MINIFY_DEPTH.exceeded(depth) {
        return source.to_owned();
    }
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::with_config(source, local_config).tokenise_all() else {
        return source.to_owned();
    };

    let commands = parse_commands(source, &tokens, local_config);
    let original_words = if let Some(region) = env.original_region(source) {
        let capture = || -> Option<tcl_lexer::NativeScriptWordsPlan> {
            let image = env.identities.original_source_image()?.clone();
            if image.bytes() != env.original_source.as_bytes()
                || env.config != env.input.lexer_config()
                || !env.identities.matches_resolved_analysis_input(env.input)
                || !env
                    .identities
                    .matches_original_source_image(&image, env.config)
            {
                return None;
            }
            let plan =
                tcl_lexer::native_script_words_in_with_purpose(image, region, env.config, purpose)
                    .ok()?;
            (plan.fatal_tail.is_none() && plan.commands.len() == commands.len()).then_some(plan)
        };
        let Some(plan) = capture() else {
            return source.to_owned();
        };
        Some(plan)
    } else {
        // Deliberately decoded/generated fields have no original coordinates.
        None
    };
    if commands.is_empty() {
        return String::new();
    }

    let mut rendered: Vec<Vec<String>> = Vec::with_capacity(commands.len());
    for (index, cmd_args) in commands.iter().enumerate() {
        let original = original_words
            .as_ref()
            .and_then(|plan| plan.commands.get(index));
        let Some(command) = render_command(&sm, cmd_args, original, env, depth) else {
            return source.to_owned();
        };
        rendered.push(command);
    }

    // NB: no template deduplication here.  The former `[subst $alias]`
    // rewrite injected a `set alias {…}` preamble — a real Tcl variable
    // write that could clobber a live variable read through a
    // name-taking command (`puts [set a]`), fire traces, and change
    // `info vars` — so it is banned from this semantics-preserving tier.
    // Aggressive aliasing (an explicitly
    // behaviour-changing tier) covers the same compression ground.
    let is_irules = dialect.is_irules();
    let mut parts: Vec<String> = Vec::new();
    for arg_strs in &rendered {
        if is_irules && arg_strs.len() > 1 {
            // In iRules, `}{` is a valid word boundary — omit the
            // space between adjacent braced args to save bytes.
            let mut piece = arg_strs[0].clone();
            for w in arg_strs.windows(2) {
                let (prev, cur) = (&w[0], &w[1]);
                if prev.ends_with('}') && cur.starts_with('{') {
                    piece.push_str(cur);
                } else {
                    piece.push(' ');
                    piece.push_str(cur);
                }
            }
            parts.push(piece);
        } else {
            parts.push(arg_strs.join(" "));
        }
    }
    parts.join(";")
}

// Local-name compaction (compact_names tier)

/// A text edit: replace `length` bytes at `offset` with `text`.
type Edit = (usize, usize, String);

/// Apply non-overlapping `(offset, length, new_text)` edits in
/// reverse offset order, deduplicating identical `(offset, length)`
/// pairs.
fn apply_edits(source: &str, mut edits: Vec<Edit>) -> String {
    if edits.is_empty() {
        return source.to_owned();
    }
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.0));
    let mut seen: FxHashSet<(usize, usize)> = FxHashSet::default();
    let mut result = source.to_owned();
    for (offset, length, new_text) in edits {
        if !seen.insert((offset, length)) {
            continue;
        }
        if offset + length <= result.len() {
            result.replace_range(offset..offset + length, &new_text);
        }
    }
    result
}

/// Display label only: `::` for the root, then `parent::child`.
fn child_scope_label(parent_label: &str, child_name: &str) -> String {
    tcl_syntax::naming::rooted_member_key(parent_label, child_name)
}

/// Reborrow the actual deepest source scope and render its separate label.
fn source_scope_at_offset<'a>(
    scope: &'a Scope,
    offset: u32,
    prefix: &str,
    include_global: bool,
) -> Option<(&'a Scope, String)> {
    for child in &scope.children {
        let label = child_scope_label(prefix, &child.name);
        if let Some(body) = child.body_span
            && body.start() <= offset
            && offset <= body.end()
        {
            if let Some(deeper) = source_scope_at_offset(child, offset, &label, include_global) {
                return Some(deeper);
            }
            return Some((child, label));
        }
    }
    match scope.kind {
        ScopeKind::Proc => Some((scope, prefix.to_owned())),
        ScopeKind::Global if include_global => Some((scope, prefix.to_owned())),
        _ => None,
    }
}

/// Report collisions are rendered after edits have been selected. These keys
/// never select a source scope or decide whether an edit is authorised.
fn record_variable_report(
    symbol_map: &mut SymbolMap,
    label: &str,
    mapping: BTreeMap<String, String>,
) {
    let mut report_label = label.to_owned();
    let mut occurrence = 2;
    while symbol_map.variables.contains_key(&report_label) {
        report_label = format!("{label} [source {occurrence}]");
        occurrence += 1;
    }
    symbol_map.variables.insert(report_label, mapping);
}

/// Where renaming is unsafe, as proven by registry-declared
/// observability facts over the script's command invocations.
#[derive(Debug, Default)]
struct RenameBarriers<'a> {
    /// Actual borrowed source scopes containing a local rename barrier.
    /// Equality is allocation correspondence within this retained analysis.
    scopes: Vec<&'a Scope>,
    /// Variable renaming is barred in **every** scope: a caller-frame
    /// command (`upvar` / `uplevel`) or a cross-proc parameter
    /// introspection (`info args PROC`) can observe any frame.
    all_variable_scopes: bool,
    /// Variable renaming is barred in the global scope specifically —
    /// a scope-alias command (`global` / `variable`) or a global
    /// introspection links / enumerates global cells by name.
    global_variables: bool,
    /// Procedure renaming is barred: a command-name-reflecting
    /// command or a computed command name is present, so proc names
    /// are observable data.
    procs: bool,
}

impl RenameBarriers<'_> {
    fn allows_scope(&self, scope: &Scope) -> bool {
        !self.all_variable_scopes
            && !self
                .scopes
                .iter()
                .any(|barred| std::ptr::eq(*barred, scope))
            && (scope.kind != ScopeKind::Global || !self.global_variables)
    }
}

/// Observability comes from the same selected complete-word metadata.
/// Missing table/selector coverage is a barrier, rather than an empty trait.
fn find_rename_barriers<'a>(
    analysis: &'a AnalysisResult,
    env: MinifyEnv<'_>,
    commands: &[tcl_lexer::NativeScriptCommandWords],
    include_global: bool,
) -> RenameBarriers<'a> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let mut out = RenameBarriers::default();
    for command in commands {
        let traits = lexical_metadata::with_editable_command_schema(env, command, |schema| {
            let spec = env.context.context().resolve_spec_in_realm(
                env.registry,
                schema.canonical_command,
                tcl_dialect::model::InvocationRealm::RuleLoader,
            )?;
            if !spec.subcommands.is_empty() && schema.subcommand.resolved().is_none() {
                return None;
            }
            Some(schema.semantics.traits)
        })
        .flatten();
        let Some(traits) = traits else {
            out.all_variable_scopes = true;
            out.procs = true;
            continue;
        };
        apply_rename_traits(
            &mut out,
            traits,
            source_scope_at_offset(
                &analysis.global_scope,
                command.span.start(),
                "::",
                include_global,
            )
            .map(|(scope, _)| scope),
        );
    }
    out
}

fn apply_rename_traits<'a>(out: &mut RenameBarriers<'a>, traits: Traits, scope: Option<&'a Scope>) {
    let reflects_vars =
        traits.intersects(Traits::INTROSPECTS_BY_NAME | Traits::TARGETS_VARIABLE_BY_NAME);
    let reflects_commands = traits.contains(Traits::REFLECTS_COMMAND_NAMES);
    if (reflects_vars || traits.contains(Traits::CREATES_DYNAMIC_BARRIER))
        && let Some(scope) = scope
        && !out.scopes.iter().any(|barred| std::ptr::eq(*barred, scope))
    {
        out.scopes.push(scope);
    }
    out.global_variables |= reflects_vars || traits.contains(Traits::CREATES_SCOPE_ALIAS);
    out.all_variable_scopes |= traits
        .intersects(Traits::ALIASES_CALLER_FRAME | Traits::EVALUATES_IN_SHIFTED_FRAME)
        || (reflects_vars && reflects_commands);
    out.procs |= reflects_commands;
}

/// Apply only the original formal-name fields already checked for this header.
/// Defaults, list wrappers and unchanged rest parameters remain outside edits.
fn rename_params_in_list(
    formals: &[tcl_compiler::signature_scan::formal_parameters::OriginalSourceFormalName],
    var_map: &BTreeMap<String, String>,
    edits: &mut Vec<Edit>,
) {
    for formal in formals {
        let (Ok(name), Some(span)) = (std::str::from_utf8(formal.name()), formal.source_span())
        else {
            continue;
        };
        if let Some(short) = var_map.get(name) {
            let range = span.as_range();
            edits.push((range.start, range.len(), short.clone()));
        }
    }
}

/// Whether `b` is `[A-Za-z0-9_]`.
fn is_word_byte(b: Option<u8>) -> bool {
    matches!(b, Some(c) if c.is_ascii_alphanumeric() || c == b'_')
}

/// Byte-span slice of `source`.
fn slice(source: &str, span: Span) -> &str {
    let (s, e) = (span.start() as usize, span.end() as usize);
    if s <= e && e <= source.len() {
        &source[s..e]
    } else {
        ""
    }
}

/// Written execution heads selected by the shared current procedure reader.
fn current_proc_call_sites(
    source: &str,
    procedure: &tcl_compiler::analyser::ProcDef,
    analysis: &AnalysisResult,
) -> Vec<Span> {
    if !analysis.allows_lexical_declaration_advice()
        || crate::original_context::CurrentSourceContext::capture(source, analysis).is_none()
        || !analysis
            .all_procs
            .values()
            .any(|current| std::ptr::eq(current, procedure))
    {
        return Vec::new();
    }
    let mut seen = FxHashSet::default();
    analysis
        .command_invocations
        .iter()
        .filter(|invocation| {
            !invocation.indirect
                && invocation.lookup.is_execution_site()
                && crate::references::invocation_references_proc(
                    analysis,
                    invocation,
                    &procedure.qualified_name,
                    procedure,
                    source,
                )
                && seen.insert((invocation.range.start(), invocation.range.end()))
        })
        .map(|invocation| invocation.range)
        .collect()
}

/// Constructed presentation rows retained independently from source rewriting.
#[cfg(test)]
fn find_proc_call_sites(name: &str, qualified_name: &str, analysis: &AnalysisResult) -> Vec<Span> {
    let qn_no_prefix =
        tcl_syntax::naming::unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
    let mut out = Vec::new();
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    for inv in &analysis.command_invocations {
        // An indirect site (a constant `$cmd` head) carries no written name
        // at its span — the minifier must not rewrite it.
        if inv.indirect {
            continue;
        }
        let matches = match &inv.resolved_qualified_name {
            Some(resolved) => resolved == qualified_name,
            None => inv.name == name || inv.name == qualified_name || inv.name == qn_no_prefix,
        };
        if matches && seen.insert((inv.range.start(), inv.range.end())) {
            out.push(inv.range);
        }
    }
    out
}

/// Compact proc-local (and, when `isolated`, global) variable and
/// parameter names — plus, under `isolated` only, proc names —
/// returning `(renamed_source, symbol_map)`.
///
/// Array member keys are **never** compacted: `arr(member)` is Tcl
/// data observable through `array get` / `array names` / traces /
/// serialisation, not a private compiler symbol.
fn compact_names(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    isolated: bool,
) -> (String, SymbolMap) {
    let analysis = original::analysis_with_input(source, input);
    let context = input.context_registry();
    let Some(identities) = analysis.retained_command_realm() else {
        return (source.to_owned(), SymbolMap::default());
    };
    let env = MinifyEnv {
        original_source: source,
        analysis: &analysis,
        input,
        dialect: input.unit_profile(),
        config: input.lexer_config(),
        registry: context.commands(),
        context: &context,
        identities,
    };
    let Some(census) = lexical_metadata::command_census(source, env) else {
        return (source.to_owned(), SymbolMap::default());
    };
    let commands = lexical_metadata::CommandCensus::observed_commands(&census);
    let mut symbol_map = SymbolMap::default();
    let mut edits: Vec<Edit> = Vec::new();

    let mut barriers = find_rename_barriers(&analysis, env, commands, isolated);
    if !lexical_metadata::CommandCensus::is_complete(&census, source, &analysis) {
        barriers.all_variable_scopes = true;
        barriers.procs = true;
    }
    let rmw_targets = rmw_target_var_names(env, commands);
    let builtin_names: FxHashSet<&str> = context
        .command_names()
        .filter(|name| {
            context
                .context()
                .resolve_spec_in_realm(
                    context.commands(),
                    name,
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .is_some()
        })
        .collect();

    let references =
        crate::source_structure::OriginalVariableReferences::capture(source, &analysis);
    let mut renamed_scopes = Vec::new();
    let scope_ctx = ScopeCtx {
        references: references.as_ref(),
        config: input.lexer_config(),
        source,
        analysis: &analysis,
        isolated,
        barriers: &barriers,
        rmw_targets: &rmw_targets,
    };
    process_scope(
        scope_ctx,
        &analysis.global_scope,
        "::",
        &mut ScopeOut {
            symbol_map: &mut symbol_map,
            edits: &mut edits,
            renamed_scopes: &mut renamed_scopes,
        },
    );

    // Proc renaming.  A proc name is a *public command identity* —
    // observable via `info procs` / `info commands`, `rename`,
    // `namespace export`, `unknown`, traces, and callable by code
    // outside this script — so it is renamed only when the caller
    // asserts a closed world (`isolated`) AND no command-name
    // reflection or computed command name is present.
    if isolated && !barriers.procs {
        compact_isolated_procedure_names(
            source,
            input,
            &analysis,
            &builtin_names,
            &mut symbol_map,
            &mut edits,
        );
    }

    compact_logical_formals(
        source,
        input,
        &analysis,
        isolated,
        &renamed_scopes,
        &mut symbol_map,
        &mut edits,
    );
    let result = apply_edits(source, edits);
    (result, symbol_map)
}

fn compact_isolated_procedure_names(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    analysis: &AnalysisResult,
    builtin_names: &FxHashSet<&str>,
    symbol_map: &mut SymbolMap,
    edits: &mut Vec<Edit>,
) {
    let mut proc_gen = AuthoredCompactNameSequence::new(input.lexer_config());
    let mut used_proc_names: FxHashSet<String> = FxHashSet::default();
    let mut proc_keys: Vec<&String> = analysis.all_procs.keys().collect();
    proc_keys.sort();
    for qname in proc_keys {
        let proc_def = &analysis.all_procs[qname];
        let name = &proc_def.name;
        if name.len() <= 1 || tcl_syntax::naming::is_qualified(name.as_bytes()) {
            continue;
        }
        // A proc that overrides a registry-known command (`proc unknown
        // …`, an `auto_*` replacement, a shadowed builtin) is invoked by
        // the interpreter or library through that exact spelling — its
        // name is load-bearing.
        if builtin_names.contains(name.as_str()) {
            continue;
        }
        let Some(short) = proc_gen.next_avoiding_observed(
            |name| builtin_names.contains(name) || used_proc_names.contains(name),
            None,
        ) else {
            continue;
        };
        let short = short.into_name();
        if short.len() >= name.len() {
            continue;
        }
        used_proc_names.insert(short.clone());

        let r = proc_def.name_span;
        let actual = slice(source, r);
        let def_key = (r.start() as usize, actual.len());
        if actual == *name {
            edits.push((r.start() as usize, actual.len(), short.clone()));
        }
        for call in current_proc_call_sites(source, proc_def, analysis) {
            let call_text = slice(source, call);
            let key = (call.start() as usize, call_text.len());
            if key != def_key && call_text == *name {
                edits.push((call.start() as usize, call_text.len(), short.clone()));
            }
        }
        symbol_map.procs.insert(name.clone(), short);
    }
}

/// The isolated Logical formal purpose has its own exact binding inventory.
/// General unknown-mutation keyword and variable barriers remain unchanged.
fn compact_logical_formals(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    analysis: &AnalysisResult,
    isolated: bool,
    renamed_scopes: &[&Scope],
    symbol_map: &mut SymbolMap,
    edits: &mut Vec<Edit>,
) {
    // naming.minifier.logical-formal-binding-alpha
    // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
    if !isolated {
        return;
    }
    let Some(inventory) =
        tcl_compiler::registry_invocation::logical_formals::original_logical_procedure_bindings(
            source, analysis,
        )
    else {
        return;
    };
    for procedure in inventory {
        if !procedure.matches_source(source, input) || !procedure.alpha_binding_lookup_closed() {
            continue;
        }
        let selected_scope = source_scope_at_offset(
            &analysis.global_scope,
            procedure.body().content_span().start(),
            "::",
            false,
        )
        .filter(|(scope, _)| {
            scope.kind == ScopeKind::Proc
                && scope.body_span == Some(procedure.body().content_span())
        });
        if selected_scope.as_ref().is_some_and(|(scope, _)| {
            renamed_scopes
                .iter()
                .any(|renamed| std::ptr::eq(*renamed, *scope))
        }) {
            continue;
        }
        let label = selected_scope.map_or_else(
            || format!("logical@{}", procedure.body().content_span().start()),
            |(_, label)| label,
        );
        let existing = procedure
            .formals()
            .iter()
            .map(|formal| formal.name().to_owned())
            .chain(std::iter::once("args".to_owned()))
            .collect::<FxHashSet<_>>();
        let mut names = AuthoredCompactNameSequence::new(input.lexer_config());
        let mut mapping = BTreeMap::new();
        let mut claimed = FxHashSet::default();
        for formal in procedure.formals() {
            if formal.name().len() <= 1 {
                continue;
            }
            let Some(short) = names.next_avoiding_observed(
                |name| existing.contains(name) || claimed.contains(name),
                Some(1000),
            ) else {
                continue;
            };
            let short = short.into_name();
            if short.len() >= formal.name().len() {
                continue;
            }
            claimed.insert(short.clone());
            for span in std::iter::once(formal.declaration_span())
                .chain(formal.references().iter().map(tcl_compiler::registry_invocation::logical_formals::OriginalLogicalFormalReference::name_span))
            {
                let range = span.as_range();
                edits.push((range.start, range.len(), short.clone()));
            }
            mapping.insert(formal.name().to_owned(), short);
        }
        if !mapping.is_empty() {
            record_variable_report(symbol_map, &label, mapping);
        }
    }
}

/// Read-only context for the recursive scope rename walk: the document
/// `source`, the analyser result, the `isolated` flag, and the precomputed
/// rename-barrier / read-modify-write-target sets.
#[derive(Clone, Copy)]
struct ScopeCtx<'a> {
    references: Option<&'a crate::source_structure::OriginalVariableReferences<'a>>,
    config: LexerConfig,
    source: &'a str,
    analysis: &'a AnalysisResult,
    isolated: bool,
    barriers: &'a RenameBarriers<'a>,
    rmw_targets: &'a FxHashSet<String>,
}

/// Mutable outputs accumulated by [`process_scope`]: the symbol map and the
/// edit list.
struct ScopeOut<'a, 'scope> {
    symbol_map: &'a mut SymbolMap,
    edits: &'a mut Vec<Edit>,
    renamed_scopes: &'a mut Vec<&'scope Scope>,
}

/// Exact name extents within current original reads, plus unchanged bare role
/// operands. Missing correspondence prevents a partial variable rename.
fn variable_reference_name_spans(
    source: &str,
    geometry: &crate::source_structure::OriginalVariableReferences<'_>,
    name: &str,
    references: &[tcl_lexer::Span],
) -> Option<Vec<tcl_lexer::Span>> {
    let mut names = Vec::new();
    for &span in references {
        if let Some(original) = geometry.reference_at(span.start()) {
            if original.root != name
                || (original.token_span != span && original.whole_span != span)
                || source.get(original.name_span.as_range()) != Some(name)
            {
                return None;
            }
            names.push(original.name_span);
        } else if source.get(span.as_range()) == Some(name) {
            names.push(span);
        } else {
            return None;
        }
    }
    names.sort_by_key(|span| (span.start(), span.end()));
    names.dedup();
    Some(names)
}

/// Recursively rename variables (and params) in a scope, mirroring
/// `_process_scope`.
fn process_scope<'a>(
    ctx: ScopeCtx<'a>,
    scope: &'a Scope,
    scope_label: &str,
    out: &mut ScopeOut<'_, 'a>,
) {
    let ScopeCtx {
        references,
        config,
        source,
        analysis,
        isolated,
        barriers,
        rmw_targets,
    } = ctx;
    let rename_scope = (scope.kind == ScopeKind::Proc
        || (isolated && scope.kind == ScopeKind::Global))
        && barriers.allows_scope(scope);

    if rename_scope {
        // Identify the proc this scope belongs to by its body span — unique per
        // proc — not a namespace-blind `pd.name == scope.name` scan.  The scope
        // name is the proc name *as written* (a bare `dup` inside two different
        // namespaces), so the simple-name scan matched an arbitrary same-named
        // proc in `HashMap` order and handed `rename_params_in_list` the wrong
        // declaration's parameter region, corrupting the output on a collision
        // (renaming one proc's `$use` sites while leaving its parameter — or a
        // colliding local — under the other proc's name).  The body span keys
        // straight to this proc regardless of name collisions.
        let proc_def = if scope.kind == ScopeKind::Proc {
            scope
                .body_span
                .and_then(|bs| analysis.all_procs.values().find(|pd| pd.body_span == bs))
        } else {
            None
        };
        let param_names: FxHashSet<&str> =
            proc_def.map_or_default(|pd| pd.params.iter().map(|p| p.name.as_str()).collect());
        // Obtain all formal geometry before adding any parameter-body edits.
        // An unavailable header or cooked name cannot leave a partial rename.
        let formal_sources = proc_def.and_then(|procedure| {
            lexical_metadata::parameter_name_sources(source, analysis, procedure)
        });

        let mut var_gen = AuthoredCompactNameSequence::new(config);
        let existing: FxHashSet<String> = scope.variables.keys().cloned().collect();
        let mut var_map: BTreeMap<String, String> = BTreeMap::new();

        let mut var_names: Vec<&String> = scope.variables.keys().collect();
        var_names.sort();
        for var_name in var_names {
            let var_def = &scope.variables[var_name];
            if var_name.len() <= 1
                || var_name.contains('\0')
                || tcl_syntax::naming::is_qualified(var_name.as_bytes())
            {
                continue;
            }
            // A local that aliases a namespace / global cell (`global v`,
            // `variable v`, `namespace upvar …`) shares that cell's public
            // name — renaming the local spelling would detach it from the
            // cell.  (Scopes containing the aliasing command are already
            // barred; this guards the same variable observed from a scope
            // that is not.)
            if var_def.link_target.is_some() {
                continue;
            }
            // Skip variables that are the bare write-target of a mutating
            // command (`incr` / `append` / `lappend`): the analyser records
            // those as definitions, not reads, so `VarDef.references` (reads
            // only) misses the target argument. Renaming the `set` / `$var`
            // sites but not the `incr var` site would mutate a different
            // variable than is read, corrupting the program. Keeping the name
            // unchanged everywhere is semantics-preserving (at the cost of less
            // compaction for that one name).
            if rmw_targets.contains(var_name.as_str()) {
                continue;
            }
            let is_param = param_names.contains(var_name.as_str());
            if is_param
                && (tcl_syntax::formal_params::is_variadic_formal_name(var_name)
                    || formal_sources.as_ref().is_none_or(|formals| {
                        !formals
                            .iter()
                            .any(|formal| formal.name() == var_name.as_bytes())
                    }))
            {
                continue;
            }

            if !is_param && slice(source, var_def.definition_span) != *var_name {
                continue;
            }
            let Some(reference_names) = references.and_then(|geometry| {
                variable_reference_name_spans(source, geometry, var_name, &var_def.references)
            }) else {
                continue;
            };
            let claimed: FxHashSet<String> = var_map.values().cloned().collect();
            let Some(short) = var_gen.next_avoiding_observed(
                |name| existing.contains(name) || claimed.contains(name),
                Some(1000),
            ) else {
                continue;
            };
            let short = short.into_name();
            if short.len() >= var_name.len() {
                continue;
            }
            if !is_param {
                let span = var_def.definition_span;
                out.edits
                    .push((span.start() as usize, span.len() as usize, short.clone()));
            }
            for span in reference_names {
                out.edits
                    .push((span.start() as usize, span.len() as usize, short.clone()));
            }
            var_map.insert(var_name.clone(), short);
        }

        if let Some(formals) = formal_sources.as_deref()
            && !var_map.is_empty()
        {
            rename_params_in_list(formals, &var_map, out.edits);
        }
        if !var_map.is_empty() {
            out.renamed_scopes.push(scope);
            record_variable_report(out.symbol_map, scope_label, var_map);
        }
    }

    for child in &scope.children {
        let label = child_scope_label(scope_label, &child.name);
        process_scope(ctx, child, &label, out);
    }
}

/// Variable names that are the bare target of a read-modify-write
/// command (`incr` / `append` / `lappend` / `lset` — the registry's
/// `rmw_first_arg_variable` set; a whole-value `set` is rename-safe) or
/// of a variable-destroying command (`unset` —
/// [`Traits::DESTROYS_VARIABLE`]). The analyser records these as
/// definitions, not reads, so [`AnalysisResult`]'s `VarDef.references`
/// (reads plus re-definition sites) can miss the target argument. The
/// name compaction excludes them so it cannot rename the `set` / `$var`
/// sites while leaving the `incr var` / `unset var` target untouched
/// (which would corrupt the program).
fn rmw_target_var_names(
    env: MinifyEnv<'_>,
    commands: &[tcl_lexer::NativeScriptCommandWords],
) -> FxHashSet<String> {
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let mut names = FxHashSet::default();
    for command in commands {
        lexical_metadata::with_command_schema(env, command, |schema| {
            if !schema
                .semantics
                .traits
                .contains(Traits::FIRST_ARG_VARNAME | Traits::READS_BEFORE_WRITE)
                && !schema.semantics.traits.contains(Traits::DESTROYS_VARIABLE)
            {
                return;
            }
            let (roles, complete) = schema.authored_source_argument_roles();
            if !complete {
                return;
            }
            for (index, role) in roles {
                if role != ArgRole::VarWrite {
                    continue;
                }
                if let Some(name) = schema
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(index))
                    .and_then(|index| index.checked_add(1))
                    .and_then(|index| command.words.get(index))
                    .and_then(lexical_metadata::ascii_value)
                {
                    names.insert(name);
                }
            }
        });
    }
    names
}

// Aggressive-tier aliasing (command / argument / string-literal)

/// Whether `word` is a clause word an `if`/`try` grammar matches by literal
/// value, so it must stay literal — aliasing it to `$var` would break the
/// clause parsing (body/expr index detection checks these by value).  The
/// set is the registry's clause-keyword vocabulary: the highlighted clause
/// keywords (`else` / `elseif` / `on` / `trap` / `finally`) plus the
/// non-highlighted clause noise word (`then`).
fn is_clause_keyword(word: &str) -> bool {
    tcl_registry::traits::clause_keywords_without_command_spec().contains(&word)
        || tcl_registry::traits::clause_noise_keywords().contains(&word)
}

/// Every compacted short name across the symbol map, so aggressive
/// aliases avoid colliding with a (possibly proc-local) compacted
/// name — a `$alias` used in command position must not resolve to a
/// local variable.
fn collect_symbol_shorts(sm: &SymbolMap) -> HashSet<String> {
    let mut out = HashSet::new();
    out.extend(sm.procs.values().cloned());
    for m in sm.variables.values() {
        out.extend(m.values().cloned());
    }
    out
}

/// Every variable name the compiler can see in `source`, for seeding
/// the aggressive tier's alias generators: the
/// analyser's per-scope variable tables (recursively), the SSA
/// symbol tables of every function unit (which include names only
/// *read* through name-taking commands, e.g. `[set a]`), and every
/// textual `$name` / `${name}` reference, plus inert UTF-8 name candidates.
/// Conservative observed candidate set; misses remain explicit debts and
/// do not establish source or physical-name absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveNameInventoryGap {
    VariableNameCoverage,
    CompilationUnit,
    ComplexityGuardedBody,
    SourceSyntaxCoverage,
}

#[derive(Debug)]
struct LiveNameInventory {
    names: HashSet<String>,
    gaps: Vec<LiveNameInventoryGap>,
}

fn collect_live_names(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> LiveNameInventory {
    let mut out: HashSet<String> = HashSet::new();

    // Analyser scope tables.
    let analysis = original::analysis_with_input(source, input);
    let mut stack: Vec<&Scope> = vec![&analysis.global_scope];
    while let Some(scope) = stack.pop() {
        out.extend(scope.variables.keys().cloned());
        stack.extend(scope.children.iter());
    }

    // SSA symbol tables (per function unit). Guarded units retain only a
    // deliberately incomplete trivial report; their names are candidates,
    // while their omitted names remain an explicit collision gap.
    let compilation_unit = crate::original_invocation::source_compilation_unit(
        source,
        &analysis,
        input.context_registry().commands(),
    );
    if let Some(cu) = &compilation_unit {
        for fu in cu.all_body_function_units() {
            out.extend(fu.ssa.var_names().iter().cloned());
        }
    }

    // Textual `$name` / `${name}` references.
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            // Collision reservations may include inert UTF-8 source candidates.
            // This superset grants no configured reference, lookup or edit.
            let start = i + 1;
            let end = tcl_core_types::naming::scan_var_name_end_with(bytes, start, true);
            if let Some(candidate) = source.get(start..end).filter(|name| !name.is_empty()) {
                out.insert(candidate.to_owned());
            }
            if let Ok(Some(reference)) = tcl_lexer::scan_var_ref(bytes, i, input.lexer_config())
                && let Ok(name) = std::str::from_utf8(reference.name)
            {
                out.insert(name.to_owned());
                if reference.index.is_none() {
                    i = reference.next.max(i + 1);
                    continue;
                }
            }
        }
        i += 1;
    }
    let mut gaps = Vec::new();
    let syntax_complete = crate::SourceSyntaxCoverage::capture(source, &analysis)
        .and_then(|coverage| coverage.applicability(source, &analysis))
        == Some(crate::SourceSyntaxCoverageApplicability::Complete);
    if !syntax_complete {
        gaps.push(LiveNameInventoryGap::SourceSyntaxCoverage);
    }
    if !analysis.original_variable_alias_source_is_complete(
        &tcl_lexer::SourceImage::document(source),
        input.lexer_config(),
    ) {
        gaps.push(LiveNameInventoryGap::VariableNameCoverage);
    }
    match &compilation_unit {
        None => gaps.push(LiveNameInventoryGap::CompilationUnit),
        Some(cu) if cu.all_body_function_units().any(|fu| fu.complexity_guarded) => {
            gaps.push(LiveNameInventoryGap::ComplexityGuardedBody);
        }
        Some(_) => {}
    }

    LiveNameInventory { names: out, gaps }
}

/// Configured syntax only; insertion, store, frame and dispatch obligations
/// remain the separate explicit aggressive-authoring policy, never Native grants.
struct AuthoredAliasSpelling {
    assignment: String,
    reference: String,
}

fn authored_alias_spelling(
    alias: &str,
    value: &str,
    config: LexerConfig,
) -> Option<AuthoredAliasSpelling> {
    let command = tcl_syntax::word_rules::lexical_unicode_source_word("set", config)?;
    let target = tcl_syntax::word_rules::lexical_unicode_source_word(alias, config)?;
    let literal = tcl_syntax::word_rules::lexical_unicode_source_word(value, config)?;
    let reference = tcl_syntax::naming::scalar_source_reference(alias, config, false)?;
    Some(AuthoredAliasSpelling {
        assignment: format!("{command} {target} {literal}\n"),
        reference,
    })
}

#[derive(Clone, Copy)]
enum AliasWordPurpose {
    CommandHead,
    Argument,
}

/// Reborrow original complete words from the existing configured source-phase
/// owner. Observed partial candidates provide no absence or complete inventory.
fn source_alias_uses(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    purpose: AliasWordPurpose,
) -> (Vec<String>, HashMap<String, Vec<usize>>) {
    let mut order = Vec::new();
    let mut uses = HashMap::<String, Vec<usize>>::new();
    let analysis = original::analysis_with_input(source, input);
    let Some(identities) = analysis.retained_command_realm() else {
        return (order, uses);
    };
    let context = input.context_registry();
    let env = MinifyEnv {
        original_source: source,
        analysis: &analysis,
        input,
        dialect: input.unit_profile(),
        config: input.lexer_config(),
        registry: context.commands(),
        context: &context,
        identities,
    };
    let Some(census) = lexical_metadata::command_census(source, env) else {
        return (order, uses);
    };
    let mut seen = HashSet::new();
    for command in census.observed_commands() {
        // Selected descriptor compression heuristic, not nominal lookup.
        // Unavailable metadata is unknown; unknown written commands retain the
        // existing explicit behavior-changing Logical authoring policy.
        let known_byte_compiled = matches!(purpose, AliasWordPurpose::CommandHead)
            && lexical_metadata::with_command_schema(env, command, |schema| {
                schema.semantics.traits.contains(Traits::BYTE_COMPILED)
            }) == Some(true);
        if known_byte_compiled {
            continue;
        }
        for (ordinal, word) in command.words.iter().enumerate() {
            if matches!(purpose, AliasWordPurpose::CommandHead) != (ordinal == 0)
                || word.group().expand
                || word.group().kind != tcl_lexer::WordKind::Bare
            {
                continue;
            }
            let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word)
                .and_then(|bytes| String::from_utf8(bytes).ok())
            else {
                continue;
            };
            let span = word.word_span();
            if value.len() < 3
                || source.get(span.as_range()) != Some(value.as_str())
                || (matches!(purpose, AliasWordPurpose::Argument) && is_clause_keyword(&value))
                || !seen.insert((span.start(), span.end()))
            {
                continue;
            }
            uses.entry(value.clone())
                .or_insert_with(|| {
                    order.push(value);
                    Vec::new()
                })
                .push(span.start() as usize);
        }
    }
    (order, uses)
}

/// Shared cost-benefit + apply for command / argument aliasing:
/// each name used ≥ 2 times becomes `$alias` with a `set alias name`
/// preamble when that saves bytes.  `used` accumulates claimed alias
/// names across phases.
fn alias_by_uses(
    source: &str,
    order: &[String],
    uses: &HashMap<String, Vec<usize>>,
    claimed: &mut HashSet<String>,
    config: LexerConfig,
) -> (String, BTreeMap<String, String>) {
    if order.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }
    let mut cands = order.to_vec();
    cands.sort_by(|a, b| {
        let (ka, kb) = (uses[a].len() * a.len(), uses[b].len() * b.len());
        kb.cmp(&ka).then_with(|| {
            order
                .iter()
                .position(|x| x == a)
                .cmp(&order.iter().position(|x| x == b))
        })
    });
    let mut names = AuthoredCompactNameSequence::new(config);
    let mut aliases: Vec<(String, String, AuthoredAliasSpelling)> = Vec::new();
    for name in &cands {
        let count = uses[name].len();
        if count < 2 {
            continue;
        }
        let Some(alias) = names.next_avoiding_observed(|name| claimed.contains(name), None) else {
            continue;
        };
        let alias = alias.into_name();
        let Some(spelling) = authored_alias_spelling(&alias, name, config) else {
            continue;
        };
        if uses[name].iter().any(|off| {
            off.checked_add(name.len())
                .and_then(|end| source.get(*off..end))
                != Some(name.as_str())
        }) {
            continue;
        }
        let original_cost = count * name.len();
        let aliased_cost = spelling.assignment.len() + count * spelling.reference.len();
        if aliased_cost >= original_cost {
            continue;
        }
        claimed.insert(alias.clone());
        aliases.push((name.clone(), alias, spelling));
    }
    if aliases.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }
    let mut edits: Vec<Edit> = Vec::new();
    let mut preamble = String::new();
    let mut map = BTreeMap::new();
    for (name, alias, spelling) in &aliases {
        preamble.push_str(&spelling.assignment);
        for &off in &uses[name] {
            edits.push((off, name.len(), spelling.reference.clone()));
        }
        map.insert(name.clone(), alias.clone());
    }
    let body = apply_edits(source, edits);
    (format!("{preamble}{body}"), map)
}

/// Alias repeated long command names (`HTTP::uri` → `$a`) — the first of the
/// three aliasing passes.
fn alias_repeated_commands(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    claimed: &mut HashSet<String>,
) -> (String, BTreeMap<String, String>) {
    let (order, uses) = source_alias_uses(source, input, AliasWordPurpose::CommandHead);
    alias_by_uses(source, &order, &uses, claimed, input.lexer_config())
}

/// Emit unique-prefix keyword abbreviations, after the aliasing passes.
///
/// Tcl's `Tcl_GetIndexFromObj` accepts any unique prefix of an ensemble
/// subcommand or an `-option`, and `Tcl_GetBoolean` any unique prefix of a
/// boolean word, so `string equal -nocase` can be written `string eq -noc` —
/// a pure length win on top of whitespace stripping, and exactly what
/// hand-minified iRules already do, but done correctly.
///
/// Every abbreviation is computed by the registry
/// ([`tcl_registry::abbrev`]); the minifier never pattern-matches a command
/// name. Two safety rules make the rewrite observationally invisible:
///
/// * **Version-range safety.** A prefix unique today can become ambiguous
///   when a later release adds a keyword (`string cat` in 8.6.2 shortened
///   what `string c…` could mean). The abbreviation is computed against the
///   target dialect's table *and every later core-Tcl table*, so minified
///   output stays correct if it is later run on a newer interpreter.
/// * **Only dispatch-consumed words.** A subcommand or option word is
///   consumed by dispatch and never observable as a string. Boolean *values*
///   are not abbreviated here at all: `set flag true` is a value-definition
///   site whose bytes may be observed (`eq "true"`, a `switch` arm, `string
///   length`), and the minifier has no proof otherwise.
///
/// Abstains on strict tables, dynamic and `{*}`-expanded words, and anything
/// the registry does not resolve `Unique`.
fn abbreviate_keywords(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> (String, usize) {
    let analysis = original::analysis_with_input(source, input);
    let Some(identities) = analysis.retained_command_realm() else {
        return (source.to_owned(), 0);
    };
    let context = input.context_registry();
    lexical_keywords::abbreviate(
        source,
        MinifyEnv {
            original_source: source,
            analysis: &analysis,
            input,
            dialect: input.unit_profile(),
            config: input.lexer_config(),
            context: &context,
            registry: context.commands(),
            identities,
        },
    )
}

/// Alias repeated literal arguments (`-normalized` → `$a`) — the second
/// aliasing pass.
fn alias_repeated_arguments(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    claimed: &mut HashSet<String>,
) -> (String, BTreeMap<String, String>) {
    let (order, uses) = source_alias_uses(source, input, AliasWordPurpose::Argument);
    alias_by_uses(source, &order, &uses, claimed, input.lexer_config())
}

/// Build a suffix array for `text` (naive sort of byte suffixes).
fn build_suffix_array(text: &[u8]) -> Vec<usize> {
    let mut sa: Vec<usize> = (0..text.len()).collect();
    sa.sort_by(|&a, &b| text[a..].cmp(&text[b..]));
    sa
}

/// Kasai's LCP array from a suffix array.
fn build_lcp_array(text: &[u8], sa: &[usize]) -> Vec<usize> {
    let n = text.len();
    if n == 0 {
        return Vec::new();
    }
    let mut rank = vec![0usize; n];
    for (i, &s) in sa.iter().enumerate() {
        rank[s] = i;
    }
    let mut lcp = vec![0usize; n];
    let mut k = 0usize;
    for i in 0..n {
        if rank[i] == n - 1 {
            k = 0;
            continue;
        }
        let j = sa[rank[i] + 1];
        while i + k < n && j + k < n && text[i + k] == text[j + k] {
            k += 1;
        }
        lcp[rank[i]] = k;
        k = k.saturating_sub(1);
    }
    lcp
}

/// Find repeated substrings (≥ 3 bytes, ≥ 2 sentinel-free
/// occurrences) across the quoted-string segments of `source`,
/// returning `substring -> source byte offsets`.  Steps 1–5 of
/// `_alias_string_literals`.
fn string_alias_candidates(segments: &[StringLiteralFragment]) -> BTreeMap<Vec<u8>, Vec<usize>> {
    if segments.is_empty() {
        return BTreeMap::new();
    }
    // Concatenate segments with a `\0` sentinel; map concat byte →
    // source byte offset (`usize::MAX` for sentinels).
    let mut concat: Vec<u8> = Vec::new();
    let mut concat_to_src: Vec<usize> = Vec::new();
    for fragment in segments {
        let src_offset = fragment.span.start() as usize;
        for (j, b) in fragment.text.bytes().enumerate() {
            concat.push(b);
            concat_to_src.push(src_offset + j);
        }
        concat.push(0);
        concat_to_src.push(usize::MAX);
    }

    let sa = build_suffix_array(&concat);
    let lcp = build_lcp_array(&concat, &sa);
    let min_len = 3;

    let mut candidates: HashSet<Vec<u8>> = HashSet::new();
    for i in 0..sa.len().saturating_sub(1) {
        if lcp[i] < min_len {
            continue;
        }
        let (p1, p2) = (sa[i], sa[i + 1]);
        if concat_to_src[p1] == usize::MAX || concat_to_src[p2] == usize::MAX {
            continue;
        }
        let mut actual = 0;
        for k in 0..lcp[i] {
            if p1 + k >= concat_to_src.len()
                || p2 + k >= concat_to_src.len()
                || concat_to_src[p1 + k] == usize::MAX
                || concat_to_src[p2 + k] == usize::MAX
            {
                break;
            }
            actual = k + 1;
        }
        if actual >= min_len {
            candidates.insert(concat[p1..p1 + actual].to_vec());
        }
    }

    let mut occ: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
    for sub in &candidates {
        let mut offsets = Vec::new();
        let mut start = 0;
        while let Some(idx) = find_subslice(&concat, sub, start) {
            if !(0..sub.len()).any(|k| concat_to_src[idx + k] == usize::MAX) {
                offsets.push(concat_to_src[idx]);
            }
            start = idx + 1;
        }
        if offsets.len() >= 2 {
            occ.insert(sub.clone(), offsets);
        }
    }
    occ
}

/// Alias repeated substrings inside double-quoted strings — the last
/// aliasing pass. (+ `_collect_string_literals`).
fn alias_string_literals(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
    claimed: &mut HashSet<String>,
) -> (String, BTreeMap<String, String>) {
    let fragments = collect_string_literals(source, input);
    let occ = string_alias_candidates(&fragments);
    if occ.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }

    // Step 6: score, then greedily select non-overlapping aliases.
    let mut scored: Vec<(usize, Vec<u8>, Vec<usize>)> = Vec::new();
    for (sub, offsets) in &occ {
        let count = offsets.len();
        let original_cost = count * sub.len();
        let preamble_cost = 4 + 1 + 1 + 1 + sub.len() + 1 + 1;
        let aliased_cost = preamble_cost + count * 2;
        let savings = original_cost.saturating_sub(aliased_cost);
        if savings > 0 {
            scored.push((savings, sub.clone(), offsets.clone()));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.len().cmp(&a.1.len())));

    let mut names = AuthoredCompactNameSequence::new(input.lexer_config());
    let mut claimed_bytes: HashSet<usize> = HashSet::new();
    let mut aliases: Vec<(String, String, Vec<Edit>)> = Vec::new();
    for (_, sub, offsets) in scored {
        let Ok(sub_string) = std::str::from_utf8(&sub) else {
            continue;
        };
        let preamble_word = format!("{{{sub_string}}}");
        if standalone_alias_word(&preamble_word, input.lexer_config())
            .and_then(|word| tcl_syntax::word_rules::original_static_word_unicode_value(&word))
            .as_deref()
            != Some(sub.as_slice())
        {
            continue;
        }
        let free: Vec<usize> = offsets
            .iter()
            .copied()
            .filter(|&off| !(off..off + sub.len()).any(|p| claimed_bytes.contains(&p)))
            .collect();
        if free.len() < 2 {
            continue;
        }
        let Some(alias) = names.next_avoiding_observed(|name| claimed.contains(name), None) else {
            continue;
        };
        let alias = alias.into_name();
        let count = free.len();
        let original_cost = count * sub.len();
        let preamble_cost = 4 + alias.len() + 1 + 1 + sub.len() + 1 + 1;
        let proposals = free
            .iter()
            .map(|&off| {
                let end = off.checked_add(sub.len())?;
                let fragment = fragments.iter().find(|fragment| {
                    fragment.span.start() as usize <= off && end <= fragment.span.end() as usize
                })?;
                let replacement =
                    alias_string_reference(source, fragment, off, end, &alias, sub_string)?;
                Some((off, sub.len(), replacement))
            })
            .collect::<Option<Vec<_>>>();
        let Some(proposals) = proposals else {
            continue;
        };
        let aliased_cost = preamble_cost + proposals.iter().map(|edit| edit.2.len()).sum::<usize>();
        if aliased_cost >= original_cost {
            continue;
        }
        claimed.insert(alias.clone());
        for &off in &free {
            for p in off..off + sub.len() {
                claimed_bytes.insert(p);
            }
        }
        aliases.push((sub_string.to_owned(), alias, proposals));
    }
    if aliases.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }

    // Step 7: build preamble + edits.
    let mut preamble = String::new();
    let mut edits: Vec<Edit> = Vec::new();
    let mut map = BTreeMap::new();
    for (sub, alias, proposals) in aliases {
        let _ = writeln!(preamble, "set {alias} {{{sub}}}");
        edits.extend(proposals);
        map.insert(sub, alias);
    }
    let body = apply_edits(source, edits);
    (format!("{preamble}{body}"), map)
}

struct StringLiteralFragment {
    span: Span,
    text: String,
    config: LexerConfig,
    following_end: usize,
}

/// Original quoted text components from the selected source-role inventory.
fn collect_string_literals(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> Vec<StringLiteralFragment> {
    let analysis = original::analysis_with_input(source, input);
    let Some(identities) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    let context = input.context_registry();
    let env = MinifyEnv {
        original_source: source,
        analysis: &analysis,
        input,
        dialect: input.unit_profile(),
        config: input.lexer_config(),
        registry: context.commands(),
        context: &context,
        identities,
    };
    let Some(census) = lexical_metadata::command_census(source, env) else {
        return Vec::new();
    };
    let mut fragments = Vec::new();
    for command in census.observed_commands() {
        for word in command.words.iter().skip(1) {
            if word.group().kind != tcl_lexer::WordKind::Quoted {
                continue;
            }
            let Ok(word_span) = word.content_span() else {
                continue;
            };
            let arena = word.executable_parts();
            for component in arena.list(arena.root()) {
                let span = component.span;
                if matches!(component.part, tcl_lexer::ExecutablePart::Text(_))
                    && let Some(text) = source.get(span.as_range())
                    && text.len() >= 2
                {
                    fragments.push(StringLiteralFragment {
                        span,
                        text: text.to_owned(),
                        config: word.config(),
                        following_end: word_span.end() as usize,
                    });
                }
            }
        }
    }
    fragments.sort_by_key(|fragment| fragment.span.start());
    fragments.dedup_by_key(|fragment| fragment.span);
    fragments
}

fn standalone_alias_word(source: &str, config: LexerConfig) -> Option<tcl_lexer::NativeWord> {
    let image = tcl_lexer::SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image,
        Span::new(0, u32::try_from(source.len()).ok()?),
        config,
    )
    .ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    let mut words = plan.commands.into_iter().next()?.words;
    if words.len() != 1 || words[0].word_span().as_range() != (0..source.len()) {
        return None;
    }
    words.pop()
}

/// Hypothetical value comparison supplies syntax only; alias movement keeps its
/// separate aggressive-authoring admission.
fn alias_string_reference(
    source: &str,
    fragment: &StringLiteralFragment,
    start: usize,
    end: usize,
    alias: &str,
    value: &str,
) -> Option<String> {
    let prefix = source.get(fragment.span.start() as usize..start)?;
    let suffix = source.get(end..fragment.span.end() as usize)?;
    if source.get(start..end)? != value {
        return None;
    }
    let config = fragment.config;
    let replacement = [false, true].into_iter().find_map(|braced| {
        let candidate = tcl_syntax::naming::scalar_source_reference(alias, config, braced)?;
        let following = source.get(end..fragment.following_end)?;
        let joined = format!("{candidate}{following}");
        let reference = tcl_lexer::scan_var_ref(joined.as_bytes(), 0, config).ok()??;
        (reference.next == candidate.len()
            && reference.name == alias.as_bytes()
            && reference.index.is_none())
        .then_some(candidate)
    })?;
    let original = standalone_alias_word(&format!("\"{}\"", fragment.text), config)?;
    let original_value = tcl_syntax::word_rules::original_static_word_unicode_value(&original)?;
    let proposed = standalone_alias_word(&format!("\"{prefix}{replacement}{suffix}\""), config)?;
    let mut selected = false;
    let proposed_value = tcl_syntax::word_rules::original_word_unicode_value_with_variables(
        &proposed,
        |arena, part| {
            let tcl_lexer::ExecutablePart::Variable { name, index: None } = part.part else {
                return None;
            };
            if selected
                || arena.bytes(name)? != alias.as_bytes()
                || part.span.as_range() != (1 + prefix.len()..1 + prefix.len() + replacement.len())
            {
                return None;
            }
            selected = true;
            Some(value.as_bytes().to_vec())
        },
    )?;
    (selected && original_value == proposed_value).then_some(replacement)
}

/// First index ≥ `from` where `needle` occurs in `haystack`.
fn find_subslice(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from + needle.len() > haystack.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

// Static-substring folding: the aggressive tier's pass between the
// optimiser rewrites and name compaction.

/// Taint colours that prove a fixed-form value, so folding stays
/// safe even when the underlying value is tainted.
fn safe_taint_colours() -> TaintColour {
    TaintColour::IP_ADDRESS
        | TaintColour::PORT
        | TaintColour::FQDN
        | TaintColour::LIST_CANONICAL
        | TaintColour::REGEX_LITERAL
}

/// Fold only original quoted words whose exact reads have independent erasure proof.
fn fold_original_static_substrings(
    source: &str,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> (String, usize, BTreeMap<String, String>) {
    let analysis = original::analysis_with_input(source, input);
    let context = input.context_registry();
    let registry = context.commands();
    let Some(unit) =
        crate::original_invocation::source_compilation_unit(source, &analysis, registry)
    else {
        return (source.to_owned(), 0, BTreeMap::new());
    };
    if unit.ir_module.source_metadata_input.as_ref() != Some(input) {
        return (source.to_owned(), 0, BTreeMap::new());
    }
    let mut edits = BTreeMap::new();
    let mut folds = BTreeMap::new();
    for function in unit.all_body_function_units() {
        if function
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .is_none()
        {
            continue;
        }
        for (&block, body) in &function.ssa.blocks {
            if !function.sccp.executable_blocks.contains(&block) {
                continue;
            }
            for index in 0..body.statements.len() {
                let view =
                    tcl_compiler::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let Some(original) = tcl_compiler::ssa::original_analysis_words_for_statement(
                    &unit, function, block, index, registry,
                ) else {
                    continue;
                };
                let config = original.config();
                for word in tokens.words().iter().skip(1) {
                    let tcl_compiler::ir::WordExpr::Template {
                        parts,
                        rejected: None,
                        ..
                    } = word
                    else {
                        continue;
                    };
                    let span = function.abs_span(word.source().span);
                    if !original
                        .words()
                        .iter()
                        .any(|source_word| source_word.span() == span)
                    {
                        continue;
                    }
                    let Some(inner_text) = source
                        .get(span.as_range())
                        .and_then(|source_text| source_text.strip_prefix('"'))
                        .and_then(|source_text| source_text.strip_suffix('"'))
                    else {
                        continue;
                    };
                    let Some(value) =
                        fold_original_word_parts(&unit, function, view, parts, registry, config)
                    else {
                        continue;
                    };
                    let Some(replacement) =
                        tcl_syntax::word_rules::lexical_unicode_source_word(&value, config)
                    else {
                        continue;
                    };
                    edits.insert((span.start(), span.end()), replacement);
                    folds.insert(inner_text.to_owned(), value);
                }
            }
        }
    }
    let count = edits.len();
    let edits = edits
        .into_iter()
        .map(|((start, end), replacement)| (start as usize, (end - start) as usize, replacement))
        .collect();
    (apply_edits(source, edits), count, folds)
}

fn fold_original_word_parts(
    unit: &CompilationUnit,
    function: &FunctionUnit,
    view: tcl_compiler::ssa::SsaSourceView<'_>,
    parts: &[tcl_compiler::ir::WordPart],
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<String> {
    let mut value = String::new();
    let mut substituted = false;
    for part in parts {
        match part {
            tcl_compiler::ir::WordPart::Text { text, .. } => {
                value.push_str(&tcl_lexer::backslash_subst_in(text, config.escapes));
            }
            tcl_compiler::ir::WordPart::Variable { spelling, source } => {
                let physical = view.replaceable_read_at(source, spelling, registry)?;
                let version = physical.version?;
                let original = tcl_compiler::ssa::original_source_read_at_unit(
                    unit,
                    function.abs_span(source.span),
                    registry,
                )?;
                if !std::ptr::eq(original.function(), function)
                    || original.symbol() != physical.symbol
                    || original.version() != Some(version)
                {
                    return None;
                }
                let key = (physical.symbol, version);
                if function.taints.get(&key).is_some_and(|taint| {
                    taint.is_tainted() && !taint.colours.intersects(safe_taint_colours())
                }) {
                    return None;
                }
                let LatticeValue::Const(constant) = function.sccp.values.get(&key)? else {
                    return None;
                };
                value.push_str(&const_to_string(constant)?);
                substituted = true;
            }
            tcl_compiler::ir::WordPart::CommandSubstitution { .. }
            | tcl_compiler::ir::WordPart::Opaque { .. } => return None,
        }
    }
    substituted.then_some(value)
}

/// Replace dynamic quoted strings with their static values where the
/// compiler's SCCP pass proves every `$var` substitution is a
/// compile-time constant and no unsanitised tainted value is
/// involved.  Folds `$var` interpolations resolving to integer /
/// string constants; bails on command substitutions (`[…]`) and on
/// boolean / float constants.  Returns `(folded_source, fold_count,
/// fold_map)`.
#[cfg(test)]
fn fold_static_substrings(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> (String, usize, BTreeMap<String, String>) {
    let cu = CompilationUnit::build_for_profile(source, registry, false, dialect);
    let mut edits: Vec<Edit> = Vec::new();
    let mut fold_map: BTreeMap<String, String> = BTreeMap::new();

    let mut scopes: Vec<&FunctionUnit> = vec![&cu.top_level];
    scopes.extend(cu.procedures.values());
    for fu in scopes {
        let config = LexerConfig::for_profile(Some(dialect));
        let cx = FoldContext {
            fu,
            style: config.braced_var,
            config,
            view: None,
        };
        collect_folds_for_scope(source, cx, &mut edits, &mut fold_map);
    }
    if edits.is_empty() {
        return (source.to_owned(), 0, fold_map);
    }
    // Deduplicate by (offset, length).
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut unique: Vec<Edit> = Vec::new();
    for e in edits {
        if seen.insert((e.0, e.1)) {
            unique.push(e);
        }
    }
    let fold_count = unique.len();
    let result = apply_edits(source, unique);
    (result, fold_count, fold_map)
}

/// The read-only context a constant-fold decision needs: the scope's
/// analysed unit and the document's `${…}` close rule (which decides where
/// a `${…}` reference's name ends).
#[cfg(test)]
#[derive(Clone, Copy)]
struct FoldContext<'a> {
    fu: &'a FunctionUnit,
    style: tcl_dialect::BracedVarStyle,
    config: LexerConfig,
    view: Option<tcl_compiler::ssa::SsaSourceView<'a>>,
}

/// Collect static-fold edits for one function scope.
#[cfg(test)]
fn collect_folds_for_scope(
    source: &str,
    cx: FoldContext<'_>,
    edits: &mut Vec<Edit>,
    fold_map: &mut BTreeMap<String, String>,
) {
    let (fu, style) = (cx.fu, cx.style);
    for (block_id, block) in &fu.cfg.blocks {
        let Some(ssa_block) = fu.ssa.blocks.get(block_id) else {
            continue;
        };
        if !fu.sccp.executable_blocks.contains(block_id) {
            continue;
        }
        let mut block_vars = ssa_block.entry_versions.clone();
        for (stmt_idx, stmt) in block.statements.iter().enumerate() {
            let Some(ssa_stmt) = ssa_block.statements.get(stmt_idx) else {
                continue;
            };
            let mut uses = block_vars.clone();
            uses.extend(
                ssa_stmt
                    .uses
                    .iter()
                    .map(|(&symbol, &version)| (symbol, version)),
            );
            let view = tcl_compiler::ssa::SsaSourceView::at_statement(&fu.ssa, *block_id, stmt_idx);
            let cx = FoldContext {
                view: Some(view),
                ..cx
            };
            match stmt {
                Statement::AssignValue {
                    span,
                    value,
                    value_needs_backsubst: true,
                    ..
                } if value.contains('$') || value.contains('[') => {
                    try_fold_region(source, *span, value, &uses, cx, edits, fold_map);
                }
                Statement::Call {
                    tokens: Some(toks), ..
                } => {
                    for i in 1..toks.argv.len() {
                        let arg_off = toks.argv[i].start() as usize;
                        if source.as_bytes().get(arg_off) != Some(&b'"') {
                            continue;
                        }
                        let content = &toks.argv_texts[i];
                        if !(content.contains('$') || content.contains('[')) {
                            continue;
                        }
                        if let Some(folded) =
                            fold_string_via_sccp(content, &uses, &fu.sccp.values, view, style)
                        {
                            if has_unsafe_tainted_inputs(content, &uses, &fu.taints, view, style) {
                                continue;
                            }
                            if let Some(close) = close_quote_offset(source, arg_off)
                                && let Some(replacement) =
                                    tcl_syntax::word_rules::lexical_unicode_source_word(
                                        &folded, cx.config,
                                    )
                            {
                                edits.push((arg_off, close - arg_off + 1, replacement));
                                fold_map.insert(content.clone(), folded);
                            }
                        }
                    }
                }
                _ => {}
            }
            for (name, ver) in &ssa_stmt.defs {
                block_vars.insert(*name, *ver);
            }
        }
    }
}

/// Fold the quoted string inside the `span` region of an
/// `AssignValue` (`set x "…"`).
#[cfg(test)]
fn try_fold_region(
    source: &str,
    span: Span,
    value: &str,
    uses: &HashMap<tcl_compiler::ssa::Symbol, Version>,
    cx: FoldContext<'_>,
    edits: &mut Vec<Edit>,
    fold_map: &mut BTreeMap<String, String>,
) {
    let (fu, style) = (cx.fu, cx.style);
    let Some(view) = cx.view else {
        return;
    };
    let Some(folded) = fold_string_via_sccp(value, uses, &fu.sccp.values, view, style) else {
        return;
    };
    if has_unsafe_tainted_inputs(value, uses, &fu.taints, view, style) {
        return;
    }
    let (start, end) = (span.start() as usize, span.end() as usize);
    let region = &source[start..end.min(source.len())];
    let Some(q) = region.find('"') else {
        return;
    };
    let abs_start = start + q;
    let Some(close) = close_quote_offset(source, abs_start) else {
        return;
    };
    let inner = &source[abs_start + 1..close];
    if !(inner.contains('$') || inner.contains('[')) {
        return;
    }
    let Some(replacement) = tcl_syntax::word_rules::lexical_unicode_source_word(&folded, cx.config)
    else {
        return;
    };
    edits.push((abs_start, close - abs_start + 1, replacement));
    fold_map.insert(inner.to_owned(), folded);
}

/// Resolve a quoted-string body to a static value via SCCP, or
/// `None` when any substitution is non-constant.  `$var` only;
/// command substitutions (`[…]`) bail out.
#[cfg(test)]
fn fold_string_via_sccp(
    content: &str,
    uses: &HashMap<tcl_compiler::ssa::Symbol, Version>,
    values: &HashMap<(tcl_compiler::ssa::Symbol, Version), LatticeValue>,
    view: tcl_compiler::ssa::SsaSourceView<'_>,
    style: tcl_dialect::BracedVarStyle,
) -> Option<String> {
    let bytes = content.as_bytes();
    let n = bytes.len();
    let mut out = String::new();
    let mut has_dynamic = false;
    let mut pos = 0;
    while pos < n {
        match bytes[pos] {
            b'$' => {
                let (end, name) = parse_var_ref(content, pos, style);
                let name = name?;
                let symbol = view.symbol(name)?;
                let ver = uses.get(&symbol).copied().unwrap_or(0);
                if ver == 0 {
                    return None;
                }
                let lv = values.get(&(symbol, ver))?;
                let LatticeValue::Const(cv) = lv else {
                    return None;
                };
                out.push_str(&const_to_string(cv)?);
                has_dynamic = true;
                pos = end;
            }
            b'[' => return None, // command substitution — not handled.
            b'\\' if pos + 1 < n => {
                match bytes[pos + 1] {
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    _ => out.push_str(&content[pos + 1..pos + 1 + utf8_len(bytes[pos + 1])]),
                }
                pos += 1 + utf8_len(bytes[pos + 1]);
            }
            _ => {
                let len = utf8_len(bytes[pos]);
                out.push_str(&content[pos..pos + len]);
                pos += len;
            }
        }
    }
    if has_dynamic { Some(out) } else { None }
}

/// Render an integer / string SCCP constant; `None` for boolean /
/// float (rendering is ambiguous, so folding bails for safety).
fn const_to_string(cv: &ConstValue) -> Option<String> {
    match cv {
        ConstValue::Int(n) => Some(n.to_string()),
        ConstValue::String(s) => Some(s.clone()),
        ConstValue::Bool(_) | ConstValue::Float(_) => None,
    }
}

/// Whether any `$var` in `content` is tainted without a
/// fixed-form mitigation colour.
#[cfg(test)]
fn has_unsafe_tainted_inputs(
    content: &str,
    uses: &HashMap<tcl_compiler::ssa::Symbol, Version>,
    taints: &HashMap<(tcl_compiler::ssa::Symbol, Version), TaintLattice>,
    view: tcl_compiler::ssa::SsaSourceView<'_>,
    style: tcl_dialect::BracedVarStyle,
) -> bool {
    let safe = safe_taint_colours();
    let mut pos = 0;
    let bytes = content.as_bytes();
    while pos < bytes.len() {
        if bytes[pos] == b'$' {
            let (end, name) = parse_var_ref(content, pos, style);
            if let Some(name) = name {
                let Some(symbol) = view.symbol(name) else {
                    return true;
                };
                let ver = uses.get(&symbol).copied().unwrap_or(0);
                if ver > 0
                    && let Some(t) = taints.get(&(symbol, ver))
                    && t.is_tainted()
                    && !t.colours.intersects(safe)
                {
                    return true;
                }
                pos = end;
            } else {
                pos += 1;
            }
        } else {
            pos += 1;
        }
    }
    false
}

/// Fixture scalar eligibility uses the shared reference extent and grammar.
#[cfg(test)]
fn scalar_reference_at(text: &str, pos: usize, config: LexerConfig) -> (usize, Option<&str>) {
    if text.as_bytes().get(pos) != Some(&b'$') {
        return (pos.saturating_add(1), None);
    }
    let Ok(Some(reference)) = tcl_lexer::scan_var_ref(text.as_bytes(), pos, config) else {
        return (pos.saturating_add(1), None);
    };
    if reference.index.is_some() {
        return (reference.next, None);
    }
    (reference.next, std::str::from_utf8(reference.name).ok())
}

#[cfg(test)]
fn parse_var_ref(
    text: &str,
    pos: usize,
    style: tcl_dialect::BracedVarStyle,
) -> (usize, Option<&str>) {
    scalar_reference_at(
        text,
        pos,
        LexerConfig {
            braced_var: style,
            ..LexerConfig::default()
        },
    )
}

/// Project shared complete groups into the renderer's argument container.
/// Group kinds/ranges and expansion ownership come from the same lexer owner.
fn parse_commands(source: &str, tokens: &[Token], config: LexerConfig) -> Vec<Vec<Arg>> {
    // naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    tcl_lexer::group_commands(tokens, source, config)
        .into_iter()
        .map(|command| {
            command
                .words
                .into_iter()
                .map(|word| {
                    let mut fragments = command
                        .expand_markers
                        .iter()
                        .copied()
                        .filter(|index| {
                            word.expand
                                && *index < word.tokens.start
                                && tokens[*index].span.end() <= word.span.start()
                                && !tokens[*index + 1..word.tokens.start]
                                    .iter()
                                    .any(|token| token.kind != TokenType::Expand)
                        })
                        .map(|index| tokens[index])
                        .collect::<Vec<_>>();
                    fragments.extend_from_slice(&tokens[word.tokens]);
                    Arg {
                        tokens: fragments,
                        is_braced: word.kind == tcl_lexer::WordKind::Braced,
                        is_quoted: word.kind == tcl_lexer::WordKind::Quoted,
                    }
                })
                .collect()
        })
        .collect()
}

fn render_command(
    sm: &SourceMap,
    cmd_args: &[Arg],
    original: Option<&tcl_lexer::NativeScriptCommandWords>,
    env: MinifyEnv<'_>,
    depth: u32,
) -> Option<Vec<String>> {
    let words = original.and_then(|command| {
        tcl_compiler::registry_invocation::source_structure::original_logical_registry_words_for_command(
            env.identities, env.input, command,
        )
    });
    // Effective aliases and expansions retain their original producers. Only
    // a genuine complete written operand can receive a whole-word rewrite.
    let written_role = |index: usize, role: ArgRole| {
        let words = words.as_ref()?;
        let argument = words.roles()?.iter().find_map(|&(argument, selected)| {
            (selected == role
                && words.origins().get(argument + 1)
                    == Some(
                        &tcl_compiler::registry_invocation::InvocationWordOrigin::Written(index),
                    ))
            .then_some(argument)
        })?;
        let operand = words.operands().get(argument)?.as_ref()?.word()?;
        (original?.words.get(index)? == operand).then_some(())
    };
    let bodies = words
        .as_ref()
        .map(|words| words.source_script_bodies(env.context));
    let case = words.as_ref().and_then(|words| {
        words.with_source_schema(env.context, |schema| {
            schema.authored_source_case_invocation()
        })?
    });
    let list_index = case.as_ref().and_then(|(_, layout)| {
        let ordinal = layout.clause_list_index?.checked_add(1)?;
        match words.as_ref()?.origins().get(ordinal)? {
            tcl_compiler::registry_invocation::InvocationWordOrigin::Written(index) => Some(*index),
            _ => None,
        }
    });
    let mut out: Vec<String> = Vec::with_capacity(cmd_args.len());
    for (i, arg) in cmd_args.iter().enumerate() {
        let single_braced = arg.is_braced && arg.tokens.len() == 1;
        let original_word = original.and_then(|command| command.words.get(i));
        let selected_body = original_word.and_then(|word| {
            let content = word.content_span().ok()?;
            bodies
                .as_ref()?
                .iter()
                .find(|body| body.original_container() == word && body.content_span() == content)
        });
        if list_index == Some(i) && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            let minified =
                minify_case_list(inner, &case.as_ref()?.0, env, depth + 1, bodies.as_deref()?);
            out.push(format!("{{{minified}}}"));
        } else if selected_body.is_some() && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            out.push(format!(
                "{{{}}}",
                minify_body(
                    inner,
                    env,
                    depth + 1,
                    tcl_lexer::SourceSegmentationPurpose::NestedScript
                )
            ));
        } else if written_role(i, ArgRole::LambdaLiteral).is_some() && single_braced {
            out.push(minify_lambda_literal(
                sm,
                arg.tokens[0],
                original_word?,
                env,
                depth + 1,
            ));
        } else if written_role(i, ArgRole::Expr).is_some() && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            out.push(format!("{{{}}}", compress_expr(inner, env, depth + 1)));
        } else {
            out.push(reconstruct_arg(sm, arg, original_word, env, depth)?);
        }
    }
    Some(out)
}

/// Original whole-command words under the actual complete body configuration.
#[cfg(test)]
fn lexical_script_words(
    source: &str,
    config: LexerConfig,
) -> Option<tcl_lexer::NativeScriptWordsPlan> {
    // naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    let image = tcl_lexer::SourceImage::document(source);
    let region = tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?);
    tcl_lexer::native_script_words_in(image, region, config).ok()
}

/// Static ASCII metadata value of the exact complete original head word.
#[cfg(test)]
fn lexical_head_value(
    sm: &SourceMap,
    arg: &Arg,
    word: &tcl_lexer::NativeWord,
    config: LexerConfig,
) -> Option<String> {
    // naming.minifier.complete-lexical-head
    // docs/design/analysis/name-resolution-proofs/minifier-complete-lexical-head.md
    if word.image().channel() != tcl_lexer::SourceChannel::Document
        || word.image().bytes() != sm.source().as_bytes()
        || word.config() != config
        || word.tokens() != arg.tokens.as_slice()
    {
        return None;
    }
    let value = tcl_syntax::word_rules::original_static_word_ascii_presentation(word)?;
    String::from_utf8(value).ok()
}

/// Minify the selected lambda's body, retaining complete configured original
/// field values. A cooked field has no original region or nested role receipt.
fn minify_lambda_literal(
    sm: &SourceMap,
    tok: Token,
    word: &tcl_lexer::NativeWord,
    env: MinifyEnv<'_>,
    depth: u32,
) -> String {
    let fallback = || format!("{{{}}}", sm.token_text(tok));
    let Some(elems) =
        tcl_compiler::lambda_literal::split_original_lambda_literal_lexical_decoded(word)
    else {
        return fallback();
    };
    let Some(decoded_body) = elems.body.as_deref() else {
        return fallback();
    };
    let body = tcl_compiler::lambda_literal::split_original_lambda_literal(word)
        .and_then(|lambda| lambda.braced_body())
        .and_then(|span| env.original_source.get(span.as_range()))
        .filter(|body| *body == decoded_body)
        .unwrap_or(decoded_body);
    let minified_body = minify_body(
        body,
        env,
        depth,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    );
    let mut fields = vec![elems.params.as_ref(), minified_body.as_str()];
    if let Some(namespace) = elems.namespace.as_deref() {
        fields.push(namespace);
    }
    let value = fields
        .iter()
        .map(|field| tcl_syntax::list::list_element(field))
        .collect::<Vec<_>>()
        .join(" ");
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&env.config);
    let Ok(reparsed) = rules.split_list(&value) else {
        return fallback();
    };
    if reparsed.len() != fields.len()
        || reparsed
            .iter()
            .zip(&fields)
            .any(|(parsed, field)| parsed.as_ref() != *field)
    {
        return fallback();
    }
    tcl_syntax::word_rules::lexical_unicode_source_word(&value, env.config).unwrap_or_else(fallback)
}

/// Readonly source roles and case shape from the actual retained generation.
#[cfg(test)]
fn lexical_source_schema<'r: 'w, 'w, T>(
    env: MinifyEnv<'r>,
    name: &'w str,
    arguments: &'w [tcl_registry::InvocationWord<'w>],
    project: impl FnOnce(&tcl_registry::ResolvedInvocation<'r, 'w>) -> T,
) -> Option<T> {
    // naming.minifier.retained-lexical-context
    // docs/design/analysis/name-resolution-proofs/minifier-retained-lexical-context.md
    if name.is_empty()
        || env.context.commands().snapshot().semantic_key()
            != env.registry.snapshot().semantic_key()
    {
        return None;
    }
    let mut dialect = tcl_registry::InvocationDialect::of_profile(env.dialect);
    dialect.lexer_grammar = env.config.grammar_over(dialect.lexer_grammar);
    let words = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(name),
        arguments,
    )
    .with_dialect(dialect);
    tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
        env.registry,
        Some(env.context.context()),
        words,
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
    .resolved()
    .as_ref()
    .map(project)
}

/// One original reference, preserving arrays and checking scalar proposals against
/// the next rendered component under the complete selected source grammar.
fn reconstruct_variable_reference(
    sm: &SourceMap,
    tok: Token,
    following: &str,
    config: LexerConfig,
) -> Option<String> {
    let source = sm.source_bytes();
    let at = tok.span.as_range().start;
    let original = tcl_lexer::word_parts::scan_var_ref(source, at, config).ok()??;
    if original.source_span(source, at, 0)? != tok.span
        || original.name_range_in(source)?.start
            != at.checked_add(usize::from(tok.content_offset))?
    {
        return None;
    }
    let spelling = std::str::from_utf8(source.get(at..original.next)?).ok()?;
    let Some(name) = tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
    else {
        return Some(spelling.to_owned());
    };
    tcl_syntax::naming::scalar_source_reference_followed_by(name, following, config)
}

/// Rebuild source text from a single token, re-adding delimiters
/// and recursively minifying `[…]` substitutions.
fn reconstruct_raw(
    sm: &SourceMap,
    tok: Token,
    following: &str,
    env: MinifyEnv<'_>,
    in_quotes: bool,
    depth: u32,
) -> Option<String> {
    Some(match tok.kind {
        // Inside a double-quoted word the caller re-wraps the arg in `"…"`, so a
        // `Str` token (e.g. a lone `$` classified as literal) is string data —
        // emit it verbatim, not brace-wrapped as `{$}`.
        TokenType::Str if in_quotes => sm.text(tok.span).to_owned(),
        TokenType::Str => format!("{{{}}}", sm.token_text(tok)),
        TokenType::Cmd => format!(
            "[{}]",
            minify_body(
                sm.token_text(tok),
                env,
                depth + 1,
                tcl_lexer::SourceSegmentationPurpose::NestedScript
            )
        ),
        TokenType::Var => {
            return reconstruct_variable_reference(sm, tok, following, env.config);
        }
        TokenType::Expand => "{*}".to_owned(),
        TokenType::ExprSugar => format!("$({})", sm.token_text(tok)),
        _ => sm.token_text(tok).to_owned(),
    })
}

#[cfg(test)]
fn can_strip_quotes(raw: &str, style: tcl_dialect::BracedVarStyle) -> bool {
    let config = LexerConfig {
        braced_var: style,
        ..LexerConfig::default()
    };
    standalone_alias_word(&format!("\"{raw}\""), config).is_some_and(|word| {
        tcl_syntax::word_rules::lexical_source_word_preserves_components(&word, raw)
    })
}

/// Remove `${…}` references from `raw` so the residual brace check
/// in [`can_strip_quotes`] only sees bare braces.
///
/// Delimited by the document's own `${…}` close rule: under 8.x `${a{b}c}`
/// ends at the first `}`, so `c}` is a *real* residual brace and the quotes
/// must stay; under 9.x the whole thing is one reference and they may go.
/// Scanning by the wrong rule strips quotes that were load-bearing, or
/// keeps ones that were not.
#[cfg(test)]
fn strip_braced_var_refs(raw: &str, style: tcl_dialect::BracedVarStyle) -> String {
    let bytes = raw.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        if bytes[i] == b'$'
            && i + 1 < n
            && bytes[i + 1] == b'{'
            && let tcl_lexer::BracedVarEnd::Closed(close) =
                tcl_lexer::braced_var_name_end(bytes, i + 2, style)
        {
            i = close + 1;
            continue;
        }
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&raw[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Byte length of the UTF-8 char whose lead byte is `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Rebuild the source text of an argument from its tokens.
fn reconstruct_arg(
    sm: &SourceMap,
    arg: &Arg,
    original: Option<&tcl_lexer::NativeWord>,
    env: MinifyEnv<'_>,
    depth: u32,
) -> Option<String> {
    let mut raw = String::new();
    // Every scalar proposal sees all subsequently emitted bytes in its word.
    for &tok in arg.tokens.iter().rev() {
        let component = reconstruct_raw(sm, tok, &raw, env, arg.is_quoted, depth)?;
        raw.insert_str(0, &component);
    }
    if arg.is_quoted
        && !original.is_some_and(|word| {
            tcl_syntax::word_rules::lexical_source_word_preserves_components(word, &raw)
        })
    {
        Some(format!("\"{raw}\""))
    } else {
        Some(raw)
    }
}

// case-list (clause list) handling

/// One element of a braced clause list, with enough shape to re-emit
/// it exactly as written.
struct CaseElement {
    /// Interior byte range in the list content (delimiters stripped).
    value: std::ops::Range<usize>,
    /// Written `{…}`-braced.
    braced: bool,
    /// Written `"…"`-quoted.
    quoted: bool,
}

/// Split the content of a braced clause list into elements with the
/// central Tcl **list** grammar ([`tcl_syntax::list::find_element`]).
/// Returns `None` when the content is not a well-formed list — the
/// caller must then leave the original text untouched.
fn case_list_elements(inner: &str) -> Option<Vec<CaseElement>> {
    let bytes = inner.as_bytes();
    let mut out = Vec::new();
    let mut scan = 0usize;
    loop {
        match tcl_syntax::list::find_element(inner, scan) {
            Ok(Some(el)) => {
                let quoted = !el.braced
                    && el.value.start > 0
                    && bytes.get(el.value.start - 1) == Some(&b'"');
                let next = el.next;
                out.push(CaseElement {
                    value: el.value,
                    braced: el.braced,
                    quoted,
                });
                if next <= scan {
                    break;
                }
                scan = next;
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    Some(out)
}

/// Re-emit a clause-list element exactly as written (original
/// delimiters and interior spelling preserved).
fn case_element_text<'s>(inner: &'s str, el: &CaseElement) -> &'s str {
    if el.braced || el.quoted {
        // `value` strips the delimiters; the closer sits at `value.end`.
        &inner[(el.value.start - 1)..=el.value.end]
    } else {
        &inner[el.value.clone()]
    }
}

/// Minify the content of a `case_list` command's braced clause list.
///
/// A braced case list is a Tcl **list**, not a script: `#` and `;`
/// are ordinary pattern characters there (C Tcl's
/// `TclNRSwitchObjCmd` splits it with `TclListObjGetElements`,
/// `generic/tclCmdMZ.c`), so the content is decoded with the central
/// list grammar, never the script lexer: a script lexer drops a valid `#`
/// pattern and its body as a "comment" (tclsh 9.0.4: `switch # { # {puts
/// matched} default {puts default} }` prints `matched`).
///
/// Only `{…}`-braced **body** elements are recursively minified (their
/// content is literal script text); every flag, pattern, fall-through
/// `-`, and non-braced body is re-emitted exactly as written.  The
/// original text is preserved untouched when the content is not a
/// well-formed list or when any clause is missing its body (Tcl
/// errors on the odd-length list and the error message quotes the
/// original).
fn minify_case_list(
    inner: &str,
    cl: &tcl_registry::CaseListSpec,
    env: MinifyEnv<'_>,
    depth: u32,
    bodies: &[tcl_compiler::registry_invocation::OriginalSourceScriptBody],
) -> String {
    let Some(elements) = case_list_elements(inner) else {
        return inner.to_owned();
    };
    if elements.is_empty() {
        return String::new();
    }

    // Walk clauses with the shared registry-derived shape (clause flags may
    // precede a pattern — Expect's `-re` / `-timeout 5`).  In particular,
    // this resolves the same unique abbreviations as the compiler, folding,
    // and semantic-token walkers.
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: cl.clause_flags,
        clause_value_flags: cl.clause_value_flags,
    };

    let mut parts: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < elements.len() {
        let mut options_ended = false;
        // Leading clause flags (and their value words).
        while !shape.clause_flags.is_empty()
            && i < elements.len()
            && !options_ended
            && !elements[i].braced
        {
            let flag = &elements[i];
            let word = &inner[flag.value.clone()];
            if !word.starts_with('-') {
                break;
            }
            let Some(canonical_flag) = shape.resolve_flag(word) else {
                // An option-shaped word that is neither canonical nor a
                // unique abbreviation is an Expect error.  Retain the
                // source rather than accidentally reclassifying its words.
                return inner.to_owned();
            };
            parts.push(case_element_text(inner, flag).to_owned());
            i += 1;
            // Read off `cl`'s own descriptor, like `CaseListSpec::inline_clauses`
            // does for the compiler — a future case-list command's own
            // end-of-clause-options flag (if it even has one) is whatever its
            // pack declares, not necessarily "--".
            if cl.clause_end_options_flag == Some(canonical_flag) {
                options_ended = true;
            } else if shape.flag_takes_value(canonical_flag) && i < elements.len() {
                parts.push(case_element_text(inner, &elements[i]).to_owned());
                i += 1;
            }
        }
        // Pattern + body.  A trailing pattern with no body is a malformed
        // list ("extra switch pattern with no body") — preserve the
        // original so the runtime error (and its quoted text) is
        // unchanged.
        if i >= elements.len() {
            break;
        }
        if i + 1 >= elements.len() {
            return inner.to_owned();
        }
        let (pattern, body) = (&elements[i], &elements[i + 1]);
        parts.push(case_element_text(inner, pattern).to_owned());
        let body_src = &inner[body.value.clone()];
        if body.braced
            && env
                .original_region(body_src)
                .is_some_and(|span| bodies.iter().any(|body| body.content_span() == span))
        {
            let minified = minify_body(
                body_src,
                env,
                depth,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            );
            parts.push(format!("{{{minified}}}"));
        } else {
            parts.push(case_element_text(inner, body).to_owned());
        }
        i += 2;
    }
    parts.join(" ")
}

// expr whitespace compression

/// One token of an `expr` body for whitespace compression.
enum ExprTok {
    /// A `[…]` command substitution (already minified).
    Cmd(String),
    /// Any other token (string, var, word, operator, punctuation).
    Other(String),
    /// A run of whitespace.
    Space,
}

/// Remove expression gaps only when the complete configured token sequence
/// remains unchanged. Required word boundaries and literal contents survive;
/// this step performs no AST shrinking.
fn strip_expr_whitespace(text: &str, env: MinifyEnv<'_>, depth: u32) -> String {
    strip_expr_whitespace_with_script_compaction(text, env, depth, true)
}

fn strip_expr_whitespace_with_script_compaction(
    text: &str,
    env: MinifyEnv<'_>,
    depth: u32,
    compact_scripts: bool,
) -> String {
    let mut syntax = tcl_syntax::expr::parser::ExprParseContext::for_profile(env.dialect);
    syntax.lexer_grammar = env.config.grammar_over(syntax.lexer_grammar);
    if matches!(
        tcl_syntax::expr::parser::parse_expr_with_syntax_context(text, &syntax),
        ExprNode::Raw { .. }
    ) {
        return text.to_owned();
    }
    let tokens = if compact_scripts {
        tokenise_expr(text, env, depth)
    } else {
        tokenise_expr_with_script_compaction(text, env, depth, false)
    };
    let rendered = tokens
        .iter()
        .filter_map(|token| match token {
            ExprTok::Space => None,
            ExprTok::Cmd(value) | ExprTok::Other(value) => Some(value.as_str()),
        })
        .collect::<Vec<_>>();
    let Some(first) = rendered.first() else {
        return text.to_owned();
    };
    let baseline = rendered.join(" ");
    let Some(expected) = expression_token_signature(&baseline, env, false) else {
        return text.to_owned();
    };
    if expression_token_signature(text, env, true)
        != expression_token_signature(&baseline, env, true)
    {
        return text.to_owned();
    }
    let mut output = (*first).to_owned();
    for pair in rendered.windows(2) {
        let (previous, current) = (pair[0], pair[1]);
        let joined = format!("{previous}{current}");
        let spaced = format!("{previous} {current}");
        let preserves = expression_token_signature(&joined, env, false)
            .zip(expression_token_signature(&spaced, env, false))
            .is_some_and(|(joined, spaced)| joined == spaced);
        if !preserves {
            output.push(' ');
        }
        output.push_str(current);
    }
    if expression_token_signature(&output, env, false) == Some(expected) {
        output
    } else {
        baseline
    }
}

fn configured_expression_tokens(
    text: &str,
    env: MinifyEnv<'_>,
) -> Option<Vec<tcl_lexer::ExprToken<Vec<u8>>>> {
    let (tokens, unknown) = tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
        text.as_bytes(),
        &env.config.grammar_over(env.dialect.grammar),
        env.dialect.expr_grammar_base,
        env.dialect.f5_core_expr_grammar(),
    );
    if unknown {
        return None;
    }
    tcl_lexer::expression_terms(text.as_bytes(), &tokens, env.config)?;
    Some(tokens)
}

/// Ordered lexical tokens only; rewritten child scripts retain their own
/// source purpose and AST transformations keep their independent contract.
fn expression_token_signature(
    text: &str,
    env: MinifyEnv<'_>,
    omit_command_text: bool,
) -> Option<Vec<(tcl_lexer::ExprTokenType, Vec<u8>)>> {
    Some(
        configured_expression_tokens(text, env)?
            .into_iter()
            .filter(|token| !token.kind.is_skipped() && token.kind != tcl_lexer::ExprTokenType::Eof)
            .map(|token| {
                let text = if omit_command_text && token.kind == tcl_lexer::ExprTokenType::Command {
                    Vec::new()
                } else {
                    token.text
                };
                (token.kind, text)
            })
            .collect(),
    )
}

/// Compress and shrink an `expr` body: strip whitespace, then try
/// AST transforms (De Morgan / comparison inversion / double
/// negation) and keep whichever is shorter.
fn compress_expr(text: &str, env: MinifyEnv<'_>, depth: u32) -> String {
    let compressed = strip_expr_whitespace(text, env, depth);
    let shrunk = shrink_expr_ast(&compressed, env, depth);
    if shrunk.len() < compressed.len() {
        shrunk
    } else {
        compressed
    }
}

/// AST-based expression shrinking.
fn shrink_expr_ast(text: &str, env: MinifyEnv<'_>, depth: u32) -> String {
    let node = parse_expr_for_profile(text, Some(env.dialect));
    if matches!(node, ExprNode::Raw { .. }) {
        return text.to_owned();
    }
    let shrunk = shrink_node(&node);
    if shrunk == node {
        return text.to_owned();
    }
    let rendered = render_expr(&shrunk);
    strip_expr_whitespace(&rendered, env, depth)
}

/// The logical complement of a comparison / membership operator the minifier
/// may safely substitute, or `None` when there is none it can use.
///
/// The table itself is [`BinOp::inverse`] (`tcl_syntax::expr::operators`), so
/// the minifier cannot disagree with the shared evaluator about those 14 rows.
///
/// The four *ordered numeric* rows are refused outright. Their identity holds
/// only when neither operand can be NaN (`expr {!(NaN < 1)}` is 1 while
/// `expr {NaN >= 1}` is 0 — [`BinOp::inverse_needs_non_nan`]), and the minifier
/// rewrites source text with no type information whatsoever, so it can never
/// discharge that precondition for an operand like `$x`. `==`/`!=` are exact
/// complements even for NaN, and the string / membership operators never
/// compare numerically, so those rows stay available.
fn comparison_inversion(op: BinOp) -> Option<BinOp> {
    if op.inverse_needs_non_nan() {
        return None;
    }
    op.inverse()
}

/// Build a `!operand` node.
fn negate(operand: ExprNode) -> ExprNode {
    ExprNode::Unary {
        op: UnaryOp::Not,
        operand: Box::new(operand),
    }
}

/// Pick `candidate` over `original` when its rendering is shorter.
fn pick_shorter(candidate: ExprNode, original: &ExprNode) -> ExprNode {
    if render_expr(&candidate).len() < render_expr(original).len() {
        candidate
    } else {
        original.clone()
    }
}

/// Recursively try size-reducing transforms on an expression node.
fn shrink_node(node: &ExprNode) -> ExprNode {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => shrink_not(node, operand),
        ExprNode::Binary { op, left, right }
            if matches!(op, BinOp::Or | BinOp::WordOr) && both_negations(left, right) =>
        {
            // De Morgan reverse: !a || !b → !(a && b) (if shorter).
            let (a, b) = (unwrap_not(left), unwrap_not(right));
            let dual = if *op == BinOp::Or {
                BinOp::And
            } else {
                BinOp::WordAnd
            };
            let combined = negate(ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(a)),
                right: Box::new(shrink_node(b)),
            });
            pick_shorter(combined, node)
        }
        ExprNode::Binary { op, left, right }
            if matches!(op, BinOp::And | BinOp::WordAnd) && both_negations(left, right) =>
        {
            // De Morgan reverse: !a && !b → !(a || b) (if shorter).
            let (a, b) = (unwrap_not(left), unwrap_not(right));
            let dual = if *op == BinOp::And {
                BinOp::Or
            } else {
                BinOp::WordOr
            };
            let combined = negate(ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(a)),
                right: Box::new(shrink_node(b)),
            });
            pick_shorter(combined, node)
        }
        ExprNode::Binary { op, left, right } => {
            let new_left = shrink_node(left);
            let new_right = shrink_node(right);
            ExprNode::Binary {
                op: *op,
                left: Box::new(new_left),
                right: Box::new(new_right),
            }
        }
        ExprNode::Unary { op, operand } => ExprNode::Unary {
            op: *op,
            operand: Box::new(shrink_node(operand)),
        },
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ExprNode::Ternary {
            condition: Box::new(shrink_node(condition)),
            true_branch: Box::new(shrink_node(true_branch)),
            false_branch: Box::new(shrink_node(false_branch)),
        },
        other => other.clone(),
    }
}

/// Whether both operands are `!`-negations.
fn both_negations(left: &ExprNode, right: &ExprNode) -> bool {
    matches!(
        left,
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    ) && matches!(
        right,
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    )
}

/// The operand of a `!`-negation (caller guarantees the shape).
fn unwrap_not(node: &ExprNode) -> &ExprNode {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => operand,
        _ => node,
    }
}

/// Handle the `!`-prefixed shrink cases (double negation,
/// comparison inversion, De Morgan forward), falling back to a
/// generic operand recurse.
fn shrink_not(node: &ExprNode, operand: &ExprNode) -> ExprNode {
    // NB: `!!x → x` is deliberately NOT folded. It is only sound when the whole
    // expression is consumed as a boolean (`if {!!$x}` ≡ `if {$x}`); in a value
    // context or as a subexpression operand it changes the result, since `!!x`
    // yields the 0/1 boolean coercion while `x` yields x's value (tclsh:
    // `expr {!!5}` → 1, `expr {5}` → 5). The minifier processes Expr-role
    // arguments without knowing whether the result is consumed as a boolean, so
    // the fold is unsafe here. The De Morgan rewrite below preserves the 0/1
    // result for any operand values and remains unconditional; the
    // comparison-inversion rewrite preserves it only for the operators
    // `comparison_inversion` still offers — the ordered numeric four (`<`,
    // `<=`, `>`, `>=`) are excluded there because their identity fails on a
    // NaN operand and the minifier cannot rule one out.
    if let ExprNode::Binary { op, left, right } = operand {
        // Comparison inversion: !($a == $b) → $a != $b.
        if let Some(inv) = comparison_inversion(*op) {
            let inverted = ExprNode::Binary {
                op: inv,
                left: Box::new(shrink_node(left)),
                right: Box::new(shrink_node(right)),
            };
            return pick_shorter(inverted, node);
        }
        // De Morgan forward.
        if matches!(op, BinOp::And | BinOp::WordAnd | BinOp::Or | BinOp::WordOr) {
            let neg_l = negate(shrink_node(left));
            let neg_r = negate(shrink_node(right));
            let dual = match op {
                BinOp::And => BinOp::Or,
                BinOp::WordAnd => BinOp::WordOr,
                BinOp::Or => BinOp::And,
                _ => BinOp::WordAnd,
            };
            let demorgan = ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(&neg_l)),
                right: Box::new(shrink_node(&neg_r)),
            };
            return pick_shorter(demorgan, node);
        }
    }
    // Generic recurse into the operand.
    negate(shrink_node(operand))
}

/// Project the selected shared expression tokens and original command terms.
/// Unknown syntax keeps the entire original expression, including its gaps.
fn tokenise_expr(text: &str, env: MinifyEnv<'_>, nest_depth: u32) -> Vec<ExprTok> {
    tokenise_expr_with_script_compaction(text, env, nest_depth, true)
}

fn tokenise_expr_with_script_compaction(
    text: &str,
    env: MinifyEnv<'_>,
    nest_depth: u32,
    compact_scripts: bool,
) -> Vec<ExprTok> {
    let project = || -> Option<Vec<ExprTok>> {
        let tokens = configured_expression_tokens(text, env)?;
        let terms = tcl_lexer::expression_terms(text.as_bytes(), &tokens, env.config)?;
        let mut output = Vec::new();
        for token in tokens {
            if token.kind == tcl_lexer::ExprTokenType::Eof {
                continue;
            }
            if token.kind.is_skipped() {
                output.push(ExprTok::Space);
                continue;
            }
            let end = token.end.checked_add(1)?;
            let original = text.get(token.start as usize..end as usize)?;
            if original.as_bytes() != token.text {
                return None;
            }
            if compact_scripts && token.kind == tcl_lexer::ExprTokenType::Command {
                let term = terms.iter().find(|term| {
                    term.kind == tcl_lexer::ExprTermKind::Command
                        && term.source == (token.start..end)
                })?;
                let body = text.get(term.value.start as usize..term.value.end as usize)?;
                output.push(ExprTok::Cmd(format!(
                    "[{}]",
                    minify_body(
                        body,
                        env,
                        nest_depth + 1,
                        tcl_lexer::SourceSegmentationPurpose::NestedScript
                    )
                )));
            } else {
                output.push(ExprTok::Other(original.to_owned()));
            }
        }
        Some(output)
    };
    project().unwrap_or_else(|| vec![ExprTok::Other(text.to_owned())])
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod constructed_compatibility_tests;

#[cfg(test)]
mod selected_scalar_reference_emission_tests;

#[cfg(test)]
mod symbol_report_tests;

#[cfg(test)]
mod original_formal_name_geometry_tests;

#[cfg(test)]
mod authored_compact_candidate_tests;

#[cfg(test)]
mod original_alias_phase_tests;
