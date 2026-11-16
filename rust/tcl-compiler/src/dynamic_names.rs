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
//! | a stub's `-frame caller` | the frame that *called* it | this module, from [`CfgFunction::declared_frame_effects`] |
//! | `uplevel 1 $body` in a proc | that proc's caller | the proc's [frame-effect summary][sum], read at each call site |
//! | `upvar 1 $computed x` | that proc's caller | the same summary |
//!
//! The first three are visible in the function's own statements, so the walk
//! below raises the flags itself: a command the document declares as a plain
//! call brings the frame effect its declaration states where the catalogue
//! holds no command of that name ([`crate::ir::DeclaredFrameEffects`]), which
//! the CFG builder records on the function for this walk.  The last two are visible only with the
//! module-wide proc summaries, which the CFG builder holds and this
//! per-function walk does not — so it records them on
//! [`CfgFunction::caller_frame_barrier`], and
//! [`dynamic_name_barrier`] folds them in.  Either way the fact lands in
//! the same three bits, so every consumer's abstention rule is unchanged
//! and the cost stays `O(1)` per query.
//!
//! The function's flags are the flow-insensitive union, which every
//! whole-function consumer (`W210` / `W211` / `W220`, `O109` / `O126`)
//! reads once and abstains on. The one flow-sensitive consumer, the
//! existence rung, reads each statement's own flags
//! ([`statement_barrier`], [`terminator_barrier`]) and applies them from
//! that statement on, so a computed name after a `[info exists …]` test no
//! longer blinds the test.
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
use crate::ir::{DeclaredFrameEffects, Statement};

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
/// This explicit standalone C local-name classifier does not establish
/// supplied command semantics or the absence of a namespace-cell hazard.
/// Actual source edit consumers use [`source_name_word_is_dynamic`] with a
/// complete retained input and an independently checked current source owner.
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
    let base = literal_variable_root(word);
    let tail = tcl_syntax::naming::written_command_tail(base.as_bytes());
    tail.contains(&b'$') || tail.contains(&b'[')
}

/// Explicit standalone authored-C lexical bound. It grants no actual target,
/// Native/Jim/hosted name-policy or edit authority. Supplied consumers use
/// [`source_name_pattern_can_spell`], where unavailable purpose stays unknown.
///
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
/// [`tcl_dialect::BracedVarStyle::default`]. Explicit standalone callers
/// provide their authoring grammar; supplied edit consumers use the actual
/// retained config through [`source_name_pattern_can_spell`].
#[must_use]
pub fn dynamic_variable_word_can_spell(
    word: &str,
    qualified_cell: &str,
    braced_var: tcl_dialect::BracedVarStyle,
) -> bool {
    // An element suffix names an element of the base array, so the *variable*
    // this word names is the base — the same split `names_a_dynamic_variable`
    // makes.
    let base = literal_variable_root(word);
    let pattern = name_word_pattern(
        base,
        LexerConfig {
            braced_var,
            ..LexerConfig::default()
        },
    );
    cell_spellings(qualified_cell).any(|spelling| pattern_matches(&pattern, &spelling))
}

/// Source-name pattern purpose. A namespace name has no variable-element
/// suffix; parentheses remain ordinary namespace bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceNamePatternPurpose {
    /// A variable cell root; only a complete array-element suffix is removed.
    VariableRoot,
    /// A namespace name; parentheses remain literal name characters.
    NamespaceName,
}

fn literal_variable_root(word: &str) -> &str {
    tcl_syntax::naming::split_element_ref(word).map_or(word, |(root, _)| root)
}

fn source_pattern_word(word: &str, purpose: SourceNamePatternPurpose) -> &str {
    match purpose {
        SourceNamePatternPurpose::VariableRoot => literal_variable_root(word),
        SourceNamePatternPurpose::NamespaceName => word,
    }
}

fn authored_c_pattern_config(
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<LexerConfig> {
    use tcl_syntax::naming::{NamePolicyAuthority, NativeNameProtocol};
    let metadata = metadata?;
    let input = metadata.source_analysis_input()?;
    if !metadata.permits_logical_source_names() {
        return None;
    }
    let policy =
        tcl_registry::InvocationDialect::of_profile(input.unit_profile()).authored_name_policy()?;
    (policy.authority() == NamePolicyAuthority::AuthoredSimulation
        && matches!(policy.recipe(), NativeNameProtocol::C(_)))
    .then_some(input.lexer_config())
}

/// Whether an original source operand computes the named root, only in its
/// supplied Logical authored-C domain. A fixed local tail cannot rule out a
/// namespace-cell hazard. Unavailable or other naming purposes return
/// `None`; they never mean a Native/Jim/hosted name is statically known.
#[must_use]
pub fn source_name_word_is_dynamic(
    word: &str,
    purpose: SourceNamePatternPurpose,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<bool> {
    authored_c_pattern_config(metadata)?;
    Some(source_pattern_word(word, purpose).contains(['$', '[']))
}

/// Conditional lexical pattern advice for a constructed Logical C source
/// coordinate, including a separately retained dominating source value. This
/// is neither Native target identity nor an edit licence. The caller retains
/// current whole source/schema/origin currency independently. `None` is an
/// unresolved hazard, including availability-only and standalone contexts.
#[must_use]
pub fn source_name_pattern_can_spell(
    word: &str,
    source_coordinate: &str,
    purpose: SourceNamePatternPurpose,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<bool> {
    let config = authored_c_pattern_config(metadata)?;
    let pattern = name_word_pattern(source_pattern_word(word, purpose), config);
    Some(cell_spellings(source_coordinate).any(|spelling| pattern_matches(&pattern, &spelling)))
}

/// Conditional overlap of one original command-name operand and a constructed
/// Logical source coordinate. Unknown substitutions remain possible targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OriginalCommandNameOverlap {
    pub(crate) may_equal: bool,
    pub(crate) dynamic: bool,
    pub(crate) empty: bool,
}

/// The same question for a release profile, where no fragment of a computed
/// word is read: a static word compares by value and any other word may
/// name every command.
pub(crate) fn release_command_name_overlap(
    word: &tcl_lexer::NativeWord,
    coordinate: &str,
) -> OriginalCommandNameOverlap {
    let dynamic = OriginalCommandNameOverlap {
        may_equal: true,
        dynamic: true,
        empty: false,
    };
    if word.group().expand {
        return dynamic;
    }
    let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word) else {
        return dynamic;
    };
    let Ok(value) = std::str::from_utf8(&value) else {
        return dynamic;
    };
    if value.contains('\0') {
        return dynamic;
    }
    OriginalCommandNameOverlap {
        may_equal: tcl_syntax::naming::qualify("::", value) == coordinate
            || (!tcl_syntax::naming::is_qualified(value.as_bytes())
                && tcl_syntax::naming::key_tail(coordinate) == value),
        dynamic: false,
        empty: value.is_empty(),
    }
}

pub(crate) fn original_command_name_overlap(
    word: &tcl_lexer::NativeWord,
    coordinate: &str,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<OriginalCommandNameOverlap> {
    let config = authored_c_pattern_config(metadata)?;
    if word.config() != config || word.group().expand {
        return None;
    }
    if let Some(value) = tcl_syntax::word_rules::original_static_word_unicode_value(word) {
        let value = std::str::from_utf8(&value).ok()?;
        if value.contains('\0') {
            return None;
        }
        return Some(OriginalCommandNameOverlap {
            may_equal: tcl_syntax::naming::qualify("::", value) == coordinate
                || (!tcl_syntax::naming::is_qualified(value.as_bytes())
                    && tcl_syntax::naming::key_tail(coordinate) == value),
            dynamic: false,
            empty: value.is_empty(),
        });
    }
    let arena = word.executable_parts();
    let mut rest = coordinate;
    for part in arena.list(arena.root()) {
        match part.part {
            tcl_lexer::ExecutablePart::Text(_) => {
                let text = std::str::from_utf8(arena.bytes(part.span)?).ok()?;
                if text.contains(['\\', '\0']) {
                    return Some(OriginalCommandNameOverlap {
                        may_equal: true,
                        dynamic: true,
                        empty: false,
                    });
                }
                let literal = tcl_syntax::naming::canonical_written_command(
                    tcl_syntax::naming::without_surrounding_colons(text),
                );
                let Some(offset) = rest.find(&literal) else {
                    return Some(OriginalCommandNameOverlap {
                        may_equal: false,
                        dynamic: true,
                        empty: false,
                    });
                };
                rest = &rest[offset + literal.len()..];
            }
            tcl_lexer::ExecutablePart::Variable { index: Some(_), .. }
            | tcl_lexer::ExecutablePart::ParseError(_) => {
                return Some(OriginalCommandNameOverlap {
                    may_equal: true,
                    dynamic: true,
                    empty: false,
                });
            }
            tcl_lexer::ExecutablePart::Variable { .. }
            | tcl_lexer::ExecutablePart::Command { .. }
            | tcl_lexer::ExecutablePart::Expression { .. } => {}
        }
    }
    Some(OriginalCommandNameOverlap {
        may_equal: true,
        dynamic: true,
        empty: false,
    })
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
fn name_word_pattern(word: &str, config: LexerConfig) -> Vec<NamePiece> {
    use tcl_lexer::{ExecutablePart, ExecutablePartArena, SourceImage, Span};
    let Some(length) = u32::try_from(word.len()).ok() else {
        return vec![NamePiece::Wildcard];
    };
    let Ok(arena) = ExecutablePartArena::decompose(
        SourceImage::document(word),
        Span::new(0, length),
        tcl_lexer::word_parts::SubstFlags::default(),
        config,
    ) else {
        return vec![NamePiece::Wildcard];
    };
    let mut pieces = Vec::new();
    for component in arena.list(arena.root()) {
        match component.part {
            ExecutablePart::Text(_) => {
                let Some(text) = arena
                    .bytes(component.span)
                    .and_then(|text| std::str::from_utf8(text).ok())
                else {
                    return vec![NamePiece::Wildcard];
                };
                if text.contains(['\\', '\0']) {
                    return vec![NamePiece::Wildcard];
                }
                if !text.is_empty() {
                    pieces.push(NamePiece::Literal(text.to_owned()));
                }
            }
            ExecutablePart::Variable { .. }
            | ExecutablePart::Command { .. }
            | ExecutablePart::Expression { .. } => {
                pieces.push(NamePiece::Wildcard);
            }
            ExecutablePart::ParseError(_) => return vec![NamePiece::Wildcard],
        }
    }
    if pieces
        .iter()
        .any(|piece| matches!(piece, NamePiece::Wildcard))
    {
        pieces
            .into_iter()
            .filter_map(|piece| match piece {
                NamePiece::Literal(text) => {
                    let literal = tcl_syntax::naming::canonical_written_command(
                        tcl_syntax::naming::without_surrounding_colons(&text),
                    );
                    (!literal.is_empty()).then_some(NamePiece::Literal(literal))
                }
                NamePiece::Wildcard => Some(NamePiece::Wildcard),
            })
            .collect()
    } else {
        let literal: String = pieces
            .into_iter()
            .filter_map(|piece| match piece {
                NamePiece::Literal(text) => Some(text),
                NamePiece::Wildcard => None,
            })
            .collect();
        vec![NamePiece::Literal(
            tcl_syntax::naming::canonical_written_command(&literal),
        )]
    }
}

/// Every way `qualified_cell` can be written at a use site: the rooted name
/// and each of its `::`-segment suffixes.
///
/// `::ns::v` yields `::ns::v`, `ns::v`, `v` — the absolute spelling plus the
/// relative ones a site inside an enclosing namespace would use.  Over-wide
/// on purpose: a spelling that could not actually resolve to this cell from
/// some site only ever adds a refusal, never removes one.
fn cell_spellings(qualified_cell: &str) -> impl Iterator<Item = String> {
    let mut suffixes = tcl_syntax::naming::key_suffixes(qualified_cell);
    if tcl_syntax::naming::key_tail(qualified_cell).is_empty()
        && !suffixes.iter().any(String::is_empty)
    {
        suffixes.push(String::new());
    }
    suffixes.into_iter()
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
    dynamic_name_barrier_in(cfg, registry, config, |_| true)
}

/// [`dynamic_name_barrier`] over the blocks `executable` admits: a statement
/// in a block that never runs writes, reads and destroys nothing.
#[must_use]
pub fn dynamic_name_barrier_in(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    config: LexerConfig,
    executable: impl Fn(&crate::cfg::BlockId) -> bool,
) -> DynamicNameBarrier {
    let mut barrier = cfg.caller_frame_barrier_in(&executable);
    for (id, block) in &cfg.blocks {
        if !executable(id) {
            continue;
        }
        for (index, stmt) in block.statements.iter().enumerate() {
            barrier = barrier.union(statement_barrier_for_cfg(
                stmt,
                (cfg, *id, index),
                registry,
                config,
            ));
        }
        if let Some(terminator) = &block.terminator {
            barrier = barrier.union(terminator_barrier_for_cfg(
                terminator, cfg, registry, config,
            ));
        }
    }
    barrier
}

/// Computed-name facts for an explicitly standalone statement scan.
/// Source-producing consumers use [`statement_barrier_for_cfg`].
#[must_use]
pub fn statement_barrier(
    stmt: &Statement,
    registry: &CommandRegistry,
    declared: &DeclaredFrameEffects,
    config: LexerConfig,
) -> DynamicNameBarrier {
    statement_barrier_with_commands(stmt, None, Commands::standalone(registry, declared), config)
}

/// Computed-name facts under the CFG's retained metadata owner, for the
/// statement at `index` of `block`. Missing or foreign supplied metadata
/// cannot reopen catalogue compatibility.
#[must_use]
pub fn statement_barrier_for_cfg(
    stmt: &Statement,
    (cfg, block, index): (&CfgFunction, crate::cfg::BlockId, usize),
    registry: &CommandRegistry,
    config: LexerConfig,
) -> DynamicNameBarrier {
    statement_barrier_with_commands(
        stmt,
        cfg.source_tokens_at(block, index),
        Commands::for_cfg(cfg, registry, config),
        config,
    )
}

/// `point` is the source command the CFG retains for `stmt`, which names the
/// nested commands of an expression the statement carries no tokens for
/// (`set n [expr {[string length $s]}]`, #2454).
fn statement_barrier_with_commands(
    stmt: &Statement,
    point: Option<&crate::ir::CommandTokens>,
    commands: Commands<'_>,
    config: LexerConfig,
) -> DynamicNameBarrier {
    let mut barrier = DynamicNameBarrier::default();
    scan_statement(stmt, point, commands, &mut barrier, config);
    for script in crate::ir_helpers::nested_bodies(stmt) {
        crate::ir::for_each_statement(script, &mut |inner| {
            scan_statement(inner, None, commands, &mut barrier, config);
        });
    }
    barrier
}

/// Computed-name facts for an explicitly standalone terminator scan.
#[must_use]
pub fn terminator_barrier(
    terminator: &Terminator,
    registry: &CommandRegistry,
    declared: &DeclaredFrameEffects,
    config: LexerConfig,
) -> DynamicNameBarrier {
    terminator_barrier_with_commands(terminator, Commands::standalone(registry, declared), config)
}

/// Per-site terminator facts under the same owner as the CFG's statements.
#[must_use]
pub fn terminator_barrier_for_cfg(
    terminator: &Terminator,
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> DynamicNameBarrier {
    let commands = Commands::for_cfg(cfg, registry, config);
    if !commands.standalone
        && let Terminator::Branch {
            condition,
            condition_base,
            ..
        } = terminator
    {
        let parent = cfg.blocks.iter().find_map(|(block_id, block)| {
            std::ptr::eq(block.terminator.as_ref()?, terminator)
                .then(|| cfg.source_tokens_at(*block_id, usize::MAX))
                .flatten()
        });
        let mut barrier = DynamicNameBarrier::default();
        scan_original_expression(
            condition,
            *condition_base,
            parent,
            commands,
            &mut barrier,
            config,
        );
        return barrier;
    }
    terminator_barrier_with_commands(terminator, commands, config)
}

fn terminator_barrier_with_commands(
    terminator: &Terminator,
    commands: Commands<'_>,
    config: LexerConfig,
) -> DynamicNameBarrier {
    let mut barrier = DynamicNameBarrier::default();
    match terminator {
        Terminator::Branch { condition, .. } => {
            // Detached expression text has no supplied original child lookup.
            if commands.standalone {
                scan_expr(condition, commands, &mut barrier, config);
            } else {
                let mut children = Vec::new();
                crate::ir_helpers::collect_expr_commands(condition, &mut children);
                if !children.is_empty() {
                    barrier = DynamicNameBarrier::OPAQUE_SCRIPT;
                }
            }
        }
        Terminator::Return {
            tokens: Some(tokens),
            ..
        } => {
            scan_original_substitutions(tokens, commands, &mut barrier, config);
        }
        Terminator::Return { value, expr, .. } => {
            if let Some(v) = value {
                scan_text(v, commands, &mut barrier, 0, config);
            }
            if let Some(e) = expr {
                scan_expr(e, commands, &mut barrier, config);
            }
        }
        Terminator::Goto { .. } | Terminator::Complete { .. } => {}
    }
    barrier
}

#[derive(Clone, Copy)]
struct Commands<'a> {
    registry: &'a CommandRegistry,
    declared: &'a DeclaredFrameEffects,
    // None is terminal supplied refusal; Standalone explicitly selects the
    // profile-less standalone mode.
    metadata: Option<crate::registry_invocation::InvocationMetadataSelection<'a>>,
    standalone: bool,
}

impl<'a> Commands<'a> {
    fn standalone(registry: &'a CommandRegistry, declared: &'a DeclaredFrameEffects) -> Self {
        Self {
            registry,
            declared,
            metadata: Some(crate::registry_invocation::InvocationMetadataSelection::Standalone),
            standalone: true,
        }
    }

    fn for_cfg(cfg: &'a CfgFunction, registry: &'a CommandRegistry, config: LexerConfig) -> Self {
        let owner = &cfg.metadata_context;
        let metadata = if owner
            .source_analysis_input()
            .is_some_and(|input| input.lexer_config().normalized() != config.normalized())
        {
            None
        } else {
            owner.metadata_context(registry)
        };
        Self {
            registry,
            declared: &cfg.declared_frame_effects,
            metadata,
            standalone: owner.is_standalone(),
        }
    }

    fn declared_frame_effect(self, command: &str) -> Option<FrameEffectSpec> {
        crate::declared_frame_effects::declared_frame_effect_for_metadata(self.declared, command)
    }

    /// Whether a document declaration states that `command` runs in its own
    /// frame or none, so it reaches no name of the caller's.
    fn declares_no_caller_frame(self, command: &str) -> bool {
        matches!(
            self.declared
                .get(&tcl_registry::rooted_command_descriptor_label(command)),
            Some(None)
        )
    }
}

fn scan_call_with_tokens(
    command: &str,
    args: &[String],
    tokens: &crate::ir::CommandTokens,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) {
    if !commands.standalone && tokens.carries_proved_names() {
        // A definition point whose producer proved every name it writes
        // (`CommandTokens::carries_proved_names`): iteration bindings,
        // `[…]` word effects, condition kill-sets, upvar invalidation and
        // arm writes. It runs no command of its own; the commands that
        // produce its effects are scanned at their own statements.
        return;
    }
    if !commands.standalone
        && tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments)
    {
        // A structured command's argument evaluation: its own head and
        // bodies are separate statements and phases, so what it can do
        // to names is what its written words' substitutions do, read
        // under the argument-evaluation word owner.
        if tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_argument_lexer_config(tokens))
            .is_none_or(|original| original.normalized() != config.normalized())
        {
            *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
            return;
        }
        scan_original_substitutions(tokens, commands, barrier, config);
        return;
    }
    if !commands.standalone
        && tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_lexer_config_for_tokens(tokens))
            .is_none_or(|original| original.normalized() != config.normalized())
    {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    }
    let names = crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
        commands.registry,
        metadata,
        tokens,
    );
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        commands.registry,
        metadata,
        tokens,
    );
    if let Some(names) = &names {
        scan_possible_variable_names(names, barrier);
    }
    if let Some(invocation) = &invocation {
        scan_retained_name_effects(invocation, barrier);
    }
    if commands.standalone {
        scan_standalone_template(command, args, tokens, commands, barrier, config);
    } else {
        if invocation.as_ref().is_some_and(|invocation| {
            invocation
                .facts
                .traits
                .contains(Traits::PERFORMS_SUBSTITUTION)
        }) {
            scan_materialized_names(tokens, commands, barrier);
        }
        // A closed call to user procedures names nothing computed in this
        // frame by itself: what the procedures do to the caller's frame is
        // their summaries', which the CFG records as the caller-frame
        // barrier (opaque where a summary is missing).
        if invocation.is_none()
            && names.as_ref().is_none_or(
                crate::registry_invocation::PossibleVariableNameOperands::unknown_residual,
            )
            && !tokens.source_binding.as_ref().is_some_and(
                crate::command_binding::SourceInvocationBinding::targets_only_source_procedures,
            )
            && !commands.declares_no_caller_frame(command)
        {
            *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        }
    }
    scan_original_substitutions(tokens, commands, barrier, config);
}

fn scan_standalone_template(
    command: &str,
    args: &[String],
    tokens: &crate::ir::CommandTokens,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    // Template plans are source advice; only explicit standalone scans reparse them.
    let braced: Vec<bool> = (0..args.len())
        .map(|index| tokens.arg_is_braced_literal(index))
        .collect();
    let head_dynamic = tokens
        .argv_kinds
        .first()
        .is_some_and(|kind| matches!(kind, tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd));
    if !head_dynamic {
        scan_command(
            command,
            &CommandWords {
                args,
                braced: Some(&braced),
            },
            commands,
            barrier,
            (0, config),
        );
    }
}

fn scan_call(
    command: &str,
    args: &[String],
    tokens: Option<&crate::ir::CommandTokens>,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    let Some(metadata) = commands.metadata else {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    };
    let metadata = metadata.context();
    if let Some(tokens) = tokens {
        scan_call_with_tokens(command, args, tokens, commands, barrier, config, metadata);
    } else if commands.standalone {
        scan_command(
            command,
            &CommandWords { args, braced: None },
            commands,
            barrier,
            (0, config),
        );
        for arg in args {
            scan_text(arg, commands, barrier, 0, config);
        }
    } else {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
    }
}

fn scan_materialized_names(
    tokens: &crate::ir::CommandTokens,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
) {
    let footprint = (|| {
        let binding = tokens.source_binding.as_ref()?;
        let source = binding
            .invocation_site()?
            .source
            .source_image()
            .try_text()
            .ok()?;
        let footprint = binding.original_materialized_footprint(
            tokens,
            source,
            commands.registry,
            commands
                .metadata
                .and_then(crate::registry_invocation::InvocationMetadataSelection::context),
        )?;
        Some(footprint.invocation_footprint())
    })();
    if footprint.is_none_or(|footprint| footprint.opaque) {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
    }
}

/// Original child dispatch receipts retain their own lookup horizon.
fn scan_original_substitutions(
    tokens: &crate::ir::CommandTokens,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    let Some(metadata) = commands.metadata else {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    };
    let metadata = metadata.context();
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
            crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
                commands.registry,
                metadata,
                tokens,
            );
        let invocation =
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                commands.registry,
                metadata,
                tokens,
            );
        if let Some(names) = &names {
            scan_possible_variable_names(names, barrier);
        }
        if let Some(invocation) = &invocation {
            scan_retained_name_effects(invocation, barrier);
            if !commands.standalone
                && invocation
                    .facts
                    .traits
                    .contains(Traits::PERFORMS_SUBSTITUTION)
            {
                scan_materialized_names(tokens, commands, barrier);
            }
        }
        // A closed call to user procedures names nothing computed in this
        // frame by itself: what the procedures do to the caller's frame is
        // their summaries', which the CFG records as the caller-frame
        // barrier (opaque where a summary is missing).
        if invocation.is_none()
            && names.as_ref().is_none_or(
                crate::registry_invocation::PossibleVariableNameOperands::unknown_residual,
            )
            && !tokens.source_binding.as_ref().is_some_and(
                crate::command_binding::SourceInvocationBinding::targets_only_source_procedures,
            )
            && !tokens
                .argv_texts
                .first()
                .is_some_and(|head| commands.declares_no_caller_frame(head))
        {
            *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        }
    }
}

fn scan_statement(
    stmt: &Statement,
    point: Option<&crate::ir::CommandTokens>,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    match stmt {
        Statement::NativeCall { .. } => *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT),
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
            scan_call(command, args, tokens.as_ref(), commands, barrier, config);
        }
        Statement::AssignConst { .. } => {
            if let Some(tokens) = stmt.tokens() {
                scan_original_substitutions(tokens, commands, barrier, config);
            }
        }
        Statement::AssignValue { value, .. } => {
            if let Some(tokens) = stmt.tokens() {
                scan_original_substitutions(tokens, commands, barrier, config);
            } else {
                scan_text(value, commands, barrier, 0, config);
            }
        }
        Statement::AssignExpr {
            expr, expr_base, ..
        }
        | Statement::ExprEval {
            expr, expr_base, ..
        } => {
            if let Some(tokens) = stmt.tokens() {
                scan_original_substitutions(tokens, commands, barrier, config);
            }
            if commands.standalone {
                scan_expr(expr, commands, barrier, config);
            } else {
                scan_original_expression(
                    expr,
                    *expr_base,
                    stmt.tokens().or(point),
                    commands,
                    barrier,
                    config,
                );
            }
        }
        Statement::Return {
            value,
            expr,
            expr_base,
            ..
        } => {
            if let Some(tokens) = stmt.tokens() {
                scan_original_substitutions(tokens, commands, barrier, config);
                if let Some(expr) = expr {
                    scan_original_expression(
                        expr,
                        *expr_base,
                        Some(tokens),
                        commands,
                        barrier,
                        config,
                    );
                }
            } else {
                if let Some(value) = value {
                    scan_text(value, commands, barrier, 0, config);
                }
                if let Some(expr) = expr {
                    scan_expr(expr, commands, barrier, config);
                }
            }
        }
        Statement::Switch { subject, .. } => {
            if let Some(tokens) = stmt.tokens() {
                scan_original_substitutions(tokens, commands, barrier, config);
            } else {
                scan_text(subject, commands, barrier, 0, config);
            }
        }
        _ => {}
    }
}

/// Only each unchanged expression child's own lookup can supply its name roles.
/// The original expression base joins lexical extents; it grants no entered
/// frame or evaluation result. Missing source geometry retains an opaque tail.
fn scan_original_expression(
    expression: &ExprNode,
    base: Option<u32>,
    parent: Option<&crate::ir::CommandTokens>,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    let mut pending = vec![(expression, 0)];
    while let Some((node, depth)) = pending.pop() {
        if crate::depth_guard::MAX_EXPR_NODE_DEPTH.exceeded(depth) {
            *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
            continue;
        }
        match node {
            ExprNode::Command { text, start, end } => {
                let nested = (|| {
                    let parent = parent?;
                    let base = base?;
                    let site = crate::ir::SourceSite::source(tcl_lexer::Span::new(
                        base.checked_add(*start)?,
                        base.checked_add(*end)?,
                    ));
                    let mut nested =
                        crate::word_subst::nested_command_words(text, &site, config).ok()?;
                    nested.inherit_nested_bindings(parent);
                    Some(nested)
                })();
                if let Some(tokens) = nested {
                    if let Some((head, args)) = tokens.argv_texts.split_first() {
                        scan_call(head, args, Some(&tokens), commands, barrier, config);
                    } else {
                        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                    }
                } else {
                    *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                }
            }
            ExprNode::Unary { operand, .. } => pending.push((operand, depth + 1)),
            ExprNode::Binary { left, right, .. } => {
                pending.push((right, depth + 1));
                pending.push((left, depth + 1));
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.push((false_branch, depth + 1));
                pending.push((true_branch, depth + 1));
                pending.push((condition, depth + 1));
            }
            ExprNode::Call { args, .. } => pending.extend(args.iter().map(|arg| (arg, depth + 1))),
            ExprNode::Raw { text } | ExprNode::Var { text, .. } => {
                if !crate::var_refs::command_subst_texts_with_config(text, config).is_empty() {
                    *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                }
            }
            ExprNode::String { text, .. } => {
                if !text.starts_with('{')
                    && !crate::var_refs::command_subst_texts_with_config(text, config).is_empty()
                {
                    *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                }
            }
            ExprNode::CompiledWord { text, braced } => {
                if !*braced
                    && !crate::var_refs::command_subst_texts_with_config(text, config).is_empty()
                {
                    *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
                }
            }
            ExprNode::Literal { .. } => {}
        }
    }
}

fn scan_expr(
    expr: &ExprNode,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    config: LexerConfig,
) {
    if !commands.standalone {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    }

    let mut cmds = Vec::new();
    crate::ir_helpers::collect_expr_commands(expr, &mut cmds);
    for cmd_text in &cmds {
        let trimmed = cmd_text.trim();
        let inner = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(trimmed);
        scan_script_text(inner, commands, barrier, 0, config);
    }
}

/// Scan a word's raw text for nested `[…]` command substitutions.
fn scan_text(
    text: &str,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    depth: u32,
    config: LexerConfig,
) {
    if !commands.standalone {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    }

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
        scan_script_text(&inner, commands, barrier, depth + 1, config);
    }
}

/// Scan a run of script text (a `[…]` substitution's own content) for
/// dynamic-name accesses, then descend into whatever `[…]` it nests.
fn scan_script_text(
    text: &str,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    depth: u32,
    config: LexerConfig,
) {
    if !commands.standalone {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    }

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
        let words = CommandWords {
            args: &arg_texts,
            braced: Some(&braced),
        };
        scan_command(&command.text, &words, commands, barrier, (depth, config));
    }
    // The script's own words may nest further substitutions; `text` still has
    // their brackets intact (a word's raw spelling does not — a delimited
    // token's closer sits one past its span), so recurse from here.
    scan_text(text, commands, barrier, depth, config);
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
            let braced = arg_braced
                .and_then(|braced| braced.get(index))
                .copied()
                .unwrap_or(false);
            if crate::value_transfer::SourceWord::of(Some(word), braced)
                == crate::value_transfer::SourceWord::Substituted
            {
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
        | Word::KnownByteExpansion(_)
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
    // Template name uncertainty belongs to its selected TemplateWordPlan;
    // disabled variable/command substitution cannot donate a read barrier.
}

/// Apply the registry's name-role answers for one `command args…` call.
///
/// `arg_braced`, when present, says for each argument whether it is a single
/// brace-quoted word — a distinction the segmenter's reconstructed `args`
/// text has already erased, and the one thing that separates a literal name
/// or template (`set {$n} 1`, `subst {$a}`) from a substituted one
/// (`set $n 1`, `subst $a`).
/// One call's argument words, as [`scan_command`] reads them.
#[derive(Clone, Copy)]
struct CommandWords<'a> {
    /// The argument texts.
    args: &'a [String],
    /// When present, whether each argument is a single brace-quoted word —
    /// a distinction the segmenter's reconstructed `args` text has already
    /// erased, and the one thing that separates a literal name or template
    /// (`set {$n} 1`, `subst {$a}`) from a substituted one (`set $n 1`,
    /// `subst $a`).
    braced: Option<&'a [bool]>,
}

/// Apply the registry's name-role answers for one `command args…` call,
/// `at` the bracket depth and lexer configuration its script is read under.
fn scan_command(
    command: &str,
    words: &CommandWords<'_>,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    at: (u32, LexerConfig),
) {
    if !commands.standalone {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    }
    let CommandWords {
        args,
        braced: arg_braced,
    } = *words;
    let registry = commands.registry;
    let spec = registry.get_for_surface(command, registry.own_surface_query());
    let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    let source_words = source_argument_words(&arg_strs, arg_braced);
    let arguments = tcl_registry::InvocationArguments::structured(&source_words);
    if let Some(frame) = spec.map_or_else(
        || commands.declared_frame_effect(command),
        |spec| spec.frame_effect,
    ) {
        scan_frame_effect(frame, &arg_strs, arg_braced, barrier, registry, None);
    }
    let Some(spec) = spec else {
        return;
    };
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

    let Some(writes) =
        commands
            .registry
            .arg_indices_for_role_words(command, arguments, ArgRole::VarWrite)
    else {
        *barrier = barrier.union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    };
    let Some(reads) =
        commands
            .registry
            .arg_indices_for_role_words(command, arguments, ArgRole::VarRead)
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
    let introspecting: Vec<&str> = commands
        .registry
        .subcommands_with_trait(command, Traits::INTROSPECTS_BY_NAME);
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
    if spec.traits.contains(Traits::PERFORMS_SUBSTITUTION) {
        scan_template(command, &arg_strs, arg_braced, commands, barrier, at);
    }
}

/// A template-expanding command (`subst`) performs substitution over its
/// template word, as the call's template-word plan says. A literal template
/// names its reads in its text, which the ordinary scanners see, and each
/// `[…]` region it runs is script in this frame, scanned here. A computed
/// one reads names that come from run-time data whenever variable or
/// command substitution runs over it — `[subst $[subst $locVar]]` is exactly
/// this shape, and `subst -novariables $t` still runs `[set x]` — so every
/// local is reachable; `subst -nocommands -novariables $t` reads none. A
/// call with no plan to read keeps the conservative answer: any substituted
/// word past the switches reads.
fn scan_template(
    command: &str,
    args: &[&str],
    arg_braced: Option<&[bool]>,
    commands: Commands<'_>,
    barrier: &mut DynamicNameBarrier,
    (depth, config): (u32, LexerConfig),
) {
    use crate::value_transfer::SourceWord;
    let source = |index: usize| {
        let braced = arg_braced
            .and_then(|b| b.get(index))
            .copied()
            .unwrap_or(false);
        SourceWord::of(args.get(index).copied(), braced)
    };
    match crate::value_transfer::literal_template_plan(commands.registry, command, args, source) {
        Some(plan) => {
            if plan.dynamic && (plan.kinds.variables || plan.kinds.commands) {
                barrier.reads = true;
            }
            for region in &plan.script_regions {
                scan_script_text(&region.script.script, commands, barrier, depth + 1, config);
            }
        }
        None => {
            if args.iter().enumerate().any(|(index, word)| {
                !word.starts_with('-') && source(index) == SourceWord::Substituted
            }) {
                barrier.reads = true;
            }
        }
    }
}

#[cfg(test)]
mod braced_var_close_rule_tests;

#[cfg(test)]
mod tests;
