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

//! Dynamic-name barrier facts.
//!
//! Name-level SSA and the def-use chains built on it answer questions of the
//! form "is `x` defined here?" and "is this store to `x` ever read?".  Both
//! are only answerable while every variable access in the function spells its
//! target out.  Tcl lets a program compute the name instead:
//!
//! ```tcl
//! set $switch {}          ;# writes whatever variable $switch names
//! lappend out [set $name] ;# reads whatever variable $name names
//! ```
//!
//! After a **dynamic write**, *any* name may be defined, so "`x` was never
//! defined here" is no longer provable.  After a **dynamic read**, *any*
//! store may have been observed, so "this store is never read" is no longer
//! provable.  After a **dynamic destroy** (`unset $n`), *any* name may have
//! ceased to exist, so even "this parameter certainly exists" is no longer
//! provable.
//!
//! The three facts are deliberately **flags, not name sets**: a dynamic
//! access clobbers the whole name space, so enumerating candidates would be
//! both unsound (the value can come from anywhere) and unbounded.  Three
//! bools is the whole lattice, computed in one flow-insensitive walk, so the
//! consumers pay `O(1)` per query.
//!
//! Which argument of which command names a variable is **the registry's**
//! answer ([`ArgRole::VarWrite`] / [`ArgRole::VarRead`] /
//! [`Traits::DESTROYS_VARIABLE`] / [`Traits::PERFORMS_SUBSTITUTION`]) — this
//! module holds no command names.
//!
//! # Soundness direction
//!
//! Every consumer abstains when its fact is set: warnings go silent
//! (`W210` / `W211` / `W220` / `I230`) and the optimiser declines to fold or
//! eliminate (`O101` / `O109` / `O126`).  Both are the same direction — say
//! less rather than say something wrong.
//!
//! # Caller-frame injection
//!
//! A second, independent way to lose the name space is to have somebody
//! *else* write it.  Tcl's frame-crossing commands make a callee's writes
//! land in the caller's frame, and the caller's own text shows nothing:
//!
//! ```tcl
//! proc runner {body} { uplevel 1 $body }   ;# runs $body in the CALLER's frame
//! proc f {s} { runner $s; puts $x }        ;# $x may well be set — by $s
//! ```
//!
//! Which frame each such command targets is registry data
//! ([`FrameEffectSpec`]), and the answer decides who goes blind:
//!
//! | shape | frame written | recorded by |
//! |---|---|---|
//! | `eval $body` | the frame it is written in | this module, directly |
//! | `argparse {…}` | the frame that *called* it | this module, directly |
//! | `uplevel 1 $body` in a proc | that proc's caller | the proc's [frame-effect summary][sum], read at each call site |
//! | `upvar 1 $computed x` | that proc's caller | the same summary |
//!
//! The first two are visible in the function's own statements, so the walk
//! below raises the flags itself.  The last two are visible only with the
//! module-wide proc summaries, which the CFG builder holds and this
//! per-function walk does not — so it records them on
//! [`CfgFunction::caller_frame_barrier`], and
//! [`dynamic_name_barrier`] folds them in.  Either way the fact lands in
//! the same three bits, so every consumer's abstention rule is unchanged
//! and the cost stays `O(1)` per query.
//!
//! Deliberately **not** a per-call-site fact: every consumer of this
//! lattice (`W210` / `W211` / `W220` / `I230`, `O101` / `O109` / `O126`)
//! already reads it once per function and abstains for the whole function.
//! A per-site fact would need flow-sensitivity none of them have, and would
//! buy nothing — the flow-insensitive union is what they would compute.
//!
//! `uplevel 1 $body` written *inside* a proc raises nothing for that proc:
//! the script runs one frame **up**, so the proc's own locals are
//! untouched. tclsh 9.0.4 / 8.6.14, identical: `proc runner {body} {set
//! helper 42; uplevel 1 $body}` invoked with `{set x $helper}` raises
//! `can't read "helper": no such variable`, while the same body under
//! `eval $body` reads `42`.
//!
//! [sum]: crate::cfg_builder::upvar_info::UpvarInfo
//!
//! A **computed command head** (`$cmd length foo`, `[pick] $n 1`) cannot
//! donate a handler through its written text. Retained source resolution can
//! nevertheless prove its actual handler or enumerate possible handlers after
//! argv. The name-role walk consumes those identities, including their separate
//! write and destruction obligations; uncertainty never grants a physical store.
//!
//! For an unbounded head no role candidate exists here. Unknown-command effects
//! belong to the source owner and CFG barriers, rather than to guessed catalogue
//! roles. In particular, `$set length foo` must not borrow `set` merely because
//! its variable is named `set`; invoking it with `string` defines no variable.

use tcl_lexer::LexerConfig;
use tcl_registry::frame_effect::{FrameArgLayout, FrameEffectSpec};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::expr_ast::ExprNode;
use crate::ir::Statement;

/// Whether a function accesses variables whose *name* is computed at run
/// time, split by the direction each blinds.
///
/// See the [module docs](self) for the lattice and the soundness direction
/// each flag imposes on its consumers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DynamicNameBarrier {
    /// A command creates or overwrites a variable whose name is computed at
    /// run time (`set $var v`, `lappend $n 1`, `array set $a {…}`).  After
    /// it, **any** name may be defined.
    pub writes: bool,
    /// A command destroys a variable whose name is computed at run time
    /// (`unset $n`).  After it, **any** name may have ceased to exist.
    pub destroys: bool,
    /// A command reads a variable whose name is computed at run time
    /// (`set $v`, `parray $a`, `subst $tmpl`).  After it, **any** store may
    /// have been observed.
    pub reads: bool,
}

impl DynamicNameBarrier {
    /// True when no direction is blinded — the function names every variable
    /// it touches.
    #[must_use]
    pub const fn is_clear(self) -> bool {
        !self.writes && !self.destroys && !self.reads
    }

    /// Union with `other` — the lattice join.  Blindness only accumulates,
    /// so merging two sources of it (this walk and the CFG builder's
    /// caller-frame record) is a per-flag `or`.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            writes: self.writes || other.writes,
            destroys: self.destroys || other.destroys,
            reads: self.reads || other.reads,
        }
    }

    /// Every direction blinded — an arbitrary script ran in this frame, so
    /// any name may have been created, read, or destroyed.
    pub const OPAQUE_SCRIPT: Self = Self {
        writes: true,
        destroys: true,
        reads: true,
    };
}

/// Whether *word* — an argument sitting in a registry-declared
/// variable-name role — names a variable the compiler cannot resolve
/// statically.
///
/// `word` arrives either as the segmenter's *reconstructed* text, in which a
/// `$name` substitution is canonicalised to its braced spelling `${name}`, or
/// as a token's verbatim `$name` spelling.  Both mean the same thing —
/// `${name}` really is a substitution in Tcl, not a literal name (tclsh
/// 9.0.4 / 8.6.14: `set x foo; set ${x} bar; info exists foo` → `1`) — so any
/// `$` or `[` in the *name position* means the name comes from run-time data.
///
/// The caller must first rule out a **brace-quoted** word: `{$n}` carries the
/// same `$` but substitutes nothing, and this function cannot tell the two
/// spellings apart on text alone.  [`scan_command`] applies that check.
///
/// Three shapes are deliberately **not** dynamic:
///
/// - `a($k)` — a run-time-chosen *element* of the statically named array
///   `a`. The [place model](crate::place) already tracks element identity;
///   treating it as a whole-name clobber would silence array diagnostics
///   wholesale.
/// - `${ns}::tail` — a `::`-qualified name binds or targets its **tail**, so
///   a computed *namespace* leaves the name itself known.  `variable
///   ${name}::graphAttr` creates the local `graphAttr` whatever `$name`
///   holds (tclsh 9.0.4 / 8.6.14), and cannot conjure any other local.
/// - the empty word — `set {} 1` names the (literal) empty-named variable.
#[must_use]
pub fn names_a_dynamic_variable(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let base = word.split_once('(').map_or(word, |(base, _)| base);
    let tail = base.rsplit("::").next().unwrap_or(base);
    tail.contains('$') || tail.contains('[')
}

/// Whether the variable-name word `word` — one
/// [`names_a_dynamic_variable`] has already classified dynamic — **can spell**
/// the fully-qualified cell `qualified_cell`.
///
/// The per-site provenance question the rename gate needs: "any dynamic
/// variable word in the document" is a far blunter refusal than the language
/// requires, because a word's *written* text already bounds the set of names
/// it can produce.  A substitution can evaluate to anything, but the literal
/// characters around it cannot change — so the word is a pattern, its
/// substitutions are wildcards, and a cell no spelling of which matches that
/// pattern is provably out of reach.
///
/// The pattern is matched against every way the cell can be *written* at a
/// use site: the rooted name and each of its `::`-segment suffixes
/// (`::ns::v` → `::ns::v`, `ns::v`, `v`), which covers the absolute spelling
/// and every relative one.  A wildcard matches the empty string too — an
/// empty substitution really does collapse (tclsh-proof, 8.6.14: `set i {};
/// set v$i 1` leaves `info exists v` -> 1).
///
/// Abstain-toward-refuse stays the rule: `true` (could spell it) is the
/// answer for anything unproven, including a bare `$n`, which is a lone
/// wildcard and matches every spelling.
///
/// tclsh-proof (8.6.14) for the two facts the bound rests on:
///
/// ```text
/// namespace eval ::ns { variable v 1 } ; namespace eval ::other {}
/// set n {::ns::v} ; set ::other::$n 99
///   -> can't set "::other::::ns::v": parent namespace doesn't exist
/// ;# a written prefix confines the name — a substitution cannot escape it
/// namespace eval ::ns { variable total 5 } ; set j 1 ; set v$j 2
///   -> info vars ::v* is {::v1}, ::ns::total still 5
/// ```
/// `braced_var` is the **document's** `${…}` close rule, and it is a required
/// parameter rather than a defaulted one on purpose. Where the closer lands
/// decides how much of the word is *literal*, and the literal characters are
/// the whole bound this gate rests on, so the two release rules move the
/// answer in opposite directions:
///
/// - Reading an **8.x** document with the 9.x rule makes `${a{b}c}` one
///   wildcard, which matches every cell — so a rename that is provably safe is
///   refused (the LSP declines an otherwise-fine namespace/variable rename).
/// - Reading a **9.x** document with the 8.x rule makes it a wildcard followed
///   by the literal `c}` — a narrower pattern, so a cell the word really can
///   spell is judged out of reach and the rename proceeds *unsafely*.
///
/// There is deliberately no overload defaulting to
/// [`tcl_dialect::BracedVarStyle::default`]: every production caller
/// (`tcl_lsp_core::rename_safety`, `tcl_lsp_core::namespace_rename`) holds a
/// resolved `DialectProfile`, and silently taking the default is the defect
/// this parameter exists to prevent.
#[must_use]
pub fn dynamic_variable_word_can_spell(
    word: &str,
    qualified_cell: &str,
    braced_var: tcl_dialect::BracedVarStyle,
) -> bool {
    // An element suffix names an element of the base array, so the *variable*
    // this word names is the base — the same split `names_a_dynamic_variable`
    // makes.
    let base = word.split_once('(').map_or(word, |(base, _)| base);
    let pattern = name_word_pattern(base, braced_var);
    cell_spellings(qualified_cell).any(|spelling| pattern_matches(&pattern, spelling))
}

/// One piece of a variable-name word: a run of literal characters, or a
/// substitution whose value is unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NamePiece {
    Literal(String),
    /// `$name`, `${name}`, or `[cmd …]` — any string, the empty one included.
    Wildcard,
}

/// Split a variable-name word into its literal runs and its substitutions.
///
/// Text arrives either as the segmenter's reconstructed form (a `$name`
/// substitution canonicalised to `${name}`) or as a token's verbatim
/// spelling, so both are recognised — the same double spelling
/// [`names_a_dynamic_variable`] documents.
fn name_word_pattern(word: &str, braced_var: tcl_dialect::BracedVarStyle) -> Vec<NamePiece> {
    let bytes = word.as_bytes();
    let mut pieces: Vec<NamePiece> = Vec::new();
    let mut literal = String::new();
    let mut i = 0usize;
    let push_wildcard = |pieces: &mut Vec<NamePiece>, literal: &mut String| {
        if !literal.is_empty() {
            pieces.push(NamePiece::Literal(std::mem::take(literal)));
        }
        if pieces.last() != Some(&NamePiece::Wildcard) {
            pieces.push(NamePiece::Wildcard);
        }
    };
    while i < bytes.len() {
        match bytes[i] {
            b'$' if i + 1 < bytes.len() && bytes[i + 1] == b'{' => {
                // The name starts just past the `${`. An unterminated
                // reference has no closer, so the wildcard runs to the end of
                // the word — which is what a lenient tokeniser does with it.
                i = match tcl_lexer::braced_var_name_end(bytes, i + 2, braced_var) {
                    tcl_lexer::BracedVarEnd::Closed(end) => end + 1,
                    tcl_lexer::BracedVarEnd::Unterminated => bytes.len(),
                };
                push_wildcard(&mut pieces, &mut literal);
            }
            b'$' => {
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric()
                        || bytes[i] == b'_'
                        || (bytes[i] == b':' && bytes.get(i + 1) == Some(&b':')))
                {
                    i += if bytes[i] == b':' { 2 } else { 1 };
                }
                push_wildcard(&mut pieces, &mut literal);
            }
            b'[' => {
                let mut depth = 0usize;
                while i < bytes.len() {
                    match bytes[i] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                push_wildcard(&mut pieces, &mut literal);
            }
            _ => {
                let ch_len = word[i..].chars().next().map_or(1, char::len_utf8);
                literal.push_str(&word[i..i + ch_len]);
                i += ch_len;
            }
        }
    }
    if !literal.is_empty() {
        pieces.push(NamePiece::Literal(literal));
    }
    pieces
}

/// Every way `qualified_cell` can be written at a use site: the rooted name
/// and each of its `::`-segment suffixes.
///
/// `::ns::v` yields `::ns::v`, `ns::v`, `v` — the absolute spelling plus the
/// relative ones a site inside an enclosing namespace would use.  Over-wide
/// on purpose: a spelling that could not actually resolve to this cell from
/// some site only ever adds a refusal, never removes one.
fn cell_spellings(qualified_cell: &str) -> impl Iterator<Item = &str> {
    let rooted = qualified_cell.starts_with("::");
    let bare = qualified_cell.trim_start_matches("::");
    std::iter::once(qualified_cell)
        .chain(std::iter::once(bare).filter(move |_| rooted))
        .chain(bare.match_indices("::").map(|(at, _)| &bare[at + 2..]))
}

/// Wildcard match — `NamePiece::Wildcard` is `*`, literals must appear in
/// order.  The classic greedy scan: each literal is found at or after the
/// current position, anchored when it is the first / last piece.
fn pattern_matches(pattern: &[NamePiece], candidate: &str) -> bool {
    let mut rest = candidate;
    let mut free = false;
    for (idx, piece) in pattern.iter().enumerate() {
        match piece {
            NamePiece::Wildcard => free = true,
            NamePiece::Literal(lit) => {
                let at = if free {
                    match rest.find(lit.as_str()) {
                        Some(at) => at,
                        None => return false,
                    }
                } else if rest.starts_with(lit.as_str()) {
                    0
                } else {
                    return false;
                };
                // The final literal must reach the end of the candidate.
                if idx + 1 == pattern.len() {
                    return if free {
                        rest.ends_with(lit.as_str())
                    } else {
                        rest == lit.as_str()
                    };
                }
                rest = &rest[at + lit.len()..];
                free = false;
            }
        }
    }
    // Ended on a wildcard (or an empty pattern): the remainder is free only
    // when a wildcard can absorb it.
    free || rest.is_empty()
}

/// The lexer config a **profile-built** registry implies — the dialect
/// [`dynamic_name_barrier`] must split its `[…]` texts under.
///
/// `registry_for_dialect` / `registry_for_profile` stamp the dialect profile
/// on the registry, and that profile's `grammar` is the very field
/// [`LexerConfig::for_profile`] reads, so the two cannot drift.  A
/// hand-assembled registry carries no profile and answers the default
/// (Tcl-8.5+) config, which is what a caller with no dialect view had anyway.
/// A caller that already holds the document's own config (the compile
/// pipeline's [`crate::compilation_unit::UnitBuildOptions::config`]) should
/// pass that instead.
#[must_use]
pub fn lexer_config_for(registry: &CommandRegistry) -> LexerConfig {
    LexerConfig::for_profile(registry.profile())
}

/// Compute the [`DynamicNameBarrier`] for `cfg`.
///
/// One flow-insensitive walk over every statement, terminator condition, and
/// nested `[…]` command substitution in the function.  Flow-insensitivity is
/// the conservative choice: a read *before* the dynamic write is blinded too,
/// which only ever silences, never invents, a fact.
///
/// `config` is the **document's** lexer config, and it is load-bearing rather
/// than cosmetic: the `[…]` texts this walk re-splits must break into words
/// the same way the IR did, or the two disagree about which argument sits in a
/// variable-name role.  Under `f5-irules` the segmenter splits `cmd {a}{b}`
/// into three words and a Tcl-configured split into two; under `tcl8.4` a
/// `{*}` is a literal word rather than an expansion marker.  A boundary
/// disagreement can drop a dynamic write off the name-role index, leaving the
/// barrier clear and letting SCCP propagate a constant across it.
/// Build it from the same dialect the lowering used —
/// [`lexer_config_for`] does that from a profile-built registry.
#[must_use]
pub fn dynamic_name_barrier(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> DynamicNameBarrier {
    // Caller-frame injection the CFG builder already resolved against the
    // module-wide proc summaries — the same three bits, joined in.
    let mut barrier = cfg.caller_frame_barrier;
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            scan_statement(stmt, registry, &mut barrier, config);
            // The CFG builder flattens structured control flow, but a
            // non-lowered (glob / regexp / fall-through) `switch` keeps its
            // arm bodies inline; descend through whatever nests.
            for script in crate::ir_helpers::nested_bodies(stmt) {
                crate::ir::for_each_statement(script, &mut |inner| {
                    scan_statement(inner, registry, &mut barrier, config);
                });
            }
        }
        match &block.terminator {
            Some(Terminator::Branch { condition, .. }) => {
                scan_expr(condition, registry, &mut barrier, config);
            }
            // `return [set $n]` lowers to a terminator, not a statement.
            Some(Terminator::Return { value, expr, .. }) => {
                if let Some(v) = value {
                    scan_text(v, registry, &mut barrier, 0, config);
                }
                if let Some(e) = expr {
                    scan_expr(e, registry, &mut barrier, config);
                }
            }
            _ => {}
        }
    }
    barrier
}

fn scan_call(
    command: &str,
    args: &[String],
    tokens: Option<&crate::ir::CommandTokens>,
    registry: &CommandRegistry,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    // `args` is the segmenter's *reconstructed* text, which cannot
    // tell a brace-quoted `{$a}` (literal) from a substituted `$a`;
    // the per-word token kinds can — asked via the shared
    // `CommandTokens::arg_is_braced_literal`.
    let braced: Option<Vec<bool>> = tokens.map(|t| {
        (0..args.len())
            .map(|i| t.arg_is_braced_literal(i))
            .collect()
    });
    // Retained invocation identities, including a frozen computed head,
    // supply candidates. The written head text cannot donate a handler.
    if let Some(tokens) = tokens {
        // Name hazards consume the actual retained handler's phased operands
        // independently of strict execution facts. Re-querying its private
        // implementation slot by spelling can lose the public selector roles.
        if let Some(names) =
            crate::registry_invocation::possible_variable_name_operands(registry, None, tokens)
        {
            scan_possible_variable_names(&names, barrier);
        }
        if let Some(invocation) =
            crate::registry_invocation::resolved_tokens_invocation(registry, None, tokens)
        {
            scan_retained_name_effects(&invocation, barrier);
        }
    } else {
        scan_command(command, args, braced.as_deref(), registry, barrier, None);
    }
    if let Some(tokens) = tokens {
        scan_original_substitutions(tokens, registry, barrier, config);
    } else {
        for arg in args {
            scan_text(arg, registry, barrier, 0, config);
        }
    }
}

/// Original child dispatch receipts retain name effects across the enclosing
/// value operation. Missing child ownership or a truncated walk stays opaque.
fn scan_original_substitutions(
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    let Some(calls) = crate::word_subst::checked_lifted_calls(tokens, config) else {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    };
    for call in calls {
        let Some(tokens) = call.tokens.as_ref() else {
            *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
            continue;
        };
        let names =
            crate::registry_invocation::possible_variable_name_operands(registry, None, tokens);
        let invocation =
            crate::registry_invocation::resolved_tokens_invocation(registry, None, tokens);
        if let Some(names) = &names {
            scan_possible_variable_names(names, barrier);
        }
        if let Some(invocation) = &invocation {
            scan_retained_name_effects(invocation, barrier);
        }
        if invocation.is_none()
            && names.as_ref().is_none_or(
                super::registry_invocation::PossibleVariableNameOperands::unknown_residual,
            )
        {
            *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        }
    }
}

fn scan_statement(
    stmt: &Statement,
    registry: &CommandRegistry,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    match stmt {
        Statement::NativeCall { .. } => {
            *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        }
        Statement::Call {
            command,
            args,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            args,
            tokens,
            ..
        } => {
            scan_call(command, args, tokens.as_ref(), registry, barrier, config);
        }
        Statement::AssignValue { value, .. } => {
            if let Some(tokens) = stmt.tokens() {
                scan_original_substitutions(tokens, registry, barrier, config);
            } else {
                scan_text(value, registry, barrier, 0, config);
            }
        }
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            scan_expr(expr, registry, barrier, config);
        }
        Statement::Return { value, expr, .. } => {
            if let Some(v) = value {
                scan_text(v, registry, barrier, 0, config);
            }
            if let Some(e) = expr {
                scan_expr(e, registry, barrier, config);
            }
        }
        // A non-lowered `switch`'s subject is a word; its arm bodies are
        // reached by the caller's `nested_bodies` descent.
        Statement::Switch { subject, .. } => {
            scan_text(subject, registry, barrier, 0, config);
        }
        // `Incr` names its target literally: `try_lower_incr` declines the
        // specialisation for a computed name word — as `lower_set` does —
        // and falls back to `Call`, which the arm above already scans. Every
        // remaining statement carries no word text.
        _ => {}
    }
}

fn scan_expr(
    expr: &ExprNode,
    registry: &CommandRegistry,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    let mut cmds = Vec::new();
    crate::ir_helpers::collect_expr_commands(expr, &mut cmds);
    for cmd_text in &cmds {
        let trimmed = cmd_text.trim();
        let inner = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(trimmed);
        scan_script_text(inner, registry, barrier, 0, config);
    }
}

/// Scan a word's raw text for nested `[…]` command substitutions.
fn scan_text(
    text: &str,
    registry: &CommandRegistry,
    barrier: &mut DynamicNameBarrier,
    depth: u32,
    config: LexerConfig,
) {
    // Native-stack safety net: `[a [b [c …]]]` nests inside one word. This is
    // a soundness fact, not a best-effort diagnostic: past the cap the unread
    // suffix may contain any dynamic read, write, or destroy. Fail closed for
    // every consumer rather than treating a bounded walk as a proof that no
    // barrier exists.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    }
    for inner in crate::var_refs::command_subst_texts_with_config(text, config) {
        scan_script_text(&inner, registry, barrier, depth + 1, config);
    }
}

/// Scan a run of script text (a `[…]` substitution's own content) for
/// dynamic-name accesses, then descend into whatever `[…]` it nests.
fn scan_script_text(
    text: &str,
    registry: &CommandRegistry,
    barrier: &mut DynamicNameBarrier,
    depth: u32,
    config: LexerConfig,
) {
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    }
    for words in crate::ir_helpers::tokenise_command_words(text, config) {
        let Some((command, args)) = words.split_first() else {
            continue;
        };
        // A substituted head names an unknown command (see the module docs).
        if command.substituted {
            continue;
        }
        // A substituted word needs its original spelling to retain the
        // dynamic-name obligation. A literal needs the shared token owner's
        // content, so braces/quotes cannot change option or level selection.
        let arg_texts: Vec<String> = args
            .iter()
            .map(|word| {
                if word.substituted {
                    word.raw.clone()
                } else {
                    word.text.clone()
                }
            })
            .collect();
        let braced: Vec<bool> = args.iter().map(|w| w.braced_literal).collect();
        scan_command(
            &command.text,
            &arg_texts,
            Some(&braced),
            registry,
            barrier,
            None,
        );
    }
    // The script's own words may nest further substitutions; `text` still has
    // their brackets intact (a word's raw spelling does not — a delimited
    // token's closer sits one past its span), so recurse from here.
    scan_text(text, registry, barrier, depth, config);
}

/// Whether a template word handed to a substituting command was itself
/// produced by substitution — i.e. its content comes from run-time data
/// rather than from source text the ordinary scanners can read.
///
/// A brace-quoted word (`subst {$a}`) is the one literal form that can
/// legally carry a `$`, and Tcl leaves it verbatim: the `$a` inside is
/// source text naming `a`, so the read is *not* blind.  Every other word
/// carrying `$` or `[` (`subst $t`, `subst "$t"`, `subst [gen]`) reaches
/// `subst` already substituted, so the names it then expands come from data.
fn template_word_is_substituted(word: &str, braced_literal: bool) -> bool {
    !braced_literal && (word.contains('$') || word.contains('['))
}

/// Preserve source-value uncertainty for both layout and frame selection.
/// This fallback scanner has spelling/quoting, not evaluated argv receipts.
fn source_argument_words<'a>(
    args: &[&'a str],
    arg_braced: Option<&[bool]>,
) -> Vec<tcl_registry::InvocationWord<'a>> {
    args.iter()
        .enumerate()
        .map(|(index, word)| {
            if template_word_is_substituted(
                word,
                arg_braced
                    .and_then(|braced| braced.get(index))
                    .copied()
                    .unwrap_or(false),
            ) {
                tcl_registry::InvocationWord::Dynamic
            } else {
                tcl_registry::InvocationWord::Literal(word)
            }
        })
        .collect()
}

/// Raise the flags a frame-crossing command imposes on the frame it is
/// *written in*.
///
/// Only two of the four layouts do:
///
/// * [`FrameArgLayout::ScriptInCurrentFrame`] — `eval $body` runs unreadable
///   code right here.
/// * [`FrameArgLayout::OpaqueCallerVars`] — `argparse` creates locals in the
///   frame that called it, which *is* this one.
///
/// [`FrameArgLayout::AliasPairs`] (`upvar`) binds a statically named local,
/// and [`FrameArgLayout::ScriptInSelectedFrame`] (`uplevel`) runs its script
/// somewhere else — unless the level word selects this very frame
/// (`uplevel 0 $body`), or the global frame (`uplevel #0 $body`), whose
/// names a proc cannot separate from its own bare locals.  Both of those
/// land here.
fn scan_frame_effect(
    frame: FrameEffectSpec,
    args: &[&str],
    arg_braced: Option<&[bool]>,
    barrier: &mut DynamicNameBarrier,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
) {
    match frame.layout {
        // Every caller-frame variable `argparse` creates is named from its
        // definition-list mini-language, which nothing here interprets.
        FrameArgLayout::OpaqueCallerVars => barrier.writes = true,
        FrameArgLayout::ScriptInCurrentFrame => {
            if script_words_are_opaque(args, arg_braced, 0) {
                *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
            }
        }
        FrameArgLayout::ScriptInSelectedFrame => {
            let words = source_argument_words(args, arg_braced);
            let mut arguments = tcl_registry::InvocationArguments::Structured(&words)
                .with_profile(registry.profile());
            if let Some(dialect) = dialect {
                arguments = arguments.with_dialect(dialect);
            }
            let (level, taken) = match frame.resolve_arguments(arguments) {
                tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                    level,
                    level_word_len,
                } => (level, level_word_len),
                tcl_registry::frame_effect::FrameArgumentResolution::Invalid => return,
                tcl_registry::frame_effect::FrameArgumentResolution::Unknown => {
                    *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                    return;
                }
            };
            let script = &args[taken..];
            // A caller-frame or further-up `uplevel` is the callee's effect
            // on *its* caller, summarised per proc and applied at call
            // sites; it does not blind the frame it is written in.
            if !level.is_current_frame() && !level.is_global_frame() {
                return;
            }
            if script_words_are_opaque(script, arg_braced, taken) {
                *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
            }
        }
        FrameArgLayout::AliasPairs => {}
    }
}

/// Whether a script assembled from `words` is code this analysis cannot
/// read — i.e. any word substitutes.
///
/// A brace-quoted word is source text the ordinary walkers already see (the
/// lowering inlines it), so a fully literal script is not a barrier.
/// `offset` is where `words` starts within the original argument list, so
/// the per-word `braced` flags line up.
fn script_words_are_opaque(words: &[&str], arg_braced: Option<&[bool]>, offset: usize) -> bool {
    words.iter().enumerate().any(|(i, w)| {
        let braced = arg_braced
            .and_then(|b| b.get(offset + i))
            .copied()
            .unwrap_or(false);
        !braced && (w.contains('$') || w.contains('['))
    })
}

fn variable_word_has_unknown_root(
    word: &crate::registry_invocation::EffectiveInvocationWord,
) -> bool {
    use crate::registry_invocation::EffectiveInvocationWord as Word;
    match word {
        Word::Literal(_) | Word::ArrayElementName { .. } => false,
        Word::ByteLiteral(_)
        | Word::Dynamic
        | Word::Expanded
        | Word::KnownExpansion(_)
        | Word::Opaque => true,
    }
}

fn scan_possible_variable_names(
    invocation: &crate::registry_invocation::PossibleVariableNameOperands,
    barrier: &mut DynamicNameBarrier,
) {
    let unknown_roles = invocation
        .phased_operands()
        .filter_map(|(role, word, _, destroys)| {
            variable_word_has_unknown_root(word).then_some((role, destroys))
        })
        .chain(invocation.phased_unresolved_roles());
    for (role, destroys) in unknown_roles {
        match role {
            ArgRole::VarRead => barrier.reads = true,
            ArgRole::VarWrite if destroys => barrier.destroys = true,
            ArgRole::VarWrite => barrier.writes = true,
            _ => {}
        }
    }
}

/// Consume selected handler facts without querying its slot spelling again.
/// Captured argv bytes are values, not reconstructed source substitutions.
fn scan_retained_name_effects(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    barrier: &mut DynamicNameBarrier,
) {
    scan_partial_arguments(invocation, barrier);
    let facts = &invocation.facts;
    if let Some(frame) = facts.frame_effect {
        match frame.layout {
            FrameArgLayout::OpaqueCallerVars => barrier.writes = true,
            FrameArgLayout::ScriptInCurrentFrame => {
                if (0..invocation.arguments.len()).any(|index| {
                    invocation
                        .argument_word(index)
                        .as_registry_word()
                        .literal()
                        .is_none()
                }) {
                    *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                }
            }
            FrameArgLayout::ScriptInSelectedFrame => {
                invocation.with_argument_words(|words| {
                    match frame.resolve_arguments(words.arguments()) {
                        tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                            level,
                            level_word_len,
                        } if level.is_current_frame() || level.is_global_frame() => {
                            if (level_word_len..invocation.arguments.len()).any(|index| {
                                invocation
                                    .argument_word(index)
                                    .as_registry_word()
                                    .literal()
                                    .is_none()
                            }) {
                                *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                            }
                        }
                        tcl_registry::frame_effect::FrameArgumentResolution::Unknown => {
                            *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                        }
                        _ => {}
                    }
                });
            }
            FrameArgLayout::AliasPairs => {}
        }
    }
    // The actual selected introspection handler observes all names only when
    // it has no individual variable-name role. A parent/private QName cannot
    // recover this selected member's role layout.
    if facts.traits.contains(Traits::INTROSPECTS_BY_NAME)
        && !facts
            .arg_roles
            .iter()
            .any(|(_, role)| matches!(role, ArgRole::VarRead | ArgRole::VarWrite))
    {
        barrier.reads = true;
    }
}

/// Missing captured bytes retain their role's dynamic-name obligation while
/// unrelated value slots do not make every variable reachable.
fn scan_partial_arguments(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    barrier: &mut DynamicNameBarrier,
) {
    if !invocation.facts.arg_roles_complete {
        // The selected resolver's authored possible role classes own this
        // residual. Unknown channel/value positions are not variable names.
        for role in invocation.facts.arg_role_resolver_roles {
            match role {
                ArgRole::VarRead => barrier.reads = true,
                ArgRole::VarWrite
                    if invocation.facts.traits.contains(Traits::DESTROYS_VARIABLE) =>
                {
                    barrier.destroys = true;
                }
                ArgRole::VarWrite => barrier.writes = true,
                ArgRole::Body | ArgRole::Expr => {
                    *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                }
                _ => {}
            }
        }
    }
    for (index, role) in &invocation.facts.arg_roles {
        if invocation
            .arguments
            .get(invocation.facts.argument_offset + usize::from(*index))
            .is_some_and(Option::is_none)
            && variable_word_has_unknown_root(
                &invocation.argument_word(invocation.facts.argument_offset + usize::from(*index)),
            )
        {
            match role {
                ArgRole::VarRead => barrier.reads = true,
                ArgRole::VarWrite
                    if invocation.facts.traits.contains(Traits::DESTROYS_VARIABLE) =>
                {
                    barrier.destroys = true;
                }
                ArgRole::VarWrite => barrier.writes = true,
                ArgRole::Body | ArgRole::Expr => {
                    *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                }
                _ => {}
            }
        }
    }
    if invocation
        .facts
        .traits
        .contains(Traits::PERFORMS_SUBSTITUTION)
        && (0..invocation.arguments.len()).any(|index| {
            invocation
                .argument_word(index)
                .as_registry_word()
                .literal()
                .is_none()
        })
    {
        barrier.reads = true;
    }
}

/// Apply the registry's name-role answers for one `command args…` call.
///
/// `arg_braced`, when present, says for each argument whether it is a single
/// brace-quoted word — a distinction the segmenter's reconstructed `args`
/// text has already erased, and the one thing that separates a literal name
/// or template (`set {$n} 1`, `subst {$a}`) from a substituted one
/// (`set $n 1`, `subst $a`).
fn scan_command(
    command: &str,
    args: &[String],
    arg_braced: Option<&[bool]>,
    registry: &CommandRegistry,
    barrier: &mut DynamicNameBarrier,
    dialect: Option<tcl_registry::InvocationDialect>,
) {
    let Some(spec) = registry.get_for_surface(
        command,
        dialect
            .and_then(tcl_registry::InvocationDialect::authoring_query)
            .or_else(|| registry.own_surface_query()),
    ) else {
        return;
    };
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    let words = source_argument_words(&arg_strs, arg_braced);
    let arguments = tcl_registry::InvocationArguments::structured(&words);
    let arguments = dialect.map_or(arguments, |dialect| arguments.with_dialect(dialect));
    if let Some(frame) = spec.frame_effect {
        scan_frame_effect(frame, &arg_strs, arg_braced, barrier, registry, dialect);
    }
    let destroys = spec.traits.contains(Traits::DESTROYS_VARIABLE);
    // A brace-quoted word is Tcl's literal spelling for a name that contains
    // `$` or `[`: `set {$n} v` creates a variable *called* `$n`, unrelated to
    // `n` (tclsh 9.0.4 / 8.6.14: `set {$n} v; info exists {$n}` → 1 while
    // `info exists n` → 0). Such a name is statically known, so it is not a
    // barrier — reading the word's text alone would see the `$` and blind the
    // whole function for code that only ever names variables statically.
    let dynamic_name_at = |idx: usize| {
        !arg_braced
            .and_then(|b| b.get(idx))
            .copied()
            .unwrap_or(false)
            && arg_strs
                .get(idx)
                .is_some_and(|w| names_a_dynamic_variable(w))
    };

    let Some(writes) = registry.arg_indices_for_role_words(command, arguments, ArgRole::VarWrite)
    else {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    };
    let Some(reads) = registry.arg_indices_for_role_words(command, arguments, ArgRole::VarRead)
    else {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    };
    for idx in writes {
        if dynamic_name_at(idx) {
            if destroys {
                barrier.destroys = true;
            } else {
                barrier.writes = true;
            }
        }
    }
    for idx in reads {
        if dynamic_name_at(idx) {
            barrier.reads = true;
        }
    }
    // An ENUMERATING variable introspection — an
    // [`Traits::INTROSPECTS_BY_NAME`] subcommand that names no specific
    // variable (`info locals` / `info vars` / `info globals`, all
    // pattern-only) — observes which variables *exist* in the frame, so
    // every store is observable: deleting a "dead" `set longvariable 1`
    // changes what `info locals` returns.
    // Subcommands that do take a name argument (`info exists x`) are
    // covered precisely by the `VarRead` role walk above and raise
    // nothing here.  A dynamic subcommand word (`info $sub`) could be any
    // of them, so it counts as enumerating.
    let introspecting: Vec<&str> =
        registry.subcommands_with_trait(command, Traits::INTROSPECTS_BY_NAME);
    if !introspecting.is_empty() {
        let enumerating = match arg_strs.first() {
            Some(word) if !word.contains('$') && !word.contains('[') => {
                introspecting.contains(word)
                    && spec.subcommand(word).is_some_and(|sub| {
                        sub.arg_roles
                            .iter()
                            .all(|(_, role)| !matches!(role, ArgRole::VarRead | ArgRole::VarWrite))
                    })
            }
            Some(_) => true, // dynamic subcommand word — assume the worst.
            None => false,
        };
        if enumerating {
            barrier.reads = true;
        }
    }
    // A template-expanding command (`subst`) performs `$name` substitution
    // over its argument string. With a literal template the names are in the
    // text and the ordinary scanners see them; with a computed one the names
    // come from run-time data, so every local is reachable —
    // `[subst $[subst $locVar]]` is exactly this shape.
    if spec.traits.contains(Traits::PERFORMS_SUBSTITUTION)
        && arg_strs.iter().enumerate().any(|(i, w)| {
            !w.starts_with('-')
                && template_word_is_substituted(
                    w,
                    arg_braced.and_then(|b| b.get(i)).copied().unwrap_or(false),
                )
        })
    {
        barrier.reads = true;
    }
}

#[cfg(test)]
mod braced_var_close_rule_tests {
    use super::dynamic_variable_word_can_spell as can_spell;
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};

    /// The wildcard extents come from the shared owner, so the
    /// *literal* characters that bound what a dynamic word can spell move with
    /// the release.
    ///
    /// Under the default (9.x) rule `${a{b}c}` is one wildcard and can spell
    /// any cell; under 8.x the name ends at the first `}` and the literal `c}`
    /// remains, which no ordinary cell name ends with. A narrower pattern is
    /// the unsafe direction: it judges a cell the word really can reach as out
    /// of reach, and rename proceeds.
    #[test]
    fn wildcard_extent_follows_the_release_close_rule() {
        assert!(can_spell("${a{b}c}", "::v", Tcl9Nesting));
        assert!(!can_spell("${a{b}c}", "::v", FirstClose));
        // With the 8.x remainder actually present in the cell name, the
        // narrower pattern does match — the literal run is the whole bound.
        assert!(can_spell("${a{b}c}", "::xc}", FirstClose));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// These cases carry no `{`/`}`/`\\` in a `${…}` name, so both release
    /// rules answer alike; pin them to the default document's rule.
    fn dynamic_variable_word_can_spell(word: &str, cell: &str) -> bool {
        super::dynamic_variable_word_can_spell(word, cell, tcl_dialect::BracedVarStyle::default())
    }

    fn barrier_for(src: &str) -> DynamicNameBarrier {
        barrier_for_dialect(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        )
    }

    /// The barrier for `src` analysed **as** `dialect` — registry, lexer
    /// config, and expression grammar all that dialect's, which is the
    /// configuration a real host builds.
    fn barrier_for_dialect(
        src: &str,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> DynamicNameBarrier {
        let registry = tcl_registry::model::ingress::static_context_for_profile(dialect).commands();
        let cu = crate::compilation_unit::CompilationUnit::build_for_profile(
            src, registry, false, dialect,
        );
        let fu = cu.procedures.values().next().unwrap_or(&cu.top_level);
        dynamic_name_barrier(&fu.cfg, registry, lexer_config_for(registry))
    }

    #[test]
    fn original_substitution_name_effects_keep_native_child_receipts() {
        assert!(barrier_for("set x [list]; lappend x a; puts $x").is_clear());
        assert!(barrier_for("set x {[set $name 2]}; puts $x").is_clear());
        assert!(barrier_for("set {name[} DATA; puts ${name[}").is_clear());
        assert!(barrier_for("puts [list [set $name 2]]").writes);
        assert!(barrier_for("set x [unknown_child]").reads);
        assert!(barrier_for("set x $a([set $name 2])").writes);
    }

    #[test]
    fn braced_substitution_is_dynamic_plain_name_is_not() {
        // `${x}` is a substitution, not a literal name — tclsh 9.0.4 / 8.6.14:
        // `set x foo; set ${x} bar; info exists foo` → 1.
        assert!(names_a_dynamic_variable("${x}"));
        assert!(!names_a_dynamic_variable("plain"));
        assert!(!names_a_dynamic_variable(""));
    }

    #[test]
    fn qualified_name_with_a_dynamic_namespace_binds_a_static_tail() {
        // `variable ${name}::graphAttr` creates the local `graphAttr`
        // whatever `$name` holds, so no other local becomes reachable.
        assert!(!names_a_dynamic_variable("${name}::graphAttr"));
        assert!(names_a_dynamic_variable("::ns::${tail}"));
    }

    #[test]
    fn array_element_with_dynamic_key_is_not_a_name_barrier() {
        // The base array `a` is statically named; only the element key is
        // computed, which the place model already tracks.
        assert!(!names_a_dynamic_variable("a($k)"));
        assert!(names_a_dynamic_variable("$a(k)"));
    }

    #[test]
    fn substituted_and_bracketed_names_are_dynamic() {
        assert!(names_a_dynamic_variable("$x"));
        assert!(names_a_dynamic_variable("[string trim $x]"));
        assert!(names_a_dynamic_variable("pre$x"));
    }

    // Per-site provenance: which cells a dynamic name word can actually
    // spell.  `true` is the abstention (could reach it), `false` is the proof
    // it cannot.

    /// TP (must stay refused): a bare substitution is a lone wildcard, so it
    /// reaches every spelling of every cell.
    #[test]
    fn a_bare_substitution_can_spell_any_cell() {
        for word in ["$n", "${n}", "[pick]"] {
            assert!(dynamic_variable_word_can_spell(word, "::ns::v"), "{word}");
            assert!(
                dynamic_variable_word_can_spell(word, "::a::b::deep"),
                "{word}"
            );
        }
    }

    /// TN: a written namespace prefix confines the name — a substitution
    /// cannot escape it.
    ///
    /// tclsh-proof (8.6.14): `namespace eval ::ns {variable v 1}; namespace
    /// eval ::other {}; set n {::ns::v}; set ::other::$n 99` fails with
    /// `can't set "::other::::ns::v": parent namespace doesn't exist` — the
    /// value is appended under the prefix, never resolved as an absolute
    /// name.  `set ::other::$n` with `n` = `sub::x` writes `::other::sub::x`.
    #[test]
    fn a_written_namespace_prefix_confines_the_name() {
        assert!(!dynamic_variable_word_can_spell("::other::$n", "::ns::v"));
        assert!(!dynamic_variable_word_can_spell("::other::${n}", "::ns::v"));
        // …and still reaches cells that really are under it, at any depth.
        assert!(dynamic_variable_word_can_spell("::other::$n", "::other::v"));
        assert!(dynamic_variable_word_can_spell(
            "::other::$n",
            "::other::sub::x"
        ));
    }

    /// TN: a written affix constrains the name the same way.
    ///
    /// tclsh-proof (8.6.14): `namespace eval ::ns {variable total 5}; set j
    /// 1; set v$j 2` leaves `info vars ::v*` = `::v1` and `::ns::total`
    /// untouched.
    #[test]
    fn a_written_affix_constrains_the_name() {
        assert!(!dynamic_variable_word_can_spell("v$j", "::ns::total"));
        // `${j}_count`, not `$j_count` — the latter is the single variable
        // `j_count`, so it is one wildcard with no written affix at all.
        assert!(!dynamic_variable_word_can_spell(
            "${j}_count",
            "::ns::total"
        ));
        assert!(dynamic_variable_word_can_spell("$j_count", "::ns::total"));
        // TP guard: the affix is satisfiable, so the abstention stands — an
        // empty substitution collapses (`set i {}; set v$i 1` -> `info exists
        // v` is 1 on 8.6.14).
        assert!(dynamic_variable_word_can_spell("v$j", "::ns::v"));
        assert!(dynamic_variable_word_can_spell("v$j", "::ns::verbose"));
        assert!(dynamic_variable_word_can_spell("$j", "::ns::total"));
    }

    /// TN: a *relative* prefix is a prefix too — `b::$m` names something
    /// under some `…::b`, so a cell with no `b` segment is out of reach.
    #[test]
    fn a_relative_prefix_still_excludes_unrelated_cells() {
        assert!(!dynamic_variable_word_can_spell("b::$m", "::ns::v"));
        assert!(dynamic_variable_word_can_spell("b::$m", "::a::b::tail"));
    }

    /// The relative spellings count: a cell written from inside its own
    /// namespace is a bare tail, so a wildcard-tail word reaches it.
    #[test]
    fn every_relative_spelling_of_the_cell_is_considered() {
        assert!(dynamic_variable_word_can_spell("ns::$t", "::ns::v"));
        assert!(dynamic_variable_word_can_spell("$t", "::ns::v"));
        assert!(!dynamic_variable_word_can_spell("nz::$t", "::ns::v"));
    }

    /// An element suffix is not part of the variable name.
    #[test]
    fn the_element_suffix_is_not_part_of_the_name() {
        assert!(dynamic_variable_word_can_spell("$a(k)", "::ns::v"));
        assert!(!dynamic_variable_word_can_spell(
            "::other::$a(k)",
            "::ns::v"
        ));
    }

    #[test]
    fn plain_proc_has_no_barrier() {
        assert!(barrier_for("proc f {} { set a 1; return $a }\n").is_clear());
    }

    #[test]
    fn dynamic_set_sets_the_write_flag_only() {
        let b = barrier_for("proc f {n} { set $n 1; return ok }\n");
        assert!(b.writes, "`set $n 1` is a dynamic write");
        assert!(!b.reads);
        assert!(!b.destroys);
    }

    #[test]
    fn fallback_role_scan_distinguishes_unknown_options_from_ordinary_values() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let dialect = Some(tcl_registry::InvocationDialect::of_profile(profile));
        let scan = |command, arguments: &[&str], braced: &[bool]| {
            let arguments: Vec<_> = arguments.iter().map(|word| (*word).to_owned()).collect();
            let mut barrier = DynamicNameBarrier::default();
            scan_command(
                command,
                &arguments,
                Some(braced),
                registry,
                &mut barrier,
                dialect,
            );
            barrier
        };
        assert_eq!(
            scan(
                "regsub",
                &["$option", "x", "x", "value", "target"],
                &[false; 5]
            ),
            DynamicNameBarrier::OPAQUE_SCRIPT,
            "a computed switch cannot be treated as literal source spelling"
        );
        let ordinary = scan("set", &["$name", "$value"], &[false; 2]);
        assert!(ordinary.writes);
        assert!(!ordinary.reads && !ordinary.destroys);
        assert!(scan("set", &["$name", "$value"], &[true, false]).is_clear());
        let mut quoted_level = DynamicNameBarrier::default();
        scan_script_text(
            "uplevel {0} $body",
            registry,
            &mut quoted_level,
            0,
            LexerConfig::from_grammar(profile.grammar),
        );
        assert_eq!(
            quoted_level,
            DynamicNameBarrier::OPAQUE_SCRIPT,
            "the quoted zero still selects the current frame"
        );
        let mut quoted_option = DynamicNameBarrier::default();
        scan_script_text(
            "regsub {-nocase} x x value target",
            registry,
            &mut quoted_option,
            0,
            LexerConfig::from_grammar(profile.grammar),
        );
        assert!(quoted_option.is_clear(), "{quoted_option:?}");
    }

    #[test]
    fn dynamic_one_arg_set_sets_the_read_flag_only() {
        let b = barrier_for("proc f {n} { return [set $n] }\n");
        assert!(b.reads, "`[set $n]` is a dynamic read");
        assert!(!b.writes);
        assert!(!b.destroys);
    }

    #[test]
    fn dynamic_unset_sets_the_destroy_flag_only() {
        let b = barrier_for("proc f {n} { unset $n; return ok }\n");
        assert!(b.destroys, "`unset $n` is a dynamic destroy");
        assert!(!b.writes);
    }

    #[test]
    fn dynamic_subst_template_sets_the_read_flag() {
        let b = barrier_for("proc f {t} { return [subst $t] }\n");
        assert!(b.reads, "`subst $t` can dereference any name");
    }

    #[test]
    fn literal_subst_template_is_not_a_barrier() {
        let b = barrier_for("proc f {} { set a 1; return [subst {$a}] }\n");
        assert!(b.is_clear(), "a literal template names its reads");
    }

    #[test]
    fn dynamic_array_set_is_a_write_barrier() {
        let b = barrier_for("proc f {n} { array set $n {x 1} }\n");
        assert!(b.writes);
        let literal = barrier_for("proc f {} { array set table {x 1} }");
        assert!(
            !literal.writes,
            "the selected handler's literal destination does not blind every name"
        );
    }

    #[test]
    fn a_captured_literal_name_is_not_a_source_substitution() {
        let literal = barrier_for("interp alias {} assign {} set {$n}\nproc f {} {assign VALUE}");
        assert!(
            !literal.writes,
            "captured $n is a literal variable name: {literal:?}"
        );
        let dynamic = barrier_for("proc f {n} {set $n VALUE}");
        assert!(dynamic.writes, "the original dynamic name remains a hazard");
    }

    #[test]
    fn a_computed_head_uses_its_retained_handler_for_name_hazards() {
        let barrier =
            barrier_for("proc f {name} {set operation array; $operation set $name {x 1}}");
        assert!(barrier.writes);
        assert!(!barrier.destroys);
        let unrelated = barrier_for("proc f {array name} {$array set $name {x 1}}");
        assert!(
            !unrelated.writes,
            "a variable name cannot donate array roles"
        );
    }

    #[test]
    fn dynamic_name_nested_in_a_command_substitution_is_seen() {
        let b = barrier_for("proc f {n} { set out {}; lappend out [set $n]; return $out }\n");
        assert!(b.reads, "the dynamic read sits inside a `[…]` argument");
    }

    #[test]
    fn dynamic_name_in_a_branch_condition_is_seen() {
        let b = barrier_for("proc f {n} { if {[set $n] eq {}} { return empty }; return full }\n");
        assert!(b.reads);
    }

    // A brace-quoted name is a *literal* name.
    //
    // tclsh 9.0.4 / 8.6.14 (identical):
    //   set {$n} v; info exists {$n} → 1 ; info exists n → 0
    //   set i 5; set {arr($i)} 1; array names arr → {$i} ; info exists arr(5) → 0

    #[test]
    fn brace_quoted_write_target_is_not_a_barrier() {
        let b = barrier_for("proc f {} { set {$n} 1; return ok }\n");
        assert!(
            b.is_clear(),
            "`set {{$n}} 1` names the literal variable `$n`, so nothing is \
computed; got {b:?}"
        );
    }

    #[test]
    fn brace_quoted_read_target_is_not_a_barrier() {
        let b = barrier_for("proc f {} { return [set {$n}] }\n");
        assert!(b.is_clear(), "got {b:?}");
    }

    #[test]
    fn brace_quoted_destroy_target_is_not_a_barrier() {
        let b = barrier_for("proc f {} { unset {$n}; return ok }\n");
        assert!(b.is_clear(), "got {b:?}");
    }

    #[test]
    fn unbraced_substituted_target_still_raises_the_write_flag() {
        // TN control for the three above: dropping the braces restores the
        // barrier, so the brace check cannot be over-applied.
        let b = barrier_for("proc f {n} { set $n 1; return ok }\n");
        assert!(b.writes, "`set $n 1` is still a dynamic write; got {b:?}");
    }

    // A computed command head is an unknown command.

    #[test]
    fn substituted_command_head_raises_no_flag() {
        // The head's content spelling is `set`, but `string length foo` is
        // what runs. Resolving the lookalike would answer for a command that
        // never executes, so the call contributes nothing — see the module
        // docs for why it raises no flag either.
        let b = barrier_for("proc f {set n} { $set $n 1; return ok }\n");
        assert!(
            b.is_clear(),
            "a computed head is an unknown command, not a computed name; got {b:?}"
        );
    }

    #[test]
    fn literal_head_with_a_computed_name_still_raises_the_write_flag() {
        // TN control: the same argument shape under a *literal* head is the
        // genuine dynamic write.
        let b = barrier_for("proc f {n} { set $n 1; return ok }\n");
        assert!(b.writes, "got {b:?}");
    }

    // The barrier is computed under the *document's* dialect.
    //
    // A tokenisation pinned to `LexerConfig::default()` would see word
    // boundaries the IR was not built from under a non-default dialect.  The
    // barrier is an optimisation-soundness fact — SCCP and every value-motion
    // pass abstain on it — so a boundary disagreement that hides a dynamic
    // write lets a constant propagate across one.

    /// The two shapes the dialects disagree about, as facts about the
    /// splitter now shared with the lowering.
    #[test]
    fn word_splitting_follows_the_dialect() {
        let words =
            |src: &str, dialect: &'static tcl_dialect::DialectProfile| -> Vec<Vec<String>> {
                crate::ir_helpers::tokenise_command_words(
                    src,
                    tcl_lexer::LexerConfig::from_grammar(dialect.grammar),
                )
                .into_iter()
                .map(|command| command.into_iter().map(|word| word.text).collect())
                .collect()
            };

        // iRules treats `}{` as a word separator; Tcl concatenates the two
        // braced groups into one word.
        assert_eq!(
            words("cmd {a}{b}", tcl_dialect::DialectProfile::irules()),
            vec![vec!["cmd", "a", "b"]],
            "an iRule splits `{{a}}{{b}}` into two words",
        );
        assert_eq!(
            words(
                "cmd {a}{b}",
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
            ),
            vec![vec!["cmd", "ab"]],
            "Tcl welds them into one",
        );

        // `{*}` expands only where the dialect has it — never in 8.4 or an
        // iRule, where it is an ordinary (literal) word.
        assert_eq!(
            words(
                "cmd {*}$args x",
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
            ),
            vec![vec!["cmd", "${args}", "x"]],
            "8.5+ reads `{{*}}` as the expansion marker",
        );
        assert_eq!(
            words(
                "cmd {*}$args x",
                tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile()
            ),
            vec![vec!["cmd", "*${args}", "x"]],
            "tcl8.4 has no `{{*}}` expansion",
        );
        // F5 reclassification (measurements §1/§3 row 6,
        // `docs/design/f5/bigip-irule-parser-measurements.md`): on the F5
        // fork the implicit word break wins over everything — `{*}` is
        // the literal `*` word plus the *separate* unexpanded word
        // (measured `list {*}{a b}` → `* {a b}` on TMM), never the old
        // welded `*${args}` single word and never an expansion.
        assert_eq!(
            words("cmd {*}$args x", tcl_dialect::DialectProfile::irules()),
            vec![vec!["cmd", "*", "${args}", "x"]],
            "the F5 word break splits after the literal `*`",
        );
    }

    /// The per-word facts `scan_command` reads, as the segmenter now supplies
    /// them: the written spelling (which carries the name-position `$`), the
    /// brace-literal flag (which says a `$` is *not* a substitution), and the
    /// substitution flag (which disqualifies a computed command head).
    #[test]
    fn word_facts_survive_the_segmenter_mapping() {
        let words = crate::ir_helpers::tokenise_command_words(
            "set $x {a $b}",
            tcl_lexer::LexerConfig::default(),
        )
        .remove(0);
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            vec!["set", "${x}", "a $b"],
        );
        assert_eq!(
            words.iter().map(|w| w.raw.as_str()).collect::<Vec<_>>(),
            vec!["set", "$x", "{a $b"],
        );
        assert_eq!(
            words.iter().map(|w| w.substituted).collect::<Vec<_>>(),
            vec![false, true, false],
        );
        assert_eq!(
            words.iter().map(|w| w.braced_literal).collect::<Vec<_>>(),
            vec![false, false, true],
        );

        // A compound word substitutes as a whole and is not a brace literal,
        // whichever fragment carries the `$`.
        for src in ["cmd a$b", "cmd {a}$b", "cmd $b-a"] {
            let word =
                crate::ir_helpers::tokenise_command_words(src, tcl_lexer::LexerConfig::default())
                    .remove(0)
                    .remove(1);
            assert!(word.substituted, "{src}");
            assert!(!word.braced_literal, "{src}");
        }
    }

    /// The soundness pin: a dynamic write inside a shape whose word
    /// boundaries are dialect-dependent still raises the write flag.
    #[test]
    fn a_dynamic_write_raises_the_barrier_under_every_dialect() {
        for dialect in ["tcl8.6", "tcl8.4", "f5-irules"] {
            let b = barrier_for_dialect(
                "proc f {n} { set $n 1; return ok }\n",
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(b.writes, "`set $n 1` is a dynamic write under {dialect}");

            // …and through a `[…]` substitution, which is where the barrier
            // walk re-splits script text itself.
            let b = barrier_for_dialect(
                "proc f {n} { set out {}; lappend out [set $n 1]; return $out }\n",
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(b.writes, "nested dynamic write missed under {dialect}");
        }
    }

    /// The dialect-dependent word shapes themselves: a `}{` weld (which only
    /// an iRule splits) and a `{*}` (which only 8.5+ expands), each wrapped
    /// around a nested dynamic write.  Whichever way the words fall, the
    /// write inside the `[…]` is still there and the barrier must find it —
    /// this is the walk re-splitting script text under the document's own
    /// config.
    #[test]
    fn dialect_specific_word_shapes_do_not_hide_a_dynamic_write() {
        for dialect in ["tcl8.6", "tcl8.4", "f5-irules"] {
            for body in [
                "set out [list {a}{b} [set $n 1]]",
                "set out [list {*}$n [set $n 1]]",
                "set out [concat {a}{b} {*}$n [set $n 1]]",
            ] {
                let b = barrier_for_dialect(
                    &format!("proc f {{n}} {{ {body}; return $out }}\n"),
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                );
                assert!(b.writes, "{dialect}: `{body}` hides a dynamic write");
            }
        }
    }

    /// The discriminating case, and the reason the config is threaded at all:
    /// a `{*}` welded to a braced name.  Under **tcl8.4** there is no
    /// expansion, so the word is the computed name `*$n` and the `[…]` holds a
    /// dynamic write; read under the default (8.5+) grammar the very same text
    /// is an expansion marker plus the brace-literal name `{$n}`, which is
    /// *static* — so a barrier computed from a differently-configured
    /// tokenisation reports "no dynamic write" for a script that has one, and
    /// SCCP then propagates constants across it.
    #[test]
    fn an_84_expansionless_name_word_is_still_a_dynamic_write() {
        let b = barrier_for_dialect(
            "proc f {n} { set out [list [set {*}{$n} 1]]; return $out }\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        );
        assert!(
            b.writes,
            "`{{*}}{{$n}}` names `*$n` under 8.4 — a computed name; got {b:?}",
        );
    }

    /// TN control: a brace-literal name is static under every dialect, so the
    /// dialect threading cannot be over-applied into blanket blindness.
    #[test]
    fn a_literal_name_stays_clear_under_every_dialect() {
        for dialect in ["tcl8.6", "tcl8.4", "f5-irules"] {
            let b = barrier_for_dialect(
                "proc f {} { set {$n} 1; return ok }\n",
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(b.is_clear(), "{dialect}: got {b:?}");
        }
    }

    /// The same expansionless `{*}` word as a *statement* of its own, not
    /// buried in a `[…]` the text walk re-segments.
    ///
    /// Under 8.4 / iRules `set {*}$n 1` is the literal `*` welded to `$n`: a
    /// computed name.  Specialising it to `AssignConst`, whose name is static
    /// by contract, would keep [`scan_statement`] from looking at it and leave
    /// the write barrier down — freeing the value-motion passes to move stores
    /// across a write that can land on any name.
    #[test]
    fn an_expansionless_computed_name_statement_raises_the_write_barrier() {
        let b = barrier_for_dialect(
            "proc f {n} { set {*}$n 1; return ok }\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        );
        assert!(
            b.writes,
            "tcl8.4: `set {{*}}$n 1` names `*$n` — a computed name; got {b:?}",
        );
        // F5 reclassification (measurements §1/§3 row 6): under the fork's
        // implicit word break `{*}$n` is TWO words — the literal `*` and
        // `$n` — so there is no welded computed name here any more; the
        // genuinely computed spelling still raises the barrier.
        let b = barrier_for_dialect(
            "proc f {n} { set x$n 1; return ok }\n",
            tcl_dialect::DialectProfile::irules(),
        );
        assert!(
            b.writes,
            "f5-irules: `set x$n 1` is a computed name; got {b:?}",
        );
    }

    /// The 9.0 grammar reaches the same answer by a different path: `{*}$n`
    /// really is an expansion there, so the word is a plain `$n` substitution
    /// and `set` is not specialised at all.  Pinned so the expansionless gate
    /// cannot be credited with this result, or quietly change it.
    #[test]
    fn an_expanded_computed_name_statement_still_raises_the_write_barrier() {
        for dialect in ["tcl9.0", "tcl8.6"] {
            let b = barrier_for_dialect(
                "proc f {n} { set {*}$n 1; return ok }\n",
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(
                b.writes,
                "{dialect}: expanded `{{*}}$n` write missed; got {b:?}"
            );
        }
    }

    #[test]
    fn an_expansion_after_a_fixed_name_does_not_make_the_name_dynamic() {
        for dialect in ["tcl9.0", "tcl8.6"] {
            for body in [
                "set fixed {*}$n",
                "set {*}{fixed} VALUE",
                "set fixed VALUE EXTRA",
            ] {
                let b = barrier_for_dialect(
                    &format!("proc f {{n}} {{ {body}; return ok }}\n"),
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                );
                assert!(b.is_clear(), "{dialect}: `{body}` got {b:?}");
            }
        }
    }

    /// TN control: a fully spelled-out `set` names one variable, so
    /// the gate must not blind a function that computes nothing.  The array
    /// element with a computed *key* is the near miss — its array is named
    /// statically, which is the line [`names_a_dynamic_variable`] draws.
    #[test]
    fn a_spelled_out_set_stays_clear_under_every_dialect() {
        for dialect in ["tcl9.0", "tcl8.6", "tcl8.4", "f5-irules"] {
            for body in ["set x 1", "set a($i) 1", "set x 1; set y $x"] {
                let b = barrier_for_dialect(
                    &format!("proc f {{i}} {{ {body}; return ok }}\n"),
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                );
                assert!(b.is_clear(), "{dialect}: `{body}` got {b:?}");
            }
        }
    }

    /// `incr $n` is a computed name, not a literal target.  Specialising it
    /// to `Statement::Incr { name: "${n}" }`, whose name is static by
    /// contract, would keep [`scan_statement`]'s `Incr` arm — which reads no
    /// word text at all — from looking at it and leave the write barrier down,
    /// freeing the value-motion passes to move stores across a write that can
    /// land on any name. Unlike the `{*}$n` shape, a bare `$n` substitutes
    /// under every grammar, so all four dialects see the same computed name.
    #[test]
    fn incr_of_a_computed_name_raises_the_write_barrier() {
        for dialect in ["tcl8.4", "f5-irules", "tcl8.6", "tcl9.0"] {
            let b = barrier_for_dialect(
                "proc f {n} { incr $n; return ok }\n",
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            );
            assert!(
                b.writes,
                "{dialect}: `incr $n` names `$n` — a computed name; got {b:?}",
            );
        }
    }

    /// TN control: a fully spelled-out `incr` names one variable,
    /// and a computed array *element* (`incr a($i)`) is not a computed
    /// *name* — the array is named statically, the same line
    /// [`names_a_dynamic_variable`] draws for `set` — so neither shape may
    /// blind a function that computes nothing.
    #[test]
    fn a_spelled_out_incr_stays_clear_under_every_dialect() {
        for dialect in ["tcl9.0", "tcl8.6", "tcl8.4", "f5-irules"] {
            for body in ["incr x", "incr a($i)", "incr x 1"] {
                let b = barrier_for_dialect(
                    &format!("proc f {{i}} {{ set x 0; {body}; return ok }}\n"),
                    tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
                );
                assert!(b.is_clear(), "{dialect}: `{body}` got {b:?}");
            }
        }
    }

    /// The bracket-depth cap protects the native stack, but it
    /// cannot certify that the unread suffix has no dynamic variable access.
    /// The source mutation is deliberately under the cap boundary: replacing
    /// the dynamic `set $name` with a literal `set x` must not make a bounded
    /// scan claim either source is fully known.
    #[test]
    fn bracket_depth_exhaustion_fails_closed_for_dynamic_name_facts() {
        let nest = |leaf: &str| {
            let mut text = leaf.to_owned();
            for _ in 0..=MAX_BRACKET_TEXT_DEPTH.0 {
                text = format!("[list {text}]");
            }
            text
        };
        let dynamic = barrier_for(&format!(
            "proc f {{name}} {{ set sink {}; return ok }}\n",
            nest("[set $name 2]")
        ));
        let literal = barrier_for(&format!(
            "proc f {{}} {{ set sink {}; return ok }}\n",
            nest("[set x 2]")
        ));
        assert_eq!(dynamic, DynamicNameBarrier::OPAQUE_SCRIPT, "{dynamic:?}");
        assert_eq!(literal, DynamicNameBarrier::OPAQUE_SCRIPT, "{literal:?}");
    }
}
