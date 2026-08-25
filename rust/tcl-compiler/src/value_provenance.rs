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

//! Flow-sensitive constant **value provenance** for a variable use.
//!
//! Answers, at a specific program point (an SSA use of a variable), the
//! question "which *written constants* can this variable hold here, and
//! where in the source is each one written?" — the fact the constant-
//! `$cmd` dispatch settlement needs (issue #945 faults 1 and 2):
//!
//! * the **value set** must be flow-sensitive: a `set cmd bar` inside an
//!   `if` arm *joins* the outer `set cmd foo` at the dispatch point, it
//!   does not overwrite it (both are may-targets — C Tcl dispatches
//!   whichever assignment ran), and the value reaching the use is the
//!   SSA use-version's, never "the lexically last write in the file";
//! * each value carries **writable provenance**: the content span of the
//!   literal as written, so a rename can rewrite the defining constant
//!   (`set cmd target` → `set cmd renamed`) instead of leaving the
//!   variable holding the old command name — or `None` when the constant
//!   has no exact source representation, in which case a rename must
//!   abstain;
//! * anything the walk cannot prove is a **sound abstention**: a
//!   computed value, a write through `upvar`/trace/`uplevel`, a proc
//!   parameter, an `{*}`-expanded or backslash-processed word all
//!   surface as `None` ("unknown"), never as a guessed constant.
//!
//! The walk operates on the compiler's own SSA form: from the use
//! version it chases each reaching definition through φ-joins (control-
//! flow merges: `if` / `switch` / loops / `catch` / `try`) and through
//! single-`$var` copy chains, bottoming out at literal assignment
//! statements.  It dispatches purely on IR statement shape
//! ([`Statement::AssignConst`] / [`Statement::AssignValue`]) — which
//! commands *produce* those shapes is the lowering hooks' knowledge, so
//! no command name appears here.

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;

use crate::compilation_unit::FunctionUnit;
use crate::ir::Statement;
use crate::lowering_hooks::word_content_base;
use crate::ssa::{Symbol, Version};

/// One constant definition contributing to a variable's value at a use
/// point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueContributor {
    /// The constant value the contributing definition stores.
    pub value: String,
    /// Absolute content span of the value as **written** at the
    /// contributing definition — the span a rename may splice a new
    /// command name over.  `None` when the constant is proven but has no
    /// exact source representation (a folded / synthesised constant):
    /// the value participates in navigation but a rename must abstain.
    pub literal_span: Option<Span>,
}

/// Where a `(symbol, version)` pair is defined inside one function.
enum DefSite<'a> {
    /// A φ-node joining the versions flowing in from predecessor blocks.
    Phi(&'a crate::ssa::Phi),
    /// A concrete defining statement, with its SSA use map (for copy
    /// chains).
    Stmt(&'a crate::ssa::SsaStatement),
}

/// Index every φ and defining statement of `fu` by `(symbol, version)`.
fn def_index(fu: &FunctionUnit) -> HashMap<(Symbol, Version), DefSite<'_>> {
    let mut index = HashMap::new();
    for block in fu.ssa.blocks.values() {
        for phi in &block.phis {
            index.insert((phi.name, phi.version), DefSite::Phi(phi));
        }
        for stmt in &block.statements {
            for (&sym, &version) in &stmt.defs {
                index.insert((sym, version), DefSite::Stmt(stmt));
            }
        }
    }
    index
}

/// The SSA use-version of `sym` at the narrowest statement covering
/// `offset` (absolute) that actually reads it, or `None` when no
/// covering statement uses the symbol.
fn use_version_at(fu: &FunctionUnit, offset: u32, sym: Symbol) -> Option<Version> {
    let mut best: Option<(u32, Version)> = None;
    for block in fu.ssa.blocks.values() {
        for stmt in &block.statements {
            let span = fu.abs_span(stmt.statement.span());
            if !(span.start() <= offset && offset <= span.end()) {
                continue;
            }
            let Some(&version) = stmt.uses.get(&sym) else {
                continue;
            };
            let width = span.end() - span.start();
            if best.is_none_or(|(bw, _)| width < bw) {
                best = Some((width, version));
            }
        }
    }
    best.map(|(_, version)| version)
}

/// The variable name a pure single-`$var` copy reads, when `value` is
/// exactly one scalar substitution (`$name` / `${name}`) and nothing
/// else.  An array element (`$a(k)`), a compound word, or any extra
/// text disqualifies it.
fn pure_copy_source(value: &str) -> Option<&str> {
    let rest = value.strip_prefix('$')?;
    let name = rest
        .strip_prefix('{')
        .and_then(|inner| inner.strip_suffix('}'))
        .unwrap_or(rest);
    // The unbraced form must consume the whole word with name
    // characters only; the braced form must not be an array element.
    let plain = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':');
    (!name.is_empty() && plain && !name.contains('(')).then_some(name)
}

/// Fold a `[list W1 W2 ...]` value to its space-joined string when every
/// element is a **plain** literal word — no `$`/`[`/`{`/`"`/backslash/
/// whitespace, so Tcl's own `list` command needs no brace/backslash
/// quoting to represent it (tclsh9.0/8.6-verified: `[list a b]` is
/// byte-identical to the string `"a b"` for such elements). Deliberately
/// narrow — the exact shape issue #923 idx 94's own repro needs, not a
/// general `list`-command simulator: any element needing quoting protection
/// (containing whitespace, braces, or another special character) bails, as
/// does anything other than a single, whole-value `[list ...]` call.
///
/// Returns `(joined, first_offset, first_len)`: the joined string, plus the
/// first element's own byte offset and length — both relative to `value`'s
/// own start (its opening `[`) — so the caller can anchor a rename-writable
/// span on just that one argument.
fn fold_literal_list_call(value: &str) -> Option<(String, u32, u32)> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let cmds = crate::segmenter::segment_commands_with_offset(inner, 0);
    let [cmd] = cmds.as_slice() else {
        return None;
    };
    if cmd.name() != "list" {
        return None;
    }
    let args = cmd.args();
    let arg_tokens = cmd.arg_tokens();
    let singles = cmd.arg_single_token();
    if args.is_empty() || args.len() != singles.len() || args.len() != arg_tokens.len() {
        return None;
    }
    let mut words = Vec::with_capacity(args.len());
    for (word, &single) in args.iter().zip(singles) {
        if !single
            || word.is_empty()
            || word.contains(['{', '}', '"', '\\', '$', '[', ']'])
            || word.chars().any(char::is_whitespace)
        {
            return None;
        }
        words.push(word.as_str());
    }
    // `inner`'s own offset 0 is one byte past `value`'s opening `[`.
    let first_offset = arg_tokens[0].span.start() + 1;
    let first_len = u32::try_from(words[0].len()).ok()?;
    Some((words.join(" "), first_offset, first_len))
}

/// The contributing constant definitions for the value of `var_name` at
/// the statement covering `use_offset` (absolute), walking φ-joins and
/// single-`$var` copy chains to the literal assignments that feed it.
///
/// Returns `None` — the **sound abstention** — when the value cannot be
/// proven to be a finite set of written constants: the function is
/// complexity-guarded, the variable is not an SSA local here (written
/// through `upvar` / traces / dynamic names), the use version reaches
/// function entry (a parameter or an out-of-frame value), or any
/// reaching definition is not a literal / pure-copy assignment.
///
/// The returned set is de-duplicated by `(value, literal_span)` and
/// non-empty.
#[must_use]
pub fn const_contributors(
    fu: &FunctionUnit,
    use_offset: u32,
    var_name: &str,
) -> Option<Vec<ValueContributor>> {
    if fu.complexity_guarded {
        return None;
    }
    let sym = fu.ssa.var_symbol(var_name)?;
    let version = use_version_at(fu, use_offset, sym)?;
    const_contributors_for_version(fu, var_name, version)
}

/// [`const_contributors`] when the caller already owns the exact reaching SSA
/// version. This is the phase-correct adapter for deferred text: a variable in
/// a brace-quoted script may not be a direct use of the enclosing statement,
/// even though a registry-declared evaluator will read that same version.
#[must_use]
pub fn const_contributors_for_version(
    fu: &FunctionUnit,
    var_name: &str,
    version: Version,
) -> Option<Vec<ValueContributor>> {
    if fu.complexity_guarded {
        return None;
    }
    let sym = fu.ssa.var_symbol(var_name)?;
    let index = def_index(fu);
    let mut visited: HashSet<(Symbol, Version)> = HashSet::new();
    let mut out: Vec<ValueContributor> = Vec::new();
    collect(fu, &index, sym, version, &mut visited, &mut out)?;
    if out.is_empty() {
        return None;
    }
    deduplicate_contributors(&mut out);
    Some(out)
}

/// Evidence-backed constant contributors even when another reaching arm is
/// unknown.
///
/// Security may-analysis uses this to retain a proven sink target at a join
/// without claiming the command value is closed. Rename and other exact-value
/// consumers must continue to use [`const_contributors_for_version`].
#[must_use]
pub fn known_const_contributors_for_version(
    fu: &FunctionUnit,
    var_name: &str,
    version: Version,
) -> Vec<ValueContributor> {
    if fu.complexity_guarded {
        return Vec::new();
    }
    let Some(sym) = fu.ssa.var_symbol(var_name) else {
        return Vec::new();
    };
    let index = def_index(fu);
    let mut visited = HashSet::new();
    let mut out = Vec::new();
    let _ = collect(fu, &index, sym, version, &mut visited, &mut out);
    deduplicate_contributors(&mut out);
    out
}

/// [`known_const_contributors_for_version`] at the SSA use covering an
/// absolute source offset.
#[must_use]
pub fn known_const_contributors(
    fu: &FunctionUnit,
    use_offset: u32,
    var_name: &str,
) -> Vec<ValueContributor> {
    if fu.complexity_guarded {
        return Vec::new();
    }
    let Some(sym) = fu.ssa.var_symbol(var_name) else {
        return Vec::new();
    };
    let Some(version) = use_version_at(fu, use_offset, sym) else {
        return Vec::new();
    };
    known_const_contributors_for_version(fu, var_name, version)
}

fn deduplicate_contributors(out: &mut Vec<ValueContributor>) {
    // Keep first-seen order; one literal can feed a use along several φ paths.
    let mut seen: HashSet<(String, Option<(u32, u32)>)> = HashSet::new();
    out.retain(|c| {
        seen.insert((
            c.value.clone(),
            c.literal_span.map(|s| (s.start(), s.end())),
        ))
    });
}

/// Recursive collector for [`const_contributors`]: resolve the defining
/// site of `(sym, version)` and either recurse (φ, copy) or record the
/// literal.  Returns `None` on the first unprovable definition.
fn collect(
    fu: &FunctionUnit,
    index: &HashMap<(Symbol, Version), DefSite<'_>>,
    sym: Symbol,
    version: Version,
    visited: &mut HashSet<(Symbol, Version)>,
    out: &mut Vec<ValueContributor>,
) -> Option<()> {
    if !visited.insert((sym, version)) {
        // Already being resolved along this walk (a loop-carried φ
        // cycle): the cycle's value comes from its other operands.
        return Some(());
    }
    match index.get(&(sym, version)) {
        // No defining φ or statement: the version flows in from function
        // entry — a parameter, an out-of-frame binding, or "possibly
        // undefined".  Not provable.
        None => None,
        Some(DefSite::Phi(phi)) => {
            let mut complete = true;
            for &incoming in phi.incoming.values() {
                if collect(fu, index, sym, incoming, visited, out).is_none() {
                    complete = false;
                }
            }
            complete.then_some(())
        }
        Some(DefSite::Stmt(stmt)) => contributor_from_stmt(fu, index, stmt, visited, out),
    }
}

/// Extract the contribution of one defining statement, recursing
/// through pure single-`$var` copies.
fn contributor_from_stmt(
    fu: &FunctionUnit,
    index: &HashMap<(Symbol, Version), DefSite<'_>>,
    stmt: &crate::ssa::SsaStatement,
    visited: &mut HashSet<(Symbol, Version)>,
    out: &mut Vec<ValueContributor>,
) -> Option<()> {
    match &stmt.statement {
        Statement::AssignConst {
            value, value_span, ..
        } => {
            out.push(ValueContributor {
                value: value.clone(),
                literal_span: value_span.map(|s| fu.abs_span(s)),
            });
            Some(())
        }
        Statement::AssignValue {
            value,
            value_needs_backsubst,
            tokens,
            ..
        } => {
            // A pure `$other` copy chains to the source variable's own
            // contributors at this statement's use version.
            if let Some(src_name) = pure_copy_source(value) {
                let src_sym = fu.ssa.var_symbol(src_name)?;
                let &src_version = stmt.uses.get(&src_sym)?;
                return collect(fu, index, src_sym, src_version, visited, out);
            }
            // A `[list W1 W2 ...]` value whose every element is a plain
            // literal folds to the space-joined string — issue #923 idx
            // 94's own minimal repro (`set cmdD [list greetD World]; eval
            // $cmdD`). The joined *value* has no single source span (it's
            // synthesised from several separate argument tokens), but the
            // first element — the actual command-dispatch anchor, the only
            // part `settle_one_site`'s `head_expanded` narrowing ever reads
            // — does: anchor `literal_span` there directly (skipping
            // `command_component`'s usual "narrow a whole-value span down
            // to its first word" step, since there is no whole-value span
            // to narrow) so a rename can still safely rewrite just that
            // one argument in place. The value token's own span *starts*
            // exactly at its opening `[` (`tcl-lexer`'s `Cmd`-token span
            // convention — the same one idx 95 already found excludes the
            // *closing* delimiter, so `word_content_base`'s length-based
            // delta below underflows for it and correctly declines; the
            // *start* offset needs no such adjustment), so this reads it
            // directly rather than reusing that helper.
            if !*value_needs_backsubst
                && let Some((joined, first_offset, first_len)) = fold_literal_list_call(value)
            {
                let literal_span = tokens.as_ref().and_then(|t| {
                    let base = t.argv.get(2)?.start() + first_offset;
                    Some(fu.abs_span(Span::new(base, base + first_len)))
                });
                out.push(ValueContributor {
                    value: joined,
                    literal_span,
                });
                return Some(());
            }
            // A plain literal word (no substitution, no backslash
            // processing): source-exact when the value word's content is
            // a verbatim slice.
            if *value_needs_backsubst || value.contains('$') || value.contains('[') {
                return None;
            }
            let literal_span = tokens.as_ref().and_then(|t| {
                let base = word_content_base(
                    *t.argv.get(2)?,
                    t.single_token_word.get(2).copied().unwrap_or(false),
                    value,
                )?;
                let end = base + u32::try_from(value.len()).ok()?;
                Some(fu.abs_span(Span::new(base, end)))
            });
            out.push(ValueContributor {
                value: value.clone(),
                literal_span,
            });
            Some(())
        }
        // Any other defining shape (a command with variable defs —
        // `upvar` / `global` / `foreach` / `lassign` / traces —, an
        // `incr`, an expression assignment) is not a written command-
        // name constant: abstain.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_copy_source_accepts_plain_and_braced() {
        assert_eq!(pure_copy_source("$cmd"), Some("cmd"));
        assert_eq!(pure_copy_source("${cmd}"), Some("cmd"));
        assert_eq!(pure_copy_source("$ns::cmd"), Some("ns::cmd"));
    }

    #[test]
    fn pure_copy_source_rejects_compound_and_array_shapes() {
        assert_eq!(pure_copy_source("x$cmd"), None);
        assert_eq!(pure_copy_source("$cmd tail"), None);
        assert_eq!(pure_copy_source("$a(k)"), None);
        assert_eq!(pure_copy_source("${a(k)}"), None);
        assert_eq!(pure_copy_source("$"), None);
        assert_eq!(pure_copy_source("plain"), None);
    }
}
