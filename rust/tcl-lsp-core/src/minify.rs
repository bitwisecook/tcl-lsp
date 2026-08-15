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
//! Pure function: source in, minified source out.  The **default
//! tier** ([`minify_tcl`]) is complete and preserves semantic
//! equivalence by:
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
//!    script, so `#` is an ordinary pattern there, never a comment
//!    (issue #1197).
//! 9. Abbreviating ensemble subcommands for fixed-ensemble
//!    dialects (`f5-irules` / `f5-iapps` / `f5-bigip`).
//!
//! The default tier never introduces variables, writes, or any other
//! observable behaviour — its output is frame-transparent (issue
//! #1194 removed the former `[subst $alias]` template
//! deduplication, whose `set alias {…}` preamble could clobber a
//! live variable, fire traces, and change `info vars`).
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
//! its body references.  Renaming is fenced by registry-declared
//! observability facts (issues #1192/#1193):
//!
//! * Scopes containing a dynamic-barrier command (`upvar`, `eval`,
//!   `trace`, … — [`Traits::CREATES_DYNAMIC_BARRIER`]) or a
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
//!   compacted: `array get` / `array names` / serialization observe
//!   them (issue #1192).
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
//! SCCP static-substring folding (phase 1.5,
//! [`fold_static_substrings`]) replaces `$var` interpolations inside
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

use rustc_hash::FxHashSet;

use tcl_compiler::analyser::{Analyser, AnalysisResult, ProcDef, Scope, ScopeKind};
use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::expr_ast::render_expr;
use tcl_compiler::ir::Statement;
use tcl_compiler::lambda_literal::split_lambda_literal_decoded;
use tcl_compiler::ssa::Version;
use tcl_compiler::taint::{TaintColour, TaintLattice};
use tcl_compiler::{BinOp, ExprNode, UnaryOp, parse_expr};
use tcl_lexer::{Lexer, SourceMap, Span, Token, TokenType, close_quote_offset};
use tcl_registry::abbrev::{KeywordTable, PrefixMatching};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

/// Depth cap for [`minify_body`]'s recursion over nested control-flow
/// bodies, `[…]` command substitutions, and `expr` bodies — issue #996.
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolMap {
    /// Per-scope `{original_var: short}` maps, keyed by scope label.
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

impl SymbolMap {
    /// Human-readable symbol map.
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
        lines.join("\n")
    }

    /// Reverse lookup: compacted name → original.  Variables also
    /// get a `scope:short` → `scope:original` entry; the bare entry
    /// keeps the first scope seen.
    #[must_use]
    pub fn reverse(&self) -> BTreeMap<String, String> {
        let mut rev: BTreeMap<String, String> = BTreeMap::new();
        for (original, short) in &self.procs {
            rev.entry(short.clone()).or_insert_with(|| original.clone());
        }
        for (scope, var_map) in &self.variables {
            for (original, short) in var_map {
                rev.insert(format!("{scope}:{short}"), format!("{scope}:{original}"));
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

    /// Parse a symbol map from the [`Self::format`] text (only the
    /// sections the active tiers emit).
    #[must_use]
    pub fn parse(text: &str) -> Self {
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
    let rev = symbol_map.reverse();
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

/// Full result from aggressive minification.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinifyResult {
    /// The minified source.
    pub source: String,
    /// Compaction symbol map.
    pub symbol_map: SymbolMap,
    /// Number of optimiser rewrites applied.
    pub optimisations_applied: usize,
    /// Length of the original source (bytes).
    pub original_length: usize,
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

/// Minify a Tcl source string for the given dialect (default tier).
#[must_use]
pub fn minify_tcl(source: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let identities =
        tcl_compiler::head_identity::command_head_identities(source, dialect, registry);
    minify_body(
        source,
        MinifyEnv {
            dialect,
            registry,
            identities: &identities,
        },
        0,
    )
}

/// Aggressive minification: apply the compiler's optimiser
/// rewrites, compact names, alias repeated commands / arguments /
/// string substrings, then minify whitespace.  Returns a
/// [`MinifyResult`].
///
/// **Not frame-transparent** (issue #1194): the aliasing phases
/// inject `set alias …` preambles, which create real Tcl variables
/// — observable via `info vars`, variable traces, and any
/// same-named variable in the hosting interpreter.  The alias
/// generators avoid every name the compiler can see (compacted
/// shorts, every analysed / SSA-known variable name, every textual
/// `$name` reference — [`collect_live_names`]), so a collision with
/// a name *present in the script* cannot happen, but names that
/// exist only in the hosting interpreter's frames cannot be proven
/// absent.  Use the default or compact tier where the script must
/// not add variables.
#[must_use]
pub fn minify_tcl_aggressive(
    source: &str,
    dialect: &str,
    isolated: bool,
    registry: &CommandRegistry,
) -> MinifyResult {
    minify_tcl_aggressive_with(source, dialect, isolated, registry, true)
}

/// [`minify_tcl_aggressive`] with the keyword-abbreviation phase (#1230)
/// switchable.
///
/// `abbreviations = false` is the CLI's `--no-abbreviations`: abbreviated
/// output is correct but harder to eyeball-diff, so the emitter can be turned
/// off without giving up the rest of the tier.
#[must_use]
pub fn minify_tcl_aggressive_with(
    source: &str,
    dialect: &str,
    isolated: bool,
    registry: &CommandRegistry,
    abbreviations: bool,
) -> MinifyResult {
    let original_length = source.len();

    // Phase 1: apply the optimiser's semantic-preserving rewrites.
    let optimisations =
        tcl_compiler::optimiser::optimise_with_dialect(source, registry, Some(dialect));
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

    // Phase 1.5: static-substring folding (SCCP-proven constants).
    let (folded, fold_count, static_folds) = fold_static_substrings(&optimised, dialect, registry);

    // Phase 2: compact names.
    let (renamed, mut symbol_map) = compact_names(&folded, dialect, isolated, registry);
    symbol_map.static_folds = static_folds;

    // Phases 2.5–2.7: aliasing.  Seed claimed names with every
    // compacted short so aliases never shadow a local variable, and
    // with every live name the compiler can see in the renamed
    // source (analysed scopes, SSA symbol tables, textual `$refs`)
    // so a preamble `set alias …` never clobbers a variable the
    // script reads through a name-taking command like `[set a]`
    // (issue #1194).
    let mut claimed_names = collect_symbol_shorts(&symbol_map);
    claimed_names.extend(collect_live_names(&renamed, dialect, registry));
    let (renamed, cmd_aliases) =
        alias_repeated_commands(&renamed, dialect, &mut claimed_names, registry);
    symbol_map.command_aliases = cmd_aliases;
    let (renamed, arg_aliases) = alias_repeated_arguments(&renamed, &mut claimed_names);
    symbol_map.argument_aliases = arg_aliases;
    let (renamed, str_aliases) = alias_string_literals(&renamed, &mut claimed_names);
    symbol_map.string_aliases = str_aliases;

    // Phase 2.8: emit unique-prefix keyword abbreviations.
    let (renamed, abbrev_count) = if abbreviations {
        abbreviate_keywords(&renamed, dialect, registry)
    } else {
        (renamed, 0)
    };

    // Phase 3: minify whitespace.
    // The identity facts come from the *renamed* text, which is what the
    // recursion below actually sees.
    let identities =
        tcl_compiler::head_identity::command_head_identities(&renamed, dialect, registry);
    let minified = minify_body(
        &renamed,
        MinifyEnv {
            dialect,
            registry,
            identities: &identities,
        },
        0,
    );

    MinifyResult {
        source: minified,
        symbol_map,
        optimisations_applied: opt_count + fold_count + abbrev_count,
        original_length,
    }
}

/// Minify with local-name compaction: rename proc-local variables,
/// parameters, and proc names to short identifiers, then run the
/// default minifier.  Returns the minified source plus a
/// [`SymbolMap`].
///
/// `isolated` also compacts global-scope variables (safe for
/// self-contained scripts like iRules event handlers).
#[must_use]
pub fn minify_tcl_compact(
    source: &str,
    dialect: &str,
    isolated: bool,
    registry: &CommandRegistry,
) -> (String, SymbolMap) {
    let (renamed, symbol_map) = compact_names(source, dialect, isolated, registry);
    // The identity facts come from the *renamed* text, which is what the
    // recursion below actually sees.
    let identities =
        tcl_compiler::head_identity::command_head_identities(&renamed, dialect, registry);
    let minified = minify_body(
        &renamed,
        MinifyEnv {
            dialect,
            registry,
            identities: &identities,
        },
        0,
    );
    (minified, symbol_map)
}

/// The document-wide context every step of the minify recursion carries
/// unchanged.
///
/// Bundled rather than passed as three parameters because the recursion is
/// deep (`minify_body` → `render_command` → `reconstruct_raw` → `minify_body`)
/// and one of its steps was already at the argument limit.
#[derive(Clone, Copy)]
struct MinifyEnv<'a> {
    /// The document's dialect, for the lexer and the abbreviation tables.
    dialect: &'a str,
    /// The registry the argument roles and clause-list shapes come from.
    registry: &'a CommandRegistry,
    /// The document's statically proven command-identity facts
    /// ([`tcl_compiler::head_identity`]), so a body / lambda / expression /
    /// clause-list argument is recognised by the command a head *is* rather
    /// than the one it is spelled as (issue #1275).
    ///
    /// Read *unpositioned*: this recursion re-minifies each nested body from
    /// its own slice, segmented at offset 0, so no document-absolute offset
    /// exists at the point of the query.
    identities: &'a tcl_compiler::head_identity::HeadIdentityMap,
}

impl<'a> MinifyEnv<'a> {
    /// The registry name a head spelling effectively resolves to.
    fn resolve<'h>(&'h self, head: &'h str) -> &'h str
    where
        'a: 'h,
    {
        self.identities.head_words_unpositioned(head).resolved
    }
}

/// Minify a Tcl script body (top-level or inside braces). `depth` is this
/// body's nesting level — see [`MAX_MINIFY_DEPTH`].
fn minify_body(source: &str, env: MinifyEnv<'_>, depth: u32) -> String {
    let dialect = env.dialect;
    // Native-stack safety net — see `MAX_MINIFY_DEPTH`'s doc comment
    // (issue #996). Past the cap, leave this (deeply nested) body
    // unminified rather than recursing further, matching the existing
    // give-up-gracefully fallback just below for an unparseable body.
    if MAX_MINIFY_DEPTH.exceeded(depth) {
        return source.to_owned();
    }
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::new(source).tokenise_all() else {
        return source.to_owned();
    };

    let commands = parse_commands(source, &tokens);
    if commands.is_empty() {
        return String::new();
    }

    // Render each command, abbreviating ensemble subcommands.
    let mut rendered: Vec<Vec<String>> = Vec::with_capacity(commands.len());
    for cmd_args in &commands {
        let mut arg_strs = render_command(&sm, cmd_args, env, depth);
        if arg_strs.len() >= 2 {
            arg_strs[1] = abbreviated_subcommand(&arg_strs[0], &arg_strs[1], dialect);
        }
        rendered.push(arg_strs);
    }

    // NB: no template deduplication here.  The former `[subst $alias]`
    // rewrite injected a `set alias {…}` preamble — a real Tcl variable
    // write that could clobber a live variable read through a
    // name-taking command (`puts [set a]`), fire traces, and change
    // `info vars` — so it is banned from this semantics-preserving tier
    // (issue #1194).  Aggressive aliasing (an explicitly
    // behaviour-changing tier) covers the same compression ground.
    let is_irules = tcl_registry::prelude::DialectSet::is_irules_dialect(Some(dialect));
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

/// Lazy generator of short identifier names: `a`, `b`, …, `z`,
/// `aa`, `ab`, ….
struct NameGenerator {
    indices: Vec<usize>,
}

impl NameGenerator {
    fn new() -> Self {
        Self { indices: vec![0] }
    }

    fn next_name(&mut self) -> String {
        let name: String = self
            .indices
            .iter()
            .map(|&i| (b'a' + u8::try_from(i).unwrap_or(0)) as char)
            .collect();
        self.advance();
        name
    }

    fn advance(&mut self) {
        let mut pos = self.indices.len();
        loop {
            if pos == 0 {
                // All positions wrapped — grow the length.
                self.indices = vec![0; self.indices.len() + 1];
                return;
            }
            pos -= 1;
            if self.indices[pos] + 1 < 26 {
                self.indices[pos] += 1;
                return;
            }
            self.indices[pos] = 0;
        }
    }
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

/// Scope label: `::` for the root, then `parent::child`.
fn child_scope_label(parent_label: &str, child_name: &str) -> String {
    if parent_label == "::" {
        format!("::{child_name}")
    } else {
        format!("{parent_label}::{child_name}")
    }
}

/// Deepest scope label whose body span contains `offset`
/// (byte-offset based).
fn scope_label_at_offset(
    scope: &Scope,
    offset: u32,
    prefix: &str,
    include_global: bool,
) -> Option<String> {
    for child in &scope.children {
        let label = child_scope_label(prefix, &child.name);
        if let Some(body) = child.body_span
            && body.start() <= offset
            && offset <= body.end()
        {
            if let Some(deeper) = scope_label_at_offset(child, offset, &label, include_global) {
                return Some(deeper);
            }
            return Some(label);
        }
    }
    match scope.kind {
        ScopeKind::Proc => Some(prefix.to_owned()),
        ScopeKind::Global if include_global => Some(prefix.to_owned()),
        _ => None,
    }
}

/// Where renaming is unsafe, as proven by registry-declared
/// observability facts over the script's command invocations.
#[derive(Debug, Default)]
struct RenameBarriers {
    /// Scope labels where variable renaming is barred (a
    /// dynamic-barrier or variable-introspection command runs there).
    scopes: FxHashSet<String>,
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

impl RenameBarriers {
    /// Whether variables in the scope with the given label may be renamed.
    fn allows_scope(&self, label: &str) -> bool {
        !self.all_variable_scopes
            && !self.scopes.contains(label)
            && (label != "::" || !self.global_variables)
    }
}

/// The literal subcommand word following a command head at `inv`, or
/// `None` when the next word is dynamic (`$var` / `[…]` / quoted) or
/// absent.  A dynamic subcommand word means the invocation could be
/// *any* subcommand, so callers must assume the worst-case traits.
fn static_subcommand_word<'s>(
    source: &'s str,
    inv: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
) -> Option<&'s str> {
    let bytes = source.as_bytes();
    let mut pos = inv.range.end() as usize;
    while bytes.get(pos).is_some_and(|b| matches!(b, b' ' | b'\t')) {
        pos += 1;
    }
    let start = pos;
    while bytes
        .get(pos)
        .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-' || *b == b':')
    {
        pos += 1;
    }
    if pos == start {
        return None;
    }
    source.get(start..pos)
}

/// Compute every rename barrier the script's invocations impose.
///
/// All observability knowledge is registry data — traits on command
/// and subcommand specs — never a spelled command name:
///
/// * [`Traits::CREATES_DYNAMIC_BARRIER`] (command level) bars the
///   containing scope, as before.
/// * [`Traits::CREATES_SCOPE_ALIAS`] (`global` / `variable` /
///   `upvar`) additionally bars the global scope — the alias links a
///   global / namespace cell by name from elsewhere.
/// * [`Traits::ALIASES_CALLER_FRAME`] (`upvar`) and
///   [`Traits::EVALUATES_IN_SHIFTED_FRAME`] (`uplevel`) bar **every**
///   scope: the observed frame is chosen at runtime.
/// * [`Traits::REFLECTS_COMMAND_NAMES`] (command or subcommand
///   level) bars proc renaming, as does any computed command name
///   (`inv.indirect`).
/// * [`Traits::INTROSPECTS_BY_NAME`] / [`Traits::TARGETS_VARIABLE_BY_NAME`]
///   subcommands (`info locals` / `info exists` / `trace add
///   variable`) bar the containing scope and the global scope; a
///   subcommand that *also* reflects command names (`info args PROC`
///   — another proc's parameter list) bars every scope.
/// * A **dynamic** subcommand word on a command that has any flagged
///   subcommand is assumed to be the worst-case subcommand.
fn find_rename_barriers(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    identities: &tcl_compiler::head_identity::HeadIdentityMap,
    include_global: bool,
) -> RenameBarriers {
    let mut out = RenameBarriers::default();
    let scope_at =
        |offset: u32| scope_label_at_offset(&analysis.global_scope, offset, "::", include_global);
    for inv in &analysis.command_invocations {
        if inv.indirect {
            // A computed command head can spell any proc name at runtime.
            out.procs = true;
            continue;
        }
        // The head's *effective command identity*: an observability trait
        // belongs to the command a head really names.  A proven
        // `interp alias {} peek {} upvar` still bars every variable scope, and
        // a `proc upvar …` that takes the name over does not (issue #1275).
        // The invocation carries its own absolute offset, so this is the
        // positioned read.
        let written = inv.name.trim_start_matches(':');
        let head = identities.head_words(written, inv.range.start()).resolved;
        let Some(spec) = registry.get(head) else {
            continue;
        };
        if spec.traits.contains(Traits::CREATES_DYNAMIC_BARRIER)
            && let Some(label) = scope_at(inv.range.start())
        {
            out.scopes.insert(label);
        }
        if spec.traits.contains(Traits::CREATES_SCOPE_ALIAS) {
            out.global_variables = true;
        }
        if spec.traits.contains(Traits::ALIASES_CALLER_FRAME)
            || spec.traits.contains(Traits::EVALUATES_IN_SHIFTED_FRAME)
        {
            out.all_variable_scopes = true;
        }
        if spec.traits.contains(Traits::REFLECTS_COMMAND_NAMES) {
            out.procs = true;
        }

        // Subcommand-level observability.
        let var_subs = Traits::INTROSPECTS_BY_NAME | Traits::TARGETS_VARIABLE_BY_NAME;
        let flagged: Vec<&tcl_registry::SubCommand> = spec
            .subcommands
            .iter()
            .filter(|s| {
                s.traits.intersects(var_subs) || s.traits.contains(Traits::REFLECTS_COMMAND_NAMES)
            })
            .collect();
        if flagged.is_empty() {
            continue;
        }
        let (reflects_vars, reflects_cmds) = match static_subcommand_word(source, inv) {
            Some(word) => {
                let hit = flagged.iter().find(|s| s.name == word);
                (
                    hit.is_some_and(|s| s.traits.intersects(var_subs)),
                    hit.is_some_and(|s| s.traits.contains(Traits::REFLECTS_COMMAND_NAMES)),
                )
            }
            // Dynamic subcommand word — assume the worst flagged one.
            None => (
                flagged.iter().any(|s| s.traits.intersects(var_subs)),
                flagged
                    .iter()
                    .any(|s| s.traits.contains(Traits::REFLECTS_COMMAND_NAMES)),
            ),
        };
        if reflects_cmds {
            out.procs = true;
        }
        if reflects_vars {
            if let Some(label) = scope_at(inv.range.start()) {
                out.scopes.insert(label);
            }
            // `info globals` (and a dynamic `info $sub`) can enumerate the
            // global frame from anywhere.
            out.global_variables = true;
            if reflects_cmds {
                // `info args PROC` / `info default PROC` reflect *another*
                // proc's parameter names — any scope may be observed.
                out.all_variable_scopes = true;
            }
        }
    }
    out
}

/// Next short name avoiding existing and claimed names.
fn next_unused_name(
    r#gen: &mut NameGenerator,
    existing: &FxHashSet<String>,
    claimed: &FxHashSet<String>,
) -> Option<String> {
    for _ in 0..1000 {
        let short = r#gen.next_name();
        if !existing.contains(&short) && !claimed.contains(&short) {
            return Some(short);
        }
    }
    None
}

/// Rename parameter *names* within the proc's parameter-list region.
///
/// Only each parameter's name is renamed — the first word of a
/// `{name default}` pair, or a bare `name`. A raw word-boundary byte scan over
/// the whole region also rewrote occurrences inside *other* parameters' default
/// values (`proc f {{x 1} {y x}}` renamed `y`'s default `x` too), changing the
/// default the proc receives.
fn rename_params_in_list(
    source: &str,
    proc_def: &ProcDef,
    var_map: &BTreeMap<String, String>,
    edits: &mut Vec<Edit>,
) {
    let region_start = proc_def.name_span.end() as usize;
    let region_end = proc_def.body_span.start() as usize;
    if region_start > region_end || region_end > source.len() {
        return;
    }
    let Some(region) = source.get(region_start..region_end) else {
        return;
    };
    let bytes = region.as_bytes();
    let is_ws = |b: u8| matches!(b, b' ' | b'\t' | b'\n' | b'\r');
    let is_sep = |b: u8| is_ws(b) || matches!(b, b'{' | b'}');

    // A bare (unbraced) param list is a single word (`proc f args …`).
    let Some(open) = bytes.iter().position(|&b| b == b'{') else {
        let trimmed_start = bytes.iter().position(|&b| !is_ws(b));
        if let Some(s) = trimmed_start {
            let mut e = s;
            while e < bytes.len() && !is_sep(bytes[e]) {
                e += 1;
            }
            maybe_rename(source, region_start, s, e - s, var_map, edits);
        }
        return;
    };

    // Braced list: walk top-level elements between the outer `{ … }`.
    let mut i = open + 1;
    while i < bytes.len() {
        while i < bytes.len() && is_ws(bytes[i]) {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b'}' {
            break;
        }
        if bytes[i] == b'{' {
            // `{name default…}` — the name is the first inner word.
            let mut j = i + 1;
            while j < bytes.len() && is_ws(bytes[j]) {
                j += 1;
            }
            let name_start = j;
            while j < bytes.len() && !is_sep(bytes[j]) {
                j += 1;
            }
            maybe_rename(
                source,
                region_start,
                name_start,
                j - name_start,
                var_map,
                edits,
            );
            // Skip to the end of this balanced braced element.
            let mut depth = 1u32;
            let mut k = i + 1;
            while k < bytes.len() && depth > 0 {
                match bytes[k] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                k += 1;
            }
            i = k;
        } else {
            // Bare-word param (no default).
            let name_start = i;
            while i < bytes.len() && !is_sep(bytes[i]) {
                i += 1;
            }
            maybe_rename(
                source,
                region_start,
                name_start,
                i - name_start,
                var_map,
                edits,
            );
        }
    }
}

/// Emit a rename edit for the param name at `region_start + rel_off` (length
/// `len`) when it is in `var_map`.
fn maybe_rename(
    source: &str,
    region_start: usize,
    rel_off: usize,
    len: usize,
    var_map: &BTreeMap<String, String>,
    edits: &mut Vec<Edit>,
) {
    if len == 0 {
        return;
    }
    let abs = region_start + rel_off;
    if let Some(name) = source.get(abs..abs + len)
        && let Some(short) = var_map.get(name)
    {
        edits.push((abs, len, short.clone()));
    }
}

/// Whether `b` is `[A-Za-z0-9_]`.
fn is_word_byte(b: Option<u8>) -> bool {
    matches!(b, Some(c) if c.is_ascii_alphanumeric() || c == b'_')
}

/// Whether a following byte would extend a bare `$name` reference, so a
/// preceding `${name}` cannot safely drop its braces.
///
/// Beyond the `[A-Za-z0-9_]` name characters this includes `(` (an array
/// index — `${x}(k)` is scalar `$x` + literal `(k)`, but `$x(k)` is an array
/// element) and `:` (a namespace separator — `$x` + `::y` vs the variable
/// `x::y`). Keeping the braces when any of these follow is always safe;
/// dropping them changes the reference.
fn extends_dollar_ref(b: Option<u8>) -> bool {
    is_word_byte(b) || matches!(b, Some(b'(' | b':'))
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

/// Call sites of the proc `name` / `qualified_name`.
fn find_proc_call_sites(name: &str, qualified_name: &str, analysis: &AnalysisResult) -> Vec<Span> {
    let qn_no_prefix = qualified_name.strip_prefix("::").unwrap_or(qualified_name);
    let mut out = Vec::new();
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    for inv in &analysis.command_invocations {
        // An indirect site (constant `$cmd` head, M7) carries no written name
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
/// serialization, not a private compiler symbol (issue #1192).
fn compact_names(
    source: &str,
    dialect: &str,
    isolated: bool,
    registry: &CommandRegistry,
) -> (String, SymbolMap) {
    let analysis = Analyser::new().analyse(source, dialect).clone();
    let mut symbol_map = SymbolMap::default();
    let mut edits: Vec<Edit> = Vec::new();

    let identities =
        tcl_compiler::head_identity::command_head_identities(source, dialect, registry);
    let barriers = find_rename_barriers(source, &analysis, registry, &identities, isolated);
    let rmw_targets = rmw_target_var_names(source, dialect, registry);
    let builtin_names: FxHashSet<&str> = registry.command_names().collect();

    let scope_ctx = ScopeCtx {
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
        },
    );

    // Proc renaming.  A proc name is a *public command identity* —
    // observable via `info procs` / `info commands`, `rename`,
    // `namespace export`, `unknown`, traces, and callable by code
    // outside this script — so it is renamed only when the caller
    // asserts a closed world (`isolated`) AND no command-name
    // reflection or computed command name is present (issue #1193).
    if isolated && !barriers.procs {
        let mut proc_gen = NameGenerator::new();
        let mut used_proc_names: FxHashSet<String> = FxHashSet::default();
        let mut proc_keys: Vec<&String> = analysis.all_procs.keys().collect();
        proc_keys.sort();
        for qname in proc_keys {
            let proc_def = &analysis.all_procs[qname];
            let name = &proc_def.name;
            if name.len() <= 1 || name.contains("::") {
                continue;
            }
            // A proc that overrides a registry-known command (`proc unknown
            // …`, an `auto_*` replacement, a shadowed builtin) is invoked by
            // the interpreter or library through that exact spelling — its
            // name is load-bearing.
            if builtin_names.contains(name.as_str()) {
                continue;
            }
            let mut short = proc_gen.next_name();
            while builtin_names.contains(short.as_str()) || used_proc_names.contains(&short) {
                short = proc_gen.next_name();
            }
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
            for call in find_proc_call_sites(name, &proc_def.qualified_name, &analysis) {
                let call_text = slice(source, call);
                let key = (call.start() as usize, call_text.len());
                if key != def_key && call_text == *name {
                    edits.push((call.start() as usize, call_text.len(), short.clone()));
                }
            }
            symbol_map.procs.insert(name.clone(), short);
        }
    }

    let result = apply_edits(source, edits);
    (result, symbol_map)
}

/// Read-only context for the recursive scope rename walk: the document
/// `source`, the analyser result, the `isolated` flag, and the precomputed
/// rename-barrier / read-modify-write-target sets.
#[derive(Clone, Copy)]
struct ScopeCtx<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    isolated: bool,
    barriers: &'a RenameBarriers,
    rmw_targets: &'a FxHashSet<String>,
}

/// Mutable outputs accumulated by [`process_scope`]: the symbol map and the
/// edit list.
struct ScopeOut<'a> {
    symbol_map: &'a mut SymbolMap,
    edits: &'a mut Vec<Edit>,
}

/// Recursively rename variables (and params) in a scope, mirroring
/// `_process_scope`.
fn process_scope(ctx: ScopeCtx<'_>, scope: &Scope, scope_label: &str, out: &mut ScopeOut<'_>) {
    let ScopeCtx {
        source,
        analysis,
        isolated,
        barriers,
        rmw_targets,
    } = ctx;
    let rename_scope = (scope.kind == ScopeKind::Proc
        || (isolated && scope.kind == ScopeKind::Global))
        && barriers.allows_scope(scope_label);

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

        let mut var_gen = NameGenerator::new();
        let existing: FxHashSet<String> = scope.variables.keys().cloned().collect();
        let mut var_map: BTreeMap<String, String> = BTreeMap::new();

        let mut var_names: Vec<&String> = scope.variables.keys().collect();
        var_names.sort();
        for var_name in var_names {
            let var_def = &scope.variables[var_name];
            if var_name.len() <= 1 || var_name.contains("::") {
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
            let claimed: FxHashSet<String> = var_map.values().cloned().collect();
            let Some(short) = next_unused_name(&mut var_gen, &existing, &claimed) else {
                continue;
            };
            if short.len() >= var_name.len() {
                continue;
            }
            let is_param = param_names.contains(var_name.as_str());

            // Definition site (non-params only — param defs point at
            // the proc-name token).
            if !is_param {
                let r = var_def.definition_span;
                if slice(source, r) == *var_name {
                    out.edits
                        .push((r.start() as usize, var_name.len(), short.clone()));
                }
            }
            // Reference sites.  A reference is either a `$var` read (skip
            // the `$`) or a **bare name word** — a re-definition (`set x 2`
            // twice), a registry `VarRead`/`VarWrite`-role argument
            // (`unset x`, `lappend x …`, `catch {…} x`).  Bare sites must
            // be rewritten in lock-step: renaming the declaration and the
            // `$` reads while leaving `set x 2` / `unset x` spelled with
            // the old name silently splits one variable into two
            // (pre-#1193 corruption: `set v 1;set v 2;return $v` returned
            // 1 after compaction).
            for &reference in &var_def.references {
                let ref_text = slice(source, reference);
                if let Some(rest) = ref_text.strip_prefix('$') {
                    if rest == var_name {
                        out.edits.push((
                            reference.start() as usize + 1,
                            var_name.len(),
                            short.clone(),
                        ));
                    }
                } else if ref_text == var_name {
                    out.edits
                        .push((reference.start() as usize, var_name.len(), short.clone()));
                }
            }
            var_map.insert(var_name.clone(), short);
        }

        if let Some(pd) = proc_def
            && !var_map.is_empty()
        {
            rename_params_in_list(source, pd, &var_map, out.edits);
        }
        if !var_map.is_empty() {
            out.symbol_map
                .variables
                .insert(scope_label.to_owned(), var_map);
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
    source: &str,
    dialect: &str,
    registry: &CommandRegistry,
) -> FxHashSet<String> {
    let cu = CompilationUnit::build_for_dialect(source, registry, false, dialect);
    let mut names = FxHashSet::default();
    let mut units: Vec<&FunctionUnit> = vec![&cu.top_level];
    units.extend(cu.procedures.values());
    for fu in units {
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                match stmt {
                    Statement::Incr { name, .. } => {
                        names.insert(name.clone());
                    }
                    Statement::Call { command, defs, .. }
                        if registry.rmw_first_arg_variable(command)
                            || registry
                                .get(command)
                                .is_some_and(|s| s.traits.contains(Traits::DESTROYS_VARIABLE)) =>
                    {
                        names.extend(defs.iter().cloned());
                    }
                    _ => {}
                }
            }
        }
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
    tcl_registry::traits::CLAUSE_KEYWORDS_WITHOUT_COMMAND_SPEC.contains(&word)
        || tcl_registry::traits::CLAUSE_NOISE_KEYWORDS.contains(&word)
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
/// the aggressive tier's alias generators (issue #1194): the
/// analyser's per-scope variable tables (recursively), the SSA
/// symbol tables of every function unit (which include names only
/// *read* through name-taking commands, e.g. `[set a]`), and every
/// textual `$name` / `${name}` reference.  Conservative superset —
/// an alias name is rejected on any hit.
fn collect_live_names(source: &str, dialect: &str, registry: &CommandRegistry) -> HashSet<String> {
    let mut out: HashSet<String> = HashSet::new();

    // Analyser scope tables.
    let analysis = Analyser::new().analyse(source, dialect).clone();
    let mut stack: Vec<&Scope> = vec![&analysis.global_scope];
    while let Some(scope) = stack.pop() {
        out.extend(scope.variables.keys().cloned());
        stack.extend(scope.children.iter());
    }

    // SSA symbol tables (per function unit).
    let cu = CompilationUnit::build_for_dialect(source, registry, false, dialect);
    let mut units: Vec<&FunctionUnit> = vec![&cu.top_level];
    units.extend(cu.procedures.values());
    for fu in units {
        out.extend(fu.ssa.var_names().iter().cloned());
    }

    // Textual `$name` / `${name}` references.
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let (end, name) = parse_var_ref(source, i);
            if let Some(name) = name {
                out.insert(name.to_owned());
                i = end.max(i + 1);
                continue;
            }
        }
        i += 1;
    }
    out
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
    let mut r#gen = NameGenerator::new();
    let mut aliases: Vec<(String, String)> = Vec::new();
    for name in &cands {
        let count = uses[name].len();
        if count < 2 {
            continue;
        }
        let mut alias = r#gen.next_name();
        while claimed.contains(&alias) {
            alias = r#gen.next_name();
        }
        let original_cost = count * name.len();
        let preamble_cost = 4 + alias.len() + 1 + name.len() + 1;
        let aliased_cost = preamble_cost + count * (alias.len() + 1);
        if aliased_cost >= original_cost {
            continue;
        }
        claimed.insert(alias.clone());
        aliases.push((name.clone(), alias));
    }
    if aliases.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }
    let mut edits: Vec<Edit> = Vec::new();
    let mut preamble = String::new();
    let mut map = BTreeMap::new();
    for (name, alias) in &aliases {
        let _ = writeln!(preamble, "set {alias} {name}");
        for &off in &uses[name] {
            edits.push((off, name.len(), format!("${alias}")));
        }
        map.insert(name.clone(), alias.clone());
    }
    let body = apply_edits(source, edits);
    (format!("{preamble}{body}"), map)
}

/// Phase 2.5: alias repeated long command names (`HTTP::uri` → `$a`).
fn alias_repeated_commands(
    source: &str,
    dialect: &str,
    claimed: &mut HashSet<String>,
    registry: &CommandRegistry,
) -> (String, BTreeMap<String, String>) {
    let analysis = Analyser::new().analyse(source, dialect).clone();
    let mut order: Vec<String> = Vec::new();
    let mut uses: HashMap<String, Vec<usize>> = HashMap::new();
    for inv in &analysis.command_invocations {
        let name = &inv.name;
        if inv.indirect || name.len() <= 2 || registry.is_byte_compiled(name) {
            continue;
        }
        uses.entry(name.clone())
            .or_insert_with(|| {
                order.push(name.clone());
                Vec::new()
            })
            .push(inv.range.start() as usize);
    }
    alias_by_uses(source, &order, &uses, claimed)
}

/// Phase 2.8: emit unique-prefix keyword abbreviations (#1230).
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
fn abbreviate_keywords(source: &str, dialect: &str, registry: &CommandRegistry) -> (String, usize) {
    let identities =
        tcl_compiler::head_identity::command_head_identities(source, dialect, registry);
    let mut edits: Vec<Edit> = Vec::new();
    let mut stack: Vec<(String, u32)> = vec![(source.to_owned(), 0)];
    let later = later_core_registries(dialect);
    while let Some((text, base)) = stack.pop() {
        let sm = SourceMap::new(&text);
        let Ok(tokens) = Lexer::new(&text).tokenise_all() else {
            continue;
        };
        for command in command_word_runs(&sm, &tokens) {
            // Recurse into braced/bracketed words so nested scripts get the
            // same treatment.
            for word in &command {
                if matches!(word.kind, TokenType::Str | TokenType::Cmd) {
                    let inner = sm.token_text(word.token);
                    if inner.len() >= 3 {
                        stack.push((inner.to_owned(), base + word.token.span.start() + 1));
                    }
                }
            }
            abbreviate_command(
                &command,
                registry,
                &later,
                AbbrevSite {
                    base,
                    identities: &identities,
                },
                &mut edits,
            );
        }
    }
    if edits.is_empty() {
        return (source.to_owned(), 0);
    }
    let count = edits.len();
    (apply_edits(source, edits), count)
}

/// Where an abbreviation candidate sits, and what the document has proven
/// about the command it belongs to.
///
/// Bundled so [`abbreviate_command`] keeps a small signature — the scan
/// re-enters nested braced words with a shifted `base`, and both fields travel
/// together.
#[derive(Clone, Copy)]
struct AbbrevSite<'a> {
    /// Byte offset of the scanned slice within the whole document, so a head's
    /// own span resolves to a document-absolute offset.
    base: u32,
    /// The document's proven command-identity facts.
    identities: &'a tcl_compiler::head_identity::HeadIdentityMap,
}

/// One word of a command, with the token it came from.
struct CommandWord {
    token: Token,
    kind: TokenType,
    text: String,
    /// The word is a substitution / expansion — never rewritten.
    dynamic: bool,
}

/// Split a token stream into per-command word runs.
fn command_word_runs(sm: &SourceMap, tokens: &[Token]) -> Vec<Vec<CommandWord>> {
    let mut out: Vec<Vec<CommandWord>> = Vec::new();
    let mut current: Vec<CommandWord> = Vec::new();
    let mut expand_next = false;
    for tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Eol => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
                expand_next = false;
            }
            TokenType::Sep => {}
            TokenType::Expand => expand_next = true,
            kind => {
                let dynamic =
                    expand_next || matches!(kind, TokenType::Var | TokenType::Cmd | TokenType::Str);
                current.push(CommandWord {
                    token: *tok,
                    kind,
                    text: sm.token_text(*tok).to_owned(),
                    dynamic,
                });
                expand_next = false;
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// The core-Tcl registries for every release *after* `dialect`, so a prefix
/// that a later Tcl makes ambiguous is never emitted.
///
/// The range and its packs come from
/// [`tcl_registry::version_range`] — the one helper the formatter and the
/// analyser share — rather than a release list kept here (issue #1257). The
/// target's own pack is dropped because the caller already holds it and
/// [`keyword_tables`] puts it first.
fn later_core_registries(dialect: &str) -> Vec<&'static CommandRegistry> {
    use tcl_registry::version_range::{forward_range, registries_over_range};
    let mut packs = registries_over_range(forward_range(dialect));
    if !packs.is_empty() {
        packs.remove(0);
    }
    packs
}

/// Shorten every abbreviable keyword word of one command.
fn abbreviate_command(
    words: &[CommandWord],
    registry: &CommandRegistry,
    later: &[&'static CommandRegistry],
    site: AbbrevSite<'_>,
    edits: &mut Vec<Edit>,
) {
    let AbbrevSite { base, identities } = site;
    let Some(head) = words.first() else { return };
    if head.dynamic || head.kind != TokenType::Esc {
        return;
    }
    // Which subcommands and options a head has is registry data about the
    // command it *is*: abbreviating `myfmt`'s words under `format`'s tables
    // when `myfmt` is not `format` would rewrite live text (issue #1275).
    // `base` makes the head's offset document-absolute even inside a
    // recursively-scanned braced word, so this is the positioned read.
    let head_name = identities
        .head_words(&head.text, base + head.token.span.start())
        .resolved;
    let Some(spec) = registry.get(head_name) else {
        return;
    };
    let args = &words[1..];
    let mut subcommand: Option<&'static str> = None;
    let mut start = 0usize;
    if !spec.subcommands.is_empty() {
        let Some(word) = args.first() else { return };
        if word.dynamic || word.kind != TokenType::Esc {
            return;
        }
        let Some(canonical) = spec
            .resolve_subcommand_word(&word.text, None, None, None)
            .unique()
        else {
            return;
        };
        let tables = keyword_tables(head_name, TableScope::Subcommands, registry, later);
        if let Some(short) = shortest_spelling(&tables, canonical)
            && short.len() < word.text.len()
        {
            push_word_edit(word, base, short, edits);
        }
        subcommand = Some(canonical);
        start = 1;
    }
    let scope = subcommand.map_or(TableScope::CommandOptions, TableScope::SubcommandOptions);
    let option_tables = keyword_tables(head_name, scope, registry, later);
    if option_tables.iter().all(KeywordTable::is_empty) {
        return;
    }
    for word in &args[start.min(args.len())..] {
        if word.text == "--" {
            break;
        }
        if word.dynamic || word.kind != TokenType::Esc || !word.text.starts_with('-') {
            continue;
        }
        let Some(canonical) = option_tables
            .first()
            .and_then(|t| t.names().find(|n| *n == word.text))
        else {
            continue;
        };
        if let Some(short) = shortest_spelling(&option_tables, canonical)
            && short.len() < word.text.len()
        {
            push_word_edit(word, base, short, edits);
        }
    }
}

fn push_word_edit(word: &CommandWord, base: u32, short: &str, edits: &mut Vec<Edit>) {
    let start = (base + word.token.span.start()) as usize;
    edits.push((start, word.text.len(), short.to_owned()));
}

/// The keyword tables for `cmd` in the target registry followed by every
/// later core-Tcl registry — the subcommand table when `subcommand` is
/// `None`, otherwise that subcommand's option table.
///
/// The target's table is always first; the rest are what the version-range
/// check consults. A later release that no longer carries the command (or
/// the subcommand) contributes an empty table, which can never vouch for a
/// prefix, so the abbreviation is abandoned.
fn keyword_tables(
    cmd: &str,
    scope: TableScope<'_>,
    registry: &CommandRegistry,
    later: &[&'static CommandRegistry],
) -> Vec<KeywordTable<'static>> {
    let table_for = |reg: &CommandRegistry| -> KeywordTable<'static> {
        let empty = KeywordTable::new(std::iter::empty(), PrefixMatching::Enabled);
        let Some(spec) = reg.get(cmd) else {
            return empty;
        };
        match scope {
            TableScope::Subcommands => spec.subcommand_table(None, None, None),
            TableScope::CommandOptions => spec.option_table(None, None, None),
            TableScope::SubcommandOptions(name) => spec
                .subcommands
                .iter()
                .find(|s| s.name == name)
                .map_or(empty, |sub| sub.option_table(None, None, None)),
        }
    };
    std::iter::once(table_for(registry))
        .chain(later.iter().map(|reg| table_for(reg)))
        .collect()
}

/// Which keyword table of a command [`keyword_tables`] should build.
#[derive(Debug, Clone, Copy)]
enum TableScope<'a> {
    /// The ensemble's subcommand words.
    Subcommands,
    /// The command's own option words (a command with no subcommands).
    CommandOptions,
    /// The named subcommand's option words.
    SubcommandOptions(&'a str),
}

/// The shortest spelling of `canonical` that resolves to it in **every**
/// table, or `None` when no abbreviation is safe across the whole range.
fn shortest_spelling(
    tables: &[KeywordTable<'static>],
    canonical: &'static str,
) -> Option<&'static str> {
    let short = tables.first()?.minimal_unique_prefix(canonical)?;
    tables[1..]
        .iter()
        .all(|t| t.resolve(short).unique() == Some(canonical))
        .then_some(short)
}

/// Phase 2.6: alias repeated literal arguments (`-normalized` → `$a`).
fn alias_repeated_arguments(
    source: &str,
    claimed: &mut HashSet<String>,
) -> (String, BTreeMap<String, String>) {
    let mut order: Vec<String> = Vec::new();
    let mut uses: HashMap<String, Vec<usize>> = HashMap::new();
    let mut stack: Vec<(String, u32)> = vec![(source.to_owned(), 0)];
    while let Some((text, base)) = stack.pop() {
        let sm = SourceMap::new(&text);
        let Ok(tokens) = Lexer::new(&text).tokenise_all() else {
            continue;
        };
        let mut is_command_word = true;
        let mut in_quoted = false;
        for tok in &tokens {
            match tok.kind {
                TokenType::Eof => break,
                TokenType::Eol => {
                    is_command_word = true;
                    in_quoted = false;
                    continue;
                }
                TokenType::Sep => {
                    is_command_word = false;
                    in_quoted = false;
                    continue;
                }
                TokenType::Str => {
                    let inner = sm.token_text(*tok);
                    if inner.len() >= 3 {
                        stack.push((inner.to_owned(), base + tok.span.start() + 1));
                    }
                    is_command_word = false;
                    in_quoted = false;
                    continue;
                }
                TokenType::Cmd => {
                    let inner = sm.token_text(*tok);
                    if inner.len() >= 3 {
                        stack.push((inner.to_owned(), base + tok.span.start() + 1));
                    }
                    is_command_word = false;
                    continue;
                }
                _ => {}
            }
            if is_command_word {
                is_command_word = false;
                continue;
            }
            if tok.kind != TokenType::Esc {
                in_quoted = false;
                continue;
            }
            let abs_off = (base + tok.span.start()) as usize;
            if source.as_bytes().get(abs_off) == Some(&b'"') {
                in_quoted = true;
            }
            if in_quoted {
                continue;
            }
            let val = sm.token_text(*tok);
            if val.len() < 3
                || val.contains([' ', '\t', '\n', '"', '{', '}', '[', ']', '$', '\\', ';'])
                || is_clause_keyword(val)
            {
                continue;
            }
            uses.entry(val.to_owned())
                .or_insert_with(|| {
                    order.push(val.to_owned());
                    Vec::new()
                })
                .push(abs_off);
        }
    }
    alias_by_uses(source, &order, &uses, claimed)
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
fn string_alias_candidates(source: &str) -> BTreeMap<Vec<u8>, Vec<usize>> {
    let segments = collect_string_literals(source);
    if segments.is_empty() {
        return BTreeMap::new();
    }
    // Concatenate segments with a `\0` sentinel; map concat byte →
    // source byte offset (`usize::MAX` for sentinels).
    let mut concat: Vec<u8> = Vec::new();
    let mut concat_to_src: Vec<usize> = Vec::new();
    for &(src_offset, ref seg) in &segments {
        for (j, b) in seg.bytes().enumerate() {
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

/// Phase 2.7: alias repeated substrings inside double-quoted
/// strings. (+ `_collect_string_literals`).
fn alias_string_literals(
    source: &str,
    claimed: &mut HashSet<String>,
) -> (String, BTreeMap<String, String>) {
    let occ = string_alias_candidates(source);
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

    let src_bytes = source.as_bytes();
    let mut r#gen = NameGenerator::new();
    let mut claimed_bytes: HashSet<usize> = HashSet::new();
    let mut aliases: Vec<(Vec<u8>, String, Vec<usize>)> = Vec::new();
    for (_, sub, offsets) in scored {
        if !braces_balanced(&sub) {
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
        let mut alias = r#gen.next_name();
        while claimed.contains(&alias) {
            alias = r#gen.next_name();
        }
        let count = free.len();
        let original_cost = count * sub.len();
        let preamble_cost = 4 + alias.len() + 1 + 1 + sub.len() + 1 + 1;
        let mut aliased_cost = preamble_cost;
        for &off in &free {
            let end = off + sub.len();
            if extends_dollar_ref(src_bytes.get(end).copied()) {
                aliased_cost += alias.len() + 3;
            } else {
                aliased_cost += alias.len() + 1;
            }
        }
        if aliased_cost >= original_cost {
            continue;
        }
        claimed.insert(alias.clone());
        for &off in &free {
            for p in off..off + sub.len() {
                claimed_bytes.insert(p);
            }
        }
        aliases.push((sub, alias, free));
    }
    if aliases.is_empty() {
        return (source.to_owned(), BTreeMap::new());
    }

    // Step 7: build preamble + edits.
    let mut preamble = String::new();
    let mut edits: Vec<Edit> = Vec::new();
    let mut map = BTreeMap::new();
    for (sub, alias, offsets) in &aliases {
        let sub_str = String::from_utf8_lossy(sub).into_owned();
        let _ = writeln!(preamble, "set {alias} {{{sub_str}}}");
        for &off in offsets {
            let end = off + sub.len();
            let replacement = if extends_dollar_ref(src_bytes.get(end).copied()) {
                format!("${{{alias}}}")
            } else {
                format!("${alias}")
            };
            edits.push((off, sub.len(), replacement));
        }
        map.insert(sub_str, alias.clone());
    }
    let body = apply_edits(source, edits);
    (format!("{preamble}{body}"), map)
}

/// Collect `(abs_offset, text)` of `ESC` segments inside
/// double-quoted strings, descending into braced / command-subst
/// tokens.
fn collect_string_literals(top_source: &str) -> Vec<(usize, String)> {
    let mut segments: Vec<(usize, String)> = Vec::new();
    let mut stack: Vec<(String, u32)> = vec![(top_source.to_owned(), 0)];
    while let Some((text, base)) = stack.pop() {
        let sm = SourceMap::new(&text);
        let Ok(tokens) = Lexer::new(&text).tokenise_all() else {
            continue;
        };
        let mut is_command_word = true;
        let mut in_quoted = false;
        for tok in &tokens {
            match tok.kind {
                TokenType::Eof => break,
                TokenType::Eol => {
                    is_command_word = true;
                    in_quoted = false;
                    continue;
                }
                TokenType::Sep => {
                    is_command_word = false;
                    in_quoted = false;
                    continue;
                }
                TokenType::Str => {
                    let inner = sm.token_text(*tok);
                    if inner.len() >= 2 {
                        stack.push((inner.to_owned(), base + tok.span.start() + 1));
                    }
                    is_command_word = false;
                    in_quoted = false;
                    continue;
                }
                TokenType::Cmd => {
                    let inner = sm.token_text(*tok);
                    if inner.len() >= 2 {
                        stack.push((inner.to_owned(), base + tok.span.start() + 1));
                    }
                    is_command_word = false;
                    continue;
                }
                _ => {}
            }
            let mut abs_off = (base + tok.span.start()) as usize;
            if is_command_word {
                is_command_word = false;
                in_quoted = false;
                continue;
            }
            if !in_quoted && top_source.as_bytes().get(abs_off) == Some(&b'"') {
                in_quoted = true;
                abs_off += 1;
            }
            if in_quoted && tok.kind == TokenType::Esc {
                let inner = sm.token_text(*tok);
                if inner.len() >= 2 {
                    segments.push((abs_off, inner.to_owned()));
                }
            }
            if !matches!(tok.kind, TokenType::Esc | TokenType::Var | TokenType::Cmd) {
                in_quoted = false;
            }
        }
    }
    segments
}

/// Whether `s` has equal numbers of `{` and `}` bytes.
fn braces_balanced(s: &[u8]) -> bool {
    let mut balance: i64 = 0;
    for &c in s {
        match c {
            b'{' => balance += 1,
            b'}' => balance -= 1,
            _ => {}
        }
    }
    balance == 0
}

/// Whether braces in `s` are properly nested (depth never negative,
/// ends at zero).
fn braces_nested(s: &str) -> bool {
    let mut depth: i64 = 0;
    for c in s.bytes() {
        match c {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

/// First index ≥ `from` where `needle` occurs in `haystack`.
fn find_subslice(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || from + needle.len() > haystack.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

// Static-substring folding (aggressive phase 1.5)

/// Taint colours that prove a fixed-form value, so folding stays
/// safe even when the underlying value is tainted.
fn safe_taint_colours() -> TaintColour {
    TaintColour::IP_ADDRESS
        | TaintColour::PORT
        | TaintColour::FQDN
        | TaintColour::LIST_CANONICAL
        | TaintColour::REGEX_LITERAL
}

/// Replace dynamic quoted strings with their static values where the
/// compiler's SCCP pass proves every `$var` substitution is a
/// compile-time constant and no unsanitised tainted value is
/// involved.  Folds `$var` interpolations resolving to integer /
/// string constants; bails on command substitutions (`[…]`) and on
/// boolean / float constants.  Returns `(folded_source, fold_count,
/// fold_map)`.
fn fold_static_substrings(
    source: &str,
    dialect: &str,
    registry: &CommandRegistry,
) -> (String, usize, BTreeMap<String, String>) {
    let cu = CompilationUnit::build_for_dialect(source, registry, false, dialect);
    let mut edits: Vec<Edit> = Vec::new();
    let mut fold_map: BTreeMap<String, String> = BTreeMap::new();

    let mut scopes: Vec<&FunctionUnit> = vec![&cu.top_level];
    scopes.extend(cu.procedures.values());
    for fu in scopes {
        collect_folds_for_scope(source, fu, &mut edits, &mut fold_map);
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

/// Collect static-fold edits for one function scope.
fn collect_folds_for_scope(
    source: &str,
    fu: &FunctionUnit,
    edits: &mut Vec<Edit>,
    fold_map: &mut BTreeMap<String, String>,
) {
    for (block_id, block) in &fu.cfg.blocks {
        let Some(ssa_block) = fu.ssa.blocks.get(block_id) else {
            continue;
        };
        if !fu.sccp.executable_blocks.contains(block_id) {
            continue;
        }
        let mut block_vars: HashMap<String, Version> = ssa_block
            .entry_versions
            .iter()
            .map(|(sym, ver)| (fu.ssa.var_name(*sym).to_owned(), *ver))
            .collect();
        for (stmt_idx, stmt) in block.statements.iter().enumerate() {
            let Some(ssa_stmt) = ssa_block.statements.get(stmt_idx) else {
                continue;
            };
            let mut uses = block_vars.clone();
            for (name, ver) in &ssa_stmt.uses {
                uses.insert(fu.ssa.var_name(*name).to_owned(), *ver);
            }
            match stmt {
                Statement::AssignValue {
                    span,
                    value,
                    value_needs_backsubst: true,
                    ..
                } if value.contains('$') || value.contains('[') => {
                    try_fold_region(source, *span, value, &uses, fu, edits, fold_map);
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
                            fold_string_via_sccp(content, &uses, &fu.sccp.values, &fu.ssa)
                        {
                            if has_unsafe_tainted_inputs(content, &uses, &fu.taints, &fu.ssa) {
                                continue;
                            }
                            if let Some(close) = close_quote_offset(source, arg_off) {
                                edits.push((
                                    arg_off,
                                    close - arg_off + 1,
                                    build_replacement(&folded),
                                ));
                                fold_map.insert(content.clone(), folded);
                            }
                        }
                    }
                }
                _ => {}
            }
            for (name, ver) in &ssa_stmt.defs {
                block_vars.insert(fu.ssa.var_name(*name).to_owned(), *ver);
            }
        }
    }
}

/// Fold the quoted string inside the `span` region of an
/// `AssignValue` (`set x "…"`).
fn try_fold_region(
    source: &str,
    span: Span,
    value: &str,
    uses: &HashMap<String, Version>,
    fu: &FunctionUnit,
    edits: &mut Vec<Edit>,
    fold_map: &mut BTreeMap<String, String>,
) {
    let Some(folded) = fold_string_via_sccp(value, uses, &fu.sccp.values, &fu.ssa) else {
        return;
    };
    if has_unsafe_tainted_inputs(value, uses, &fu.taints, &fu.ssa) {
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
    edits.push((abs_start, close - abs_start + 1, build_replacement(&folded)));
    fold_map.insert(inner.to_owned(), folded);
}

/// Resolve a quoted-string body to a static value via SCCP, or
/// `None` when any substitution is non-constant.  `$var` only;
/// command substitutions (`[…]`) bail out.
fn fold_string_via_sccp(
    content: &str,
    uses: &HashMap<String, Version>,
    values: &HashMap<(tcl_compiler::ssa::Symbol, Version), LatticeValue>,
    ssa: &tcl_compiler::ssa::SsaFunction,
) -> Option<String> {
    let bytes = content.as_bytes();
    let n = bytes.len();
    let mut out = String::new();
    let mut has_dynamic = false;
    let mut pos = 0;
    while pos < n {
        match bytes[pos] {
            b'$' => {
                let (end, name) = parse_var_ref(content, pos);
                let name = name?;
                let ver = uses.get(name).copied().unwrap_or(0);
                if ver == 0 {
                    return None;
                }
                let lv = values.get(&(ssa.var_symbol(name)?, ver))?;
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
fn has_unsafe_tainted_inputs(
    content: &str,
    uses: &HashMap<String, Version>,
    taints: &HashMap<(tcl_compiler::ssa::Symbol, Version), TaintLattice>,
    ssa: &tcl_compiler::ssa::SsaFunction,
) -> bool {
    let safe = safe_taint_colours();
    let mut pos = 0;
    let bytes = content.as_bytes();
    while pos < bytes.len() {
        if bytes[pos] == b'$' {
            let (end, name) = parse_var_ref(content, pos);
            if let Some(name) = name {
                let ver = uses.get(name).copied().unwrap_or(0);
                if ver > 0
                    && let Some(sym) = ssa.var_symbol(name)
                    && let Some(t) = taints.get(&(sym, ver))
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

/// Parse a `$var` / `${var}` reference at `pos`, returning
/// `(end, name)`.  Rejects array (`$a(i)`) and namespaced
/// (`$a::b`) forms.
fn parse_var_ref(text: &str, pos: usize) -> (usize, Option<&str>) {
    let bytes = text.as_bytes();
    if pos >= bytes.len() || bytes[pos] != b'$' {
        return (pos + 1, None);
    }
    let start = pos + 1;
    if start >= bytes.len() {
        return (start, None);
    }
    if bytes[start] == b'{' {
        return match text[start + 1..].find('}') {
            Some(rel) => {
                let close = start + 1 + rel;
                (close + 1, Some(&text[start + 1..close]))
            }
            None => (start, None),
        };
    }
    let mut end = start;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    if end == start {
        return (start, None);
    }
    if end < bytes.len() {
        if bytes[end] == b'(' {
            return (start, None);
        }
        if bytes[end] == b':' && end + 1 < bytes.len() && bytes[end + 1] == b':' {
            return (start, None);
        }
    }
    (end, Some(&text[start..end]))
}

/// Build a replacement token for a folded static string: bare when
/// no quoting is needed, braced when safe, else escaped double
/// quotes.
fn build_replacement(folded: &str) -> String {
    let needs_quoting = folded.is_empty()
        || folded.contains([' ', '\t', '\n', '"', '{', '}', '[', ']', '$', '\\', ';']);
    if !needs_quoting {
        return folded.to_owned();
    }
    if !folded.contains('\\') && braces_nested(folded) {
        return format!("{{{folded}}}");
    }
    let escaped = folded
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('[', "\\[");
    format!("\"{escaped}\"")
}

/// Return the abbreviated subcommand text when safe for `dialect`.
fn abbreviated_subcommand(command_name: &str, subcommand_name: &str, dialect: &str) -> String {
    if !tcl_registry::prelude::DialectSet::has_fixed_ensembles(Some(dialect)) {
        return subcommand_name.to_owned();
    }
    subcommand_abbreviation(command_name, subcommand_name)
        .unwrap_or(subcommand_name)
        .to_owned()
}

/// Shortest unambiguous abbreviation for `sub` of ensemble
/// `command`, or `None`. (only the
/// entries strictly shorter than the full subcommand are kept).
fn subcommand_abbreviation(command: &str, sub: &str) -> Option<&'static str> {
    let table: &[(&str, &str)] = match command {
        "string" => &[
            ("bytelength", "b"),
            ("cat", "ca"),
            ("compare", "co"),
            ("equal", "e"),
            ("first", "f"),
            ("index", "in"),
            ("last", "la"),
            ("length", "le"),
            ("match", "mat"),
            ("range", "ra"),
            ("repeat", "repe"),
            ("replace", "repl"),
            ("reverse", "rev"),
            ("tolower", "tol"),
            ("totitle", "tot"),
            ("toupper", "tou"),
            ("trimleft", "triml"),
            ("trimright", "trimr"),
            ("wordend", "worde"),
            ("wordstart", "words"),
        ],
        "info" => &[
            ("args", "a"),
            ("body", "b"),
            ("cmdcount", "cm"),
            ("commands", "comm"),
            ("complete", "comp"),
            ("default", "d"),
            ("exists", "e"),
            ("frame", "fr"),
            ("functions", "fu"),
            ("globals", "g"),
            ("hostname", "h"),
            ("level", "le"),
            ("library", "li"),
            ("loaded", "loa"),
            ("locals", "loc"),
            ("nameofexecutable", "n"),
            ("patchlevel", "pa"),
            ("procs", "pr"),
            ("script", "sc"),
            ("sharedlibextension", "sh"),
            ("tclversion", "t"),
        ],
        "clock" => &[
            ("add", "a"),
            ("clicks", "c"),
            ("format", "f"),
            ("microseconds", "mic"),
            ("milliseconds", "mil"),
            ("scan", "sc"),
            ("seconds", "se"),
        ],
        _ => return None,
    };
    table
        .iter()
        .find(|(full, _)| *full == sub)
        .map(|(_, abbr)| *abbr)
}

/// Group a token stream into commands (lists of arguments),
/// dropping comments and whitespace.
fn parse_commands(source: &str, tokens: &[Token]) -> Vec<Vec<Arg>> {
    let mut commands: Vec<Vec<Arg>> = Vec::new();
    let mut current: Vec<Arg> = Vec::new();
    let mut prev_type = TokenType::Eol;

    for &tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Comment => continue,
            TokenType::Sep => {
                prev_type = TokenType::Sep;
                continue;
            }
            TokenType::Eol => {
                if !current.is_empty() {
                    commands.push(std::mem::take(&mut current));
                }
                prev_type = TokenType::Eol;
                continue;
            }
            _ => {}
        }

        let is_start = matches!(prev_type, TokenType::Sep | TokenType::Eol);
        let detected_quoted =
            is_start && source.as_bytes().get(tok.span.start() as usize) == Some(&b'"');

        if is_start || current.is_empty() {
            current.push(Arg {
                tokens: vec![tok],
                is_braced: tok.kind == TokenType::Str,
                is_quoted: detected_quoted,
            });
        } else {
            current.last_mut().expect("non-empty").tokens.push(tok);
        }
        prev_type = tok.kind;
    }
    if !current.is_empty() {
        commands.push(current);
    }
    commands
}

/// Render one command's arguments to their minified string forms.
fn render_command(sm: &SourceMap, cmd_args: &[Arg], env: MinifyEnv<'_>, depth: u32) -> Vec<String> {
    let registry = env.registry;
    let cmd_name = cmd_args
        .first()
        .map(|a| token_text(sm, a))
        .unwrap_or_default();
    // The head's *effective command identity*: which registry command the
    // spelling really names once the document's `namespace import` / `interp
    // alias` / `rename` / built-in-shadowing `proc` statements are folded in
    // (issue #1275).  Without it a rebound command's body / lambda /
    // expression / clause-list arguments were re-minified as the grammar of
    // the command it no longer is.
    let head = env.resolve(&cmd_name);
    let post: Vec<String> = cmd_args.iter().skip(1).map(|a| token_text(sm, a)).collect();
    let post_refs: Vec<&str> = post.iter().map(String::as_str).collect();

    let body_indices = role_indices(registry, head, &post_refs, ArgRole::Body);
    let lambda_indices = role_indices(registry, head, &post_refs, ArgRole::LambdaLiteral);
    let expr_indices = role_indices(registry, head, &post_refs, ArgRole::Expr);
    // The braced clause-list form of a registry `case_list` command
    // (`switch … { pat body … }`, Expect's `expect { … }`).  Registry
    // data, never a spelled command name (issue #1197).
    let case_list_spec = registry.get(head).and_then(|s| s.case_list);
    let dialect = tcl_dialect::DialectProfile::by_name(env.dialect).availability_mask;
    let case_invocation = registry.case_invocation(head, &post_refs, dialect);
    let clause_list_index = case_invocation
        .as_ref()
        .and_then(|(_, invocation)| invocation.clause_list_index)
        .map(|index| index + 1);
    let inline_body_indices: Vec<usize> = case_invocation
        .as_ref()
        .and_then(|(spec, invocation)| {
            invocation
                .inline_clause_start
                .and_then(|start| spec.inline_clauses(&post_refs, start))
        })
        .map(|clauses| {
            clauses
                .into_iter()
                .filter_map(|clause| clause.body_index.map(|index| index + 1))
                .collect()
        })
        .unwrap_or_default();

    let mut out: Vec<String> = Vec::with_capacity(cmd_args.len());
    for (i, arg) in cmd_args.iter().enumerate() {
        let single_braced = arg.is_braced && arg.tokens.len() == 1;
        if clause_list_index == Some(i) && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            let minified = case_list_spec.map_or_else(
                || minify_body(inner, env, depth + 1),
                |cl| minify_case_list(inner, cl, env, depth + 1),
            );
            out.push(format!("{{{minified}}}"));
        } else if (inline_body_indices.contains(&i) || body_indices.contains(&i)) && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            out.push(format!("{{{}}}", minify_body(inner, env, depth + 1)));
        } else if lambda_indices.contains(&i) && single_braced {
            out.push(minify_lambda_literal(sm, arg.tokens[0], env, depth + 1));
        } else if expr_indices.contains(&i) && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            out.push(format!("{{{}}}", compress_expr(inner, env, depth + 1)));
        } else {
            out.push(reconstruct_arg(sm, arg, env, depth));
        }
    }
    out
}

/// Minify an `ArgRole::LambdaLiteral` argument (`apply`'s `{argList body
/// ?ns?}` shape): only the body element is recursively minified as a
/// script; the parameter list and optional namespace elements are copied
/// through untouched (`ArgRole::ParamList` isn't itself minified anywhere in
/// this file — issue #954 only needs the body reached correctly, not new
/// parameter-list compaction). Re-segmenting the whole literal as a script
/// (the pre-`LambdaLiteral`-role behaviour) misread the parameter word as a
/// command name and never reached the real body at all.
///
/// Each element is decoded (backslash escapes collapsed for a bare/quoted
/// element — [`split_lambda_literal_decoded`]) before use and re-quoted with
/// [`tcl_syntax::list::list_element`] on reassembly, rather than pasted back
/// as raw source spelling wrapped in a bare `{}`: a non-literal body's
/// backslash escapes would otherwise survive into the "minified" text and
/// change what it runs (codex review of #954's follow-up — `apply {{}
/// puts\ hi}`'s real body is `puts hi`, not `puts\ hi`), and a multi-word
/// parameter list (`apply {{x y} …}`) would otherwise lose the braces that
/// group it into one list element.
fn minify_lambda_literal(sm: &SourceMap, tok: Token, env: MinifyEnv<'_>, depth: u32) -> String {
    let source = sm.source();
    let fallback = || format!("{{{}}}", sm.token_text(tok));
    let Some(elems) = split_lambda_literal_decoded(source, tok) else {
        return fallback();
    };
    let Some(body) = elems.body.as_deref() else {
        return fallback();
    };
    let minified_body = minify_body(body, env, depth);
    let mut parts = vec![
        tcl_syntax::list::list_element(&elems.params),
        tcl_syntax::list::list_element(&minified_body),
    ];
    if let Some(ns) = elems.namespace.as_deref() {
        parts.push(tcl_syntax::list::list_element(ns));
    }
    format!("{{{}}}", parts.join(" "))
}

/// Registry role indices, offset by 1 for the command-name slot.
fn role_indices(
    registry: &CommandRegistry,
    name: &str,
    post_args: &[&str],
    role: ArgRole,
) -> Vec<usize> {
    if name.is_empty() {
        return Vec::new();
    }
    registry
        .arg_indices_for_role(name, post_args, role)
        .into_iter()
        .map(|i| i + 1)
        .collect()
}

/// Text of an argument's first token.
fn token_text(sm: &SourceMap, arg: &Arg) -> String {
    arg.tokens
        .first()
        .map(|&t| sm.token_text(t).to_owned())
        .unwrap_or_default()
}

/// First character a token will render as.
fn first_rendered_char(sm: &SourceMap, tok: Token) -> Option<char> {
    match tok.kind {
        TokenType::Str | TokenType::Expand => Some('{'),
        TokenType::Cmd => Some('['),
        TokenType::Var => Some('$'),
        _ => sm.token_text(tok).chars().next(),
    }
}

/// Rebuild source text from a single token, re-adding delimiters
/// and recursively minifying `[…]` substitutions.
fn reconstruct_raw(
    sm: &SourceMap,
    tok: Token,
    next_tok: Option<Token>,
    env: MinifyEnv<'_>,
    in_quotes: bool,
    depth: u32,
) -> String {
    match tok.kind {
        // Inside a double-quoted word the caller re-wraps the arg in `"…"`, so a
        // `Str` token (e.g. a lone `$` classified as literal) is string data —
        // emit it verbatim, not brace-wrapped as `{$}`.
        TokenType::Str if in_quotes => sm.text(tok.span).to_owned(),
        TokenType::Str => format!("{{{}}}", sm.token_text(tok)),
        TokenType::Cmd => format!("[{}]", minify_body(sm.token_text(tok), env, depth + 1)),
        TokenType::Var => {
            // Keep `${var}` when the next token would otherwise extend the
            // variable name.  Beyond name characters, `(` (array index) and `:`
            // (namespace separator) also extend a `$` reference, so dropping
            // the braces before them changes the read (`${x}(k)` scalar-plus-
            // literal vs `$x(k)` array element).
            if let Some(next) = next_tok
                && let Some(c) = first_rendered_char(sm, next)
                && (c.is_alphanumeric() || c == '_' || c == '(' || c == ':')
            {
                return format!("${{{}}}", sm.token_text(tok));
            }
            format!("${}", sm.token_text(tok))
        }
        TokenType::Expand => "{*}".to_owned(),
        _ => sm.token_text(tok).to_owned(),
    }
}

/// Characters that would change semantics if they appear unquoted.
const NEEDS_QUOTING: &[char] = &[' ', '\t', '\n', '\r', '\u{0b}', '\u{0c}', ';', '"', '\0'];

/// Whether a quoted argument can safely drop its double quotes.
fn can_strip_quotes(raw: &str) -> bool {
    if raw.is_empty() {
        return false;
    }
    let first = raw.chars().next().unwrap();
    if matches!(first, '"' | '{' | '#') {
        return false;
    }
    if raw == "{*}" {
        return false;
    }
    if raw.chars().any(|c| NEEDS_QUOTING.contains(&c)) {
        return false;
    }
    // Any `{` / `}` outside `${var}` references blocks stripping.
    let stripped = strip_braced_var_refs(raw);
    !(stripped.contains('{') || stripped.contains('}'))
}

/// Remove `${…}` references from `raw` so the residual brace check
/// in [`can_strip_quotes`] only sees bare braces.
fn strip_braced_var_refs(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        if bytes[i] == b'$'
            && i + 1 < n
            && bytes[i + 1] == b'{'
            && let Some(close) = raw[i + 2..].find('}')
        {
            i = i + 2 + close + 1;
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
fn reconstruct_arg(sm: &SourceMap, arg: &Arg, env: MinifyEnv<'_>, depth: u32) -> String {
    let mut raw = String::new();
    for (idx, &tok) in arg.tokens.iter().enumerate() {
        // The next token within the *same* word can extend a preceding
        // `${var}` reference whether the word is quoted or bare
        // (`"${a}jumps"` and bare `${a}jumps` both read `ajumps` if the braces
        // are dropped), so the name-extension guard must see it in both cases.
        let next = arg.tokens.get(idx + 1).copied();
        raw.push_str(&reconstruct_raw(sm, tok, next, env, arg.is_quoted, depth));
    }
    if arg.is_quoted && !can_strip_quotes(&raw) {
        format!("\"{raw}\"")
    } else {
        raw
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
/// list grammar, never the script lexer — the previous script-lexer
/// implementation dropped a valid `#` pattern and its body as a
/// "comment" (issue #1197; tclsh 9.0.4: `switch # { # {puts matched}
/// default {puts default} }` prints `matched`).
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
            if canonical_flag == "--" {
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
        if body.braced {
            let body_src = &inner[body.value.clone()];
            let minified = minify_body(body_src, env, depth);
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

/// Remove unnecessary whitespace inside an `expr` body, keeping
/// spaces only around word-operators and between adjacent word
/// tokens. (no AST shrinking).
fn strip_expr_whitespace(text: &str, env: MinifyEnv<'_>, depth: u32) -> String {
    let toks = tokenise_expr(text, env, depth);
    let rendered: Vec<String> = toks
        .iter()
        .filter_map(|t| match t {
            ExprTok::Space => None,
            ExprTok::Cmd(s) | ExprTok::Other(s) => Some(s.clone()),
        })
        .collect();
    if rendered.is_empty() {
        return text.to_owned();
    }
    let mut out = String::new();
    out.push_str(&rendered[0]);
    for w in rendered.windows(2) {
        let (prev, cur) = (&w[0], &w[1]);
        if is_word_op(prev) || is_word_op(cur) || (is_word_token(prev) && is_word_token(cur)) {
            out.push(' ');
        }
        out.push_str(cur);
    }
    out
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
    let node = parse_expr(text, Some(env.dialect));
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
/// The table itself is [`BinOp::inverse`] (`tcl_syntax::expr::operators`) —
/// this used to be a fourth hand-typed copy of those 14 rows, which is how it
/// came to disagree with the shared evaluator.
///
/// The four *ordered numeric* rows are refused outright. Their identity holds
/// only when neither operand can be NaN (`expr {!(NaN < 1)}` is 1 while
/// `expr {NaN >= 1}` is 0 — [`BinOp::inverse_needs_non_nan`]), and the minifier
/// rewrites source text with no type information whatsoever, so it can never
/// discharge that precondition for an operand like `$x`. `==`/`!=` are exact
/// complements even for NaN, and the string / membership operators never
/// compare numerically, so those rows stay available (issue #1437).
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

/// Tokenise an `expr` body, with a catch-all so no character is
/// dropped.
fn tokenise_expr(text: &str, env: MinifyEnv<'_>, nest_depth: u32) -> Vec<ExprTok> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            let start = i;
            while i < n && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let _ = start;
            out.push(ExprTok::Space);
        } else if c == b'"' {
            let start = i;
            i += 1;
            while i < n {
                if bytes[i] == b'\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else if c == b'[' {
            let start = i;
            i += 1;
            let mut depth = 1;
            while i < n && depth > 0 {
                match bytes[i] {
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            // Clamp the inner slice to a char boundary: on an unbalanced
            // `[` the scan runs to EOF and `i - 1` can land mid-codepoint
            // (`expr {[é}`), which would panic the slice in debug *and*
            // release (a char-boundary violation, not an overflow) (F2a).
            let mut end = i.saturating_sub(1).max(start + 1);
            while end > start + 1 && !text.is_char_boundary(end) {
                end -= 1;
            }
            let inner = &text[start + 1..end];
            out.push(ExprTok::Cmd(format!(
                "[{}]",
                minify_body(inner, env, nest_depth + 1)
            )));
        } else if c == b'$' {
            let start = i;
            i += 1;
            while i < n
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':')
            {
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else if c.is_ascii_alphanumeric() || c == b'.' || c == b'_' {
            let start = i;
            while i < n
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'_')
            {
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else if is_expr_op_byte(c) {
            let start = i;
            while i < n && is_expr_op_byte(bytes[i]) {
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else {
            // Catch-all single char (`(`, `)`, `,`, etc.).
            let ch_len = utf8_len(c);
            out.push(ExprTok::Other(text[i..i + ch_len].to_owned()));
            i += ch_len;
        }
    }
    out
}

/// Whether `b` is a byte that forms a symbolic `expr` operator.
fn is_expr_op_byte(b: u8) -> bool {
    matches!(
        b,
        b'+' | b'-'
            | b'*'
            | b'/'
            | b'%'
            | b'<'
            | b'>'
            | b'='
            | b'!'
            | b'&'
            | b'|'
            | b'^'
            | b'?'
            | b':'
            | b'~'
    )
}

/// Whether `tok` is a Tcl expr word-operator needing surrounding
/// whitespace (`eq`, `ne`, `in`, `ni`, the TIP 461 `lt`/`le`/`gt`/`ge`, the
/// iRules word operators, …).
///
/// Derived from `tcl_syntax::expr::operators` (issue #983's unification)
/// rather than a hand-typed 4-entry list that used to miss every other
/// word-form operator — `lt`/`le`/`gt`/`ge` in particular, since they were
/// never classified as an operator *anywhere* until this same unification
/// effort added them to the registry. Not a confirmed corruption bug today
/// (the `is_word_token(prev) && is_word_token(cur)` branch at this
/// function's call site already independently forces the needed space for
/// every operand-adjacent case), but this function's own contract — "is
/// this a word operator" — was simply wrong for those spellings, and
/// relying on a second, unstated heuristic to paper over it is fragile.
fn is_word_op(tok: &str) -> bool {
    static OPS: std::sync::OnceLock<std::collections::HashSet<&'static str>> =
        std::sync::OnceLock::new();
    OPS.get_or_init(|| {
        tcl_syntax::expr::operators::ALL_BIN_OPS
            .iter()
            .map(|op| op.spec().spelling)
            .chain(
                tcl_syntax::expr::operators::ALL_UNARY_OPS
                    .iter()
                    .map(|op| op.spec().spelling),
            )
            .filter(|s| s.as_bytes().first().is_some_and(u8::is_ascii_alphabetic))
            .collect()
    })
    .contains(tok)
}

/// Whether `tok` is a "word" (identifier / number / variable /
/// string / command-substitution).
fn is_word_token(tok: &str) -> bool {
    let Some(c) = tok.chars().next() else {
        return false;
    };
    c == '$' || c == '"' || c == '[' || c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn min(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl(src, "tcl8.6", &registry)
    }

    /// Regression coverage for issue #996: `minify_body`'s recursive
    /// descent (shared with `render_command`, `reconstruct_arg`/
    /// `reconstruct_raw`, `minify_switch_case_list`, `minify_lambda_literal`,
    /// `compress_expr`/`tokenise_expr`) is now capped at `MAX_MINIFY_DEPTH`
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

    fn min_dialect(src: &str, dialect: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl(src, dialect, &registry)
    }

    fn min_compact(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl_compact(src, "tcl8.6", false, &registry).0
    }

    fn min_compact_isolated(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl_compact(src, "tcl8.6", true, &registry).0
    }

    #[test]
    fn compact_renames_proc_local_vars_and_params() {
        // Param `name`→`b`, local `message`→`a`.  The PROC name is a public
        // command identity (issue #1193): non-isolated compaction must keep
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
        // observable change (issue #1193).
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
        // `set a 1` would print `a` (issue #1193).
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
        // reads, or one variable silently splits into two (pre-#1193:
        // this returned 1 instead of 2 after compaction).
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
        // Non-isolated: variables compact, procs do not (issue #1193).
        let (_, sym) = minify_tcl_compact(
            "proc greet {name} {\n    return $name\n}\n",
            "tcl8.6",
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
            "tcl8.6",
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
            "tcl8.6",
            true,
            &registry,
        );
        assert_eq!(out, "set a 1;puts $a");
    }

    #[test]
    fn unminify_error_round_trips_via_symbol_map() {
        let registry = CommandRegistry::build_default();
        // Isolated so the proc is renamed too (issue #1193 keeps proc
        // names in the non-isolated tier).
        let (_, sym) = minify_tcl_compact(
            "proc greet {name} {\n    return $name\n}\n",
            "tcl8.6",
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
        minify_tcl_aggressive(src, "tcl8.6", false, &registry).source
    }

    #[test]
    fn static_fold_folds_sccp_constant_interpolation() {
        let registry = CommandRegistry::build_default();
        // `$x` is a proven integer constant; fold `"n=$x"` -> `n=5`.
        let (out, count, map) =
            fold_static_substrings("set x 5\nputs \"n=$x\"\n", "tcl8.6", &registry);
        assert_eq!(out, "set x 5\nputs n=5\n");
        assert_eq!(count, 1);
        assert!(map.values().any(|v| v == "n=5"), "{map:?}");
    }

    /// Issue #1424: the quoted word embeds a command substitution whose own
    /// argument is quoted. The shared close-quote scanner skips the whole
    /// `[…]`, so no edit is ever anchored on the inner `"b"` — a scanner
    /// stopping there would splice a replacement over `"a[string toupper "`
    /// and leave `b"] $x"` dangling.
    #[test]
    fn static_fold_keeps_a_quoted_command_substitution_well_formed() {
        let registry = CommandRegistry::build_default();
        let src = "set x 5\nputs \"a[string toupper \"b\"] $x\"\n";
        let (out, count, _) = fold_static_substrings(src, "tcl8.6", &registry);
        assert_eq!(count, 0, "a command substitution is not SCCP-foldable");
        assert_eq!(out, src, "source must come back untouched, not truncated");
    }

    /// The multiline shape reported on PR #1481: the `]` sitting in a
    /// command-position comment inside the substitution is inert in C Tcl
    /// (verified against tclsh 8.6/9.0), so the quoted word runs to the
    /// final `"`.  When the shared scanner stopped at that `]` it reported
    /// the quote opening the inner `"b"` as the closer, and any fold edge
    /// anchored there would splice over the middle of valid source.
    #[test]
    fn static_fold_keeps_a_commented_command_substitution_well_formed() {
        let registry = CommandRegistry::build_default();
        let src = "set x 5\nputs \"a[\n# ] comment\nset y \"b\"\n]c $x\"\n";
        let (out, count, _) = fold_static_substrings(src, "tcl8.6", &registry);
        assert_eq!(count, 0, "a command substitution is not SCCP-foldable");
        assert_eq!(out, src, "source must come back untouched, not truncated");
    }

    #[test]
    fn static_fold_skips_non_constant() {
        let registry = CommandRegistry::build_default();
        // `[HTTP::uri]` is dynamic — nothing folds.
        let (out, count, _) =
            fold_static_substrings("set u [HTTP::uri]\nputs \"got $u\"\n", "tcl8.6", &registry);
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
    fn aggressive_aliases_avoid_compacted_local_collisions() {
        // `request`->`a` (proc-local); the command alias must skip `a`
        // and use `b`, else `$a` in command position would resolve to
        // the local param.  (`handler` itself stays — proc names are
        // public identities in the non-isolated tier, issue #1193.)
        assert_eq!(
            agg(
                "proc handler {request} {\n    mylongcmd $request\n    mylongcmd $request\n    mylongcmd $request\n}\n"
            ),
            "set b mylongcmd;proc handler {a} {$b $a;$b $a;$b $a}",
        );
    }

    #[test]
    fn aggressive_aliases_avoid_live_variable_names() {
        // Issue #1194: the alias generator must not claim a name the
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
        let res = minify_tcl_aggressive(src, "tcl8.6", false, &registry);
        // With no applicable optimisations this equals the compact tier
        // (proc name preserved — public identity, issue #1193).
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
        // `namespace ev` is the aggressive tier's keyword abbreviation
        // (#1230): `ev` is the minimal unique prefix of `eval` in the
        // `namespace` table.  tclsh-proof (8.6.16): `namespace ev a { proc
        // dup {x} { return $x } }` then `a::dup 7` -> 7.
        let expected = "namespace ev a {proc dup {a} {set b [expr {$a+1}];return $b}};namespace ev b {proc dup {a} {return $a}}";
        for _ in 0..32 {
            let res = minify_tcl_aggressive(src, "tcl8.6", false, &registry);
            assert_eq!(res.source, expected, "collision corrupted the output");
        }
    }

    #[test]
    fn compact_never_renames_array_members() {
        // Array member names are Tcl DATA, not private symbols: `array
        // get` / `array names` / traces / serialization observe them, so
        // no tier may rename them (issue #1192).  tclsh 9.0.4: the
        // original prints `longmember 1`; the pre-fix compaction printed
        // `a 1`.
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
            "tcl8.6",
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
        // Issue #1194: the former template deduplication emitted a
        // `set a {…}` preamble + `[subst $a]` — a real variable write
        // that clobbered any live `a` (tclsh 9.0.4: `puts [set a]`
        // stopped printing the pre-existing value), fired traces, and
        // changed `info vars`.  The default tier must stay
        // frame-transparent: no `set`, no `subst`, strings verbatim.
        check(
            "puts \"value is $longvariablename here\"\nputs \"value is $longvariablename here\"\nputs [set a]\n",
            "puts \"value is $longvariablename here\";puts \"value is $longvariablename here\";puts [set a]",
        );
    }

    #[test]
    fn abbreviates_ensemble_subcommand_in_irules() {
        assert_eq!(
            min_dialect("string length $x\n", "f5-irules"),
            "string le $x"
        );
        assert_eq!(min_dialect("info exists $x\n", "f5-irules"), "info e $x");
    }

    #[test]
    fn no_subcommand_abbreviation_in_plain_tcl() {
        assert_eq!(
            min_dialect("string length $x\n", "tcl8.6"),
            "string length $x"
        );
    }

    #[test]
    fn expr_comparison_inversion() {
        check("if {!($a == $b)} {puts x}\n", "if {$a!=$b} {puts x}");
        check("if {!($a != $b)} {puts x}\n", "if {$a==$b} {puts x}");
    }

    /// Issue #1437: `!($a < $b)` is not `$a >= $b` when an operand may be NaN
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
        check(
            "if {!($a eq \"x\")} {puts x}\n",
            "if {$a ne \"x\"} {puts x}",
        );
        check(
            "if {!($a ne \"x\")} {puts x}\n",
            "if {$a eq \"x\"} {puts x}",
        );
        check("if {!($a in $b)} {puts x}\n", "if {$a ni $b} {puts x}");
        check("if {!($a ni $b)} {puts x}\n", "if {$a in $b} {puts x}");
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
        check(
            "if {$a eq $b} {\n    puts hi\n}\n",
            "if {$a eq $b} {puts hi}",
        );
    }

    /// Issue #983/#986: `is_word_op` used to only recognise `eq`/`ne`/`in`/
    /// `ni`, so a TIP 461 `lt` (unlike `eq`) relied entirely on the
    /// separate `is_word_token(prev) && is_word_token(cur)` heuristic to
    /// keep its surrounding whitespace — this proves the now-correct
    /// classification doesn't regress that spacing.
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
        // Issue #1197: a braced case list is a Tcl LIST, so `#` is an
        // ordinary pattern there, never a comment.  tclsh 9.0.4:
        // `switch # { # {puts matched} default {puts default} }`
        // prints `matched`; the pre-fix minifier dropped the `#` arm
        // and the output printed `default`.
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
        let out = min("switch $x {\n    a {puts 1}\n    b\n}\n");
        assert!(out.contains("a {puts 1}"), "{out}");
        assert!(out.contains('b'), "{out}");
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

    /// Issue #954: `apply`'s lambda-literal argument (`ArgRole::LambdaLiteral`,
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

    /// Codex review of #954's follow-up: a bare body element's backslash
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

    // Keyword abbreviations (#1230).
    //
    // tclsh ground truth (8.6.16): `string le abc` → `3`, `string eq a a` → `1`,
    // `lsearch -noc {A b} a` → `0`; `string l abc` → `unknown or ambiguous
    // subcommand "l"`, so an ambiguous prefix must never be emitted.
    mod abbreviations {
        use super::*;

        fn aggressive(src: &str) -> String {
            let registry = tcl_registry::registry_for_dialect("tcl8.6");
            minify_tcl_aggressive(src, "tcl8.6", false, registry).source
        }

        fn aggressive_no_abbrev(src: &str) -> String {
            let registry = tcl_registry::registry_for_dialect("tcl8.6");
            minify_tcl_aggressive_with(src, "tcl8.6", false, registry, false).source
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
            let registry = tcl_registry::registry_for_dialect("tcl8.5");
            let out =
                minify_tcl_aggressive("puts [string compare $a $b]\n", "tcl8.5", false, registry)
                    .source;
            let emitted = out.split_whitespace().find(|w| "compare".starts_with(*w));
            if let Some(word) = emitted {
                let t90 = tcl_registry::registry_for_dialect("tcl9.0")
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

    /// Issue #1275 — minification must resolve a command head's *effective
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
            "tcl8.6",
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
            "tcl8.6",
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
    fn rename_barriers_follow_the_resolved_head() {
        const UPVAR_BODY: &str =
            "proc p {v} {\n    set local 1\n    upvar 1 $v alias\n    return $local\n}\n";
        const PEEK_BODY: &str =
            "proc p {v} {\n    set local 1\n    peek 1 $v alias\n    return $local\n}\n";
        let registry = CommandRegistry::build_default();
        let minify = |src: &str| minify_tcl_aggressive(src, "tcl8.6", true, &registry).source;
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
