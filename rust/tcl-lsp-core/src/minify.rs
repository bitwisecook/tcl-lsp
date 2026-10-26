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
    if parent_label == "::" {
        format!("::{child_name}")
    } else {
        format!("{parent_label}::{child_name}")
    }
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
        let param_names: FxHashSet<&str> = proc_def
            .map(|pd| pd.params.iter().map(|p| p.name.as_str()).collect())
            .unwrap_or_default();
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
                && (var_name == "args"
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
mod tests {
    use super::*;

    #[test]
    fn original_minifier_body_parsing_keeps_bom_content_and_complete_parent_config() {
        // Original source geometry only; this does not grant body execution.
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..LexerConfig::for_file_grammar(profile.grammar)
        };
        let input =
            tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        for source in [
            "proc p {} {\u{feff}puts    1}",
            "set x [\u{feff}puts    1]",
            "apply {{} {\u{feff}puts    1}}",
            "switch x {x {\u{feff}puts    1}}",
        ] {
            let analysis = original::analysis_with_input(source, &input);
            let result = lexical_default(source, &analysis).unwrap();
            assert_eq!(
                result.matches('\u{feff}').count(),
                1,
                "{source:?}: {result:?}"
            );
            assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
            assert!(
                analysis.matches_original_source_image(
                    &tcl_lexer::SourceImage::document(source),
                    config
                )
            );
        }
        let source = "\u{feff}puts    1";
        let analysis = original::analysis_with_input(source, &input);
        assert_eq!(lexical_default(source, &analysis).unwrap(), "puts 1");
    }

    fn min(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &registry,
        )
    }

    /// A `${…}` whose name is not a bare `$name` word may never shed its
    /// braces, whatever follows it: `${a{b}c}` reads the variable `a{b}c`,
    /// while `$a{b}c` reads `a` and appends the literal `{b}c`.
    ///
    /// Found while pinning the plumb above — minify dropped the braces
    /// because it only asked whether the *next* token would extend the
    /// reference, never whether the name itself could be written bare.
    #[test]
    fn a_non_bare_braced_name_keeps_its_braces() {
        let registry = CommandRegistry::build_default();
        for release in ["tcl9.0", "tcl8.6"] {
            let out = minify_tcl(
                "set {a b} 1\nputs ${a b}\n",
                tcl_registry::model::ingress::resolve_environment(release).analyser_profile(),
                &registry,
            );
            assert!(
                out.contains("${a b}"),
                "a name with a space must stay braced on {release}: {out}"
            );
        }
        // A plain name still sheds them.
        let out = minify_tcl(
            "set plain 1\nputs ${plain}\n",
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        );
        assert!(out.contains("$plain"), "{out}");
        assert!(!out.contains("${plain}"), "{out}");
    }

    #[test]
    fn expression_commands_keep_configured_literal_closers_and_unavailable_extents() {
        for label in ["tcl8.6", "tcl9.1"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(label).analyser_profile();
            for expression in [
                "[list {literal ] value}] + 1",
                "[list \"literal ] value\"] + 1",
                "[list [list {literal ] value}]] + 1",
                "[\n# literal ]\nlist {literal [ value}\n] + 1",
            ] {
                let source = format!("expr {{{expression}}}");
                let output = min_dialect(&source, profile);
                let data = if expression.contains("literal [ value") {
                    "literal [ value"
                } else {
                    "literal ] value"
                };
                assert!(output.contains(data), "{label}: {output}");
                let opener = output.find('[').unwrap();
                let end = tcl_lexer::command_subst_close(
                    output.as_bytes(),
                    opener,
                    tcl_lexer::SubstFlags::default(),
                    LexerConfig::for_file_grammar(profile.grammar),
                )
                .unwrap();
                assert_eq!(&output[end..], "+1}", "{label}: {output}");
            }
            let source = "expr {[é + 1}";
            let registry = CommandRegistry::build_default();
            let analysis = original::analysis_for_profile(source, profile, &registry);
            let input = analysis.resolved_input.as_ref().unwrap();
            let context = input.context_registry();
            let env = MinifyEnv {
                original_source: source,
                analysis: &analysis,
                input,
                dialect: input.unit_profile(),
                config: input.lexer_config(),
                registry: context.commands(),
                context: &context,
                identities: analysis.retained_command_realm().unwrap(),
            };
            let expression = "[é + 1";
            let tokens = tokenise_expr(expression, env, 0);
            assert!(matches!(tokens.as_slice(), [ExprTok::Other(value)] if value == expression));
            assert_eq!(strip_expr_whitespace(expression, env, 0), expression);
        }
    }

    #[test]
    fn expression_minification_preserves_original_array_index_name_data() {
        for label in ["tcl8.6", "tcl9.1"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(label).analyser_profile();
            for reference in ["$arr(a + b)", "$arr($key + 1)", "${$literal}", "${}"] {
                let source = format!("expr {{{reference} + 1}}");
                let output = min_dialect(&source, profile);
                assert!(output.contains(reference), "{label}: {output}");
            }
        }
    }

    /// The **plumb**: minify must take the `${…}` rule from the document's
    /// own dialect, not a constant. A quoted `"${a{b}c}"` may shed its
    /// quotes on 9.x (the whole word is one variable reference) but not on
    /// 8.x, where the reference stops at the first `}` and the residual `c}`
    /// is a bare brace.
    #[test]
    fn quote_stripping_follows_the_documents_dialect() {
        let registry = CommandRegistry::build_default();
        let src = "puts \"${a{b}c}\"\n";
        let nine = minify_tcl(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
        );
        let eight = minify_tcl(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &registry,
        );
        assert!(
            !nine.contains('"'),
            "9.x reads one reference, so the quotes may go: {nine}"
        );
        assert!(
            eight.contains('"'),
            "8.x leaves a bare `c}}`, so the quotes must stay: {eight}"
        );
    }

    /// Minify decides whether a quoted word may drop its
    /// quotes by checking for braces left over after `${…}` references are
    /// removed, so the removal must use the document's own `${…}` close
    /// rule.
    ///
    /// Oracle: `puts ${a{b}c}` prints `NINE` on tclsh 9.0.4 (the whole thing
    /// is one reference, so nothing brace-like remains) and `EIGHTc}` on
    /// 8.6.16 (the reference ends at the first `}` and the residual `c}`
    /// really is a bare brace).
    #[test]
    fn braced_var_stripping_follows_the_documents_release() {
        use tcl_dialect::BracedVarStyle;
        // 9.x: one reference, nothing left behind.
        assert_eq!(
            strip_braced_var_refs("${a{b}c}", BracedVarStyle::Tcl9Nesting),
            ""
        );
        // 8.x: the reference is `${a{b}`, so `c}` survives as a bare brace.
        assert_eq!(
            strip_braced_var_refs("${a{b}c}", BracedVarStyle::FirstClose),
            "c}"
        );
        // …which is exactly what decides whether the quotes may go.
        assert!(can_strip_quotes("${a{b}c}", BracedVarStyle::Tcl9Nesting));
        assert!(!can_strip_quotes("${a{b}c}", BracedVarStyle::FirstClose));
    }

    #[test]
    fn quote_stripping_uses_complete_configured_components() {
        let registry = CommandRegistry::build_default();
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        for source in [
            "puts \"[list {a b}]\"",
            "puts \"[list \"a b\"]\"",
            "puts \"${x}tail\"",
            "puts \"${x}(key)\"",
            "puts \"$array($index)\"",
            "puts \"a\\ b\"",
        ] {
            let analysis = original::analysis_for_profile(source, profile, &registry);
            let config = analysis.body_lexer_config.unwrap();
            let output = lexical_default(source, &analysis).unwrap();
            let original = lexical_script_words(source, config).unwrap();
            let proposed = output.strip_prefix("puts ").unwrap();
            if source == "puts \"[list {a b}]\"" {
                assert_eq!(output, "puts [list {a b}]");
            }
            assert!(
                tcl_syntax::word_rules::lexical_source_word_preserves_components(
                    &original.commands[0].words[1],
                    proposed,
                ),
                "{source} -> {output}"
            );
        }
    }

    /// The `$var` reader behind constant folding and the live-name sweep
    /// splits the same bytes at the same release-dependent point.
    #[test]
    fn parse_var_ref_reads_braced_names_by_the_documents_release() {
        use tcl_dialect::BracedVarStyle;
        let text = "${a{b}c}";
        assert_eq!(
            parse_var_ref(text, 0, BracedVarStyle::Tcl9Nesting),
            (8, Some("a{b}c"))
        );
        assert_eq!(
            parse_var_ref(text, 0, BracedVarStyle::FirstClose),
            (6, Some("a{b"))
        );
        // An unterminated `${` yields no name under either rule.
        assert_eq!(
            parse_var_ref("${abc", 0, BracedVarStyle::Tcl9Nesting),
            (1, None)
        );
    }

    /// `minify_body`'s recursive
    /// descent (shared with `render_command`, `reconstruct_arg`/
    /// `reconstruct_raw`, `minify_switch_case_list`, `minify_lambda_literal`,
    /// `compress_expr`/`tokenise_expr`) is capped at `MAX_MINIFY_DEPTH`
    /// (128), mirroring `formatting::engine`'s `MAX_FORMAT_DEPTH` and the
    /// same empirical crash-range reasoning documented there (this
    /// recursion's per-level shape is close enough not to warrant a
    /// separate calibration run). 2000 is comfortably past the cap; the
    /// assertion is that minifying returns at all, not what it returns.
    #[test]
    fn deeply_nested_if_survives_minify() {
        const DEPTH: usize = 2000;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        let _ = min(&src);
    }

    /// A moderately nested body (well under `MAX_MINIFY_DEPTH`) still
    /// minifies normally — the safety net must not fire on realistic
    /// nesting depths.
    #[test]
    fn moderately_nested_if_still_minifies() {
        let out = min("if {1} {\nif {2} {\nset x 1\n}\n}\n");
        assert!(out.contains("set x 1"), "{out:?}");
    }

    #[test]
    fn bare_dollar_in_quoted_string_not_brace_wrapped() {
        // Minifying a lone `$` inside a double-quoted string
        // must keep it literal, not emit `{$}`.
        let out = min("puts \"cost: $\"\n");
        assert!(out.contains("\"cost: $\""), "{out:?}");
        assert!(!out.contains("{$}"), "bare $ was brace-wrapped: {out:?}");
    }

    fn check(input: &str, expected: &str) {
        let got = min(input);
        assert_eq!(
            got, expected,
            "\ninput:    {input:?}\ngot:      {got:?}\nexpected: {expected:?}"
        );
    }

    #[test]
    fn strips_comments() {
        check("# a comment\nputs hi\n", "puts hi");
    }

    #[test]
    fn keeps_braces_when_next_token_extends_bare_word() {
        // A bare word `${a}jumps` must keep its braces — dropping
        // them yields `$ajumps`, reading a different variable.
        let out = min("set a 1\nputs ${a}jumps\n");
        assert!(
            out.contains("${a}jumps"),
            "bare-word ${{a}}jumps was unbraced unsafely:\n{out}"
        );
        assert!(
            !out.contains("$ajumps"),
            "unsafe unbrace produced $ajumps:\n{out}"
        );
    }

    #[test]
    fn keeps_braces_before_paren_in_quoted_word() {
        // `"${x}(k)"` must not become `"$x(k)"` (array element).
        let out = min("set x 1\nputs \"${x}(k)\"\n");
        assert!(
            out.contains("${x}(k)"),
            "`(` after ${{x}} must keep braces (array-element hazard):\n{out}"
        );
    }

    #[test]
    fn keeps_braces_before_namespace_separator() {
        // `${x}::y` unbraced reads variable `x::y`, not `$x` + literal `::y`.
        let out = min("set x 1\nputs \"${x}::y\"\n");
        assert!(
            out.contains("${x}::y"),
            "`::` after ${{x}} must keep braces:\n{out}"
        );
    }

    #[test]
    fn still_unbraces_when_safe() {
        // A following non-extending char (`.`) is safe to unbrace.
        let out = min("set a 1\nputs ${a}.txt\n");
        assert!(
            out.contains("$a.txt"),
            "safe unbrace did not happen:\n{out}"
        );
    }

    fn min_dialect(src: &str, dialect: &'static tcl_dialect::DialectProfile) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl(src, dialect, &registry)
    }

    fn min_compact(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl_compact(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            false,
            &registry,
        )
        .0
    }

    fn min_compact_isolated(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl_compact(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            true,
            &registry,
        )
        .0
    }

    #[test]
    fn compact_renames_proc_local_vars_and_params() {
        // Param `name`→`b`, local `message`→`a`.  The PROC name is a public
        // command identity: non-isolated compaction must keep
        // `greet` callable by external code.
        assert_eq!(
            min_compact(
                "proc greet {name} {\n    set message \"hi $name\"\n    return $message\n}\n"
            ),
            "proc greet {b} {set a \"hi $b\";return $a}",
        );
    }

    #[test]
    fn compact_isolated_renames_procs() {
        // Under `isolated` the caller asserts a closed world, so the proc
        // name may be compacted (definition + call sites in lock-step).
        assert_eq!(
            min_compact_isolated("proc greet {name} {\n    return $name\n}\nputs [greet world]\n"),
            "proc a {a} {return $a};puts [a world]",
        );
    }

    #[test]
    fn compact_isolated_keeps_procs_when_reflection_present() {
        // `info procs` reflects proc names (registry
        // REFLECTS_COMMAND_NAMES), so even `isolated` must keep them.
        // tclsh 9.0.4: renaming only the definition+call leaves
        // `info procs longprocedure` returning an empty list — an
        // observable change.
        let out = min_compact_isolated(
            "proc longprocedure {} {return ok}\nputs [info procs longprocedure]\nputs [longprocedure]\n",
        );
        assert!(out.contains("proc longprocedure {}"), "{out}");
        assert!(out.contains("info procs longprocedure"), "{out}");
    }

    #[test]
    fn compact_keeps_locals_when_info_locals_present() {
        // `info locals` reflects local variable names (registry
        // INTROSPECTS_BY_NAME), so the scope must not be renamed.
        // tclsh 9.0.4: the original prints `longvariable`; a compacted
        // `set a 1` would print `a`.
        let out = min_compact("proc f {} {\n    set longvariable 1\n    return [info locals]\n}\n");
        assert_eq!(out, "proc f {} {set longvariable 1;return [info locals]}");
    }

    #[test]
    fn compact_keeps_locals_when_info_exists_present() {
        // `info exists NAME` reads a variable by bare name; renaming the
        // `set` site but not the introspection flips the result 1 → 0.
        let out = min_compact("proc f {} {\n    set myvar 1\n    return [info exists myvar]\n}\n");
        assert_eq!(out, "proc f {} {set myvar 1;return [info exists myvar]}");
    }

    #[test]
    fn compact_renames_repeated_set_sites_in_lock_step() {
        // A re-definition (`set myvar 2`) is a bare-name reference site;
        // it must be renamed together with the declaration and the `$`
        // reads, or one variable silently splits into two and this
        // returns 1 instead of 2 after compaction.
        assert_eq!(
            min_compact("proc f {} {\n    set myvar 1\n    set myvar 2\n    return $myvar\n}\n"),
            "proc f {} {set a 1;set a 2;return $a}",
        );
    }

    #[test]
    fn compact_never_splits_unset_from_its_variable() {
        // `unset myvar` names the variable as a bare argument the
        // analyser does not link as a reference, so `myvar` must be
        // left unrenamed everywhere (renaming only the `set` site
        // would unset a different variable).  Unrelated locals still
        // compact.
        let out = min_compact(
            "proc f {} {\n    set myvar 1\n    unset myvar\n    set other 2\n    return $other\n}\n",
        );
        assert_eq!(out, "proc f {} {set myvar 1;unset myvar;set a 2;return $a}");
    }

    #[test]
    fn compact_upvar_blocks_all_scopes() {
        // `upvar` aliases a caller frame chosen at runtime (registry
        // ALIASES_CALLER_FRAME): any proc could be the caller, so no
        // scope may rename its locals while one exists.  tclsh 9.0.4:
        // renaming `callervar` in `use` breaks `get`'s upvar target.
        let out = min_compact(
            "proc get {} {\n    upvar 1 callervar m\n    return $m\n}\nproc use {} {\n    set callervar 5\n    return [get]\n}\n",
        );
        assert!(out.contains("set callervar 5"), "{out}");
    }

    #[test]
    fn compact_renames_param_name_not_other_defaults() {
        // Renaming param `x` must touch only its NAME, not a
        // literal `x` sitting in another param's default value. Here `{y x}`
        // gives `y` the default string `x`; that `x` must survive the `x`→…
        // rename, otherwise the proc's default silently changes.
        let out =
            min_compact("proc f {{alpha 1} {beta alpha}} {\n    return [list $alpha $beta]\n}\n");
        // The name `alpha` is renamed; `beta`'s default literal `alpha` is
        // left intact (it is the string value `alpha`, not the parameter).
        assert!(out.contains("{a 1}"), "alpha name renamed: {out:?}");
        assert!(out.contains("{b alpha}"), "beta default intact: {out:?}");
        assert!(
            !out.contains("{b a}"),
            "default must not be renamed: {out:?}"
        );
    }

    #[test]
    fn compact_variable_reads_keep_original_braced_names_and_current_source() {
        let source =
            "proc helper {} {set longVariable VALUE; puts ${longVariable}; puts $longVariable}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        let compact = min_compact(source);
        assert!(compact.contains("set a VALUE"), "{compact}");
        assert!(!compact.contains("longVariable"), "{compact}");
        let scope = analysis
            .global_scope
            .children
            .iter()
            .find(|scope| scope.kind == ScopeKind::Proc)
            .unwrap();
        let variable = &scope.variables["longVariable"];
        let geometry =
            crate::source_structure::OriginalVariableReferences::capture(source, &analysis)
                .expect("current source geometry");
        let spans =
            variable_reference_name_spans(source, &geometry, "longVariable", &variable.references)
                .expect("actual configured original reference names");
        assert_eq!(spans.len(), 2);
        assert!(
            spans
                .iter()
                .all(|span| source.get(span.as_range()) == Some("longVariable"))
        );
        assert!(
            variable_reference_name_spans(source, &geometry, "donated", &variable.references)
                .is_none()
        );
        let mut stale = analysis.clone();
        stale.resolved_input = None;
        assert!(
            crate::source_structure::OriginalVariableReferences::capture(source, &stale).is_none()
        );
    }

    #[test]
    fn compact_renames_refs_inside_expr_and_command_subst() {
        // The `$value` ref lives inside `[expr {...}]`; it must be
        // renamed in lock-step with the param declaration (relies on
        // the analyser tracking expr/command-subst references).  Proc
        // names stay — public identities in the non-isolated tier.
        assert_eq!(
            min_compact(
                "proc helper {value} {\n    return [expr {$value * 2}]\n}\nproc main {} {\n    set result [helper 21]\n    puts $result\n}\n"
            ),
            "proc helper {a} {return [expr {$a*2}]};proc main {} {set a [helper 21];puts $a}",
        );
    }

    #[test]
    fn compact_returns_symbol_map() {
        let registry = CommandRegistry::build_default();
        // Non-isolated: variables compact, procs do not.
        let (_, sym) = minify_tcl_compact(
            "proc greet {name} {\n    return $name\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            false,
            &registry,
        );
        assert!(sym.procs.is_empty(), "{:?}", sym.procs);
        assert!(
            sym.variables
                .values()
                .any(|m| m.get("name").map(String::as_str) == Some("a"))
        );
        // Isolated: the proc is renamed and reported.
        let (_, sym) = minify_tcl_compact(
            "proc greet {name} {\n    return $name\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            true,
            &registry,
        );
        assert_eq!(sym.procs.get("greet").map(String::as_str), Some("a"));
    }

    #[test]
    fn compact_isolated_renames_global_vars() {
        let registry = CommandRegistry::build_default();
        let (out, _) = minify_tcl_compact(
            "set globalvar 1\nputs $globalvar\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            true,
            &registry,
        );
        assert_eq!(out, "set a 1;puts $a");
    }

    #[test]
    fn unminify_error_round_trips_via_symbol_map() {
        let registry = CommandRegistry::build_default();
        // Isolated so the proc is renamed too — the non-isolated tier keeps
        // proc names.
        let (_, sym) = minify_tcl_compact(
            "proc greet {name} {\n    return $name\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            true,
            &registry,
        );
        // Procs win the bare-name reverse entry, so `a` -> `greet`.
        assert_eq!(
            unminify_error("can't read \"a\": no such variable", &sym),
            "can't read \"greet\": no such variable",
        );
        assert_eq!(
            unminify_error("invalid command name \"a\"", &sym),
            "invalid command name \"greet\"",
        );
    }

    #[test]
    fn remap_line_references_maps_proportionally() {
        let orig = "proc f {} {\n    set x 1\n    set y 2\n    set z 3\n}\n";
        let mini = "proc f {} {set x 1;set y 2;set z 3}";
        // min_commands = 3; line 2 -> proportional original line 3.
        assert_eq!(
            remap_line_references("error at line 2", mini, orig),
            "error at line 3 (minified line 2)",
        );
    }

    #[test]
    fn remap_line_references_procline_single_pass() {
        let orig = "proc f {} {\n    set x 1\n    set y 2\n    set z 3\n}\n";
        let mini = "proc f {} {set x 1;set y 2;set z 3}";
        // Single pass: line 3 -> 5.
        assert_eq!(
            remap_line_references("(procedure \"f\" line 3)", mini, orig),
            "(procedure \"f\" line 5, minified line 3)",
        );
    }

    #[test]
    fn unminify_error_parse_round_trip() {
        let mut sym = SymbolMap::default();
        sym.procs.insert("greet".to_owned(), "a".to_owned());
        let text = sym.format();
        let parsed = SymbolMap::parse(&text);
        assert_eq!(parsed.procs.get("greet").map(String::as_str), Some("a"));
        assert_eq!(
            unminify_error("calling $a now", &parsed),
            "calling $greet now"
        );
    }

    fn agg(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl_aggressive(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            false,
            &registry,
        )
        .source
    }

    #[test]
    fn original_static_folds_do_not_borrow_erasure_from_logical_source_values() {
        let source = "set value 42\nputs \"n=$value\"\n";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let registry = CommandRegistry::build_default();
        let analysis = original::analysis_for_profile(source, profile, &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        assert!(analysis.allows_retained_logical_declaration_advice());
        let (out, count, map) = fold_original_static_substrings(source, input);
        assert_eq!(out, source);
        assert_eq!(count, 0);
        assert!(map.is_empty());
    }

    #[test]
    fn live_name_inventory_uses_the_actual_complete_reference_grammar() {
        let source = "# ${a{b}c} $a::value $a(index) $Ω\n";
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = CommandRegistry::build_default();
        let analysis = original::analysis_for_profile(source, profile, &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        let names = collect_live_names(source, input);
        for name in ["a{b}c", "a::value", "a", "Ω"] {
            assert!(names.names.contains(name), "{name}: {names:?}");
        }
    }

    #[test]
    fn guarded_body_keeps_collision_candidates_but_not_absence_authority() {
        let body = format!(
            "# $retained_candidate {}\nmystery {{set a 1}}\n",
            "x".repeat(270_000)
        );
        let command = "repeatedlyLongCommandName";
        let source = format!("proc p {{}} {{{body}}}\n{command} x;{command} x;{command} x\n");
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = CommandRegistry::build_default();
        let analysis = original::analysis_for_profile(&source, profile, &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        let names = collect_live_names(&source, input);
        assert!(names.names.contains("retained_candidate"));
        assert!(
            names
                .gaps
                .contains(&LiveNameInventoryGap::ComplexityGuardedBody)
        );
        assert!(
            names
                .gaps
                .contains(&LiveNameInventoryGap::SourceSyntaxCoverage)
        );
        // These debts remain explicit, while this patch leaves the existing
        // aggressive authoring policy and its alias output unchanged.
        let result = lexical_aggressive(&source, input, false, false).unwrap();
        assert!(result.source.contains(command));
    }

    #[test]
    fn conservative_collision_candidates_keep_configured_reads_and_insertion_separate() {
        let source = "# $Ω $café ${braced name}\nfoo --long-argument;foo --long-argument\n";
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = CommandRegistry::build_default();
        let analysis = original::analysis_for_profile(source, profile, &registry);
        let input = analysis.resolved_input.as_ref().unwrap();
        let names = collect_live_names(source, input);
        for name in ["Ω", "caf", "café", "braced name"] {
            assert!(names.names.contains(name), "{name}: {names:?}");
        }
        // Native Tcl keeps the non-ASCII suffix outside its bare reference.
        let unicode = source.find("$Ω").unwrap();
        assert!(
            tcl_lexer::scan_var_ref(source.as_bytes(), unicode, input.lexer_config())
                .unwrap()
                .is_none()
        );
        let mixed = source.find("$café").unwrap();
        assert_eq!(
            tcl_lexer::scan_var_ref(source.as_bytes(), mixed, input.lexer_config())
                .unwrap()
                .unwrap()
                .name,
            b"caf"
        );
        let result = minify_with_analysis(
            source,
            &analysis,
            MinifyOptions {
                tier: MinifyTier::Aggressive,
                isolated: true,
                abbreviations: true,
            },
        );
        assert!(
            result
                .refusals
                .contains(&MinifyRefusal::MissingAliasInsertionContract)
        );
        assert!(result.symbol_map.command_aliases.is_empty());
        assert!(result.symbol_map.argument_aliases.is_empty());
        assert_eq!(result.source, "foo --long-argument;foo --long-argument");
    }

    #[test]
    fn static_fold_folds_sccp_constant_interpolation() {
        let registry = CommandRegistry::build_default();
        // `$x` is a proven integer constant; fold `"n=$x"` -> `n=5`.
        let (out, count, map) = fold_static_substrings(
            "set x 5\nputs \"n=$x\"\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &registry,
        );
        assert_eq!(out, "set x 5\nputs n=5\n");
        assert_eq!(count, 1);
        assert!(map.values().any(|v| v == "n=5"), "{map:?}");
    }

    #[test]
    fn static_fold_quotes_carriage_return_value() {
        let registry = CommandRegistry::build_default();
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let (out, count, _) =
            fold_static_substrings("set x \"a\\rb\"\nputs \"v=$x\"\n", profile, &registry);
        assert_eq!(count, 1, "{out}");
        let word = out
            .split_once("\nputs ")
            .unwrap()
            .1
            .strip_suffix('\n')
            .unwrap();
        assert!(
            tcl_syntax::word_rules::lexical_unicode_source_word_matches(
                word,
                "v=a\rb",
                LexerConfig::for_profile(Some(profile)),
            ),
            "{out}"
        );
    }

    /// The quoted word embeds a command substitution whose own
    /// argument is quoted. The shared close-quote scanner skips the whole
    /// `[…]`, so no edit is ever anchored on the inner `"b"` — a scanner
    /// stopping there would splice a replacement over `"a[string toupper "`
    /// and leave `b"] $x"` dangling.
    #[test]
    fn static_fold_keeps_a_quoted_command_substitution_well_formed() {
        let registry = CommandRegistry::build_default();
        let src = "set x 5\nputs \"a[string toupper \"b\"] $x\"\n";
        let (out, count, _) = fold_static_substrings(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &registry,
        );
        assert_eq!(count, 0, "a command substitution is not SCCP-foldable");
        assert_eq!(out, src, "source must come back untouched, not truncated");
    }

    /// The multiline shape: the `]` sitting in a command-position comment
    /// inside the substitution is inert in C Tcl (verified against tclsh
    /// 8.6/9.0), so the quoted word runs to the final `"`.  A scanner that
    /// stopped at that `]` would report the quote opening the inner `"b"` as
    /// the closer, and any fold edge anchored there would splice over the
    /// middle of valid source.
    #[test]
    fn static_fold_keeps_a_commented_command_substitution_well_formed() {
        let registry = CommandRegistry::build_default();
        let src = "set x 5\nputs \"a[\n# ] comment\nset y \"b\"\n]c $x\"\n";
        let (out, count, _) = fold_static_substrings(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &registry,
        );
        assert_eq!(count, 0, "a command substitution is not SCCP-foldable");
        assert_eq!(out, src, "source must come back untouched, not truncated");
    }

    #[test]
    fn static_fold_skips_non_constant() {
        let registry = CommandRegistry::build_default();
        // `[HTTP::uri]` is dynamic — nothing folds.
        let (out, count, _) = fold_static_substrings(
            "set u [HTTP::uri]\nputs \"got $u\"\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &registry,
        );
        assert_eq!(count, 0);
        assert!(out.contains("$u"));
    }

    #[test]
    fn aggressive_aliases_repeated_commands() {
        assert_eq!(
            agg("mylongcommand a\nmylongcommand b\nmylongcommand c\n"),
            "set a mylongcommand;$a a;$a b;$a c",
        );
    }

    #[test]
    fn aggressive_aliases_repeated_arguments() {
        assert_eq!(
            agg("foo --somelongflag\nbar --somelongflag\nbaz --somelongflag\n"),
            "set a --somelongflag;foo $a;bar $a;baz $a",
        );
    }

    #[test]
    fn aggressive_aliases_string_substrings() {
        assert_eq!(
            agg("puts \"the quick brown fox jumps\"\nputs \"the quick brown fox runs\"\n"),
            "set a {the quick brown fox };puts ${a}jumps;puts ${a}runs",
        );
    }

    #[test]
    fn string_alias_reference_preserves_original_six_provider_suffix_values() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
            let config = LexerConfig::for_profile(Some(profile));
            for (suffix, braced) in [
                ("::tail", true),
                (":tail", false),
                ("x", true),
                ("(k)", true),
                ("é", dialect == "jim"),
            ] {
                let source = format!("ALIAS{suffix}");
                let fragment = StringLiteralFragment {
                    span: Span::new(0, u32::try_from(source.len()).unwrap()),
                    text: source.clone(),
                    config,
                    following_end: source.len(),
                };
                let expected = if braced { "${a}" } else { "$a" };
                assert_eq!(
                    alias_string_reference(&source, &fragment, 0, 5, "a", "ALIAS").as_deref(),
                    Some(expected),
                    "{dialect}: {suffix}",
                );
            }
        }
    }

    #[test]
    fn string_alias_reference_rejects_escaped_insertion_and_split_unicode() {
        let config = LexerConfig::for_profile(Some(tcl_dialect::DialectProfile::plain_tcl()));
        for (source, start, end, value) in [(r"\ALIAS!", 1, 6, "ALIAS"), ("éALIAS!", 1, 7, "ALIAS")]
        {
            let fragment = StringLiteralFragment {
                span: Span::new(0, u32::try_from(source.len()).unwrap()),
                text: source.to_owned(),
                config,
                following_end: source.len(),
            };
            assert!(alias_string_reference(source, &fragment, start, end, "a", value).is_none());
        }
    }

    #[test]
    fn string_alias_inventory_keeps_data_braces_inert_and_real_body_geometry() {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let registry = CommandRegistry::build_default();
        let source =
            "set data {puts \"inert repeated text\"}\nproc p {} {puts \"live repeated text\"}\n";
        let analysis = original::analysis_for_profile(source, profile, &registry);
        let fragments = collect_string_literals(source, analysis.resolved_input.as_ref().unwrap());
        assert!(!fragments.is_empty());
        assert!(
            fragments
                .iter()
                .all(|fragment| !fragment.text.contains("inert"))
        );
        assert!(
            fragments
                .iter()
                .any(|fragment| fragment.text == "live repeated text")
        );
        for fragment in fragments {
            assert_eq!(
                source.get(fragment.span.as_range()),
                Some(fragment.text.as_str())
            );
        }
    }

    #[test]
    fn aggressive_aliases_avoid_compacted_local_collisions() {
        // `request`->`a` (proc-local); the command alias must skip `a`
        // and use `b`, else `$a` in command position would resolve to
        // the local param.  (`handler` itself stays — proc names are
        // public identities in the non-isolated tier.)
        assert_eq!(
            agg(
                "proc handler {request} {\n    mylongcmd $request\n    mylongcmd $request\n    mylongcmd $request\n}\n"
            ),
            "set b mylongcmd;proc handler {a} {$b $a;$b $a;$b $a}",
        );
    }

    #[test]
    fn aggressive_aliases_avoid_live_variable_names() {
        // The alias generator must not claim a name the
        // script reads through a name-taking command (`[set a]` has no
        // `$a` spelling anywhere).  tclsh 9.0.4: with `a` pre-set to
        // SENTINEL, an alias preamble `set a mylongcmd` changes what
        // `[set a]` returns.
        let out = agg("mylongcmd x\nmylongcmd y\nmylongcmd z\nputs [set a]\n");
        assert!(
            !out.contains("set a mylongcmd"),
            "alias clobbered live variable `a`: {out}"
        );
        assert!(out.contains("puts [set a]"), "{out}");
    }

    #[test]
    fn aggressive_runs_optimise_compact_minify() {
        let registry = CommandRegistry::build_default();
        let src = "proc greet {name} {\n    set message \"hi $name\"\n    return $message\n}\n";
        let res = minify_tcl_aggressive(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            false,
            &registry,
        );
        // With no applicable optimisations this equals the compact tier
        // (proc name preserved — public identity).
        assert_eq!(res.source, "proc greet {b} {set a \"hi $b\";return $a}");
        assert_eq!(res.original_length, src.len());
        assert_eq!(res.minified_length(), res.source.len());
        assert!(res.savings_pct() > 0.0);
    }

    #[test]
    fn compact_two_namespace_same_proc_name_is_deterministic_and_intact() {
        // Two namespaces define a `dup` proc.  `::a::dup` has a local
        // (`collidevar`) whose name equals `::b::dup`'s *parameter*.  Keying the
        // proc by simple name matched whichever `dup` came first in `HashMap`
        // order, so `rename_params_in_list` was handed the wrong declaration's
        // parameter region — renaming one proc's `$use` sites while leaving its
        // definition (or the colliding local) under the other proc's name.  The
        // corruption surfaced non-deterministically, so drive it many times and
        // require every run to produce the same intact output.
        let registry = CommandRegistry::build_default();
        let src = "namespace eval a {\n    proc dup {arg} { set collidevar [expr {$arg + 1}]; return $collidevar }\n}\nnamespace eval b {\n    proc dup {collidevar} { return $collidevar }\n}\n";
        // `namespace ev` is the aggressive tier's keyword abbreviation:
        // `ev` is the minimal unique prefix of `eval` in the
        // `namespace` table.  tclsh-proof (8.6.16): `namespace ev a { proc
        // dup {x} { return $x } }` then `a::dup 7` -> 7.
        let expected = "namespace ev a {proc dup {a} {set b [expr {$a+1}];return $b}};namespace ev b {proc dup {a} {return $a}}";
        for _ in 0..32 {
            let res = minify_tcl_aggressive(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                false,
                &registry,
            );
            assert_eq!(res.source, expected, "collision corrupted the output");
        }
    }

    #[test]
    fn compact_never_renames_array_members() {
        // Array member names are Tcl DATA, not private symbols: `array
        // get` / `array names` / traces / serialisation observe them, so
        // no tier may rename them.  tclsh 9.0.4: the original prints
        // `longmember 1`, so a compaction that renames the key prints `a 1`
        // instead.
        let out = min_compact(
            "proc f {} {\n    set arr(longmember) 1\n    return [array get arr]\n}\nputs [f]\n",
        );
        assert!(out.contains("arr(longmember)"), "{out}");
        // And even in isolated mode — observability does not depend on
        // the closed-world assertion.
        let out = min_compact_isolated(
            "proc f {} {\n    set arr(longmember) 1\n    return [array get arr]\n}\nputs [f]\n",
        );
        assert!(out.contains("arr(longmember)"), "{out}");
    }

    #[test]
    fn compact_symbol_map_has_no_array_member_section() {
        let registry = CommandRegistry::build_default();
        let (_, sym) = minify_tcl_compact(
            "proc f {} {\n    set config(database) 1\n    puts $config(database)\n}\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            false,
            &registry,
        );
        assert!(!sym.format().contains("Array members"), "{}", sym.format());
    }

    #[test]
    fn compact_non_isolated_keeps_global_vars() {
        assert_eq!(
            min_compact("set globalvar 1\nputs $globalvar\n"),
            "set globalvar 1;puts $globalvar"
        );
    }

    #[test]
    fn default_tier_never_introduces_variables() {
        // Template deduplication would emit a `set a {…}` preamble +
        // `[subst $a]` — a real variable write that clobbers any live `a`
        // (tclsh 9.0.4: `puts [set a]` then stops printing the pre-existing
        // value), fires traces, and changes `info vars`.  The default tier
        // must stay frame-transparent: no `set`, no `subst`, strings
        // verbatim.
        check(
            "puts \"value is $longvariablename here\"\nputs \"value is $longvariablename here\"\nputs [set a]\n",
            "puts \"value is $longvariablename here\";puts \"value is $longvariablename here\";puts [set a]",
        );
    }

    #[test]
    fn abbreviates_ensemble_subcommand_in_irules() {
        assert_eq!(
            min_dialect("string length $x\n", tcl_dialect::DialectProfile::irules()),
            "string le $x"
        );
        // `info e` would also prefix `info errorstack` (Tcl 8.6+) — reading
        // the abbreviation from `info`'s own registry table (rather than a
        // hand-kept copy that only knew of `exists`) surfaces that and picks
        // the shortest spelling actually safe against the whole table:
        // `ex`, not the old table's `e`.
        assert_eq!(
            min_dialect("info exists $x\n", tcl_dialect::DialectProfile::irules()),
            "info ex $x"
        );
    }

    #[test]
    fn no_subcommand_abbreviation_in_plain_tcl() {
        assert_eq!(
            min_dialect(
                "string length $x\n",
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
            ),
            "string length $x"
        );
    }

    #[test]
    fn expr_comparison_inversion() {
        check("if {!($a == $b)} {puts x}\n", "if {$a!=$b} {puts x}");
        check("if {!($a != $b)} {puts x}\n", "if {$a==$b} {puts x}");
    }

    /// `!($a < $b)` is not `$a >= $b` when an operand may be NaN
    /// (`expr {!(NaN < 1)}` is 1, `expr {NaN >= 1}` is 0). The minifier rewrites
    /// source text with no type information, so it can never prove an operand
    /// NaN-free and must leave the ordered four alone — the parenthesised form
    /// survives verbatim even though inverting would be shorter.
    #[test]
    fn expr_ordered_comparison_inversion_is_declined() {
        check("if {!($a < $b)} {puts x}\n", "if {!($a<$b)} {puts x}");
        check("if {!($a <= $b)} {puts x}\n", "if {!($a<=$b)} {puts x}");
        check("if {!($a > $b)} {puts x}\n", "if {!($a>$b)} {puts x}");
        check("if {!($a >= $b)} {puts x}\n", "if {!($a>=$b)} {puts x}");
    }

    /// The string and membership operators never compare numerically, so no NaN
    /// rule reaches them and their inversions remain available.
    #[test]
    fn expr_string_and_membership_inversion_still_applies() {
        // 13:17 whitespace decision; pinned C/Jim wordop controls. The
        // inversion expectations remain independent and unchanged.
        check("if {!($a eq \"x\")} {puts x}\n", "if {$a ne\"x\"} {puts x}");
        check("if {!($a ne \"x\")} {puts x}\n", "if {$a eq\"x\"} {puts x}");
        check("if {!($a in $b)} {puts x}\n", "if {$a ni$b} {puts x}");
        check("if {!($a ni $b)} {puts x}\n", "if {$a in$b} {puts x}");
    }

    #[test]
    fn expr_de_morgan_forward() {
        check("if {!($a && $b)} {puts x}\n", "if {!$a||!$b} {puts x}");
    }

    #[test]
    fn expr_de_morgan_reverse() {
        check("if {!$a || !$b} {puts x}\n", "if {!$a||!$b} {puts x}");
    }

    #[test]
    fn expr_double_negation_not_folded() {
        // `!!x → x` is unsound outside a top-level boolean condition (it drops
        // the 0/1 coercion), and the minifier can't tell the context apart, so
        // `!!$x` is left intact rather than risk changing a value-context result.
        check("if {!!$x} {puts x}\n", "if {!!$x} {puts x}");
    }

    #[test]
    fn expr_no_change_when_already_minimal() {
        check("if {$a < $b} {puts x}\n", "if {$a<$b} {puts x}");
    }

    #[test]
    fn expr_shrink_nested_in_command_subst() {
        check(
            "set y [expr {!($a==1 && $b==2)}]\n",
            "set y [expr {$a!=1||$b!=2}]",
        );
    }

    #[test]
    fn collapses_commands_to_semicolons() {
        check("set x 1\nset y 2\n", "set x 1;set y 2");
    }

    #[test]
    fn collapses_intra_command_whitespace() {
        check("set    x     1\n", "set x 1");
    }

    #[test]
    fn recurses_into_proc_body() {
        check(
            "proc f {} {\n    # c\n    set x 1\n}\n",
            "proc f {} {set x 1}",
        );
    }

    #[test]
    fn recurses_into_command_substitution() {
        check("set y [ expr {1 + 2} ]\n", "set y [expr {1+2}]");
    }

    #[test]
    fn strips_redundant_quotes() {
        check("puts \"hello\"\n", "puts hello");
    }

    #[test]
    fn keeps_quotes_when_needed() {
        check("puts \"hello world\"\n", "puts \"hello world\"");
    }

    #[test]
    fn compresses_expr_whitespace() {
        check("if {$a == 1} {\n    puts hi\n}\n", "if {$a==1} {puts hi}");
    }

    #[test]
    fn keeps_word_operator_spacing() {
        // Remove every safe gap. The space before eq is required here,
        // while the gap between eq and $b is removable.
        check(
            "if {$a eq $b} {\n    puts hi\n}\n",
            "if {$a eq$b} {puts hi}",
        );
    }

    /// Shared TIP 461 word-operator spellings retain canonical spacing.
    #[test]
    fn keeps_tip461_word_operator_spacing() {
        check(
            "if {$a lt $b} {\n    puts hi\n}\n",
            "if {$a lt $b} {puts hi}",
        );
    }

    #[test]
    fn minifies_switch_case_bodies() {
        check(
            "switch $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}\n",
            "switch $x {a {puts 1} b {puts 2}}",
        );
    }

    #[test]
    fn switch_fallthrough_preserved() {
        check(
            "switch $x {\n    a -\n    b {\n        puts 2\n    }\n}\n",
            "switch $x {a - b {puts 2}}",
        );
    }

    #[test]
    fn switch_hash_pattern_is_not_a_comment() {
        // A braced case list is a Tcl LIST, so `#` is an ordinary pattern
        // there, never a comment.  tclsh 9.0.4: `switch # { # {puts matched}
        // default {puts default} }` prints `matched`, so a minifier that
        // drops the `#` arm changes the output to `default`.
        check(
            "switch # {\n    # {puts matched}\n    default {puts default}\n}\n",
            "switch # {# {puts matched} default {puts default}}",
        );
    }

    #[test]
    fn switch_hash_prefixed_pattern_and_semicolon_pattern_survive() {
        // `#foo` and `;` are ordinary list elements too.
        check(
            "switch $x {\n    #foo {puts a}\n    {;} {puts b}\n    default {puts c}\n}\n",
            "switch $x {#foo {puts a} {;} {puts b} default {puts c}}",
        );
    }

    #[test]
    fn switch_backslash_escaped_pattern_survives_verbatim() {
        // A bare element with backslash escapes (`a\ b` — one pattern
        // word containing a space) must be re-emitted exactly as
        // written, not decoded or re-quoted.
        check(
            "switch $x {\n    a\\ b {puts one}\n    default {puts two}\n}\n",
            "switch $x {a\\ b {puts one} default {puts two}}",
        );
    }

    #[test]
    fn switch_odd_length_case_list_preserved_verbatim() {
        // An odd-length list is a runtime error ("extra switch pattern
        // with no body") whose message quotes the original text — the
        // minifier must not restructure it.
        check(
            "switch $x {\n    a {puts 1}\n    b\n}\n",
            "switch $x {\n    a {puts 1}\n    b\n}",
        );
    }

    #[test]
    fn switch_dynamic_case_list_not_treated_as_case_list() {
        // A non-braced final arg (`$cases`) is not the braced clause-list
        // form; nothing to recurse into.
        check("switch $x $cases\n", "switch $x $cases");
    }

    #[test]
    fn renamed_switch_lookalike_user_command_untouched() {
        // A same-named USER command in a namespace (`::my::switch`) is not
        // the registry `switch`; its braced arg is not a case list.  The
        // registry lookup is by resolved head, so the qualified spelling
        // does not match and the argument body is left as an opaque word.
        let out = min("my::switch x {\n    # {puts matched}\n}\n");
        assert!(out.contains("# {puts matched}"), "{out}");
    }

    #[test]
    fn switch_braced_and_quoted_multiword_patterns_keep_delimiters() {
        // A braced `{a b}` / quoted `"c d"` pattern must keep its
        // delimiters so the case list stays balanced and re-parses as one word
        // per pattern.  Dropping the quotes turned `"c d"` into two bare words.
        check(
            "switch $x {\n  {a b} { puts one }\n  \"c d\" { puts two }\n  default { puts def }\n}\n",
            "switch $x {{a b} {puts one} \"c d\" {puts two} default {puts def}}",
        );
    }

    #[test]
    fn nested_body_recursion() {
        check(
            "proc f {} {\n    if {$x} {\n        set y 1\n    }\n}\n",
            "proc f {} {if {$x} {set y 1}}",
        );
    }

    #[test]
    fn empty_source_minifies_to_empty() {
        check("\n\n# only a comment\n", "");
    }

    /// `apply`'s lambda-literal argument (`ArgRole::LambdaLiteral`,
    /// not `Body`) must have its real body element minified — not the whole
    /// `{argList} {body}` blob re-segmented as one script (which misreads
    /// the parameter word as a command name and never reaches the real
    /// body, leaving comments / extra whitespace inside it untouched). The
    /// parameter-list element itself stays verbatim, matching how a plain
    /// `proc` parameter list is never specially minified either.
    #[test]
    fn apply_lambda_body_minifies() {
        check(
            "apply {dir {\n    # a comment\n    puts    $dir\n}} /tmp\n",
            "apply {dir {puts $dir}} /tmp",
        );
    }

    /// A bare body element's backslash
    /// escape must be decoded before minification, not pasted through raw —
    /// `puts\ hi`'s real runtime body is the two-word command `puts hi`, and
    /// the minified output must preserve that (not keep the backslash, which
    /// would change what `apply` actually runs).
    #[test]
    fn apply_lambda_body_with_backslash_escape_decodes() {
        check(r"apply {{} puts\ hi}", "apply {{} {puts hi}}");
    }

    /// A multi-word parameter list is itself a *nested* list element
    /// (`{x y}`) — reassembling it without its own braces would collapse it
    /// into separate top-level elements and corrupt the lambda's arity.
    #[test]
    fn apply_lambda_multi_word_param_list_keeps_its_braces() {
        check(
            "apply {{x y} {\n    return [expr {$x + $y}]\n}} 1 2",
            "apply {{x y} {return [expr {$x+$y}]}} 1 2",
        );
    }

    // Keyword abbreviations.
    //
    // tclsh ground truth (8.6.16): `string le abc` → `3`, `string eq a a` → `1`,
    // `lsearch -noc {A b} a` → `0`; `string l abc` → `unknown or ambiguous
    // subcommand "l"`, so an ambiguous prefix must never be emitted.
    mod abbreviations {
        use super::*;

        fn aggressive(src: &str) -> String {
            let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
            minify_tcl_aggressive(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                false,
                registry,
            )
            .source
        }

        fn aggressive_no_abbrev(src: &str) -> String {
            let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
            minify_tcl_aggressive_with(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                false,
                registry,
                false,
            )
            .source
        }

        #[test]
        fn subcommand_words_shorten_to_their_minimal_unique_prefix() {
            let out = aggressive("puts [string length $x]\n");
            assert!(out.contains("string le "), "{out}");
            assert!(!out.contains("string length"), "{out}");
        }

        // tclsh-proof (8.6.16): `string equal -n ABC abc` -> 1 -- `-n` is the
        // minimal unique prefix in `string equal`'s two-option table
        // (`-nocase`, `-length`).
        #[test]
        fn option_words_shorten_too() {
            let out = aggressive("puts [string equal -nocase $a $b]\n");
            assert!(out.contains("-n "), "{out}");
            assert!(!out.contains("-nocase"), "{out}");
        }

        #[test]
        fn the_emitter_can_be_turned_off() {
            let out = aggressive_no_abbrev("puts [string length $x]\n");
            assert!(out.contains("string length"), "{out}");
        }

        #[test]
        fn a_keyword_with_no_shorter_unique_spelling_is_left_alone() {
            // `trim` is a proper prefix of `trimleft`/`trimright`, so it can only
            // be written in full.
            let out = aggressive("puts [string trim $x]\n");
            assert!(out.contains("string trim"), "{out}");
        }

        #[test]
        fn dynamic_and_expanded_words_are_never_abbreviated() {
            for src in ["puts [string $sub $x]\n", "puts [string {*}$words $x]\n"] {
                let out = aggressive(src);
                assert!(!out.contains("string le"), "{src} -> {out}");
            }
        }

        #[test]
        fn command_names_are_never_abbreviated() {
            // Tcl does not prefix-match command names.
            let out = aggressive("puts hello\n");
            assert!(out.contains("puts"), "{out}");
            assert!(!out.contains("put "), "{out}");
        }

        #[test]
        fn a_boolean_value_definition_keeps_its_bytes() {
            // `set flag true` is a value-definition site: `$flag` may later meet
            // `eq "true"`, so `t` and `true` are not interchangeable.
            let out = aggressive("set flag true\nputs $flag\n");
            assert!(out.contains("true"), "{out}");
        }

        #[test]
        fn a_prefix_a_later_release_makes_ambiguous_is_not_emitted_for_an_older_target() {
            // `string compare` is `co` in 8.5 but `string cat` (8.6.2) collides
            // at `c`; the emitter checks every later core-Tcl table, so whatever
            // it emits must still resolve to `compare` under 9.0.
            let registry = tcl_registry::model::ingress::static_context_for("tcl8.5").commands();
            let out = minify_tcl_aggressive(
                "puts [string compare $a $b]\n",
                tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
                false,
                registry,
            )
            .source;
            let emitted = out.split_whitespace().find(|w| "compare".starts_with(*w));
            if let Some(word) = emitted {
                let t90 = tcl_registry::model::ingress::static_context_for("tcl9.0")
                    .commands()
                    .get("string")
                    .expect("string in 9.0")
                    .subcommand_table(None, None, None);
                assert_eq!(
                    t90.resolve(word).unique(),
                    Some("compare"),
                    "emitted {word:?} is not `compare` under Tcl 9.0: {out}"
                );
            }
        }
    }

    /// Minification must resolve a command head's *effective
    /// identity*, not its written spelling.
    ///
    /// A body-role argument is re-minified as the script it is (comments
    /// stripped, whitespace collapsed); an unrecognised command's braced word
    /// is emitted verbatim.  That difference is the witness.
    ///
    /// tclsh oracle (8.6.16 and 9.0.4, byte-identical): `interp alias {} maybe
    /// {} if` makes `maybe` run `if`; `rename if maybe` moves it and leaves
    /// `if` gone; a top-level `proc if …` takes the name over.
    fn body_was_minified(src: &str) -> bool {
        !min(src).contains("# c")
    }

    const BODY_CALL: &str = " {$x} {\n    # c\n    puts a\n}\n";

    #[test]
    fn minify_follows_an_aliased_body_command() {
        assert!(body_was_minified(&format!(
            "interp alias {{}} maybe {{}} if\nmaybe{BODY_CALL}"
        )));
        // The `::`-qualified spelling of the alias classifies alike.
        assert!(body_was_minified(&format!(
            "interp alias {{}} maybe {{}} if\n::maybe{BODY_CALL}"
        )));
        // Guard: an unbound `maybe` has no body argument to descend into.
        assert!(!body_was_minified(&format!("set y 1\nmaybe{BODY_CALL}")));
    }

    #[test]
    fn minify_follows_a_renamed_body_command() {
        assert!(body_was_minified(&format!(
            "rename if maybe\nmaybe{BODY_CALL}"
        )));
        assert!(
            !body_was_minified(&format!("rename if maybe\nif{BODY_CALL}")),
            "a renamed-away `if` must not keep the built-in's body grammar"
        );
    }

    #[test]
    fn minify_abstains_for_a_builtin_shadowed_by_a_user_proc() {
        assert!(
            !body_was_minified(&format!("proc if {{c b}} {{return 1}}\nif{BODY_CALL}")),
            "a user `proc if` takes the name over; its braced word is opaque data"
        );
        // Guard: the unshadowed built-in still descends.
        assert!(body_was_minified(&format!("set y 1\nif{BODY_CALL}")));
    }

    #[test]
    fn minify_abstains_for_a_dynamic_binding() {
        assert!(
            !body_was_minified(&format!("rename $old maybe\nmaybe{BODY_CALL}")),
            "a dynamic rename must not give `maybe` a body grammar"
        );
        assert!(
            body_was_minified(&format!("rename $old maybe\nif{BODY_CALL}")),
            "a dynamic rename must not take `if`'s body grammar away either"
        );
    }

    /// The aggressive tier's keyword-abbreviation phase reads the resolved
    /// head too: the subcommand table it shortens against belongs to the
    /// command the head *is*.
    #[test]
    fn keyword_abbreviation_follows_an_aliased_head() {
        let registry = CommandRegistry::build_default();
        let aliased = minify_tcl_aggressive(
            "interp alias {} str {} string\nstr toupper $::env(HOME)\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            true,
            &registry,
        );
        assert!(
            aliased.source.contains("str tou "),
            "an alias of `string` must abbreviate against `string`'s subcommand \
             table; got {:?}",
            aliased.source
        );
        // Guard: an unbound `str` has no subcommand table, so `toupper` is
        // ordinary data and must survive untouched.
        let unbound = minify_tcl_aggressive(
            "set y 1\nstr toupper $::env(HOME)\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            true,
            &registry,
        );
        assert!(
            unbound.source.contains("str toupper"),
            "{:?}",
            unbound.source
        );
    }

    /// The aggressive tier's rename-*barrier* scan reads the resolved head:
    /// `upvar`'s "aliases the caller's frame" trait bars local-name compaction
    /// in every scope, and that trait belongs to the command a head *is*.
    ///
    /// The witness is whether `set local 1` survives: barred, the local keeps
    /// its name; unbarred, it compacts to a single letter.
    #[test]
    fn rename_barriers_retain_distinct_actual_scopes_with_identical_labels() {
        let source = "proc repeated {} {set firstlong 1; puts $firstlong}\nproc repeated {} {set secondlong 2; puts $secondlong}\n";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        let scopes = analysis
            .global_scope
            .children
            .iter()
            .filter(|scope| scope.kind == ScopeKind::Proc)
            .collect::<Vec<_>>();
        assert_eq!(scopes.len(), 2);
        assert_eq!(scopes[0].name, scopes[1].name);
        assert!(!std::ptr::eq(scopes[0], scopes[1]));
        let first = source_scope_at_offset(
            &analysis.global_scope,
            scopes[0].body_span.unwrap().start(),
            "::",
            false,
        )
        .unwrap();
        let second = source_scope_at_offset(
            &analysis.global_scope,
            scopes[1].body_span.unwrap().start(),
            "::",
            false,
        )
        .unwrap();
        assert_eq!(first.1, second.1);
        assert!(std::ptr::eq(first.0, scopes[0]));
        assert!(std::ptr::eq(second.0, scopes[1]));
        let mut barriers = RenameBarriers::default();
        apply_rename_traits(
            &mut barriers,
            Traits::CREATES_DYNAMIC_BARRIER,
            Some(first.0),
        );
        apply_rename_traits(
            &mut barriers,
            Traits::CREATES_DYNAMIC_BARRIER,
            Some(first.0),
        );
        assert_eq!(barriers.scopes.len(), 1);
        assert!(!barriers.allows_scope(first.0));
        assert!(barriers.allows_scope(second.0));
        barriers.global_variables = true;
        assert!(!barriers.allows_scope(&analysis.global_scope));
        assert!(barriers.allows_scope(second.0));
    }

    #[test]
    fn report_scope_collisions_preserve_every_selected_mapping() {
        let mut map = SymbolMap::default();
        let first = BTreeMap::from([("longfirst".to_owned(), "a".to_owned())]);
        let second = BTreeMap::from([("longsecond".to_owned(), "b".to_owned())]);
        record_variable_report(&mut map, "::same", first.clone());
        record_variable_report(&mut map, "::same [source 2]", second.clone());
        record_variable_report(&mut map, "::same", second.clone());
        assert_eq!(map.variables.len(), 3);
        assert_eq!(map.variables["::same"], first);
        assert_eq!(map.variables["::same [source 2]"], second);
        assert_eq!(map.variables["::same [source 3]"], second);
    }

    #[test]
    fn logical_formal_edit_deduplication_uses_actual_scope_not_report_keys() {
        let source = "proc first {longname} {return $longname}";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            LexerConfig::for_file_grammar(profile.grammar),
        );
        let analysis = original::analysis_with_input(source, &input);
        let body = analysis
            .global_scope
            .children
            .iter()
            .find(|scope| scope.kind == ScopeKind::Proc)
            .unwrap();
        let (_, label) = source_scope_at_offset(
            &analysis.global_scope,
            body.body_span.unwrap().start(),
            "::",
            false,
        )
        .unwrap();
        let unrelated_report = BTreeMap::from([("unrelated".to_owned(), "x".to_owned())]);
        let mut report = SymbolMap::default();
        report
            .variables
            .insert(label.clone(), unrelated_report.clone());
        let mut edits = Vec::new();
        compact_logical_formals(
            source,
            &input,
            &analysis,
            true,
            &[],
            &mut report,
            &mut edits,
        );
        assert_eq!(edits.len(), 2);
        assert_eq!(report.variables[&label], unrelated_report);
        assert_eq!(report.variables.len(), 2);
        let mut report = SymbolMap::default();
        let mut edits = Vec::new();
        compact_logical_formals(
            source,
            &input,
            &analysis,
            true,
            &[body],
            &mut report,
            &mut edits,
        );
        assert!(edits.is_empty());
        assert!(report.variables.is_empty());
        compact_logical_formals(
            source,
            &input,
            &analysis,
            false,
            &[],
            &mut report,
            &mut edits,
        );
        assert!(edits.is_empty());
    }

    #[test]
    fn rename_barriers_follow_the_resolved_head() {
        const UPVAR_BODY: &str =
            "proc p {v} {\n    set local 1\n    upvar 1 $v alias\n    return $local\n}\n";
        const PEEK_BODY: &str =
            "proc p {v} {\n    set local 1\n    peek 1 $v alias\n    return $local\n}\n";
        let registry = CommandRegistry::build_default();
        let minify = |src: &str| {
            minify_tcl_aggressive(
                src,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                true,
                &registry,
            )
            .source
        };
        let barred = |src: &str| minify(src).contains("set local 1");

        // Baseline: the built-in bars compaction.
        assert!(barred(&format!("set y 1\n{UPVAR_BODY}")));

        // A user `proc upvar` takes the name over, so the built-in's trait no
        // longer applies and the local compacts.
        assert!(
            !barred(&format!("proc upvar {{a b c}} {{return 1}}\n{UPVAR_BODY}")),
            "a shadowed `upvar` must not keep barring compaction"
        );
        // Likewise once the name has been renamed away.
        assert!(
            !barred(&format!("rename upvar peek\n{UPVAR_BODY}")),
            "a renamed-away `upvar` must not keep barring compaction"
        );
        // An alias of `upvar` bars exactly as `upvar` does.
        assert!(barred(&format!(
            "interp alias {{}} peek {{}} upvar\n{PEEK_BODY}"
        )));
        // A dynamic rename proves nothing about either name.
        assert!(
            !barred(&format!("rename $old peek\n{PEEK_BODY}")),
            "a dynamic rename must not make `peek` a barrier"
        );
        assert!(
            barred(&format!("rename $old peek\n{UPVAR_BODY}")),
            "a dynamic rename must not take `upvar`'s barrier away either"
        );
    }
}

#[cfg(test)]
mod constructed_compatibility_tests {
    use super::*;

    #[test]
    fn shared_minifier_call_sites_keep_same_names_in_their_actual_namespace() {
        let source = "namespace eval A {proc longName {} {}; longName}\nnamespace eval B {proc longName {} {}; longName}\n";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let selected = analysis
            .all_procs
            .get("::A::longName")
            .expect("actual A declaration");
        let called = source.find("; longName").unwrap() + 2;
        assert_eq!(
            current_proc_call_sites(source, selected, &analysis),
            vec![Span::new(
                u32::try_from(called).unwrap(),
                u32::try_from(called + "longName".len()).unwrap(),
            )]
        );
        let detached = selected.clone();
        assert!(current_proc_call_sites(source, &detached, &analysis).is_empty());
        assert!(current_proc_call_sites(&format!("{source} "), selected, &analysis).is_empty());
    }

    #[test]
    fn constructed_minifier_call_sites_keep_original_indirect_and_naming_guards() {
        // Descriptive compatibility rows do not authorise public source rewriting.
        use tcl_compiler::analyser::Analyser;
        use tcl_compiler::signature_scan::types::SignatureCommandInvocation;
        let source = ":箱::méthode\n箱::méthode\nother\n:箱::méthode\n$cmd\n";
        let written = |name: &str, start: usize| {
            SignatureCommandInvocation::written(
                name.to_owned(),
                Span::new(
                    u32::try_from(start).unwrap(),
                    u32::try_from(start + name.len()).unwrap(),
                ),
                Some(0),
            )
        };
        let selected = written(":箱::méthode", 0);
        let wrong = written("箱::méthode", source.find("\n箱").unwrap() + 1);
        let mut resolved = written("other", source.find("other").unwrap());
        resolved.resolved_qualified_name = Some(":::箱::méthode".to_owned());
        let mut foreign = written(":箱::méthode", source.rfind(":箱::méthode").unwrap());
        foreign.resolved_qualified_name = Some("::箱::méthode".to_owned());
        let mut indirect = written("$cmd", source.find("$cmd").unwrap());
        indirect.resolved_qualified_name = Some(":::箱::méthode".to_owned());
        indirect.indirect = true;
        let expected = vec![selected.range, resolved.range];
        let other = vec![wrong.range, foreign.range];
        let indirect_span = indirect.range;
        let mut analysis = Analyser::new().analyse("", "tcl");
        analysis.command_invocations = vec![
            selected.clone(),
            selected,
            wrong,
            resolved,
            foreign,
            indirect,
        ];
        assert_eq!(
            find_proc_call_sites("méthode", ":::箱::méthode", &analysis),
            expected
        );
        assert_eq!(
            find_proc_call_sites("méthode", "::箱::méthode", &analysis),
            other
        );
        assert!(
            !find_proc_call_sites("méthode", ":::箱::méthode", &analysis).contains(&indirect_span)
        );

        let source = "proc publicName {} {return VALUE}\npublicName\n";
        let original = Analyser::new().analyse(source, "tcl8.6");
        assert!(!original.allows_lexical_declaration_advice());
        let result = minify_with_analysis(
            source,
            &original,
            MinifyOptions {
                tier: MinifyTier::Compact,
                isolated: true,
                abbreviations: false,
            },
        );
        assert!(
            result
                .refusals
                .contains(&MinifyRefusal::MissingAlphaRenameContract)
        );
        assert_eq!(result.symbol_map, SymbolMap::default());
        assert!(result.source.contains("proc publicName"));
        assert!(result.source.contains("publicName"));
    }
}

#[cfg(test)]
mod selected_scalar_reference_emission_tests {
    use super::*;

    #[test]
    fn logical_scalar_emission_uses_selected_proposals_and_original_component_boundaries() {
        for (dialect, source, expected) in [
            ("tcl8.4", "${a{b}c}", "${a{b}"),
            ("tcl9.0", "${a{b}c}", "${a{b}c}"),
            ("tcl9.0", "${a{b}}", "${a{b}}"),
            ("tcl8.6", "${$n}", "${$n}"),
            ("tcl8.6", "${}", "${}"),
            ("tcl8.6", "${x}tail", "${x}"),
            ("tcl8.6", "${x}::tail", "${x}"),
            ("tcl9.0", "${x}::tail", "${x}"),
            ("tcl8.6", "${x}(k)", "${x}"),
            ("tcl8.6", "$array($index)", "$array($index)"),
            ("tcl8.6", "${array(key)}", "${array(key)}"),
        ] {
            let config = LexerConfig::for_dialect(dialect);
            let tokens = Lexer::with_config(source, config).tokenise_all().unwrap();
            let token = tokens
                .iter()
                .find(|token| token.kind == TokenType::Var)
                .copied()
                .unwrap();
            let end = tcl_lexer::word_parts::scan_var_ref(
                source.as_bytes(),
                token.span.start() as usize,
                config,
            )
            .unwrap()
            .unwrap()
            .next;
            let result = reconstruct_variable_reference(
                &SourceMap::new(source),
                token,
                &source[end..],
                config,
            )
            .unwrap();
            assert_eq!(result, expected, "{dialect}: {source}");
        }
        let source = "${a{b}c}";
        let token = Lexer::with_config(source, LexerConfig::for_dialect("tcl8.4"))
            .tokenise_all()
            .unwrap()[0];
        assert!(
            reconstruct_variable_reference(
                &SourceMap::new(source),
                token,
                "",
                LexerConfig::for_dialect("tcl9.0")
            )
            .is_none()
        );

        let source = "puts ${x}tail; puts ${x}(k); puts $array($index); puts ${x}::tail; puts \"${x}::tail\"";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        let config = analysis.body_lexer_config.unwrap();
        let result = lexical_default(source, &analysis).unwrap();
        assert_eq!(
            tcl_compiler::var_refs::scan_var_ref_forms_braced_with_config(&result, config),
            tcl_compiler::var_refs::scan_var_ref_forms_braced_with_config(source, config)
        );
    }
}

#[cfg(test)]
mod symbol_report_tests {
    use super::*;

    #[test]
    fn symbol_report_round_trip_retains_all_literal_labels_and_sections() {
        let mut symbols = SymbolMap::default();
        let original = " \n# Variables in injected\n  z <- forged\r\0 ";
        symbols.procs.insert(original.to_owned(), "a".to_owned());
        symbols
            .variables
            .entry(" scope\n# Procs\r ".to_owned())
            .or_default()
            .insert(original.to_owned(), "b".to_owned());
        for values in [
            &mut symbols.command_aliases,
            &mut symbols.argument_aliases,
            &mut symbols.string_aliases,
            &mut symbols.static_folds,
        ] {
            values.insert(original.to_owned(), " value\n\r\0 ".to_owned());
        }
        let report = symbols.format();
        assert!(report.contains("# Procs"));
        assert_eq!(SymbolMap::parse_structured(&report), Ok(symbols.clone()));
        assert_eq!(SymbolMap::parse(&report), symbols);
        assert_eq!(SymbolMap::parse(&format!("{report}\n\n")), symbols);
        let unsupported = format!(
            "{SYMBOL_MAP_RECORD}{}",
            serde_json::json!({"version": 99, "symbols": symbols})
        );
        assert_eq!(
            SymbolMap::parse_structured(&unsupported),
            Err(SymbolMapReportError::UnsupportedVersion(99))
        );
        let corrupted = format!("{report}\n{SYMBOL_MAP_RECORD}broken");
        assert_eq!(
            SymbolMap::parse_structured(&corrupted),
            Err(SymbolMapReportError::InvalidRecord)
        );
        assert_eq!(SymbolMap::parse(&corrupted), SymbolMap::default());
    }

    #[test]
    fn symbol_report_scope_labels_stay_separate_and_ambiguous_display_declines() {
        let mut symbols = SymbolMap::default();
        symbols
            .variables
            .entry("a:b".to_owned())
            .or_default()
            .insert("left".to_owned(), "c".to_owned());
        symbols
            .variables
            .entry("a".to_owned())
            .or_default()
            .insert("right".to_owned(), "b:c".to_owned());
        let reversed = symbols.scoped_reverse();
        assert_eq!(
            reversed.get(&ScopedSymbolMapLabel {
                scope_label: "a:b".to_owned(),
                compacted: "c".to_owned()
            }),
            Some(&vec!["left".to_owned()])
        );
        assert_eq!(
            reversed.get(&ScopedSymbolMapLabel {
                scope_label: "a".to_owned(),
                compacted: "b:c".to_owned()
            }),
            Some(&vec!["right".to_owned()])
        );
        assert!(!symbols.reverse().contains_key("a:b:c"));
        assert_eq!(SymbolMap::parse(&symbols.format()), symbols);
    }
}

#[cfg(test)]
mod original_formal_name_geometry_tests {
    use super::*;

    fn logical_input() -> tcl_compiler::analyser::ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            LexerConfig::for_profile(Some(profile)),
        )
    }

    #[test]
    fn compact_original_formal_fields_preserve_wrappers_and_unicode_defaults() {
        let input = logical_input();
        let source = r#"proc p {{{longName} {default α}} {"otherName" longName} args} {return [list $longName $otherName $args]}"#;
        let analysis = original::analysis_with_input(source, &input);
        let procedure = analysis.all_procs.values().next().unwrap();
        let fields = lexical_metadata::parameter_name_sources(source, &analysis, procedure)
            .expect("actual Logical header, ordered formal fields and body card");
        assert_eq!(
            fields
                .iter()
                .map(
                    tcl_compiler::signature_scan::formal_parameters::OriginalSourceFormalName::name
                )
                .collect::<Vec<_>>(),
            [
                b"longName".as_slice(),
                b"otherName".as_slice(),
                b"args".as_slice()
            ]
        );
        let mut edits = Vec::new();
        rename_params_in_list(
            &fields,
            &BTreeMap::from([
                ("longName".to_owned(), "a".to_owned()),
                ("otherName".to_owned(), "b".to_owned()),
            ]),
            &mut edits,
        );
        assert_eq!(
            apply_edits(source, edits),
            r#"proc p {{{a} {default α}} {"b" longName} args} {return [list $longName $otherName $args]}"#
        );
    }

    #[test]
    fn compact_original_formal_fields_reach_the_existing_editing_pass() {
        let input = logical_input();
        let source = "proc p {{longName default} {otherName longName} args} {return VALUE}";
        let (output, mapping) = compact_names(source, &input, false);
        assert_eq!(
            output,
            "proc p {{a default} {b longName} args} {return VALUE}"
        );
        assert_eq!(mapping.variables["::p"]["longName"], "a");
        assert_eq!(mapping.variables["::p"]["otherName"], "b");
        assert!(!mapping.variables["::p"].contains_key("args"));
    }

    #[test]
    fn compact_original_formal_fields_refuse_cooked_dynamic_and_foreign_geometry() {
        let input = logical_input();
        for source in [
            r#"proc p "first\x20second" {return $first}"#,
            "proc p $parameters {return VALUE}",
        ] {
            let analysis = original::analysis_with_input(source, &input);
            for procedure in analysis.all_procs.values() {
                assert!(
                    lexical_metadata::parameter_name_sources(source, &analysis, procedure)
                        .is_none(),
                    "{source}"
                );
            }
            let (output, _) = compact_names(source, &input, false);
            assert_eq!(output, source);
        }
        let source = "proc p {longName} {return $longName}";
        let mut analysis = original::analysis_with_input(source, &input);
        let actual = analysis.all_procs.values().next().unwrap();
        assert!(lexical_metadata::parameter_name_sources(source, &analysis, actual).is_some());
        let procedure = actual.clone();
        let mut wrong_procedure = procedure.clone();
        wrong_procedure.params[0].name = "otherName".to_owned();
        assert!(
            lexical_metadata::parameter_name_sources(source, &analysis, &wrong_procedure).is_none()
        );
        assert!(
            lexical_metadata::parameter_name_sources(
                "proc p {otherName} {return $longName}",
                &analysis,
                analysis.all_procs.values().next().unwrap()
            )
            .is_none()
        );
        analysis.body_lexer_config = Some(LexerConfig {
            leading_bom: match input.lexer_config().leading_bom {
                tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
                tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
            },
            ..input.lexer_config()
        });
        assert!(
            lexical_metadata::parameter_name_sources(
                source,
                &analysis,
                analysis.all_procs.values().next().unwrap()
            )
            .is_none()
        );
        let native = tcl_registry::model::ingress::resolve_environment("tcl8.6");
        let native_analysis =
            tcl_compiler::analyser::Analyser::new().analyse(source, native.analyser_profile().name);
        for procedure in native_analysis.all_procs.values() {
            assert!(
                lexical_metadata::parameter_name_sources(source, &native_analysis, procedure)
                    .is_none()
            );
        }
    }
}

#[cfg(test)]
mod authored_compact_candidate_tests {
    use super::*;

    #[test]
    fn aggressive_alias_candidates_reject_only_observed_spellings_under_whole_config() {
        let word = "retained_long_argument";
        let source = [word; 3].join(" ");
        let order = vec![word.to_owned()];
        let uses = HashMap::from([(
            word.to_owned(),
            vec![0, word.len() + 1, 2 * (word.len() + 1)],
        )]);
        let mut claimed = HashSet::from([
            "a".to_owned(),
            "b".to_owned(),
            "$c".to_owned(),
            "c(index)".to_owned(),
            "::c".to_owned(),
            "é".to_owned(),
        ]);
        let config = LexerConfig {
            strict_quoting: true,
            leading_bom: tcl_lexer::LeadingBom::Skip,
            ..LexerConfig::default()
        };
        let (rewritten, aliases) = alias_by_uses(&source, &order, &uses, &mut claimed, config);
        assert_eq!(aliases.get(word).map(String::as_str), Some("c"));
        assert_eq!(rewritten, format!("set c {word}\n$c $c $c"));
        assert!(claimed.contains("c"));
        assert_eq!(
            tcl_syntax::naming::scalar_source_reference("c", config, false).as_deref(),
            Some("$c")
        );
    }
}

#[cfg(test)]
mod original_alias_phase_tests {
    use super::*;

    #[test]
    fn alias_inventory_keeps_inert_data_out_of_selected_nested_phases() {
        let source = "set data {foo --verylongflag --verylongflag}\nproc p {} {foo --verylongflag;foo --verylongflag}\nputs [foo --verylongflag]\n";
        let registry = CommandRegistry::build_default();
        let analysis = original::analysis_for_profile(
            source,
            tcl_dialect::DialectProfile::plain_tcl(),
            &registry,
        );
        let input = analysis.resolved_input.as_ref().unwrap();
        let (_, uses) = source_alias_uses(source, input, AliasWordPurpose::Argument);
        assert_eq!(uses["--verylongflag"].len(), 3);
        assert!(
            uses["--verylongflag"]
                .iter()
                .all(|off| *off > source.find("proc p").unwrap())
        );
        let (rewritten, aliases) = alias_repeated_arguments(source, input, &mut HashSet::new());
        assert!(aliases.contains_key("--verylongflag"));
        assert!(rewritten.contains("{foo --verylongflag --verylongflag}"));
    }

    #[test]
    fn alias_heads_retain_whole_bare_geometry_and_native_refusal() {
        let source =
            "mylongcommand x\n{mylongcommand} x\n\"mylongcommand\" x\nset x 1\nset y 2\nset z 3\n";
        let registry = CommandRegistry::build_default();
        let analysis = original::analysis_for_profile(
            source,
            tcl_dialect::DialectProfile::plain_tcl(),
            &registry,
        );
        let (_, uses) = source_alias_uses(
            source,
            analysis.resolved_input.as_ref().unwrap(),
            AliasWordPurpose::CommandHead,
        );
        assert_eq!(uses["mylongcommand"], vec![0]);
        assert!(!uses.contains_key("set"));
        let native = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            source_alias_uses(
                source,
                native.resolved_input.as_ref().unwrap(),
                AliasWordPurpose::CommandHead
            )
            .1
            .is_empty()
        );
    }

    #[test]
    fn alias_source_constructor_preserves_literal_data_and_scalar_read() {
        let config = LexerConfig::for_profile(Some(tcl_dialect::DialectProfile::plain_tcl()));
        let value = "literal;set victim [danger] $read\\tail";
        let spelling = authored_alias_spelling("a", value, config).unwrap();
        let image = tcl_lexer::SourceImage::document(&spelling.assignment);
        let plan = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(spelling.assignment.len()).unwrap()),
            config,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_none());
        assert_eq!(plan.commands.len(), 1);
        assert_eq!(plan.commands[0].words.len(), 3);
        assert_eq!(
            tcl_syntax::word_rules::original_static_word_unicode_value(&plan.commands[0].words[2])
                .as_deref(),
            Some(value.as_bytes())
        );
        let reference = tcl_lexer::scan_var_ref(spelling.reference.as_bytes(), 0, config)
            .unwrap()
            .unwrap();
        assert_eq!(reference.next, spelling.reference.len());
        assert_eq!(reference.name, b"a");
        assert!(reference.index.is_none());
        assert!(authored_alias_spelling("bad(name)", value, config).is_none());
    }
}
