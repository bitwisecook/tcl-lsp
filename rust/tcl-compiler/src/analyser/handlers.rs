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

//! Per-command handlers for the variable-mutation commands.
//!
//! The variable-write trio:
//!
//! - [`Analyser::handle_set_command`] — `set var ?value?`
//! - [`Analyser::handle_variable_command`] /
//!   [`Analyser::handle_global_command`] —
//!   `variable name ?value? ...?` and `global name...`
//! - [`Analyser::handle_incr_command`] — `incr var ?amount?`
//!
//! Plus the remaining structural command handlers: proc-body
//! walking, `namespace eval`/`namespace ensemble`, `foreach`/`for`/
//! `switch`, `catch`/`try`, `interp alias`, `oo::objdefine`, and
//! alias resolution.

use tcl_core_types::DiagCode;
use tcl_lexer::{SourceMap, Span, Token, TokenType};
use tcl_syntax::list::find_element;

use crate::alias::{detect_interp_alias, resolve_alias};
use crate::parsing::syntax::descend::descend_token;
use crate::parsing::syntax::segment::segments_from_tree;
use crate::segmenter::SegmentedCommand;
use crate::signature_scan::params::bind_proc_formals;
use crate::signature_scan::types::ParamDef;
use crate::signature_scan::types::SignatureCommandAlias;

use super::state::Analyser;
use super::types::{ClassDef, ClassFactory, DefinedSymbol, FactoryMember, FactoryWord, ProcDef};
use super::utils::{param_name_spans_for_token, parse_param_list};

/// The three per-proc facts [`Analyser::infer_proc_param_traits`] derives from
/// one view of a proc body: the per-parameter trait map, the caller-frame
/// parameter names ([`super::types::ProcDef::caller_frame_params`]), and the
/// literal caller-frame targets
/// ([`super::types::ProcDef::caller_frame_literals`], name → written-through-alias).
type ProcParamFacts = (
    std::collections::HashMap<String, std::collections::HashSet<super::types::ProcArgTrait>>,
    std::collections::HashSet<String>,
    std::collections::HashMap<String, bool>,
);

/// Tcl *library* procedures (defined in init.tcl / auto.tcl / history.tcl /
/// package.tcl / word.tcl) that are script-defined and documented as
/// user-replaceable — redefining one is the supported overlay idiom, not
/// shadowing a C built-in.  Genuine C commands that are not byte-compiled
/// but still dangerous to redefine (`clock`, `after`, `socket`, `glob`) are
/// deliberately excluded — they keep firing W113.
/// Memoised set of the redefinable Tcl library procs (`unknown`,
/// `auto_*`, `pkg_*`, `tclLog`, `tcl_findLibrary`, the word-boundary
/// helpers, …), sourced from the registry's
/// [`Traits::OVERRIDABLE_LIBRARY_PROC`] trait. Redefining one of these
/// must not fire W113. Cached once; the set is dialect-agnostic core Tcl.
fn overridable_library_procs() -> &'static std::collections::HashSet<String> {
    use std::sync::OnceLock;
    static SET: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        tcl_registry::CommandRegistry::build_default()
            .commands_with_trait(tcl_registry::Traits::OVERRIDABLE_LIBRARY_PROC)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Build a fully-qualified Tcl proc / class name from a namespace
/// prefix and a possibly-relative name.
///
/// `ns_prefix` is a **constructed** namespace key, rooted (`::a::b`, `::`)
/// or unrooted (`a::b`, empty = global); it is joined verbatim — only the
/// *written* `name`'s colon runs canonicalise (#934).
pub(super) fn qualify(ns_prefix: &str, name: &str) -> String {
    crate::naming::qualify(ns_prefix, name)
}

/// The [`super::types::Scope`] name for a routine whose written name is
/// `resolved_name` (after [`Analyser::resolve_dynamic_word`]).
///
/// A `Proc` / `Method` scope's name is not decoration: it *is* the routine's
/// qualified name for
/// [`super::scope::advance_command_resolution_namespace`], which takes
/// everything before the last `::` as the namespace the body runs in.  So a
/// scope named with the raw, unresolved word invents a namespace out of the
/// substitution: the real tcllib idiom `proc [namespace current]::_x {...}`
/// inside `::pki` (tcllib 2.0 `modules/pki/pki.tcl`:316) made every definition
/// in the body home to `::pki::[namespace current]`.
///
/// When the name still carries a `$` / `[` after resolution there is no
/// trustworthy qualifier in it, so the scope keeps only the trailing segment —
/// whose holder is the *lexical* parent namespace, which is where the idiom
/// actually lands (tclsh 8.6.16 / 9.0.4: a `proc helper` inside
/// `proc [namespace current]::_inner` in `::pki` becomes `::pki::helper`).
pub(super) fn scope_name_for_routine(resolved_name: &str) -> &str {
    if crate::naming::is_dynamic_word(resolved_name) {
        crate::naming::key_tail(resolved_name)
    } else {
        resolved_name
    }
}

/// Normalise a literal `interp` path word to the interpreter-domain map
/// key: the path is a Tcl *list* naming a descent through child
/// interpreters (`{s t}` = child `t` of child `s`), so whitespace runs
/// collapse to one separator.  A single-element path (`s`) keys as
/// itself.
pub(super) fn interp_path_key(path: &str) -> String {
    path.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse `interp create`'s `?-safe? ?--? ?path?` words (issue #923 idx 9):
/// returns `(safe, path)`, where `path` is the sole non-flag word (or
/// `None` for a bare, uncaptured `interp create -safe`). Shared by
/// `Analyser::handle_interp_create_command` and the `set VAR [interp
/// create ...]` value-flow detector in `Analyser::handle_set_command`, so
/// both parse the flag/path shape identically.
///
/// Mirrors C Tcl's own loop (`Tcl_InterpObjCmd`, the `create` arm): every
/// word is examined, and a `-`-prefixed word is a *flag* until `--` is
/// seen, whether or not the path has already been read. The scan does not
/// stop at the path.
///
/// Oracle (tclsh8.6 and tclsh9.0):
///
/// * `interp create x -safe` — path `x`, and it **is** safe.
/// * `interp create -safe -- z` — path `z`, safe.
/// * `interp create -- -safe` — path is the literal `-safe`, not safe.
/// * `interp create n -bogus` — `bad option "-bogus"`, so flags really are
///   still being parsed after the path.
/// * `interp create a b` — `wrong # args`; a second path word is an error
///   shape, not a rebinding, so no path is reported rather than the wrong
///   one.
fn parse_interp_create_words<'a>(words: &[&'a str]) -> (bool, Option<&'a str>) {
    let mut safe = false;
    let mut path: Option<&str> = None;
    let mut past_flags = false;
    let mut too_many_paths = false;
    for &word in words {
        if !past_flags && word.starts_with('-') {
            if word == "--" {
                past_flags = true;
            } else if word == "-safe" {
                safe = true;
            }
            // Any other `-` word is a bad option in real Tcl. Skipping it
            // keeps the path reading unaffected, which is the conservative
            // choice for a command that will fail anyway.
            continue;
        }
        if path.is_none() {
            path = Some(word);
        } else {
            too_many_paths = true;
        }
    }
    if too_many_paths {
        return (safe, None);
    }
    (safe, path)
}

/// `set VAR [interp create ?-safe? ?--? ?path?]`'s value word, stripped
/// to just the words after `interp`/`create`, when `text` is exactly
/// that literal `[...]` substitution shape and nothing else — feeds
/// `Analyser::handle_set_command`'s value-flow detector through
/// [`parse_interp_create_words`] (issue #923 idx 9).
fn interp_create_words_from_value(text: &str) -> Option<Vec<&str>> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    // Tcl-list parse, not `split_whitespace` (issue #1025): the direct
    // `interp create` handler sees segmenter-decoded words, so a braced
    // path word (`{child}`, `{parent child}`) reaches it already stripped.
    // Whitespace-splitting the raw substitution text instead keeps the
    // braces and can even split one word in two (`{parent child}` →
    // `"{parent"`, `"child}"`), binding the variable to a key the direct
    // handler never records — later `$i eval` / `interp alias $i …` then
    // resolve against a phantom interpreter.
    // A parse error means `inner` is not a well-formed Tcl list at all
    // (`interp create {child]`), so there is no interpreter here to record.
    // Keeping the words parsed so far would bind the variable to a path
    // real Tcl never creates — a phantom interpreter that later `$i eval` /
    // `interp alias $i …` sites then resolve against, which is exactly what
    // an incomplete edit looks like mid-keystroke.
    let mut words = Vec::new();
    let mut pos = 0usize;
    loop {
        match find_element(inner, pos) {
            Ok(Some(el)) => {
                words.push(inner.get(el.value.clone())?);
                pos = el.next;
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    let mut words = words.into_iter();
    if words.next()? != "interp" || words.next()? != "create" {
        return None;
    }
    Some(words.collect())
}

/// Condense a definition's description argument into a single-line outline
/// detail: the first non-empty line, trimmed, and length-capped so a verbose
/// multi-line description doesn't bloat the outline entry.
fn summarise_detail(description: &str) -> String {
    const MAX: usize = 80;
    let line = description
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .trim();
    if line.chars().count() > MAX {
        let truncated: String = line.chars().take(MAX - 1).collect();
        format!("{truncated}…")
    } else {
        line.to_string()
    }
}

/// A recorded class that is itself a `TclOO` class factory, with the
/// definition-body grammar the bodies it manufactures obey.
///
/// See [`Analyser::user_metaclass_of_command`].
struct UserMetaclass {
    /// The registry metaclass command at the root of the chain
    /// (`oo::class`), whose definition-body grammar governs the bodies this
    /// factory makes.
    root_command: String,
    /// The root registry metaclass's definition-body grammar.
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
}

/// One definition-body member a class factory injects into every class it
/// manufactures, with each word resolved to a real source token.
struct InjectedMember {
    /// Member keyword followed by its argument words.
    texts: Vec<String>,
    /// Parallel tokens — each in the manufacturer's own body (a literal it
    /// always splices) or in the creation call (a `{*}$param` splice).
    argv: Vec<Token>,
}

/// Where a class-manufacturing call's words sit, and what its manufacturer
/// contributes on top of the written body.
///
/// See [`Analyser::manufacturer_layout`].
struct ManufacturerLayout {
    /// Argument index of the new class's name.
    name_arg: usize,
    /// Argument index of the new class's definition body.
    body_arg: usize,
    /// Members the manufacturer always injects.
    injected: Vec<InjectedMember>,
    /// `true` when the manufacturer override could not be read, so the
    /// class's superclass list is unknown — see
    /// [`super::types::ClassDef::inheritance_unknown`].
    inheritance_unknown: bool,
}

/// Registry/user-factory facts needed after a class-manufacturing call has
/// been classified.
struct ResolvedClassCreation {
    /// Published factory proof for a user-defined metaclass, absent for a
    /// registry definer.
    user_factory: Option<ClassFactory>,
    /// Name/body positions and injected definition members.
    layout: ManufacturerLayout,
}

impl ManufacturerLayout {
    /// A registry-declared manufacturer's own word layout, with nothing
    /// injected. Anonymous manufacturers (`new`) and ordinary-instance
    /// manufacturers (snit's type `create`) have no statically nameable
    /// class definition and therefore return `None`.
    fn builtin(method: &tcl_registry::definer::ManufacturerMethod) -> Option<Self> {
        Some(Self {
            name_arg: usize::from(method.names_instance_at?),
            body_arg: usize::from(method.definition_body_at?),
            injected: Vec::new(),
            inheritance_unknown: false,
        })
    }
}

/// How deep [`Analyser::collect_nested_statements`] descends into nested
/// script words before giving up.
///
/// A metaclass's unknown-dispatch member is a handful of lines in every real
/// corpus instance; the cap only bounds a pathological or generated body, and
/// stopping early can only *lose* evidence, which abstains.
const MAX_UNKNOWN_BODY_DEPTH: u32 = 8;

/// One creation call inside a proc body whose name word reads a parameter —
/// see [`Analyser::record_literal_parameter_definitions`].
struct ParameterisedCreation {
    /// Qualified name of the proc whose body holds the call.
    proc_qname: String,
    /// That proc's Tcl formals, including defaults and the trailing `args`
    /// collector.
    params: Vec<ParamDef>,
    /// The name word, as written (`${ns}::class`).
    name_word: String,
    /// Source offset of the creation call.  Literal-provenance evaluation
    /// executes only dominating assignments before this point.
    call_off: u32,
    /// Registry-described control bodies enclosing the creation, from the
    /// proc body inwards.  Each body must be selected by its controller for
    /// the creation to materialise.
    control_path: Vec<ControlArm>,
}

#[derive(Clone, Copy)]
struct ParameterisedProc<'a> {
    qname: &'a str,
    params: &'a [ParamDef],
}

#[derive(Clone)]
struct ControlArm {
    controller: SegmentedCommand,
    body_span: Span,
}

#[derive(Default)]
struct ControlArms {
    arms: Vec<ControlArm>,
    complete: bool,
}

struct LoadTimeCall {
    args: Vec<String>,
    control_path: Vec<ControlArm>,
    call_off: u32,
}

enum StaticScriptOutcome {
    FallsThrough(String),
    Returns(String),
}

/// `word` with each `$name` / `${name}` reading one of `params` replaced by
/// the call site's argument at that parameter's position.
///
/// `None` — abstain — when the word carries a command substitution or a
/// backslash escape (whose value this cannot compute), when an interpolation
/// names something other than a parameter, or when the call site supplied no
/// argument for the parameter it does name.
fn substitute_bound_words(word: &str, params: &[String], args: &[String]) -> Option<String> {
    if word.contains('[') || word.contains('\\') {
        return None;
    }
    let mut out = String::with_capacity(word.len());
    let mut rest = word;
    while let Some(dollar) = rest.find('$') {
        out.push_str(&rest[..dollar]);
        let after = &rest[dollar + 1..];
        let (name, tail) = if let Some(braced) = after.strip_prefix('{') {
            let close = braced.find('}')?;
            (&braced[..close], &braced[close + 1..])
        } else {
            let len = after
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
                .unwrap_or(after.len());
            if len == 0 {
                return None;
            }
            (&after[..len], &after[len..])
        };
        let idx = params.iter().position(|p| p == name)?;
        let value = args.get(idx)?;
        // A call site whose own argument is itself dynamic proves nothing.
        if tcl_syntax::naming::is_dynamic_word(value) {
            return None;
        }
        out.push_str(value);
        rest = tail;
    }
    out.push_str(rest);
    Some(out)
}

/// Join two observations of the same computed-name class creation.
///
/// The source walk can observe the load-time wrapper with no structural body,
/// while the proc-body walk observes the body under a dynamic placeholder.
/// Empty structural relations are therefore lower bounds at this boundary:
/// the non-empty compatible observation wins, and an already-published
/// factory never disappears behind a later empty observation. Two different
/// concrete superclass relations are not compatible evidence for one class;
/// the caller must publish an abstaining record instead.
fn join_parameterised_class_observations(
    existing: &ClassDef,
    observed: &ClassDef,
) -> Option<ClassDef> {
    if existing.qualified_name != observed.qualified_name
        || crate::naming::normalise_qualified_name(&existing.metaclass)
            != crate::naming::normalise_qualified_name(&observed.metaclass)
        || (!existing.superclasses.is_empty()
            && !observed.superclasses.is_empty()
            && existing.superclasses != observed.superclasses)
    {
        return None;
    }

    let (mut joined, other) = if !existing.superclasses.is_empty()
        && observed.superclasses.is_empty()
        || existing.superclasses == observed.superclasses
            && existing.factory.is_some()
            && observed.factory.is_none()
    {
        (existing.clone(), observed)
    } else {
        (observed.clone(), existing)
    };
    if joined.superclasses.is_empty() {
        joined.superclasses.clone_from(&other.superclasses);
    }
    if joined.factory.is_none() {
        joined.factory.clone_from(&other.factory);
    }
    if joined.class_command_fallback == super::types::ClassCommandFallback::None {
        joined.class_command_fallback = other.class_command_fallback;
    }
    joined.inheritance_unknown |= other.inheritance_unknown;
    joined.member_set_incomplete |= other.member_set_incomplete;
    Some(joined)
}

/// Whether a substituted command-name word could produce `candidate`.
///
/// The answer is deliberately one-sided: `false` is returned only when a
/// literal fragment of `word` cannot occur in `candidate`, in order. Every
/// variable/command/backslash substitution is an unconstrained wildcard.
/// This lets a command-table trust query distinguish
/// `${ns}::define::$method` from `::string` without guessing either
/// substitution, while `$name` remains compatible with every command.
fn dynamic_command_name_may_equal(word: &str, candidate: &str) -> bool {
    if !tcl_syntax::naming::is_dynamic_word(word) {
        let unqualified = !word.contains("::");
        let word = crate::naming::normalise_qualified_name(word);
        let candidate = crate::naming::normalise_qualified_name(candidate);
        return word == candidate
            || (unqualified
                && candidate
                    .rsplit_once("::")
                    .is_some_and(|(_, tail)| tail == word.trim_start_matches("::")));
    }

    let mut fragments = Vec::new();
    let mut literal = String::new();
    let mut chars = word.char_indices().peekable();
    'scan: while let Some((_, ch)) = chars.next() {
        match ch {
            '$' => {
                if !literal.is_empty() {
                    fragments.push(std::mem::take(&mut literal));
                }
                if chars.peek().is_some_and(|(_, next)| *next == '{') {
                    chars.next();
                    for (_, next) in chars.by_ref() {
                        if next == '}' {
                            break;
                        }
                    }
                } else {
                    while chars.peek().is_some_and(|(_, next)| {
                        next.is_alphanumeric() || matches!(*next, '_' | ':')
                    }) {
                        chars.next();
                    }
                    if chars.peek().is_some_and(|(_, next)| *next == '(') {
                        // Array indices have their own substitution grammar.
                        // The prefix fragments are still fixed; the remainder
                        // is conservatively unconstrained.
                        break 'scan;
                    }
                }
            }
            '[' => {
                if !literal.is_empty() {
                    fragments.push(std::mem::take(&mut literal));
                }
                // A quoted/escaped `]` makes bracket peeling a Tcl parser's
                // job. Treat the whole suffix as wildcard instead of risking
                // a false fixed fragment after the wrong closer.
                break;
            }
            '\\' => {
                if !literal.is_empty() {
                    fragments.push(std::mem::take(&mut literal));
                }
                // Backslash-newline also consumes following whitespace.
                // Discard the suffix rather than claim any of it is fixed.
                break;
            }
            _ => literal.push(ch),
        }
    }
    if !literal.is_empty() {
        fragments.push(literal);
    }

    let rooted = format!("::{}", candidate.trim_start_matches("::"));
    let mut tail = rooted.as_str();
    for fragment in fragments {
        let mut canonical = String::with_capacity(fragment.len());
        let mut chars = fragment.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch != ':' || chars.peek() != Some(&':') {
                canonical.push(ch);
                continue;
            }
            canonical.push_str("::");
            while chars.peek() == Some(&':') {
                chars.next();
            }
        }
        let fragment = canonical;
        let Some(pos) = tail.find(fragment.as_str()) else {
            return false;
        };
        tail = &tail[pos + fragment.len()..];
    }
    true
}

/// What one statement of an unknown-dispatch body proves — see
/// [`Analyser::unknown_dispatch_binds_instance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnknownBodyEvidence {
    /// It constructs an object named from the fallback's first parameter.
    Constructs,
    /// It completes the member with exactly that parameter.
    ReturnsWord,
    /// It completes the member with something else — fatal to the proof.
    ReturnsSomethingElse,
    /// It ends this path with a non-normal completion and returns no handle.
    Terminates,
    /// It says nothing either way.
    Nothing,
}

#[derive(Default)]
struct UnknownPathProof {
    proved_return: bool,
    invalid_return: bool,
    fallthrough_constructed: Vec<bool>,
}

/// Whether `head` is a bracketed self-receiver word — `[self]` / `[self
/// object]` written as a command head, which dispatches on the current object
/// exactly as `my` does.
///
/// Registry data via [`tcl_registry::CommandRegistry::is_self_receiver_call`];
/// the bracket peeling is this function's whole contribution.
fn bracketed_self_receiver(registry: &tcl_registry::CommandRegistry, head: &str) -> bool {
    let Some(inner) = head
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .map(str::trim)
    else {
        return false;
    };
    let mut words = inner.split_whitespace();
    let Some(cmd) = words.next() else {
        return false;
    };
    let arg = words.next();
    words.next().is_none() && registry.is_self_receiver_call(cmd, arg)
}

/// A literal list word's elements, each with the source span it occupies
/// inside the word — the static half of `{*}` expansion.
///
/// Returns `None` when the word is not a literal (a substitution's runtime
/// value is not a knowable list) or when an unbraced element itself
/// interpolates, so a caller can abstain instead of splicing text the
/// runtime would never produce.  An empty word yields an empty splice,
/// which is exactly what `{*}{}` contributes.
fn literal_list_words(text: &str, tok: Token) -> Option<Vec<(String, Token)>> {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return None;
    }
    if tok.kind == TokenType::Esc && crate::naming::is_dynamic_word(text) {
        return None;
    }
    let base = tok.span.start() + u32::from(tok.content_offset);
    let mut words = Vec::new();
    let mut pos = 0usize;
    while let Some(element) = tcl_syntax::list::find_element(text, pos).ok()? {
        let value = text.get(element.value.clone())?;
        if !element.braced && crate::naming::is_dynamic_word(value) {
            return None;
        }
        let start = base + u32::try_from(element.value.start).ok()?;
        let end = base + u32::try_from(element.value.end).ok()?;
        words.push((
            value.to_string(),
            Token {
                kind: TokenType::Esc,
                span: Span::new(start, end),
                content_offset: 0,
                in_quote: false,
            },
        ));
        pos = element.next;
    }
    Some(words)
}

/// One piece of a manufacturer prologue word, as
/// [`Analyser::manufacturer_injected_members`] accounts for it.
#[derive(Debug, PartialEq, Eq)]
enum ProloguePiece<'a> {
    /// A `[…]` command substitution — one prologue member to read.
    Substitution,
    /// A `$name` / `${name}` variable read.
    VarRead(&'a str),
    /// Whitespace, a statement separator, or an escaped separator — text
    /// that joins pieces without contributing any member of its own.
    Separator,
    /// Anything else: literal prologue text, or a fragment the scanner
    /// cannot classify.  Its presence means the prologue is *not* fully
    /// read.
    Opaque,
}

/// Split a manufacturer prologue word into the pieces its reader must
/// account for.
///
/// Operates on the word's round-tripped source text (the segmenter's own
/// per-word reconstruction), which preserves every substitution verbatim.
/// A single `Opaque` piece is enough for the caller to abstain, so the scan
/// stops at the first one.
fn prologue_pieces(word: &str) -> Vec<ProloguePiece<'_>> {
    let mut pieces = Vec::new();
    let mut rest = word;
    while !rest.is_empty() {
        let first = rest.as_bytes()[0];
        if first.is_ascii_whitespace() || first == b';' {
            rest = &rest[1..];
            if !matches!(pieces.last(), Some(ProloguePiece::Separator)) {
                pieces.push(ProloguePiece::Separator);
            }
            continue;
        }
        // A backslash escape of a separator (`\;` — the idiomatic way to
        // end the injected statement inside one word) joins, like the
        // separator it escapes.
        if first == b'\\' {
            let escaped = rest.as_bytes().get(1).copied();
            if escaped.is_some_and(|c| c.is_ascii_whitespace() || c == b';' || c == b'n') {
                rest = &rest[2..];
                if !matches!(pieces.last(), Some(ProloguePiece::Separator)) {
                    pieces.push(ProloguePiece::Separator);
                }
                continue;
            }
            pieces.push(ProloguePiece::Opaque);
            break;
        }
        if first == b'[' {
            let Some(end) = matching_bracket(rest) else {
                pieces.push(ProloguePiece::Opaque);
                break;
            };
            pieces.push(ProloguePiece::Substitution);
            rest = &rest[end + 1..];
            continue;
        }
        if first == b'$' {
            let after = &rest[1..];
            if let Some(braced) = after.strip_prefix('{') {
                let Some(close) = braced.find('}') else {
                    pieces.push(ProloguePiece::Opaque);
                    break;
                };
                pieces.push(ProloguePiece::VarRead(&braced[..close]));
                rest = &braced[close + 1..];
                continue;
            }
            let len = after
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
                .unwrap_or(after.len());
            if len == 0 {
                pieces.push(ProloguePiece::Opaque);
                break;
            }
            pieces.push(ProloguePiece::VarRead(&after[..len]));
            rest = &after[len..];
            continue;
        }
        pieces.push(ProloguePiece::Opaque);
        break;
    }
    pieces
}

/// Byte offset of the `]` closing the `[` at the start of `text`, honouring
/// nesting.  `None` when it is unbalanced.
fn matching_bracket(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in text.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// Read one prologue member statement from a manufacturer override into the
/// **call-site-independent template** the factory publishes.
///
/// A literal word is kept with its own token (it lives in the manufacturer's
/// body, which is where the reference genuinely is written).  A `{*}$param`
/// word becomes a [`FactoryWord::CallerSplice`] of the creation call's
/// corresponding argument, resolved per call.  Anything else — a substitution
/// with no statically-known value, or a `{*}` over one — yields `None`, and
/// the caller abstains rather than inventing a superclass list.
fn template_injected_member(seg: &SegmentedCommand, params: &[&str]) -> Option<FactoryMember> {
    let texts_in = seg.args();
    let tokens_in = seg.arg_tokens();
    let mut words: Vec<FactoryWord> = Vec::new();
    for (i, (text, tok)) in texts_in.iter().zip(tokens_in.iter()).enumerate() {
        let expanded = seg
            .expand_word
            .as_ref()
            .and_then(|e| e.get(i + 1).copied())
            .unwrap_or(false);
        if expanded {
            let refs = crate::var_refs::scan_var_ref_forms(text);
            let [name] = refs.as_slice() else {
                return None;
            };
            // Parameter `i` of the override binds argument `i + 1` of the
            // call (argument 0 being the manufacturer subcommand itself).
            let arg_index = params.iter().position(|p| *p == name)? + 1;
            words.push(FactoryWord::CallerSplice(arg_index));
            continue;
        }
        if crate::naming::is_dynamic_word(text) {
            return None;
        }
        words.push(FactoryWord::Literal {
            text: text.clone(),
            token: *tok,
        });
    }
    Some(FactoryMember { words })
}

/// Resolve one injected-member template against the creation call that
/// triggered it.
///
/// The literal words keep the tokens they were templated with; a
/// [`FactoryWord::CallerSplice`] contributes the call argument's literal list
/// elements, each with its own call-site token.  `None` when this call's
/// argument is not a statically-known list — the caller then marks the
/// class's inheritance unknown rather than claiming an injection it cannot
/// spell.
fn resolve_factory_member(
    member: &FactoryMember,
    args: &[String],
    arg_tokens: &[Token],
) -> Option<InjectedMember> {
    let mut member_words: Vec<String> = Vec::new();
    let mut member_tokens: Vec<Token> = Vec::new();
    for word in &member.words {
        match word {
            FactoryWord::Literal { text, token } => {
                member_words.push(text.clone());
                member_tokens.push(*token);
            }
            FactoryWord::CallerSplice(arg_index) => {
                let call_tok = *arg_tokens.get(*arg_index)?;
                let call_text = args.get(*arg_index)?;
                for (element, element_tok) in literal_list_words(call_text, call_tok)? {
                    member_words.push(element);
                    member_tokens.push(element_tok);
                }
            }
        }
    }
    Some(InjectedMember {
        texts: member_words,
        argv: member_tokens,
    })
}

/// The word layout a class factory's `args[0]` manufacturer imposes on this
/// creation call, with its injected members resolved against the call's own
/// arguments.
///
/// A subcommand the factory does not override runs the inherited `oo::class`
/// manufacturer, so the builtin `create Name Body` layout applies with
/// nothing injected.
fn manufacturer_layout(
    factory: &ClassFactory,
    builtin: &tcl_registry::definer::ManufacturerMethod,
    args: &[String],
    arg_tokens: &[Token],
) -> Option<ManufacturerLayout> {
    let Some(spec) = factory.overrides.get(&args[0]) else {
        return ManufacturerLayout::builtin(builtin);
    };
    let injected = spec.injected.as_ref().and_then(|members| {
        members
            .iter()
            .map(|m| resolve_factory_member(m, args, arg_tokens))
            .collect::<Option<Vec<_>>>()
    });
    Some(ManufacturerLayout {
        name_arg: spec.name_arg,
        body_arg: spec.body_arg,
        inheritance_unknown: injected.is_none(),
        injected: injected.unwrap_or_default(),
    })
}

/// Bundled arguments for [`Analyser::walk_proc_body_in_new_scope`] — kept
/// under clippy's argument-count limit (mirrors `TaintScan` in `taint.rs`).
#[derive(Clone, Copy)]
#[allow(clippy::struct_field_names)]
struct ProcBodyWalkArgs<'a> {
    /// Parent scope path, *before* the new proc scope is pushed.
    path: &'a [usize],
    /// The routine's name after [`Analyser::resolve_dynamic_word`] — the
    /// `Scope` name is derived from it via [`scope_name_for_routine`], never
    /// from the raw written word.
    resolved_name: &'a str,
    body_span: Span,
    arg_tokens: &'a [Token],
    name_tok: Token,
    params: &'a [crate::signature_scan::types::ParamDef],
    args: &'a [String],
    body_tok: Token,
    ns_prefix: &'a str,
}

impl Analyser {
    /// Handle the `set` command: `set var ?value?`.
    ///
    /// - **Two-arg form** (`set var value`) — defines the variable
    ///   in the scope at `scope_path` and tracks the value as a
    ///   constant string when the value is a single-token literal
    ///   (no interpolation, no command sub).
    /// - **One-arg form** (`set var`) — records a var read on the
    ///   variable.  Tcl `set` with no value returns the current
    ///   value, so this is a reference, not a definition.
    ///
    /// `single_token_word` parallels `args` and `arg_tokens` —
    /// `true` when the corresponding word is a single atomic
    /// token, i.e. when the word's text is the same as a single
    /// token's raw text.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Set`];
    /// only the argument-shape checks live here.
    pub fn handle_set_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        single_token_word: &[bool],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }

        // Arg-count branch: two-arg form defines, one-arg form reads.
        let Some(name_tok) = arg_tokens.first() else {
            return;
        };
        if args.len() >= 2 {
            self.define_var(&args[0], *name_tok, scope_path, true, None);
        } else {
            // `[set {$n}]` reads the variable literally called `$n` — the
            // braces suppressed substitution, so the word's content is the
            // name (issue #1078).
            self.record_var_read_braced(
                &args[0],
                name_tok.span,
                scope_path,
                name_tok.kind == TokenType::Str,
            );
        }

        // Track constant-string assignments for regex propagation.
        // Skipped for the 1-arg read form (no value to track).
        if args.len() < 2 || arg_tokens.len() < 2 {
            return;
        }
        let value_token = arg_tokens[1];
        let value_is_single_token = single_token_word.get(1).copied().unwrap_or(false);
        let value_token_kind = value_token.kind;
        if value_is_single_token && matches!(value_token_kind, TokenType::Esc | TokenType::Str) {
            self.set_const_string(&args[0], args[1].clone(), value_token.span, scope_path);
            self.clear_interp_var_binding(&args[0], scope_path);
        } else if let Some(words) = interp_create_words_from_value(&args[1]) {
            // `set VAR [interp create ?-safe? ?--? ?path?]` (issue #923
            // idx 9) — bind VAR, in this scope, to the interpreter-domain
            // key this call records. Mirrors `record_instance_creation`'s
            // TclOO `set g [Foo new]` value-flow shape, but scope-chain
            // -aware like `const_strings` rather than flat like
            // `instance_classes` — see `interp_var_bindings`'s doc for why.
            self.clear_const_string(&args[0], scope_path);
            let (safe, path) = parse_interp_create_words(&words);
            match path {
                Some(p) if crate::naming::is_dynamic_word(p) => {
                    // A dynamic path argument (not just a missing one) —
                    // mirrors `handle_interp_create_command`'s own
                    // handling of the identical shape: existence becomes
                    // unknowable file-wide, nothing recorded.
                    self.dynamic_interp_ops = true;
                    self.clear_interp_var_binding(&args[0], scope_path);
                }
                _ => {
                    // A literal path resolves to its qualified key; a
                    // missing path (Tcl auto-generates a fresh, always-
                    // unique name) gets a synthetic per-call-site key —
                    // mirrors `handle_namespace_eval_command`'s
                    // `@dynns@<offset>` pattern — so two unrelated `set
                    // VAR [interp create -safe]` call sites never collide
                    // just because they wrote the same variable name.
                    let key = match path {
                        Some(p) => self.qualified_interp_key(p),
                        None => {
                            self.mint_synthetic_offset_name("@autoname@", value_token.span.start())
                        }
                    };
                    self.interpreters.insert(
                        key.clone(),
                        super::state::InterpState {
                            safe,
                            ..Default::default()
                        },
                    );
                    self.set_interp_var_binding(&args[0], key, scope_path);
                }
            }
        } else if value_is_single_token
            && value_token_kind == TokenType::Cmd
            && let Some(folded) = self.try_fold_const_cmd_subst_rhs(&args[1], scope_path)
        {
            // `set VAR [cmd …]` whose substitution is a compile-time
            // constant (issue #1132): the registry `const_fold` /
            // frame-fact engine proves the value, so VAR enters the same
            // constant-string lattice a literal RHS does — unblocking the
            // `${ns}::setdef`-style navigation chain
            // (`resolve_dynamic_command_head`) for the
            // `set ns [namespace qualifiers ::tc::X]` shape.
            self.set_const_string(&args[0], folded, value_token.span, scope_path);
            self.clear_interp_var_binding(&args[0], scope_path);
        } else {
            self.clear_const_string(&args[0], scope_path);
            self.clear_interp_var_binding(&args[0], scope_path);
        }
    }

    /// Fold a `set` command's single-token `[cmd …]` RHS to its constant
    /// value via the shared engine ([`crate::const_subst::ConstSubstCtx`]),
    /// or `None` to abstain (issue #1132).
    ///
    /// Sound-by-construction gates, in evaluation order:
    ///
    /// 1. **Fold surface** — [`crate::const_subst::head_may_fold`]: the
    ///    static head must carry a registry fold or frame-fact table. Also
    ///    what keeps the trust oracle below lazy: no candidate, no cost.
    /// 2. **Whole-module trust** — [`Self::whole_file_command_trust`]: a
    ///    `rename` / `interp alias` / shadowing `proc` anywhere in the
    ///    module (even later in the file, even buried in a body) unbinds
    ///    the head from its builtin semantics, so the fold declines. The
    ///    mid-walk `renamed_commands` map is deliberately NOT used — it
    ///    only knows about mutations *before* this point.
    /// 3. **Frame facts** — `[self class]` folds only inside an
    ///    instance-side `TclOO` method of a statically-named class
    ///    ([`super::types::Scope::oo_defining_class`]) whose class command
    ///    binding is itself trusted; class-side frames (`classmethod`,
    ///    `self method`), snit / itcl members, and init scripts abstain
    ///    (`self class` raises there — tclsh 9.0.4).
    /// 4. **Constant arguments** — `$var` words resolve only through the
    ///    *dominating* constant lattice
    ///    ([`Self::lookup_dominating_const_string`]), so a
    ///    branch-conditional binding abstains.
    fn try_fold_const_cmd_subst_rhs(
        &mut self,
        value_text: &str,
        scope_path: &[usize],
    ) -> Option<String> {
        let registry = self.registry.clone()?;
        let trimmed = value_text.trim();
        let inner = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(trimmed);
        if !crate::const_subst::head_may_fold(&registry, inner) {
            return None;
        }
        let trust = self.whole_file_command_trust()?;
        let defining_class = self.oo_defining_class_at(scope_path);
        if let Some(class) = &defining_class
            && !trust.trusts_proc_binding(class)
        {
            return None;
        }
        let dialect = &self.result.dialect;
        let dialect = (!dialect.is_empty()).then_some(dialect.as_str());
        let trusts = |name: &str| trust.trusts(name);
        let lookup = |name: &str| {
            self.lookup_dominating_const_string(name, scope_path)
                .map(str::to_owned)
        };
        crate::const_subst::ConstSubstCtx {
            registry: &registry,
            dialect,
            defining_class: defining_class.as_deref(),
            trusts: &trusts,
            lookup_var: &lookup,
        }
        .fold_cmd_subst(inner)
    }

    /// The defining class of the innermost enclosing **instance-side
    /// `TclOO` method** scope, or `None` when the current frame is not one
    /// (a proc, namespace, class-side member, snit / itcl member, or any
    /// nested frame inside the method that opens a new Tcl frame). See
    /// [`super::types::Scope::oo_defining_class`].
    fn oo_defining_class_at(&self, scope_path: &[usize]) -> Option<String> {
        let mut scope = &self.result.global_scope;
        let mut found: Option<String> = None;
        for &i in scope_path {
            scope = scope.children.get(i)?;
            // Every scope node in this tree (namespace, proc, method) is a
            // frame change; only an instance-side method scope carries the
            // fact, and any deeper frame resets it.
            found.clone_from(&scope.oo_defining_class);
        }
        found
    }

    /// Handle a `global` declaration: a flat list of names; each gets
    /// a var binding with `warn_if_unused = false` (declared, not
    /// "set but unused").
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Global`].
    pub fn handle_global_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        // `global ::ns::v` makes the *unqualified tail* (`v`) a local alias
        // to the global variable, so define the tail name locally (matches
        // Tcl + completion's expectation of both `$v` and `$::ns::v`).
        for (i, name) in args.iter().enumerate() {
            let local = name.rsplit("::").next().unwrap_or(name);
            // A dynamic name (`global $dyn`) computes its name at runtime — not
            // a static declaration.
            if let Some(tok) = arg_tokens.get(i)
                && !crate::naming::is_dynamic_word(name)
            {
                self.define_var(local, *tok, scope_path, false, None);
                // `global v` aliases the global cell `::v`; `global ::ns::v`
                // aliases `::ns::v` as written.  Record the target so every
                // `global` alias and the global declaration unify.
                let target = if name.starts_with("::") {
                    name.clone()
                } else {
                    format!("::{name}")
                };
                // The declaration word *is* the cell's name here (`global v`
                // / `global ::ns::v`), so a rename of the cell rewrites this
                // very word — see `VarDef::link_target_span`.
                self.set_var_link_target(local, scope_path, target, tok.span);
            }
        }
    }

    /// Handle a `variable` declaration: alternating `name ?value?`
    /// pairs; only the names get bindings. The optional value words
    /// are skipped (the IR pass handles their assignment if the value
    /// form actually fires).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Variable`].
    pub fn handle_variable_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        // `variable name ?value? name ?value? ...`
        // Each `name` aliases the cell `<current-namespace>::<name>`; every
        // `variable name` across that namespace's procs, plus the namespace
        // -level declaration, shares that target and so unifies.
        let ns = self.command_resolution_namespace(scope_path);
        let ns_prefix = ns.trim_end_matches("::");
        let mut i = 0;
        while i < args.len() {
            // A dynamic name (`variable $dyn` / `variable [f]`) is computed at
            // runtime — its literal text is not the variable's name, so it is
            // not a static declaration.  Skip it rather than record the
            // substitution text as a variable.
            if let Some(tok) = arg_tokens.get(i)
                && !crate::naming::is_dynamic_word(&args[i])
            {
                // Mirror `global` above: the *unqualified tail* is the local
                // alias name, but the target keeps the FULL qualified path so a
                // relative `variable child::v` aliases `<ns>::child::v` (and an
                // absolute `variable ::x::v` aliases `::x::v`), never the
                // tail-collapsed `<ns>::v`.  Keying the link on the tail is what
                // lets a later `$v` reference share the target and unify.
                let local = args[i].rsplit("::").next().unwrap_or(&args[i]);
                self.define_var(local, *tok, scope_path, false, None);
                let target = if args[i].starts_with("::") {
                    args[i].clone()
                } else {
                    format!("{ns_prefix}::{}", args[i])
                };
                // As with `global`, the declaration word names the cell.
                self.set_var_link_target(local, scope_path, target, tok.span);
            }
            i += if i + 1 < args.len() { 2 } else { 1 };
        }
    }

    /// Record a lightweight named definition for a registry *symbol-definer*
    /// command (`tcltest::test NAME …`, …) so it appears in the document /
    /// workspace outline alongside procs, classes, and variables (issue #790).
    ///
    /// Everything command-specific is registry data: which argument holds the
    /// name, which holds the description, and the outline category all come from
    /// the command's [`tcl_registry::SymbolDef`] — there is no `cmd == "test"`
    /// arm here, so registering the next such command (a benchmark runner, a
    /// custom test wrapper) is a one-line spec change.
    ///
    /// The name is resolved through the analyser's constant-propagation lattice
    /// (`const_strings` / [`Self::lookup_const_string`]): a bare literal, a
    /// constant `$var`, or a quoted literal all resolve identically, and a
    /// genuinely dynamic name (`test $unknown …`) is skipped rather than
    /// recorded as the literal text `$unknown`.  This is a void handler — it
    /// records the symbol and returns, leaving the command's body recursion to
    /// the generic `ArgRole::Body` walk.
    pub fn handle_defines_symbol(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) {
        let Some(sym) = self.resolve_symbol_definer(cmd_name, scope_path) else {
            return;
        };
        // A command that defines only in one form (the `testConstraint NAME
        // value` setter, not the `testConstraint NAME` getter) records a symbol
        // only when its defining argument is present.
        if let Some(req) = sym.requires_arg
            && args.get(req as usize).is_none()
        {
            return;
        }
        let name_idx = sym.name_arg as usize;
        let (Some(name_tok), Some(name_text), Some(name_single)) = (
            arg_tokens.get(name_idx).copied(),
            args.get(name_idx),
            arg_single.get(name_idx).copied(),
        ) else {
            return;
        };
        // Resolve the name through constant propagation; skip a dynamic name.
        let Some(name) = self.resolve_const_word(name_text, name_tok, name_single, scope_path)
        else {
            return;
        };
        if name.is_empty() {
            return;
        }

        // The description argument, but only when it too resolves to a constant.
        let detail = sym.detail_arg.and_then(|d| {
            let di = d as usize;
            let tok = arg_tokens.get(di).copied()?;
            let text = args.get(di)?;
            let single = arg_single.get(di).copied().unwrap_or(false);
            self.resolve_const_word(text, tok, single, scope_path)
                .map(|d| summarise_detail(&d))
        });

        // Fold range: the name token through the end of the last argument.
        let end = arg_tokens
            .last()
            .map_or(name_tok.span.end(), |t| t.span.end())
            .max(name_tok.span.end());
        let full_span = Span::new(name_tok.span.start(), end);

        // A registry definer called inside `proc ::ns::p {}` creates its symbol
        // in `::ns` (the proc's defining namespace), not `::` — the
        // command-resolution rule, not the lexical one (issue #923 idx 85).
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let qualified = qualify(ns_prefix.trim_start_matches(':'), &name);

        let symbol = DefinedSymbol {
            name,
            qualified_name: qualified,
            kind: sym.kind,
            name_span: name_tok.span,
            full_span,
            detail,
        };
        self.result.all_defined_symbols.push(symbol.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.defined_symbols.push(symbol);
        }
    }

    /// Resolve `cmd_name` at `scope_path` to its [`tcl_registry::SymbolDef`], if
    /// the command (or an imported bare form of it) declares one.
    ///
    /// Mirrors the imported-command fallback the body-role walk uses so a bare
    /// `test` reached through `namespace import ::tcltest::*` resolves to the
    /// qualified `tcltest::test` spec — the import must be in effect at the call
    /// site (its own namespace, or a global-scope import via Tcl's `::`
    /// fallback).
    fn resolve_symbol_definer(
        &mut self,
        cmd_name: &str,
        scope_path: &[usize],
    ) -> Option<tcl_registry::SymbolDef> {
        let dialect = self.profile.availability_mask;
        // Which namespace's imports are in effect is the *command-resolution*
        // namespace: a proc body resolves unqualified commands (and so the
        // imports covering them) in the proc's defining namespace, which the
        // lexical walk misses for a qualified-name proc (issue #923 idx 85).
        let cur_ns = if self.result.namespace_imports.is_empty() {
            String::new()
        } else {
            self.command_resolution_namespace(scope_path)
        };
        // Does `cmd_name` (or an imported bare form of it) name a registry
        // definer?  The `registry` borrow is confined to this block so the
        // shadowing check below can re-borrow `self`.
        let sym = {
            let registry = self.registry.as_deref()?;
            if let Some(sym) = registry.defines_symbol(cmd_name, dialect) {
                Some(*sym)
            } else if cmd_name.contains("::") {
                None
            } else {
                self.result
                    .namespace_imports
                    .iter()
                    .filter(|imp| imp.ns == cur_ns || imp.ns == "::")
                    .find_map(|imp| {
                        let candidate = if let Some(prefix) = imp.pattern.strip_suffix('*') {
                            format!("{prefix}{cmd_name}")
                        } else if imp.pattern.rsplit("::").next() == Some(cmd_name) {
                            imp.pattern.clone()
                        } else {
                            return None;
                        };
                        registry.defines_symbol(&candidate, dialect).copied()
                    })
            }
        }?;
        // A user-defined proc of the same name shadows the imported / built-in
        // definer under Tcl's command resolution — a local `proc test` beats an
        // imported `::test` — so a bare call to it invokes that proc, not the
        // tcltest definer, and must not be recorded as a test symbol.
        if self.resolve_proc_call(cmd_name, scope_path).is_some() {
            return None;
        }
        Some(sym)
    }

    /// Resolve a single argument word to a constant string, or `None` when it is
    /// not statically constant.
    ///
    /// The analyser-level counterpart to the optimiser's SCCP: a plain literal
    /// word (its delimiters already stripped into `text`) is itself constant; a
    /// bare `$var` resolves through the constant-string lattice
    /// ([`Self::lookup_const_string`]); anything with embedded substitution or
    /// concatenation is not statically known.
    fn resolve_const_word(
        &self,
        text: &str,
        tok: Token,
        is_single: bool,
        scope_path: &[usize],
    ) -> Option<String> {
        if !is_single {
            return None;
        }
        match tok.kind {
            TokenType::Str | TokenType::Esc => Some(text.to_string()),
            TokenType::Var => {
                let sm = Analyser::source_map(
                    &self.source,
                    &self.cached_line_index,
                    self.cached_line_index_source_len,
                );
                let var_name = sm.token_text(tok);
                self.lookup_const_string(var_name, scope_path)
                    .map(str::to_string)
            }
            _ => None,
        }
    }

    /// Handle the `proc` command: `proc NAME PARAMS BODY`.
    ///
    /// Returns `true` when the command was handled (callers use the
    /// bool to decide whether further processing is needed), `false`
    /// when the input doesn't match the expected shape.
    ///
    /// Records the [`ProcDef`] in
    /// both `scope.procs` (keyed by simple name) and
    /// `result.all_procs` (keyed by qualified name).  When the
    /// body is a braced literal, opens a fresh
    /// [`super::types::ScopeKind::Proc`] child scope, defines each parameter
    /// in it, and re-segments the body via
    /// [`crate::segmenter::segment_commands_with_offset`] —
    /// every body command is dispatched through
    /// [`Analyser::process_command`] with the new scope path.
    /// Body recursion does **not** invoke segmenter recovery —
    /// that fires only at the top level.
    /// Dynamic bodies (`$body`, `[gen]`) skip the walk because
    /// they cannot be statically re-segmented; the proc record
    /// is still emitted so downstream consumers see the
    /// signature.
    ///
    /// Infer per-parameter traits for a proc body.  Always runs
    /// the shallow pass (`infer_param_traits`); when
    /// [`Self::deep_param_traits`] is set, also runs the
    /// recursive deep pass (`infer_param_traits_deep`) and
    /// unions both via [`super::param_traits::merge_traits`].
    /// Threads the analyser's pre-built dialect-aware registry
    /// through so iRules-only `arg_role_resolver` callbacks
    /// (e.g. `when`) fire on body args.  When [`Self::registry`]
    /// is `None` (outside an active `analyse` run, e.g. a unit-
    /// test harness) we skip the inference rather than pay the
    /// cost of a fresh `build_default` on every proc.
    ///
    /// Returns the trait map together with
    /// [`ProcDef::caller_frame_params`](super::types::ProcDef::caller_frame_params)
    /// — the strictly narrower "this parameter's value names a variable in the
    /// *immediate caller's* frame" fact, which the trait map alone cannot
    /// answer because it records no frame level — and
    /// [`ProcDef::caller_frame_literals`](super::types::ProcDef::caller_frame_literals),
    /// the literal caller-frame names the body spells itself (`upvar 1 name
    /// name`, issue #1139).  Computed here, from the same
    /// body text and the same dialect config, so the three can never be
    /// derived from different views of the proc.
    fn infer_proc_param_traits(
        &self,
        params: &[crate::signature_scan::types::ParamDef],
        body_text: &str,
    ) -> ProcParamFacts {
        let Some(registry) = self.registry.as_deref() else {
            return (
                std::collections::HashMap::new(),
                std::collections::HashSet::new(),
                std::collections::HashMap::new(),
            );
        };
        let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        // One environment for all four scans, so the traits, the caller-frame
        // params, and the caller-frame literals can never be derived from
        // different views of the proc *or* of the document's command bindings
        // (issue #1275).
        let env = super::param_traits::TraitScanEnv {
            registry,
            stub_overlay: self.stub_overlay.as_ref(),
            config: self.lexer_config(),
            identities: &self.head_identities,
        };
        let caller_frame_params =
            super::param_traits::caller_frame_upvar_params(&param_names, body_text, env);
        let caller_frame_literals =
            super::param_traits::caller_frame_literal_targets(body_text, env);
        let shallow = super::param_traits::infer_param_traits(&param_names, body_text, env);
        let traits = if self.deep_param_traits {
            let deep = super::param_traits::infer_param_traits_deep(&param_names, body_text, env);
            super::param_traits::merge_traits(shallow, deep)
        } else {
            shallow
        };
        (traits, caller_frame_params, caller_frame_literals)
    }

    /// **W113** — a `proc` name shadows a built-in command.
    ///
    /// The check runs against both the unqualified `raw_name` and the
    /// fully-qualified form, with the leading `::` trimmed (the registry
    /// indexes by bare command name).  The inner borrow on
    /// `self.builtin_command_names()` is dropped before the diagnostic push so
    /// `self.result` is free to mutate.
    ///
    /// Only *core global* built-ins are shadow-worthy (unqualified `set` /
    /// `dict` / …).  A namespace-qualified match (`::base64::encode`,
    /// `::snit::type`) is a library/package command living in its own namespace
    /// — its own definition or a deliberate override, not shadowing a built-in.
    /// And the overridable Tcl *library* procedures (`unknown`, `history`,
    /// `auto_*`, …) are script-defined, documented user-replaceable overlays.
    /// A third-party/package command (`argparse`, a tcllib/itcl/ticklecharts
    /// proc, …) is excluded too (issue #923 idx 11): `registry.command_names()`
    /// is the full package-inclusive command universe, not the set of names a
    /// bare Tcl interpreter actually starts with, so a proc named after a
    /// registered package command (e.g. `proc ::argparse {args} {...}`
    /// defining the `argparse` package's own entry point) is not shadowing
    /// anything until that package is actually loaded — and even then it is
    /// the package's own definition, not a redefinition of a core built-in.
    /// The gate reads `CommandSpec::owning_package` (`required_package` or
    /// its `tcllib_package` alias) rather than a hardcoded package-name list,
    /// so it stays correct as new package specs are added. A package a
    /// profile ships **ambient** (an F5 command pack, an EDA vendor tool
    /// surface) is the exception: that profile's real, always-present command
    /// surface, so it keeps firing W113 like any other core built-in (e.g.
    /// iRules' `pool`, which — being ambient and never `required_package`-gated
    /// in the first place — is untouched by this filter).
    fn emit_w113_proc_shadows_builtin(&mut self, raw_name: &str, qualified: &str, name_span: Span) {
        let normalised_proc: String = raw_name.trim_start_matches(':').to_string();
        let normalised_qual: String = qualified.trim_start_matches(':').to_string();
        let shadow_name: Option<String> = {
            let builtins = self.builtin_command_names();
            // A `tcl::mathop` operator (`%`, `+`, `eq`, …) carries a bare-name
            // registry entry only so an `import`ed / `namespace path`-reachable
            // call resolves; the command itself lives in `::tcl::mathop`, not the
            // global namespace, so `proc %` shadows nothing reachable. Detect it
            // by its qualified spelling and treat it like the other namespaced
            // (non-core-global) commands the check already skips.
            let is_mathop = |n: &str| builtins.contains(&format!("tcl::mathop::{n}"));
            if builtins.contains(&normalised_proc) && !is_mathop(&normalised_proc) {
                Some(normalised_proc.clone())
            } else if builtins.contains(&normalised_qual) && !is_mathop(&normalised_qual) {
                Some(normalised_qual.clone())
            } else {
                None
            }
        };
        let shadow_name = shadow_name.filter(|name| {
            !name.contains("::")
                && !overridable_library_procs().contains(name.as_str())
                && !self.is_package_gated_non_ambient(name)
        });
        if shadow_name.is_some() {
            // The permissive fallback profile means "no specific dialect" —
            // no parenthetical label (the old empty-string contract).
            let dialect_label = if self.profile.is_fallback() {
                String::new()
            } else {
                format!(" ({})", self.dialect())
            };
            let message = format!("Procedure '{raw_name}' shadows built-in command{dialect_label}");
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W113,
                span: name_span,
                message,
                severity: super::types::Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// Whether `name` (a bare or fully-qualified command name already known
    /// to be in `registry.command_names()`) resolves to a `CommandSpec` gated
    /// behind a `required_package` / `tcllib_package` (`CommandSpec::
    /// owning_package`) that this profile does **not** ship ambiently
    /// (issue #923 idx 11).
    ///
    /// Data-driven, not a hardcoded package-name list: the answer comes
    /// straight from the resolved spec's package attribution and the
    /// profile's own `is_ambient_package` query (the same query
    /// `emit_missing_package_require_diagnostics` / W120 already uses for the
    /// converse fact), so a new package-gated spec is covered automatically —
    /// no per-package entry to remember to add here.
    fn is_package_gated_non_ambient(&self, name: &str) -> bool {
        use tcl_registry::ProfileQueries;
        let registry = tcl_registry::cache::registry_for_profile(self.profile);
        self.profile
            .resolve_command(registry, name)
            .and_then(tcl_registry::CommandSpec::owning_package)
            .is_some_and(|pkg| !self.profile.is_ambient_package(pkg))
    }

    /// **W314.** The definition's name has no absolute (fully-qualified)
    /// written form (issue #934).  A written colon run of two or more is a
    /// namespace separator (the whole run collapses, `TclGetNamespaceForQualName`,
    /// invariant C 8.4→9.1), so an **all-colon simple name** — only `:` is
    /// writable — can never be spelled absolutely: `namespace which :` renders
    /// `:::`, which parses back to the global `{}` command, ensembles cannot
    /// dispatch an exported `:`, and `interp alias` / callback qualification
    /// break the same way.  The command remains reachable by *relative*
    /// lookup only.  (tclsh 8.6/9.0-pinned.)
    pub(super) fn emit_w314_no_absolute_name(&mut self, raw_name: &str, name_span: Span) {
        // The written simple name is the last colon-run-delimited segment.  A
        // name *ending* in a run names the `{}` command, which IS addressable
        // (`::x::`); an all-colon segment can only be a lone `:` (a run of ≥2
        // is consumed as the separator).
        let unaddressable = !tcl_syntax::naming::ends_with_separator(raw_name.as_bytes())
            && crate::naming::qualifier_segments(raw_name.as_bytes())
                .last()
                .is_some_and(|seg| *seg == b":");
        if !unaddressable {
            return;
        }
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W314,
            span: name_span,
            message: format!(
                "'{raw_name}' has no absolute (fully-qualified) name — a written colon run \
                 is a namespace separator, so this definition is reachable only by \
                 unqualified/relative lookup ([namespace which] output will not resolve, \
                 and ensembles cannot dispatch it)"
            ),
            severity: super::types::Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **W315.** Drain `class`'s recorded
    /// [`DefinitionAbort`](super::types::DefinitionAbort)s and report each as
    /// "this class definition cannot run" (issue #1120).
    ///
    /// Three oracle-pinned shapes abort a whole `TclOO` definition body, so no
    /// class is created at all — byte-identical on tclsh 9.0.4 and 8.6.14:
    ///
    /// ```tcl
    /// oo::class create ::E1 { deletemethod ghost ; method ghost {} {} }
    /// ;# -> method ghost does not exist              [info object isa class ::E1] -> 0
    /// oo::class create ::E2 { self { method cm {} {} } ; deletemethod cm }
    /// ;# -> method cm does not exist                 (cross-side: `cm` is class-side)
    /// oo::class create ::E3 { method a {} {} ; method b {} {} ; renamemethod a b }
    /// ;# -> method called b already exists
    /// ```
    ///
    /// unlike a cross-side `export` / `unexport`, which is a **silent no-op**
    /// and must stay silent here (`oo::define E { self unexport onlyinst }` over
    /// an instance-only `onlyinst` succeeds and changes nothing).
    ///
    /// `via_define` is the gate, and it is exactly the right one: it marks the
    /// records that describe a class **created in another file**, where a
    /// retraction naming a member this record cannot see is the *normal*
    /// cross-file shape (`oo::define ::C { deletemethod m }` against an `m`
    /// declared in `a.tcl`) rather than an error — that retraction travels as a
    /// [`ClassDef::retracted_members`](super::types::ClassDef::retracted_members)
    /// tombstone instead. A same-file
    /// `oo::define` extending a class created earlier in the file is **not** a
    /// stub: it reuses that class's own record, member tables included, so its
    /// table state is complete and an absent name really is the hard error.
    /// The walker records both readings; this is where the one that applies is
    /// kept and the other dropped.
    ///
    /// The partial class stays recorded either way — a body that cannot run has
    /// no outline at all in real Tcl, but degrading navigation to nothing is
    /// worse than describing what the author meant, the same judgement parse
    /// errors already get.
    pub(super) fn emit_w315_definition_cannot_run(&mut self, class: &mut super::types::ClassDef) {
        let aborts = core::mem::take(&mut class.definition_aborts);
        if class.via_define {
            // A cross-file extension stub knows nothing about the class's real
            // member tables, so it has no evidence for any of these — the
            // retraction's tombstone carries the fact instead.
            return;
        }
        // The walker records a tombstone *and* an abort for the same retraction,
        // because only here is it known which reading applies. This record has
        // complete tables, so the retraction is the hard error, not a cross-file
        // removal: the tombstone would suppress a name in some other document
        // for a body that never runs at all. Drop it and keep the diagnostic.
        if !aborts.is_empty() {
            class.retracted_members.retain(|record| {
                !aborts.iter().any(|a| {
                    a.kind == super::types::DefinitionAbortKind::MissingMember
                        && a.member == record.member
                })
            });
        }
        for abort in aborts {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W315,
                span: abort.span,
                message: abort.message(),
                severity: super::types::Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// **W314**, namespace flavour: a `namespace eval` (or sibling) whose
    /// written name carries an **all-colon segment** (`namespace eval :`)
    /// creates a namespace no absolute path can spell — its entire contents
    /// are reachable only relatively (`namespace inscope : :`), and
    /// `namespace current` inside renders an unresolvable `:::`.
    pub(super) fn emit_w314_unaddressable_namespace(&mut self, written_ns: &str, span: Span) {
        let has_all_colon_segment = crate::naming::qualifier_segments(written_ns.as_bytes())
            .into_iter()
            .any(|seg| seg == b":");
        if !has_all_colon_segment {
            return;
        }
        self.result.diagnostics.push(super::types::Diagnostic {
            code: DiagCode::W314,
            span,
            message: format!(
                "namespace '{written_ns}' has no absolute (fully-qualified) path — a \
                 written colon run is a namespace separator, so everything defined \
                 inside is reachable only by relative lookup"
            ),
            severity: super::types::Severity::Warning,
            fixes: Vec::new(),
        });
    }

    /// **W218.** `args` declared anywhere but the final parameter position
    /// is an ordinary parameter named `args` — C Tcl sets `VAR_IS_ARGS`
    /// only on the last formal (`tclProc.c`), so the variadic
    /// collect-the-rest meaning is silently lost.  Anchors at the
    /// parameter's own name span; `fallback_tok` is used when the name
    /// span could not be recovered.  Shared by the `proc`, `apply`, and
    /// OO-method param walks.
    pub(super) fn emit_w218_args_not_final(
        &mut self,
        params: &[crate::signature_scan::types::ParamDef],
        param_spans: &[tcl_lexer::Span],
        fallback_tok: Token,
    ) {
        let Some(last) = params.len().checked_sub(1) else {
            return;
        };
        for (i, p) in params.iter().enumerate() {
            if i == last || p.name != "args" {
                continue;
            }
            let span = param_spans.get(i).copied().unwrap_or(fallback_tok.span);
            self.result.diagnostics.push(super::types::Diagnostic {
                code: DiagCode::W218,
                span,
                message: "`args` here is an ordinary parameter — it only has its special                           collect-the-rest meaning as the final parameter. Move it last, or                           rename it if a plain parameter is intended."
                    .to_string(),
                severity: super::types::Severity::Warning,
                fixes: Vec::new(),
            });
        }
    }

    /// Handle a `proc name params body` definition: record the procedure
    /// (name, parameter list, body, and harvested doc-comment) in the
    /// analysis result and run per-parameter trait inference. Returns `true`
    /// when the definition had the full three-argument shape this handler
    /// consumes.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Proc`].
    pub fn handle_proc_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 3 || arg_tokens.len() < 3 {
            return false;
        }

        let raw_name = &args[0];
        let name_tok = arg_tokens[0];
        // A constant-foldable dynamic name (`proc ::$wtype {args} {...}` with
        // `wtype` a known constant) resolves the same way `rename`'s operands
        // already do (issue #923 idx 86: `tk/library/accessibility.tcl`'s
        // rename-away-and-reinstall idiom names its wrapper proc this way).
        // An unresolvable dynamic name falls back to the raw written text
        // unchanged — this only *improves* the resolvable case, the same
        // scope boundary `resolve_dynamic_word`'s other callers keep.
        let resolved_name = self
            .resolve_dynamic_word(
                raw_name,
                Some(name_tok),
                arg_single.first().copied().unwrap_or(false),
                scope_path,
            )
            .unwrap_or_else(|| raw_name.clone());
        // Home the proc to the *command-resolution* namespace, not the purely
        // lexical one: a proc defined inside another proc's body homes to that
        // enclosing proc's **defining** namespace (the prefix of its qualified
        // name), the way Tcl resolves the `proc` command at run time.  The
        // purely lexical namespace walk skips proc scopes and so homed a
        // nested `proc helper` under `proc a::outer` to `::helper`, overwriting
        // the real global `::helper` in `all_procs`; the command-resolution
        // namespace homes it to `::a::helper`.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        // `qualify` takes the constructed (rooted) namespace key verbatim and
        // `key_tail` inverts the construction — a `char`-pattern colon trim or
        // an `rsplit("::")` here would collapse a lone-colon name (#934).
        let qualified = qualify(&ns_prefix, &resolved_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = name_tok.span;
        let body_tok = arg_tokens[2];
        let body_span = body_tok.span;

        // **W113** — proc name shadows a built-in command.
        self.emit_w113_proc_shadows_builtin(&resolved_name, &qualified, name_span);
        // **W314** — the name has no absolute written form (#934).
        self.emit_w314_no_absolute_name(raw_name, name_span);

        // A **computed** parameter-list word (`proc p [makeargs] {…}`,
        // `proc q $params {…}`) builds the formals from a run-time value, so
        // nothing about them is knowable: the proc's params are unmodelled
        // (issue #1079).  Reading the unresolved word as a one-parameter
        // literal registered a `VarDef` literally named `"[makeargs]"` and
        // made the call-site arity checker demand exactly one argument, which
        // tclsh 9.0.4 / 8.6.16 contradict (`proc makeargs {} {return {a b}}`;
        // `proc p [makeargs] {…}`; `info args p` → `a b`; `p 1 2` runs).
        //
        // The literalness rule itself lives in
        // `signature_scan::params::param_word_is_literal` — the one predicate
        // this tier, the signature-scan tier, and the LSP's cursor classifier
        // all share (issue #1107).
        let params_computed = !crate::signature_scan::params::param_word_is_literal(
            arg_tokens[1].kind,
            arg_single.get(1).copied().unwrap_or(true),
        );
        let params = if params_computed {
            Vec::new()
        } else {
            parse_param_list(&args[1])
        };
        // Doc string: prefer the preceding-comment harvest from
        // the segmenter; fall back to ``extract_body_docstring``
        // (leading comment block at the top of the body).
        let mut doc = std::mem::take(&mut self.last_comment);
        if doc.is_empty() && args.len() >= 3 {
            doc = super::utils::extract_body_docstring(&args[2]);
        }

        // When a user defines the *global* unresolved-command handler,
        // inspect the body to determine which commands it can resolve. The
        // result gates W123 (unresolved command) file-wide — with a
        // user-supplied handler in place we cannot statically prove a
        // command is truly unresolved.
        if self.defines_global_unresolved_handler(&qualified) {
            let info = self.extract_unknown_proc_info(&args[2], &params);
            self.result.unknown_proc_info = Some(info);
        }

        let body_text = &args[2];
        let (param_traits, caller_frame_params, caller_frame_literals) =
            self.infer_proc_param_traits(&params, body_text);

        let proc = ProcDef {
            name: simple,
            qualified_name: qualified.clone(),
            params: params.clone(),
            params_computed,
            name_span,
            body_span,
            doc,
            param_traits,
            caller_frame_params,
            caller_frame_literals,
        };

        // Register globally and in the current scope. ``scope.procs``
        // is keyed by the *simple* (unqualified) proc name (so
        // per-scope lookup and shadowing rules work locally), while
        // ``result.all_procs`` is keyed by the fully-qualified
        // name. The full qualified name is still on
        // ``ProcDef.qualified_name`` for callers that need it.
        self.register_proc_definition(&qualified, &proc, name_span);
        let simple_key = proc.name.clone();
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.procs.insert(simple_key.clone(), proc);
        }
        // A `proc` defined anywhere inside an `interp eval` body creates a
        // real command in that interpreter's ordinary command table,
        // independent of the hidden set the safe-interp gate consults —
        // see `mark_locally_defined_in_enclosing_interp`'s doc comment.
        self.mark_locally_defined_in_enclosing_interp(&simple_key);

        // Walk the body in a fresh proc scope when the body is a
        // braced literal. The *resolved* name is the proc-scope name:
        // ``define_var`` keys ``result.all_variables`` on
        // ``"<scope_name>::<var>"``, and the scope name is also what
        // ``advance_command_resolution_namespace`` reads the body's namespace
        // off — see ``scope_name_for_routine``.
        if body_tok.kind == TokenType::Str {
            self.walk_proc_body_in_new_scope(ProcBodyWalkArgs {
                path: &path,
                resolved_name: &resolved_name,
                body_span,
                arg_tokens,
                name_tok,
                params: &params,
                args,
                body_tok,
                ns_prefix: &ns_prefix,
            });
        }

        true
    }

    /// Record one `proc` declaration into the whole-document tables —
    /// `all_procs` (keyed by qualified name, last redefinition winning, which
    /// is plain Tcl's own semantics), the never-deduplicated
    /// `proc_declaration_sites` span list, and, when this declaration
    /// displaces an earlier one of the same qualified name,
    /// `superseded_procs`.
    ///
    /// The displaced definition is kept because "last wins" is only true
    /// *from the second declaration onward*: a call written between the two
    /// reaches the first one, with the first one's span and parameter list.
    /// Oracle (tclsh 8.6.16 and 9.0.4): with `proc p {} {return first}`,
    /// `p` between the declarations returns `first`, and after a later
    /// `proc p {a} {…}` the same bare `p` fails `wrong # args: should be "p
    /// a"`. Dropping the first definition on the map insert made
    /// go-to-definition from the in-between call jump to the *later* header
    /// and left its arity unknowable (issue #923 idx 45).
    ///
    /// `superseded_procs` stays empty for every document without a
    /// redefinition, so the common case pays one `HashMap::insert` return
    /// value and nothing else.
    fn register_proc_definition(
        &mut self,
        qualified: &str,
        proc: &ProcDef,
        name_span: tcl_lexer::Span,
    ) {
        if let Some(previous) = self.result.all_procs.insert(qualified.into(), proc.clone()) {
            self.result
                .superseded_procs
                .entry(qualified.to_string())
                .or_default()
                .push(previous);
        }
        self.result
            .proc_declaration_sites
            .push((qualified.to_string(), name_span));
    }

    /// Walks a `proc`'s body in a freshly created child scope: binds formal
    /// parameters as locals, then recurses into the body (or, on the
    /// per-item shell pass, defers it). Split out of
    /// [`Self::handle_proc_command`] to keep that function's line count
    /// down; `ctx.path` is the *parent* scope path (before the new proc
    /// scope is pushed).
    fn walk_proc_body_in_new_scope(&mut self, ctx: ProcBodyWalkArgs<'_>) {
        let ProcBodyWalkArgs {
            path,
            resolved_name,
            body_span,
            arg_tokens,
            name_tok,
            params,
            args,
            body_tok,
            ns_prefix,
        } = ctx;
        // One scope key for both the inline and the deferred path — a
        // divergence here would make the per-item and whole-file walks
        // disagree about which namespace the body runs in.
        let scope_name = scope_name_for_routine(resolved_name);
        let proc_scope_idx = {
            let parent = super::scope::scope_at_mut(&mut self.result.global_scope, path)
                .expect("scope_path resolved when registering proc must still resolve");
            let mut child =
                super::types::Scope::new(super::types::ScopeKind::Proc, scope_name.to_string());
            child.body_span = Some(body_span);
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = path.to_vec();
        child_path.push(proc_scope_idx);

        // Parameters become locals in the proc scope. Each param's
        // definition range is anchored to its *name* in the param-list
        // literal (issue #727) so go-to-definition / references / rename on
        // a formal parameter resolve to the parameter, not the proc name.
        // The spans are recovered from the raw param-list word token
        // (`arg_tokens[1]`); any param whose name can't be located falls
        // back to the proc name token.
        let params_tok = arg_tokens[1];
        let param_spans = param_name_spans_for_token(&self.source, params_tok);
        for (i, p) in params.iter().enumerate() {
            self.define_var(
                &p.name,
                name_tok,
                &child_path,
                false,
                param_spans.get(i).copied(),
            );
        }
        self.emit_w218_args_not_final(params, &param_spans, params_tok);

        // Save / restore `last_comment` around the body walk so a
        // doc-comment inside the proc body doesn't bleed to whatever
        // follows the proc at the outer scope. Same treatment for
        // `current_event`: a `proc` body is a new call frame, so W142's
        // event-body-only bare-`return` restriction must not leak in from
        // an enclosing `when` — see `emit_w142_context_gate`'s doc comment.
        let (saved_comment, saved_event) = (
            std::mem::take(&mut self.last_comment),
            self.current_event.take(),
        );

        // Body recursion via the shared helper.  Re-segments
        // the body (no recovery — top-level only) and
        // dispatches each command at the new proc scope path.
        // Per-item shell pass: defer the body (its scope is already
        // created with params; a second pass fills it in place).
        let body_text: std::sync::Arc<str> = std::sync::Arc::from(args[2].as_str());
        if self.defer_proc_bodies {
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies.push(super::per_item::DeferredBody {
                body_text,
                body_tok,
                scope_path: child_path.clone(),
                is_method: false,
                oo_global_resolution: false,
                namespace: ns_prefix.to_string(),
                scope_name: scope_name.to_string(),
                params: params.to_vec(),
                class_variables: Vec::new(),
                // Attached later by `fill_deferred_bodies` for bodies with a
                // fold candidate (issue #1132).
                command_trust: None,
                ensemble_targets: Vec::new(),
                oo_defining_class: None,
                safe_interp_ctx,
            });
        } else {
            self.analyse_body(&body_text, body_tok, &child_path);
        }

        (self.last_comment, self.current_event) = (saved_comment, saved_event);
    }

    /// `tcl::OptProc name optlist body` — the `opt` package's automatic-
    /// option-parsing proc definer (issue #923 idx 90).
    ///
    /// At runtime this installs a REAL proc via `uplevel 1 [list ::proc
    /// $name args ...]` — the Tcl-level formal parameter is always the
    /// single literal word `args` (any call arity is accepted; `optlist`
    /// itself is never arity-checked). `optlist`'s own descriptor entries
    /// (`{child -use -display}`) share `proc`'s own `{name default}` /
    /// bare-`name` list shape, so [`parse_param_list`] applies directly —
    /// but they are bound as LOCAL VARIABLES inside the body by
    /// `::tcl::OptKeyParse`, with a leading `-` on a flag descriptor
    /// STRIPPED for the bound name (tclsh9.0/8.6-verified: `-use`/
    /// `-display` bind as `use`/`display`, never with the dash).
    ///
    /// Mirrors [`Self::handle_proc_command`]'s register/scope/walk glue
    /// largely as a separate function rather than a shared abstraction —
    /// the two definers' arity/local-binding stories diverge enough
    /// (`ProcDef.params` is `[args]` here, never `optlist`'s own entries)
    /// that factoring out a shared helper would need as many branches as
    /// duplicating the glue outright.
    pub fn handle_opt_proc_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 3 || arg_tokens.len() < 3 {
            return false;
        }

        let (raw_name, name_tok) = (&args[0], arg_tokens[0]);
        let resolved_name = self
            .resolve_dynamic_word(
                raw_name,
                Some(name_tok),
                arg_single.first().copied().unwrap_or(false),
                scope_path,
            )
            .unwrap_or_else(|| raw_name.clone());
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let qualified = qualify(&ns_prefix, &resolved_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = name_tok.span;
        let body_tok = arg_tokens[2];

        self.emit_w113_proc_shadows_builtin(&resolved_name, &qualified, name_span);
        self.emit_w314_no_absolute_name(raw_name, name_span);

        let (real_params, opt_locals) = Self::opt_proc_params(&args[1]);

        let mut doc = std::mem::take(&mut self.last_comment);
        if doc.is_empty() && args.len() >= 3 {
            doc = super::utils::extract_body_docstring(&args[2]);
        }

        // Combined list — the real `args` catch-all plus every
        // optlist-derived local — feeds hover/param-trait inference and
        // the body's own local-variable scope (never `ProcDef.params`,
        // which stays `[args]`-only for correct arity).
        let mut combined_params = real_params.clone();
        combined_params.extend(opt_locals.iter().cloned());

        let body_text = &args[2];
        let (param_traits, caller_frame_params, caller_frame_literals) =
            self.infer_proc_param_traits(&combined_params, body_text);

        let proc = ProcDef {
            name: simple,
            qualified_name: qualified.clone(),
            params: real_params,
            // `tcl::OptProc`'s Tcl-level signature is always the single
            // catch-all `args`, derived above — a known list, not a computed
            // one.
            params_computed: false,
            name_span,
            body_span: body_tok.span,
            doc,
            param_traits,
            caller_frame_params,
            caller_frame_literals,
        };

        self.register_proc_definition(&qualified, &proc, name_span);
        let simple_key = proc.name.clone();
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.procs.insert(simple_key.clone(), proc);
        }
        self.mark_locally_defined_in_enclosing_interp(&simple_key);

        if body_tok.kind == TokenType::Str {
            // The *resolved* name keys the scope — see
            // `scope_name_for_routine`; the raw written word would invent a
            // namespace out of a substitution.
            let scope_name = scope_name_for_routine(&resolved_name).to_string();
            let proc_scope_idx = {
                let parent = super::scope::scope_at_mut(&mut self.result.global_scope, &path)
                    .expect("scope_path resolved when registering proc must still resolve");
                let mut child =
                    super::types::Scope::new(super::types::ScopeKind::Proc, scope_name.clone());
                child.body_span = Some(body_tok.span);
                parent.children.push(child);
                parent.children.len() - 1
            };
            let mut child_path = path.clone();
            child_path.push(proc_scope_idx);

            // Bind the real `args` catch-all — a body reference to
            // `$args` (inspecting leftovers `::tcl::OptKeyParse` didn't
            // consume) is legitimate, exactly like an ordinary proc's own
            // `args` parameter. No literal `args` word is ever written for
            // this idiom, so — unlike an ordinary proc's own parameters,
            // each anchored to its own written span — there is no sensible
            // non-synthetic span to anchor it to: `name_tok` collides with
            // the proc *name*'s own span (`$args` hover would resolve to
            // the declaration token instead of `greet`), and the whole
            // `optlist` word collides with every one of its own descriptor
            // sub-spans (`child`'s / `-use`'s own hover would resolve to
            // `args` instead). A zero-width span at the `optlist` word's
            // own opening brace sits before any descriptor's span starts,
            // so it collides with neither.
            let params_tok = arg_tokens[1];
            let args_span = Span::new(params_tok.span.start(), params_tok.span.start());
            self.define_var("args", params_tok, &child_path, false, Some(args_span));
            // Bind every optlist-derived local, anchored to its own
            // descriptor's span (dash included — the written token, not a
            // byte-sliced substring) so go-to-definition / references /
            // rename on the parameter land on the real declaration.
            let param_spans = param_name_spans_for_token(&self.source, params_tok);
            for (i, p) in opt_locals.iter().enumerate() {
                self.define_var(
                    &p.name,
                    name_tok,
                    &child_path,
                    false,
                    param_spans.get(i).copied(),
                );
            }

            let saved_comment = std::mem::take(&mut self.last_comment);
            let body_text: std::sync::Arc<str> = std::sync::Arc::from(args[2].as_str());
            if self.defer_proc_bodies {
                let safe_interp_ctx = self.safe_interp_ctx_snapshot();
                self.deferred_bodies.push(super::per_item::DeferredBody {
                    body_text,
                    body_tok,
                    scope_path: child_path.clone(),
                    is_method: false,
                    oo_global_resolution: false,
                    namespace: ns_prefix.clone(),
                    scope_name,
                    params: combined_params,
                    class_variables: Vec::new(),
                    // Attached later by `fill_deferred_bodies` for bodies
                    // with a fold candidate (issue #1132).
                    command_trust: None,
                    ensemble_targets: Vec::new(),
                    oo_defining_class: None,
                    safe_interp_ctx,
                });
            } else {
                self.analyse_body(&body_text, body_tok, &child_path);
            }

            self.last_comment = saved_comment;
        }

        true
    }

    /// Split `tcl::OptProc`'s `optlist` argument into `(real_params,
    /// opt_locals)` (issue #923 idx 90): the real, arity-relevant Tcl-level
    /// signature — always the single catch-all `args`, regardless of what
    /// `optlist` declares — and `optlist`'s own descriptors, dash-stripped
    /// to the LOCAL VARIABLE name `::tcl::OptKeyParse` actually binds at
    /// runtime (used only for local-variable binding in the body, never
    /// for arity). A pure text transform — no analyser state needed — kept
    /// out of [`Self::handle_opt_proc_command`] purely to stay within the
    /// line-count lint.
    fn opt_proc_params(
        optlist_text: &str,
    ) -> (
        Vec<crate::signature_scan::types::ParamDef>,
        Vec<crate::signature_scan::types::ParamDef>,
    ) {
        let real_params = vec![crate::signature_scan::types::ParamDef {
            name: "args".to_string(),
            has_default: false,
            default_value: None,
        }];
        let opt_locals: Vec<crate::signature_scan::types::ParamDef> =
            parse_param_list(optlist_text)
                .into_iter()
                .map(|p| crate::signature_scan::types::ParamDef {
                    name: p
                        .name
                        .strip_prefix('-')
                        .map(str::to_string)
                        .unwrap_or(p.name),
                    has_default: p.has_default,
                    default_value: p.default_value,
                })
                .collect();
        (real_params, opt_locals)
    }

    /// Split an `apply` call's lambda-literal first argument
    /// (`{{params} body ?ns?}`) into its list elements — `(token, text)`
    /// pairs carrying absolute source spans, in declaration order (params,
    /// body, and an optional target-namespace pin).
    ///
    /// Returns `None` when there are no arguments or the first argument
    /// isn't a *braced* literal (a `$var` / `[cmd]` / quoted lambda is
    /// opaque — its element boundaries can't be split statically).  Both
    /// callers gate on [`tcl_registry::hooks::AnalyserHookId::Apply`]
    /// first. Shared by [`Self::handle_apply_command`] (body
    /// / scope walk) and the `apply` direct-call arity check
    /// ([`super::diagnostics::validity::Analyser::emit_arity_diagnostics`])
    /// so both consumers agree on exactly what counts as a
    /// statically-inspectable lambda, rather than each re-implementing the
    /// brace-literal guard and segmentation independently.
    /// Resolve a *dynamic* `apply` lambda argument — `apply $lambda …` or
    /// `apply [list {params} $body ns] …` — one hop through the constant-
    /// value lattice, to the same `Vec<(Token, String)>` shape
    /// [`Self::parse_apply_lambda_elements`] returns for a literal braced
    /// lambda (issue #923 idx 116): so `handle_apply_command`'s downstream
    /// code (`elements[0]` = params, `[1]` = body, `[2]` = namespace)
    /// needs no changes regardless of which path supplied it.
    ///
    /// `arg_tok`'s kind decides the strategy:
    /// - `Var` (`$lambda` / `$ns::lambda`): resolve the variable — a bare
    ///   name via [`Self::lookup_const_string_with_span`] (the lexical
    ///   ancestor-chain lookup), a `::`-qualified one via the namespace-
    ///   targeted [`Self::lookup_const_string_in_namespace`] (Tcl variable
    ///   qualifiers never search — exactly one namespace is consulted).
    ///   A braced-literal resolution (`{` at the resolved span's start,
    ///   the same guard `parse_apply_lambda_elements` itself applies)
    ///   delegates straight to it — no re-implementation.
    /// - `Cmd` (`[list {params} $body ns]`): the mined idiom's actual
    ///   shape — a `list`-constructor call whose own arguments *are* the
    ///   three lambda elements, positionally. Requires the inner command's
    ///   head to be literally `"list"`; each of its own arguments is kept
    ///   verbatim (already a real absolute span) unless it is itself a
    ///   `Var`, which gets exactly one more hop through the same bare/
    ///   qualified lookup (no further recursion — a deliberate depth
    ///   bound). Anything else at any position aborts the whole fold
    ///   (`None`) rather than emit a partial, misleading result.
    ///
    /// Bounded to one hop by design: `set a $lambda; apply $a` remains
    /// unresolved (a second `$var`-to-`$var` forward), as does any deeper
    /// list-element indirection — see the type's own module docs on the
    /// `apply`-namespace-override limitation for the full rationale.
    fn resolve_dynamic_apply_lambda(
        &self,
        arg_tok: Token,
        scope_path: &[usize],
    ) -> Option<Vec<(Token, String)>> {
        let one_hop = |word_tok: Token| -> Option<(Token, String)> {
            if word_tok.kind != TokenType::Var {
                let word_text = Analyser::source_slice(
                    &self.source,
                    word_tok.span.start() as usize,
                    word_tok.span.end() as usize,
                )?;
                return Some((word_tok, word_text.to_string()));
            }
            let sm = tcl_lexer::SourceMap::new(&self.source);
            let var_name = sm.token_text(word_tok);
            let (holder, base_name) = crate::naming::key_holder_and_tail(var_name);
            let (value, span) = if holder.is_empty() {
                self.lookup_const_string_with_span(var_name, scope_path)?
            } else {
                let target_ns = if holder.starts_with("::") {
                    holder.to_string()
                } else {
                    let caller_ns = self.command_resolution_namespace(scope_path);
                    crate::naming::qualify(caller_ns.trim_start_matches(':'), holder)
                };
                self.lookup_const_string_in_namespace(&target_ns, base_name)?
            };
            Some((
                Token::with_content_offset(TokenType::Str, span, 1),
                value.to_string(),
            ))
        };

        match arg_tok.kind {
            TokenType::Var => {
                let (tok, text) = one_hop(arg_tok)?;
                if text.as_bytes().first() != Some(&b'{') {
                    return None;
                }
                self.parse_apply_lambda_elements(&[text], &[tok])
            }
            TokenType::Cmd => {
                let (inner, base) = super::scope::inner_of(&self.source, arg_tok)?;
                let segmented = crate::segmenter::segment_commands_with_offset(inner, base);
                let [cmd] = segmented.as_slice() else {
                    return None;
                };
                if cmd.texts.first().map(String::as_str) != Some("list") {
                    return None;
                }
                let mut out = Vec::with_capacity(cmd.texts.len().saturating_sub(1));
                for (tok, text) in cmd.argv.iter().skip(1).zip(cmd.texts.iter().skip(1)) {
                    if tok.kind == TokenType::Var {
                        // A `$body` / `$ns` list element still needs one hop
                        // through the constant-value lattice.
                        out.push(one_hop(*tok)?);
                    } else {
                        // Use the segmenter's already-delimiter-stripped
                        // element text (`cmd.texts`) paired with its token —
                        // the same shape `parse_apply_lambda_elements` yields
                        // for a literal lambda. Slicing the token's raw source
                        // span instead would keep a `{cleanup done}` element's
                        // braces, so the body re-segments as one braced word
                        // and the real `cleanup` call is missed (Codex review,
                        // PR #1020).
                        out.push((*tok, text.clone()));
                    }
                }
                Some(out)
            }
            _ => None,
        }
    }

    pub(in crate::analyser) fn parse_apply_lambda_elements(
        &self,
        args: &[String],
        arg_tokens: &[Token],
    ) -> Option<Vec<(Token, String)>> {
        if args.is_empty() || arg_tokens.is_empty() {
            return None;
        }
        let lambda_tok = arg_tokens[0];
        // Only a *braced* literal lambda can be split and offset-mapped safely
        // (its content is verbatim source).
        let lambda_start = lambda_tok.span.start() as usize;
        if lambda_tok.kind != TokenType::Str
            || self.source.as_bytes().get(lambda_start) != Some(&b'{')
        {
            return None;
        }

        // Split the lambda literal into its list elements. Re-segmenting the
        // brace-stripped content (`args[0]`) at the lambda's absolute content
        // offset yields the elements as command words carrying absolute-span
        // tokens; flattening across commands keeps params / body paired even
        // when a multi-line lambda puts them on separate lines (a newline
        // splits *commands*, not *list elements*).
        let base = lambda_tok.span.start() + u32::from(lambda_tok.content_offset);
        let segmented = crate::segmenter::segment_commands_with_offset_and_config(
            &args[0],
            base,
            self.lexer_config(),
        );
        Some(
            segmented
                .iter()
                .flat_map(|c| c.argv.iter().copied().zip(c.texts.iter().cloned()))
                .collect(),
        )
    }

    /// Handle an `apply {{params} body ?ns?} ?arg ...?` invocation.
    ///
    /// `apply`'s first argument is a *lambda* (an anonymous procedure), **not**
    /// a plain script body: element 0 is the parameter list and element 1 is
    /// the body. The `apply` registry spec marks the argument `ArgRole::Body`,
    /// so without this handler the generic body-recursion in
    /// [`Analyser::dispatch_body_arguments`] treats the whole `{{params} body}`
    /// literal as a *script* — mis-reading the parameter list as a command
    /// (`apply {{a} {…}}` ⇒ a spurious W123 "unknown command 'a'") and never
    /// linting the real body.
    ///
    /// This models the lambda like a `proc`: the parameters bind as locals in a
    /// fresh proc scope and element 1 is walked as the body (deferred during the
    /// per-item shell pass, inline otherwise — mirroring
    /// [`Analyser::handle_proc_command`], so the incremental and full paths stay
    /// byte-identical). The body is walked in the *lambda's* namespace — element
    /// 2 of the lambda, or the global namespace `::` when absent — not the
    /// caller's, so a nested `proc` registers under the qualified name the
    /// runtime `apply` gives it (`::p`, not `::caller::p`). Returns `true` when
    /// the command was an `apply` with a
    /// statically-inspectable braced lambda literal, so the caller skips the
    /// generic body recursion. A dynamic lambda (`apply $lambda …`) returns
    /// `false` and falls through to the generic path (whose `analyse_body` is a
    /// no-op on the non-braced word), preserving its existing behaviour.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Apply`].
    pub fn handle_apply_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        // A literal braced lambda parses directly; a dynamic one (`apply
        // $lambda`, `apply [list {params} $body ns]`) gets one hop through
        // the constant-value lattice (issue #923 idx 116) before giving up
        // — same downstream shape either way, so nothing past this point
        // needs to know which path supplied `elements`.
        let elements = if let Some(elements) = self.parse_apply_lambda_elements(args, arg_tokens) {
            elements
        } else {
            let Some(&arg_tok) = arg_tokens.first() else {
                return false;
            };
            let Some(elements) = self.resolve_dynamic_apply_lambda(arg_tok, scope_path) else {
                return false;
            };
            elements
        };
        // A lambda needs at least a parameter list and a body.
        if elements.len() < 2 {
            return true;
        }
        let (params_tok, params_text) = (elements[0].0, elements[0].1.as_str());
        let (body_tok, body_text) = (elements[1].0, elements[1].1.as_str());

        // The body must itself be a braced literal to walk statically (matching
        // `analyse_body`'s own `TokenType::Str` guard); a bare / substituted
        // body is left un-analysed, identically on the inline and per-item
        // paths so the two never diverge.
        if body_tok.kind != TokenType::Str
            || self.source.as_bytes().get(body_tok.span.start() as usize) != Some(&b'{')
        {
            return true;
        }

        // `apply` runs the lambda body in the namespace named by lambda element
        // 2, or the *global* namespace when it is absent — never the caller's
        // namespace. Element 2 is interpreted relative to the **global**
        // namespace even when it does not start with `::` (`doc/apply.n`:
        // "If given, namespace is interpreted relative to the global namespace
        // even if its name does not start with ::"; `tclProc.c`
        // `TclNRApplyObjCmd` literally `::`-prefixes the word before the
        // lookup). So `apply {{} {…} sub}` homes to `::sub` no matter which
        // namespace the call sits in, and a nested `proc` registers under the
        // qualified name the runtime `apply` would give it.
        let body_ns = match elements.get(2).map(|(_, t)| t.as_str()) {
            Some(ns) if !ns.is_empty() && !ns.starts_with('$') && !ns.starts_with('[') => {
                qualify("", ns)
            }
            _ => "::".to_string(),
        };
        // That element is also a first-class *reference* to the namespace it
        // names, not merely a semantic input (issue #1113 item 4): it sits
        // inside an `ArgRole::LambdaLiteral` word, which no whole-word role
        // reaches, so the identity is recorded here, where the element has
        // already been split. Non-declaring — `apply` looks the namespace up,
        // it does not create it (tclsh 8.6.16 / 9.0.4: `apply {{} {} ::nope}`
        // fails `namespace "::nope" not found`).
        if let Some((ns_tok, ns_text)) = elements.get(2)
            && body_ns != "::"
            && !crate::naming::is_dynamic_word(ns_text)
        {
            let span = tcl_lexer::Span::new(
                ns_tok.span.start() + u32::from(ns_tok.content_offset),
                ns_tok.span.end(),
            );
            if span.start() < span.end() {
                self.result.namespace_refs.push(super::types::NamespaceRef {
                    qualified_name: body_ns.clone(),
                    span,
                    declares: false,
                });
            }
        }

        let params = parse_param_list(params_text);
        let body_text: std::sync::Arc<str> = std::sync::Arc::from(body_text);
        let body_span = body_tok.span;
        // Anonymous, but keyed by source position so two lambdas never collide
        // in `all_variables` (keyed `"<scope_name>::<var>"`).
        let scope_name = format!("apply@{}", arg_tokens[0].span.start());

        // Record the namespace override for `tcl-lsp-core`'s command-
        // resolution lookups (issue #923 idx 116): the `Scope` subtree
        // rooted below, via `reconstruct_proc_scope`, sits under fresh
        // `body_span`-less namespace wrapper nodes the ordinary lexical
        // span-containment walk can never reach — `namespace_overrides` is
        // a separate, flat, span-keyed fast path consulted ahead of that
        // walk. Pushed once regardless of the inline/deferred split below.
        self.result
            .namespace_overrides
            .push((body_span, body_ns.clone()));

        // Root the lambda scope at `body_ns` under the global scope — NOT under
        // the caller — via the same `reconstruct_proc_scope` the per-item path
        // uses, so the inline (full) and per-item (incremental) walks build
        // byte-identical structure and nested definitions resolve to `body_ns`.
        let child_path = super::per_item::reconstruct_proc_scope(
            &mut self.result.global_scope,
            &body_ns,
            &scope_name,
            super::types::ScopeKind::Proc,
        );
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &child_path)
        {
            scope.body_span = Some(body_span);
        }

        // Parameters become locals, each anchored to its name in the param-list
        // literal (issue #727) so go-to-definition / references / rename on a
        // formal resolve to the parameter, not the `apply` call.
        let param_spans = param_name_spans_for_token(&self.source, params_tok);
        for (i, p) in params.iter().enumerate() {
            self.define_var(
                &p.name,
                params_tok,
                &child_path,
                false,
                param_spans.get(i).copied(),
            );
        }
        self.emit_w218_args_not_final(&params, &param_spans, params_tok);

        // Save / restore last_comment around the body walk, as `proc` does, so a
        // doc-comment inside the lambda body doesn't bleed to what follows.
        let saved_comment = std::mem::take(&mut self.last_comment);
        if self.defer_proc_bodies {
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies.push(super::per_item::DeferredBody {
                body_text,
                body_tok,
                scope_path: child_path.clone(),
                is_method: false,
                oo_global_resolution: false,
                namespace: body_ns,
                scope_name,
                params,
                class_variables: Vec::new(),
                // Attached later by `fill_deferred_bodies` for bodies with a
                // fold candidate (issue #1132).
                command_trust: None,
                ensemble_targets: Vec::new(),
                oo_defining_class: None,
                safe_interp_ctx,
            });
        } else {
            self.analyse_body(&body_text, body_tok, &child_path);
        }
        self.last_comment = saved_comment;
        true
    }

    /// Whether a `proc` defined as `qualified` is the interpreter's *global*
    /// unresolved-command handler — the one whose presence gates W123 for the
    /// whole file.
    ///
    /// Which command is the handler comes from
    /// [`tcl_registry::Traits::UNRESOLVED_COMMAND_HANDLER`], never a literal
    /// name here — the same registry query `tcl_compiler::unit_scope` uses for
    /// the interprocedural call-site seed.
    ///
    /// **Global only.** Tcl consults `::unknown` for a bare unresolved word
    /// regardless of the calling namespace, so a namespace-local
    /// `proc unknown` inside `namespace eval ::mylib { … }` is an ordinary
    /// proc that happens to share the name and must not suppress anything.
    /// tclsh8.6.14 and tclsh9.0.4 both confirm the split: with a global
    /// `proc unknown {args} {return handled}` a call to
    /// `totallyBogusCommand` returns `handled`, while with the same proc
    /// defined inside `namespace eval ::mylib` it still fails
    /// `invalid command name "totallyBogusCommand"`. (`namespace unknown
    /// NAME` registers a per-namespace handler explicitly; that path is
    /// modelled separately by its handler argument's
    /// [`tcl_registry::ArgRole::CommandPrefix`] role.)
    ///
    /// `::tcl::unknown` is admitted alongside `::unknown`: it is the Tcl
    /// library's own handler spelling, installed as the interpreter-wide
    /// handler rather than scoped to callers inside `::tcl`.
    fn defines_global_unresolved_handler(&self, qualified: &str) -> bool {
        let Some(registry) = self.registry.as_deref() else {
            return false;
        };
        let mut carriers =
            registry.commands_with_trait(tcl_registry::Traits::UNRESOLVED_COMMAND_HANDLER);
        carriers.sort_unstable();
        carriers.iter().any(|name| {
            qualified == crate::naming::qualify("::", name)
                || qualified == crate::naming::qualify("::tcl", name)
        })
    }

    /// Handle `namespace eval`: opens a new namespace scope and
    /// schedules its body for analysis.
    ///
    /// Returns `true` when the command was handled.
    ///
    /// Creates the child namespace scope so downstream handlers see
    /// qualified names resolve through it, then recurses into the
    /// body.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceEval`] (stamped
    /// on `namespace`'s `eval` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_namespace_eval_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 2 {
            return false;
        }
        let ns_name = args[1].clone();
        // **W314** — an all-colon segment (`namespace eval :`) creates a
        // namespace no absolute path can spell (#934).
        if let Some(ns_tok) = arg_tokens.get(1) {
            self.emit_w314_unaddressable_namespace(&ns_name, ns_tok.span);
        }
        // The scope's `body_span` is what offset-keyed lookups resolve
        // *against* the namespace frame, so it may only cover text that runs
        // in that frame.  A literal `{…}` block does.  A `[…]` substitution
        // does **not**: Tcl evaluates it in the *calling* frame and hands
        // `namespace eval` the resulting value, so `namespace eval ::
        // [list source [file join $::tk_library $file.tcl]]` (Tk's
        // `::tk::SourceLibFile`) reads the proc's own `$file` parameter
        // before the namespace is entered at all.  Claiming those bytes for
        // the namespace made the scope-chain lookup stop there and answer
        // nothing for that read (issue #1138 idx 102).
        let body_tok = arg_tokens.get(2).copied();
        let body_span = body_tok
            .filter(|t| t.kind == TokenType::Str)
            .map(|t| t.span);
        let body_text = args.get(2).cloned();
        // `namespace eval $ns [list namespace unknown $handler]` — the
        // list-wrapped installer idiom `analyse_body`'s literal-`{...}`
        // -only gate below never sees (issue #923 idx 110).
        if let Some(tok) = body_tok {
            self.detect_list_wrapped_namespace_unknown(tok);
        }

        // A dynamic target (`namespace eval $name { … }`, the irc.tcl
        // per-connection idiom) can't be resolved to a real namespace path —
        // and, unlike a literal one, two lexically unrelated occurrences
        // that happen to write the *same* variable name (an unremarkable
        // choice for this exact idiom) must never collapse into one scope:
        // each occurrence gets its own synthetic, per-call-site domain,
        // keyed by this argument token's own source offset (unique per
        // occurrence, deterministic, and — like `@interp@` — unrepresentable
        // in real Tcl, so it can never collide with a literal namespace of
        // the same written text). Mirrors `interp eval`'s dynamic-path
        // handling a few hooks below, which is conservatively isolated the
        // same way rather than merged by raw text.
        //
        // A computed target whose value is **constant-dominated** is not
        // dynamic in any way that matters: `set ns ::app; namespace eval $ns
        // { … }` creates `::app` on every run, so the block's procs really do
        // home to `::app::…` and the block really is a declaring site for
        // `::app`. That case is settled through the same identity-resolution
        // helper the command head (issue #923 idx 44), `source`, `rename`, and
        // `oo::define`'s target word already use — one lattice answers "what
        // does this word name" everywhere, and its dominance requirement keeps
        // a branch-conditional binding out (issue #1113 item 3).
        let resolved_dynamic = crate::naming::is_dynamic_word(&ns_name)
            .then(|| {
                self.resolve_dynamic_word(
                    &ns_name,
                    arg_tokens.get(1).copied(),
                    arg_single.get(1).copied().unwrap_or(false),
                    scope_path,
                )
            })
            .flatten()
            .filter(|resolved| !resolved.is_empty() && !crate::naming::is_dynamic_word(resolved));
        if let (Some(resolved), Some(tok)) = (resolved_dynamic.as_deref(), arg_tokens.get(1)) {
            // The word is also the block's declaring occurrence of that
            // namespace, which is what lets navigation reach it from a
            // `namespace children ::app` elsewhere — the whole-word role scan
            // in `record_namespace_name_refs` skips dynamic words, so the
            // identity is recorded here, where it has just been settled.
            let here = self.command_resolution_namespace(scope_path);
            self.result.namespace_refs.push(super::types::NamespaceRef {
                qualified_name: crate::naming::qualify(&here, resolved),
                span: tok.span,
                declares: true,
            });
        }
        let scope_name = match resolved_dynamic {
            Some(resolved) => resolved,
            None if crate::naming::is_dynamic_word(&ns_name) => match arg_tokens.get(1) {
                Some(tok) => self.mint_synthetic_offset_name("@dynns@", tok.span.start()),
                None => ns_name.clone(),
            },
            None => ns_name,
        };

        let path = scope_path.to_vec();
        let child_scope_idx = {
            let mut child =
                super::types::Scope::new(super::types::ScopeKind::Namespace, scope_name);
            child.body_span = body_span;
            // The written `NAME` word, so the outline can point its
            // `selectionRange` at the name rather than the whole body
            // (issue #1218).  Recorded even for a dynamic `$ns` target: the
            // word is still where the user would want the cursor.
            child.name_span = arg_tokens.get(1).map(|t| t.span);
            let Some(parent) = super::scope::scope_at_mut(&mut self.result.global_scope, &path)
            else {
                return false;
            };
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = path;
        child_path.push(child_scope_idx);

        // Body recursion lets procs and classes declared inside
        // ``namespace eval`` register with the correct namespace
        // prefix.  Words past the body join into the script exactly as
        // `eval`'s do, so a multi-word call is analysed as the whole
        // concatenation or not at all (issue #1051) — never as its first
        // word alone, which invented an E002 on the trailing words'
        // arguments and lost every write they performed.
        if args.len() > 3 {
            self.analyse_namespace_eval_tail(args, arg_tokens, &child_path);
        } else if let (Some(text), Some(tok)) = (body_text, body_tok) {
            self.analyse_body(&text, tok, &child_path);
        }
        true
    }

    /// Analyse the script a multi-word `namespace eval` / `namespace inscope`
    /// evaluates, in the namespace scope `child_path` already opened for it.
    ///
    /// The two subcommands share this hook but not their tail semantics, and
    /// the registry records the split:
    ///
    /// - `namespace eval ns arg ?arg …?` carries
    ///   [`tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS`] alone — the tail
    ///   space-joins into the script (`namespace eval ::n set l2 hello` sets
    ///   `::n::l2`, tclsh8.6.14/9.0.4-confirmed), so a fully-static tail is
    ///   joined and walked.
    /// - `namespace inscope ns script ?arg …?` adds
    ///   [`tcl_registry::Traits::SCRIPT_APPENDS_LIST_ARGS`] — the tail is
    ///   appended as *list elements*, so `namespace inscope :: {puts} {a b}`
    ///   prints `a b` where `namespace eval :: {puts} {a b}` errors. A join
    ///   would be simply wrong, and reconstructing the list quoting is beyond
    ///   what the analyser models, so any trailing word means the call is
    ///   consumed without walking.
    ///
    /// A dynamic word takes the same consume-without-walk route as
    /// [`Self::handle_interp_eval_command`]'s multi-word arm: substitution
    /// runs before concatenation, so the real script is unknowable.
    fn analyse_namespace_eval_tail(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        child_path: &[usize],
    ) {
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let list_append = registry
            .get("namespace")
            .and_then(|spec| spec.subcommand(&args[0]))
            .is_some_and(|sub| {
                sub.traits
                    .contains(tcl_registry::Traits::SCRIPT_APPENDS_LIST_ARGS)
            });
        if list_append {
            return;
        }
        let (Some(words), Some(tokens)) = (args.get(2..), arg_tokens.get(2..)) else {
            return;
        };
        let Some(first_tok) = tokens.first() else {
            return;
        };
        let Some((script, span)) = super::utils::concat_script_window(words, tokens, &self.source)
        else {
            // The tail cannot be walked (a dynamic word). A braced first word
            // is still a literal script prefix — concatenation appends after
            // it — and it is the namespace's whole visible body in the common
            // mangled-document case (an unbalanced brace inside the body word
            // drags trailing text into extra words). Walk it rather than
            // discarding every proc and variable the namespace declares.
            if first_tok.kind == TokenType::Str {
                self.analyse_body(&words[0], *first_tok, child_path);
            }
            return;
        };
        let anchor = Token::new(TokenType::Str, span);
        self.analyse_body(&script, anchor, child_path);
    }

    /// Handle `interp eval PATH SCRIPT`: the script runs in a **child**
    /// interpreter — a separate command / variable space — so its `proc` /
    /// `oo::class` / variable definitions and calls must not merge into the
    /// parent namespace (a parent `rename foo` must not rewrite a child `proc
    /// foo`; [`tcl_vm::interp`] isolates the child at run time).
    ///
    /// Only the single-script form (`interp eval child { … }`) is isolated: an
    /// **empty** path (`interp eval {} script`) targets the *current*
    /// interpreter — its definitions belong here — and a multi-word /
    /// non-literal script cannot be statically re-assembled, so both fall back
    /// to the generic body recursion by returning `false`.  Otherwise an
    /// isolated child scope is opened, named for the interpreter path so the
    /// child's definitions home under it (`::<path>::foo`) and the child's own
    /// calls still resolve within the block; a dynamic path (`$i`) can't
    /// collide with a real namespace, so it stays conservatively isolated too.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpEval`]
    /// (stamped on `interp`'s `eval` subcommand); `args[0]` is the subcommand
    /// word.
    pub fn handle_interp_eval_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        // `interp eval PATH arg ?arg ...?` — the subcommand word, the path,
        // and one or more script words (C concatenates them).
        if args.len() < 3 {
            return false;
        }
        // An empty path runs in the current interpreter — not isolated.
        if args[1].is_empty() {
            return false;
        }
        // The interpreter path is a *list* relative to the current
        // interpreter — inside a child's eval body it is relative to that
        // child — so qualify against the walk's interpreter-path stack. A
        // dynamic path also resolves through a tracked `set VAR [interp
        // create ...]` binding (issue #923 idx 9) — that key is already
        // fully qualified at bind time, so it's used as-is, not
        // requalified.
        let literal_path = !crate::naming::is_dynamic_word(&args[1]);
        let resolved_dynamic = (!literal_path)
            .then(|| self.resolve_dynamic_interp_path(&args[1], scope_path))
            .flatten();
        let key = resolved_dynamic
            .clone()
            .unwrap_or_else(|| self.qualified_interp_key(&args[1]));
        if literal_path || resolved_dynamic.is_some() {
            // Interpreter existence (issue #945 fault 8): evaluating into a
            // known-but-never-created child raises `could not find
            // interpreter` at run time.  Abstains when any interp operation
            // in the file used a dynamic, *unresolvable* path (existence
            // then unknowable).
            if !self.interpreters.contains_key(&key)
                && !self.dynamic_interp_ops
                && let Some(tok) = arg_tokens.get(1)
            {
                self.result.diagnostics.push(super::types::Diagnostic {
                    code: tcl_core_types::DiagCode::W140,
                    span: tok.span,
                    message: format!(
                        "interpreter '{}' is never created in this file — \
                         `interp eval` will raise `could not find interpreter`",
                        args[1]
                    ),
                    severity: super::types::Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
        // Multiple script words concatenate at run time (`Tcl_ConcatObj`),
        // so commands can span word boundaries — no per-word walk is sound.
        // Consume the command *without* walking, keeping the parent domain
        // clean (the old fall-through analysed the words in the parent
        // scope — issue #945 fault 8); W312 separately flags the shape.
        if args.len() > 3 {
            return true;
        }
        let Some(body_tok) = arg_tokens.get(2).copied() else {
            return true;
        };
        // A path we could not resolve still gets its own domain (two
        // `interp eval $a {…}` / `interp eval $b {…}` bodies must not
        // merge), but it is flagged unresolved so per-interpreter state
        // widens rather than asserting this is a *different* interpreter
        // from every other — `$path` could hold any name, including one
        // already used literally elsewhere in the file.
        let resolved = literal_path || resolved_dynamic.is_some();
        self.isolate_interp_eval_body(&key, resolved, &args[2], body_tok, scope_path);
        true
    }

    /// Isolate `body_text` — already known to belong to interpreter `key` —
    /// into a synthetic `@interp@<key>` child scope, so its `proc` /
    /// `oo::class` / variable definitions and calls don't merge into the
    /// parent namespace. Shared core of [`Self::handle_interp_eval_command`]
    /// (literal `interp eval PATH { … }`) and
    /// [`Self::handle_interp_handle_eval_command`] (`NAME eval { … }` via the
    /// interpreter's own object command) — both recognise the *same*
    /// isolated-body shape, just via different call-site spellings, so both
    /// must isolate it identically rather than maintaining two copies of
    /// this logic that could drift apart.
    ///
    /// `path_resolved` says whether `key` names an interpreter the walk could
    /// identify statically (a literal path, or a dynamic one resolved through
    /// a tracked `set VAR [interp create …]` binding).  It is recorded on the
    /// pushed [`InterpFrame`](super::state::InterpFrame) so consumers of the
    /// domain identity — analyser state that models *per-interpreter runtime
    /// state*, such as the Tk widget/geometry hierarchy (issue #1141) — can
    /// widen conservatively for a body whose interpreter is unknowable
    /// instead of treating it as provably distinct from every other domain.
    ///
    /// Returns `false` (never isolated) only when the scope-tree path has
    /// gone stale — shouldn't happen during a healthy walk.
    fn isolate_interp_eval_body(
        &mut self,
        key: &str,
        path_resolved: bool,
        body_text: &str,
        body_tok: Token,
        scope_path: &[usize],
    ) -> bool {
        let outer = scope_path.to_vec();
        let domain = self.interp_domain_name(key);
        let child_scope_idx = {
            // The child's global namespace is its own domain, not a parent
            // namespace — home the scope under a synthetic
            // `@interp@<path>` name (unrepresentable in Tcl, mirroring
            // `@objdefine@`) so a real parent namespace of the same name
            // can never collide.  Repeated evals into the same live
            // interpreter share one domain name (their definitions
            // accumulate, as in C); the name carries the path's deletion
            // *epoch*, so a deleted-and-recreated interpreter is a fresh
            // domain that never merges with its predecessor's definitions
            // (issue #945 fault 8's temporal identity).
            let mut child =
                super::types::Scope::new(super::types::ScopeKind::Namespace, domain.clone());
            child.body_span = Some(body_tok.span);
            let Some(parent) = super::scope::scope_at_mut(&mut self.result.global_scope, &outer)
            else {
                return false;
            };
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = outer;
        child_path.push(child_scope_idx);

        // A safe target interpreter evaluates the body with the unsafe
        // command set hidden, and any interpreter with explicit `interp
        // hide`s carries those deltas (issue #945 fault 7): push the
        // visibility context so the per-command gate flags hidden calls
        // (W129) and builds no effects from them.  A tainted state (dynamic
        // hide/expose) abstains.
        let safe_ctx = self.interpreters.get(key).and_then(|st| {
            (!st.tainted && (st.safe || !st.hidden.is_empty())).then(|| {
                super::state::SafeInterpCtx {
                    base_hidden: st.safe,
                    hidden_extra: st.hidden.clone(),
                    exposed: st.exposed.clone(),
                }
            })
        });
        let pushed = if let Some(ctx) = safe_ctx {
            self.safe_interp_stack.push(ctx);
            true
        } else {
            false
        };
        // `interp` operations inside the body name paths relative to *this*
        // child — push its path so they qualify correctly.  The frame also
        // carries the body's domain identity for per-interpreter state; an
        // unresolvable path anywhere in the enclosing chain makes the whole
        // frame unresolved, since a body nested inside an unknowable
        // interpreter is itself in an unknowable one.
        let enclosing_resolved = self.interp_path_stack.last().is_none_or(|f| f.resolved);
        self.interp_path_stack.push(super::state::InterpFrame {
            key: key.to_string(),
            domain,
            resolved: path_resolved && enclosing_resolved,
        });
        self.analyse_body(body_text, body_tok, &child_path);
        self.interp_path_stack.pop();
        if pushed {
            self.safe_interp_stack.pop();
        }
        true
    }

    /// Handle `NAME eval SCRIPT` — a far more common real-world spelling of
    /// `interp eval NAME SCRIPT` than the `interp eval` form itself: `interp
    /// create` binds the child interpreter's *own* object command to its
    /// create-time name (`interp create sandbox` makes `sandbox` itself
    /// callable; `sandbox eval { … }` dispatches exactly like `interp eval
    /// sandbox { … }`). Without this, the body is neither isolated nor
    /// walked at all: no symbols are produced for procs defined inside it,
    /// and hover/go-to-definition on a call inside falls back to a
    /// scope-blind, file-wide "any proc anywhere with this bare name" match
    /// — which can resolve to a same-named proc in a completely unrelated
    /// interpreter.
    ///
    /// Recognised purely from analysis state — `cmd_name` is looked up
    /// against [`Self::interpreters`] (built by
    /// [`Self::handle_interp_create_command`]), never matched against a
    /// hardcoded name — so an ordinary proc that happens to be named `eval`
    /// as its first argument (`foo eval bar`, `foo` an unrelated command) is
    /// untouched. Only the single-script form is isolated, mirroring
    /// [`Self::handle_interp_eval_command`]; every other shape (no `eval`
    /// first word, wrong arity, an untracked head) falls through to the
    /// generic per-command dispatch by returning `false`.
    ///
    /// Handles the mined idiom's `$handle eval { … }` spelling too (issue
    /// #923 idx 9): `cmd_name` is the literal, unsubstituted head text, so
    /// a `$`-prefixed handle only resolves through a tracked `set VAR
    /// [interp create ...]` binding
    /// ([`Self::resolve_dynamic_interp_path`]) — a handle sourced any
    /// other way (a proc parameter, a value read from elsewhere) still
    /// falls through untouched, the same conservative fallback this
    /// handler has always used for an untracked head.
    ///
    /// Dispatched from [`Self::dispatch_analyser_hook`]'s hookless fallback
    /// chain — `cmd_name` never matches a registry command (it's a
    /// user-chosen interpreter name), so this can't be reached via a
    /// registry hook the way the literal `interp eval` form is.
    pub fn handle_interp_handle_eval_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() != 2 || args[0] != "eval" {
            return false;
        }
        let key = if crate::naming::is_dynamic_word(cmd_name) {
            let Some(resolved) = self.resolve_dynamic_interp_path(cmd_name, scope_path) else {
                return false;
            };
            resolved
        } else {
            self.qualified_interp_key(cmd_name)
        };
        if !self.interpreters.contains_key(&key) {
            return false;
        }
        let Some(body_tok) = arg_tokens.get(1).copied() else {
            return false;
        };
        // Always a resolved target: a literal head is the interpreter's own
        // object command, and a `$handle` head only reaches here through a
        // tracked `set VAR [interp create …]` binding (both arms above
        // return `false` otherwise).
        self.isolate_interp_eval_body(&key, true, &args[1], body_tok, scope_path)
    }

    /// Qualify a literal `interp` path operand against the enclosing
    /// `interp eval` bodies on the walk stack: paths are relative to the
    /// *current* interpreter, so `interp create t` inside
    /// `interp eval s {…}` names `s t`.  A dynamic operand is returned
    /// key-normalised only (its stack context cannot make it literal).
    fn qualified_interp_key(&self, path: &str) -> String {
        let local = interp_path_key(path);
        match self.interp_path_stack.last() {
            Some(enclosing) if !local.is_empty() => format!("{} {local}", enclosing.key),
            _ => local,
        }
    }

    /// The `::`-rooted prefix that qualifies a name into the interpreter
    /// domain the walk is currently inside: empty at the top level (the main
    /// interpreter), `::@interp@<path>[#<epoch>]` inside a child's eval body.
    ///
    /// Concatenate it with an already-`::`-rooted name (`::foo` →
    /// `::@interp@c::foo`) or use it as the namespace half of a join. It is
    /// what command-table facts (`interp alias`, alias deletion) written with
    /// an *empty* interpreter path must be keyed by: an empty path means "the
    /// interpreter this command runs in", which is the child inside a
    /// `child eval { … }` body, not the main interpreter.
    pub(super) fn current_interp_domain_prefix(&self) -> String {
        self.interp_path_stack
            .last()
            .map_or_else(String::new, |frame| format!("::{}", frame.domain))
    }

    /// The synthetic namespace name for an interpreter domain: the
    /// current epoch of `key` is folded in so a deleted-and-recreated
    /// interpreter never shares its predecessor's definitions.
    fn interp_domain_name(&self, key: &str) -> String {
        match self.interp_epochs.get(key) {
            Some(epoch) if *epoch > 0 => format!("@interp@{key}#{epoch}"),
            _ => format!("@interp@{key}"),
        }
    }

    /// Resolve a `$name`/`${name}` word to the interpreter-domain key a
    /// tracked `set name [interp create ...]` bound it to (issue #923
    /// idx 9) — the key is already fully resolved/qualified at bind
    /// time, so callers must NOT re-run [`Self::qualified_interp_key`]/
    /// interp-path-stack qualification on it.
    ///
    /// `None` for anything that isn't a plain scalar reference to a
    /// tracked binding: [`crate::naming::split_array_name`] rejects an
    /// array-indexed read (`$arr(idx)`) by returning an index, and a
    /// concatenated/substituted word (`prefix$s`, `[cmd]`) is never a
    /// key any `set` binds — [`Self::lookup_interp_var_binding`] simply
    /// finds no entry for it, the same "fails closed" shape as looking
    /// up an unknown name in `const_strings`.
    fn resolve_dynamic_interp_path(&self, word: &str, scope_path: &[usize]) -> Option<String> {
        let (base, index) = crate::naming::split_array_name(word);
        if index.is_some() {
            return None;
        }
        self.lookup_interp_var_binding(base, scope_path)
            .map(str::to_string)
    }

    /// Resolve an `interp alias` path operand to the domain prefix its
    /// alias/target command name qualifies under (issue #923 idx 9): the
    /// plain-current-interpreter sentinel (`""`/`"{}"`) is the empty
    /// prefix; a literal word resolves via [`Self::qualified_interp_key`]
    /// and then [`Self::interp_domain_name`]; a dynamic word resolves
    /// ONLY through [`Self::resolve_dynamic_interp_path`] — anything else
    /// (an untracked dynamic word) returns `None`, aborting the whole
    /// cross-domain alias (unchanged conservative behaviour).
    fn resolve_alias_domain_prefix(&self, path: &str, scope_path: &[usize]) -> Option<String> {
        if matches!(path, "" | "{}") {
            // The empty path is *this* interpreter — which is the child when
            // the command is written inside a child's eval body, not always
            // the main one.
            return Some(self.current_interp_domain_prefix());
        }
        let key = if crate::naming::is_dynamic_word(path) {
            self.resolve_dynamic_interp_path(path, scope_path)?
        } else {
            self.qualified_interp_key(path)
        };
        Some(format!("::{}", self.interp_domain_name(&key)))
    }

    /// Handle `interp create ?-safe? ?--? ?path?` — record the child
    /// interpreter's existence and safe state in the interpreter-domain
    /// map (issue #945 faults 7–8).  A dynamic path makes interpreter
    /// existence unknowable file-wide; a missing path auto-generates a
    /// name this file cannot reference literally, so nothing is recorded.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpCreate`];
    /// `args[0]` is the subcommand word.
    pub fn handle_interp_create_command(&mut self, args: &[String]) {
        let words: Vec<&str> = args[1..].iter().map(String::as_str).collect();
        let (safe, path) = parse_interp_create_words(&words);
        let Some(path) = path else { return };
        if crate::naming::is_dynamic_word(path) {
            self.dynamic_interp_ops = true;
            return;
        }
        let state = super::state::InterpState {
            safe,
            ..Default::default()
        };
        let key = self.qualified_interp_key(path);
        self.interpreters.insert(key, state);
    }

    /// Handle `interp delete ?path ...?` — remove the recorded state and
    /// bump the path's **epoch**, so a later re-creation is a fresh
    /// domain (issue #945 fault 8's temporal identity).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpDelete`].
    pub fn handle_interp_delete_command(&mut self, args: &[String]) {
        for word in &args[1..] {
            if crate::naming::is_dynamic_word(word) {
                self.dynamic_interp_ops = true;
                continue;
            }
            let key = self.qualified_interp_key(word);
            if self.interpreters.remove(&key).is_some() {
                *self.interp_epochs.entry(key).or_insert(0) += 1;
            }
        }
    }

    /// Handle `interp hide path cmdName ?hiddenName?` — mark the command
    /// hidden in the target interpreter's domain.  The optional third word
    /// only renames the hidden-table entry for `invokehidden` — it doesn't
    /// change `cmdName`'s ordinary-lookup visibility, which is all this
    /// gate tracks — so it's not read here.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpHide`].
    pub fn handle_interp_hide_command(&mut self, args: &[String]) {
        self.apply_interp_visibility_delta(args, true);
    }

    /// Handle `interp expose path hiddenName ?exposedName?` — re-expose a
    /// hidden command in the target interpreter's domain, visible under
    /// `exposedName` when given (`hiddenName` itself stays absent from
    /// ordinary lookup — tclsh 9.0.4-verified).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpExpose`].
    pub fn handle_interp_expose_command(&mut self, args: &[String]) {
        self.apply_interp_visibility_delta(args, false);
    }

    /// Shared body of the `interp hide` / `interp expose` handlers: apply
    /// the visibility delta for a literal `(path, command)` pair; a dynamic
    /// operand taints the interpreter's state (its visible set becomes
    /// unknowable, so the safe-context gate abstains for it).
    fn apply_interp_visibility_delta(&mut self, args: &[String], hide: bool) {
        let (Some(path), Some(cmd)) = (args.get(1), args.get(2)) else {
            return;
        };
        if crate::naming::is_dynamic_word(path) {
            self.dynamic_interp_ops = true;
            return;
        }
        let key = self.qualified_interp_key(path);
        let Some(state) = self.interpreters.get_mut(&key) else {
            return;
        };
        if crate::naming::is_dynamic_word(cmd) {
            state.tainted = true;
            return;
        }
        if hide {
            state.hidden.insert(cmd.clone());
            state.exposed.remove(cmd);
            return;
        }
        // `interp expose path hiddenName ?exposedName?` restores the
        // hidden command under `exposedName` when given — `hiddenName`
        // itself stays unavailable to ordinary lookup (tclsh 9.0.4:
        // `interp expose s source src` leaves `source` absent and makes
        // `src` callable).  A dynamic `exposedName` makes the resulting
        // visible name unknowable, so it taints like a dynamic `cmd`
        // would.
        let exposed_name = match args.get(3) {
            Some(name) if crate::naming::is_dynamic_word(name) => {
                state.tainted = true;
                return;
            }
            Some(name) => name.clone(),
            None => cmd.clone(),
        };
        state.exposed.insert(exposed_name);
        state.hidden.remove(cmd);
    }

    /// Record that `bare_name` is now a real, locally-defined command in
    /// the interpreter body currently being walked (a `proc` (re)definition
    /// — issue #945 fault 7 follow-up).  C creates it in the ordinary
    /// command table, a separate table from the hidden set entirely, so it
    /// is callable independent of any hide the base safe set or an earlier
    /// `interp hide` applied (tclsh 9.0.4-verified).  Updates both the
    /// interpreter's persistent state (so a *later*, separate `interp eval`
    /// into the same path also sees it) and the live gate context on top of
    /// the walk stack (so a call later in *this same* body sees it too — the
    /// stack entry is a snapshot taken before the body walk began).  A
    /// no-op outside any interpreter body.
    pub(super) fn mark_locally_defined_in_enclosing_interp(&mut self, bare_name: &str) {
        let Some(frame) = self.interp_path_stack.last() else {
            return;
        };
        if let Some(state) = self.interpreters.get_mut(&frame.key) {
            state.exposed.insert(bare_name.to_string());
        }
        if let Some(ctx) = self.safe_interp_stack.last_mut() {
            ctx.exposed.insert(bare_name.to_string());
        }
    }

    /// Handle `namespace path {…}`: record the current namespace's
    /// command-resolution search path for the post-walk settlement
    /// ([`Self::finalise_invocation_resolutions`]).
    ///
    /// Only the two-word set form
    /// with a *literal* list is recorded — the one-word query form mutates
    /// nothing, and a dynamic list (`$var` / `[cmd]`) is statically
    /// unknowable, so it keeps the conservative empty path. Each
    /// declaration replaces the namespace's whole path, as in C Tcl, so
    /// the lexically-last one wins (settlement is call-time / whole-file,
    /// like the rest of the resolution model).
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespacePath`] (stamped
    /// on `namespace`'s `path` subcommand); `args[0]` is still the
    /// subcommand word.
    ///
    /// The argument is a *list* of namespace names inside one word, so each
    /// element is additionally recorded as a
    /// [`NamespaceRef`](super::types::NamespaceRef) at its own source span
    /// (issue #1113 item 2). Whole-word `ArgRole`s cannot express that — the
    /// word as a whole is not one namespace name — so the split happens here,
    /// where the semantics are already modelled, using the shared Tcl list
    /// grammar rather than a second scanner. That is also what makes a braced
    /// element (`namespace path {{my ns} ::b}`) come out right, which the
    /// previous `split_whitespace` did not.
    pub fn handle_namespace_path_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() != 2 {
            return;
        }
        // The word arrives with its braces already stripped, so a text-only
        // dynamism scan mistakes `namespace path {$ns ::a}` for a computed
        // path and abstains from the whole command. Braces suppress
        // substitution, so the token kind is the authority (issue #1245).
        // tclsh-proof: tclsh8.6.14 —
        //   set nn {::$ns}; namespace eval $nn {}; namespace eval a {}
        //   namespace eval n { namespace path {::$ns ::a} }
        //   namespace eval n { namespace path }   ->  {::$ns} ::a
        // i.e. the entry is a namespace literally *named* `::$ns`; without
        // that namespace existing the same line errors with
        // `namespace "$ns" not found in "::n"` — never a variable read.
        let braced = arg_tokens
            .get(1)
            .is_some_and(|tok| tok.kind == TokenType::Str);
        if tcl_syntax::naming::word_is_dynamic(&args[1], braced) {
            // Record the abstention rather than merely staying silent: a
            // consumer that has to be exhaustive about namespace occurrences
            // (the namespace rename tier) cannot otherwise tell this document
            // from one that declares no path at all (issue #1261).
            if let Some(tok) = arg_tokens.get(1) {
                self.result.namespace_path_computed.push(tok.span);
            }
            return;
        }
        // The `namespace path` command-resolution tier is an 8.5 addition
        // (`NamespacePathCmd`, tclNamesp.c); 8.4 has no path tier, so a bare
        // call there never reaches a path namespace.  Recording the path under
        // a pre-8.5 dialect would make command resolution / definition / hover
        // falsely settle a call onto a path entry the runtime never consults —
        // so skip it, matching the `namespace path` subcommand's own dialect
        // gate (which already flags the command W002 there).
        if !self
            .profile
            .availability_mask
            .intersects(tcl_dialect::DialectSet::TCL85_PLUS)
        {
            return;
        }
        let ns = self.command_resolution_namespace(scope_path);
        let Ok(elements) = tcl_syntax::list::split_list(&args[1]) else {
            // A malformed list (an unbalanced brace) is not a path Tcl would
            // accept either; record nothing rather than half of one.
            return;
        };
        let entries: Vec<String> = elements.iter().map(ToString::to_string).collect();
        self.record_namespace_path_element_refs(&args[1], arg_tokens.get(1), braced, &ns);
        self.namespace_paths.insert(ns, entries);
    }

    /// Record one [`NamespaceRef`](super::types::NamespaceRef) per element of
    /// a `namespace path {…}` list, at the element's own source span.
    ///
    /// Non-declaring: a path entry *refers* to a namespace, it does not
    /// create one (tclsh 8.6.16 / 9.0.4: `namespace path ::nope` fails
    /// `namespace "::nope" not found`, so the name must already exist).
    /// Rooting follows the ordinary relative rule — a relative entry is
    /// current-namespace-relative only, which is what
    /// `command_resolution_candidates` already assumes.
    ///
    /// Silently does nothing when the word is not a braced/bare literal the
    /// span arithmetic can trust (`token` absent, or the element offsets fall
    /// outside it).
    ///
    /// `word_braced` says whether the *whole* path word was brace-quoted. If
    /// it was, no element substitutes however many `$`/`[` characters it
    /// holds, so the per-element dynamism scan must be suppressed exactly as
    /// the whole-word one already is (issue #1252) — otherwise `namespace
    /// path {::$ns ::a}` records `::$ns` as a path entry but not as a
    /// reference, and the same function disagrees with itself.
    fn record_namespace_path_element_refs(
        &mut self,
        raw: &str,
        token: Option<&Token>,
        word_braced: bool,
        here: &str,
    ) {
        let Some(token) = token else { return };
        let base = token.span.start() + u32::from(token.content_offset);
        let mut scan = 0usize;
        while let Ok(Some(el)) = tcl_syntax::list::find_element(raw, scan) {
            let (start, end) = (el.value.start, el.value.end);
            if el.next <= scan {
                break;
            }
            scan = el.next;
            let Some(text) = raw.get(start..end) else {
                continue;
            };
            if text.is_empty() || tcl_syntax::naming::word_is_dynamic(text, word_braced) {
                continue;
            }
            let Ok(start_u32) = u32::try_from(start) else {
                continue;
            };
            let Ok(end_u32) = u32::try_from(end) else {
                continue;
            };
            self.result.namespace_refs.push(super::types::NamespaceRef {
                qualified_name: crate::naming::qualify(here, text),
                span: tcl_lexer::Span::new(base + start_u32, base + end_u32),
                declares: false,
            });
        }
    }

    /// Handle `uplevel #0 { body }`: the script runs in the global
    /// frame, so the body's locals belong to a global-rooted scope
    /// rather than the enclosing proc.  Open an [`ScopeKind::Uplevel`]
    /// child scope (nested under the current scope, but tagged so
    /// completion / definition treat it as a global frame and ignore the
    /// proc's locals) and analyse the body there.
    ///
    /// Returns `true` for every single-braced-body `uplevel` form; a
    /// multi-word / non-literal script is left to the generic recursion (the
    /// W301 injection check already flags that shape).
    ///
    /// The frame the body runs in depends on the level word: `#0` is the global
    /// frame; `N` / `#N` / an implicit level is a *caller* frame that is
    /// statically unknown.  Either way the body's locals do **not** belong to
    /// the enclosing proc, so both open an [`ScopeKind::Uplevel`] child scope
    /// (tagged with the level word so variable resolution can tell global-frame
    /// from unknown-frame: `#0` resolves outward to the global namespace, a
    /// non-`#0` level abstains — the true frame can't be named).  Without the
    /// child scope the body's variables merged into the enclosing proc's
    /// locals, silently unifying two variables in different frames.
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Uplevel`];
    /// the level word is a shape check, not a command name.
    pub fn handle_uplevel_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        // Two literal-body shapes: `uplevel LEVEL {body}` (level word + single
        // braced body) and `uplevel {body}` (implicit level 1).  A `#0` level
        // records the global frame; anything else records an unknown caller
        // frame.  Multi-word scripts (`args.len() > 2`) fall through.
        let (level_word, body_tok, body_text) = match args.len() {
            2 => {
                let Some(bt) = arg_tokens.get(1).copied() else {
                    return false;
                };
                (args[0].clone(), bt, args[1].clone())
            }
            1 => {
                let Some(bt) = arg_tokens.first().copied() else {
                    return false;
                };
                ("1".to_owned(), bt, args[0].clone())
            }
            _ => return false,
        };
        // A literal `{…}` block, or a script *built* with `list` — the latter
        // is not dynamic (`uplevel #0 [list upvar #0 A B]` runs exactly one
        // deterministic command, tclsh-identical to the braced spelling), so
        // it opens the same frame scope (issue #1138).  Any other shape falls
        // through to the generic recursion.
        if body_tok.kind != TokenType::Str
            && !self.registry.as_deref().is_some_and(|r| {
                crate::script_arg::list_quoted_script_command(r, body_tok, &body_text).is_some()
            })
        {
            return false;
        }

        let path = scope_path.to_vec();
        let child_idx = {
            let Some(parent) = super::scope::scope_at_mut(&mut self.result.global_scope, &path)
            else {
                return false;
            };
            // The scope name carries the level word so `lookup_var_in_scope_chain`
            // can distinguish the `#0` global frame (resolve outward) from a
            // non-`#0` unknown caller frame (abstain outward).
            let mut child = super::types::Scope::new(super::types::ScopeKind::Uplevel, level_word);
            // Only a literal `{…}` block may claim the token bytes as the
            // frame's `body_span` — same rule as `namespace eval` above.  A
            // `[list …]` build substitutes its elements in the *calling*
            // frame before `uplevel` changes frames, so an offset-keyed
            // lookup on (say) `$file` in `uplevel #0 [list source [file join
            // $file]]` must keep resolving lexically to the proc's own
            // parameter, not stop at this frame (issue #1138).
            child.body_span = (body_tok.kind == TokenType::Str).then_some(body_tok.span);
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = path;
        child_path.push(child_idx);
        self.analyse_body(&body_text, body_tok, &child_path);
        true
    }

    /// Handle `namespace ensemble create` — record the namespace as
    /// an ensemble so its tail names become valid commands, plus an
    /// explicit `-command name` override when present.
    ///
    /// The implicit form (`namespace eval ::ens { namespace ensemble
    /// create }`) dispatches through a command named after the enclosing
    /// namespace; `-command NAME` instead creates the ensemble's dispatch
    /// command under an arbitrary, possibly differently-namespaced, name.
    /// Without recording that name too, a call through it (`myEns
    /// subcmd …`) resolves to nothing the analyser knows — drawing a
    /// spurious W123 and abstaining from arity checking for the wrong
    /// reason (an unresolved name) rather than the right one (a
    /// dynamically-defined ensemble the analyser can't see the
    /// subcommand map of).
    ///
    /// `namespace ensemble create`'s options are registry data
    /// (`ENSEMBLE_CREATE_OPTIONS` in `tcl-registry`'s `namespace_`
    /// module), not a hardcoded name list — walking by each option's
    /// declared value arity (rather than a bare `opt == "-command"` scan
    /// over every word) is what keeps another option's *value* word
    /// (`-map`'s dict, `-subcommands`' list, …) from ever being misread as
    /// `-command`'s own flag or value.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceEnsemble`]
    /// (stamped on `namespace`'s `ensemble` subcommand); `args[0]` is
    /// still the subcommand word, and only the `create` form (checked
    /// on `args[1]`) mutates anything.
    pub fn handle_namespace_ensemble(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() < 2 {
            return;
        }
        // `create` operates on the *current* namespace, with no explicit name
        // argument; `configure NAME ...` (issue #1001 follow-up — the ensemble
        // side of the `namespace ensemble configure -map` gap, previously
        // silently ignored here entirely) names an *existing* ensemble
        // command explicitly, resolved like any other command reference.
        // Both share the same `-command`/`-map`/`-subcommands` option set.
        let is_create = args[1] == "create";
        let is_configure = args[1] == "configure";
        if !is_create && !is_configure {
            return;
        }
        // `namespace ensemble create` names the ensemble after the namespace
        // the command *resolves in*, not the lexically enclosing one: run
        // inside the body of `proc ::tk::foo {} {...}` declared at top level,
        // the current namespace is `::tk`, so the ensemble is `::tk` — the
        // purely lexical namespace walk skips proc scopes and homed it
        // to `::` instead, losing every `<ns> sub` call site (issue #923
        // idx 85).
        let ns = self.command_resolution_namespace(scope_path);
        let ns_prefix = ns.trim_start_matches(':').to_owned();

        let (opts, opt_tokens, configure_target) = if is_create {
            if !ns.is_empty() && ns != "::" {
                self.ensemble_namespaces.insert(ns.clone());
            }
            (&args[2..], arg_tokens.get(2..).unwrap_or(&[]), None)
        } else {
            let Some(name) = args.get(2) else { return };
            if crate::naming::is_dynamic_word(name) {
                return;
            }
            let target = self.resolve_command_qualified_name(name, scope_path);
            (
                args.get(3..).unwrap_or(&[]),
                arg_tokens.get(3..).unwrap_or(&[]),
                Some(target),
            )
        };

        // The ensemble's own resolved command name (issue #1001 follow-up):
        // `-command NAME`, when present, *replaces* the default naming
        // entirely (tclsh 8.6.14-verified: `namespace ensemble create
        // -command ::alt` creates only `::alt`, never the enclosing
        // namespace's own name too) — so scan for it before recording any
        // `-map`, rather than defaulting eagerly the way `ensemble_namespaces`
        // (a broader, over-inclusive "valid command name" recovery set) does.
        let explicit_command = is_create
            .then(|| {
                opts.iter()
                    .enumerate()
                    .find_map(|(i, o)| (o == "-command").then(|| opts.get(i + 1)).flatten())
            })
            .flatten()
            .filter(|v| !v.is_empty() && !crate::naming::is_dynamic_word(v))
            .map(|v| qualify(&ns_prefix, v));
        let ensemble_key = configure_target
            .or(explicit_command)
            .or_else(|| (is_create && !ns.is_empty() && ns != "::").then(|| ns.clone()));

        let option_specs: Vec<&tcl_registry::hover::OptionSpec> = self
            .registry
            .as_deref()
            .and_then(|r| r.get("namespace"))
            .and_then(|spec| spec.subcommand("ensemble").map(|sub| (spec, sub)))
            .map(|(spec, sub)| {
                use tcl_registry::ProfileQueries;
                self.profile.available_sub_option_specs(spec, sub)
            })
            .unwrap_or_default();

        let mut i = 0usize;
        while i < opts.len() {
            let Some(spec) = option_specs.iter().find(|o| o.matches(opts[i].as_str())) else {
                i += 1;
                continue;
            };
            let value = opts.get(i + 1);
            let value_tok = opt_tokens.get(i + 1).copied();
            match spec.name {
                // `-command NAME` names the ensemble command — its namespace
                // is recorded so `<ns> sub` calls resolve.  A dynamic value
                // (`$var` / `[cmd]`) can't be resolved statically.
                "-command" => {
                    if let Some(value) = value
                        && !value.is_empty()
                        && !value.starts_with('$')
                        && !value.starts_with('[')
                    {
                        self.ensemble_namespaces.insert(qualify(&ns_prefix, value));
                    }
                }
                // `-map {sub target sub target …}` — every *target* (an
                // odd-indexed element) is a command the ensemble dispatches to,
                // recorded so it is reached by references / definition / rename
                // — and, keyed by the ensemble's own resolved command name, so
                // the W129 safe-interpreter gate can resolve a call through
                // this redirect to the target (issue #1001 follow-up).
                "-map" => {
                    if let (Some(value), Some(tok)) = (value, value_tok)
                        && let Some((text, text_tok)) = self.ensemble_list_literal(value, tok)
                    {
                        self.record_ensemble_map_targets(
                            &text,
                            text_tok,
                            scope_path,
                            ensemble_key.as_deref(),
                        );
                    }
                }
                // `-subcommands {a b c}` — each subcommand `a` dispatches to
                // the command `<ns>::a` in the ensemble's namespace.
                "-subcommands" => {
                    if let (Some(value), Some(tok)) = (value, value_tok)
                        && let Some((text, text_tok)) = self.ensemble_list_literal(value, tok)
                    {
                        self.record_ensemble_subcommands(
                            &text,
                            text_tok,
                            &ns_prefix,
                            ensemble_key.as_deref(),
                        );
                    }
                }
                "-prefixes" => self.record_ensemble_prefixes(value, ensemble_key.as_deref()),
                _ => {}
            }
            i += 1 + spec.value_word_count(opts, i);
        }
    }

    /// Record a `namespace ensemble … -prefixes 0` configuration.
    ///
    /// `-prefixes 0` turns off this ensemble's `Tcl_GetIndexFromObj` prefix
    /// matching, so an abbreviated subcommand word there is a plain
    /// unknown-subcommand error rather than an abbreviation. Keyed by the
    /// ensemble's resolved command name so the abbreviation machinery (W145,
    /// the formatter's expansion) abstains on it.
    ///
    /// tclsh-proof (8.6.16): `namespace ensemble create -command ::e
    /// -subcommands {alpha} -prefixes 0` then `::e al` →
    /// `unknown subcommand "al": must be alpha`; without `-prefixes 0` the
    /// same call succeeds.
    fn record_ensemble_prefixes(&mut self, value: Option<&String>, ensemble_key: Option<&str>) {
        if let (Some(value), Some(key)) = (value, ensemble_key)
            && matches!(tcl_registry::abbrev::resolve_boolean(value), Some(false))
        {
            self.prefixless_ensembles.insert(key.to_owned());
        }
    }

    /// Resolve a `-map`/`-subcommands` option value to its statically-known
    /// list text + representative token, or `None` when nothing static can
    /// be extracted.
    ///
    /// A plain literal (`{sub target …}`) passes through unchanged — the
    /// pre-existing, common case, where `-command`'s own equivalent
    /// dynamic-value check (`starts_with('$')`/`starts_with('[')`) already
    /// guards a *whole*-value dynamic. `-map`/`-subcommands` additionally
    /// tolerate one dynamic *element* among literal ones today
    /// ([`Self::list_word_elements`]'s per-element `is_dynamic_word`), but a
    /// value that is itself one whole dynamic `[...]` substitution is not a
    /// list at all — naively word-splitting
    /// `[dict merge [namespace ensemble configure tk -map] {systray
    /// ::tk::systray}]` would misread fragments of the *expression*
    /// (`"tk"`, `"configure"`, …) as bogus subcommand/target pairs, which is
    /// worse than abstaining. Falls back to
    /// [`Self::dict_merge_literal_tail`] for the one dynamic shape real code
    /// needs (issue #923 idx 84); anything else abstains.
    fn ensemble_list_literal(&self, value: &str, value_tok: Token) -> Option<(String, Token)> {
        if !crate::naming::is_dynamic_word(value) {
            return Some((value.to_owned(), value_tok));
        }
        self.dict_merge_literal_tail(value_tok)
    }

    /// Recognise the `[dict merge EXISTING {literal}]`-shaped value the real
    /// `tk/library/systray.tcl` (and `print.tcl`, `fileicon.tcl`,
    /// `accessibility.tcl`) idiom uses to splice new entries onto a
    /// pre-existing ensemble's `-map`/`-subcommands` without a literal
    /// `{...}` value of its own (issue #923 idx 84): `namespace ensemble
    /// configure tk -map [dict merge [namespace ensemble configure tk -map]
    /// {systray ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]`.
    /// `EXISTING` (whatever it evaluates to — typically a self-referential
    /// query of the ensemble's own current map) is left unknown, but the
    /// spliced literal tail is a statically known fact regardless of what
    /// `EXISTING` is.
    ///
    /// Deliberately narrow, matching the issue #923 idx 110 precedent:
    /// exactly `dict merge ARG {literal}` (2 dict-merge operands, the
    /// second a literal word) — does not recognise `dict set`/`dict
    /// replace`/`concat`/a list-building helper proc, or a `dict merge`
    /// with more than 2 operands. A documented scope boundary, not an
    /// oversight (no attested real-world instance of those forms).
    fn dict_merge_literal_tail(&self, value_tok: Token) -> Option<(String, Token)> {
        if value_tok.kind != TokenType::Cmd {
            return None;
        }
        let config = self.lexer_config();
        let sm = SourceMap::new(&self.source);
        let descended = descend_token(&sm, value_tok, config);
        let segs = segments_from_tree(descended.tree(), &sm);
        let [seg] = segs.as_slice() else { return None };
        if seg.texts.len() != 4 || seg.texts[0] != "dict" || seg.texts[1] != "merge" {
            return None;
        }
        let tail = seg.texts[3].clone();
        if crate::naming::is_dynamic_word(&tail) {
            return None;
        }
        seg.argv.get(3).map(|tok| (tail, *tok))
    }

    /// The `(element, span)` pairs of a list word's *top-level* Tcl-list
    /// elements — proper brace/quote-aware splitting
    /// ([`find_element`]), not naive whitespace splitting, so a braced
    /// multi-word element (`{source b.tcl}`, the shape a `-map` *target*
    /// commonly takes — see [`Self::record_ensemble_map_targets`]) comes
    /// back as one element instead of being shredded into stray fragments
    /// that no longer line up in pairs (codex review, #1001 follow-up: a
    /// naive `split_whitespace` turned `-map {go {source b.tcl}}` into
    /// `["go", "{source", "b.tcl}"]`, an unmatched three-way split that
    /// silently dropped the pairing entirely). Each element's span is
    /// located inside the token's content (`content_offset` skips the
    /// opening delimiter). A malformed trailing element (unmatched
    /// brace/quote, typically mid-edit) simply stops the scan early,
    /// matching this codebase's established lenient-list-parsing
    /// convention (`tcl_syntax::list::split_list_lenient`) rather than
    /// discarding everything already parsed. Shared by the ensemble
    /// `-map` / `-subcommands` extraction; a dynamic element is left for
    /// the caller to skip.
    fn list_word_elements(list_text: &str, tok: Token) -> Vec<(String, Span)> {
        let content_start = tok.span.start() + u32::from(tok.content_offset);
        let mut out = Vec::new();
        let mut pos = 0usize;
        while let Ok(Some(el)) = find_element(list_text, pos) {
            if let Some(text) = list_text.get(el.value.clone()) {
                let start = content_start + u32::try_from(el.value.start).unwrap_or(0);
                let end = content_start + u32::try_from(el.value.end).unwrap_or(0);
                out.push((text.to_string(), Span::new(start, end)));
            }
            pos = el.next;
        }
        out
    }

    /// The head word `(text, span)` of a command-prefix string — Tcl-list
    /// parses `text` and returns just its first element, the command
    /// actually invoked once the prefix's trailing words and the caller's
    /// own arguments are appended (mirrors
    /// `signature_scan::command_prefix::extract_prefix_head`'s
    /// braced-multi-word case, applied to a string with no lexer token of
    /// its own — a `-map` target sits *inside* another list element, not
    /// as a distinct token). `base_start` is `text`'s own absolute start
    /// offset in the source, so the returned span locates the head word
    /// there, not merely within `text`. `None` for an empty or malformed
    /// (unmatched brace/quote) prefix.
    fn command_prefix_head(text: &str, base_start: u32) -> Option<(String, Span)> {
        let head_el = find_element(text, 0).ok().flatten()?;
        let head = text.get(head_el.value.clone())?;
        if head.is_empty() {
            return None;
        }
        let start = base_start + u32::try_from(head_el.value.start).ok()?;
        let end = base_start + u32::try_from(head_el.value.end).ok()?;
        Some((head.to_string(), Span::new(start, end)))
    }

    /// Record every `-map` target (the odd elements of the `sub target …`
    /// list) as a command reference resolved in the caller's namespace, and
    /// — when `ensemble_key` is `Some` (the ensemble's own resolved
    /// qualified command name, computed by [`Self::handle_namespace_ensemble`])
    /// — also record each `sub -> target` pair into
    /// `self.ensemble_command_maps`, so the W129 safe-interpreter gate can
    /// resolve a call reaching a hidden command only through this ensemble's
    /// redirect (issue #1001 follow-up; `None` when the ensemble's own name
    /// couldn't be resolved statically, matching every other guard in this
    /// handler), *and* — when the paired subcommand word is also static —
    /// file the same `sub -> target` fact under `ensemble_key` in
    /// [`AnalysisResult::ensemble_subcommand_targets`] so `definition`/
    /// `hover`/`references` can resolve a `<ensemble> <subcommand>` call site
    /// (issue #923 idx 106) — a distinct field from `ensemble_command_maps`
    /// above, serving navigation rather than the safe-interpreter gate.
    ///
    /// The navigation entry is tagged
    /// [`EnsembleSubcommandProvenance::Map`](crate::signature_scan::types::EnsembleSubcommandProvenance::Map)
    /// (issue #1281): a `-map` key is an arbitrary name, so a consumer that
    /// *rewrites* the subcommand word — rename — must leave it alone, unlike
    /// the `-subcommands` sibling below whose entry is the target's own tail.
    ///
    /// A `-map` target is a command name **or a command prefix** in real
    /// Tcl (tclsh 8.6.14-verified: `-map {go {string length}}` dispatches
    /// `myens go hello` to `string length hello`) — the command actually
    /// invoked is the prefix's *head*; the rest are baked-in arguments, not
    /// part of the command's identity. [`Self::command_prefix_head`]
    /// extracts just that head (codex review, #1001 follow-up: recording
    /// the whole multi-word target text verbatim, or worse, splitting it
    /// on whitespace before pairing it with its subcommand at all, means
    /// a target like `{source b.tcl}` never matches the registry's bare
    /// `source` and W129 stays silently missed for this valid indirection
    /// shape) — the reference, the map entry, and the navigation entry all
    /// use only the head.
    ///
    /// Every `-map` value **replaces** the ensemble's entire subcommand
    /// table in real Tcl, whether given at `create` or a later
    /// `configure` (tclsh 8.6.14-verified: `configure myens -map {ok
    /// puts}` after `create ... -map {bad source ok puts}` turns `myens
    /// bad` into an "unknown or ambiguous subcommand" error, not a
    /// leftover redirect to `source`) — codex review, #1001 follow-up:
    /// merging new pairs into the existing cached map instead of
    /// replacing it would leave a subcommand a later `-map` dropped still
    /// resolving to its stale target, a false-positive risk. The cached
    /// map for `ensemble_key` is cleared before any of its new pairs are
    /// inserted.
    ///
    /// The map stores the *raw written* head text (`"source"`), not
    /// `resolved` (the namespace-qualified form used for the reference
    /// below) — [`Self::check_ensemble_redirect_hiding`] hands it straight
    /// to [`Self::safe_interp_visibility_gate`], which — like the direct
    /// literal-head case and every other indirection path this fix adds —
    /// checks the bare written spelling against the registry, not a
    /// namespace-qualified path. This matters concretely: a `-map` target is
    /// namespace-qualified relative to the ensemble's own (possibly
    /// synthetic, interp-domain-rooted) home namespace by real Tcl's own
    /// rule (tclsh 8.6.14-verified: `-map {go source}` inside `namespace
    /// eval myns {…}` really dispatches `go` to `::myns::source`, not the
    /// global builtin, and raises its own unrelated `invalid command name
    /// ::myns::source` in every interpreter, safe or not, when no such proc
    /// exists) — using the qualified form here would make the check depend
    /// on the interp-domain namespace model lining up with the registry's
    /// flat, unqualified command-name keying, which it structurally can't.
    /// The raw-text check this uses instead only fires for a target that is
    /// unqualified or explicitly `::`-rooted, matching the shape a
    /// `SAFE_INTERP_HIDDEN` registry command's name actually takes; a
    /// locally shadowing `proc` by the same bare name, anywhere in the
    /// tracked safe interpreter body, still suppresses it via the *existing*
    /// `ctx.exposed` check inside `safe_interp_visibility_gate` (populated
    /// by `mark_locally_defined_in_enclosing_interp`, which is namespace-
    /// blind in exactly the same way already, for the direct-call case).
    fn record_ensemble_map_targets(
        &mut self,
        list_text: &str,
        tok: Token,
        scope_path: &[usize],
        ensemble_key: Option<&str>,
    ) {
        if let Some(key) = ensemble_key {
            self.ensemble_command_maps
                .entry(key.to_string())
                .or_default()
                .clear();
        }
        for pair in Self::list_word_elements(list_text, tok).chunks(2) {
            let [(sub, _), (target, span)] = pair else {
                continue;
            };
            if crate::naming::is_dynamic_word(target) {
                continue;
            }
            let Some((head, head_span)) = Self::command_prefix_head(target, span.start()) else {
                continue;
            };
            if let Some(key) = ensemble_key {
                self.ensemble_command_maps
                    .entry(key.to_string())
                    .or_default()
                    .insert(sub.clone(), head.clone());
            }
            let resolved = self.resolve_command_qualified_name(&head, scope_path);
            if let Some(key) = ensemble_key
                && !crate::naming::is_dynamic_word(sub)
            {
                self.result
                    .ensemble_subcommand_targets
                    .entry(key.to_owned())
                    .or_default()
                    .insert(
                        sub.clone(),
                        super::types::EnsembleSubcommandTarget {
                            target: resolved.clone(),
                            provenance:
                                crate::signature_scan::types::EnsembleSubcommandProvenance::Map,
                        },
                    );
                self.ensemble_record_offsets
                    .insert(key.to_owned(), tok.span.start());
            }
            self.push_command_reference(head, head_span, resolved, None);
        }
    }

    /// Record each `-subcommands` name as a reference to the command
    /// `<ns>::<name>` the ensemble maps it to, and file the same
    /// `subcommand → resolved target` fact under `ensemble_key` in
    /// [`AnalysisResult::ensemble_subcommand_targets`] — the `-subcommands`
    /// sibling of [`Self::record_ensemble_map_targets`]'s issue #923 idx 106
    /// fix (same one-directional-only gap, same fix shape).
    ///
    /// Tagged
    /// [`EnsembleSubcommandProvenance::Subcommands`](crate::signature_scan::types::EnsembleSubcommandProvenance::Subcommands)
    /// (issue #1281): here the subcommand word *is* the target's tail — the
    /// ensemble derives `<ns>::<name>` from it — so renaming the target must
    /// rewrite the entry and the dispatch word with it.
    fn record_ensemble_subcommands(
        &mut self,
        list_text: &str,
        tok: Token,
        ns_prefix: &str,
        ensemble_key: Option<&str>,
    ) {
        for (elem, span) in Self::list_word_elements(list_text, tok) {
            if crate::naming::is_dynamic_word(&elem) {
                continue;
            }
            let resolved = qualify(ns_prefix, &elem);
            if let Some(key) = ensemble_key {
                self.result
                    .ensemble_subcommand_targets
                    .entry(key.to_owned())
                    .or_default()
                    .insert(
                        elem.clone(),
                        super::types::EnsembleSubcommandTarget {
                            target: resolved.clone(),
                            provenance: crate::signature_scan::types::
                                EnsembleSubcommandProvenance::Subcommands,
                        },
                    );
                self.ensemble_record_offsets
                    .insert(key.to_owned(), tok.span.start());
            }
            self.push_command_reference(elem, span, resolved, None);
        }
    }

    /// Define a list of variables from a varList token (e.g. the
    /// loop-variable list of `foreach`).
    ///
    /// Each name gets its **own** definition span, located inside the
    /// varList token's content (the token's `content_offset` skips a
    /// leading `{`/`"`). This — crucially — gives same-token bindings
    /// like `foreach {b a}` / `dict for {k v}` distinct,
    /// declaration-ordered spans, so downstream offset-sorted consumers
    /// (the `symbols` / `symbolgraph` CLI verbs) stay deterministic and
    /// source-ordered. Tcl list grouping and backslash substitutions are
    /// decoded for the variable name while each definition span remains the
    /// exact source range of its list element.
    fn define_vars_from_list(&mut self, var_list_text: &str, tok: Token, scope_path: &[usize]) {
        let Ok(names) = tcl_syntax::list::split_list(var_list_text) else {
            return;
        };
        let content_start = tok.span.start() + u32::from(tok.content_offset);
        let mut pos = 0usize;
        for name in names {
            let Ok(Some(element)) = tcl_syntax::list::find_element(var_list_text, pos) else {
                return;
            };
            let start = content_start + u32::try_from(element.value.start).unwrap_or(0);
            let end = content_start + u32::try_from(element.value.end).unwrap_or(0);
            self.define_var(
                name.as_ref(),
                tok,
                scope_path,
                true,
                Some(Span::new(start, end)),
            );
            pos = element.next;
        }
    }

    /// Handle `foreach varList1 list1 ?varList2 list2 ...? body` (and the
    /// `foreach_in_collection` dialect variant, whose spec carries the same
    /// hook).
    ///
    /// Defines every `varListN` (the registry's own arity spec,
    /// `Arity::stepped(3, Arity::UNLIMITED, 2)`, documents an unlimited
    /// number of `varList`/`list` pairs, tclsh 8.6/9.0-verified — issue
    /// #923 idx 70) in the active scope, then recurses into the body so
    /// vars defined inside the loop land in the enclosing scope.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Foreach`].
    pub fn handle_foreach_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 3 {
            return false;
        }
        // Every `varList` sits at an even index, paired with its `list` at
        // the next odd index; the final argument (odd count, since each
        // pair contributes 2 and the body adds 1 more) is always the body,
        // so this walks pairs only up to (not including) the last index.
        for i in (0..args.len() - 1).step_by(2) {
            if let Some(tok) = arg_tokens.get(i) {
                self.define_vars_from_list(&args[i], *tok, scope_path);
            }
        }
        // A single-pair `foreach VAR {literal list} { rename ::$VAR ... ;
        // proc ::$VAR ... }` loop over a fully literal list (issue #923 idx
        // 86: the real `tk/library/accessibility.tcl` rename-away-and
        // -reinstall idiom) binds `VAR` to a *different* value each
        // iteration — a fact the single-value `const_strings` scope map
        // can't represent generally (`foreach` shares one flat cell for the
        // whole loop, confirmed against tclsh: no per-iteration scope to key
        // a value under). Bind `VAR` to the *first* literal element before
        // the one normal body walk below, so `rename`/`proc`'s own
        // constant-fold (unchanged) resolves that iteration correctly;
        // `simulate_remaining_foreach_iterations` afterwards narrowly
        // re-dispatches just `rename`/`proc` for every *additional* literal
        // element, since those are the only two commands go-to-definition/
        // references/rename need to see resolved here.
        let list_braced = arg_tokens
            .get(1)
            .is_some_and(|tok| tok.kind == TokenType::Str);
        let literal_binding = (args.len() == 3)
            .then(|| Self::literal_foreach_binding(&args[0], &args[1], list_braced))
            .flatten();
        if let Some((var, elements)) = &literal_binding
            && let Some(first) = elements.first()
        {
            self.set_const_string(var, first.clone(), arg_tokens[1].span, scope_path);
        }
        // The body is always the last argument; recurse so vars
        // defined inside the loop land in the enclosing scope.
        if let (Some(body_text), Some(body_tok)) = (args.last(), arg_tokens.last().copied()) {
            self.analyse_body(body_text, body_tok, scope_path);
            if let Some((var, elements)) = &literal_binding {
                self.simulate_remaining_foreach_iterations(var, elements, body_tok, scope_path);
            }
        }
        true
    }

    /// Whether a `foreach`'s only var/list pair (`var_list_text`/
    /// `list_text`) is the narrow, fully-literal shape
    /// [`Self::simulate_remaining_foreach_iterations`] can simulate: a
    /// single plain identifier (not itself dynamic, not a multi-name list)
    /// bound to a Tcl-list-valued sequence of literal (non-dynamic)
    /// elements. Returns `(var, elements)`.
    ///
    /// `list_braced` says whether the value word was brace-quoted. Braces
    /// suppress substitution across the whole word, so *every* element of a
    /// braced list is literal however many `$`/`[` characters it holds — the
    /// element text arrives with the braces stripped, and scanning it alone
    /// made one odd element abstain from the whole simulation (issue #1252).
    /// tclsh-proof (8.6.16 / 9.0.4):
    ///   foreach n {aa {$b} cc} { puts $n }   ->  aa / $b / cc
    /// i.e. the loop runs three times over literal values, and no read of
    /// `b` happens at any point.
    fn literal_foreach_binding(
        var_list_text: &str,
        list_text: &str,
        list_braced: bool,
    ) -> Option<(String, Vec<String>)> {
        let vars = tcl_syntax::list::split_list(var_list_text).ok()?;
        let [var] = vars.as_slice() else {
            return None;
        };
        if crate::naming::is_dynamic_word(var) {
            return None;
        }
        // Parse the value as a real Tcl list, not `split_whitespace`: a
        // braced element like `{bar baz}` in `foo {bar baz}` is a *single*
        // list element (`bar baz`), and whitespace-splitting would mis-slice
        // it into `{bar` + `baz}` and bind the loop var to those bogus
        // fragments — the re-dispatched `rename`/`proc` handlers would then
        // invent command facts for `{bar`/`baz}` and miss the real `bar baz`
        // iteration (Codex review, PR #1020). A malformed list (unbalanced
        // brace/quote) is not a valid `foreach` value at all — real Tcl
        // errors on it — so `split_list`'s `Err` means abstain entirely.
        let elements: Vec<String> = tcl_syntax::list::split_list(list_text)
            .ok()?
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect();
        if elements.is_empty()
            || elements
                .iter()
                .any(|e| tcl_syntax::naming::word_is_dynamic(e, list_braced))
        {
            return None;
        }
        Some((var.to_string(), elements))
    }

    /// For each literal element *after* the first (the first iteration is
    /// already covered by [`Self::handle_foreach_command`]'s own
    /// pre-binding + the loop's one normal body walk), narrowly re-dispatch
    /// just the body's own **named-definition installers** with `var`
    /// temporarily rebound to that element — the commands whose registry
    /// spec carries [`tcl_registry::Traits::INSTALLS_NAMED_DEFINITION`],
    /// because those (and only those) name their target with a word whose
    /// substituted value changes per iteration, so each element installs a
    /// *different* definition (issue #923 idx 86 for `proc`/`rename`, idx
    /// 55 for `oo::define`'s class target). Which commands those are is
    /// registry data, never a name list here — a newly-stamped spec joins
    /// the simulation with no edit to this walker.
    ///
    /// Every *other* command in the body keeps the single evaluation the
    /// normal walk above already gave it — deliberately narrow, matching
    /// the issue #923 idx 110 precedent: this does not generally re-walk
    /// the body per iteration (which would duplicate diagnostics/scope
    /// entries for everything else).
    ///
    /// Cost is `O(elements × body-commands)` with no fixpoint: the element
    /// list is a bounded literal and each element re-dispatches only the
    /// already-segmented installer commands.
    ///
    /// Leaves `var` bound to the *last* element afterwards — the same
    /// value real Tcl leaves the loop variable holding once `foreach`
    /// completes.
    fn simulate_remaining_foreach_iterations(
        &mut self,
        var: &str,
        elements: &[String],
        body_tok: Token,
        scope_path: &[usize],
    ) {
        use tcl_registry::hooks::AnalyserHookId as Hook;

        if elements.len() < 2 || body_tok.kind != TokenType::Str {
            return;
        }
        // `body_tok`'s span must address real text in `self.source` — a
        // direct unit-level `handle_foreach_command` call (bypassing the
        // real tokeniser) can pass a synthetic span that doesn't, the same
        // out-of-bounds guard `Self::cmd_fragments` already needs for the
        // identical reason.
        let start = body_tok.span.start() as usize;
        let end = body_tok.span.end() as usize;
        if start >= self.source.len() || end > self.source.len() || start >= end {
            return;
        }
        let config = self.lexer_config();
        let segs: Vec<SegmentedCommand> = {
            let sm = SourceMap::new(&self.source);
            let descended = descend_token(&sm, body_tok, config);
            segments_from_tree(descended.tree(), &sm)
        };
        // Which of the body's commands install a per-iteration name is a
        // fixed property of the segmented body, so classify once (registry
        // lookup per body command) rather than once per element.
        let installers: Vec<(SegmentedCommand, Hook)> = segs
            .iter()
            .filter_map(|seg| {
                let resolved = self.resolve_analyser_hook_call(seg.name(), seg.args())?;
                resolved
                    .traits
                    .contains(tcl_registry::Traits::INSTALLS_NAMED_DEFINITION)
                    .then(|| (seg.clone(), resolved.hook))
            })
            .collect();
        if installers.is_empty() {
            return;
        }
        for element in &elements[1..] {
            self.set_const_string(var, element.clone(), body_tok.span, scope_path);
            for (seg, hook) in &installers {
                let args = seg.args();
                let arg_tokens = seg.arg_tokens();
                let arg_single = seg.arg_single_token();
                match hook {
                    Hook::Proc => {
                        self.handle_proc_command(args, arg_tokens, arg_single, scope_path);
                    }
                    Hook::Rename => {
                        self.handle_rename(
                            args,
                            arg_tokens,
                            arg_single,
                            scope_path,
                            seg.span.start(),
                        );
                    }
                    Hook::OoDefine => {
                        self.handle_oo_define_command(
                            seg.name(),
                            args,
                            arg_tokens,
                            arg_single,
                            scope_path,
                        );
                    }
                    Hook::OoObjdefine => {
                        self.handle_oo_objdefine(args, arg_tokens, arg_single, scope_path);
                    }
                    // Unreachable for a spec carrying the trait —
                    // `installer_hook_is_redispatched` pins that every one
                    // of them lands on an arm above, so a newly-stamped
                    // spec cannot silently promise a re-dispatch this match
                    // never delivers (Codex review of PR #1074).
                    _ => {}
                }
            }
        }
    }

    /// Handle `for init test next body`.
    ///
    /// Recurses into init / next / body so locals defined inside any
    /// of the three statement positions land in the enclosing scope's
    /// variable set.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::For`].
    pub fn handle_for_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 4 {
            return false;
        }
        // init body
        if let Some(tok) = arg_tokens.first().copied() {
            self.analyse_body(&args[0], tok, scope_path);
        }
        // next body
        if let Some(tok) = arg_tokens.get(2).copied() {
            self.analyse_body(&args[2], tok, scope_path);
        }
        // main body
        if let Some(tok) = arg_tokens.get(3).copied() {
            self.analyse_body(&args[3], tok, scope_path);
        }
        true
    }

    /// Handle `switch ?options? string ?pattern body? ...`.
    ///
    /// Arity checking lives in `compiler_checks::arity_checks` via
    /// the IR; this handler walks each arm body so locals defined
    /// inside an arm land in the enclosing scope.
    ///
    /// Switch has two forms:
    ///
    /// 1. ``switch ?options? string pattern body ?pattern body? ...``
    ///    — pattern and body args alternate inline.
    /// 2. ``switch ?options? string {pattern body ?pattern body? ...}``
    ///    — pattern/body pairs live inside a single braced
    ///    block.  See [`crate::segmenter::flatten_clause_list_elements`]
    ///    for how that braced form is split.
    ///
    /// Bodies that are literally ``-`` are fall-through markers
    /// (the next arm's body fires) and are skipped — recursing
    /// into the literal ``-`` would produce a useless command.
    ///
    /// `-regexp` pattern recording: each non-`default`
    /// pattern arm is recorded as a `RegexPattern` with
    /// ``command = "switch"`` when its token is a literal
    /// (`Esc` / `Str`) or a `Var` token whose name resolves via
    /// `lookup_const_string_with_span` to a constant string set
    /// earlier in the same scope.  Command-substitution patterns
    /// are skipped (runtime-computed values can't be statically
    /// resolved).
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Switch`].
    pub fn handle_switch_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 2 {
            return false;
        }

        let Some(registry) = self.registry.as_deref() else {
            return true;
        };
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some((case, invocation)) =
            registry.case_invocation(cmd_name, &arg_refs, self.profile.availability_mask)
        else {
            return true;
        };
        let is_regexp = invocation.mode == tcl_registry::spec::CaseMatchMode::Regexp;

        if let Some(i) = invocation.clause_list_index {
            // Form 2 — single braced body containing all pairs.
            let body_text = args[i].clone();
            let Some(body_tok) = arg_tokens.get(i).copied() else {
                return true;
            };
            let elements =
                crate::segmenter::flatten_clause_list_elements(&self.source, &body_text, body_tok);
            let clause_count = elements.len() / 2;
            let mut j = 0;
            while j + 1 < elements.len() {
                let (pat_text, pat_tok) = &elements[j];
                let (body_text, body_tok) = &elements[j + 1];
                if is_regexp {
                    self.record_switch_regexp_pattern(
                        cmd_name,
                        case,
                        pat_text,
                        *pat_tok,
                        (j / 2, clause_count),
                        scope_path,
                    );
                }
                if case.fallthrough_body != Some(body_text.as_str()) {
                    self.analyse_body(body_text, *body_tok, scope_path);
                }
                j += 2;
            }
        } else if let Some(mut i) = invocation.inline_clause_start {
            // Form 1 — pattern/body pairs inline in args/arg_tokens.
            let clause_count = (args.len() - i) / 2;
            let mut clause_index = 0usize;
            while i + 1 < args.len() {
                if is_regexp && let Some(pat_tok) = arg_tokens.get(i).copied() {
                    self.record_switch_regexp_pattern(
                        cmd_name,
                        case,
                        &args[i],
                        pat_tok,
                        (clause_index, clause_count),
                        scope_path,
                    );
                }
                let body_text = &args[i + 1];
                if let Some(body_tok) = arg_tokens.get(i + 1).copied()
                    && case.fallthrough_body != Some(body_text.as_str())
                {
                    self.analyse_body(body_text, body_tok, scope_path);
                }
                i += 2;
                clause_index += 1;
            }
        }
        true
    }

    /// Record one ``switch -regexp`` arm's pattern.  Skipped for
    /// the ``default`` keyword (Tcl's catch-all).  Literal tokens
    /// are recorded verbatim; `Var` tokens are resolved via the
    /// `const_strings` map (the ``regex-vars`` propagation);
    /// `Cmd` substitutions are skipped (can't statically
    /// resolve).
    fn record_switch_regexp_pattern(
        &mut self,
        command: &str,
        case: tcl_registry::spec::CaseListSpec,
        pattern: &str,
        tok: Token,
        clause_position: (usize, usize),
        scope_path: &[usize],
    ) {
        let (clause_index, clause_count) = clause_position;
        if case.is_keyword_pattern(pattern, clause_index, clause_count) {
            return;
        }
        self.record_regex_pattern_token(pattern, tok, command, scope_path);
    }

    /// Handle `catch SCRIPT ?RESULTVAR? ?OPTIONSVAR?`.
    ///
    /// Defines the optional `RESULTVAR` and `OPTIONSVAR` bindings
    /// (they receive values when the body throws / completes) and
    /// bumps `conditional_depth` for the duration of the body.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Catch`].
    pub fn handle_catch_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        // The script body (args[0]) is evaluated by `catch`, so walk it like
        // every other body-bearing command — otherwise the per-command
        // syntactic checks (W100 unbraced `expr`, W104, W304, …) never reach
        // inside a `catch { … }`, under-reporting. `analyse_body` no-ops on a
        // dynamic body (`catch $cmd`).
        //
        // The body is a *guarded probe*: `catch { package require Foo }` is the
        // idiomatic optional-dependency check, so facts recorded inside it
        // (package requirements, …) must be marked conditional. Bump
        // `conditional_depth` for the walk, matching `if`/`try` bodies (and this
        // handler's own contract — see the doc comment above).
        if let Some(body_tok) = arg_tokens.first().copied() {
            self.conditional_depth += 1;
            self.analyse_body(&args[0], body_tok, scope_path);
            self.conditional_depth -= 1;
        }
        // The result-var / options-var positions come from the registry's
        // `ArgRole::VarWrite` rows on the `catch` spec, matching the nested
        // `[catch …]` path in `dispatch_nested_segment`. The literal spec
        // name is sound here: `AnalyserHookId::Catch` dispatch already
        // resolved the head (qualified spellings included) to this spec.
        if let Some(registry) = self.registry.as_deref() {
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            for i in registry.arg_indices_for_role(
                "catch",
                &arg_strs,
                tcl_registry::arg_role::ArgRole::VarWrite,
            ) {
                if let (Some(name), Some(tok)) = (args.get(i), arg_tokens.get(i)) {
                    let name = name.clone();
                    self.define_var(&name, *tok, scope_path, false, None);
                }
            }
        }
        true
    }

    /// Bind the output variables of any command that writes results into
    /// named arguments — `lassign`, `scan`, `regexp`, `regsub`, `gets`,
    /// `binary scan`, `vwait`, `chan gets`, `file stat`, `dict set`,
    /// `info default`, … — every command whose registry spec marks a
    /// `VarWrite`-role argument (an empty role set no-ops).
    ///
    /// The registry's `VarWrite` arg-role resolver already encodes each
    /// command's option/positional shape (leading `-switches`, the `--`
    /// terminator, the pattern / format / subSpec positionals, and the
    /// variadic capture tail), so this reuses it rather than re-deriving the
    /// layout per command.  Binding these makes the destructured / captured
    /// names visible to completion, hover, and go-to-definition in the
    /// enclosing scope.  `warn_if_unused = false`: like `catch`'s result
    /// variable, the binding is a documented side effect, not a "set but never
    /// read" target, so it must not raise W211.
    ///
    /// Two trait families opt out:
    ///
    /// - [`tcl_registry::Traits::CREATES_SCOPE_ALIAS`] (`global` /
    ///   `variable` / `upvar`): their dedicated handlers
    ///   ([`Self::handle_var_declaration_command`] /
    ///   [`Self::handle_upvar_command`]) own the alias layout —
    ///   tail-stripping (`global ::ns::v` binds the local alias `v`, not
    ///   the qualified name) and name/value pairing — which a flat
    ///   `VarWrite` walk would get wrong.
    /// - [`tcl_registry::Traits::DESTROYS_VARIABLE`] (`unset`): its
    ///   `VarWrite` role marks a *removal* target (an SSA def that kills
    ///   the value), not a binding to record.
    ///
    /// Commands whose dedicated handlers already bind the same name
    /// (`set` / `incr` / `append` / `lappend`, the `dict` subforms) stay
    /// in: those handlers run first and [`Self::define_var`]'s
    /// re-definition path is idempotent for the same token span — no
    /// duplicate reference is pushed and this call's
    /// `warn_if_unused = false` never downgrades an earlier `true`.
    ///
    /// Void-returning (self-guards on the role set) so it composes with the
    /// other side-effect handlers — `regexp` / `regsub` also feed
    /// `handle_regex_pattern_capture`, which must still run.
    pub fn handle_var_binding_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        if registry.get(cmd_name).is_none_or(|spec| {
            spec.traits.intersects(
                tcl_registry::Traits::CREATES_SCOPE_ALIAS | tcl_registry::Traits::DESTROYS_VARIABLE,
            )
        }) {
            return;
        }
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let indices = registry.arg_indices_for_role(
            cmd_name,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::VarWrite,
        );
        for idx in indices {
            let (Some(name), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                continue;
            };
            // Only a plain literal names a definite scope variable.  A computed
            // target (`scan $s $fmt $dyn`), an array element (`arr(i)`), or a
            // brace/bracket-bearing word is not a simple local to bind.
            if name.is_empty() || name.contains(['$', '[', ']', '(', ')', '{', '}', ' ']) {
                continue;
            }
            self.define_var(name, *tok, scope_path, false, None);
        }
    }

    /// Handle `try BODY ?on/trap CODE VARLIST BODY?... ?finally BODY?`.
    ///
    /// Walks the main try body and every handler / finally clause;
    /// arity checking lives in `compiler_checks::arity_checks`
    /// already.
    ///
    /// Clause shapes:
    ///
    /// - ``finally BODY`` (2 words) — recurse into ``BODY``.
    /// - ``on CODE VARLIST BODY`` / ``trap PATTERN VARLIST BODY``
    ///   (4 words) — define the handler's ``VARLIST`` (e.g.
    ///   ``{result options}``), then recurse into ``BODY``.
    ///
    /// Conditional-body depth, per clause kind (issue #1065): the main body
    /// and the `on` / `trap` handler bodies are branch-selected, the
    /// `finally` body is not.  See [`Self::analyse_selected_body`].
    ///
    /// `traits` are the composed traits of the concrete spec / subcommand the
    /// dispatch already resolved this head to, threaded in rather than
    /// re-fetched by name: a name lookup would put per-command knowledge back
    /// in the analyser and would silently diverge from the dispatch the moment
    /// a dialect variant shares this hook.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Try`].
    pub fn handle_try_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        traits: tcl_registry::Traits,
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        // The same trait-driven depth the generic body walk in
        // `dispatch_body_arguments` applies — `Traits::BRANCH_SELECTED_BODY`,
        // carried by exactly `if` and `try`.  `try` reaches its bodies
        // through this hook instead of that walk, so without asking here a
        // `package require` inside a `try` was recorded unconditional
        // (issue #1065).
        let branch_selected = traits.contains(tcl_registry::Traits::BRANCH_SELECTED_BODY);
        // Main try body at args[0].
        if let Some(body_tok) = arg_tokens.first().copied() {
            self.analyse_selected_body(&args[0], body_tok, scope_path, branch_selected);
        }
        // Walk handler / finally clauses.
        let mut i = 1;
        while i < args.len() {
            let kw = args[i].as_str();
            if kw == "finally" && i + 1 < args.len() {
                if let Some(body_tok) = arg_tokens.get(i + 1).copied() {
                    // A `finally` body is *not* branch-selected — see
                    // `analyse_selected_body`.
                    self.analyse_body(&args[i + 1], body_tok, scope_path);
                }
                i += 2;
            } else if matches!(kw, "on" | "trap") && i + 3 < args.len() {
                // `on CODE {msg opts} body` / `trap PAT {msg opts} body` — the
                // var-list at i+2 binds the result message + options dict in
                // the handler body, so define them before walking it.
                if let Some(vl_tok) = arg_tokens.get(i + 2).copied() {
                    self.define_vars_from_list(&args[i + 2], vl_tok, scope_path);
                }
                // A handler body of literal `-` is a fallthrough marker (shares
                // the next handler's body, like `switch`); it is not a script,
                // so it must not be re-lexed as one — otherwise the solo `-`
                // reads as a zero-arg `-` command and trips a spurious arity
                // error (issue #703). Mirrors the `switch` arm handling above.
                if let Some(body_tok) = arg_tokens.get(i + 3).copied()
                    && args[i + 3] != "-"
                {
                    self.analyse_selected_body(&args[i + 3], body_tok, scope_path, branch_selected);
                }
                i += 4;
            } else {
                i += 1;
            }
        }
        true
    }

    /// [`Self::analyse_body`] with `conditional_depth` raised for the walk
    /// when the owning command's bodies are branch-selected, so facts
    /// recorded inside (a `package require`, a const-string write) do not
    /// claim to dominate the code after the command.
    ///
    /// `branch_selected` comes from the owning command's
    /// [`tcl_registry::Traits::BRANCH_SELECTED_BODY`] — the same trait the
    /// generic body walk keys on — never from the command's name.
    ///
    /// Which of `try`'s clause bodies pass `true` follows C Tcl's `try`
    /// semantics, modelled the way `if` already is: `if`'s always-evaluated
    /// condition is an `ArgRole::Expr` argument and is never depth-bumped,
    /// only its branch-selected bodies are.  For `try`:
    ///
    /// - the **main body** always *starts* running, but any statement in it
    ///   may be superseded by an exception a handler then swallows, so
    ///   nothing it establishes dominates the code after the `try`.
    ///   Branch-selected — the same guarded-probe reading
    ///   [`Self::handle_catch_command`] applies to `catch`'s script, and
    ///   `try { package require Foo } on error {} {}` is precisely the
    ///   idiomatic optional-dependency check.
    /// - an **`on` / `trap` handler body** runs only when the body completed
    ///   with a matching completion code / `-errorcode` prefix.
    ///   Branch-selected.
    /// - a **`finally` body** always runs: "an optional trailing finally
    ///   script always runs — even when body or the handler raised an error,
    ///   and irrespective of which handler, if any, matched" (Tcl 9.0.4
    ///   `try(n)`; in `generic/tclCmdMZ.c` both of `TclNRTryObjCmd`'s
    ///   continuations, `TryPostBody` and `TryPostHandler`, schedule the
    ///   finally script before propagating).  So whenever control reaches
    ///   past the `try` at all, the finally body has run — it is the one
    ///   `try` clause that is **not** branch-selected, exactly as
    ///   [`tcl_registry::Traits::BRANCH_SELECTED_BODY`]'s own documentation
    ///   names it.  It is no more conditional than a straight-line statement,
    ///   which can equally fail part-way.
    fn analyse_selected_body(
        &mut self,
        body_text: &str,
        body_tok: Token,
        scope_path: &[usize],
        branch_selected: bool,
    ) {
        if branch_selected {
            self.conditional_depth += 1;
        }
        self.analyse_body(body_text, body_tok, scope_path);
        if branch_selected {
            self.conditional_depth -= 1;
        }
    }

    /// Register the local-alias names introduced by `upvar`, and link the
    /// alias to its target cell whenever the target is frame-independent.
    ///
    /// `upvar ?level? otherVar myVar ?otherVar myVar ...?` — C Tcl decides
    /// whether the level word is present from the **argument count parity**
    /// (`Tcl_UpvarObjCmd` tests `objc`), so an odd count means the first word
    /// is the level. tclsh 9.0.4 / 8.6.14, identical: `upvar 1 b` has an even
    /// count and therefore aliases the caller variable literally named `1`,
    /// while `upvar $lvl a b` really does take `$lvl` as its level.
    ///
    /// **`otherVar`** normally names a variable in *another frame*, which has
    /// no namespace path to link to — a bare `x` at level 1 is whatever local
    /// the caller happens to have. Two spellings escape that and name one
    /// fixed cell whatever the call depth, so both get a link target
    /// (issue #923 audit idx 98, where `upvar ::tk::FocusGrab($index) data`
    /// left the array completely unregistered):
    ///
    /// * a **fully-qualified** target — `upvar 1 ::myns::q g` binds
    ///   `::myns::q` from any depth (tclsh 9.0.4 / 8.6.14: reached
    ///   identically through `upvar 1` and `upvar 2`);
    /// * any target at **`#0`** — the global frame, so `upvar #0 counter c`
    ///   binds `::counter` however deep the call stack is.
    ///
    /// An array element (`::tk::FocusGrab($index)`) links to its **base**:
    /// the element key is runtime data, but the array it lives in is the
    /// named cell every sibling access shares.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Upvar`].
    pub fn handle_upvar_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        use tcl_registry::frame_effect::FrameLevel;

        if args.is_empty() || arg_tokens.is_empty() {
            return;
        }
        let pair_start = usize::from(args.len() % 2 == 1);
        let level = if pair_start == 1 {
            FrameLevel::parse(&args[0]).unwrap_or(FrameLevel::Dynamic)
        } else {
            FrameLevel::DEFAULT
        };
        let mut i = pair_start + 1;
        while i < args.len() && i < arg_tokens.len() {
            // The local alias name may be dynamic (`upvar 1 x $local`) — skip
            // it rather than record the substitution text.
            if !crate::naming::is_dynamic_word(&args[i]) {
                self.define_var(&args[i], arg_tokens[i], scope_path, false, None);
                if let Some(target) = Self::upvar_link_target(&args[i - 1], level)
                    && let Some(other_tok) = arg_tokens.get(i - 1)
                {
                    // `otherVar` (at `i - 1`) names the cell; `args[i]` is an
                    // independent local spelling.  Renaming the cell must
                    // rewrite the former, never the latter.
                    self.set_var_link_target(&args[i], scope_path, target.clone(), other_tok.span);
                    // The fixed cell itself gets a definition at the
                    // `otherVar` word (issue #923 audit idx 98 / issue
                    // #1139): `upvar ::tk::FocusGrab($index) data` is the
                    // only place the array ever comes to exist, so without
                    // a `VarDef` for `::tk::FocusGrab` every occurrence —
                    // this word, a sibling proc's `$::tk::FocusGrab($idx)`
                    // read, its `info exists` / `unset` — answered nothing.
                    // Defined in the GLOBAL scope: both qualifying
                    // spellings (a `::`-qualified target, any target at
                    // `#0`) name a cell reachable from everywhere.
                    self.define_var(&target, *other_tok, &[], false, None);
                }
            }
            i += 2;
        }
    }

    /// The fixed cell an `upvar` `otherVar` word names, or `None` when it
    /// names a frame-relative variable with no stable path.
    ///
    /// See [`Self::handle_upvar_command`] for the two qualifying spellings
    /// and their oracle transcripts.
    fn upvar_link_target(
        other: &str,
        level: tcl_registry::frame_effect::FrameLevel,
    ) -> Option<String> {
        // `a($k)` names the array `a`; the element key is runtime data, so
        // the base is tested for dynamism, not the whole word.
        let base = other.split_once('(').map_or(other, |(base, _)| base);
        if base.is_empty() || crate::naming::is_dynamic_word(base) {
            return None;
        }
        if base.starts_with("::") {
            return Some(base.to_owned());
        }
        if level.is_global_frame() {
            return Some(format!("::{base}"));
        }
        None
    }

    /// Register the local aliases introduced by `namespace upvar`.
    ///
    /// `namespace upvar nsname otherVar myVar ?otherVar myVar ...?` — `myVar`
    /// lives at indices 3, 5, 7, …
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceUpvar`] (stamped
    /// on `namespace`'s `upvar` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_namespace_upvar_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() < 4 {
            return;
        }
        // `namespace upvar nsname otherVar myVar ?otherVar myVar ...?`: `myVar`
        // (indices 3, 5, …) aliases `nsname::otherVar` (`otherVar` at 2, 4, …).
        // Resolve a relative `nsname` against the current namespace.
        let cur = self.command_resolution_namespace(scope_path);
        let cur_prefix = cur.trim_end_matches("::");
        let target_ns = if args[1].starts_with("::") {
            args[1].trim_end_matches("::").to_string()
        } else {
            format!("{cur_prefix}::{}", args[1].trim_end_matches("::"))
        };
        let mut i = 3;
        while i < args.len() && i < arg_tokens.len() {
            // Skip a dynamic local alias name (`namespace upvar ns x $local`).
            if !crate::naming::is_dynamic_word(&args[i]) {
                self.define_var(&args[i], arg_tokens[i], scope_path, false, None);
                // `otherVar` is resolved *within* the target namespace, so keep
                // its full path: `namespace upvar ::a b::c local` aliases `local`
                // to `::a::b::c`, not the tail-collapsed `::a::c`.  An absolute
                // `otherVar` names its cell directly.
                let other = &args[i - 1];
                let target = if other.starts_with("::") {
                    other.clone()
                } else {
                    format!("{target_ns}::{other}")
                };
                if let Some(other_tok) = arg_tokens.get(i - 1) {
                    // `otherVar` (at `i - 1`) names the cell, not the local
                    // alias at `i` — see `VarDef::link_target_span`.
                    self.set_var_link_target(&args[i], scope_path, target, other_tok.span);
                }
            }
            i += 2;
        }
    }

    /// Register the loop variables of `dict for {keyVar valueVar}
    /// dictValue body`.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::DictFor`]
    /// (stamped on `dict`'s `for` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_dict_for_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() >= 4 && arg_tokens.len() >= 2 {
            self.define_vars_from_list(&args[1], arg_tokens[1], scope_path);
        }
    }

    /// Register the alias variables of `dict update dictVar key1 var1
    /// ?key2 var2 ...? body` — vars at 3, 5, 7, … (i.e. `len-2` last).
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::DictUpdate`] (stamped on
    /// `dict`'s `update` subcommand).
    pub fn handle_dict_update_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() < 5 || !(args.len() - 3).is_multiple_of(2) {
            return;
        }
        let mut i = 3;
        while i + 1 < args.len() {
            if let Some(tok) = arg_tokens.get(i) {
                self.define_var(&args[i], *tok, scope_path, false, None);
            }
            i += 2;
        }
    }

    /// Register the key variables of `dict with dictVar body` — only
    /// the no-path case is statically resolvable, and only when the
    /// dict came from a const literal.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::DictWith`]
    /// (stamped on `dict`'s `with` subcommand).
    pub fn handle_dict_with_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.len() != 3 || arg_tokens.len() < 2 {
            return;
        }
        let Some(const_val) = self
            .lookup_const_string(&args[1], scope_path)
            .map(str::to_owned)
        else {
            return;
        };
        let elements = crate::tcl_expr_eval::split_tcl_list(&const_val);
        let mut i = 0;
        while i < elements.len() {
            if !elements[i].is_empty() {
                self.define_var(&elements[i], arg_tokens[1], scope_path, false, None);
            }
            i += 2;
        }
    }

    /// Handle `interp alias {} ALIAS {} TARGET ?ARG ...?` —
    /// records the alias for later argument-role resolution.
    ///
    /// Delegates the actual detection logic to
    /// `crate::alias::detect_interp_alias` (which already handles
    /// the canonical `interp alias {}` shape and the `args[5..]`
    /// prepended-args slice). `offset` is the command token's start,
    /// recorded in [`Analyser::alias_offsets`] for the same-file arity
    /// resolver's top-level order gate.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::InterpAlias`] (stamped on
    /// `interp`'s `alias` subcommand); `args[0]` is still the
    /// subcommand word the detectors expect.
    pub fn handle_interp_alias(&mut self, args: &[String], scope_path: &[usize], offset: u32) {
        if let Some(deleted) = crate::alias::detect_interp_alias_delete(args) {
            // Deleting an alias destroys the command object, so every
            // `namespace import` edge pointing at it dies too (issue #1103).
            // An empty srcPath means the interpreter this command *runs in*,
            // so inside a child's eval body the deletion homes under that
            // child's domain rather than the parent's command table.
            let deleted = format!("{}{deleted}", self.current_interp_domain_prefix());
            self.result
                .destroyed_commands
                .insert(deleted.clone(), offset);
            self.deleted_commands.insert(deleted, offset);
            return;
        }
        // The `interp alias {} A {} B` fast path homes both names at the
        // global root, which is only right in the main interpreter: inside a
        // child's eval body `{}` names *that child*. There, fall through to
        // the domain-aware branch below, which qualifies both sides through
        // `resolve_alias_domain_prefix` exactly as the explicit-path form
        // already does (issue #1141's flaw class).
        if self.interp_path_stack.is_empty()
            && let Some((qualified, target_cmd, prepended)) = detect_interp_alias(args)
        {
            self.record_interp_alias(qualified, target_cmd, prepended, offset);
            return;
        }
        // Cross-domain alias (issue #945 fault 8): `interp alias PATH name
        // TPATH target ?arg…?` deliberately crosses interpreter domains —
        // the alias `name` becomes callable in the *source* interpreter,
        // running `target` resolved in the *target* interpreter.  With
        // literal paths both sides home under their `@interp@` domains, so
        // calls of the alias inside the child's eval bodies resolve to the
        // target through the ordinary alias link machinery. A path need
        // not itself be a source literal any more (issue #923 idx 9): it
        // also resolves through a tracked `set VAR [interp create ...]`
        // binding — only the alias/target *command* names stay hard
        // literal requirements (a dynamic one genuinely names nothing
        // statically, same reasoning `crate::alias::detect_interp_alias`
        // already documents for the same-interpreter form).
        if args.len() >= 5 {
            let (src_path, alias_name, target_path, target_cmd) =
                (&args[1], &args[2], &args[3], &args[4]);
            let literal = |w: &String| !crate::naming::is_dynamic_word(w);
            if literal(alias_name)
                && literal(target_cmd)
                && let Some(src_prefix) = self.resolve_alias_domain_prefix(src_path, scope_path)
                && let Some(target_prefix) =
                    self.resolve_alias_domain_prefix(target_path, scope_path)
                && !(src_prefix.is_empty() && target_prefix.is_empty())
            {
                let qualified = format!("{src_prefix}::{}", alias_name.trim_start_matches(':'));
                let target = format!("{target_prefix}::{}", target_cmd.trim_start_matches(':'));
                let prepended: Vec<String> = args[5..].to_vec();
                self.record_interp_alias(qualified, target, prepended, offset);
            }
        }
    }

    /// Record one resolved `interp alias` fact into the three alias
    /// tables (shared by the current-interp and cross-domain forms).
    fn record_interp_alias(
        &mut self,
        qualified: String,
        target_cmd: String,
        prepended: Vec<String>,
        offset: u32,
    ) {
        // Binding a second name onto `source` (or another external-unit
        // loader) means calls through that name never reach the `Source` hook,
        // so the files they pull in are invisible — see
        // [`Self::note_external_unit_command_moved`].
        self.note_external_unit_command_moved(&target_cmd);
        self.command_aliases
            .insert(qualified.clone(), (target_cmd.clone(), prepended.clone()));
        self.alias_offsets.insert(qualified.clone(), offset);
        self.result.alias_offsets.insert(qualified.clone(), offset);
        self.result.command_aliases.insert(
            qualified.clone(),
            SignatureCommandAlias {
                qualified_name: qualified,
                target: target_cmd,
                extras: prepended,
            },
        );
    }

    /// Resolve one command argument word to a constant string: the word
    /// unchanged when it's already static, else an attempt to
    /// constant-fold it through the same lexical (last-write-wins)
    /// constant-string lattice [`Self::lookup_const_string`] already
    /// serves [`Self::resolve_expansion_count`] for the analogous
    /// `{*}$var`-with-known-value case (issue #923 idx 3) — first via
    /// [`Self::resolve_const_word`] (a pure single `Var`/literal token),
    /// then via [`crate::text::fold_interpolation_single`] for a
    /// multi-token concatenation (`::mypkg::${c}_$key`). `None` means
    /// genuinely unresolvable: no token to inspect, a command
    /// substitution (`[…]`) anywhere in the word, or a variable that
    /// isn't a tracked constant.
    ///
    /// Shared by any command whose argument names something else purely
    /// as data rather than computing it — [`Self::handle_rename`]'s
    /// `OLD`/`NEW` words, and [`Self::handle_source_command`]'s path word
    /// (issue #923 idx 46: `set p "e.tcl"; source $p`, the same shape
    /// `rename $old new` already resolved for idx 3), the `oo::define`
    /// target word, and the command head itself
    /// ([`Self::resolve_dynamic_command_head`]).
    pub(super) fn resolve_dynamic_word(
        &self,
        text: &str,
        tok: Option<Token>,
        is_single: bool,
        scope_path: &[usize],
    ) -> Option<String> {
        if !crate::naming::is_dynamic_word(text) {
            return Some(text.to_string());
        }
        let tok = tok?;
        // Identity resolution (a `source`/`rename` target) must resolve only
        // through const values that *dominate* this use site — never the
        // last-write-wins branch value the lexical map otherwise carries.
        // `set p a.tcl; if {$c} {set p b.tcl}; source $p` must abstain here,
        // not pin the source to `b.tcl` (Codex review, PR #1020): a
        // branch-conditional binding cannot prove a unique target, so
        // `lookup_dominating_const_string` yields `None` for it. This differs
        // from `resolve_const_word` / `lookup_const_string`, which other
        // callers (expansion counts, regex-var tagging) still use with their
        // existing last-write-wins semantics.
        if is_single && matches!(tok.kind, TokenType::Str | TokenType::Esc) {
            return Some(text.to_string());
        }
        if is_single && tok.kind == TokenType::Var {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            // The token's *true source bytes*, which for a braced composite
            // head (`${ns}::setdef`) are the whole word — the lexer merges
            // the `${…}` substitution and everything glued to it into one
            // `Var` token, so the raw text is `ns}::setdef`, not the
            // variable name.  Reading it whole looks up a variable that
            // cannot exist and abstains on a word that is in fact
            // statically resolvable (issue #923 idx 44).  The dispatched
            // variable ends at the brace the lexer left in place; the rest
            // is an ordinary word suffix, folded like any other (it may
            // itself interpolate, `${ns}::${sub}`).  Same truncation rule
            // as `record_var_or_cmd_command_site`'s W307 head reading, so
            // the two agree on which bytes name the variable.
            let raw = sm.token_text(tok);
            let (var_name, suffix) = if tok.content_offset >= 2 {
                raw.split_once('}')
                    .map_or((raw, ""), |(name, rest)| (name, rest))
            } else {
                (raw, "")
            };
            let value = self.lookup_dominating_const_string(var_name, scope_path)?;
            if suffix.is_empty() {
                return Some(value.to_string());
            }
            let folded_suffix = crate::text::fold_interpolation_single(suffix, |name| {
                self.lookup_dominating_const_string(name, scope_path)
                    .map(str::to_string)
            })?;
            return Some(format!("{value}{folded_suffix}"));
        }
        crate::text::fold_interpolation_single(text, |name| {
            self.lookup_dominating_const_string(name, scope_path)
                .map(str::to_string)
        })
    }

    /// Handle `rename OLD NEW` — record a static rename so calls to
    /// `NEW` resolve to whatever `OLD` denoted (the same proc, unchanged
    /// signature — a rename is a pure name move, never an arity change).
    /// Also records that `OLD` itself is no longer a callable command
    /// from this point on (confirmed against tclsh 9.0.4: `OLD` fails
    /// "invalid command name" afterwards, not a "wrong # args" against
    /// its original signature).
    ///
    /// `OLD`/`NEW` need not themselves be literal words: each is first
    /// run through [`Self::resolve_dynamic_word`], which also resolves a
    /// constant-foldable dynamic word (`rename $old ::new`, `set key
    /// impl; rename ::foo_$key ::foo`) — a bare variable read of a
    /// runtime value (`rename [somecommand] ::new`) or one that folding
    /// can't pin down still can't be, and only *that* residual case
    /// returns `true` (dynamic) — the caller widens
    /// `has_dynamic_providers` then, the same wildcard-collapse
    /// convention `command_binding.rs`'s flow-sensitive lattice uses for
    /// the identical shape. A malformed `rename` (wrong argument count,
    /// already flagged by the registry arity check) is not treated as
    /// dynamic — there is no new binding to widen for. `offset` is the
    /// command token's start, recorded in [`Analyser::rename_offsets`] /
    /// [`Analyser::deleted_commands`] for the same-file arity resolver's
    /// top-level order gate. A deleting `rename OLD {}` records only
    /// `OLD`'s deletion (confirmed against tclsh 9.0.4: also "invalid
    /// command name" afterwards) — there is no `NEW` to map it to.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Rename`].
    pub fn handle_rename(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        offset: u32,
    ) -> bool {
        if args.len() != 2 {
            return false;
        }
        let Some(old_resolved) = self.resolve_dynamic_word(
            &args[0],
            arg_tokens.first().copied(),
            arg_single.first().copied().unwrap_or(false),
            scope_path,
        ) else {
            return true;
        };
        let Some(new_resolved) = self.resolve_dynamic_word(
            &args[1],
            arg_tokens.get(1).copied(),
            arg_single.get(1).copied().unwrap_or(false),
            scope_path,
        ) else {
            return true;
        };
        let old = crate::naming::normalise_qualified_name(&old_resolved);
        let new = crate::naming::normalise_qualified_name(&new_resolved);
        if old.is_empty() {
            return false;
        }
        // Moving `source` (or any other external-unit loader) out from under
        // its own name takes its file-loading out of static view — see
        // [`Self::note_external_unit_command_moved`].
        self.note_external_unit_command_moved(&old);
        // `OLD` names an existing command as data — manipulated, not called —
        // the same shape `ArgRole::CommandName` already models for `info body
        // PROC` / `namespace origin NAME`. Recorded as an ordinary command
        // invocation so find-references / go-to-definition / rename see this
        // exact token like any other reference, including for a deleting
        // `rename OLD {}` (there is no `NEW` in that case, so a NEW-keyed
        // span map could never have covered it either way). Real Tcl requires
        // `OLD` to exist (`can't rename "X": command doesn't exist`
        // otherwise), so this also correctly feeds W123 like a real reference
        // would (issue #923 idx 39, main audit wave: a rename applied without
        // rewriting this occurrence leaves it pointing at a now-nonexistent
        // command, crashing the program at runtime with no diagnostic
        // warning).
        if let Some(tok) = arg_tokens.first() {
            self.push_command_reference(args[0].clone(), tok.span, old.clone(), None);
        }
        // `OLD`'s own deletion must not appear to have already happened *at*
        // the reference just pushed above for it — `deleted_commands` is
        // keyed by a single load-order offset compared with `>=` (see
        // `registry_name_deleted_before`), and `OLD`'s token always sits
        // textually after the `rename` command's own start (`offset`), so
        // recording the deletion at `offset` would make that offset compare
        // as "at or after" a call site that is really the deletion's own
        // trigger — wrongly W123-flagging `rename puts myputs`'s own `puts`
        // as a call to an already-dead builtin. Anchoring the deletion to
        // just past `OLD`'s own token keeps every *other*, later call site's
        // ordering unaffected (nothing else can sit between `OLD`'s token
        // and here) while excluding this one.
        let deletion_offset = arg_tokens.first().map_or(offset, |t| t.span.end());
        // A rename moves whatever real Tcl object `OLD` currently denotes —
        // including a live interpreter's own handle command — to `NEW` (or,
        // for a deleting `rename OLD {}`, off the command table entirely).
        // Keep `self.interpreters` in step so a later, unrelated definition
        // that reuses `OLD`'s freed name is never misidentified as still
        // being that interpreter's handle (confirmed against tclsh 9.0.4:
        // after `rename sandbox {}` the child interpreter is gone —
        // `interp slaves` no longer lists it — and a later `proc sandbox
        // {sub body} {...}` dispatches to the new proc, never to the
        // interpreter; `rename sandbox t` instead keeps the same
        // interpreter reachable, now only as `t`). Bumping the epoch on
        // both the vacated key and any interpreter the rename overwrites at
        // `NEW` mirrors `handle_interp_delete_command`, so a later
        // `interp create` recreating either name never merges with the
        // interpreter that used to be tracked there (issue #945 fault 8).
        // Keyed off the *resolved* text (identical to `args[N]` for an
        // already-static rename) so a resolvable dynamic handle
        // (`set h sandbox; rename $h moved`) migrates the tracked state
        // too, not just the ones spelled out literally.
        let old_interp_key = self.qualified_interp_key(&old_resolved);
        if let Some(state) = self.interpreters.remove(&old_interp_key) {
            *self.interp_epochs.entry(old_interp_key).or_insert(0) += 1;
            if !new.is_empty() {
                let new_interp_key = self.qualified_interp_key(&new_resolved);
                if self.interpreters.remove(&new_interp_key).is_some() {
                    *self
                        .interp_epochs
                        .entry(new_interp_key.clone())
                        .or_insert(0) += 1;
                }
                self.interpreters.insert(new_interp_key, state);
            }
        }
        // A `rename` inside a child interpreter's eval body mutates *that*
        // interpreter's command table, never this one's — `interp create c;
        // c eval { rename puts myputs }` leaves the parent's `puts` intact
        // and gives the parent no `myputs` at all (tclsh 9.0.4-verified).
        // Recording it in the file-wide command-table maps made the parent's
        // own later `puts` call look like a call to a deleted builtin and drew
        // a W123 on it — the same "state that is really per-interpreter kept
        // in one flat file-wide map" flaw issue #1141 fixed for the Tk widget
        // hierarchy. Abstain rather than model a second command table: a
        // missed diagnostic inside the child beats a false accusation in the
        // parent. (The interpreter-handle migration above is unaffected — it
        // keys through `qualified_interp_key`, which is already relative to
        // the enclosing frame.)
        if !self.interp_path_stack.is_empty() {
            return false;
        }
        // A rename nested inside a control-flow body may not run at all, so it
        // is not evidence that `OLD` is gone. Recording it anyway produced a
        // W123 on a command that is demonstrably still callable, and — once
        // class liveness started consulting the same facts — withdrew the
        // W308 on objects of that class too.
        //
        // Oracle (tclsh8.6, `review-probes/cls5.tcl`): with `if {0} { rename
        // Dog {} }`, `Dog new` succeeds and `$d fly` fails with `unknown
        // method "fly"`. The analyser proves the same branch dead in the same
        // run — it emits I230 on it — yet honoured the deletion.
        //
        // Only straight-line deletions count. This is the *syntactic* rule:
        // `if {1} { rename Dog {} }` is equally not recorded, even though it
        // does run. Reading the branch's real value needs SCCP, which runs
        // later over the IR this walk feeds, so it is not available here. The
        // narrower rule errs towards treating commands as live, matching the
        // existing "a deletion inside a definition body does not count"
        // rule in `fact_live_for_call` (issue #973).
        if self.control_flow_body_depth > 0 {
            if !new.is_empty() {
                self.renamed_commands.insert(new.clone(), old.clone());
                self.rename_offsets.insert(new.clone(), offset);
                self.result.rename_offsets.insert(new.clone(), offset);
                self.result.renamed_commands.insert(new, old);
            }
            return false;
        }
        if new.is_empty() {
            // `rename OLD {}` destroys the command object — unlike `rename OLD
            // NEW`, which hands it over and leaves every import edge alive
            // (issue #1103).
            self.result
                .destroyed_commands
                .insert(old.clone(), deletion_offset);
            self.deleted_commands.insert(old, deletion_offset);
            return false;
        }
        self.renamed_commands.insert(new.clone(), old.clone());
        self.rename_offsets.insert(new.clone(), offset);
        self.deleted_commands.insert(old.clone(), deletion_offset);
        self.result.rename_offsets.insert(new.clone(), offset);
        self.result.renamed_commands.insert(new, old);
        false
    }

    /// Handle `oo::objdefine $obj …` — record the object variable
    /// (so W308 can suppress unknown-method false positives from
    /// per-instance extensions) **and** walk the per-object
    /// definition so its method bodies are analysed exactly like an
    /// `oo::define` class's.
    ///
    /// `oo::objdefine` shares the `oo::define` member grammar, so the
    /// body / inline forms are parsed with the same helpers.  The
    /// members are collected into a *throwaway* `ClassDef` whose only
    /// purpose is to drive [`Self::parse_oo_definition_body`]'s
    /// method-body walk (variable / command resolution and in-body
    /// diagnostics light up as a side effect).  The `ClassDef` is
    /// deliberately **not** registered in `all_classes`: a per-object
    /// extension is not a class and must never surface in class
    /// listings, hover, rename, or completion.  Its method bodies
    /// home under a private synthetic name so the duplicate detector
    /// never confuses them with the object's real class methods.
    ///
    /// Returns `true` when it owns the command's body walk (an object
    /// name is present), mirroring [`Self::handle_oo_define_command`],
    /// so the generic body recursion does not also descend the body.
    ///
    /// A receiver written as a substitution keeps its **variable** name as
    /// the `objdefined_vars` / `object_methods` key — that is what a
    /// `$obj method` dispatch site is keyed by — but when the word also
    /// folds to a dominating constant (`foreach o {::a ::b} { oo::objdefine
    /// $o { … } }`) the per-object facts are recorded under the *object's*
    /// own name as well, so a bare `::a probe` call sees them too.  tclsh
    /// 9.0.4 / 8.6.16 both confirm every literal element really gains the
    /// method.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::OoObjdefine`].
    pub fn handle_oo_objdefine(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        let (obj_name, resolved_obj) =
            self.oo_objdefine_receiver(args, arg_tokens, arg_single, scope_path);
        if !obj_name.is_empty() {
            self.objdefined_vars.insert(obj_name.clone());
        }
        // A receiver that resolves to no static name (`$objs($i)`, `[pick]`)
        // may define members on *any* object, so the per-object W315 gates
        // abstain file-wide (issue #1170).
        if (obj_name.is_empty() || crate::naming::is_dynamic_word(&obj_name))
            && resolved_obj.is_none()
        {
            self.objdefine_unresolved_receiver = true;
        }

        // `oo::objdefine $obj` with no definition script — the object variable
        // is recorded above; there is nothing more to walk.
        if args.len() < 2 {
            return true;
        }

        // A throwaway holder for the walked members.  The method bodies home
        // under a private synthetic name (`@objdefine@…`, unrepresentable in
        // real Tcl) so a per-object `greet` never collides with the same-named
        // class method in the duplicate detector or the scope-name key.
        let synthetic = if obj_name.is_empty() {
            "::@objdefine@".to_string()
        } else {
            format!("::@objdefine@::{obj_name}")
        };
        let mut object_class = super::types::ClassDef {
            name: obj_name.clone(),
            qualified_name: synthetic,
            ..Default::default()
        };

        // The receiver keys this block records under, and the binding the
        // block belongs to (issue #1170): the innermost proc/method frame is
        // the walk-time spelling of the binding identity consumers re-derive
        // from `ObjectMemberState::anchor_offset`.
        let keys: Vec<String> = [
            Some(obj_name.clone()).filter(|k| !k.is_empty()),
            resolved_obj.clone(),
        ]
        .into_iter()
        .flatten()
        .collect();
        let frame = self.innermost_frame_extent(scope_path);
        let objdefine_offset = arg_tokens.first().map_or(0, |t| t.span.start());
        // Seed the holder with the binding's accumulated per-object table, so
        // this block's retractions and renames run against the state the
        // interpreter would hold — a second block retracting what the first
        // declared removes it silently instead of recording a false abort,
        // which is exactly the hazard that kept W315 out of `oo::objdefine`.
        let (seed, prior_state_conditional) = keys
            .first()
            .and_then(|k| self.objdefine_binding_state(k, frame))
            .map(|st| (st.methods.clone(), st.conditional))
            .unwrap_or_default();
        object_class.methods = seed;

        self.walk_oo_objdefine_form(args, arg_tokens, scope_path, &mut object_class);

        // **W315** candidates (issue #1170). `oo::objdefine` aborts the same
        // way a class definition does (`oo::objdefine $o { deletemethod m }`
        // naming a class-provided `m` errors `method m does not exist` on
        // 9.0.4 and 8.6.14 alike — a per-object retraction reaches only the
        // object's *own* table, never an inherited member).  The seeded walk
        // above judged this block against the binding's accumulated table, so
        // its aborts carry cross-block state; whether each one is *emittable*
        // needs document-wide facts (per-object declarations under any key,
        // receiver creations), so they are held for
        // `flush_objdefine_abort_diagnostics` rather than reported here.
        for abort in std::mem::take(&mut object_class.definition_aborts) {
            self.objdefine_abort_candidates
                .push(super::state::ObjdefineAbortCandidate {
                    abort,
                    receiver: keys.first().cloned().unwrap_or_default(),
                    prior_state_conditional,
                });
        }

        self.fold_objdefine_block(&keys, frame, objdefine_offset, &object_class);

        true
    }

    /// Resolve the source spelling and any statically-known object target of
    /// an `oo::objdefine` receiver. The source spelling remains the dispatch
    /// key, while a resolved loop literal contributes an additional key.
    fn oo_objdefine_receiver(
        &self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> (String, Option<String>) {
        let mut written = args[0].trim().to_string();
        if let Some(stripped) = written.strip_prefix('$') {
            written = stripped.trim_matches(|c| c == '{' || c == '}').to_string();
        }
        let resolved = self
            .resolve_dynamic_word(
                &args[0],
                arg_tokens.first().copied(),
                arg_single.first().copied().unwrap_or(false),
                scope_path,
            )
            .map(|name| name.trim().to_string())
            .filter(|name| {
                !name.is_empty() && *name != written && !crate::naming::is_dynamic_word(name)
            });
        (written, resolved)
    }

    /// Walk an `oo::objdefine` inline member or its braced definition body
    /// through the same registry grammar used by `oo::define`.
    fn walk_oo_objdefine_form(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        object_class: &mut super::types::ClassDef,
    ) {
        const CMD_NAME: &str = "oo::objdefine";
        let inline_form = self
            .definition_grammar(CMD_NAME)
            .is_some_and(|grammar| grammar.is_member(&args[1]));
        if inline_form {
            let inline_args: Vec<String> = args[1..].to_vec();
            let inline_tokens: Vec<Token> = arg_tokens.iter().skip(1).copied().collect();
            if let Some(grammar) = self.definition_grammar(CMD_NAME) {
                if let Some(member) = inline_args
                    .first()
                    .and_then(|subcmd| grammar.member(subcmd))
                {
                    self.emit_w002_oo_member_option_disabled(
                        member,
                        &inline_args[1..],
                        &inline_tokens[1..],
                        false,
                    );
                }
                super::oo::parse_oo_define_inline_in(
                    self,
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    object_class,
                    self.profile.availability_mask,
                );
                self.record_member_command_references(
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    scope_path,
                );
            }
        } else if let Some(body_tok) = arg_tokens.get(1).copied() {
            let grammar = self.definition_grammar(CMD_NAME);
            let definer_disabled = self.command_dialect_disabled(CMD_NAME);
            self.parse_oo_definition_body(
                &args[1],
                body_tok,
                object_class,
                scope_path,
                grammar,
                definer_disabled,
            );
        }
    }

    /// Record one walked `oo::objdefine` block's member declarations and fold
    /// it into each receiver key's durable [`super::types::ObjectMemberState`]
    /// — the home per-object visibility never had (issue #1119 item 3 /
    /// #1170).
    ///
    /// Declarations feed `object_methods` so `$obj m` navigation resolves the
    /// per-object override ahead of a same-named class method, keyed by the
    /// objdefine site's receiver offset — the receiver's *binding identity*,
    /// never the textual tail alone (issue #945 fault 5).  The diff runs per
    /// key against *that key's* own accumulated state: members another block
    /// already recorded for the key are carry-over, not new declarations,
    /// while a `foreach` re-dispatch's per-element key (empty state) still
    /// gains the block's members.  The fold then stores the block's effective
    /// table plus the explicit export/unexport flips, which may name
    /// *class*-provided members the object masks or revives.
    fn fold_objdefine_block(
        &mut self,
        keys: &[String],
        frame: Option<(u32, u32)>,
        objdefine_offset: u32,
        object_class: &super::types::ClassDef,
    ) {
        let block_conditional = self.conditional_depth > 0;
        for key in keys {
            let key_seed = self
                .objdefine_binding_state(key, frame)
                .map(|st| st.methods.clone())
                .unwrap_or_default();
            let mut new_methods: Vec<super::types::MethodDef> = object_class
                .methods
                .values()
                .filter(|m| key_seed.get(&m.name) != Some(*m))
                .cloned()
                .collect();
            // Source-ordered, so the recorded sequence is the same on every
            // run (and the same under each key) rather than the hash map's
            // iteration order.
            new_methods.sort_by_key(|def| def.name_span.start());
            if !new_methods.is_empty() {
                self.record_object_methods(key.clone(), &new_methods, objdefine_offset);
            }
            let state = self.objdefine_binding_state_mut(key, frame, objdefine_offset);
            state.methods.clone_from(&object_class.methods);
            for name in &object_class.exports {
                state.exports.insert(name.clone());
                state.unexports.remove(name);
            }
            for name in &object_class.unexports {
                state.unexports.insert(name.clone());
                state.exports.remove(name);
            }
            state.conditional |= block_conditional;
        }
    }

    /// The accumulated [`super::types::ObjectMemberState`] for `key`'s
    /// binding in `frame`, if any `oo::objdefine` block was walked for it.
    fn objdefine_binding_state(
        &self,
        key: &str,
        frame: Option<(u32, u32)>,
    ) -> Option<&super::types::ObjectMemberState> {
        let idx = *self.objdefine_bindings.get(&(key.to_owned(), frame))?;
        self.result.object_member_state.get(key)?.get(idx)
    }

    /// [`Self::objdefine_binding_state`], creating the binding's entry on
    /// first use with `anchor` as its binding-identity anchor.
    fn objdefine_binding_state_mut(
        &mut self,
        key: &str,
        frame: Option<(u32, u32)>,
        anchor: u32,
    ) -> &mut super::types::ObjectMemberState {
        let states = self
            .result
            .object_member_state
            .entry(key.to_owned())
            .or_default();
        let idx = *self
            .objdefine_bindings
            .entry((key.to_owned(), frame))
            .or_insert_with(|| {
                states.push(super::types::ObjectMemberState {
                    anchor_offset: anchor,
                    ..Default::default()
                });
                states.len() - 1
            });
        &mut states[idx]
    }

    /// The byte extent of the innermost proc / method body scope along
    /// `scope_path` — the walk-time spelling of the binding identity the
    /// LSP side re-derives from a record's anchor offset (the innermost
    /// `Proc`/`Method` frame, or `None` at the top level).
    fn innermost_frame_extent(&self, scope_path: &[usize]) -> Option<(u32, u32)> {
        let mut scope = &self.result.global_scope;
        let mut extent = None;
        for &idx in scope_path {
            let Some(child) = scope.children.get(idx) else {
                break;
            };
            if matches!(
                child.kind,
                super::types::ScopeKind::Proc | super::types::ScopeKind::Method
            ) && let Some(span) = child.body_span
            {
                extent = Some((span.start(), span.end()));
            }
            scope = child;
        }
        extent
    }

    /// **W315**, `oo::objdefine` flavour (issue #1170): report each
    /// definition-aborting word the seeded per-object walks recorded, once
    /// the whole document is walked and the gates below can be judged.
    ///
    /// A per-object table is never complete the way a fresh class body's is —
    /// members can arrive through an aliased handle, `[self]`, or another
    /// file — so every reading here demands *positive, document-wide*
    /// evidence and abstains otherwise:
    ///
    /// * an unresolved `oo::objdefine` receiver anywhere abstains file-wide
    ///   (an unknown object may be any object);
    /// * a `MissingMember` retraction is reported only when the name is
    ///   declared per-object **nowhere** in the document (any key — an alias
    ///   may have declared it) *and* the receiver's construction is in view
    ///   (`instance_classes` / `created_instance_commands`), the per-object
    ///   analogue of the `via_define` completeness gate;
    /// * a `DestinationExists` rename relies on presence evidence, which a
    ///   conditional contributing block makes unprovable;
    /// * a `RenameToItself` is an error against any table state — tclsh
    ///   9.0.4 / 8.6.14: present → `cannot rename method to itself`, absent
    ///   → `method … does not exist` — so only the file-wide receiver gate
    ///   applies.
    pub(super) fn flush_objdefine_abort_diagnostics(&mut self) {
        let candidates = std::mem::take(&mut self.objdefine_abort_candidates);
        if candidates.is_empty() || self.objdefine_unresolved_receiver {
            return;
        }
        let declared_anywhere: std::collections::HashSet<&str> = self
            .result
            .object_methods
            .values()
            .flatten()
            .map(|m| m.def.name.as_str())
            .collect();
        let mut diagnostics = Vec::new();
        for cand in &candidates {
            let emit = match cand.abort.kind {
                super::types::DefinitionAbortKind::RenameToItself => true,
                super::types::DefinitionAbortKind::DestinationExists => {
                    !cand.prior_state_conditional
                }
                super::types::DefinitionAbortKind::MissingMember => {
                    !cand.prior_state_conditional
                        && !declared_anywhere.contains(cand.abort.member.as_str())
                        && (self.result.instance_classes.contains_key(&cand.receiver)
                            || self
                                .result
                                .created_instance_commands
                                .contains(&cand.receiver))
                }
            };
            if emit {
                diagnostics.push(super::types::Diagnostic {
                    code: DiagCode::W315,
                    span: cand.abort.span,
                    message: cand.abort.object_message(),
                    severity: super::types::Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
        self.result.diagnostics.extend(diagnostics);
    }

    /// Handle ``package require`` (and ``package provide``) —
    /// record the package dependency so later passes can gate
    /// W123 (unresolved-command) suppression and dynamic-
    /// provider detection.
    ///
    /// Two shapes are recognised:
    ///
    /// - ``package require ?-exact? NAME ?VERSION?`` — appends a
    ///   ``SignaturePackageRequire`` record to
    ///   ``result.package_requires`` (carrying ``exact`` when the
    ///   flag is present, which narrows the version to the
    ///   degenerate range ``V-V`` for the resolver — issue #1090)
    ///   and flips ``has_dynamic_providers`` when the name argument
    ///   is a ``$``-substitution / ``[…]``-substitution token.
    /// - ``package provide NAME ?VERSION?`` — consumed silently;
    ///   there's no field to record it on yet.
    ///
    /// The conditional flag is ``self.conditional_depth > 0``.
    ///
    /// `cmd_tok` is the command-head token (the ``package``
    /// word).  The recorded
    /// [`SignaturePackageRequire::range`](crate::signature_scan::types::SignaturePackageRequire::range)
    /// uses its span so code-action / quick-fix UX points at the
    /// ``package`` keyword rather than at the ``require``
    /// subcommand word.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackageRequire`] (stamped
    /// on `package`'s `require` subcommand — other ``package``
    /// subcommands like ``ifneeded`` / ``forget`` / ``vsatisfies``
    /// carry no hook and aren't recorded); `args[0]` is still the
    /// subcommand word.
    pub fn handle_package_require(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        if args.len() < 2 {
            return;
        }
        // ``package require -exact NAME ?VERSION?`` —
        // record the flag and shift the name index.
        let exact = args[1] == "-exact" && args.len() >= 3;
        let (name_idx, name_text) = if exact {
            (2usize, args[2].clone())
        } else {
            (1usize, args[1].clone())
        };
        let version_idx = name_idx + 1;
        let version = if version_idx < args.len() {
            Some(args[version_idx].clone())
        } else {
            None
        };

        // Dynamic-provider detection — non-literal package
        // name suppresses W123 unknown-command emission
        // because the dynamic provider may register the
        // missing command at runtime.
        if let Some(name_tok) = arg_tokens.get(name_idx)
            && (matches!(name_tok.kind, TokenType::Var | TokenType::Cmd)
                || name_text.contains('$')
                || name_text.contains('['))
        {
            self.result.has_dynamic_providers = true;
        }

        self.result
            .package_requires
            .push(crate::signature_scan::types::SignaturePackageRequire {
                name: name_text,
                version,
                exact,
                range: cmd_tok.span,
                conditional: self.conditional_depth > 0,
            });
    }

    /// Record ``package provide NAME ?VERSION?``.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackageProvide`] (stamped
    /// on `package`'s `provide` subcommand).  See
    /// [`Self::handle_package_require`] for the `cmd_tok` anchoring
    /// convention.
    pub fn handle_package_provide(&mut self, cmd_tok: Token, args: &[String]) {
        if args.len() < 2 {
            return;
        }
        let name = args[1].clone();
        let version = if args.len() >= 3 {
            Some(args[2].clone())
        } else {
            None
        };
        self.result
            .package_provides
            .push(super::types::PackageProvide {
                name,
                version,
                range: cmd_tok.span,
                conditional: self.conditional_depth > 0,
            });
    }

    /// Record ``package ifneeded NAME VERSION ?SCRIPT?``.
    ///
    /// Only the setter form is a record: ``package ifneeded NAME
    /// VERSION`` with no script is a *query* that registers nothing,
    /// so treating it as a registration would abstain a
    /// package-derived load order on a document that merely asked
    /// what was registered.
    ///
    /// The script is not kept — see
    /// [`PackageIfneeded`](super::types::PackageIfneeded) for why the
    /// bare fact of a registration is the useful part.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackageIfneeded`]
    /// (stamped on `package`'s `ifneeded` subcommand).  See
    /// [`Self::handle_package_require`] for the `cmd_tok` anchoring
    /// convention.
    pub fn handle_package_ifneeded(&mut self, cmd_tok: Token, args: &[String]) {
        // args[0] is the `ifneeded` subcommand word; the setter form
        // is NAME + VERSION + SCRIPT.
        if args.len() < 4 {
            return;
        }
        self.result
            .package_ifneededs
            .push(super::types::PackageIfneeded {
                name: args[1].clone(),
                version: args[2].clone(),
                range: cmd_tok.span,
            });
    }

    /// Record ``package prefer latest`` — the one form of ``package
    /// prefer`` that changes the interpreter's version-selection rule
    /// (issue #1126 item 1).
    ///
    /// ``package prefer`` with no argument is a query, and ``package
    /// prefer stable`` never changes anything: it is a no-op from the
    /// default and silently ineffective once ``latest`` has been set
    /// (see [`crate::signature_scan::types::SignaturePackagePrefer`]
    /// for the transcript), so neither is recorded and the state is
    /// the monotone "has a raise already run".
    ///
    /// A non-literal mode word (``package prefer $mode``) is skipped
    /// rather than guessed at — flipping the selection rule on a
    /// substitution would silently move go-to-definition onto a
    /// different release of a package.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackagePrefer`] (stamped
    /// on `package`'s `prefer` subcommand).  See
    /// [`Self::handle_package_require`] for the `cmd_tok` anchoring
    /// convention.
    pub fn handle_package_prefer(&mut self, cmd_tok: Token, args: &[String]) {
        if args.len() < 2 || args[1] != "latest" {
            return;
        }
        self.result.package_prefer_latest.push(
            crate::signature_scan::types::SignaturePackagePrefer {
                range: cmd_tok.span,
                conditional: self.conditional_depth > 0,
            },
        );
    }

    /// Resolve a command alias to `(target_cmd, effective_args)`.
    ///
    /// Returns `(cmd_name, args)` unchanged if no alias matches;
    /// otherwise returns the target command and the prepended-args +
    /// original args list. Delegates to `crate::alias::resolve_alias` for the
    /// namespace-aware lookup.
    #[must_use]
    pub fn resolve_alias(
        &mut self,
        cmd_name: &str,
        args: &[String],
        scope_path: &[usize],
    ) -> (String, Vec<String>) {
        // An alias is looked up the way any unqualified command is, so the
        // lookup namespace is the command-resolution one (issue #923 idx 85).
        let ns = self.command_resolution_namespace(scope_path);
        // `alias::resolve_alias` accepts `CommandAliasMap` (alias map
        // keyed by qualified alias name) — the same shape as
        // `self.command_aliases` already uses.
        if let Some((target_cmd, prepended)) = resolve_alias(cmd_name, &self.command_aliases, &ns) {
            let mut effective: Vec<String> = prepended;
            effective.extend(args.iter().cloned());
            (target_cmd, effective)
        } else {
            (cmd_name.to_string(), args.to_vec())
        }
    }

    /// The definition-body grammar attached to `cmd_name`'s spec, if it is a
    /// definer command.  Registry-sourced, so member recognition / argument
    /// structure in the body walkers never hardcodes a keyword list — a new
    /// definer is picked up the moment its spec carries a `definition_body`.
    pub(super) fn definition_grammar(
        &self,
        cmd_name: &str,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        self.registry
            .as_deref()
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .and_then(|s| s.definition_body)
    }

    /// The [`ClassFactory`] `class` publishes, when `class` is itself a
    /// `TclOO` class factory — a class whose superclass chain reaches a
    /// registry metaclass.
    ///
    /// Derived **once, where the metaclass is written**, so every consumer
    /// (this file's own `Meta create …` calls and, through the workspace
    /// factory index, another file's) classifies a creation call from the
    /// same proved fact instead of re-deriving it — or, cross-file,
    /// abstaining for want of it (issue #1276).
    ///
    /// A user metaclass that does not override a manufacturer subcommand
    /// inherits `oo::class`'s own `create Name Body` shape, so that
    /// subcommand simply has no entry and the builtin layout applies.  One
    /// that *does* override it declares its own shape in the override's
    /// parameter list, and that override is read rather than guessed: Tk's
    /// `self method create {name superclasses body}` puts the body at
    /// argument 3, not 2, and splices a superclass the caller never wrote
    /// (issue #923 idx 96/97).
    fn class_factory_of(&self, qualified: &str, class: &ClassDef) -> Option<ClassFactory> {
        let meta = self.user_metaclass_of_class(qualified, class)?;
        let overrides = class
            .class_methods
            .iter()
            .filter_map(|(subcommand, override_def)| {
                let builtin = meta.grammar.manufacturer(subcommand)?;
                let default_positions = Some((
                    usize::from(builtin.names_instance_at?),
                    usize::from(builtin.definition_body_at?),
                ));
                let positions = self.manufacturer_word_positions(override_def);
                let (name_arg, body_arg) = positions.or(default_positions)?;
                let injected = positions
                    .and_then(|_| self.manufacturer_injected_template(&meta, override_def));
                Some((
                    subcommand.clone(),
                    super::types::ManufacturerSpec {
                        name_arg,
                        body_arg,
                        injected,
                    },
                ))
            })
            .collect();
        let unknown_binds_instance = self.unknown_dispatch_binds_instance(&meta, class);
        let exported_manufacturers = meta
            .grammar
            .manufacturers
            .iter()
            .filter(|method| {
                !class.class_unexports.contains(method.keyword)
                    && (method.visibility == tcl_registry::definer::MemberVisibility::Exported
                        || class.class_exports.contains(method.keyword))
            })
            .map(|method| method.keyword.to_owned())
            .collect();
        Some(ClassFactory {
            root_metaclass: meta.root_command,
            overrides,
            exported_manufacturers,
            unknown_binds_instance,
        })
    }

    /// Whether this metaclass's **unrecognised-word fallback member** proves
    /// that calling one of the classes it makes with a bare word both
    /// constructs an object and hands that same word back — Tk's
    /// `::tk::IconList .il` idiom (issue #1303).
    ///
    /// Two facts have to hold, and both are read off the metaclass's own
    /// body, where they are provable:
    ///
    /// 1. it declares the family's unknown-dispatch member
    ///    ([`DefinitionBodyGrammar::unknown_dispatch_method`], registry data —
    ///    `unknown` for `TclOO`) with at least one parameter, so an
    ///    unrecognised first word reaches code at all; and
    /// 2. that body **constructs** an object named from the first parameter
    ///    (a manufacturer call — again registry data — whose instance-name
    ///    word interpolates the parameter) **and** completes with exactly
    ///    that parameter, proved through
    ///    [`tcl_registry::ArgRole::Result`] rather than assumed.
    ///
    /// Fact 2 is the one the issue insists on: an `unknown` that returns
    /// something else — `return [self]`, a formatted string, a bare `next`
    /// fall-through — must **not** be read this way, because the value the
    /// caller binds is then not an object handle at all. Every reachable
    /// normal-result path must return the first parameter after constructing
    /// it; a different result collapses the proof to `false`, and a body with
    /// no such path proves nothing. Registry completion semantics distinguish
    /// that normal result from `return -code error`, `error`, loop transfer,
    /// and other terminating completions.
    ///
    /// Deliberate residuals, both in the abstaining direction except where
    /// noted:
    ///
    /// * a body that also dispatches through `next` (Tk's does, for a word
    ///   that is not a widget path) is accepted only when the actual class MRO
    ///   proves that the next provider terminates. A visible user provider
    ///   that returns normally invalidates the proof; an incomplete hierarchy
    ///   abstains. At a proved built-in chain end, completion comes from the
    ///   definition grammar rather than from the dispatch keyword;
    /// * a construction written through a variable holding the name
    ///   (`set n $w; [self] create $n`) is not followed, and abstains;
    /// * the member is looked up on the **metaclass**, because it is
    ///   *instances* of the metaclass — the manufactured classes — that
    ///   dispatch through it. An `unknown` on an ordinary class governs that
    ///   class's own instances, which is a different question this does not
    ///   answer.
    ///
    /// [`DefinitionBodyGrammar::unknown_dispatch_method`]: tcl_registry::definer::DefinitionBodyGrammar::unknown_dispatch_method
    fn unknown_dispatch_binds_instance(&self, meta: &UserMetaclass, class: &ClassDef) -> bool {
        let Some(keyword) = meta.grammar.unknown_dispatch_method else {
            return false;
        };
        let Some(member) = class.methods.get(keyword) else {
            return false;
        };
        if member.params_computed {
            return false;
        }
        let Some(word_param) = member.params.first().map(|p| p.name.as_str()) else {
            return false;
        };
        let Some(proof) = self.unknown_script_path_proof(
            meta,
            class,
            member.body_span,
            word_param,
            vec![false],
            0,
        ) else {
            return false;
        };
        proof.proved_return && !proof.invalid_return && proof.fallthrough_constructed.is_empty()
    }

    fn unknown_script_path_proof(
        &self,
        meta: &UserMetaclass,
        class: &ClassDef,
        body_span: Span,
        word_param: &str,
        mut states: Vec<bool>,
        depth: u32,
    ) -> Option<UnknownPathProof> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let registry = self.registry.as_deref()?;
        let mut proof = UnknownPathProof::default();
        for seg in self.direct_statements_in_span(body_span)? {
            if states.is_empty() {
                break;
            }
            if registry.get(seg.name()).and_then(|spec| spec.lowering_hook)
                == Some(tcl_registry::hooks::LoweringHookId::If)
            {
                let (body_spans, has_fallthrough) =
                    self.possible_if_body_spans(&seg, word_param)?;
                let mut next_states = Vec::new();
                for state in states {
                    for span in &body_spans {
                        let nested = self.unknown_script_path_proof(
                            meta,
                            class,
                            *span,
                            word_param,
                            vec![state],
                            depth + 1,
                        )?;
                        proof.proved_return |= nested.proved_return;
                        proof.invalid_return |= nested.invalid_return;
                        next_states.extend(nested.fallthrough_constructed);
                    }
                    if has_fallthrough {
                        next_states.push(state);
                    }
                }
                states = next_states;
                continue;
            }
            if registry
                .get(seg.name())
                .and_then(|spec| spec.lowering_hook)
                .is_some_and(|hook| {
                    matches!(
                        hook,
                        tcl_registry::hooks::LoweringHookId::Switch
                            | tcl_registry::hooks::LoweringHookId::Try
                    )
                })
            {
                return None;
            }
            match self.unknown_body_evidence(meta, class, &seg, word_param) {
                UnknownBodyEvidence::Constructs => states.fill(true),
                UnknownBodyEvidence::ReturnsWord => {
                    proof.invalid_return |= states.iter().any(|constructed| !constructed);
                    proof.proved_return |= states.iter().any(|constructed| *constructed);
                    states.clear();
                }
                UnknownBodyEvidence::ReturnsSomethingElse => {
                    proof.invalid_return = true;
                    states.clear();
                }
                UnknownBodyEvidence::Terminates => states.clear(),
                UnknownBodyEvidence::Nothing => {}
            }
        }
        proof.fallthrough_constructed = states;
        Some(proof)
    }

    fn possible_if_body_spans(
        &self,
        seg: &SegmentedCommand,
        _word_param: &str,
    ) -> Option<(Vec<Span>, bool)> {
        let registry = self.registry.as_deref()?;
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        if registry.invocation_completion(seg.name(), &args, self.profile.availability_mask)
            != tcl_registry::registry::InvocationCompletion::FallsThrough
            || registry.control_invocation_valid(seg.name(), &args, self.profile.availability_mask)
                != Some(true)
        {
            return None;
        }
        let exprs = registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Expr);
        let bodies = registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
        if bodies.iter().any(|body| {
            registry.control_arm_semantics(seg.name(), &args, *body)
                != Some(tcl_registry::registry::ControlArmSemantics::Selected)
        }) {
            return None;
        }
        let mut all_known = true;
        let mut selected = None;
        for (branch, expr_idx) in exprs.iter().copied().enumerate() {
            match self.literal_boolean_expr(seg.args().get(expr_idx)?) {
                Some(true) => {
                    selected = bodies.get(branch).copied();
                    break;
                }
                Some(false) => {}
                None => {
                    all_known = false;
                    break;
                }
            }
        }
        if all_known {
            if selected.is_none() && bodies.len() > exprs.len() {
                selected = bodies.last().copied();
            }
            let spans = selected
                .and_then(|idx| seg.arg_tokens().get(idx).map(|token| token.span))
                .into_iter()
                .collect();
            return Some((spans, selected.is_none()));
        }
        let spans = bodies
            .iter()
            .filter_map(|idx| seg.arg_tokens().get(*idx).map(|token| token.span))
            .collect();
        Some((spans, bodies.len() <= exprs.len()))
    }

    fn literal_boolean_expr(&self, word: &str) -> Option<bool> {
        if !crate::var_refs::scan_var_ref_forms(word).is_empty() || word.contains('[') {
            return None;
        }
        let expr = word
            .trim()
            .strip_prefix('{')
            .and_then(|value| value.strip_suffix('}'))
            .unwrap_or(word.trim());
        crate::static_loops::evaluate_expr_with_constants(
            &crate::parse_expr(expr, Some(self.dialect())),
            &crate::static_loops::StaticEnv::new(),
            crate::tcl_expr_eval::FoldPolicy::default(),
        )
        .map(|value| value != 0)
    }

    /// Resolve a method-chain continuation from the current metaclass's
    /// actual linearised hierarchy. A user-defined next provider is judged
    /// from its body; only a proved chain end may use the registry-declared
    /// completion of the class system's built-in fallback.
    fn next_chain_terminates(
        &self,
        meta: &UserMetaclass,
        class: &ClassDef,
        args: &[&str],
        anchor_arg: Option<usize>,
    ) -> Option<bool> {
        if class.inheritance_unknown {
            return None;
        }
        let method = meta.grammar.unknown_dispatch_method?;
        let mut classes = self.result.all_classes.clone();
        classes.insert(class.qualified_name.clone(), class.clone());
        let hierarchy = super::class_hierarchy::build_class_hierarchy(classes.clone());
        if !hierarchy.errors.is_empty() {
            return None;
        }
        if hierarchy.method_target(&class.qualified_name, method)
            != Some(class.qualified_name.as_str())
        {
            return None;
        }
        let mro = hierarchy.mro_map.get(&class.qualified_name)?;
        let current = mro.iter().position(|name| name == &class.qualified_name)?;
        let scan_from = if let Some(anchor_arg) = anchor_arg {
            let written = args.get(anchor_arg)?;
            if crate::naming::is_dynamic_word(written) {
                return None;
            }
            let namespace = class
                .qualified_name
                .rsplit_once("::")
                .map_or("", |(namespace, _)| namespace);
            let qualified = crate::naming::qualify(namespace, written);
            let anchor = mro.iter().position(|name| name == &qualified)?;
            (anchor > current).then_some(anchor)?
        } else {
            current + 1
        };
        for (index, ancestor) in mro.iter().enumerate().skip(scan_from) {
            if let Some(provider) = classes.get(ancestor) {
                if let Some(definition) = provider.methods.get(method) {
                    return self.method_body_terminates(definition);
                }
                continue;
            }
            if ancestor.trim_start_matches(':') == meta.root_command.trim_start_matches(':') {
                return Self::builtin_next_completion(meta.grammar, method, index, mro.len());
            }
            return None;
        }
        None
    }

    fn builtin_next_completion(
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        method: &str,
        provider_index: usize,
        mro_len: usize,
    ) -> Option<bool> {
        (provider_index + 1 == mro_len).then_some(grammar.builtin_method_terminates(method))
    }

    fn method_body_terminates(&self, method: &super::types::MethodDef) -> Option<bool> {
        let registry = self.registry.as_deref()?;
        for seg in self.direct_statements_in_span(method.body_span)? {
            let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            match registry.invocation_completion(seg.name(), &args, self.profile.availability_mask)
            {
                tcl_registry::registry::InvocationCompletion::FallsThrough => {
                    let controls = self.control_arms_for_segment(&seg);
                    if !controls.complete || !controls.arms.is_empty() {
                        return None;
                    }
                }
                tcl_registry::registry::InvocationCompletion::ReturnsResult(_) => {
                    return Some(false);
                }
                tcl_registry::registry::InvocationCompletion::Terminates => return Some(true),
                tcl_registry::registry::InvocationCompletion::Unknown => return None,
            }
        }
        Some(false)
    }

    /// What one statement of an unknown-dispatch body contributes to
    /// [`Self::unknown_dispatch_binds_instance`]'s proof.
    fn unknown_body_evidence(
        &self,
        meta: &UserMetaclass,
        class: &ClassDef,
        seg: &SegmentedCommand,
        word_param: &str,
    ) -> UnknownBodyEvidence {
        let Some(registry) = self.registry.as_ref() else {
            return UnknownBodyEvidence::Nothing;
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        // A compound self receiver is not itself a registry command head, so
        // classify its registry-described manufacturer method before asking
        // for ordinary command completion. The full dispatch shape remains
        // typed by the self-receiver hook and the definition grammar.
        let head = seg.name();
        let receiver_is_self = registry
            .method_dispatch_keyword(head)
            .is_some_and(|kind| kind == tcl_registry::registry::MethodDispatchKind::SelfDispatch)
            || bracketed_self_receiver(registry, head);
        if receiver_is_self
            && let Some(manufacturer) = args.first().and_then(|kw| meta.grammar.manufacturer(kw))
        {
            let Some(name_arg) = manufacturer.names_instance_at else {
                return UnknownBodyEvidence::Nothing;
            };
            // The instance-name word must be exactly the fallback's own first
            // parameter, optionally with Tcl's global namespace qualifier.
            return if args.get(name_arg as usize).is_some_and(|word| {
                let name_word = word.strip_prefix("::").unwrap_or(word);
                crate::value_shapes::whole_word_scalar_var_name(name_word) == Some(word_param)
            }) {
                UnknownBodyEvidence::Constructs
            } else {
                UnknownBodyEvidence::Nothing
            };
        }
        let completion =
            registry.invocation_completion(seg.name(), &args, self.profile.availability_mask);
        if registry.method_dispatch_keyword(seg.name())
            == Some(tcl_registry::registry::MethodDispatchKind::NextChain)
        {
            if completion != tcl_registry::registry::InvocationCompletion::FallsThrough {
                return UnknownBodyEvidence::ReturnsSomethingElse;
            }
            let anchor_arg = registry
                .arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Name)
                .into_iter()
                .next();
            return self
                .next_chain_terminates(meta, class, &args, anchor_arg)
                .map_or(UnknownBodyEvidence::ReturnsSomethingElse, |terminates| {
                    if terminates {
                        UnknownBodyEvidence::Terminates
                    } else {
                        UnknownBodyEvidence::ReturnsSomethingElse
                    }
                });
        }
        match completion {
            tcl_registry::registry::InvocationCompletion::ReturnsResult(Some(idx)) => {
                let Some(word) = args.get(idx) else {
                    return UnknownBodyEvidence::ReturnsSomethingElse;
                };
                return if crate::value_shapes::whole_word_scalar_var_name(word) == Some(word_param)
                {
                    UnknownBodyEvidence::ReturnsWord
                } else {
                    UnknownBodyEvidence::ReturnsSomethingElse
                };
            }
            tcl_registry::registry::InvocationCompletion::ReturnsResult(None)
            | tcl_registry::registry::InvocationCompletion::Unknown => {
                return UnknownBodyEvidence::ReturnsSomethingElse;
            }
            tcl_registry::registry::InvocationCompletion::Terminates => {
                return UnknownBodyEvidence::Terminates;
            }
            tcl_registry::registry::InvocationCompletion::FallsThrough => {}
        }
        UnknownBodyEvidence::Nothing
    }

    /// Registry-driven recursive script-block collection for broad structural
    /// scans that do not claim control-flow reachability.
    fn collect_script_blocks(
        &self,
        text: &str,
        base: u32,
        depth: u32,
        out: &mut Vec<Vec<SegmentedCommand>>,
    ) {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return;
        }
        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let block = crate::segmenter::segment_commands_with_offset_and_config(
            text,
            base,
            self.lexer_config(),
        );
        for seg in &block {
            let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            for idx in tcl_registry::ArgRole::ALL
                .iter()
                .filter(|role| role.carries_script())
                .flat_map(|&role| registry.arg_indices_for_role(seg.name(), &args, role))
            {
                let (Some(word), Some(tok)) = (seg.args().get(idx), seg.argv.get(idx + 1)) else {
                    continue;
                };
                // Only a braced word is a script whose text is exactly what
                // runs; a substituted or quoted one is composed at run time
                // and re-segmenting its written form would read code Tcl
                // never evaluates.
                if tok.kind != TokenType::Str {
                    continue;
                }
                let inner_base = tok
                    .span
                    .start()
                    .saturating_add(u32::from(tok.content_offset));
                self.collect_script_blocks(word, inner_base, depth + 1, out);
            }
        }
        out.push(block);
    }

    /// Record the classes a proc manufactures under a **computed name** whose
    /// value a literal call-site argument proves (issue #1306).
    ///
    /// A namespace-normalising factory commonly creates its metaclass as
    /// `Base create ${namespace}::class {…}` inside a procedure. The name
    /// word is written dynamically, so the ordinary walk abstains and the
    /// metaclass never enters the class-factory index. A literal load-time
    /// call can nevertheless prove that computed absolute name.
    ///
    /// This pass follows exactly that: a **literal argument at a load-time
    /// call site**, through the parameter binding, into the creation call's
    /// name word. It is the const/literal propagation the `foreach`
    /// simulation already does for a literal list
    /// ([`Analyser::simulate_remaining_foreach_iterations`]), with the call
    /// site rather than the list supplying the values. The ordinary walk's
    /// registry-classified placeholder supplies the already-parsed class
    /// structure; this pass changes only its proved identity and re-derives
    /// identity-dependent factory facts against the completed class lattice.
    ///
    /// **What it proves, and what it does not.** Each literal call site is
    /// independent evidence: `mkdialect ::T::D` really does create
    /// `::T::D::class`, and a *different* call site passing `$x` neither adds
    /// to nor subtracts from that. So a mixed set of call sites yields
    /// exactly the classes the literal ones prove, and nothing for the
    /// others — no guess anywhere. What is deliberately **not** claimed:
    ///
    /// * a call site the pass cannot read as naming this proc (a bare
    ///   relative spelling resolved through a `namespace path`, a
    ///   `{*}`-expanded argument list, an `interp alias`) contributes
    ///   nothing, so the class it would have proved stays unrecorded — the
    ///   abstaining direction;
    /// * only **load-time** call sites count. A call written inside another
    ///   proc's or class's body runs at *call* time, if ever, and the same
    ///   `offset_is_inside_any_definition_body` rule the load-level
    ///   destruction filter uses applies here;
    /// * the resolved name must be **absolute**. A relative one would have to
    ///   be homed against the call site's namespace, which this pass does not
    ///   model, so it abstains instead of homing it wrongly;
    /// * a name word carrying a command substitution, a backslash escape, or
    ///   an unbound interpolation resolves to nothing;
    /// * nested control-flow bodies are retained as paths. Typed registry
    ///   hooks select `if`/`switch` arms and the unconditional parts of
    ///   `try`; a dynamic condition, handler-only path, loop, or unsupported
    ///   control shape abstains instead of flattening its body into the
    ///   unconditional stream.
    ///
    /// No command is named here and no framework is special-cased: the
    /// manufacturer words come from the registry
    /// ([`tcl_registry::CommandRegistry::is_manufacturer_method`]) and the
    /// creation's structural template exists only when the ordinary generic
    /// handler classified it as a class creation.
    pub(super) fn record_literal_parameter_definitions(&mut self) {
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let candidates = self.parameterised_creation_sites(registry);
        if candidates.is_empty() {
            return;
        }
        let calls = self.load_time_call_arguments();
        for candidate in candidates {
            let Some(values) = calls.get(&candidate.proc_qname) else {
                continue;
            };
            // The ordinary source walk has already parsed this creation body
            // under its unresolved written name.  Re-dispatching the same
            // source span can legitimately be deduplicated by the body
            // walker, so retain that structural record and transplant it
            // onto every name the call-site provenance proves.
            let Some(template) = self.unresolved_class(&candidate).cloned() else {
                continue;
            };
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for call in values {
                if !self.load_time_call_is_reachable(call) {
                    continue;
                }
                let Some(resolved) =
                    self.resolve_parameterised_creation_name(&candidate, &call.args)
                else {
                    continue;
                };
                // Only an absolute name is homed without a namespace model —
                // see the doc comment.
                if !resolved.starts_with("::") || !seen.insert(resolved.clone()) {
                    continue;
                }
                self.materialise_parameterised_class(&resolved, &template);
            }
            if !seen.is_empty() {
                self.retract_unresolved_class(&candidate);
            }
        }
    }

    fn load_time_call_is_reachable(&mut self, call: &LoadTimeCall) -> bool {
        let mut body_span = Span::new(0, u32::try_from(self.source.len()).unwrap_or(u32::MAX));
        let env = std::collections::HashMap::new();
        for arm in &call.control_path {
            if !self.static_prefix_falls_through(body_span, arm.controller.span.start())
                || self.control_body_is_selected("", arm, &env, 0) != Some(true)
            {
                return false;
            }
            body_span = arm.body_span;
        }
        self.static_prefix_falls_through(body_span, call.call_off)
    }

    fn static_prefix_falls_through(&mut self, body_span: Span, stop_off: u32) -> bool {
        if self.registry.is_none() {
            return false;
        }
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return false;
        };
        for seg in statements
            .into_iter()
            .take_while(|seg| seg.span.start() < stop_off)
        {
            if !self.static_statement_falls_through(&seg, 0) {
                return false;
            }
        }
        true
    }

    /// Prove that a statically declared procedure call completes normally.
    /// `return` is normal completion at the caller, while registry-declared
    /// non-normal completions and unresolved statements make the proof
    /// abstain. Calls through a recorded class-factory oracle are ordinary
    /// fallthrough statements when their exported manufacturer layout is
    /// valid; no factory command spelling is known here.
    fn static_user_proc_call_falls_through(
        &mut self,
        caller: &str,
        seg: &SegmentedCommand,
        depth: u32,
        stack: &mut Vec<String>,
    ) -> Option<bool> {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let qname = self.resolve_static_proc_name(caller, seg.name())?;
        if stack.iter().any(|active| active == &qname) {
            return None;
        }
        let proc_def = self.result.all_procs.get(&qname)?.clone();
        if proc_def.params_computed {
            return None;
        }
        let actuals: Vec<Option<String>> = seg.args().iter().cloned().map(Some).collect();
        bind_proc_formals(&proc_def.params, &actuals)?;
        stack.push(qname.clone());
        let result = self.static_proc_body_falls_through(&qname, proc_def.body_span, depth, stack);
        stack.pop();
        result
    }

    fn static_proc_body_falls_through(
        &mut self,
        qname: &str,
        body_span: Span,
        depth: u32,
        stack: &mut Vec<String>,
    ) -> Option<bool> {
        let registry = self.registry.clone()?;
        for seg in self.direct_statements_in_span(body_span)? {
            let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            match registry.invocation_completion(seg.name(), &args, self.profile.availability_mask)
            {
                tcl_registry::registry::InvocationCompletion::ReturnsResult(_) => {
                    return Some(true);
                }
                tcl_registry::registry::InvocationCompletion::Terminates => return Some(false),
                tcl_registry::registry::InvocationCompletion::Unknown => {
                    if registry.get(seg.name()).is_some() {
                        return None;
                    }
                    if self.known_factory_invocation_falls_through(qname, &seg) {
                        continue;
                    }
                    if self.static_user_proc_call_falls_through(qname, &seg, depth + 1, stack)? {
                        continue;
                    }
                    return Some(false);
                }
                tcl_registry::registry::InvocationCompletion::FallsThrough => {
                    if !self.static_statement_falls_through(&seg, depth + 1) {
                        return None;
                    }
                }
            }
        }
        Some(true)
    }

    fn known_factory_invocation_falls_through(&self, caller: &str, seg: &SegmentedCommand) -> bool {
        if crate::naming::is_dynamic_word(seg.name()) {
            return false;
        }
        let namespace = caller
            .rsplit_once("::")
            .map_or("", |(namespace, _)| namespace);
        let candidates = crate::naming::bareword_resolution_candidates(namespace, seg.name());
        let Some(factory) = self.class_factory_for_candidates(&candidates, seg.arg_tokens()) else {
            return false;
        };
        let Some(method) = seg.args().first() else {
            return false;
        };
        if !factory.exported_manufacturers.contains(method) {
            return false;
        }
        let Some(grammar) = self.definition_grammar(&factory.root_metaclass) else {
            return false;
        };
        let Some(manufacturer) = grammar.manufacturer(method) else {
            return false;
        };
        manufacturer_layout(&factory, manufacturer, seg.args(), seg.arg_tokens())
            .is_some_and(|layout| layout.name_arg < seg.args().len())
    }

    fn static_statement_falls_through(&mut self, seg: &SegmentedCommand, depth: u32) -> bool {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return false;
        }
        let Some(registry) = self.registry.as_deref() else {
            return false;
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        match registry.invocation_completion(seg.name(), &args, self.profile.availability_mask) {
            tcl_registry::registry::InvocationCompletion::FallsThrough => {}
            tcl_registry::registry::InvocationCompletion::Unknown
                if registry.get(seg.name()).is_none() =>
            {
                if self.known_factory_invocation_falls_through("", seg) {
                    return true;
                }
                return self
                    .static_user_proc_call_falls_through("", seg, depth + 1, &mut Vec::new())
                    .unwrap_or(false);
            }
            _ => return false,
        }
        let is_control = registry
            .invocation_traits(seg.name(), &args, self.profile.availability_mask)
            .contains(tcl_registry::Traits::CONTROL_FLOW);
        if is_control
            && registry.control_invocation_valid(seg.name(), &args, self.profile.availability_mask)
                != Some(true)
        {
            return false;
        }
        let controls = self.control_arms_for_segment(seg);
        if !controls.complete {
            return false;
        }
        let arms = controls.arms;
        if arms.is_empty() {
            return true;
        }

        let mut selected = Vec::new();
        let mut saw_typed_arm = false;
        let mut saw_propagating_arm = false;
        for arm in arms {
            let Some(semantics) = self.control_arm_semantics_for(&arm) else {
                if is_control {
                    return false;
                }
                continue;
            };
            saw_typed_arm = true;
            match semantics {
                tcl_registry::registry::ControlArmSemantics::Always
                | tcl_registry::registry::ControlArmSemantics::FrameBoundary => {
                    saw_propagating_arm = true;
                    if !self.static_prefix_falls_through(arm.body_span, arm.body_span.end()) {
                        return false;
                    }
                }
                tcl_registry::registry::ControlArmSemantics::Selected => selected.push(arm),
                tcl_registry::registry::ControlArmSemantics::CompletionBoundary => {}
                tcl_registry::registry::ControlArmSemantics::Uncertain => return false,
            }
        }
        if !saw_typed_arm || selected.is_empty() {
            return true;
        }
        if saw_propagating_arm {
            return false;
        }

        let env = std::collections::HashMap::new();
        let mut selected_body = None;
        let mut selection_unknown = false;
        for arm in selected {
            match self.control_body_is_selected("", &arm, &env, depth + 1) {
                Some(true) if selected_body.is_none() => selected_body = Some(arm.body_span),
                Some(true) => return false,
                Some(false) => {}
                None => {
                    selection_unknown = true;
                    if !self.static_prefix_falls_through(arm.body_span, arm.body_span.end()) {
                        return false;
                    }
                }
            }
        }
        if selection_unknown {
            return true;
        }
        selected_body.is_none_or(|span| self.static_prefix_falls_through(span, span.end()))
    }

    /// The placeholder record made by the ordinary walk for `candidate`.
    ///
    /// A dynamic written name is intentionally retained until at least one
    /// load-time call proves a real name.  Besides making the earlier walk
    /// visible to this pass, the record is the authoritative parse of the
    /// creation body's members: a second walk over the identical source span
    /// may be suppressed by definition-body deduplication.
    fn unresolved_class(&self, candidate: &ParameterisedCreation) -> Option<&ClassDef> {
        let unresolved = Self::unresolved_class_name(candidate)?;
        self.result.all_classes.get(&unresolved)
    }

    /// Move the already-parsed structure of a computed-name creation onto a
    /// name proved by a literal call site, then derive identity-dependent
    /// facts against the now-complete class lattice.
    fn materialise_parameterised_class(&mut self, resolved: &str, template: &ClassDef) {
        let mut observed = template.clone();
        crate::naming::key_tail(resolved).clone_into(&mut observed.name);
        resolved.clone_into(&mut observed.qualified_name);
        let (mut class, compatible) = if let Some(existing) = self.result.all_classes.get(resolved)
        {
            join_parameterised_class_observations(existing, &observed).map_or_else(
                || {
                    let mut abstaining = existing.clone();
                    abstaining.factory = None;
                    abstaining.inheritance_unknown = true;
                    (abstaining, false)
                },
                |joined| (joined, true),
            )
        } else {
            (observed, true)
        };
        if compatible {
            class.factory = self.class_factory_of(resolved, &class);
        }

        let simple = class.name.clone();
        if !self
            .result
            .class_body_spans
            .iter()
            .any(|(qname, span)| qname == resolved && *span == class.body_span)
        {
            self.result
                .class_body_spans
                .push((resolved.to_owned(), class.body_span));
        }
        self.result
            .all_classes
            .insert(resolved.to_owned(), class.clone());
        self.result.global_scope.classes.insert(simple, class);
    }

    /// Resolve one computed creation name by executing the small, proven
    /// constant/provenance slice that dominates it inside its procedure.
    ///
    /// Direct parameter interpolation is the zero-statement case.  Local
    /// assignments may additionally call a user procedure when that call can
    /// be evaluated from registry `const_fold` callbacks and literal inputs.
    /// This also carries a local assigned by a namespace-normalising helper:
    /// its false branch is selected through one registry fold and its final
    /// replacement result through another registry fold.
    /// A renamed, aliased, unknown, effectful, or non-constant call yields no
    /// value and the analysis abstains.
    fn resolve_parameterised_creation_name(
        &mut self,
        candidate: &ParameterisedCreation,
        binding: &[String],
    ) -> Option<String> {
        let actuals: Vec<Option<String>> = binding.iter().cloned().map(Some).collect();
        let mut env: std::collections::HashMap<String, String> =
            bind_proc_formals(&candidate.params, &actuals)?
                .into_iter()
                .filter_map(|(name, value)| value.map(|value| (name, value)))
                .filter(|(_, value)| !tcl_syntax::naming::is_dynamic_word(value))
                .collect();
        let proc_def = self.result.all_procs.get(&candidate.proc_qname)?.clone();
        let mut body_span = proc_def.body_span;
        for arm in &candidate.control_path {
            self.apply_static_path_prefix(
                &candidate.proc_qname,
                body_span,
                arm.controller.span.start(),
                &mut env,
            )?;
            if !self.control_body_is_selected(&candidate.proc_qname, arm, &env, 0)? {
                return None;
            }
            body_span = arm.body_span;
        }
        self.apply_static_path_prefix(
            &candidate.proc_qname,
            body_span,
            candidate.call_off,
            &mut env,
        )?;
        self.static_word_value(
            &candidate.name_word,
            &candidate.proc_qname,
            &env,
            &mut Vec::new(),
            0,
        )
    }

    /// Execute the direct, dominating prefix of one selected script body.
    /// A Tcl result completion ends the path; statements after it cannot
    /// contribute provenance to a later creation.
    fn apply_static_path_prefix(
        &mut self,
        proc_qname: &str,
        body_span: Span,
        stop_off: u32,
        env: &mut std::collections::HashMap<String, String>,
    ) -> Option<()> {
        for seg in self.direct_statements_in_span(body_span)? {
            if seg.span.start() >= stop_off {
                break;
            }
            if !self.static_statement_falls_through(&seg, 0) {
                return None;
            }
            self.apply_static_provenance_statement(proc_qname, &seg, env, 0)?;
        }
        Some(())
    }

    /// Whether the typed control-flow command selects `arm` for the current
    /// literal environment.  Unknown conditions and unsupported control
    /// shapes abstain; no nested body is flattened into an unconditional
    /// statement stream.
    fn control_body_is_selected(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<bool> {
        let registry = self.registry.as_deref()?;
        let spec = registry.get(arm.controller.name())?;
        match self.control_arm_semantics_for(arm)? {
            tcl_registry::registry::ControlArmSemantics::Always => return Some(true),
            tcl_registry::registry::ControlArmSemantics::Selected => {}
            tcl_registry::registry::ControlArmSemantics::FrameBoundary
            | tcl_registry::registry::ControlArmSemantics::Uncertain => return None,
            tcl_registry::registry::ControlArmSemantics::CompletionBoundary => {
                return Some(true);
            }
        }
        match spec.lowering_hook? {
            tcl_registry::hooks::LoweringHookId::If => {
                self.if_body_is_selected(proc_qname, arm, env, depth)
            }
            tcl_registry::hooks::LoweringHookId::Switch => {
                self.switch_body_is_selected(proc_qname, arm, env, depth)
            }
            _ => None,
        }
    }

    /// Resolve typed semantics for either a direct body argument or one body
    /// nested inside a registry-described case-list argument. The latter's
    /// token span belongs to a list element, not the controller argv, so its
    /// owning clause-list position must be recovered through `CaseListSpec`.
    fn control_arm_semantics_for(
        &self,
        arm: &ControlArm,
    ) -> Option<tcl_registry::registry::ControlArmSemantics> {
        let registry = self.registry.as_deref()?;
        let args: Vec<&str> = arm.controller.args().iter().map(String::as_str).collect();
        let direct = arm
            .controller
            .arg_tokens()
            .iter()
            .position(|token| token.span == arm.body_span);
        let body_index = direct.or_else(|| {
            let (_, invocation) = registry.case_invocation(
                arm.controller.name(),
                &args,
                self.profile.availability_mask,
            )?;
            let index = invocation.clause_list_index?;
            let container = arm.controller.arg_tokens().get(index)?.span;
            (container.start() <= arm.body_span.start() && arm.body_span.end() <= container.end())
                .then_some(index)
        })?;
        registry.control_arm_semantics(arm.controller.name(), &args, body_index)
    }

    fn control_arms_for_segment(&self, seg: &SegmentedCommand) -> ControlArms {
        let Some(registry) = self.registry.as_deref() else {
            return ControlArms::default();
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let mut indices =
            registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
        indices.sort_unstable();
        indices.dedup();
        let case_call = registry
            .case_invocation(seg.name(), &args, self.profile.availability_mask)
            .and_then(|(_, invocation)| invocation.clause_list_index);
        let mut arms = Vec::new();
        let mut complete = true;
        for idx in indices {
            let (Some(word), Some(token)) =
                (seg.args().get(idx), seg.arg_tokens().get(idx).copied())
            else {
                complete = false;
                continue;
            };
            if case_call == Some(idx) {
                let elements =
                    crate::segmenter::flatten_clause_list_elements(&self.source, word, token);
                for pair in elements.chunks_exact(2) {
                    let body_word = &pair[1].0;
                    let body_token = pair[1].1;
                    if body_token.kind == TokenType::Str
                        || (body_token.kind == TokenType::Esc
                            && !tcl_syntax::naming::is_dynamic_word(body_word)
                            && !body_word.contains('\\'))
                    {
                        arms.push(ControlArm {
                            controller: seg.clone(),
                            body_span: body_token.span,
                        });
                    } else {
                        complete = false;
                    }
                }
            } else if token.kind == TokenType::Str
                || (token.kind == TokenType::Esc
                    && !tcl_syntax::naming::is_dynamic_word(word)
                    && !word.contains('\\'))
            {
                arms.push(ControlArm {
                    controller: seg.clone(),
                    body_span: token.span,
                });
            } else {
                complete = false;
            }
        }
        ControlArms { arms, complete }
    }

    fn if_body_is_selected(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<bool> {
        let registry = self.registry.as_deref()?;
        let args: Vec<&str> = arm.controller.args().iter().map(String::as_str).collect();
        let exprs = registry.arg_indices_for_role(
            arm.controller.name(),
            &args,
            tcl_registry::ArgRole::Expr,
        );
        let bodies = registry.arg_indices_for_role(
            arm.controller.name(),
            &args,
            tcl_registry::ArgRole::Body,
        );
        let wanted = arm
            .controller
            .arg_tokens()
            .iter()
            .position(|token| token.span == arm.body_span)?;
        for (branch, expr_idx) in exprs.iter().copied().enumerate() {
            let selected = self.static_expr_value(
                arm.controller.args().get(expr_idx)?,
                proc_qname,
                env,
                &mut Vec::new(),
                depth,
            )?;
            if selected {
                return Some(bodies.get(branch).copied() == Some(wanted));
            }
        }
        Some(exprs.len() < bodies.len() && bodies.last().copied() == Some(wanted))
    }

    fn switch_body_is_selected(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<bool> {
        let registry = self.registry.as_deref()?;
        let args: Vec<&str> = arm.controller.args().iter().map(String::as_str).collect();
        let (case, invocation) = registry.case_invocation(
            arm.controller.name(),
            &args,
            self.profile.availability_mask,
        )?;
        if usize::from(case.subject_args) != 1 {
            return None;
        }
        if invocation.mode == tcl_registry::spec::CaseMatchMode::Regexp {
            return None;
        }
        let subject = self.static_word_value(
            arm.controller.args().get(invocation.subject_index?)?,
            proc_qname,
            env,
            &mut Vec::new(),
            depth,
        )?;

        let mut clauses: Vec<(String, String, Span)> = Vec::new();
        if let Some(list_index) = invocation.clause_list_index {
            let word = arm.controller.args().get(list_index)?;
            let token = *arm.controller.arg_tokens().get(list_index)?;
            let elements =
                crate::segmenter::flatten_clause_list_elements(&self.source, word, token);
            for pair in elements.chunks_exact(2) {
                clauses.push((pair[0].0.clone(), pair[1].0.clone(), pair[1].1.span));
            }
        } else {
            let mut idx = invocation.inline_clause_start?;
            while idx + 1 < arm.controller.args().len() {
                let pattern = self.static_word_value(
                    arm.controller.args().get(idx)?,
                    proc_qname,
                    env,
                    &mut Vec::new(),
                    depth,
                )?;
                let body_span = arm.controller.arg_tokens().get(idx + 1)?.span;
                clauses.push((
                    pattern,
                    arm.controller.args().get(idx + 1)?.clone(),
                    body_span,
                ));
                idx += 2;
            }
        }

        let mut default = None;
        let mut selected = None;
        for (idx, (pattern, _, _body_span)) in clauses.iter().enumerate() {
            if case.is_keyword_pattern(pattern, idx, clauses.len()) {
                default.get_or_insert(idx);
                continue;
            }
            let matched = match invocation.mode {
                tcl_registry::spec::CaseMatchMode::Exact if invocation.nocase => {
                    pattern.to_lowercase() == subject.to_lowercase()
                }
                tcl_registry::spec::CaseMatchMode::Exact => pattern == &subject,
                tcl_registry::spec::CaseMatchMode::Glob => {
                    tcl_syntax::glob::string_case_match(pattern, &subject, invocation.nocase)
                }
                tcl_registry::spec::CaseMatchMode::Regexp => return None,
            };
            if matched {
                selected = Some(idx);
                break;
            }
        }
        let mut selected = selected.or(default)?;
        while clauses
            .get(selected)
            .is_some_and(|(_, body, _)| case.fallthrough_body == Some(body.as_str()))
        {
            selected += 1;
        }
        Some(clauses.get(selected)?.2 == arm.body_span)
    }

    /// Apply one dominating statement to the literal environment.  Binding
    /// layout comes from the registry's `set` handle/value descriptor; branch
    /// conditionality and expression/body positions come from traits and
    /// argument roles. Registry-declared variable/unknown writes invalidate
    /// the affected facts; an unsupported command abstains.
    fn apply_static_provenance_statement(
        &mut self,
        proc_qname: &str,
        seg: &SegmentedCommand,
        env: &mut std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<()> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let registry = self.registry.as_deref()?;
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let spec = registry.get(seg.name())?;

        if spec
            .traits
            .contains(tcl_registry::Traits::BRANCH_SELECTED_BODY)
        {
            let exprs =
                registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Expr);
            let bodies =
                registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
            if let ([expr_idx], [body_idx]) = (exprs.as_slice(), bodies.as_slice())
                && let Some(condition) = self.static_expr_value(
                    seg.args().get(*expr_idx)?,
                    proc_qname,
                    env,
                    &mut Vec::new(),
                    depth,
                )
            {
                if condition {
                    self.invalidate_static_facts_written_by_script(
                        seg.args().get(*body_idx)?,
                        env,
                        depth + 1,
                    )?;
                }
                return Some(());
            }

            // For a multi-arm or unresolved branch, keep only facts no arm
            // can change. Argument layout and same-frame body identity both
            // come from the registry. An explicit/dynamic variable write or
            // a registry-declared unknown write invalidates the affected
            // fact; unrelated locals survive without selecting an arm.
            for body_idx in bodies {
                self.invalidate_static_facts_written_by_script(
                    seg.args().get(body_idx)?,
                    env,
                    depth + 1,
                )?;
            }
            return Some(());
        }

        if let Some(binding) = spec.binds_handle
            && matches!(
                binding.class_from,
                tcl_registry::handle_binding::HandleClassSource::ConstructionValue(_)
            )
            && let Some(bound) = binding.resolve(&args)
        {
            let value =
                self.static_word_value(bound.class_word, proc_qname, env, &mut Vec::new(), depth);
            if let Some(value) = value {
                env.insert(bound.name.to_owned(), value);
            } else {
                env.remove(bound.name);
            }
            return Some(());
        }

        for idx in registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::VarWrite)
        {
            if let Some(name) = seg.args().get(idx) {
                env.remove(name);
            }
        }
        self.invalidate_static_facts_written_by_segment(seg, env, depth + 1)
    }

    /// Invalidate literal facts a same-frame script may write.
    ///
    /// Variable positions, nested body positions, body-frame ownership, and
    /// unknown write effects are all registry declarations. A dynamic write
    /// target or an unknown command/effect can name any local and therefore
    /// clears the environment; a literal target kills only that fact.
    fn invalidate_static_facts_written_by_script(
        &self,
        body: &str,
        env: &mut std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<()> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            env.clear();
            return Some(());
        }
        for seg in
            crate::segmenter::segment_commands_with_offset_and_config(body, 0, self.lexer_config())
        {
            self.invalidate_static_facts_written_by_segment(&seg, env, depth + 1)?;
        }
        Some(())
    }

    /// Invalidate facts written by one command whose body framing and
    /// variable layout are described by the registry.
    fn invalidate_static_facts_written_by_segment(
        &self,
        seg: &SegmentedCommand,
        env: &mut std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<()> {
        let registry = self.registry.as_deref()?;
        let Some(spec) = registry.get(seg.name()) else {
            env.clear();
            return Some(());
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let resolved_sub = args
            .first()
            .and_then(|first| spec.resolve_subcommand(first));

        if resolved_sub.is_some_and(|sub| sub.creates_scope_alias) {
            env.clear();
            return Some(());
        }

        for role in [
            tcl_registry::ArgRole::VarWrite,
            tcl_registry::ArgRole::LoopVarList,
        ] {
            for idx in registry.arg_indices_for_role(seg.name(), &args, role) {
                let Some(word) = seg.args().get(idx) else {
                    continue;
                };
                if tcl_syntax::naming::is_dynamic_word(word) {
                    env.clear();
                    return Some(());
                }
                let names: Vec<String> = if role == tcl_registry::ArgRole::LoopVarList {
                    let Ok(names) = tcl_syntax::list::split_list(word) else {
                        env.clear();
                        return Some(());
                    };
                    names
                        .into_iter()
                        .map(std::borrow::Cow::into_owned)
                        .collect()
                } else {
                    vec![word.clone()]
                };
                for name in names {
                    let base = name.split_once('(').map_or(name.as_str(), |(base, _)| base);
                    env.remove(base);
                }
            }
        }

        let plain_bodies = registry.plain_body_arg_indices(seg.name(), &args);
        for idx in &plain_bodies {
            if let Some(nested) = seg.args().get(*idx) {
                self.invalidate_static_facts_written_by_script(nested, env, depth + 1)?;
            }
        }

        // Control commands summarise their nested scripts as an unknown
        // effect. Same-frame bodies were inspected above, while structural
        // bodies execute in another frame. For a direct command with no
        // script argument, an unknown/variable write lacking a more precise
        // VarWrite role invalidates every remaining fact.
        let has_script_argument = tcl_registry::ArgRole::ALL
            .iter()
            .filter(|role| role.carries_script())
            .any(|&role| {
                !registry
                    .arg_indices_for_role(seg.name(), &args, role)
                    .is_empty()
            });
        if !has_script_argument {
            let effects = resolved_sub
                .filter(|sub| !sub.side_effects.is_empty())
                .map_or(spec.side_effects, |sub| sub.side_effects);
            if effects.iter().any(|effect| {
                effect.writes
                    && matches!(
                        effect.target,
                        tcl_registry::prelude::SideEffectTarget::Unknown
                            | tcl_registry::prelude::SideEffectTarget::Variable
                    )
            }) && registry
                .arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::VarWrite)
                .is_empty()
            {
                env.clear();
            }
        }
        Some(())
    }

    /// Evaluate a static word under `env`. Braced words are literal; a whole
    /// command substitution dispatches through registry folds or a proven
    /// user-procedure result; composite variable words use the same strict
    /// substitution routine as direct parameter names.
    fn static_word_value(
        &mut self,
        word: &str,
        proc_qname: &str,
        env: &std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let trimmed = word.trim();
        if let Some(inner) = trimmed.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            return Some(inner.to_owned());
        }
        let unquoted = trimmed
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(trimmed);
        if let Some((head, args)) = crate::value_shapes::parse_command_substitution(unquoted) {
            let values: Vec<Option<String>> = args
                .iter()
                .map(|arg| self.static_word_value(arg, proc_qname, env, stack, depth))
                .collect();
            return self.static_command_value(&head, &values, proc_qname, env, stack, depth);
        }
        let mut names: Vec<String> = env.keys().cloned().collect();
        names.sort();
        let values: Vec<String> = names
            .iter()
            .filter_map(|name| env.get(name).cloned())
            .collect();
        substitute_bound_words(unquoted, &names, &values)
    }

    /// Evaluate a registry-foldable command or a statically-resolved user
    /// procedure. Unknown arguments are allowed into a user procedure because
    /// a selected path may not read them; registry folds require every input.
    fn static_command_value(
        &mut self,
        head: &str,
        args: &[Option<String>],
        proc_qname: &str,
        _env: &std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let registry = self.registry.as_deref()?;
        if let Some(spec) = registry.get(head) {
            if !self.static_provenance_command_is_trusted(head) {
                return None;
            }
            let values: Vec<&str> = args
                .iter()
                .map(|value| value.as_deref())
                .collect::<Option<_>>()?;
            let dialect = (!self.result.dialect.is_empty()).then_some(self.result.dialect.as_str());
            if spec.subcommands.is_empty() {
                return spec.run_const_fold(&values, dialect);
            }
            let (sub, rest) = values.split_first()?;
            return spec.resolve_subcommand(sub)?.run_const_fold(rest, dialect);
        }

        let qname = self.resolve_static_proc_name(proc_qname, head)?;
        if !self.static_provenance_command_is_trusted(&qname)
            || stack.iter().any(|seen| seen == &qname)
        {
            return None;
        }
        stack.push(qname.clone());
        let result = self.static_proc_result(&qname, args, stack, depth + 1);
        stack.pop();
        result
    }

    /// Whether `query` still denotes its declared builtin/procedure for the
    /// narrow static-provenance proof.
    ///
    /// Command-table mutators are discovered only through the registry's
    /// [`CommandTableEffect`](tcl_registry::CommandTableEffect). Dynamic
    /// target words retain their literal fragments instead of collapsing the
    /// entire command table: `${ns}::define::$method` cannot name `::string`,
    /// whereas `$name` can name anything and therefore abstains. This scan is
    /// flow-insensitive, so a compatible mutation before or after the proof
    /// site blocks it equally.
    fn static_provenance_command_is_trusted(&self, query: &str) -> bool {
        let Some(registry) = self.registry.as_deref() else {
            return false;
        };
        let Some(statements) = self.statements_in_span(Span::new(
            0,
            u32::try_from(self.source.len()).unwrap_or(u32::MAX),
        )) else {
            return false;
        };
        let query = format!(
            "::{}",
            crate::naming::normalise_qualified_name(query).trim_start_matches("::")
        );
        let expected_declarations = usize::from(self.result.all_procs.contains_key(&query));
        let mut matching_declarations = 0_usize;

        for seg in statements {
            // A declaration body is dormant until its command is invoked.
            // Its dynamic `proc`/`rename`/`interp alias` words therefore do
            // not mutate the load-time command table merely because the body
            // is present in the source. Invoked bodies are evaluated through
            // `static_proc_result`, whose unsupported/mutating statements
            // still make the proof abstain.
            if self
                .result
                .offset_is_inside_any_definition_body(seg.span.start())
            {
                continue;
            }
            let args = seg.args();
            match registry.command_table_effect(seg.name(), args.first().map(String::as_str)) {
                Some(tcl_registry::CommandTableEffect::DefinesProcedure) => {
                    let Some(name) = args.first() else {
                        return false;
                    };
                    if dynamic_command_name_may_equal(name, &query) {
                        if tcl_syntax::naming::is_dynamic_word(name) {
                            return false;
                        }
                        matching_declarations += 1;
                        if matching_declarations > expected_declarations {
                            return false;
                        }
                    }
                }
                Some(tcl_registry::CommandTableEffect::RenamesCommands) => {
                    let [old, new] = args else {
                        return false;
                    };
                    if dynamic_command_name_may_equal(old, &query)
                        || (!new.is_empty() && dynamic_command_name_may_equal(new, &query))
                    {
                        return false;
                    }
                }
                Some(tcl_registry::CommandTableEffect::CreatesAliases) => {
                    // `interp alias SRC NAME TARGET TARGETCMD ...`: only the
                    // alias NAME changes a command identity. A dynamic target
                    // command changes what that known alias does, not what
                    // name it binds.
                    if args.len() == 3 {
                        continue; // query form; the command table is unchanged
                    }
                    let Some(alias_name) = args.get(2) else {
                        return false;
                    };
                    if dynamic_command_name_may_equal(alias_name, &query) {
                        return false;
                    }
                }
                None => {}
            }
        }
        matching_declarations == expected_declarations
    }

    fn resolve_static_proc_name(&self, caller: &str, head: &str) -> Option<String> {
        let absolute = if head.starts_with("::") {
            crate::naming::qualify("", head)
        } else {
            let ns = caller.rsplit_once("::").map_or("", |(ns, _)| ns);
            crate::naming::qualify(ns, head)
        };
        if self.result.all_procs.contains_key(&absolute) {
            return Some(absolute);
        }
        let global = crate::naming::qualify("", head);
        self.result
            .all_procs
            .contains_key(&global)
            .then_some(global)
    }

    /// Evaluate the return of a small user procedure from registry command
    /// folds. Every unsupported control-flow or command result abstains.
    fn static_proc_result(
        &mut self,
        qname: &str,
        args: &[Option<String>],
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        let proc_def = self.result.all_procs.get(qname)?.clone();
        if proc_def.params_computed {
            return None;
        }
        let mut env = std::collections::HashMap::new();
        for (name, value) in bind_proc_formals(&proc_def.params, args)? {
            if let Some(value) = value {
                env.insert(name, value);
            }
        }
        match self.static_script_result(qname, proc_def.body_span, &mut env, stack, depth)? {
            StaticScriptOutcome::FallsThrough(value) | StaticScriptOutcome::Returns(value) => {
                Some(value)
            }
        }
    }

    fn static_script_result(
        &mut self,
        qname: &str,
        body_span: Span,
        env: &mut std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<StaticScriptOutcome> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let statements = self.direct_statements_in_span(body_span)?;
        let mut result = String::new();
        for seg in statements {
            let registry = self.registry.as_deref()?;
            let raw_args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            match registry.invocation_completion(
                seg.name(),
                &raw_args,
                self.profile.availability_mask,
            ) {
                tcl_registry::registry::InvocationCompletion::ReturnsResult(Some(idx)) => {
                    let value =
                        self.static_word_value(seg.args().get(idx)?, qname, env, stack, depth)?;
                    return Some(StaticScriptOutcome::Returns(value));
                }
                tcl_registry::registry::InvocationCompletion::ReturnsResult(None) => {
                    return Some(StaticScriptOutcome::Returns(String::new()));
                }
                tcl_registry::registry::InvocationCompletion::Terminates
                | tcl_registry::registry::InvocationCompletion::Unknown => return None,
                tcl_registry::registry::InvocationCompletion::FallsThrough => {}
            }
            let spec = registry.get(seg.name())?;
            if spec
                .traits
                .contains(tcl_registry::Traits::BRANCH_SELECTED_BODY)
            {
                if registry.control_invocation_valid(
                    seg.name(),
                    &raw_args,
                    self.profile.availability_mask,
                ) != Some(true)
                {
                    return None;
                }
                if !matches!(
                    spec.lowering_hook,
                    Some(
                        tcl_registry::hooks::LoweringHookId::If
                            | tcl_registry::hooks::LoweringHookId::Switch
                    )
                ) {
                    return None;
                }
                let mut selected = None;
                let controls = self.control_arms_for_segment(&seg);
                if !controls.complete {
                    return None;
                }
                for arm in controls.arms {
                    match self.control_body_is_selected(qname, &arm, env, depth) {
                        Some(true) if selected.is_none() => selected = Some(arm.body_span),
                        Some(true) | None => return None,
                        Some(false) => {}
                    }
                }
                if let Some(span) = selected {
                    match self.static_script_result(qname, span, env, stack, depth + 1)? {
                        StaticScriptOutcome::Returns(value) => {
                            return Some(StaticScriptOutcome::Returns(value));
                        }
                        StaticScriptOutcome::FallsThrough(value) => result = value,
                    }
                } else {
                    result.clear();
                }
                continue;
            }
            if let Some(binding) = spec.binds_handle
                && matches!(
                    binding.class_from,
                    tcl_registry::handle_binding::HandleClassSource::ConstructionValue(_)
                )
                && let Some(bound) = binding.resolve(&raw_args)
            {
                result = self.static_word_value(bound.class_word, qname, env, stack, depth)?;
                env.insert(bound.name.to_owned(), result.clone());
                continue;
            }
            let values: Vec<Option<String>> = seg
                .args()
                .iter()
                .map(|arg| self.static_word_value(arg, qname, env, stack, depth))
                .collect();
            result = self.static_command_value(seg.name(), &values, qname, env, stack, depth)?;
        }
        Some(StaticScriptOutcome::FallsThrough(result))
    }

    fn static_expr_value(
        &mut self,
        word: &str,
        proc_qname: &str,
        env: &std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<bool> {
        let mut expr = word
            .trim()
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .unwrap_or(word.trim())
            .to_owned();
        while let Some(start) = expr.find('[') {
            let mut nesting = 0_u32;
            let mut end = None;
            for (idx, ch) in expr[start..].char_indices() {
                match ch {
                    '[' => nesting += 1,
                    ']' => {
                        nesting = nesting.saturating_sub(1);
                        if nesting == 0 {
                            end = Some(start + idx + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let end = end?;
            let substitution = expr.get(start..end)?;
            let folded = self.static_word_value(substitution, proc_qname, env, stack, depth)?;
            expr.replace_range(start..end, &folded);
        }
        let static_env: crate::static_loops::StaticEnv = env
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    crate::static_loops::parse_literal_value(value),
                )
            })
            .collect();
        crate::static_loops::evaluate_expr_with_constants(
            &crate::parse_expr(&expr, Some(self.dialect())),
            &static_env,
            crate::tcl_expr_eval::FoldPolicy::default(),
        )
        .map(|value| value != 0)
    }

    /// Direct statements only (no nested body flattening), used by the
    /// static provenance evaluator so a non-selected branch cannot leak a
    /// binding into the outer path.
    fn direct_statements_in_span(&self, span: Span) -> Option<Vec<SegmentedCommand>> {
        let start = span.start() as usize;
        let end = span.end() as usize;
        let raw = self.source.get(start..end)?;
        let (body, base) = if matches!(raw, "}" | "{}") {
            // A closed empty braced word has the lexer's degenerate span on
            // its closing delimiter. It contains no script to execute.
            ("", start)
        } else {
            raw.strip_prefix(['{', '"'])
                .map_or((raw, start), |inner| (inner, start.saturating_add(1)))
        };
        Some(crate::segmenter::segment_commands_with_offset_and_config(
            body,
            u32::try_from(base).unwrap_or(0),
            self.lexer_config(),
        ))
    }

    /// Drop the record the walk made for a creation whose name it could not
    /// resolve, now that the call sites have named the classes it really
    /// makes.
    ///
    /// The walk records `::T::${ns}::class` verbatim — a class of that name
    /// exists in no interpreter, and leaving it would show a phantom entry in
    /// the outline beside the real one and put a nonsense key in the
    /// workspace class index. Only removed when at least one real name was
    /// proved for that same site, so a creation nothing could settle keeps
    /// exactly the record it had before.
    fn retract_unresolved_class(&mut self, candidate: &ParameterisedCreation) {
        let Some(unresolved) = Self::unresolved_class_name(candidate) else {
            return;
        };
        self.result.all_classes.remove(&unresolved);
        self.result
            .class_body_spans
            .retain(|(qname, _)| qname != &unresolved);
    }

    fn unresolved_class_name(candidate: &ParameterisedCreation) -> Option<String> {
        let owner_ns = candidate
            .proc_qname
            .rsplit_once("::")
            .map_or("", |(head, _)| head);
        let unresolved = crate::naming::qualify(owner_ns, &candidate.name_word);
        tcl_syntax::naming::is_dynamic_word(&unresolved).then_some(unresolved)
    }

    /// Every creation call written inside a proc body whose **name word**
    /// interpolates one of that proc's parameters — the sites
    /// [`Self::record_literal_parameter_definitions`] can settle.
    ///
    /// Cheap by construction: the manufacturer-word test is a registry
    /// lookup on the call's first argument, so a body with no `create` /
    /// `new` / `createWithNamespace` in it is rejected before anything is
    /// segmented twice.
    fn parameterised_creation_sites(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<ParameterisedCreation> {
        let mut out = Vec::new();
        for (qname, proc_def) in &self.result.all_procs {
            if proc_def.params_computed || proc_def.params.is_empty() {
                continue;
            }
            self.collect_parameterised_creation_sites(
                registry,
                ParameterisedProc {
                    qname,
                    params: &proc_def.params,
                },
                proc_def.body_span,
                &mut Vec::new(),
                0,
                &mut out,
            );
        }
        out
    }

    fn collect_parameterised_creation_sites(
        &self,
        registry: &tcl_registry::CommandRegistry,
        proc_context: ParameterisedProc<'_>,
        body_span: Span,
        control_path: &mut Vec<ControlArm>,
        depth: u32,
        out: &mut Vec<ParameterisedCreation>,
    ) {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return;
        }
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return;
        };
        for seg in statements {
            let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            if let Some(first) = seg.args().first()
                && registry.is_manufacturer_method(first)
                && let Some(name_arg) = registry.uniform_manufacturer_names_instance_at(first)
                && let Some(name_word) = seg.args().get(name_arg)
                && !crate::var_refs::scan_var_ref_forms(name_word).is_empty()
            {
                out.push(ParameterisedCreation {
                    proc_qname: proc_context.qname.to_owned(),
                    params: proc_context.params.to_vec(),
                    name_word: name_word.clone(),
                    call_off: seg.span.start(),
                    control_path: control_path.clone(),
                });
            }

            let case_call = registry
                .case_invocation(seg.name(), &args, self.profile.availability_mask)
                .and_then(|(_, invocation)| invocation.clause_list_index);
            let mut body_indices =
                registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
            body_indices.sort_unstable();
            body_indices.dedup();
            for body_idx in body_indices {
                let (Some(word), Some(token)) = (
                    seg.args().get(body_idx),
                    seg.arg_tokens().get(body_idx).copied(),
                ) else {
                    continue;
                };
                if case_call == Some(body_idx) {
                    let elements =
                        crate::segmenter::flatten_clause_list_elements(&self.source, word, token);
                    for pair in elements.chunks_exact(2) {
                        let body_token = pair[1].1;
                        if body_token.kind != TokenType::Str {
                            continue;
                        }
                        control_path.push(ControlArm {
                            controller: seg.clone(),
                            body_span: body_token.span,
                        });
                        self.collect_parameterised_creation_sites(
                            registry,
                            ParameterisedProc {
                                qname: proc_context.qname,
                                params: proc_context.params,
                            },
                            body_token.span,
                            control_path,
                            depth + 1,
                            out,
                        );
                        control_path.pop();
                    }
                    continue;
                }
                if token.kind != TokenType::Str {
                    continue;
                }
                control_path.push(ControlArm {
                    controller: seg.clone(),
                    body_span: token.span,
                });
                self.collect_parameterised_creation_sites(
                    registry,
                    ParameterisedProc {
                        qname: proc_context.qname,
                        params: proc_context.params,
                    },
                    token.span,
                    control_path,
                    depth + 1,
                    out,
                );
                control_path.pop();
            }
        }
    }

    /// The **load-time** call sites in this document, keyed by the qualified
    /// name the head word spells, each carrying its argument words.
    ///
    /// A call inside a proc or class body is excluded: it runs at call time,
    /// if ever, so it proves nothing about what sourcing this file creates —
    /// the same rule [`Self::publish_load_level_destructions`] applies.
    fn load_time_call_arguments(&self) -> std::collections::HashMap<String, Vec<LoadTimeCall>> {
        let mut out = std::collections::HashMap::new();
        self.collect_load_time_calls(
            Span::new(0, u32::try_from(self.source.len()).unwrap_or(u32::MAX)),
            &mut Vec::new(),
            0,
            &mut out,
        );
        out
    }

    fn collect_load_time_calls(
        &self,
        body_span: Span,
        control_path: &mut Vec<ControlArm>,
        depth: u32,
        out: &mut std::collections::HashMap<String, Vec<LoadTimeCall>>,
    ) {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return;
        }
        let Some(registry) = self.registry.as_deref() else {
            return;
        };
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return;
        };
        for seg in statements {
            if !self
                .result
                .offset_is_inside_any_definition_body(seg.span.start())
            {
                let head = crate::naming::normalise_qualified_name(seg.name());
                if !head.is_empty() {
                    let qualified = format!("::{}", head.trim_start_matches("::"));
                    out.entry(qualified).or_default().push(LoadTimeCall {
                        args: seg.args().to_vec(),
                        control_path: control_path.clone(),
                        call_off: seg.span.start(),
                    });
                }
            }

            let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            let mut body_indices =
                registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
            body_indices.sort_unstable();
            body_indices.dedup();
            let case_call = registry
                .case_invocation(seg.name(), &args, self.profile.availability_mask)
                .and_then(|(_, invocation)| invocation.clause_list_index);
            for body_idx in body_indices {
                let (Some(word), Some(token)) = (
                    seg.args().get(body_idx),
                    seg.arg_tokens().get(body_idx).copied(),
                ) else {
                    continue;
                };
                if case_call == Some(body_idx) {
                    let elements =
                        crate::segmenter::flatten_clause_list_elements(&self.source, word, token);
                    for pair in elements.chunks_exact(2) {
                        let body_token = pair[1].1;
                        if body_token.kind != TokenType::Str {
                            continue;
                        }
                        control_path.push(ControlArm {
                            controller: seg.clone(),
                            body_span: body_token.span,
                        });
                        self.collect_load_time_calls(body_token.span, control_path, depth + 1, out);
                        control_path.pop();
                    }
                    continue;
                }
                if token.kind != TokenType::Str {
                    continue;
                }
                control_path.push(ControlArm {
                    controller: seg.clone(),
                    body_span: token.span,
                });
                self.collect_load_time_calls(token.span, control_path, depth + 1, out);
                control_path.pop();
            }
        }
    }

    /// The statements of the script at `span`, plus those of every nested
    /// script word, re-segmented from the document's own bytes.
    ///
    /// `span` may be a body **word**'s span (whose start sits on the opening
    /// delimiter) or a plain script range; the delimiter is stripped when
    /// present, matching [`Self::manufacturer_next_call`].
    fn statements_in_span(&self, span: Span) -> Option<Vec<SegmentedCommand>> {
        let start = span.start() as usize;
        let end = span.end() as usize;
        if start >= end || end > self.source.len() {
            return None;
        }
        let raw = self.source.get(start..end)?;
        let (body, base) = raw
            .strip_prefix(['{', '"'])
            .map_or((raw, start), |inner| (inner, start.saturating_add(1)));
        let mut blocks = Vec::new();
        self.collect_script_blocks(body, u32::try_from(base).unwrap_or(0), 0, &mut blocks);
        Some(blocks.into_iter().flatten().collect())
    }

    /// Which of the creation call's argument words carry the new class's
    /// name and body, read off the manufacturer override's own `next` call.
    ///
    /// The override re-enters the manufacturer chain through `next`
    /// (identified by [`tcl_registry::registry::MethodDispatchKind::NextChain`],
    /// registry data), whose arguments sit at the *builtin* `create Name
    /// Body` positions.  Whichever of the override's parameters those
    /// arguments read is therefore the caller's name / body word: the first
    /// parameter reference is the name, the last is the body (the body is
    /// always spliced in last — everything between is the definition
    /// prologue the manufacturer builds).  Returns `None` when the override
    /// cannot be read that way, and the caller falls back to the builtin
    /// positions with inheritance marked unknown.
    fn manufacturer_word_positions(
        &self,
        override_def: &super::types::MethodDef,
    ) -> Option<(usize, usize)> {
        let params: Vec<&str> = override_def
            .params
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        let next_seg = self.manufacturer_next_call(override_def)?;
        // Parameter `i` of the override binds argument `i + 1` of the call
        // (argument 0 being the manufacturer subcommand itself).
        let param_arg = |name: &str| params.iter().position(|p| *p == name).map(|i| i + 1);
        let refs: Vec<String> = next_seg
            .args()
            .iter()
            .flat_map(|word| crate::var_refs::scan_var_ref_forms(word))
            .collect();
        let name_arg = param_arg(refs.first()?.as_str())?;
        let body_arg = param_arg(refs.last()?.as_str())?;
        (name_arg != body_arg).then_some((name_arg, body_arg))
    }

    /// The manufacturer override's `next` statement, re-segmented from its
    /// own source bytes.
    fn manufacturer_next_call(
        &self,
        override_def: &super::types::MethodDef,
    ) -> Option<SegmentedCommand> {
        let registry = self.registry.as_ref()?;
        let start = override_def.body_span.start() as usize;
        let end = override_def.body_span.end() as usize;
        let raw = self.source.get(start..end)?;
        // `MethodDef` keeps the body **word**'s span, whose start sits on
        // the opening delimiter; the script is its content.
        let (body, base) = raw
            .strip_prefix(['{', '"'])
            .map_or((raw, start), |inner| (inner, start.saturating_add(1)));
        crate::segmenter::segment_commands_with_offset_and_config(
            body,
            u32::try_from(base).unwrap_or(0),
            self.lexer_config(),
        )
        .into_iter()
        .find(|seg| {
            !seg.is_partial
                && registry.method_dispatch_keyword(seg.name())
                    == Some(tcl_registry::registry::MethodDispatchKind::NextChain)
        })
    }

    /// The definition-body members the manufacturer splices into every class
    /// it makes, as a template over the creation call's arguments.
    ///
    /// Tk's `Megawidget` hands `next` a `[list superclass ::tk::MegawidgetClass
    /// {*}$superclasses]` prologue, so every class it makes really does
    /// inherit `::tk::MegawidgetClass` plus whatever the caller passed —
    /// tclsh 9.0.4 and 8.6.16 both report exactly that from `info class
    /// superclasses`.  Only **reference-only** members (`superclass`,
    /// `mixin`, `filter`, `export`, … — the grammar's `all_args_ref` set)
    /// are injected: they name existing entities, so every injected word
    /// keeps a real source span, either in the manufacturer's own body
    /// (a literal it always splices) or in the call's arguments (a
    /// `{*}$param` splice, resolved per call by
    /// [`resolve_factory_member`]).
    ///
    /// **Reading the whole prologue is a precondition, not a best effort.**
    /// The definition word `next` receives is scanned piece by piece, and
    /// every piece must be one the analyser can account for: a `[list
    /// <member> …]` group it parsed, the manufacturer's own `$param` read
    /// (the caller's body, spliced in verbatim), or list/statement
    /// separator text.  Anything else — most importantly a **string-built**
    /// prologue such as `next $name "superclass Base\n$body"`, which
    /// injects a superclass with no nested command at all (tclsh 9.0.4 /
    /// 8.6.16: `info class superclasses` really reports `::Base`, and its
    /// methods really are inherited) — yields `None`, and the class is
    /// recorded with its inheritance marked unknown.  Returning a
    /// *known-empty* injection for a prologue that was merely unreadable
    /// would claim the class has no superclass and let W308 fire on
    /// perfectly good inherited methods.
    fn manufacturer_injected_template(
        &self,
        meta: &UserMetaclass,
        override_def: &super::types::MethodDef,
    ) -> Option<Vec<FactoryMember>> {
        let params: Vec<&str> = override_def
            .params
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        let next_seg = self.manufacturer_next_call(override_def)?;
        // The definition word is `next`'s last argument that reads one of
        // the override's parameters — the builtin `create Name Body`
        // layout's `Body` position, which is what the prologue is composed
        // into.
        let (word_index, prologue) =
            next_seg.args().iter().enumerate().rev().find(|(_, word)| {
                crate::var_refs::scan_var_ref_forms(word)
                    .iter()
                    .any(|name| params.contains(&name.as_str()))
            })?;
        // `Cmd` sub-tokens of that word, in source order — the word is
        // `next`'s last argument, so every command substitution at or after
        // its first token belongs to it.
        let word_start = next_seg.argv.get(word_index + 1)?.span.start();
        let mut groups = next_seg
            .all_tokens
            .iter()
            .filter(|tok| tok.kind == TokenType::Cmd && tok.span.start() >= word_start);
        let mut injected: Vec<FactoryMember> = Vec::new();
        for piece in prologue_pieces(prologue) {
            match piece {
                ProloguePiece::Separator => {}
                ProloguePiece::VarRead(name) if params.contains(&name) => {}
                ProloguePiece::Substitution => {
                    injected.push(self.injected_member_from_group(
                        meta,
                        *groups.next()?,
                        &params,
                    )?);
                }
                // Literal prologue text, a read of something that is not a
                // manufacturer parameter, or an unreadable fragment: the
                // prologue is not fully accounted for.
                ProloguePiece::VarRead(_) | ProloguePiece::Opaque => return None,
            }
        }
        Some(injected)
    }

    /// The definition-body member one `[…]` group of a manufacturer
    /// prologue installs, or `None` when the group is not a
    /// reference-only member built by a canonical-list command.
    fn injected_member_from_group(
        &self,
        meta: &UserMetaclass,
        group: Token,
        params: &[&str],
    ) -> Option<FactoryMember> {
        let registry = self.registry.as_ref()?;
        let sm = SourceMap::new(&self.source);
        let descended = descend_token(&sm, group, self.lexer_config());
        let mut segs = segments_from_tree(descended.tree(), &sm).into_iter();
        let seg = segs.next()?;
        // One group builds one member; a `[a; b]` compound is not a shape
        // the prologue reader accounts for.
        if segs.next().is_some() {
            return None;
        }
        // The prologue is built by a command that quotes its arguments into
        // a canonical list — registry data (`PRODUCES_CANONICAL_LIST`),
        // never a command name here.
        if !registry.get(seg.name()).is_some_and(|spec| {
            spec.traits
                .contains(tcl_registry::Traits::PRODUCES_CANONICAL_LIST)
        }) {
            return None;
        }
        let member = seg
            .args()
            .first()
            .and_then(|keyword| meta.grammar.member(keyword))?;
        member.all_args_ref?;
        template_injected_member(&seg, params)
    }

    /// The [`ClassFactory`] the command `cmd_name` names, when that command
    /// is a user-defined `TclOO` metaclass — this file's own, else one the
    /// **workspace factory index** proves is written in another document
    /// (issue #1276).
    ///
    /// The cross-document tier is what closes the audit's multi-file half:
    /// `::tk::Megawidget create IconList FocusableWidget {…}` in a file that
    /// never mentions `::tk::Megawidget`'s definition is, on shape alone,
    /// indistinguishable from `interp create` or `image create`, so the walk
    /// used to record nothing at all.  With the index it is classified from
    /// the metaclass's *own* declaration — its `create` override's parameter
    /// list and prologue — exactly as the same-file case is.
    ///
    /// The abstention it narrows is **kept everywhere it was earned**:
    ///
    /// * a dynamic command word (`$meta create …`) names nothing statically
    ///   and is rejected before any lookup;
    /// * the name is resolved through Tcl's real current-namespace-then-global
    ///   candidate order ([`crate::naming::bareword_resolution_candidates`]),
    ///   and only an **exact** candidate hit counts — a same-tailed metaclass
    ///   in an unrelated namespace never manufactures a class here, the same
    ///   discipline [`super::class_hierarchy::resolve_class_name`] applies
    ///   same-file;
    /// * with no index entry, nothing is recorded and nothing is diagnosed,
    ///   byte-for-byte as before.
    ///
    /// A cross-document factory's injected literals carry tokens that index
    /// the *metaclass's* document, so they are re-homed onto a token of this
    /// call ([`ClassFactory::resolve_in_other_document`]) — and an injection
    /// whose member word actually reads those tokens (a `retraction` member,
    /// registry data) collapses to "inheritance unknown" rather than being
    /// applied against a substituted span.
    fn class_factory_for_command(
        &self,
        cmd_name: &str,
        arg_tokens: &[Token],
        scope_path: &[usize],
        call_off: u32,
    ) -> Option<ClassFactory> {
        if crate::naming::is_dynamic_word(cmd_name) {
            return None;
        }
        let namespace = self.command_resolution_namespace(scope_path);
        let candidates = crate::naming::bareword_resolution_candidates(&namespace, cmd_name);
        if let Some(factory) = self.class_factory_for_candidates(&candidates, arg_tokens) {
            return Some(factory);
        }
        // A `rename`d metaclass command (issue #1305): the factory record is
        // keyed on the name the metaclass was *created* with, never the name
        // a later call spells, so `rename ::R::M ::R::Mk` then `::R::Mk
        // create …` must resolve `::R::Mk` back to `::R::M` before the
        // lookup above can ever hit — through the very same `rename` /
        // `interp alias` chain-walk the W307/W308 method check and the
        // LSP's navigation providers already use, so this can never
        // recognise a rename those do not (or vice versa).
        let renamed = candidates.iter().find_map(|candidate| {
            super::indirection::walk(&self.result, candidate, call_off, &|n| {
                crate::naming::normalise_qualified_name(n)
            })
        })?;
        let renamed_candidates =
            crate::naming::bareword_resolution_candidates(&namespace, &renamed.target);
        self.class_factory_for_candidates(&renamed_candidates, arg_tokens)
    }

    /// The local-then-workspace factory lookup [`Self::class_factory_for_command`]
    /// runs once for the command's own resolution candidates and once more
    /// (issue #1305) for its rename target's, factored out so the two
    /// attempts can never drift apart.
    fn class_factory_for_candidates(
        &self,
        candidates: &[String],
        arg_tokens: &[Token],
    ) -> Option<ClassFactory> {
        // This file first: a locally-written metaclass shadows a workspace one
        // under the same name, exactly as a local proc shadows a workspace
        // proc for command resolution.
        for candidate in candidates {
            if let Some(class) = self.result.all_classes.get(candidate) {
                return class.factory.clone();
            }
        }
        let index = self.workspace_class_factories.as_ref()?;
        let foreign = candidates
            .iter()
            .find_map(|candidate| index.get(candidate))?;
        // Tokens in the foreign factory index the metaclass's document. The
        // creation call's own subcommand word (`create`) is the nearest real
        // token in *this* document that the injected member genuinely came
        // with, and it is guaranteed present — the caller has already checked
        // `arg_tokens.len() >= 2`.
        let elsewhere = *arg_tokens.first()?;
        let grammar = self.definition_grammar(&foreign.root_metaclass)?;
        Some(foreign.resolve_in_other_document(elsewhere, &|keyword| {
            grammar
                .member(keyword)
                .is_some_and(|m| m.retraction.is_some())
        }))
    }

    /// The registry metaclass at the root of `seed`'s superclass chain, when
    /// `seed` (recorded, or being recorded, under `qualified`) is **itself a
    /// class factory** — a user-defined `TclOO` metaclass.
    ///
    /// A class is a factory when its (possibly indirect) superclass chain
    /// reaches a command the registry marks [`tcl_registry::Traits::IS_OO_METACLASS`]
    /// with a `TclOo` definition-body grammar — the same seed
    /// [`Self::handle_oo_class_command`] uses for a written-out
    /// `oo::class create`.  Only the inheritance step is added here, and
    /// that is `TclOO` language semantics (`oo::class`'s subclasses
    /// manufacture classes), not per-command knowledge.
    ///
    /// The returned grammar is the *root registry metaclass's* own
    /// definition-body grammar, so the bodies this factory makes are walked
    /// by exactly the grammar the language gives them without the walker
    /// naming a metaclass command.
    ///
    /// A `superclass` word is written as the *declaring* class sees it, so
    /// a relative name (`superclass Meta` inside `::n::DerivedMeta`) is
    /// resolved through the shared owner-aware class-name resolution
    /// ([`super::class_hierarchy::resolve_class_name`]) — the same
    /// current-namespace-then-global rule the class lattice and the MRO
    /// builder already implement, never a re-derivation of it here.  That
    /// keeps this chain walk sound-by-abstention in the same places they
    /// are: an ambiguous simple name resolves to nothing rather than
    /// cross-linking a same-named class in an unrelated namespace.
    ///
    /// Chain walking is depth-bounded and visited-checked, so a cyclic
    /// `superclass` declaration (rejected by real Tcl, but writable in a
    /// half-edited buffer) terminates.
    fn user_metaclass_of_class(&self, qualified: &str, seed: &ClassDef) -> Option<UserMetaclass> {
        let registry = self.registry.as_ref()?;
        let seed_key = qualified.to_string();
        let tail_index = super::class_hierarchy::build_tail_index(
            self.result
                .all_classes
                .keys()
                .chain(std::iter::once(&seed_key)),
        );
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut queue: Vec<String> = vec![seed_key.clone()];
        // Bounded by the recorded class count — every class is visited once.
        while let Some(class_qname) = queue.pop() {
            if !visited.insert(class_qname.clone()) {
                continue;
            }
            // The seed is the class being recorded, which is not in
            // `all_classes` until the walk that is asking finishes.
            let class = if class_qname == qualified {
                seed
            } else {
                let Some(class) = self.result.all_classes.get(&class_qname) else {
                    continue;
                };
                class
            };
            for parent in &class.superclasses {
                // The registry seed: `oo::class` and its siblings are named
                // as commands, so a leading `::` is the same command.
                let bare = parent.strip_prefix("::").unwrap_or(parent);
                if let Some(spec) = registry.get(bare)
                    && spec.traits.contains(tcl_registry::Traits::IS_OO_METACLASS)
                    && let Some(grammar) = spec.definition_body
                    && grammar.family == tcl_registry::definer::DefinerFamily::TclOo
                {
                    return Some(UserMetaclass {
                        root_command: bare.to_string(),
                        grammar,
                    });
                }
                if let Some(next) = super::class_hierarchy::resolve_class_name(
                    parent,
                    &class_qname,
                    |candidate| {
                        candidate == qualified || self.result.all_classes.contains_key(candidate)
                    },
                    &tail_index,
                ) {
                    queue.push(next);
                }
            }
        }
        None
    }

    /// Add one `oo::objdefine` site's per-object methods under `key`,
    /// skipping any the same site already contributed.
    ///
    /// The `foreach`-literal simulation re-dispatches an installer once per
    /// remaining element, and the ordinary walk already covered the first
    /// one, so the first element's site is visited twice under the
    /// *variable*'s key.  A single source site can never legitimately
    /// declare the same member twice, so `(objdefine_offset, name)` is an
    /// exact identity for "already recorded" — no iteration count is
    /// needed, and a genuinely separate `oo::objdefine` block on the same
    /// object (a different offset) still accumulates.
    fn record_object_methods(
        &mut self,
        key: String,
        methods: &[super::types::MethodDef],
        objdefine_offset: u32,
    ) {
        let recorded = self.result.object_methods.entry(key).or_default();
        for def in methods {
            if recorded
                .iter()
                .any(|m| m.objdefine_offset == objdefine_offset && m.def.name == def.name)
            {
                continue;
            }
            recorded.push(super::types::ObjectMethodDef {
                def: def.clone(),
                objdefine_offset,
            });
        }
    }

    /// Classify one possible class-manufacturing call and resolve its layout.
    /// Both registry definers and published user-metaclass factories converge
    /// here, so the main recorder only handles the resulting structural data.
    fn resolve_class_creation(
        &self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        cmd_tok: Token,
    ) -> Option<ResolvedClassCreation> {
        if args.is_empty() || arg_tokens.is_empty() {
            return None;
        }
        let registry_definer = self
            .registry
            .as_ref()
            .and_then(|registry| registry.get(cmd_name))
            .filter(|spec| spec.traits.contains(tcl_registry::Traits::IS_OO_METACLASS))
            .and_then(|spec| spec.definition_body)
            .filter(|grammar| grammar.family == tcl_registry::definer::DefinerFamily::TclOo);
        let user_factory = if registry_definer.is_some() {
            None
        } else {
            Some(self.class_factory_for_command(
                cmd_name,
                arg_tokens,
                scope_path,
                cmd_tok.span.start(),
            )?)
        };
        let grammar = registry_definer.or_else(|| {
            user_factory
                .as_ref()
                .and_then(|factory| self.definition_grammar(&factory.root_metaclass))
        });
        let manufacturer = match user_factory.as_ref() {
            None => self
                .registry
                .as_deref()
                .and_then(|registry| registry.exported_manufacturer_method(cmd_name, &args[0])),
            Some(_) => grammar.and_then(|grammar| grammar.manufacturer(&args[0])),
        }?;
        if user_factory.as_ref().is_some_and(|factory| {
            !factory
                .exported_manufacturers
                .contains(manufacturer.keyword)
        }) {
            return None;
        }
        let layout = match user_factory.as_ref() {
            None => ManufacturerLayout::builtin(manufacturer),
            Some(factory) => manufacturer_layout(factory, manufacturer, args, arg_tokens),
        }?;
        Some(ResolvedClassCreation {
            user_factory,
            layout,
        })
    }

    /// Handle `oo::class create NAME ?BODY?` — record the class.
    ///
    /// Records a [`super::types::ClassDef`] in ``result.all_classes``
    /// (walking the body when present to populate method / mixin /
    /// superclass info) so consumers see the class in the workspace
    /// index. Returns `true` when the command shape matched.
    pub fn handle_oo_class_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        cmd_tok: Token,
    ) -> bool {
        // Every metaclass name behaves the same shape — ``cmd_name
        // create Name ?body?`` — so the cmd-name guard widens to the full set
        // and the recorded ``metaclass`` field still distinguishes
        // them downstream (hover / outline / class-hierarchy).
        // A fully-qualified head (`::oo::class`) names the same global command
        // as its bare form, so strip a single leading `::` before matching (and
        // use the bare form for the recorded `metaclass`, so both spellings
        // produce an identical `ClassDef`).
        let cmd_name = cmd_name.strip_prefix("::").unwrap_or(cmd_name);
        // Which commands are TclOO class *creators* is registry data, not a
        // hardcoded name list: the `IS_OO_METACLASS` trait and a
        // `TclOo`-family definition-body grammar.  Both are required:
        //  - the trait alone includes `oo::object`, which has it but makes
        //    *instances*, not classes (no definition body);
        //  - the grammar alone includes `oo::define` / `oo::objdefine`, which
        //    *extend* an existing class named at `args[0]` — so `oo::define
        //    create method …` (a class literally named `create`) must not be
        //    mistaken for a creation and stolen from `handle_oo_define_command`.
        // define/objdefine carry no `IS_OO_METACLASS`, so they fall through.
        //
        // …but *being* a metaclass is a property of the class, not of the
        // registry: `oo::class create Megawidget { superclass ::oo::class … }`
        // makes `Megawidget` every bit as much a class factory as `oo::class`
        // itself, and Tk's own `library/megawidget.tcl` builds `SimpleWidget`
        // / `FocusableWidget` / `IconList` that way (issue #923 idx 96/97).
        // A registry lookup alone can never see it, so metaclass-ness also
        // propagates down the recorded superclass chain — the *seed* is still
        // registry data (`IS_OO_METACLASS`), only the inheritance step is
        // `TclOO` language semantics.
        //
        // The factory description is read off the metaclass's own recorded
        // `ClassDef` — this file's, or, when the metaclass is written in
        // another document, the workspace factory index the host supplied
        // (issue #1276).  Either way it is a fact *proved where the metaclass
        // was written*, never one inferred from this call's shape: with no
        // such record `X create Name Supers Body` stays indistinguishable
        // from `interp create` and the walk abstains, as before.
        let Some(ResolvedClassCreation {
            user_factory,
            layout,
        }) = self.resolve_class_creation(cmd_name, args, arg_tokens, scope_path, cmd_tok)
        else {
            // Anonymous class manufacturers have no source-level class name
            // to put in `all_classes`; ordinary instance manufacturers have
            // no class-definition body. Both are handled by their respective
            // object-value paths instead.
            return false;
        };
        if layout.name_arg >= args.len() || layout.name_arg >= arg_tokens.len() {
            return false;
        }
        let raw_name = &args[layout.name_arg];
        // Home the class to the command-resolution namespace (see the same
        // reasoning in `handle_proc_command`): a class created inside a
        // qualified-name proc's body homes to that proc's defining namespace,
        // not the lexical global, so it can't overwrite a same-named global
        // class in `all_classes`.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let qualified = qualify(&ns_prefix, raw_name);
        let simple = crate::naming::key_tail(&qualified).to_string();
        let name_span = arg_tokens[layout.name_arg].span;
        // **W314** — the class name has no absolute written form (#934).
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok_opt = arg_tokens.get(layout.body_arg).copied();
        let body_span = body_tok_opt.map_or(name_span, |t| t.span);
        let doc = std::mem::take(&mut self.last_comment);
        let mut class = super::types::ClassDef {
            name: simple,
            qualified_name: qualified.clone(),
            name_span,
            body_span,
            metaclass: cmd_name.to_string(),
            doc,
            inheritance_unknown: layout.inheritance_unknown,
            ..Default::default()
        };
        // A user metaclass has no spec of its own, so the bodies it makes are
        // governed by the definition grammar of the registry metaclass at the
        // root of its superclass chain — the same `TclOO` grammar, reached
        // without naming it here.
        let grammar = user_factory.as_ref().map_or_else(
            || self.definition_grammar(cmd_name),
            |factory| self.definition_grammar(&factory.root_metaclass),
        );
        // Members the manufacturer itself injects (Tk's `Megawidget` splices
        // `superclass ::tk::MegawidgetClass` plus whatever the caller passed)
        // are applied through the *same* registry-grammar routing as a member
        // written in the body, so no member keyword is known here by name.
        if let Some(grammar) = grammar {
            for injected in &layout.injected {
                super::oo::apply_oo_subcommand_in(
                    grammar,
                    &injected.texts,
                    &injected.argv,
                    &mut class,
                    self.profile.availability_mask,
                );
            }
        }
        // Walk the class body when present — populates
        // ``superclasses`` / ``mixins`` / ``methods`` /
        // ``class_methods`` from the OO-define subcommands.
        if let (Some(body_text), Some(body_tok)) = (args.get(layout.body_arg), body_tok_opt) {
            let definer_disabled = self.command_dialect_disabled(cmd_name);
            self.parse_oo_definition_body(
                body_text,
                body_tok,
                &mut class,
                scope_path,
                grammar,
                definer_disabled,
            );
        }
        // **W315** — a retraction the body could not legally make (issue
        // #1120). Drained here, after the whole body walk, so a class extended
        // by several `oo::define` blocks reports each block's own aborts once.
        self.emit_w315_definition_cannot_run(&mut class);
        // Is the class we just recorded *itself* a class factory?  Answered
        // here, once, so every later `ThisClass create …` — in this file or,
        // via the workspace factory index, in another — reads a derived fact
        // instead of re-walking the superclass chain and re-segmenting the
        // manufacturer override per call site (issue #1276).
        class.factory = self.class_factory_of(&qualified, &class);
        // …and does the *manufacturer* let this class be constructed by a
        // bare word (Tk's `::tk::IconList .il`)?  Settled here too, for the
        // same reason: the proof lives on the metaclass, which a consuming
        // document may never see, so the answer travels on the class rather
        // than the question (issue #1303).
        if user_factory
            .as_ref()
            .is_some_and(|factory| factory.unknown_binds_instance)
        {
            class.class_command_fallback =
                super::types::ClassCommandFallback::ConstructsNamedInstance;
        }
        // Register globally and in the current scope, the same as
        // the proc registration path: ``result.all_classes`` is keyed
        // by the fully-qualified name; the per-scope
        // ``scope.classes`` map is keyed by the bare (unqualified)
        // name so per-scope lookups and shadowing rules work.
        let simple_key = class.name.clone();
        // The creation site's own body span, for `my`-dispatch resolution
        // to find (issue #923 idx 52) — see `class_body_spans`'s doc.
        self.result
            .class_body_spans
            .push((qualified.clone(), class.body_span));
        self.result.all_classes.insert(qualified, class.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.classes.insert(simple_key, class);
        }
        true
    }

    /// Handle `oo::define CLASS ?BODY?` — record an extension to
    /// an existing class.
    ///
    /// Looks up the class by qualified name in
    /// ``result.all_classes``; when found, walks the body or
    /// inline-form arguments via the OO walkers in
    /// [`super::oo`] to extend ``superclasses`` / ``mixins`` /
    /// ``methods`` / ``class_methods``.  When the class isn't
    /// in the index yet (e.g. the class definition lives in a
    /// separate file the workspace index hasn't reached), a
    /// stub ``ClassDef`` is created so subsequent
    /// ``oo::define`` calls + the workspace index see a
    /// consistent record.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::OoDefine`]; `cmd_name` is
    /// the invocation's own head (always the stamped `oo::define`
    /// spelling), used only to look its definition grammar back up.
    pub fn handle_oo_define_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        let qualified = self.oo_define_qualified_target(args, arg_tokens, arg_single, scope_path);

        // Distinguish body-form from inline-form by inspecting
        // ``args[1]``.  Body-form: ``oo::define Class { ... }``
        // — args[1] is a single body argument.  Inline-form:
        // ``oo::define Class method foo {} {}`` — args[1] is a
        // known define subcommand.
        if args.len() < 2 {
            return true;
        }

        // Look up or create the partial ClassDef in
        // ``result.all_classes``. The ``name`` field carries the
        // bare tail even when the source declared the class
        // qualified (``oo::define ::ns::Other``), the same
        // ``simple`` extraction as ``handle_oo_class_command``.
        let simple = crate::naming::key_tail(&qualified).to_string();
        // The class-name token's own span, and this specific `oo::define`
        // invocation's own extent — from the class-name token's start to
        // the last argument token's end, covering whichever of the
        // inline-form tail or the `{ body }` token is present. The *whole*
        // invocation, not just the class-name token, so any member this
        // call adds (inline `method`/`property`/… words, or a `{ … }`
        // body) stays nested inside it for document-symbol rendering — a
        // name-token-sized span would leave every subsequently-added
        // member "escaping" its own parent's range, a self-contradictory,
        // checkable-from-source-alone structural error.
        //
        // Recorded in `class_body_spans` for *every* `oo::define` call,
        // not just when creating a fresh stub (issue #923 idx 52): a class
        // extended via a *separate* `oo::define ClassName { ... }` block
        // has its methods living inside THIS span, textually disjoint from
        // the class's original `oo::class create` block (or an earlier
        // `oo::define`), so `my`-dispatch resolution needs every
        // contributing span on file, not just the first one recorded.
        let name_span = arg_tokens.first().map_or(
            super::types::Scope::default()
                .body_span
                .unwrap_or_else(|| tcl_lexer::Span::new(0, 0)),
            |t| t.span,
        );
        let this_call_span = arg_tokens.last().map_or(name_span, |last| {
            tcl_lexer::Span::new(name_span.start(), last.span.end())
        });
        self.result
            .class_body_spans
            .push((qualified.clone(), this_call_span));
        let mut class_def = self
            .result
            .all_classes
            .remove(&qualified)
            .unwrap_or_else(|| super::types::ClassDef {
                name: simple,
                qualified_name: qualified.clone(),
                name_span,
                body_span: this_call_span,
                // An `oo::define` on a class not created in this file — a
                // cross-file extension stub, not the class's definition.
                via_define: true,
                ..Default::default()
            });

        self.walk_oo_define_form(cmd_name, args, arg_tokens, scope_path, &mut class_def);

        // **W315** — as on the creation path, except that a `via_define` stub's
        // aborts are dropped rather than reported: a class created in another
        // file leaves this record with no member tables to judge against.
        self.emit_w315_definition_cannot_run(&mut class_def);
        // `oo::define C { superclass oo::class }` turns an ordinary class into
        // a factory, and `self method create …` changes the shape of the
        // classes it makes, so the derived factory record is recomputed here
        // exactly as it is on the creation path (issue #1276).
        class_def.factory = self.class_factory_of(&qualified, &class_def);
        self.register_defined_class(qualified, class_def, scope_path);
        true
    }

    /// Note that the command whose name is being moved/bound loads an external
    /// unit — a `source`, a `load`, an `auto_load` — and, if so, widen
    /// [`AnalysisResult::has_dynamic_providers`](super::types::AnalysisResult::has_dynamic_providers).
    ///
    /// Hook dispatch keys off the **written** head, so once `source` has been
    /// renamed or aliased to another name, calls through that name no longer
    /// reach [`Self::handle_source_command`] and the files they pull in become
    /// invisible. Rather than let W120 / W123 then confidently report a package
    /// or command "missing" that the moved command loads, this widens to the
    /// same unknowable state a dynamic `namespace import` pattern or a
    /// `namespace unknown` handler produces — issue #1332's option (2), applied
    /// to the one case option (1) cannot reach.
    ///
    /// The test is [`Traits::LOADS_EXTERNAL_UNIT`](tcl_registry::Traits), read
    /// off the spec: no command name appears here, so `load` and `auto_load`
    /// are covered by the same code that covers `source`, and a dialect that
    /// adds another file-loading command is covered by declaring the trait.
    fn note_external_unit_command_moved(&mut self, name: &str) {
        let bare = name.trim_start_matches("::");
        if self
            .registry
            .as_deref()
            .and_then(|r| r.get(bare))
            .is_some_and(|spec| {
                spec.traits
                    .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
            })
        {
            self.result.has_dynamic_providers = true;
        }
    }

    /// How many of `args`' leading words the **top-level** command `cmd_name`
    /// consumes as options, entirely from registry data — the whole-command
    /// counterpart of [`Self::namespace_leading_flag_words`], which answers the
    /// same question for a `namespace` *subcommand*.
    ///
    /// A leading word counts when it matches one of the command's
    /// profile-available [`OptionSpec`](tcl_registry::OptionSpec)s, and a
    /// value-taking option (`source -encoding utf-8 f.tcl`) consumes its value
    /// word too.  The scan stops at the first word that is not a declared
    /// option, so the first *positional* is what remains — which is the only
    /// thing callers want to know.
    ///
    /// Matching is exact (`OptionSpec::matches`: canonical name or a declared
    /// alias), never the generic unique-prefix rule, for the same reason
    /// [`Self::namespace_leading_flag_words`] is exact: these are hand-parsed
    /// in C and an abbreviation is not accepted.
    ///
    /// `0` when no registry is attached (the analyser runs registry-less in
    /// some unit tests) or the command has no declared options — a flagless
    /// read then treats every word as positional, which is the pre-registry
    /// behaviour.
    fn leading_option_words(&self, cmd_name: &str, args: &[String]) -> usize {
        use tcl_registry::ProfileQueries;
        let Some(spec) = self.registry.as_deref().and_then(|r| r.get(cmd_name)) else {
            return 0;
        };
        let options = self.profile.available_option_specs(spec);
        if options.is_empty() {
            return 0;
        }
        let mut consumed = 0usize;
        while let Some(word) = args.get(consumed) {
            let Some(opt) = options.iter().find(|o| o.matches(word.as_str())) else {
                break;
            };
            // A value-taking option eats the next word as well — but only when
            // that word is actually present, so a truncated `source -encoding`
            // does not report more words than were written.
            let width = 1 + usize::from(opt.takes_value());
            if consumed + width > args.len() {
                break;
            }
            consumed += width;
        }
        consumed
    }

    /// Keep the global class index and the enclosing lexical scope's class
    /// map in lockstep for any definition form that has produced a class fact.
    fn register_defined_class(
        &mut self,
        qualified: String,
        class_def: super::types::ClassDef,
        scope_path: &[usize],
    ) {
        let simple = class_def.name.clone();
        self.result.all_classes.insert(qualified, class_def.clone());
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, scope_path) {
            scope.classes.insert(simple, class_def);
        }
    }

    /// Resolve the `oo::define` target to its class key. A statically known
    /// substitution retains its actual class name; an unresolved dynamic word
    /// gets a call-site-specific synthetic key so unrelated definitions never
    /// merge state merely because they spell the same variable.
    fn oo_define_qualified_target(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> String {
        let resolved = self
            .resolve_dynamic_word(
                &args[0],
                arg_tokens.first().copied(),
                arg_single.first().copied().unwrap_or(false),
                scope_path,
            )
            .filter(|name| !crate::naming::is_dynamic_word(name));
        let raw = resolved.as_ref().unwrap_or(&args[0]);
        let dynamic_key = crate::naming::is_dynamic_word(raw).then(|| {
            arg_tokens.first().map_or_else(
                || raw.clone(),
                |token| self.mint_synthetic_offset_name("@dynclass@", token.span.start()),
            )
        });
        let namespace = self.command_resolution_namespace(scope_path);
        qualify(
            namespace.trim_start_matches(':'),
            dynamic_key.as_deref().unwrap_or(raw),
        )
    }

    /// Apply an `oo::define` inline member or braced body through the selected
    /// definition grammar. Both forms receive identical dialect diagnostics
    /// and registry-driven command-reference recording.
    fn walk_oo_define_form(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        class_def: &mut super::types::ClassDef,
    ) {
        let inline_form = self
            .definition_grammar("oo::define")
            .is_some_and(|grammar| grammar.is_member(&args[1]));
        if inline_form {
            let inline_args: Vec<String> = args[1..].to_vec();
            let inline_tokens: Vec<Token> = arg_tokens.iter().skip(1).copied().collect();
            if let Some(grammar) = self.definition_grammar(cmd_name) {
                let definer_disabled = self.command_dialect_disabled(cmd_name);
                if let (Some(subcmd), Some(token)) = (inline_args.first(), inline_tokens.first()) {
                    self.emit_w002_oo_member_disabled(grammar, subcmd, *token, definer_disabled);
                }
                if let Some(member) = inline_args
                    .first()
                    .and_then(|subcmd| grammar.member(subcmd))
                {
                    self.emit_w002_oo_member_option_disabled(
                        member,
                        &inline_args[1..],
                        &inline_tokens[1..],
                        definer_disabled,
                    );
                }
                super::oo::parse_oo_define_inline_in(
                    self,
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    class_def,
                    self.profile.availability_mask,
                );
                self.record_member_command_references(
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    scope_path,
                );
            }
        } else if let Some(body_tok) = arg_tokens.get(1).copied() {
            let grammar = self.definition_grammar(cmd_name);
            let definer_disabled = self.command_dialect_disabled(cmd_name);
            self.parse_oo_definition_body(
                &args[1],
                body_tok,
                class_def,
                scope_path,
                grammar,
                definer_disabled,
            );
        }
    }

    /// Record `source ?-encoding ENC? FILE` invocations.
    ///
    /// Which leading words are options — and therefore which word is the
    /// script path — comes from `source`'s own [`OptionSpec`
    /// list](tcl_registry::CommandSpec::options) via
    /// [`Self::leading_option_words`], not a `-encoding` literal here.  The
    /// option carries `DialectSet::TCL85_PLUS` in the registry, so the
    /// dialect gating comes along for free: under an 8.4 profile — which has
    /// no `-encoding` — the word is not recognised as an option and the path
    /// word is read at index 0, matching that spec's own declaration rather
    /// than a hardcoded assumption here.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Source`].
    pub fn handle_source_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        let file_idx = self.leading_option_words("source", args);
        if file_idx >= args.len() || file_idx >= arg_tokens.len() {
            return;
        }
        let st = arg_tokens[file_idx];
        let path = &args[file_idx];
        let is_single = arg_single.get(file_idx).copied().unwrap_or(false);
        // A dynamic path — a bare `$var` or a concatenation like
        // `${dir}lit.tcl` — is first tried against the same last-write-wins
        // constant-string lattice already proven for `rename`'s OLD/NEW
        // words (issue #923 idx 3): the real corpus idiom is `set p
        // "e.tcl"; source $p`, a few lines apart in the same file, which
        // should resolve exactly like the literal `source e.tcl` it's
        // equivalent to — instead of going untracked and leaking the
        // sourced file's definitions into every caller as if unconditionally
        // global (issue #923 idx 46). A `[...]` command substitution
        // anywhere in the word (e.g. `[file join $dir lit.tcl]`) still
        // can't be folded this way — `resolve_dynamic_word` rejects it
        // outright, same as a variable whose value originates in a
        // different file — both stay conservatively dynamic, falling
        // through to `evaluate_auto_path_expr`'s narrower `[info script]`
        // subset unchanged.
        let (path, is_lit) = match self.resolve_dynamic_word(path, Some(st), is_single, scope_path)
        {
            Some(resolved) => {
                let is_lit = !crate::naming::is_dynamic_word(&resolved);
                (resolved, is_lit)
            }
            None => (path.clone(), false),
        };
        // `source` evaluates the file in the caller's current namespace (M9):
        // record the command-resolution namespace at this call site so the
        // workspace index can re-home the sourced document's definitions.
        let site_namespace = self.command_resolution_namespace(scope_path);
        self.result
            .source_targets
            .push(crate::signature_scan::types::SignatureSource {
                raw_path: path,
                range: st.span,
                is_literal: is_lit,
                site_namespace,
            });
    }

    /// Record `namespace import ?-force? PATTERN ...` declarations.
    ///
    /// Literal patterns are recorded in ``result.namespace_imports`` (the
    /// W123 unresolved-command pass suppresses an unqualified call whose
    /// name glob-matches a recorded pattern's tail); a *dynamic* pattern
    /// (``$``/``[…]`` substitution) can't be qualified statically, so it
    /// flips ``has_dynamic_providers`` instead — the imported namespace may
    /// provide any name at runtime, which suppresses W120/W123 file-wide.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceImport`] (stamped
    /// on `namespace`'s `import` subcommand); `args[0]` is still the
    /// subcommand word.
    pub fn handle_namespace_import_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        // Skip the subcommand word + the leading flag words the registry says
        // `namespace import` consumes (`-force`, at most one). Both facts are
        // read from the spec rather than matched by name or bounded by a
        // literal here — see [`Self::namespace_leading_flag_words`]. That the
        // option word was consumed *is* `-force` (`IMPORT_OPTIONS` declares
        // exactly one option and `max_leading_option_words` caps it at one),
        // so the conflict semantics below need no name match either — the
        // same reading `namespace export`'s `-clear` tombstone already gets.
        let flag_words = self.namespace_leading_flag_words("import", &args[1..]);
        let forced = flag_words > 0;
        let mut idx = 1 + flag_words;
        // `namespace import` imports into the namespace current at the call —
        // the command-resolution namespace, so an import inside
        // `proc ::ns::p {}` lands in `::ns` (issue #923 idx 85).
        let importing_ns = self.command_resolution_namespace(scope_path);
        while idx < args.len() && idx < arg_tokens.len() {
            let pat_raw = args[idx].clone();
            // Patterns containing ``$`` / ``[`` substitutions can't be
            // statically qualified — a runtime-computed import makes the
            // available command set unknowable.
            if pat_raw.contains('$') || pat_raw.contains('[') {
                self.result.has_dynamic_providers = true;
                idx += 1;
                continue;
            }
            // Patterns that already start with ``::`` are
            // absolute; relative patterns (``bar::*`` or
            // ``foo``) qualify against the *current* namespace
            // — inside ``namespace eval my { namespace import
            // bar::* }`` this becomes ``::my::bar::*``.
            let pat = if pat_raw.starts_with("::") {
                pat_raw
            } else if importing_ns == "::" {
                format!("::{pat_raw}")
            } else {
                format!("{importing_ns}::{pat_raw}")
            };
            self.result.namespace_imports.push(
                crate::signature_scan::types::SignatureNamespaceImport {
                    ns: importing_ns.clone(),
                    pattern: pat,
                    range: arg_tokens[idx].span,
                    conjectured: false,
                    forced,
                },
            );
            idx += 1;
        }
    }

    /// Record `namespace forget ?PATTERN ...?` events — the removal half of
    /// the import edge's lifecycle log (issue #1103).
    ///
    /// A `namespace import` is not a permanent name: `namespace forget`
    /// removes the alias and a later bare call raises `invalid command name`
    /// (oracle in
    /// [`crate::signature_scan::types::SignatureNamespaceForget`], tclsh
    /// 8.6.14 / 9.0.4 byte-identical). Recorded as an ordered event beside
    /// the `namespace export` tombstones so the resolvers in `tcl-lsp-core`
    /// can ask "does this namespace still hold an alias for NAME *here*?"
    /// rather than "was one ever installed"
    /// (`tcl_lsp_core::namespace_import::alias_live_at`).
    ///
    /// The pattern shapes follow `Tcl_ForgetImport` exactly: a qualified
    /// pattern names the *source* namespace whose commands lose their import
    /// here, a simple one matches this namespace's own imported command
    /// names whatever their origin. Both are oracle-confirmed; see the record
    /// type.
    ///
    /// A dynamic pattern (`$`/`[` substitution) can't be statically resolved
    /// to a glob text and is skipped rather than guessed at: revoking an
    /// alias on a guess would silently drop real references, the same
    /// abstain-toward-answering direction the unordered `-clear` takes.
    /// `namespace forget` declares no options, so — unlike `import` /
    /// `export` — there is no leading flag word to consume; the registry's
    /// own [`Self::namespace_leading_flag_words`] is still what says so.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceForget`] (stamped on
    /// `namespace`'s `forget` subcommand); `args[0]` is still the subcommand
    /// word.
    pub fn handle_namespace_forget_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        let mut idx = 1 + self.namespace_leading_flag_words("forget", &args[1..]);
        // As for `import` / `export`: the namespace losing the aliases is the
        // one current at the call, not the lexically enclosing one.
        let forgetting_ns = self.command_resolution_namespace(scope_path);
        while idx < args.len() && idx < arg_tokens.len() {
            let raw = &args[idx];
            if raw.contains('$') || raw.contains('[') {
                idx += 1;
                continue;
            }
            let (source_ns, pattern) = match raw.rsplit_once("::") {
                Some((prefix, tail)) => {
                    let prefix = if prefix.is_empty() {
                        "::".to_string()
                    } else if prefix.starts_with("::") {
                        prefix.to_string()
                    } else if forgetting_ns == "::" {
                        format!("::{prefix}")
                    } else {
                        format!("{forgetting_ns}::{prefix}")
                    };
                    (Some(prefix), tail.to_string())
                }
                None => (None, raw.clone()),
            };
            self.result.namespace_forgets.push(
                crate::signature_scan::types::SignatureNamespaceForget {
                    ns: forgetting_ns.clone(),
                    source_ns,
                    pattern,
                    range: arg_tokens[idx].span,
                },
            );
            idx += 1;
        }
    }

    /// Record `namespace export ?-clear? PATTERN ...` declarations.
    ///
    /// Real Tcl's `namespace import NS::*` only ever imports names `NS` has
    /// actually exported (`Tcl_Export`, `tclNamesp.c`) — an unexported
    /// sibling command living in `NS` is not reachable through the import at
    /// all (tclsh9.0/8.6-verified: `invalid command name` calling it bare).
    /// Recorded in
    /// `result.namespace_exports` so the wildcard-import bareword resolvers
    /// in `tcl-lsp-core` (same-document) and the workspace index
    /// (cross-document) can gate a would-be import target on whether its
    /// source namespace actually exports it (issue #923 idx 18).
    ///
    /// `-clear` is recorded as an ordered **tombstone** entry rather than
    /// applied by dropping the namespace's earlier entries: an import
    /// snapshots the export list as it stood when the import ran, so a
    /// `-clear` written *after* an import must not revoke what that import
    /// already bound (issue #1027, oracle in
    /// [`crate::signature_scan::types::SignatureNamespaceExport`]).
    /// Collapsing it eagerly — as this handler originally did — destroys
    /// exactly the ordering the snapshot needs. Consumers reconstruct the
    /// export set as of an offset with
    /// `tcl_lsp_core::namespace_import::exported_at_import_site`.
    ///
    /// A dynamic pattern (`$`/`[` substitution) can't be statically resolved
    /// to a glob text, so it is silently skipped — the wildcard-import
    /// resolver then correctly abstains for names it might have covered,
    /// rather than guessing. A dynamic word cannot hide a `-clear`, which is
    /// a literal flag word or nothing.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceExport`] (stamped on
    /// `namespace`'s `export` subcommand); `args[0]` is still the subcommand
    /// word.
    pub fn handle_namespace_export_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        let mut idx = 1;
        // As for `import`: the exporting namespace is the one current at the
        // call, not the lexically enclosing one (issue #923 idx 85).
        let exporting_ns = self.command_resolution_namespace(scope_path);
        // Which leading words are flags — and how many of them the command
        // consumes — is registry data (`EXPORT_OPTIONS` plus
        // `max_leading_option_words`), not a name match or a loop bound here.
        // `-clear` is the only option and at most one is consumed, so a
        // second `-clear` falls through to the pattern loop below and is
        // recorded as the export pattern real Tcl treats it as. Each consumed
        // flag lands as a tombstone event at its own token so the ordering
        // survives.
        let flag_words = self.namespace_leading_flag_words("export", &args[1..]);
        for _ in 0..flag_words {
            if idx >= args.len() || idx >= arg_tokens.len() {
                break;
            }
            self.result.namespace_exports.push(
                crate::signature_scan::types::SignatureNamespaceExport {
                    ns: exporting_ns.clone(),
                    pattern: String::new(),
                    range: arg_tokens[idx].span,
                    clears: true,
                },
            );
            idx += 1;
        }
        while idx < args.len() && idx < arg_tokens.len() {
            let pattern = args[idx].clone();
            if pattern.contains('$') || pattern.contains('[') {
                idx += 1;
                continue;
            }
            self.result.namespace_exports.push(
                crate::signature_scan::types::SignatureNamespaceExport {
                    ns: exporting_ns.clone(),
                    pattern,
                    range: arg_tokens[idx].span,
                    clears: false,
                },
            );
            idx += 1;
        }
    }

    /// How many of `args`' leading words `namespace SUB` consumes as option
    /// flags, entirely from registry data: a word that matches a declared
    /// flag option of that subcommand, capped by the subcommand's declared
    /// [`tcl_registry::SubCommand::max_leading_option_words`].
    ///
    /// `args` is the word list *after* the subcommand word.
    ///
    /// Two registry facts, no literal in the walker:
    ///
    /// - **Which** words are flags — `namespace import`'s `-force` and
    ///   `namespace export`'s `-clear` (`IMPORT_OPTIONS` / `EXPORT_OPTIONS`).
    ///   Matching is exact (`OptionSpec::matches`, canonical name or declared
    ///   alias), never the generic unique-prefix rule, because both
    ///   subcommands hand-parse with `strcmp` in C: oracle (tclsh 8.6.14 /
    ///   9.0.4) `namespace export -c p` exports `-c` and `p`, and `namespace
    ///   import -f ::src::p` aborts with `no namespace specified in import
    ///   pattern "-f"`.
    /// - **How many** — one, declared as `max_leading_option_words`. A second
    ///   `-clear` is an ordinary export pattern (and `-clear` is a perfectly
    ///   valid, importable command name); a second `-force` is an import
    ///   pattern that aborts the script.
    ///
    /// The scan also stops at the first non-flag word, which is what makes
    /// `namespace export a -clear` export both (the flag is only ever the
    /// first word — oracle-verified).
    ///
    /// `0` when no registry is attached (the analyser runs registry-less in
    /// some unit tests): a flagless read then treats `-clear` as an ordinary
    /// pattern, which is inert — nothing else records a command by that name.
    fn namespace_leading_flag_words(&self, sub: &str, args: &[String]) -> usize {
        use tcl_registry::ProfileQueries;
        let Some((spec, sub_spec)) = self
            .registry
            .as_deref()
            .and_then(|r| r.get("namespace"))
            .and_then(|spec| spec.subcommand(sub).map(|s| (spec, s)))
        else {
            return 0;
        };
        let options = self.profile.available_sub_option_specs(spec, sub_spec);
        let cap = sub_spec
            .max_leading_option_words
            .map_or(usize::MAX, usize::from);
        args.iter()
            .take(cap)
            .take_while(|w| {
                options
                    .iter()
                    .any(|o| o.matches(w.as_str()) && !o.takes_value())
            })
            .count()
    }

    /// Handle `namespace unknown HANDLER` — installing a per-namespace
    /// unknown handler (TIP 181, `NamespaceUnknownCmd` in `tclNamesp.c`)
    /// makes command resolution unknowable, exactly like a dynamic user
    /// `proc unknown`: the handler runs for every failed lookup and may
    /// resolve anything.  Flips ``has_dynamic_providers``, which suppresses
    /// W120/W123 file-wide (matching the proc-unknown behaviour).
    ///
    /// Only the *setter* form suppresses: the bare query (`namespace
    /// unknown`) installs nothing, and an empty handler (`namespace unknown
    /// {}`) resets to the default lookup (`Tcl_SetNamespaceUnknownHandler`
    /// treats an empty list as a reset).
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::NamespaceUnknown`]
    /// (stamped on `namespace`'s `unknown` subcommand); `args[0]` is
    /// still the subcommand word.
    pub fn handle_namespace_unknown_command(&mut self, args: &[String]) {
        if args.len() < 2 {
            return;
        }
        if args[1].trim().is_empty() {
            return;
        }
        self.result.has_dynamic_providers = true;
    }

    /// Recognise the tcllib ``<NS>::import <ALIAS>`` wrapper idiom
    /// and record a conjectured `NamespaceImport`.
    ///
    /// A common tcllib convention is a proc ``some::ns::import`` that
    /// ``uplevel``s ``namespace eval <alias> {namespace import some::ns::*}``.
    /// Statically detecting the wrapper bodies is out of scope, but
    /// the call shape is unambiguous — its qualified name ends in
    /// ``::import`` and its single argument is a static namespace
    /// identifier.
    pub fn handle_tcllib_import_wrapper(
        &mut self,
        cmd_name: &str,
        cmd_tok: Token,
        args: &[String],
        scope_path: &[usize],
    ) {
        // Wrapper shape: ``X::import <alias>`` with exactly one
        // static argument.  Tcl's own ``namespace import`` is
        // handled separately and never falls into this branch
        // because ``cmd_name`` is ``namespace``, not ``…::import``.
        if !cmd_name.ends_with("::import") {
            return;
        }
        if args.len() != 1 {
            return;
        }
        let alias = &args[0];
        if alias.is_empty() || alias.contains('$') || alias.contains('[') {
            return;
        }
        // Strip the trailing ``::import`` to get the source
        // namespace; absolute-prefix it if missing the leading
        // ``::``.
        let stripped = &cmd_name[..cmd_name.len() - "::import".len()];
        let source_ns = if stripped.starts_with("::") {
            stripped.to_string()
        } else {
            format!("::{stripped}")
        };
        // Relative aliases live under the current namespace —
        // ``namespace eval outer { some::ns::import vt }``
        // creates ``::outer::vt``, not ``::vt``.
        let current_ns = self.command_resolution_namespace(scope_path);
        let alias_ns = if alias.starts_with("::") {
            alias.clone()
        } else if current_ns == "::" {
            format!("::{alias}")
        } else {
            format!("{current_ns}::{alias}")
        };
        self.result.namespace_imports.push(
            crate::signature_scan::types::SignatureNamespaceImport {
                ns: alias_ns,
                pattern: format!("{source_ns}::*"),
                range: cmd_tok.span,
                conjectured: true,
                // The wrapper's own body is not read, so whether the
                // `namespace import` it `uplevel`s carries `-force` is
                // unknown. `false` is the conservative reading for the one
                // thing the flag decides: a conjectured import never claims
                // to have replaced a command the target namespace already
                // holds.
                forced: false,
            },
        );
    }

    /// Record a `lappend auto_path PATH...` mutation.
    ///
    /// Dispatched from the
    /// [`tcl_registry::hooks::AnalyserHookId::Lappend`] arm alongside
    /// the ordinary variable handling — the `auto_path` check is a
    /// *variable-name* shape check, not a command guard.  Any
    /// ``auto_path`` mutation flips ``has_dynamic_providers``:
    /// packages discovered at runtime can register commands the static
    /// analyser can't see, so W123 unknown-command diagnostics
    /// suppress on the document.
    ///
    /// `lappend` appends **one list element per argument word**, so this
    /// records one [`AutoPathForm::Append`] entry per word and each is a whole
    /// directory — `lappend auto_path {p q}` names the single directory
    /// `p q`, not two.  Contrast [`Self::handle_auto_path_set`].
    pub fn handle_auto_path_lappend(&mut self, args: &[String], arg_tokens: &[Token]) {
        if args.first().map(String::as_str) != Some("auto_path") {
            return;
        }
        for (i, path) in args.iter().enumerate().skip(1) {
            let Some(tok) = arg_tokens.get(i) else {
                continue;
            };
            self.result
                .auto_path_entries
                .push(super::types::AutoPathEntry {
                    raw_path: path.clone(),
                    range: tok.span,
                    form: super::types::AutoPathForm::Append,
                });
        }
        self.result.has_dynamic_providers = true;
    }

    /// Record a `set auto_path PATH` mutation.
    ///
    /// Dispatched from the [`tcl_registry::hooks::AnalyserHookId::Set`]
    /// arm; see [`Self::handle_auto_path_lappend`] for why any
    /// ``auto_path`` mutation flips ``has_dynamic_providers``.
    ///
    /// The right-hand side is assigned as a **list**, so the one record this
    /// pushes may name several directories and is tagged
    /// [`AutoPathForm::Assign`] to say so.  The split is deliberately *not*
    /// done here: the record keeps the source word verbatim (its `range` is
    /// the whole argument), and
    /// [`crate::auto_path_eval::evaluate_auto_path_entry`] applies the list
    /// grammar at consumption.
    pub fn handle_auto_path_set(&mut self, args: &[String], arg_tokens: &[Token]) {
        if args.first().map(String::as_str) != Some("auto_path") || args.len() < 2 {
            return;
        }
        if let Some(tok) = arg_tokens.get(1) {
            self.result
                .auto_path_entries
                .push(super::types::AutoPathEntry {
                    raw_path: args[1].clone(),
                    range: tok.span,
                    form: super::types::AutoPathForm::Assign,
                });
            self.result.has_dynamic_providers = true;
        }
    }

    /// Record the pattern arguments of regex-pattern commands
    /// (`PatternType::Regex` specs — `regexp` / `regsub`) for syntax
    /// highlighting.
    ///
    /// Literal patterns (`Esc` / `Str` tokens) are recorded
    /// verbatim; `Var` tokens are resolved via the
    /// `const_strings` map (populated by `set var "..."`) so
    /// `regexp $p $line` records the literal stored in `p`.
    /// `Cmd`-substitution patterns are skipped — runtime-computed
    /// patterns can't be statically resolved.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::RegexPatternCapture`]
    /// (stamped on both `regexp` and `regsub`); `cmd_name` is data —
    /// it labels the recorded pattern — not a guard.
    pub fn handle_regex_pattern_capture(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if args.is_empty() {
            return;
        }
        // Skip leading option flags (`-nocase`, `-line`, `-all`, `-indices`,
        // `-start INDEX`, …) to the pattern arg — the one canonical option-skip.
        let Some(idx) = crate::regex_source::regexp_pattern_index(args) else {
            return;
        };
        if idx >= arg_tokens.len() {
            return;
        }
        let tok = arg_tokens[idx];
        self.record_regex_pattern_token(&args[idx], tok, cmd_name, scope_path);
    }

    /// Record one regex-pattern token for `regexp` / `regsub` /
    /// `switch -regexp`.  Var tokens are resolved against
    /// `const_strings`; `Cmd` substitutions are skipped (no
    /// static resolution); literals are recorded verbatim.
    fn record_regex_pattern_token(
        &mut self,
        text: &str,
        tok: Token,
        command: &str,
        scope_path: &[usize],
    ) {
        match tok.kind {
            TokenType::Cmd => {
                // Runtime-computed patterns can't be statically
                // resolved — skip.  Only ``Var`` tokens are
                // resolved as a substitution branch.
            }
            TokenType::Var => {
                let sm = Analyser::source_map(
                    &self.source,
                    &self.cached_line_index,
                    self.cached_line_index_source_len,
                );
                let var_name = sm.token_text(tok).to_string();
                if let Some((const_val, def_span)) =
                    self.lookup_const_string_with_span(&var_name, scope_path)
                {
                    let pattern = const_val.to_string();
                    // Record the use site (the `$var` token).
                    self.result.regex_patterns.push(super::types::RegexPattern {
                        range: tok.span,
                        pattern: pattern.clone(),
                        command: command.to_string(),
                    });
                    // Also record the defining ``set`` value's
                    // range so the semantic-token provider can
                    // highlight the literal.
                    self.result.regex_patterns.push(super::types::RegexPattern {
                        range: def_span,
                        pattern,
                        command: command.to_string(),
                    });
                    self.regex_vars.insert((scope_path.to_vec(), var_name));
                }
            }
            _ => {
                self.result.regex_patterns.push(super::types::RegexPattern {
                    range: tok.span,
                    pattern: text.to_string(),
                    command: command.to_string(),
                });
            }
        }
    }

    /// Handle the `incr` command: `incr var ?amount?`.
    ///
    /// `incr` is safe-on-uninit (it initialises the variable to 0 if
    /// not yet set), so the var binding is created with
    /// `warn_if_unused = true` — the diagnostic emitter will
    /// still flag a `set`-only-no-read variable, but won't flag
    /// an `incr`-only-no-read one.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Incr`].
    pub fn handle_incr_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if let (Some(name), Some(tok)) = (args.first(), arg_tokens.first()) {
            self.define_var(name, *tok, scope_path, true, None);
        }
    }

    /// Handle `append VARNAME ?value ...?` / `lappend VARNAME ?value ...?`.
    ///
    /// Both read-modify-write their first argument, creating it if absent, so
    /// the target is a variable *definition* for symbol/scope purposes and must
    /// surface in `symbols` / completion / hover (it previously did not).
    /// `warn_if_unused = false` because the command itself reads the prior
    /// value, so an `append`/`lappend` target is never "set but never used"
    /// (no W211 for it).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Append`]
    /// and [`tcl_registry::hooks::AnalyserHookId::Lappend`] (the two
    /// commands share this handler; the `Lappend` arm additionally
    /// records `lappend auto_path …`).
    pub fn handle_append_lappend_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if let (Some(name), Some(tok)) = (args.first(), arg_tokens.first()) {
            self.define_var(name, *tok, scope_path, false, None);
        }
    }

    /// Resolve a command name to the `ProcDef` that implements it, following
    /// Tcl's real bareword command resolution via
    /// [`crate::naming::bareword_resolution_candidates`] — the current
    /// namespace (computed by [`Self::command_resolution_namespace`]) first,
    /// then global, exactly two levels, never every enclosing ancestor
    /// namespace on the scope chain (Tcl's own command lookup does not walk
    /// intermediate namespaces absent an explicit `namespace path`).
    ///
    /// Shared with the optimiser's identical same-file resolution
    /// (`resolve_proc_qname`) and the disabled-command / arity suppression
    /// checks' resolution (`UserResolutionFacts::resolves_to_user`) so the
    /// three can't diverge on the same rule.
    ///
    /// Returns the first matching ``ProcDef`` (by reference into
    /// ``result.all_procs``), or `None` if no candidate is known.
    #[must_use]
    pub fn resolve_proc_call(
        &self,
        cmd_name: &str,
        scope_path: &[usize],
    ) -> Option<&super::types::ProcDef> {
        if cmd_name.is_empty() {
            return None;
        }
        let ns = self.command_resolution_namespace(scope_path);
        crate::naming::bareword_resolution_candidates(&ns, cmd_name)
            .into_iter()
            .find_map(|qname| self.result.all_procs.get(&qname))
    }

    /// Static element count for a `{*}`-expanded word.
    ///
    /// Used by the proc-call arity checker (which lives in
    /// `compiler_checks::arity_checks`) to decide whether a
    /// ``{*}``-expanded argument contributes a statically-known
    /// number of runtime arguments.
    ///
    /// - **Braced literal** (`Str` token, ``{a b c}``) — split
    ///   the token's inner text as a list and return its length.
    /// - **Pure variable reference** (`Var` token, ``$x``) — if
    ///   the variable has a known constant value in the current
    ///   scope chain, split that value and return its length.
    /// - **Anything else** — `None`: count not statically known.
    ///
    /// Refinement is only attempted when ``single_token`` is
    /// `true`; for concatenated words like ``{*}$x$y`` or
    /// ``{*}{a b}$suffix`` the segmenter exposes only the *first*
    /// token, which would otherwise be misinterpreted as a pure
    /// literal or pure var ref.  Token text is resolved via
    /// [`tcl_lexer::SourceMap::token_text`] — the same helper the
    /// rest of the analyser uses — so the inner-content stripping
    /// rules (kind-specific delimiter handling) stay in one
    /// place.
    #[must_use]
    pub fn resolve_expansion_count(
        &self,
        tok: Token,
        single_token: bool,
        scope_path: &[usize],
    ) -> Option<usize> {
        if !single_token {
            return None;
        }
        let sm = Analyser::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        match tok.kind {
            TokenType::Str => {
                let inner = sm.token_text(tok);
                Some(crate::codegen::helpers::split_list_simple(inner).len())
            }
            TokenType::Var => {
                let var_name = sm.token_text(tok);
                let const_val = self.lookup_const_string(var_name, scope_path)?;
                Some(crate::codegen::helpers::split_list_simple(const_val).len())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::Span;

    #[test]
    fn builtin_next_completion_requires_the_actual_mro_end() {
        assert_eq!(
            Analyser::builtin_next_completion(
                &tcl_registry::definer::TCLOO_GRAMMAR,
                "unknown",
                0,
                2,
            ),
            None
        );
        assert_eq!(
            Analyser::builtin_next_completion(
                &tcl_registry::definer::TCLOO_GRAMMAR,
                "unknown",
                1,
                2,
            ),
            Some(true)
        );
    }

    #[test]
    fn dynamic_command_name_patterns_keep_only_proven_fixed_fragments() {
        assert!(!dynamic_command_name_may_equal(
            "${ns}::define::$method",
            "::string"
        ));
        assert!(dynamic_command_name_may_equal("$name", "::string"));
        assert!(dynamic_command_name_may_equal(
            "${ns}::::helper",
            "::T::helper"
        ));
        assert!(dynamic_command_name_may_equal(
            "$array(index)::define",
            "::anything"
        ));
    }

    fn class_observation(superclass: Option<&str>, factory: bool) -> ClassDef {
        ClassDef {
            name: "class".to_owned(),
            qualified_name: "::T::D::class".to_owned(),
            superclasses: superclass.into_iter().map(str::to_owned).collect(),
            factory: factory.then(|| ClassFactory {
                root_metaclass: "oo::class".to_owned(),
                ..ClassFactory::default()
            }),
            ..ClassDef::default()
        }
    }

    #[test]
    fn parameterised_class_join_is_monotone_across_incomplete_observations() {
        let proved = class_observation(Some("::T::Mother"), true);
        let incomplete = class_observation(None, false);

        let later_incomplete = join_parameterised_class_observations(&proved, &incomplete).unwrap();
        assert_eq!(later_incomplete.superclasses, vec!["::T::Mother"]);
        assert!(later_incomplete.factory.is_some());

        let later_proof = join_parameterised_class_observations(&incomplete, &proved).unwrap();
        assert_eq!(later_proof.superclasses, vec!["::T::Mother"]);
        assert!(later_proof.factory.is_some());
    }

    #[test]
    fn parameterised_class_join_abstains_on_conflicting_concrete_relations() {
        let left = class_observation(Some("::T::Mother"), true);
        let right = class_observation(Some("::T::OtherMother"), true);
        assert!(join_parameterised_class_observations(&left, &right).is_none());
    }

    fn esc_tok(span: Span) -> Token {
        Token::new(TokenType::Esc, span)
    }

    fn str_tok(span: Span) -> Token {
        Token {
            kind: TokenType::Str,
            span,
            content_offset: 1,
            in_quote: false,
        }
    }

    fn span(start: u32, end: u32) -> Span {
        Span::new(start, end)
    }

    // interp_create_words_from_value — issue #1025

    #[test]
    fn interp_create_value_words_strip_a_single_braced_path_issue_1025() {
        // TP — `{child}` is one Tcl word naming interpreter `child`; the
        // direct handler sees it segmenter-decoded, so the value-flow path
        // must strip the braces too rather than binding to `"{child}"`.
        assert_eq!(
            interp_create_words_from_value("[interp create {child}]"),
            Some(vec!["child"])
        );
    }

    #[test]
    fn interp_create_value_words_keep_a_nested_path_as_one_word_issue_1025() {
        // TP — `{parent child}` is one word (a descent path), not two.
        // `split_whitespace` used to yield `["{parent", "child}"]`, whose
        // first fragment became the bound key.
        assert_eq!(
            interp_create_words_from_value("[interp create {parent child}]"),
            Some(vec!["parent child"])
        );
    }

    #[test]
    fn interp_create_value_words_unchanged_for_bare_and_flagged_forms_issue_1025() {
        // TN — the shapes that already worked must be byte-for-byte
        // unchanged by the switch to list parsing.
        assert_eq!(
            interp_create_words_from_value("[interp create -safe]"),
            Some(vec!["-safe"])
        );
        assert_eq!(
            interp_create_words_from_value("[interp create -safe -- name]"),
            Some(vec!["-safe", "--", "name"])
        );
        assert_eq!(
            interp_create_words_from_value("[interp create]"),
            Some(vec![])
        );
        assert_eq!(interp_create_words_from_value("[set x 1]"), None);
        assert_eq!(interp_create_words_from_value("interp create child"), None);
    }

    #[test]
    fn interp_create_value_words_reject_an_unmatched_brace_issue_1025() {
        // A malformed (mid-edit) path is not a list at all, so the whole
        // substitution is rejected. Returning the prefix parsed before the
        // error instead — `Some(vec![])` here — read as a bare `interp
        // create`, binding the variable to an auto-named interpreter real
        // Tcl never creates. Later `$i eval` / `interp alias $i …` sites
        // then resolved against that phantom (Codex review, PR #1045).
        assert_eq!(
            interp_create_words_from_value("[interp create {child]"),
            None
        );
        assert_eq!(
            interp_create_words_from_value("[interp create good {child]"),
            None,
            "a valid prefix does not rescue a malformed tail",
        );
    }

    // parse_interp_create_words — full flag scan, mirroring tclInterp.c

    #[test]
    fn interp_create_flags_are_still_scanned_after_the_path() {
        // C Tcl examines every word and treats a `-` word as a flag until
        // `--`, whether or not the path has been read. tclsh8.6/9.0:
        // `interp create x -safe` yields `interp issafe x` == 1, and
        // `interp create n -bogus` errors `bad option "-bogus"` — proof the
        // scan does not stop at the path. Stopping at the path recorded `x`
        // as an *unsafe* interpreter.
        assert_eq!(
            parse_interp_create_words(&["x", "-safe"]),
            (true, Some("x"))
        );
        assert_eq!(
            parse_interp_create_words(&["-safe", "y"]),
            (true, Some("y"))
        );
    }

    #[test]
    fn interp_create_double_dash_ends_flag_parsing() {
        // tclsh8.6/9.0: `interp create -safe -- z` creates a safe `z`, and
        // `interp create -- -safe` creates an *unsafe* interpreter whose
        // path is the literal `-safe`.
        assert_eq!(
            parse_interp_create_words(&["-safe", "--", "z"]),
            (true, Some("z"))
        );
        assert_eq!(
            parse_interp_create_words(&["--", "-safe"]),
            (false, Some("-safe"))
        );
    }

    #[test]
    fn interp_create_two_path_words_record_no_path() {
        // tclsh8.6/9.0: `interp create a b` is `wrong # args`. No
        // interpreter is created, so recording either word would bind a
        // name that does not exist. A `-safe` seen alongside is still
        // reported — it costs nothing and the command creates nothing.
        assert_eq!(parse_interp_create_words(&["a", "b"]), (false, None));
        assert_eq!(
            parse_interp_create_words(&["--", "x", "-safe"]),
            (false, None),
            "after `--` a second `-` word is a path word, so this is the error shape too",
        );
    }

    #[test]
    fn interp_create_bare_and_flag_only_forms_are_unchanged() {
        // TN control for the rewritten scan.
        assert_eq!(parse_interp_create_words(&[]), (false, None));
        assert_eq!(parse_interp_create_words(&["-safe"]), (true, None));
        assert_eq!(
            parse_interp_create_words(&["child"]),
            (false, Some("child"))
        );
    }

    // handle_set_command

    #[test]
    fn handle_set_defines_variable() {
        let mut a = Analyser::new();
        a.handle_set_command(
            &["x".to_string(), "1".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 3))],
            &[true, true],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn handle_set_tracks_single_token_literal_value() {
        let mut a = Analyser::new();
        a.handle_set_command(
            &["x".to_string(), "hello".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 7))],
            &[true, true],
            &[],
        );
        assert_eq!(a.lookup_const_string("x", &[]), Some("hello"));
    }

    #[test]
    fn handle_set_tracks_braced_string_value() {
        let mut a = Analyser::new();
        a.handle_set_command(
            &["x".to_string(), "hello world".to_string()],
            &[esc_tok(span(0, 1)), str_tok(span(2, 15))],
            &[true, true],
            &[],
        );
        assert_eq!(a.lookup_const_string("x", &[]), Some("hello world"));
    }

    #[test]
    fn handle_set_clears_const_string_for_interpolated_value() {
        let mut a = Analyser::new();
        // Pre-seed a constant tracking entry.
        a.set_const_string("x", "old".to_string(), span(0, 0), &[]);
        // Re-assign with a multi-token (interpolation) value —
        // single_token_word[1] is false, so const_string is cleared.
        a.handle_set_command(
            &["x".to_string(), "$other".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 8))],
            &[true, false],
            &[],
        );
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn handle_set_no_value_records_read_not_definition() {
        // ``set x`` (one-arg form) is a *read*, not a definition —
        // Tcl returns the current value of ``x``.
        let mut a = Analyser::new();
        // Pre-define x so the read records a reference.
        a.define_var("x", esc_tok(span(0, 1)), &[], false, None);
        a.handle_set_command(&["x".to_string()], &[esc_tok(span(10, 11))], &[true], &[]);
        // The read appended a reference; no second definition.
        assert!(a.result.global_scope.variables.contains_key("x"));
        assert_eq!(
            a.result.global_scope.variables["x"].references,
            vec![span(10, 11)],
        );
        // No const-string tracking for the 1-arg form.
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn handle_set_no_value_undefined_var_is_silent() {
        // ``set x`` on an undefined variable is still a read; the
        // record_var_read helper silently no-ops when the name
        // isn't in scope, so no spurious binding lands.
        let mut a = Analyser::new();
        a.handle_set_command(&["x".to_string()], &[esc_tok(span(0, 1))], &[true], &[]);
        assert!(!a.result.global_scope.variables.contains_key("x"));
    }

    // handle_var_declaration_command

    #[test]
    fn handle_global_defines_each_name() {
        let mut a = Analyser::new();
        a.handle_global_command(
            &["x".to_string(), "y".to_string(), "z".to_string()],
            &[
                esc_tok(span(0, 1)),
                esc_tok(span(2, 3)),
                esc_tok(span(4, 5)),
            ],
            &[],
        );
        for name in ["x", "y", "z"] {
            assert!(a.result.global_scope.variables.contains_key(name));
            assert!(!a.result.global_scope.variables[name].warn_if_unused);
        }
    }

    #[test]
    fn handle_variable_defines_only_names_skipping_values() {
        let mut a = Analyser::new();
        // `variable x 1 y 2 z` — names at 0, 2, 4; values at 1, 3.
        a.handle_variable_command(
            &[
                "x".to_string(),
                "1".to_string(),
                "y".to_string(),
                "2".to_string(),
                "z".to_string(),
            ],
            &[
                esc_tok(span(0, 1)),
                esc_tok(span(2, 3)),
                esc_tok(span(4, 5)),
                esc_tok(span(6, 7)),
                esc_tok(span(8, 9)),
            ],
            &[],
        );
        for name in ["x", "y", "z"] {
            assert!(a.result.global_scope.variables.contains_key(name));
        }
        // Numbers should NOT be variable names.
        assert!(!a.result.global_scope.variables.contains_key("1"));
        assert!(!a.result.global_scope.variables.contains_key("2"));
    }

    #[test]
    fn handle_variable_single_name_no_value() {
        let mut a = Analyser::new();
        a.handle_variable_command(&["x".to_string()], &[esc_tok(span(0, 1))], &[]);
        assert!(a.result.global_scope.variables.contains_key("x"));
    }

    // handle_upvar_command — the `otherVar` link (issue #923 audit idx 98)

    /// Run `upvar <args>` through the handler and report the alias's link
    /// target, if it got one.
    fn upvar_link_of(args: &[&str], local: &str) -> Option<String> {
        let mut a = Analyser::new();
        let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        let toks: Vec<Token> = (0..owned.len())
            .map(|i| {
                esc_tok(span(
                    u32::try_from(i).unwrap() * 4,
                    u32::try_from(i).unwrap() * 4 + 1,
                ))
            })
            .collect();
        a.handle_upvar_command(&owned, &toks, &[]);
        a.result
            .global_scope
            .variables
            .get(local)
            .and_then(|v| v.link_target.clone())
    }

    #[test]
    fn upvar_links_a_fully_qualified_other_var_at_any_level() {
        // `upvar 1 ::myns::q g` binds `::myns::q` from any call depth
        // (tclsh 9.0.4 / 8.6.14: reached identically through `upvar 1` and
        // `upvar 2`), so the alias has a stable cell to link to.
        assert_eq!(
            upvar_link_of(&["1", "::myns::q", "g"], "g"),
            Some("::myns::q".to_string()),
        );
        assert_eq!(
            upvar_link_of(&["2", "::myns::q", "g"], "g"),
            Some("::myns::q".to_string()),
        );
    }

    #[test]
    fn upvar_links_an_array_other_var_to_its_base() {
        // tk.tcl:177 `upvar ::tk::FocusGrab($index) data` — the element key
        // is runtime data, the array is the named cell every sibling access
        // shares.
        assert_eq!(
            upvar_link_of(&["::tk::FocusGrab($index)", "data"], "data"),
            Some("::tk::FocusGrab".to_string()),
        );
    }

    #[test]
    fn upvar_at_level_hash_zero_links_to_the_global_cell() {
        // tclsh 9.0.4 / 8.6.14: `upvar #0 counter c` reaches `::counter`
        // however deep the stack is.
        assert_eq!(
            upvar_link_of(&["#0", "counter", "c"], "c"),
            Some("::counter".to_string()),
        );
    }

    #[test]
    fn upvar_does_not_link_a_frame_relative_other_var() {
        // TN — `upvar 1 x y` names whatever local the *caller* happens to
        // have; there is no namespace path for it, so inventing one would
        // unify unrelated cells.
        assert_eq!(upvar_link_of(&["1", "x", "y"], "y"), None);
        assert_eq!(upvar_link_of(&["x", "y"], "y"), None);
        // A computed target has no readable name at all.
        assert_eq!(upvar_link_of(&["1", "$name", "obj"], "obj"), None);
        // A computed *level* could be `#0`, but could equally be `1` — the
        // alias must not be linked on a guess.
        assert_eq!(upvar_link_of(&["$lvl", "counter", "c"], "c"), None);
    }

    #[test]
    fn upvar_level_word_presence_follows_argument_count_parity() {
        // `upvar 1 b` is TWO words, so there is no level word and `1` is the
        // caller-side variable name — the alias is `b` (tclsh 9.0.4 /
        // 8.6.14: with `set 1 ONE` in the caller, the callee reads `ONE`).
        let mut a = Analyser::new();
        a.handle_upvar_command(
            &["1".to_string(), "b".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 3))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("b"));
        assert!(!a.result.global_scope.variables.contains_key("1"));
    }

    // handle_proc_command

    #[test]
    fn handle_proc_command_records_every_declaration_site_even_when_shadowed() {
        // TP — issue #923 idx 31 (main audit wave): a proc declared twice
        // verbatim at different spans (plain Tcl's own "last redefinition
        // wins" semantics) must still leave BOTH declarations' own name
        // spans in `proc_declaration_sites`, even though `all_procs`
        // (keyed by qualified name) only ever retains the winner's.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), "return ONE".to_string()],
            &[
                esc_tok(span(5, 8)),
                esc_tok(span(9, 9)),
                str_tok(span(10, 20)),
            ],
            &[],
            &[],
        );
        a.handle_proc_command(
            &["foo".to_string(), String::new(), "return TWO".to_string()],
            &[
                esc_tok(span(30, 33)),
                esc_tok(span(34, 34)),
                str_tok(span(35, 45)),
            ],
            &[],
            &[],
        );
        assert_eq!(a.result.all_procs["::foo"].body_span, span(35, 45));
        let sites: Vec<(&str, tcl_lexer::Span)> = a
            .result
            .proc_declaration_sites
            .iter()
            .map(|(q, s)| (q.as_str(), *s))
            .collect();
        assert_eq!(
            sites,
            vec![("::foo", span(5, 8)), ("::foo", span(30, 33))],
            "both declaration sites must be recorded, in source order"
        );
    }

    #[test]
    fn handle_proc_records_proc_at_global() {
        let mut a = Analyser::new();
        let handled = a.handle_proc_command(
            &["foo".to_string(), "a b".to_string(), "set x $a".to_string()],
            &[
                esc_tok(span(5, 8)),
                esc_tok(span(9, 14)),
                str_tok(span(15, 25)),
            ],
            &[],
            &[],
        );
        assert!(handled);
        assert!(a.result.all_procs.contains_key("::foo"));
        let proc = &a.result.all_procs["::foo"];
        assert_eq!(proc.name, "foo");
        assert_eq!(proc.qualified_name, "::foo");
        assert_eq!(proc.params.len(), 2);
        assert_eq!(proc.params[0].name, "a");
        assert_eq!(proc.params[1].name, "b");
    }

    #[test]
    fn handle_proc_qualifies_under_namespace() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        let handled = a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[0],
        );
        assert!(handled);
        assert!(a.result.all_procs.contains_key("::ns1::foo"));
    }

    #[test]
    fn handle_proc_keys_scope_procs_by_simple_name() {
        // ``scope.procs`` is keyed by the simple name so per-scope
        // lookups and shadowing rules work locally. The
        // qualified name lives on ``ProcDef.qualified_name`` and
        // is the key for ``result.all_procs``.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[0],
        );
        let scope = &a.result.global_scope.children[0];
        // Scope's procs map keyed by simple name, NOT qualified.
        assert!(
            scope.procs.contains_key("foo"),
            "scope.procs should be keyed by simple `foo`, got keys: {:?}",
            scope.procs.keys().collect::<Vec<_>>(),
        );
        assert!(
            !scope.procs.contains_key("::ns1::foo"),
            "scope.procs must not be keyed by qualified name",
        );
        // The qualified name is still on the ProcDef.
        assert_eq!(scope.procs["foo"].qualified_name, "::ns1::foo");
        // ...and result.all_procs is keyed by qualified name.
        assert!(a.result.all_procs.contains_key("::ns1::foo"));
    }

    #[test]
    fn handle_proc_absolute_name_rebases() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "outer"));
        let handled = a.handle_proc_command(
            &["::other::foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 17)),
                str_tok(span(18, 20)),
                str_tok(span(21, 23)),
            ],
            &[],
            &[0],
        );
        assert!(handled);
        // Absolute name rebases — does NOT nest under outer.
        assert!(a.result.all_procs.contains_key("::other::foo"));
        assert!(!a.result.all_procs.contains_key("::outer::other::foo"));
    }

    #[test]
    fn handle_proc_consumes_last_comment_as_doc() {
        let mut a = Analyser::new();
        a.last_comment = "doc string".to_string();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(0, 3)),
                str_tok(span(4, 6)),
                str_tok(span(7, 9)),
            ],
            &[],
            &[],
        );
        assert_eq!(a.result.all_procs["::foo"].doc, "doc string");
        // last_comment is consumed.
        assert!(a.last_comment.is_empty());
    }

    #[test]
    fn handle_proc_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_proc_command(&["foo".to_string()], &[esc_tok(span(0, 3))], &[], &[]);
        assert!(!handled);
        assert!(a.result.all_procs.is_empty());
    }

    // handle_proc_command W113 shadow check

    #[test]
    fn handle_proc_emits_w113_for_builtin_shadow() {
        // ``proc set {} {}`` — the proc name is a built-in.
        // W113 should anchor at the proc-name span and carry
        // the canonical message shape. A *real* dialect is named in the
        // parenthetical label; the permissive fallback (both the empty
        // string and its canonical "tcl" spelling resolve there —
        // dialect-profile-model.md §8, one sink) carries no label, covered
        // by the sibling no-label test below.
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("tcl8.6");
        a.handle_proc_command(
            &["set".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[],
        );
        let w113s: Vec<&crate::analyser::types::Diagnostic> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W113)
            .collect();
        assert_eq!(w113s.len(), 1);
        assert_eq!(w113s[0].span, span(5, 8));
        assert!(w113s[0].message.contains("'set' shadows built-in"));
        assert!(w113s[0].message.contains("(tcl8.6)"));
        assert_eq!(w113s[0].severity, crate::analyser::types::Severity::Warning);
    }

    #[test]
    fn handle_proc_no_w113_for_non_builtin_name() {
        // ``foo`` is not a built-in — no W113.
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("tcl");
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[],
        );
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113),
            "should NOT emit W113 for non-built-in name 'foo'",
        );
    }

    #[test]
    fn handle_proc_w113_matches_qualified_form() {
        // ``proc ::set {} {}`` — qualified form also shadows
        // ``set`` because the registry indexes by bare command
        // name (``::`` is trimmed at lookup).
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("tcl");
        a.handle_proc_command(
            &["::set".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 10)),
                str_tok(span(11, 13)),
                str_tok(span(14, 16)),
            ],
            &[],
            &[],
        );
        assert!(
            a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113)
        );
    }

    #[test]
    fn handle_proc_w113_no_dialect_label_when_dialect_empty() {
        // Empty dialect → no parenthetical label in the message.
        let mut a = Analyser::new();
        // dialect intentionally left empty
        a.handle_proc_command(
            &["set".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[],
        );
        let w113 = a
            .result
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W113)
            .expect("W113 expected");
        assert!(w113.message.contains("'set' shadows built-in"));
        assert!(!w113.message.contains('('), "no dialect label expected");
    }

    #[test]
    fn handle_proc_w113_dialect_specific_command_only_shadows_in_that_dialect() {
        // ``pool`` is an *unqualified* iRules-specific command; under the
        // ``f5-irules`` dialect a proc named ``pool`` shadows a built-in,
        // but under plain ``tcl`` it does not.  (A *namespace-qualified*
        // iRules command like ``HTTP::respond`` is never flagged, because a
        // qualified match is a library/package command living in its own
        // namespace, not a core-global shadow.)
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("f5-irules");
        a.handle_proc_command(
            &["pool".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 9)),
                str_tok(span(10, 12)),
                str_tok(span(13, 15)),
            ],
            &[],
            &[],
        );
        assert!(
            a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113),
            "f5-irules dialect should treat pool as built-in",
        );

        // Same proc, plain tcl dialect → no W113.
        let mut b = Analyser::new();
        b.profile = tcl_dialect::DialectProfile::by_name("tcl");
        b.handle_proc_command(
            &["pool".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 9)),
                str_tok(span(10, 12)),
                str_tok(span(13, 15)),
            ],
            &[],
            &[],
        );
        assert!(
            !b.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113),
            "plain tcl dialect should NOT flag pool",
        );
    }

    #[test]
    fn handle_proc_w113_silent_on_namespace_qualified_shadow() {
        // FP: a namespace-qualified match (`HTTP::respond` under f5-irules)
        // is a library/package command in its own namespace, not a
        // core-global shadow — never W113.
        let mut a = Analyser::new();
        a.profile = tcl_dialect::DialectProfile::by_name("f5-irules");
        a.handle_proc_command(
            &["HTTP::respond".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 18)),
                str_tok(span(19, 21)),
                str_tok(span(22, 24)),
            ],
            &[],
            &[],
        );
        assert!(
            !a.result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W113),
            "namespace-qualified HTTP::respond must not fire W113",
        );
    }

    // handle_proc_command body recursion

    #[test]
    fn handle_proc_creates_proc_scope_for_braced_body() {
        // ``proc foo {} {}`` — empty braced body still opens a
        // proc scope so subsequent body-walking handlers have a
        // place to record locals.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[],
        );
        assert_eq!(a.result.global_scope.children.len(), 1);
        let proc_scope = &a.result.global_scope.children[0];
        assert_eq!(proc_scope.kind, crate::analyser::types::ScopeKind::Proc);
        assert_eq!(proc_scope.name, "foo");
        assert_eq!(proc_scope.body_span, Some(span(12, 14)));
    }

    #[test]
    fn handle_proc_defines_params_in_proc_scope() {
        // ``proc foo {a b} {}`` — a, b become locals in the
        // proc scope, not in the outer scope.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), "a b".to_string(), String::new()],
            &[
                esc_tok(span(5, 8)),
                esc_tok(span(9, 14)),
                str_tok(span(15, 17)),
            ],
            &[],
            &[],
        );
        let proc_scope = &a.result.global_scope.children[0];
        assert!(proc_scope.variables.contains_key("a"));
        assert!(proc_scope.variables.contains_key("b"));
        // Outer scope must be untouched.
        assert!(!a.result.global_scope.variables.contains_key("a"));
    }

    #[test]
    fn handle_proc_walks_body_set_defines_local() {
        // ``proc foo {} {set x 1}`` — body walk segments the
        // body and dispatches `set x 1` against the proc scope,
        // landing the local in proc_scope.variables, not global.
        // The body token's span must mirror the outer source so
        // the segmenter rebases correctly: source layout is
        // ``proc foo {} {set x 1}`` with the body occupying [13, 22].
        // ``content_offset = 1`` skips the leading ``{`` so the
        // re-segmented inner runs at base 14.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), "set x 1".to_string()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(13, 22)),
            ],
            &[],
            &[],
        );
        let proc_scope = &a.result.global_scope.children[0];
        assert!(
            proc_scope.variables.contains_key("x"),
            "body walk should land 'x' in proc scope; vars: {:?}",
            proc_scope.variables.keys().collect::<Vec<_>>(),
        );
        assert!(!a.result.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn handle_proc_walks_body_global_falls_into_proc_scope() {
        // ``proc foo {} {global a b}`` — the ``global`` handler
        // defines bindings in the proc scope so the body's later
        // reads/writes resolve correctly. Real ``global`` semantics
        // (link to outer var) live with diagnostic emission later.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), "global a b".to_string()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(13, 25)),
            ],
            &[],
            &[],
        );
        let proc_scope = &a.result.global_scope.children[0];
        assert!(proc_scope.variables.contains_key("a"));
        assert!(proc_scope.variables.contains_key("b"));
    }

    #[test]
    fn handle_proc_nested_proc_creates_nested_scopes() {
        // Body-walk recursion must dispatch `proc` inside the body,
        // creating a nested proc scope under the outer proc.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &[
                "outer".to_string(),
                String::new(),
                "proc inner {} {}".to_string(),
            ],
            &[
                esc_tok(span(5, 10)),
                str_tok(span(11, 13)),
                str_tok(span(15, 33)),
            ],
            &[],
            &[],
        );
        // Outer proc registered.
        assert!(a.result.all_procs.contains_key("::outer"));
        // A nested definition homes to the enclosing proc's *defining*
        // namespace (`command_resolution_namespace` /
        // `advance_command_resolution_namespace`), not to a namespace named
        // after the proc. ``outer`` is unqualified, so its defining namespace
        // is ``::`` and the nested ``inner`` qualifies as ``::inner``.
        assert!(a.result.all_procs.contains_key("::inner"));
        // Outer's proc scope holds the nested proc scope as a child.
        let outer_scope = &a.result.global_scope.children[0];
        assert!(!outer_scope.children.is_empty());
        assert_eq!(
            outer_scope.children[0].kind,
            crate::analyser::types::ScopeKind::Proc,
        );
        assert_eq!(outer_scope.children[0].name, "inner");
    }

    #[test]
    fn handle_proc_dynamic_body_skips_walk() {
        // ``proc foo {} $body`` — the body is a Var token, not a
        // Str token; we cannot statically re-segment a dynamic
        // body, so the body walk is skipped. The proc record
        // itself still lands so downstream signature consumers see
        // the proc; only the inner walk is gated.
        let mut a = Analyser::new();
        let var_tok = Token::new(TokenType::Var, span(13, 18));
        a.handle_proc_command(
            &["foo".to_string(), String::new(), "$body".to_string()],
            &[esc_tok(span(5, 8)), str_tok(span(9, 11)), var_tok],
            &[],
            &[],
        );
        assert!(a.result.all_procs.contains_key("::foo"));
        // No proc scope opened — Str gate failed.
        assert!(a.result.global_scope.children.is_empty());
    }

    #[test]
    fn handle_proc_body_walk_increments_body_depth_temporarily() {
        // ``body_depth`` is bumped for the duration of the body
        // walk and restored on exit — top-level-only command
        // checks rely on the depth being zero outside any body.
        let mut a = Analyser::new();
        assert_eq!(a.body_depth, 0);
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[],
        );
        assert_eq!(a.body_depth, 0);
    }

    #[test]
    fn handle_proc_body_walk_does_not_leak_inner_doc_comment() {
        // A trailing comment inside the body should not bleed into
        // ``last_comment`` for whatever follows the proc at the
        // outer scope. The outer comment ("doc string") is consumed
        // as the proc's own doc; after the walk, ``last_comment``
        // is restored to empty.
        let mut a = Analyser::new();
        a.last_comment = "doc string".to_string();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[],
        );
        assert_eq!(a.result.all_procs["::foo"].doc, "doc string");
        assert!(a.last_comment.is_empty());
    }

    // handle_namespace_eval_command

    // handle_namespace_export_command

    #[test]
    fn handle_namespace_export_records_pattern() {
        let mut a = Analyser::new();
        a.handle_namespace_export_command(
            &["export".to_string(), "bar".to_string()],
            &[esc_tok(span(0, 6)), esc_tok(span(7, 10))],
            &[],
        );
        assert_eq!(a.result.namespace_exports.len(), 1);
        assert_eq!(a.result.namespace_exports[0].ns, "::");
        assert_eq!(a.result.namespace_exports[0].pattern, "bar");
    }

    #[test]
    fn handle_namespace_export_records_multiple_patterns() {
        let mut a = Analyser::new();
        a.handle_namespace_export_command(
            &["export".to_string(), "bar".to_string(), "b*".to_string()],
            &[
                esc_tok(span(0, 6)),
                esc_tok(span(7, 10)),
                esc_tok(span(11, 13)),
            ],
            &[],
        );
        let patterns: Vec<&str> = a
            .result
            .namespace_exports
            .iter()
            .map(|e| e.pattern.as_str())
            .collect();
        assert_eq!(patterns, vec!["bar", "b*"]);
    }

    #[test]
    fn handle_namespace_export_clear_records_an_ordered_tombstone() {
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.handle_namespace_export_command(
            &["export".to_string(), "bar".to_string()],
            &[esc_tok(span(0, 6)), esc_tok(span(7, 10))],
            &[],
        );
        a.handle_namespace_export_command(
            &[
                "export".to_string(),
                "-clear".to_string(),
                "baz".to_string(),
            ],
            &[
                esc_tok(span(11, 17)),
                esc_tok(span(18, 24)),
                esc_tok(span(25, 28)),
            ],
            &[],
        );
        // `-clear` is an ordered event, not an eager delete: the earlier
        // `bar` entry stays on the log so a `namespace import` that ran
        // *before* the `-clear` can still see it (issue #1027).
        let events: Vec<(&str, bool, u32)> = a
            .result
            .namespace_exports
            .iter()
            .map(|e| (e.pattern.as_str(), e.clears, e.range.start()))
            .collect();
        assert_eq!(
            events,
            vec![("bar", false, 7), ("", true, 18), ("baz", false, 25)]
        );
    }

    #[test]
    fn handle_namespace_export_consumes_only_one_clear_flag() {
        // PR #1102 review finding 3 — `NamespaceExportCmd` compares `objv[1]`
        // against `-clear` once, so a *second* `-clear` is an ordinary export
        // pattern. Oracle (tclsh 8.6.14 / 9.0.4): `namespace export -clear
        // -clear p` leaves exactly `-clear p` exported, and a command really
        // named `-clear` is then importable through `namespace import
        // ::src::*`. Consuming every matching word instead recorded two
        // tombstones and silently dropped the `-clear` export.
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.handle_namespace_export_command(
            &[
                "export".to_string(),
                "-clear".to_string(),
                "-clear".to_string(),
                "p".to_string(),
            ],
            &[
                esc_tok(span(0, 6)),
                esc_tok(span(7, 13)),
                esc_tok(span(14, 20)),
                esc_tok(span(21, 22)),
            ],
            &[],
        );
        let events: Vec<(&str, bool)> = a
            .result
            .namespace_exports
            .iter()
            .map(|e| (e.pattern.as_str(), e.clears))
            .collect();
        assert_eq!(events, vec![("", true), ("-clear", false), ("p", false)]);
    }

    #[test]
    fn handle_namespace_export_flag_after_a_pattern_is_a_pattern() {
        // The flag is only ever the *first* word: oracle `namespace export a
        // -clear` → `namespace export` returns `a -clear`.
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.handle_namespace_export_command(
            &["export".to_string(), "a".to_string(), "-clear".to_string()],
            &[
                esc_tok(span(0, 6)),
                esc_tok(span(7, 8)),
                esc_tok(span(9, 15)),
            ],
            &[],
        );
        let events: Vec<(&str, bool)> = a
            .result
            .namespace_exports
            .iter()
            .map(|e| (e.pattern.as_str(), e.clears))
            .collect();
        assert_eq!(events, vec![("a", false), ("-clear", false)]);
    }

    #[test]
    fn handle_namespace_import_consumes_only_one_force_flag() {
        // Symmetric to the export case: `namespace import -force -force
        // ::src::*` reads the second `-force` as an import *pattern* (and
        // aborts with `no namespace specified in import pattern "-force"`,
        // tclsh 8.6.14/9.0.4). Only the first is skipped as a flag, so the
        // second is recorded as the pattern word it is — one whose empty
        // source namespace both wildcard resolvers already decline.
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.handle_namespace_import_command(
            &[
                "import".to_string(),
                "-force".to_string(),
                "-force".to_string(),
                "::src::*".to_string(),
            ],
            &[
                esc_tok(span(0, 6)),
                esc_tok(span(7, 13)),
                esc_tok(span(14, 20)),
                esc_tok(span(21, 29)),
            ],
            &[],
        );
        let patterns: Vec<&str> = a
            .result
            .namespace_imports
            .iter()
            .map(|i| i.pattern.as_str())
            .collect();
        assert_eq!(patterns, vec!["::-force", "::src::*"]);
    }

    #[test]
    fn handle_namespace_export_bare_clear_records_a_tombstone() {
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.handle_namespace_export_command(
            &["export".to_string(), "-clear".to_string()],
            &[esc_tok(span(0, 6)), esc_tok(span(7, 13))],
            &[],
        );
        assert_eq!(a.result.namespace_exports.len(), 1);
        assert!(a.result.namespace_exports[0].clears);
        assert!(a.result.namespace_exports[0].pattern.is_empty());
    }

    #[test]
    fn handle_namespace_export_skips_dynamic_pattern() {
        let mut a = Analyser::new();
        a.handle_namespace_export_command(
            &["export".to_string(), "$dyn".to_string()],
            &[esc_tok(span(0, 6)), esc_tok(span(7, 11))],
            &[],
        );
        assert!(
            a.result.namespace_exports.is_empty(),
            "a dynamic pattern can't be statically recorded"
        );
    }

    // handle_namespace_path_command

    /// The two-word literal set form records entries under the declaring
    /// namespace; a later declaration replaces the whole path (C Tcl
    /// semantics), and entries are kept as written (rooting happens in the
    /// shared candidate builder).
    #[test]
    fn handle_namespace_path_records_and_replaces() {
        let mut a = Analyser::new();
        a.handle_namespace_path_command(&["path".to_string(), "::u ::v".to_string()], &[], &[]);
        assert_eq!(
            a.namespace_paths.get("::").map(Vec::as_slice),
            Some(&["::u".to_string(), "::v".to_string()][..]),
        );
        a.handle_namespace_path_command(&["path".to_string(), "inner".to_string()], &[], &[]);
        assert_eq!(
            a.namespace_paths.get("::").map(Vec::as_slice),
            Some(&["inner".to_string()][..]),
            "each namespace path declaration replaces the whole path",
        );
    }

    /// #1245 — a **braced** path word is literal, not dynamic. The word
    /// arrives with braces stripped, so a text-only dynamism scan sees the
    /// `$` and abstains from the whole command; the token kind is the
    /// authority.
    ///
    /// tclsh-proof: tclsh8.6.14 —
    /// `set nn {::$ns}; namespace eval $nn {}; namespace eval a {}` then
    /// `namespace eval n { namespace path {::$ns ::a} }` and
    /// `namespace eval n { namespace path }` answers `{::$ns} ::a` — the
    /// entry is a namespace literally *named* `::$ns`.
    #[test]
    fn handle_namespace_path_records_a_braced_dollar_bearing_word() {
        let mut a = Analyser::new();
        a.handle_namespace_path_command(
            &["path".to_string(), "::$ns ::a".to_string()],
            &[esc_tok(span(0, 4)), str_tok(span(5, 16))],
            &[],
        );
        assert_eq!(
            a.namespace_paths.get("::").map(Vec::as_slice),
            Some(&["::$ns".to_string(), "::a".to_string()][..]),
            "a braced word never substitutes, so its entries are literal names",
        );
    }

    /// TN control — the same text **unbraced** really is a substitution, so
    /// the command still abstains.
    #[test]
    fn handle_namespace_path_still_skips_an_unbraced_dollar_word() {
        let mut a = Analyser::new();
        a.handle_namespace_path_command(
            &["path".to_string(), "$entries".to_string()],
            &[esc_tok(span(0, 4)), Token::new(TokenType::Var, span(5, 13))],
            &[],
        );
        assert!(a.namespace_paths.is_empty());
    }

    /// The query form (no list) and a dynamic list (`$var` / `[cmd]`)
    /// record nothing — the conservative empty path stands.
    #[test]
    fn handle_namespace_path_skips_query_and_dynamic_forms() {
        let mut a = Analyser::new();
        a.handle_namespace_path_command(&["path".to_string()], &[], &[]);
        a.handle_namespace_path_command(&["path".to_string(), "$entries".to_string()], &[], &[]);
        a.handle_namespace_path_command(
            &["path".to_string(), "[current_path]".to_string()],
            &[],
            &[],
        );
        assert!(a.namespace_paths.is_empty());
    }

    /// A declaration inside a namespace scope keys to that namespace's
    /// accumulated name, so settlement applies it to the right call sites.
    #[test]
    fn handle_namespace_path_keys_to_declaring_namespace() {
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(crate::analyser::types::Scope::new(
                crate::analyser::types::ScopeKind::Namespace,
                "outer",
            ));
        a.handle_namespace_path_command(&["path".to_string(), "::helpers".to_string()], &[], &[0]);
        assert_eq!(
            a.namespace_paths.get("::outer").map(Vec::as_slice),
            Some(&["::helpers".to_string()][..]),
        );
    }

    /// The `namespace path` resolution tier is 8.5+: a bare call under a path
    /// gains the path namespace as a candidate from 8.5 on, but under 8.4 it
    /// must not (8.4 has no path tier, so the call never reaches it).
    #[test]
    fn bare_call_honours_namespace_path_only_from_8_5() {
        let src = "namespace eval ::app { namespace path ::mymod\n    proc run {} { helper } }\n";
        let candidates = |dialect: &str| {
            let mut a = Analyser::new();
            a.analyse(src, dialect)
                .command_invocations
                .iter()
                .find(|i| i.name == "helper")
                .map(|i| i.resolution_candidates.clone())
                .unwrap_or_default()
        };
        assert!(
            candidates("tcl8.6").iter().any(|c| c == "::mymod::helper"),
            "8.6 should add the path namespace as a candidate: {:?}",
            candidates("tcl8.6"),
        );
        assert!(
            !candidates("tcl8.4").iter().any(|c| c == "::mymod::helper"),
            "8.4 has no path tier, so it must not: {:?}",
            candidates("tcl8.4"),
        );
    }

    /// A command-table mutation inside a child interpreter's eval body
    /// belongs to *that* interpreter (issue #1141's flaw class, found while
    /// auditing for other per-interpreter state kept in flat file-wide maps).
    ///
    /// tclsh 9.0.4, verified: after `interp create c; c eval { rename puts
    /// myputs }` the parent's `puts` still works, the parent has no `myputs`,
    /// the child has `myputs` and no `puts`; and `c eval { rename set {} }`
    /// leaves the parent's `set` alone.
    #[test]
    fn rename_inside_a_child_body_does_not_touch_the_parent_command_table() {
        let src = "interp create c\nc eval { rename puts myputs }\nputs hi\n";
        let r = Analyser::new().analyse(src, "tcl8.6");
        assert!(
            r.renamed_commands.is_empty(),
            "a child's rename must not enter the parent's rename map: {:?}",
            r.renamed_commands
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.message.contains("'puts'")),
            "the parent's own `puts` is still a live builtin: {:?}",
            r.diagnostics
                .iter()
                .map(|d| (d.code.to_string(), d.message.clone()))
                .collect::<Vec<_>>()
        );
    }

    /// The incremental per-item path must agree with full analysis here: a
    /// deferred proc body is walked by a fresh `Analyser` that does not
    /// inherit the interpreter stack, so a `rename` buried in a proc defined
    /// inside a child body is the shape most likely to diverge.
    #[test]
    fn per_item_agrees_on_a_rename_nested_in_a_child_body() {
        let src = "interp create c\n\
                   c eval { proc setup {} { rename puts myputs } }\n\
                   puts hi\n";
        let codes = |r: &super::super::types::AnalysisResult| {
            let mut v: Vec<(String, u32)> = r
                .diagnostics
                .iter()
                .map(|d| (d.code.to_string(), d.span.start()))
                .collect();
            v.sort();
            v
        };
        let full = Analyser::new().analyse(src, "tcl8.6");
        let per_item = Analyser::new().analyse_per_item(src, "tcl8.6");
        assert_eq!(codes(&full), codes(&per_item));
        assert!(
            !full
                .diagnostics
                .iter()
                .any(|d| d.message.contains("'puts'")),
            "{:?}",
            codes(&full)
        );
    }

    /// The same rename written in the parent still records normally — the
    /// guard narrows the fact to its interpreter, it does not disable it.
    #[test]
    fn rename_in_the_parent_still_records_the_command_move() {
        let src = "interp create c\nrename puts myputs\n";
        let r = Analyser::new().analyse(src, "tcl8.6");
        assert_eq!(
            r.renamed_commands.get("::myputs").map(String::as_str),
            Some("::puts"),
            "{:?}",
            r.renamed_commands
        );
    }

    /// `interp alias {} A {} B` inside a child's eval body creates the alias
    /// in the *child* (an empty srcPath is "the interpreter I am running
    /// in"), so it must home under that child's `@interp@` domain rather than
    /// at the parent's global root.
    #[test]
    fn empty_path_interp_alias_inside_a_child_body_homes_in_that_child() {
        let src = "interp create c\nc eval { interp alias {} shout {} puts }\n";
        let r = Analyser::new().analyse(src, "tcl8.6");
        assert!(
            !r.command_aliases.contains_key("::shout"),
            "the parent gained no alias: {:?}",
            r.command_aliases.keys().collect::<Vec<_>>()
        );
        assert!(
            r.command_aliases.contains_key("::@interp@c::shout"),
            "the child's alias homes under its own domain: {:?}",
            r.command_aliases.keys().collect::<Vec<_>>()
        );
    }

    /// The same alias written at the top level is unchanged.
    #[test]
    fn empty_path_interp_alias_at_top_level_still_homes_globally() {
        let src = "interp alias {} shout {} puts\n";
        let r = Analyser::new().analyse(src, "tcl8.6");
        assert!(
            r.command_aliases.contains_key("::shout"),
            "{:?}",
            r.command_aliases.keys().collect::<Vec<_>>()
        );
    }

    /// `interp eval child { proc foo }` runs in a child interpreter, so `foo`
    /// is isolated from the parent's `::foo` — the two stay distinct commands,
    /// so a parent rename of `::foo` can never reach the child body.
    #[test]
    fn interp_eval_child_isolates_definitions_from_the_parent() {
        // Runnable Tcl (tclsh 9.0.4): the child is created before the
        // eval.  The child's `foo` homes under the synthetic
        // `@interp@child` domain — unrepresentable as a real namespace,
        // so a parent namespace literally named `child` can never
        // collide with the interpreter's definitions (issue #945
        // fault 8's child-global-vs-parent-namespace identity split).
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create child\nproc foo {} {}\ninterp eval child { proc foo {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(r.all_procs.contains_key("::foo"), "parent proc: {keys:?}");
        assert!(
            r.all_procs.contains_key("::@interp@child::foo"),
            "child proc isolated under the interpreter domain: {keys:?}",
        );
        assert!(
            !r.all_procs.contains_key("::child::foo"),
            "the child's global is not the parent namespace `::child`: {keys:?}",
        );
    }

    /// `sandbox eval { proc foo }` via the interpreter's own object command
    /// (`interp create sandbox` binds `sandbox` itself as a callable
    /// command) isolates definitions from the parent exactly like the
    /// literal `interp eval sandbox { … }` form above — the far more common
    /// real-world spelling (`docstrip_util.tcl`'s `$c eval {...}`, reduced to
    /// a literal name for a statically-resolvable repro).
    #[test]
    fn interp_handle_eval_isolates_definitions_from_the_parent() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandbox\nproc foo {} {}\nsandbox eval { proc foo {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(r.all_procs.contains_key("::foo"), "parent proc: {keys:?}");
        assert!(
            r.all_procs.contains_key("::@interp@sandbox::foo"),
            "child proc isolated under the interpreter domain: {keys:?}",
        );
    }

    #[test]
    fn renamed_away_interp_handle_reused_as_a_proc_is_not_treated_as_interp_eval() {
        // TP — regression for a bug found by Codex review of PR #963:
        // `self.interpreters` is only cleared by `interp delete`, so a plain
        // `rename sandbox {}` (deleting the interpreter's own object
        // command — confirmed against tclsh9.0: afterwards `info commands
        // sandbox` is empty and the child interpreter is only reachable
        // through whatever name, if any, it was renamed *to*) left a stale
        // `sandbox` entry. A later, wholly unrelated `proc sandbox {sub
        // body} {...}` reusing the freed name would then have any
        // `sandbox eval …` call misidentified as isolated interpreter-eval
        // (walking the literal second argument as a *script*) instead of an
        // ordinary two-arg proc call — confirmed against tclsh9.0 that the
        // real dispatch reaches the new proc, never the (now nameless,
        // orphaned) interpreter.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandbox\n\
             rename sandbox {}\n\
             proc sandbox {sub body} {}\n\
             sandbox eval { proc shouldNotBeIsolated {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            !r.all_procs
                .contains_key("::@interp@sandbox::shouldNotBeIsolated"),
            "the call must not be isolated into the (deleted) interpreter's \
             domain once `sandbox` has been renamed away and reused: {keys:?}",
        );
    }

    #[test]
    fn renamed_interp_handle_still_isolates_under_its_new_name() {
        // TP — the other half of the same fix: a rename that *moves* the
        // interpreter handle (`NEW` non-empty) rather than deleting it must
        // keep the tracking, now keyed by `NEW` — confirmed against
        // tclsh9.0 that `rename sandbox t; t eval {...}` still talks to the
        // same child interpreter. A name-only invalidation (removing `OLD`
        // without transferring state to `NEW`) would wrongly stop isolating
        // `t eval { ... }` too (falling through to an ordinary, unisolated
        // command call). The domain name itself is always derived from
        // whichever key the call site actually writes — `@interp@t`, not
        // the original `@interp@sandbox` — exactly like every other
        // interpreter-domain lookup in this module (e.g. the literal
        // `interp eval` form re-derives its domain from its own `PATH`
        // argument text the same way); this test only asserts that the
        // call is isolated *at all* under the new name, not which domain
        // string it lands in.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandbox\n\
             rename sandbox t\n\
             t eval { proc shouldBeIsolated {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::@interp@t::shouldBeIsolated"),
            "the renamed handle must still be tracked as an interpreter, \
             isolating the eval body under its new name's domain: {keys:?}",
        );
    }

    #[test]
    fn interp_handle_eval_and_literal_interp_eval_agree_on_the_same_domain() {
        // The two spellings of the same operation must home under the
        // *same* synthetic domain — the analyser can't tell them apart at
        // run time (both dispatch on the identical interpreter object
        // command), so it must not tell them apart statically either.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandbox\ninterp eval sandbox { proc viaLiteral {} {} }\n\
             sandbox eval { proc viaHandle {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::@interp@sandbox::viaLiteral"),
            "{keys:?}"
        );
        assert!(
            r.all_procs.contains_key("::@interp@sandbox::viaHandle"),
            "{keys:?}"
        );
    }

    #[test]
    fn two_interp_handles_never_cross_contaminate_same_named_procs() {
        // TP — the exact scenario differential audit confirmed: two
        // separate safe child interpreters, each independently defining
        // their own same-named `helper`, must never merge — a call inside
        // one script must resolve to *that* interpreter's helper, never
        // the other's.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create sandboxA\ninterp create sandboxB\n\
             interp eval sandboxA { proc helper {} { return A } }\n\
             sandboxB eval { proc helper {} { return B } }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::@interp@sandboxA::helper"),
            "{keys:?}"
        );
        assert!(
            r.all_procs.contains_key("::@interp@sandboxB::helper"),
            "{keys:?}"
        );
        // Each interpreter's own `helper` is a genuinely distinct
        // ProcDef, not the same entry aliased twice.
        assert_ne!(
            r.all_procs["::@interp@sandboxA::helper"].body_span,
            r.all_procs["::@interp@sandboxB::helper"].body_span,
        );
    }

    #[test]
    fn untracked_bareword_eval_is_not_isolated() {
        // TN / FN guard — a bareword shaped exactly like the tracked-handle
        // idiom (`NAME eval { script }`) but never created via `interp
        // create` must fall through to the generic dispatch untouched: no
        // isolated scope, no crash.
        let mut a = Analyser::new();
        let r = a.analyse("untracked eval { proc foo {} {} }\n", "tcl8.6");
        assert!(
            !r.all_procs.keys().any(|k| k.contains("@interp@")),
            "an untracked head must never open an interpreter domain: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn ordinary_proc_with_eval_as_sole_argument_is_untouched() {
        // FP guard — an unrelated proc whose single argument merely happens
        // to be the literal text `eval` must resolve as an ordinary call,
        // never trip the arity/shape check into isolating anything.
        let mut a = Analyser::new();
        let r = a.analyse("proc foo {a} { return $a }\nfoo eval\n", "tcl8.6");
        assert!(
            !r.all_procs.keys().any(|k| k.contains("@interp@")),
            "a 1-arg call must never be treated as NAME eval SCRIPT: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn interp_eval_into_uncreated_interp_warns_and_multiword_stays_isolated_945() {
        // `interp eval ghost { … }` with no `interp create ghost` anywhere
        // raises `could not find interpreter` at run time — W140 (issue
        // #945 fault 8: interpreter existence).
        let mut a = Analyser::new();
        let r = a.analyse("interp eval ghost { proc foo {} {} }\n", "tcl8.6");
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W140),
            "eval into a never-created interpreter warns: {:?}",
            r.diagnostics
        );
        // A dynamic create makes existence unknowable — W140 abstains.
        let mut a2 = Analyser::new();
        let r2 = a2.analyse(
            "interp create $name\ninterp eval ghost { proc foo {} {} }\n",
            "tcl8.6",
        );
        assert!(
            !r2.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W140),
            "dynamic interp creation abstains: {:?}",
            r2.diagnostics
        );
        // Multi-word scripts concatenate at run time — the words must not
        // be analysed in the *parent* scope (the old fall-through), nor
        // walked per-word (commands span word boundaries).
        let mut a3 = Analyser::new();
        let r3 = a3.analyse(
            "interp create c\ninterp eval c {proc leak {} {}} {puts hi}\n",
            "tcl8.6",
        );
        assert!(
            !r3.all_procs.contains_key("::leak"),
            "multi-word eval must not leak definitions into the parent: {:?}",
            r3.all_procs.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn deleted_and_recreated_interp_is_a_fresh_domain_945() {
        // C: `interp delete s; interp create s` starts with an empty
        // command table — the old definitions are gone.  The epoch-stamped
        // domain keeps the two lifetimes apart: the first eval's `foo`
        // homes under `@interp@s`, the recreated interpreter's under
        // `@interp@s#1`, and they never merge (issue #945 fault 8).
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create s\ninterp eval s { proc foo {} {} }\n\
             interp delete s\ninterp create s\n\
             interp eval s { proc bar {} {} }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::@interp@s::foo"),
            "first lifetime's proc: {keys:?}"
        );
        assert!(
            r.all_procs.contains_key("::@interp@s#1::bar"),
            "recreated interpreter is a fresh domain: {keys:?}"
        );
        assert!(
            !r.all_procs.contains_key("::@interp@s::bar"),
            "the recreated child never merges with its predecessor: {keys:?}"
        );
    }

    #[test]
    fn nested_interp_ops_qualify_against_the_enclosing_child_945() {
        // `interp create t` inside `interp eval s {…}` creates `s t` —
        // paths are relative to the *current* interpreter, so a top-level
        // `interp eval {s t} {…}` reaches the grandchild without W140 and
        // its definitions home under the composed domain.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create s\n\
             interp eval s { interp create t }\n\
             interp eval {s t} { proc deep {} {} }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W140),
            "the nested create makes `s t` exist: {:?}",
            r.diagnostics
        );
        assert!(
            r.all_procs.contains_key("::@interp@s t::deep"),
            "grandchild definitions home under the composed domain: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn cross_domain_interp_alias_links_child_calls_to_the_parent_target_945() {
        // `interp alias s helper {} ::parent_helper` makes `helper`
        // callable *inside the child* running the parent's proc — the
        // alias deliberately crosses interpreter domains while
        // definitions do not (issue #945 fault 8).  The child-side call
        // resolves through the alias link to the parent target.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc parent_helper {} {}\ninterp create s\n\
             interp alias s helper {} parent_helper\n\
             interp eval s { helper }\n",
            "tcl8.6",
        );
        let alias = r
            .command_aliases
            .get("::@interp@s::helper")
            .unwrap_or_else(|| {
                panic!(
                    "child-domain alias recorded: {:?}",
                    r.command_aliases.keys().collect::<Vec<_>>()
                )
            });
        assert_eq!(
            alias.target, "::parent_helper",
            "the alias target resolves in the parent domain",
        );
    }

    #[test]
    fn safe_interp_hides_unsafe_commands_and_expose_restores_945() {
        // tclsh 9.0.4: a `-safe` child hides `source` (and the rest of the
        // non-CMD_IS_SAFE set) — calling it raises `invalid command name`.
        // The safe-context walk flags the call (W129) and, because the
        // command never executes, builds no source edge from it.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `source` in a safe interp warns: {:?}",
            r.diagnostics
        );
        assert!(
            r.source_targets.is_empty(),
            "no source edge may be built from a call that never executes: {:?}",
            r.source_targets
        );
        // A safe command (`puts`) draws no W129; a non-safe interp draws
        // none either.
        let mut a2 = Analyser::new();
        let r2 = a2.analyse(
            "interp create -safe s\ninterp eval s { puts hi }\n\
             interp create n\ninterp eval n { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            !r2.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "safe commands and non-safe interps draw no W129: {:?}",
            r2.diagnostics
        );
        // `interp expose s source` restores the command.
        let mut a3 = Analyser::new();
        let r3 = a3.analyse(
            "interp create -safe s\ninterp expose s source\n\
             interp eval s { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            !r3.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an exposed command is callable again: {:?}",
            r3.diagnostics
        );
        // `interp hide n source` hides it in a *normal* interp too.
        let mut a4 = Analyser::new();
        let r4 = a4.analyse(
            "interp create n\ninterp hide n source\n\
             interp eval n { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            r4.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an explicitly hidden command warns in a normal interp: {:?}",
            r4.diagnostics
        );
    }

    #[test]
    fn safe_interp_child_redefinition_of_a_hidden_builtin_is_callable_945() {
        // tclsh 9.0.4: `proc source {} {…}` inside a safe interp creates a
        // real command in the ordinary command table — a table entirely
        // separate from the hidden set — so calling `source` afterwards
        // runs the user's proc, never `invalid command name`.  The gate
        // must not draw W129 for the call, and the call must still be
        // fully analysed (resolving to the local proc), not skipped as an
        // effect-free hidden call.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { proc source {} { return ok }; source }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a locally-redefined name is callable, not hidden: {:?}",
            r.diagnostics
        );
        assert!(
            r.all_procs.contains_key("::@interp@s::source"),
            "the local proc is recorded, not skipped: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        // A call *preceding* the redefinition is unaffected by this file's
        // whole-body approximation in the other direction — still hidden
        // until the walk has seen a local definition — is not asserted
        // here (the analyser's per-body model is not that finely ordered);
        // what matters is that a real, common redefinition idiom never
        // produces a false-positive W129 that also drops the call's facts.
    }

    #[test]
    fn interp_expose_with_a_new_name_tracks_the_new_name_945() {
        // tclsh 9.0.4: `interp expose s source src` restores the hidden
        // `source` implementation under the name `src` — `source` itself
        // stays absent from ordinary lookup, and `src` becomes callable.
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp expose s source src\n\
             interp eval s { source b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "the hidden name itself stays hidden after an exposed rename: {:?}",
            r.diagnostics
        );
        let mut a2 = Analyser::new();
        let r2 = a2.analyse(
            "interp create -safe s\ninterp expose s source src\n\
             interp eval s { src b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            !r2.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "the new exposed name is callable: {:?}",
            r2.diagnostics
        );
    }

    // -- issue #1001: W129 through `[...]` bracket-substitution indirection --

    /// TP (the reported repro): `package ifneeded`'s script argument is
    /// `ArgRole::Body`-tagged (`Structural` — runs later, via `uplevel #0`,
    /// never the definer's frame), so a `[list apply {…} $dir]`
    /// deferred-command idiom sitting there is genuinely invoked later —
    /// a hidden `source` nested inside the lambda body must draw W129, the
    /// same way it would if `apply {dir {source …}} $dir` were written
    /// directly (no `[list …]` wrapper).
    #[test]
    fn safe_interp_w129_list_quoted_apply_lambda_body_reports_hidden_source_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 package ifneeded myPackage 1.0 [list apply {dir {\n\
                     source [file join $dir font.tcl]\n\
                 }} $dir]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `source` nested inside a list-quoted apply lambda body \
             (reached only via `package ifneeded`'s deferred script) warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: the same `[list apply {…} $x]` idiom in an `ArgRole::CommandPrefix`
    /// position (`trace add variable … command CALLBACK`) — a different
    /// registry role than `package ifneeded`'s `Body`, exercising the same
    /// deferred-call resolution through a different gate.
    #[test]
    fn safe_interp_w129_list_quoted_apply_in_command_prefix_position_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 trace add variable x write [list apply {{a b c} {\n\
                     exec ls\n\
                 }} $x]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `exec` nested inside a list-quoted apply lambda body \
             reached via a CommandPrefix (`trace add … command`) position warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: `after idle [list apply {…} $x]` — the `after`/`after idle`
    /// deferred-callback idiom the issue calls out by name.
    #[test]
    fn safe_interp_w129_list_quoted_apply_after_idle_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 after idle [list apply {x {\n\
                     file delete $x\n\
                 }} val]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `file` nested inside a list-quoted apply lambda body \
             reached via `after idle` warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: `[list source $file]` / `[list exec …]` directly — no `apply` at
    /// all, `list` command-quoting a hidden command straight.
    #[test]
    fn safe_interp_w129_list_quoted_hidden_command_directly_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { after idle [list source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a bare `[list source …]` in a deferred-call position warns \
             directly, with no `apply` indirection needed: {:?}",
            r.diagnostics
        );
    }

    /// FP guard (mirrors #954's `set data [list apply {…} value]`
    /// non-invocation case, adapted to W129): `[list apply {…} value]`
    /// sitting in ordinary `set` data — not a `Body` / `LambdaLiteral` /
    /// `CommandPrefix` argument position — is never invoked, so it must
    /// never draw W129 even though its lambda body contains a hidden
    /// command.
    #[test]
    fn safe_interp_w129_list_quoted_apply_in_plain_data_is_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 set data [list apply {x { source $x }} value]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a `[list apply …]` value that is only ever stored, never \
             invoked, must not warn: {:?}",
            r.diagnostics
        );
    }

    /// FP guard: the exact same list-quoted-apply-with-a-hidden-command
    /// shape, but with **no** enclosing safe interpreter at all, must not
    /// warn — and, since this fix's whole mechanism is gated on a
    /// non-empty `safe_interp_stack`, must not create any new scope either
    /// (no `apply@…` proc scope, no collateral diagnostics of any other
    /// kind) — this stays exactly as un-analysed as it was before #1001.
    #[test]
    fn list_quoted_apply_lambda_outside_any_safe_interp_is_untouched_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "package ifneeded myPackage 1.0 [list apply {dir {source [file join $dir x]}} $dir]\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "no safe interpreter is involved, so no W129 can ever fire: {:?}",
            r.diagnostics
        );
        assert!(
            r.all_procs.is_empty(),
            "this fix must not widen the general analyser's scope — \
             the list-quoted lambda body stays un-analysed outside a \
             safe-interpreter context, exactly as before #1001: {:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    /// TP: a direct nested `[…]` bracket substitution (no `list`-quoting at
    /// all) — `set x [source b.tcl]` — is an *immediate* invocation
    /// (bracket substitution always evaluates its content right away,
    /// wherever it appears), so it must warn exactly like a bare top-level
    /// `source b.tcl` statement would.
    #[test]
    fn safe_interp_w129_direct_nested_bracket_substitution_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { set x [source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a directly nested `[source …]` substitution warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: the same direct-nested shape, reached through a deeper `[…]` /
    /// braced-body combination (`if {$c} { [exec ls] }` inside another
    /// substitution) — pins that the fix covers arbitrary nesting depth,
    /// not just one level.
    #[test]
    fn safe_interp_w129_direct_nested_bracket_substitution_deep_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { if {[catch {set y [exec ls]} err]} { puts $err } }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a deeply nested `[exec …]` substitution still warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: `{*}[list source $file]` as the *whole* statement — `{*}`
    /// expansion splices `list`'s result into this statement's own argv, so
    /// the command's effective head becomes `source`, even though the
    /// literal head word is the substitution text (never itself a
    /// registry name).
    #[test]
    fn safe_interp_w129_expand_list_quoted_head_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { {*}[list source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "`{{*}}[list source …]` used as the whole statement warns \
             on the effective (expanded) head: {:?}",
            r.diagnostics
        );
    }

    /// TN: `{*}$cmdList` — an opaque variable expansion. Unlike
    /// `{*}[list source $file]`, the value isn't statically known, so this
    /// must NOT warn (matches this codebase's "prefer a miss over a false
    /// positive" stance for dynamic dispatch — the same policy `$cmd $file`
    /// direct dynamic dispatch already gets).
    #[test]
    fn safe_interp_w129_expand_dynamic_var_head_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { set cmdList [list source b.tcl]; {*}$cmdList }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an opaque `{{*}}$var` expansion is not statically resolvable \
             and must not warn: {:?}",
            r.diagnostics
        );
    }

    /// TN: a bare dynamic dispatch via a variable (`set cmd source; $cmd
    /// $file`) is not statically provable and must stay unflagged, matching
    /// this codebase's existing precedent for dynamic command dispatch.
    #[test]
    fn safe_interp_w129_dynamic_variable_dispatch_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { set cmd source; $cmd b.tcl }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "dynamic dispatch through a variable is not statically \
             resolvable and must not warn: {:?}",
            r.diagnostics
        );
    }

    /// TP: `eval [list source $file]` — combining `eval` (a `Body`-role
    /// command) with list-quoting; `eval` evaluates the built string as a
    /// script, immediately invoking `source`.
    #[test]
    fn safe_interp_w129_eval_list_quoted_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { eval [list source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "`eval [list source …]` warns on the resolved head: {:?}",
            r.diagnostics
        );
    }

    /// TP: `uplevel [list source $file]` — the same combination via
    /// `uplevel` instead of `eval`.
    #[test]
    fn safe_interp_w129_uplevel_list_quoted_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\ninterp eval s { uplevel [list source b.tcl] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "`uplevel [list source …]` warns on the resolved head: {:?}",
            r.diagnostics
        );
    }

    /// TN: a *safe* command wrapped the same list-quoted-apply way must not
    /// warn — this fix widens W129's recall, it must not start flagging
    /// ordinary, allowed calls just because they are reached via `[list
    /// apply …]`.
    #[test]
    fn safe_interp_w129_list_quoted_apply_safe_command_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { after idle [list apply {x { puts $x }} val] }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a safe command (`puts`) reached via list-quoted apply must \
             not warn: {:?}",
            r.diagnostics
        );
    }

    /// A locally-redefined hidden-builtin name (issue #945 fault 7 follow-up
    /// — see `safe_interp_child_redefinition_of_a_hidden_builtin_is_callable_945`)
    /// stays callable through this fix's new indirection paths too: once
    /// `proc source {} {…}` has run earlier in the same interpreter body,
    /// `source` is a real, locally-defined command — independent of the
    /// hidden-command table — so a later `[list apply {…} $x]`-nested call
    /// to it must not warn.
    #[test]
    fn safe_interp_w129_redefined_command_not_flagged_through_indirection_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 proc source {} { return ok }\n\
                 after idle [list apply {{} { source }}]\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a locally-redefined name reached via list-quoted apply is \
             callable, not hidden: {:?}",
            r.diagnostics
        );
    }

    /// The standard safe-interpreter delegation pattern — the trusted
    /// parent creates `interp alias s foo {} source`, bridging its *own*
    /// (non-hidden) `source` into the child under a new name — must not
    /// warn when `foo` is called inside the child, including through this
    /// fix's new indirection paths: `foo` is never itself a
    /// `SAFE_INTERP_HIDDEN` registry name, so the gate correctly leaves it
    /// alone (rename/`interp alias` cannot resurrect a *hidden* command's
    /// callability from within the child — confirmed against the `tcl-vm`
    /// runtime, which resolves both only through the ordinary, hidden-
    /// command-free lookup table).
    #[test]
    fn safe_interp_w129_alias_bridged_command_not_flagged_through_indirection_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp alias s foo {} source\n\
             interp eval s { after idle [list apply {{} { foo }}] }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an alias bridging a capability in from the trusted parent is \
             not itself a hidden registry name, so it must not warn: {:?}",
            r.diagnostics
        );
    }

    /// `namespace inscope ::x { proc foo }` runs the body in `::x`, so `foo`
    /// homes to `::x::foo` — the same namespace frame as `namespace eval`, not
    /// the caller's scope.
    #[test]
    fn namespace_inscope_runs_the_body_in_the_named_namespace() {
        let mut a = Analyser::new();
        let r = a.analyse("namespace inscope ::x { proc foo {} {} }\n", "tcl8.6");
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::x::foo"),
            "inscope body should home to the named namespace: {keys:?}",
        );
        assert!(
            !r.all_procs.contains_key("::foo"),
            "the body must not resolve in the caller's global scope: {keys:?}",
        );
    }

    /// `namespace code { proc foo }` captures the *current* namespace, so its
    /// script is analysed in this scope — inside `::x`, `foo` homes to
    /// `::x::foo`.
    #[test]
    fn namespace_code_analyses_the_script_in_the_current_namespace() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace eval ::x { namespace code { proc foo {} {} } }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::x::foo"),
            "namespace code script should analyse in the current namespace: {keys:?}",
        );
    }

    /// `interp eval {} { proc foo }` targets the *current* interpreter, so
    /// `foo` is the parent's `::foo` (no isolation, no synthetic namespace).
    #[test]
    fn interp_eval_empty_path_is_the_current_interpreter() {
        let mut a = Analyser::new();
        let r = a.analyse("interp eval {} { proc foo {} {} }\n", "tcl8.6");
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::foo"),
            "current-interp proc: {keys:?}"
        );
        assert!(
            !keys.iter().any(|k| k.contains("::::")),
            "an empty path must not open a synthetic namespace: {keys:?}",
        );
    }

    #[test]
    fn handle_namespace_eval_creates_child_scope() {
        let mut a = Analyser::new();
        let handled = a.handle_namespace_eval_command(
            &[
                "eval".to_string(),
                "ns1".to_string(),
                "proc inner {} {}".to_string(),
            ],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 35)),
            ],
            &[],
            &[],
        );
        assert!(handled);
        assert_eq!(a.result.global_scope.children.len(), 1);
        assert_eq!(a.result.global_scope.children[0].name, "ns1");
        assert_eq!(
            a.result.global_scope.children[0].kind,
            crate::analyser::types::ScopeKind::Namespace,
        );
    }

    #[test]
    fn handle_namespace_eval_records_body_span() {
        let mut a = Analyser::new();
        a.handle_namespace_eval_command(
            &["eval".to_string(), "ns1".to_string(), String::new()],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 35)),
            ],
            &[],
            &[],
        );
        assert_eq!(
            a.result.global_scope.children[0].body_span,
            Some(span(19, 35))
        );
    }

    #[test]
    fn handle_namespace_eval_dynamic_target_gets_a_synthetic_span_keyed_name() {
        // TP — regression for a bug found by differential audit against
        // irc.tcl's per-connection `namespace eval $name { … }` idiom: a
        // dynamic target must never become the scope's `.name` verbatim (two
        // unrelated occurrences sharing the same variable name would then
        // collapse into one scope), so it's replaced with a synthetic name
        // keyed on this occurrence's own token offset.
        let mut a = Analyser::new();
        a.handle_namespace_eval_command(
            &[
                "eval".to_string(),
                "$name".to_string(),
                "proc inner {} {}".to_string(),
            ],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 20)),
                str_tok(span(21, 37)),
            ],
            &[],
            &[],
        );
        assert_eq!(a.result.global_scope.children[0].name, "@dynns@15");
    }

    #[test]
    fn handle_namespace_eval_literal_target_keeps_its_written_name() {
        // FN guard — a literal (non-dynamic) target must still use its own
        // written text verbatim, exactly as before the fix.
        let mut a = Analyser::new();
        a.handle_namespace_eval_command(
            &["eval".to_string(), "ns1".to_string(), String::new()],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 35)),
            ],
            &[],
            &[],
        );
        assert_eq!(a.result.global_scope.children[0].name, "ns1");
    }

    #[test]
    fn two_dynamic_namespace_eval_blocks_sharing_a_variable_name_never_collide() {
        // TP — the full-pipeline shape of the irc.tcl bug: two lexically
        // unrelated procs each open `namespace eval $name { … }` with a
        // same-named local `name` (an unremarkable choice for this exact
        // idiom) and each define their own `helper`. Before the fix both
        // blocks' scope shared the literal text "$name", so the second
        // `helper` silently overwrote the first in the flat `all_procs` map.
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc setupA {} {\n    set name connA\n    namespace eval $name {\n        proc helper {} { return A }\n    }\n}\n\
             proc setupB {} {\n    set name connB\n    namespace eval $name {\n        proc helper {} { return B }\n    }\n}\n",
            "tcl8.6",
        );
        // drift-ok: test assertion counting distinct entries by simple name
        // to prove no cross-occurrence merge, not a resolution decision.
        let helpers: Vec<_> = r
            .all_procs
            .iter()
            .filter(|(_, p)| p.name == "helper")
            .collect();
        assert_eq!(
            helpers.len(),
            2,
            "both blocks' helper must survive as distinct entries: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );
        assert_ne!(
            helpers[0].0, helpers[1].0,
            "distinct call sites must get distinct qualified keys: {helpers:?}",
        );
        assert_ne!(
            helpers[0].1.body_span, helpers[1].1.body_span,
            "each is genuinely its own definition, not the same one twice: {helpers:?}",
        );
    }

    // handle_namespace_ensemble

    #[test]
    fn handle_namespace_ensemble_records_in_set() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(&["ensemble".to_string(), "create".to_string()], &[], &[0]);
        assert!(a.ensemble_namespaces.contains("::myns"));
    }

    #[test]
    fn handle_namespace_ensemble_command_option_recorded() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        // `-command` recognition is registry-driven (`ENSEMBLE_CREATE_OPTIONS`).
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(
            &[
                "ensemble".to_string(),
                "create".to_string(),
                "-command".to_string(),
                "::ens".to_string(),
            ],
            &[],
            &[0],
        );
        assert!(a.ensemble_namespaces.contains("::myns"));
        assert!(a.ensemble_namespaces.contains("::ens"));
    }

    #[test]
    fn handle_namespace_ensemble_command_option_qualifies_relative_name() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        // `-command` recognition is registry-driven (`ENSEMBLE_CREATE_OPTIONS`).
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(
            &[
                "ensemble".to_string(),
                "create".to_string(),
                "-command".to_string(),
                "ens".to_string(),
            ],
            &[],
            &[0],
        );
        assert!(a.ensemble_namespaces.contains("::myns::ens"));
    }

    #[test]
    fn handle_namespace_ensemble_command_option_dynamic_value_not_recorded() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(
            &[
                "ensemble".to_string(),
                "create".to_string(),
                "-command".to_string(),
                "$dyn".to_string(),
            ],
            &[],
            &[0],
        );
        assert_eq!(
            a.ensemble_namespaces,
            std::collections::HashSet::from(["::myns".to_string()])
        );
    }

    #[test]
    fn namespace_ensemble_map_and_subcommands_record_command_references() {
        // `-map {get ::foo::getImpl}` — the odd element is the target command;
        // `-subcommands {show}` maps `show` to `::foo::show`.  Both become
        // command references so navigation reaches the implementing procs.
        let mut a = Analyser::new();
        let src = "namespace eval ::foo {\n    \
                   proc getImpl {} {}\n    \
                   proc show {} {}\n    \
                   namespace ensemble create -map {get ::foo::getImpl} -subcommands {show}\n\
                   }\n";
        let r = a.analyse(src, "tcl8.6");
        let resolved: Vec<&str> = r
            .command_invocations
            .iter()
            .filter_map(|i| i.resolved_qualified_name.as_deref())
            .collect();
        assert!(
            resolved.contains(&"::foo::getImpl"),
            "the -map target should be a command reference: {resolved:?}",
        );
        assert!(
            resolved.contains(&"::foo::show"),
            "the -subcommands name should map to `<ns>::show`: {resolved:?}",
        );
    }

    /// Issue #923 idx 85 (tk-shaped): the ensemble-creating command runs inside
    /// a proc whose *qualified name* homes it to `::tk`, but which is declared
    /// at the top level with no enclosing `namespace eval`.  `namespace
    /// current` there is `::tk` (tclsh 8.6.16 / 9.0.4-verified: the ensemble
    /// created is `::tk` and `tk alpha` dispatches to `::tk::alpha`), so the
    /// subcommand must map to `::tk::alpha` — the purely lexical namespace walk
    /// skips proc scopes and mapped it to `::alpha`.
    #[test]
    fn namespace_ensemble_create_in_a_qualified_name_proc_homes_to_that_namespace_923_idx85() {
        let mut a = Analyser::new();
        let src = "namespace eval ::tk {}\n\
                   proc ::tk::alpha {} {}\n\
                   proc ::tk::SetupEnsemble {} {\n    \
                   namespace ensemble create -subcommands {alpha}\n\
                   }\n";
        let r = a.analyse(src, "tcl8.6");
        let resolved: Vec<&str> = r
            .command_invocations
            .iter()
            .filter_map(|i| i.resolved_qualified_name.as_deref())
            .collect();
        assert!(
            resolved.contains(&"::tk::alpha"),
            "the subcommand should map to the proc's defining namespace: {resolved:?}",
        );
        assert!(
            !resolved.contains(&"::alpha"),
            "the lexical (global) namespace is the wrong home: {resolved:?}",
        );
        assert!(
            a.ensemble_namespaces.contains("::tk"),
            "the ensemble command itself is `::tk`: {:?}",
            a.ensemble_namespaces,
        );
    }

    /// TN control for the above: the ordinary lexical case — the ensemble is
    /// created directly inside `namespace eval ::foo`, so both the lexical and
    /// the command-resolution namespace agree on `::foo`.  Pinned so the idx 85
    /// fix cannot regress the common shape.
    #[test]
    fn namespace_ensemble_create_lexically_inside_namespace_eval_is_unchanged_923_idx85() {
        let mut a = Analyser::new();
        let src = "namespace eval ::foo {\n    \
                   proc alpha {} {}\n    \
                   namespace ensemble create -subcommands {alpha}\n\
                   }\n";
        let r = a.analyse(src, "tcl8.6");
        let resolved: Vec<&str> = r
            .command_invocations
            .iter()
            .filter_map(|i| i.resolved_qualified_name.as_deref())
            .collect();
        assert!(
            resolved.contains(&"::foo::alpha"),
            "lexical case still maps to `::foo::alpha`: {resolved:?}",
        );
        assert!(a.ensemble_namespaces.contains("::foo"));
    }

    /// The definition-homing sweep behind issue #923 idx 85: every analyser
    /// site that asks "which namespace is current here?" now answers with
    /// [`Analyser::command_resolution_namespace`], so a definition made inside
    /// a qualified-name proc's body homes to that proc's *defining* namespace.
    /// These are cases (A)–(D) from `docs/design/name-resolution.md`
    /// §3.5, each with the real interpreter's answer as the expectation.
    #[test]
    fn definitions_inside_a_qualified_name_proc_home_to_its_defining_namespace_923_idx85() {
        // (A) The already-correct lexical control.
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace eval ::x { proc mk {} { proc helper {} {} } }\n",
            "tcl8.6",
        );
        assert!(
            r.all_procs.contains_key("::x::helper"),
            "(A) lexical case: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );

        // (B) A qualified-name proc with no enclosing `namespace eval`.
        let mut a = Analyser::new();
        let r = a.analyse("proc ::x::mk {} { proc helper {} {} }\n", "tcl8.6");
        assert!(
            r.all_procs.contains_key("::x::helper"),
            "(B) qualified encloser: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );

        // (C) A qualified-name proc whose own name overrides the lexical
        // `namespace eval` it sits in.
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace eval ::x { proc ::y::mk {} { proc helper {} {} } }\n",
            "tcl8.6",
        );
        assert!(
            r.all_procs.contains_key("::y::helper"),
            "(C) absolute name rebases: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );

        // (D) The same rule for a class, not a proc.
        let mut a = Analyser::new();
        let r = a.analyse("proc ::x::mk {} { oo::class create Helper {} }\n", "tcl8.6");
        assert!(
            r.all_classes.contains_key("::x::Helper"),
            "(D) class create: {:?}",
            r.all_classes.keys().collect::<Vec<_>>(),
        );
    }

    /// The false positive the idx 85 family caused: a nested definition that
    /// mis-homed to `::` collided with a real global of the same name and one
    /// silently overwrote the other.  Both must survive under their own keys.
    #[test]
    fn a_nested_definition_no_longer_overwrites_a_same_named_global_923_idx85() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc helper {} { return global }\n\
             proc ::x::mk {} { proc helper {} { return nested } }\n",
            "tcl8.6",
        );
        let keys: Vec<&str> = r.all_procs.keys().map(String::as_str).collect();
        assert!(
            r.all_procs.contains_key("::helper"),
            "global lost: {keys:?}"
        );
        assert!(
            r.all_procs.contains_key("::x::helper"),
            "nested lost: {keys:?}",
        );
    }

    /// `namespace import` written inside a qualified-name proc imports into
    /// that proc's defining namespace, not the global one (issue #923 idx 85).
    #[test]
    fn namespace_import_inside_a_qualified_name_proc_targets_that_namespace_923_idx85() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace eval ::src { proc thing {} {} namespace export thing }\n\
             proc ::dst::setup {} { namespace import ::src::thing }\n",
            "tcl8.6",
        );
        let targets: Vec<&str> = r.namespace_imports.iter().map(|i| i.ns.as_str()).collect();
        assert!(
            targets.contains(&"::dst"),
            "the import lands in the proc's defining namespace: {targets:?}",
        );
    }

    /// A `-map` target inside the same qualified-name proc resolves the same
    /// way (issue #923 idx 85, the `-map` half the finding actually named).
    #[test]
    fn namespace_ensemble_map_in_a_qualified_name_proc_resolves_targets_923_idx85() {
        let mut a = Analyser::new();
        let src = "namespace eval ::tk {}\n\
                   proc ::tk::getImpl {} {}\n\
                   proc ::tk::SetupEnsemble {} {\n    \
                   namespace ensemble create -map {get getImpl}\n\
                   }\n";
        let r = a.analyse(src, "tcl8.6");
        let resolved: Vec<&str> = r
            .command_invocations
            .iter()
            .filter_map(|i| i.resolved_qualified_name.as_deref())
            .collect();
        assert!(
            resolved.contains(&"::tk::getImpl"),
            "the -map target resolves in the proc's defining namespace: {resolved:?}",
        );
    }

    // Issue #923 idx 85, the *call-site* half.  Everything above pins the
    // ensemble's own declaration (where the `-map`/`-subcommands` targets
    // home to); these pin the downstream `<ensemble> <sub>` dispatch sites,
    // which is what find-references / rename / code-lens / call-hierarchy
    // enumerate.  Oracle — tclsh 8.6.16 and 9.0.4 both print
    // `shown` then `configured:-x 1` for `VIAPROC_SRC`, so
    // `::app::widget show` really does dispatch to `::app::widget::Show`.

    /// The finding's exact shape: `namespace ensemble create -map` inside a
    /// proc declared with a fully-qualified name at top level, with no
    /// enclosing `namespace eval`, and the dispatch call sites written after
    /// it — one nested in a `[…]` substitution, one at the top level.
    const VIAPROC_SRC: &str = "namespace eval ::app::widget {\n    \
         variable state 0\n\
         }\n\
         proc ::app::widget::Setup {} {\n    \
         namespace ensemble create -map {\n        \
         show      ::app::widget::Show\n        \
         configure ::app::widget::Configure\n    \
         }\n\
         }\n\
         proc ::app::widget::Show {} { puts \"shown\" }\n\
         proc ::app::widget::Configure {args} { puts \"configured:$args\" }\n\
         ::app::widget::Setup\n\
         puts [::app::widget show]\n\
         ::app::widget configure -x 1\n";

    /// Every `(written name, resolved name)` pair recorded as an
    /// existence-probed reference — the shape
    /// `record_ensemble_subcommand_invocation` gives a subcommand word.
    fn probe_refs(r: &super::super::types::AnalysisResult) -> Vec<(String, String)> {
        r.command_invocations
            .iter()
            .filter(|i| i.existence_probe)
            .map(|i| {
                (
                    i.name.clone(),
                    i.resolved_qualified_name.clone().unwrap_or_default(),
                )
            })
            .collect()
    }

    #[test]
    fn ensemble_call_sites_resolve_when_the_ensemble_is_created_in_a_qualified_proc_923_idx85() {
        let r = Analyser::new().analyse(VIAPROC_SRC, "tcl8.6").clone();
        let probes = probe_refs(&r);
        assert!(
            probes.contains(&("show".to_owned(), "::app::widget::Show".to_owned())),
            "the `[::app::widget show]` dispatch must be a reference to Show: {probes:?}",
        );
        assert!(
            probes.contains(&(
                "configure".to_owned(),
                "::app::widget::Configure".to_owned()
            )),
            "the top-level `::app::widget configure` dispatch must be a reference \
             to Configure: {probes:?}",
        );
    }

    #[test]
    fn per_item_records_the_same_ensemble_call_sites_as_the_whole_file_walk_923_idx85() {
        // The regression that shipped: the per-item shell pass defers
        // `::app::widget::Setup`'s body, so the ensemble map was still empty
        // when the two call sites below it were walked and neither
        // subcommand reference was ever recorded — go-to-definition (an
        // on-demand lookup against the finished analysis) still answered,
        // find-references could not.  Comparing the two walk strategies
        // directly is the assertion that cannot drift.
        let whole = Analyser::new().analyse(VIAPROC_SRC, "tcl8.6").clone();
        let per_item = Analyser::new().analyse_per_item(VIAPROC_SRC, "tcl8.6");
        assert_eq!(
            probe_refs(&per_item),
            probe_refs(&whole),
            "per-item must record the same ensemble dispatch references as the \
             whole-file walk",
        );
        assert!(
            probe_refs(&per_item)
                .iter()
                .any(|(_, res)| res == "::app::widget::Show"),
            "and both must actually find Show's call site: {:?}",
            probe_refs(&per_item),
        );
    }

    #[test]
    fn a_dynamic_ensemble_map_records_no_call_site_reference_923_idx85() {
        // TN — deliberate abstention.  `-map [list …]` is not a literal
        // list, so no `subcommand -> target` fact exists at all; the
        // dispatch site must stay unattributed rather than being guessed
        // at from the surrounding text.  Both walk strategies abstain.
        let src = "proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map [list show ::app::widget::Show]\n\
                   }\n\
                   proc ::app::widget::Show {} {}\n\
                   ::app::widget::Setup\n\
                   ::app::widget show\n";
        for r in [
            Analyser::new().analyse(src, "tcl8.6").clone(),
            Analyser::new().analyse_per_item(src, "tcl8.6"),
        ] {
            assert!(
                !probe_refs(&r)
                    .iter()
                    .any(|(_, res)| res == "::app::widget::Show"),
                "a dynamic -map must not attribute the dispatch site: {:?}",
                probe_refs(&r),
            );
        }
    }

    #[test]
    fn an_ensemble_call_site_above_its_declaration_is_not_attributed_923_idx85() {
        // TN — the whole-file DFS walks a proc body at its *definition*
        // point, so a dispatch written above the proc that creates the
        // ensemble sees no map.  The deferred replay reproduces that
        // visibility rule rather than widening it, which is what keeps the
        // two walk strategies byte-identical.
        let src = "proc ::app::widget::Show {} {}\n\
                   ::app::widget show\n\
                   proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map {show ::app::widget::Show}\n\
                   }\n";
        for r in [
            Analyser::new().analyse(src, "tcl8.6").clone(),
            Analyser::new().analyse_per_item(src, "tcl8.6"),
        ] {
            assert!(
                !probe_refs(&r)
                    .iter()
                    .any(|(_, res)| res == "::app::widget::Show"),
                "a dispatch above the declaration must not be attributed: {:?}",
                probe_refs(&r),
            );
        }
    }

    // `namespace ensemble configure` (issue #923 idx 84): the real
    // `tk/library/systray.tcl` idiom splices new subcommands onto a
    // *pre-existing* ensemble via `configure`, not `create` — previously
    // invisible to `handle_namespace_ensemble` entirely.

    #[test]
    fn namespace_ensemble_configure_extends_a_preexisting_ensembles_map() {
        // TP — the mechanism in isolation, with a literal `-map` value and a
        // deliberately non-tk ensemble name (`myens`), proving the fix is
        // registry-agnostic rather than hardcoded to `tk`.
        let mut a = Analyser::new();
        let src = "namespace eval ::myens {\n    \
                   namespace ensemble create -subcommands {}\n\
                   }\n\
                   proc ::myens::extra {args} {}\n\
                   namespace ensemble configure ::myens -map {extra ::myens::extra}\n\
                   myens extra\n";
        let r = a.analyse(src, "tcl8.6");
        let resolved: Vec<&str> = r
            .command_invocations
            .iter()
            .filter_map(|i| i.resolved_qualified_name.as_deref())
            .collect();
        assert!(
            resolved.contains(&"::myens::extra"),
            "a configure-spliced -map target should be a command reference: {resolved:?}",
        );
    }

    #[test]
    fn namespace_ensemble_configure_dynamic_name_is_not_recorded() {
        // FP-guard — `namespace ensemble configure $x -map {...}` can't be
        // resolved statically; the whole call must abstain rather than
        // recording a `-map` splice under a guessed/wrong key.
        let mut a = Analyser::new();
        a.handle_namespace_ensemble(
            &[
                "ensemble".to_string(),
                "configure".to_string(),
                "$x".to_string(),
                "-map".to_string(),
                "sub".to_string(),
                "::real::target".to_string(),
            ],
            &[],
            &[],
        );
        assert!(a.result.ensemble_subcommand_targets.is_empty());
    }

    #[test]
    fn namespace_ensemble_configure_with_no_name_argument_is_a_no_op() {
        // TN — the bare query form `namespace ensemble configure` (no NAME,
        // no options) must not panic on `args[2]` indexing.
        let mut a = Analyser::new();
        a.handle_namespace_ensemble(&["ensemble".to_string(), "configure".to_string()], &[], &[]);
        assert!(a.result.ensemble_subcommand_targets.is_empty());
    }

    #[test]
    fn namespace_ensemble_configure_dict_merge_literal_tail_is_extracted() {
        // TP — the real, exact idiom (issue #923 idx 84):
        // `namespace ensemble configure NAME -map [dict merge [namespace
        // ensemble configure NAME -map] {literal}]`. The self-referential
        // query argument is left unknown, but the literal tail's own pairs
        // are statically known regardless of what it evaluates to.
        let mut a = Analyser::new();
        let src = "namespace eval ::myens {\n    \
                   namespace ensemble create -subcommands {}\n\
                   }\n\
                   proc ::myens::extra {args} {}\n\
                   proc ::myens::other {args} {}\n\
                   namespace ensemble configure ::myens -map \
                   [dict merge [namespace ensemble configure ::myens -map] \
                   {extra ::myens::extra other ::myens::other}]\n\
                   myens extra\n\
                   myens other\n";
        let r = a.analyse(src, "tcl8.6");
        let resolved: Vec<&str> = r
            .command_invocations
            .iter()
            .filter_map(|i| i.resolved_qualified_name.as_deref())
            .collect();
        assert!(
            resolved.contains(&"::myens::extra"),
            "the dict-merge literal tail's first pair should resolve: {resolved:?}",
        );
        assert!(
            resolved.contains(&"::myens::other"),
            "the dict-merge literal tail's second pair should resolve too, \
             proving the extraction isn't overfit to a single pair: {resolved:?}",
        );
    }

    #[test]
    fn namespace_ensemble_configure_unrecognised_dynamic_map_shape_abstains_safely() {
        // Safety regression: a `-map` value that is itself one whole
        // dynamic `[...]` substitution NOT matching the narrow `dict merge
        // ARG {literal}` shape must abstain entirely — not naively
        // word-split the expression's own source text into bogus
        // subcommand/target pairs. `[linsert {} 0 foo bar]` evaluates to
        // the *string* "foo bar" at runtime (one call to `linsert`; "foo"/
        // "bar" are plain data words, never independently invoked) — were
        // the value's raw text naively whitespace-split instead of
        // abstained on, "foo" would land at an odd (target) index and
        // wrongly resolve to the real `::myens::foo` defined below,
        // recording a spurious command reference to it.
        let mut a = Analyser::new();
        let src = "namespace eval ::myens {\n    \
                   namespace ensemble create -subcommands {}\n\
                   }\n\
                   proc ::myens::foo {} {}\n\
                   namespace ensemble configure ::myens -map [linsert {} 0 foo bar]\n";
        let r = a.analyse(src, "tcl8.6");
        assert!(
            r.ensemble_subcommand_targets
                .get("::myens")
                .is_none_or(std::collections::HashMap::is_empty),
            "an unrecognised dynamic -map shape must record no subcommand \
             targets: {:?}",
            r.ensemble_subcommand_targets.get("::myens"),
        );
        let resolved: Vec<&str> = r
            .command_invocations
            .iter()
            .filter_map(|i| i.resolved_qualified_name.as_deref())
            .collect();
        assert!(
            !resolved.contains(&"::myens::foo"),
            "must not record a spurious command reference from splitting \
             the dynamic expression's own text: {resolved:?}",
        );
    }

    #[test]
    fn ensemble_subcommand_targets_record_which_option_declared_them() {
        // Issue #1281: the two options bind the subcommand word to its
        // target in opposite ways, so the recorded fact has to say which one
        // wrote it. Oracle (tclsh 8.6.14 / 9.0.4, identical): with `-map
        // {show ::app::widget::Show}` the call `::app::widget Show` is
        // `unknown or ambiguous subcommand "Show": must be show` — the key is
        // arbitrary; with `-subcommands {alpha}` the ensemble derives
        // `::app::widget::alpha` from the entry, and a `-subcommands {alpha}`
        // whose proc is named `beta` is `invalid command name "alpha"`.
        use crate::signature_scan::types::EnsembleSubcommandProvenance;

        let mut a = Analyser::new();
        let map_src = "namespace eval ::app::widget {}\n\
                       proc ::app::widget::Show {} {}\n\
                       namespace ensemble create -command ::app::widget \
                       -map {show ::app::widget::Show}\n";
        let r = a.analyse(map_src, "tcl8.6");
        let entry = r
            .ensemble_subcommand_targets
            .get("::app::widget")
            .and_then(|subs| subs.get("show"))
            .expect("the literal -map pair is recorded");
        assert_eq!(entry.target, "::app::widget::Show");
        assert_eq!(
            entry.provenance,
            EnsembleSubcommandProvenance::Map,
            "a -map pair must be tagged as such",
        );

        let mut a = Analyser::new();
        let sub_src = "namespace eval ::app::widget {\n    \
                       proc alpha {} {}\n    \
                       namespace ensemble create -command ::app::widget \
                       -subcommands {alpha}\n\
                       }\n";
        let r = a.analyse(sub_src, "tcl8.6");
        let entry = r
            .ensemble_subcommand_targets
            .get("::app::widget")
            .and_then(|subs| subs.get("alpha"))
            .expect("the literal -subcommands entry is recorded");
        assert_eq!(entry.target, "::app::widget::alpha");
        assert_eq!(
            entry.provenance,
            EnsembleSubcommandProvenance::Subcommands,
            "a -subcommands entry must be tagged as such",
        );
    }

    #[test]
    fn ensemble_dispatch_call_sites_carry_their_mapping_provenance() {
        // The dispatch word's own invocation record carries the provenance
        // (issue #1281), because only the recording site knows *this span* is
        // a subcommand word: a `-map {Show ::app::widget::Show}` whose key
        // happens to equal the target's tail is textually indistinguishable
        // from an ordinary bare call to the target, so a consumer that
        // re-derived the fact by name would gate the wrong spans.
        use crate::signature_scan::types::EnsembleSubcommandProvenance;

        let mut a = Analyser::new();
        let map_src = "namespace eval ::app::widget {}\n\
                       proc ::app::widget::Show {} {}\n\
                       namespace ensemble create -command ::app::widget \
                       -map {show ::app::widget::Show}\n\
                       ::app::widget show\n";
        let r = a.analyse(map_src, "tcl8.6");
        let tagged: Vec<(&str, Option<EnsembleSubcommandProvenance>)> = r
            .command_invocations
            .iter()
            .filter(|i| i.resolved_qualified_name.as_deref() == Some("::app::widget::Show"))
            .map(|i| (i.name.as_str(), i.ensemble_dispatch))
            .collect();
        assert!(
            tagged.contains(&("show", Some(EnsembleSubcommandProvenance::Map))),
            "the dispatch word is tagged with the -map provenance: {tagged:?}",
        );
        assert!(
            tagged
                .iter()
                .any(|(name, prov)| *name == "::app::widget::Show" && prov.is_none()),
            "the -map *value* is an ordinary reference, not a dispatch word — \
             it carries the target's own name and rename must rewrite it: {tagged:?}",
        );

        let mut a = Analyser::new();
        let sub_src = "namespace eval ::app::widget {\n    \
                       proc alpha {} {}\n    \
                       namespace ensemble create -command ::app::widget \
                       -subcommands {alpha}\n\
                       }\n\
                       ::app::widget alpha\n";
        let r = a.analyse(sub_src, "tcl8.6");
        let tagged: Vec<(&str, Option<EnsembleSubcommandProvenance>)> = r
            .command_invocations
            .iter()
            .filter(|i| i.resolved_qualified_name.as_deref() == Some("::app::widget::alpha"))
            .map(|i| (i.name.as_str(), i.ensemble_dispatch))
            .collect();
        assert!(
            tagged.contains(&("alpha", Some(EnsembleSubcommandProvenance::Subcommands))),
            "the dispatch word is tagged with the -subcommands provenance: {tagged:?}",
        );
    }

    #[test]
    fn handle_namespace_ensemble_other_options_value_word_is_not_mistaken_for_command_flag() {
        // Regression: before the registry-driven option walk, the scan
        // checked *every* word for literal equality with `-command`,
        // including another option's own value word — `-map`'s value
        // here is (pathologically, but syntactically legally) the string
        // `-command`. A word-by-word scan misreads that value as the
        // `-command` flag itself and steals the *next* word (the real
        // `-command`'s own flag) as if it were a namespace name. Walking
        // by each option's declared value arity instead correctly skips
        // `-map`'s whole value before ever looking for `-command` again,
        // so only the genuine `-command ::real::target` is recorded.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(
            &[
                "ensemble".to_string(),
                "create".to_string(),
                "-map".to_string(),
                "-command".to_string(),
                "-command".to_string(),
                "::real::target".to_string(),
            ],
            &[],
            &[0],
        );
        assert_eq!(
            a.ensemble_namespaces,
            std::collections::HashSet::from(["::myns".to_string(), "::real::target".to_string()]),
            "only the genuine -command flag's value must be recorded, \
             not -map's value word that happens to read \"-command\""
        );
    }

    #[test]
    fn handle_namespace_ensemble_global_scope_no_op() {
        let mut a = Analyser::new();
        a.handle_namespace_ensemble(&["ensemble".to_string(), "create".to_string()], &[], &[]);
        assert!(a.ensemble_namespaces.is_empty());
    }

    #[test]
    fn handle_namespace_ensemble_wrong_subcommand_no_op() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(&["eval".to_string(), "myns".to_string()], &[], &[0]);
        assert!(a.ensemble_namespaces.is_empty());
    }

    // handle_foreach_command

    #[test]
    fn handle_foreach_defines_single_loop_var() {
        let mut a = Analyser::new();
        let handled = a.handle_foreach_command(
            &[
                "i".to_string(),
                "{1 2 3}".to_string(),
                "puts $i".to_string(),
            ],
            &[
                esc_tok(span(8, 9)),
                str_tok(span(10, 17)),
                str_tok(span(18, 28)),
            ],
            &[],
        );
        assert!(handled);
        assert!(a.result.global_scope.variables.contains_key("i"));
    }

    #[test]
    fn handle_foreach_defines_multiple_loop_vars() {
        let mut a = Analyser::new();
        a.handle_foreach_command(
            &["k v".to_string(), "{a 1 b 2}".to_string(), String::new()],
            &[
                esc_tok(span(8, 11)),
                str_tok(span(12, 21)),
                str_tok(span(22, 24)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("k"));
        assert!(a.result.global_scope.variables.contains_key("v"));
        // Each name gets its own span inside the var-list content ("k v" at
        // bytes 8..11), not the shared whole-token span — so offset-sorted
        // consumers see them in declaration order (`k` before `v`).
        let k = &a.result.global_scope.variables["k"];
        let v = &a.result.global_scope.variables["v"];
        assert_eq!(k.definition_span, span(8, 9));
        assert_eq!(v.definition_span, span(10, 11));
        assert!(k.definition_span.start() < v.definition_span.start());
    }

    #[test]
    fn handle_foreach_varlist_uses_tcl_list_names_and_source_ranges() {
        let mut a = Analyser::new();
        let var_list = r#"{one name} "two name" three\ name plain"#;
        a.handle_foreach_command(
            &[var_list.to_string(), "values".to_string(), String::new()],
            &[
                esc_tok(span(100, 139)),
                esc_tok(span(140, 146)),
                str_tok(span(147, 149)),
            ],
            &[],
        );

        let variables = &a.result.global_scope.variables;
        assert_eq!(variables["one name"].definition_span, span(101, 109));
        assert_eq!(variables["two name"].definition_span, span(112, 120));
        assert_eq!(variables["three name"].definition_span, span(122, 133));
        assert_eq!(variables["plain"].definition_span, span(134, 139));
    }

    #[test]
    fn malformed_foreach_varlist_defines_no_variables() {
        let mut a = Analyser::new();
        a.handle_foreach_command(
            &[
                "{unterminated".to_string(),
                "values".to_string(),
                String::new(),
            ],
            &[
                esc_tok(span(0, 13)),
                esc_tok(span(14, 20)),
                str_tok(span(21, 23)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.is_empty());
    }

    #[test]
    fn literal_foreach_binding_parses_single_variable_as_tcl_list() {
        assert_eq!(
            Analyser::literal_foreach_binding(r"{one name}", "alpha beta", true),
            Some((
                "one name".to_string(),
                vec!["alpha".to_string(), "beta".to_string()]
            ))
        );
        assert!(Analyser::literal_foreach_binding("{unterminated", "alpha", true).is_none());
        assert!(Analyser::literal_foreach_binding("one two", "alpha", true).is_none());
    }

    #[test]
    fn handle_foreach_defines_every_varlist_in_the_multi_list_lock_step_form() {
        // Issue #923 idx 70 (main audit wave, high severity, pix corpus):
        // `foreach varList1 list1 varList2 list2 ... body` — the parallel/
        // lock-step multi-list form (docs/pixdoc.tcl's real shape:
        // `foreach dirName {...} name {...} {...}`) — is fully static,
        // unambiguous, standard Tcl (tclsh 8.6/9.0-verified) and is even
        // arity-validated by the registry's own `foreach` spec
        // (`Arity::stepped(3, Arity::UNLIMITED, 2)`, stride 2). Previously
        // only the *first* varList (`args[0]`) was ever bound — every
        // subsequent varList/list pair's names were silently dropped, so
        // `name` was never registered as a local at all.
        let mut a = Analyser::new();
        let handled = a.handle_foreach_command(
            &[
                "dirName".to_string(),
                "{src src {src core}}".to_string(),
                "name".to_string(),
                "{alpha beta gamma}".to_string(),
                "puts $dirName-$name".to_string(),
            ],
            &[
                esc_tok(span(8, 15)),
                str_tok(span(16, 37)),
                esc_tok(span(38, 42)),
                str_tok(span(43, 62)),
                str_tok(span(63, 90)),
            ],
            &[],
        );
        assert!(handled);
        assert!(a.result.global_scope.variables.contains_key("dirName"));
        assert!(
            a.result.global_scope.variables.contains_key("name"),
            "the second varList's own loop variable must be bound too, not silently dropped"
        );
    }

    #[test]
    fn handle_foreach_multi_list_form_binds_a_third_pair_too() {
        // FN guard — the fix must generalise past exactly 2 pairs (a
        // hardcoded "first + second" special case would still miss a
        // 3-or-more-pair `foreach`, equally legal Tcl).
        let mut a = Analyser::new();
        a.handle_foreach_command(
            &[
                "a".to_string(),
                "{1 2}".to_string(),
                "b".to_string(),
                "{3 4}".to_string(),
                "c".to_string(),
                "{5 6}".to_string(),
                "puts $a$b$c".to_string(),
            ],
            &[
                esc_tok(span(0, 1)),
                str_tok(span(2, 7)),
                esc_tok(span(8, 9)),
                str_tok(span(10, 15)),
                esc_tok(span(16, 17)),
                str_tok(span(18, 23)),
                str_tok(span(24, 35)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("a"));
        assert!(a.result.global_scope.variables.contains_key("b"));
        assert!(a.result.global_scope.variables.contains_key("c"));
    }

    #[test]
    fn handle_foreach_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_foreach_command(
            &["i".to_string(), "{1 2}".to_string()],
            &[esc_tok(span(0, 1)), str_tok(span(2, 7))],
            &[],
        );
        assert!(!handled);
    }

    // Dynamic proc names + the foreach rename-and-reinstall idiom (issue
    // #923 idx 86): `tk/library/accessibility.tcl`'s `foreach wtype {...} {
    // rename ::$wtype ::tk::accessible::orig_$wtype ; proc ::$wtype {args}
    // {...} }` renames each classic widget command away and reinstalls a
    // wrapper under the same original name.

    #[test]
    fn handle_proc_command_dynamic_name_resolves_via_constant_fold() {
        // TP — the finding's own non-foreach isolation repro: a plain `set`
        // constant, no loop at all. `proc ::$wtype {...}` previously never
        // attempted to constant-fold its name at all (unlike `rename`,
        // fixed for idx 3), registering under the literal garbled text
        // instead of resolving `wtype`'s known value.
        let mut a = Analyser::new();
        let src = "set wtype button\nproc ::$wtype {} {return ok}\n";
        let r = a.analyse(src, "tcl8.6");
        assert!(
            r.all_procs.contains_key("::button"),
            "{:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        assert!(
            !r.all_procs.keys().any(|k| k.contains('$')),
            "{:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn handle_proc_command_unresolvable_dynamic_name_keeps_raw_text() {
        // TN — a genuinely dynamic name (no constant to fold against) keeps
        // today's existing (unchanged) fallback behaviour: this fix only
        // *improves* the resolvable case, per its own scope boundary.
        let mut a = Analyser::new();
        let src = "proc ::$wtype {} {return ok}\n";
        let r = a.analyse(src, "tcl8.6");
        assert!(
            r.all_procs.keys().any(|k| k.contains("wtype")),
            "{:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn handle_foreach_rename_reinstall_idiom_resolves_every_literal_element() {
        // TP — the finding's own corpus repro shape: `tk/library/
        // accessibility.tcl` renames each classic widget command away and
        // reinstalls a wrapper proc under the same name, once per element
        // of a literal `foreach` list. tclsh9.0/8.6 both prove `button`/
        // `entry` are the *new* wrapper procs afterwards; the old bodies
        // live only at `::tk::accessible::orig_button`/`orig_entry`.
        let mut a = Analyser::new();
        let src = "proc button {args} {return orig_button}\n\
                   proc entry {args} {return orig_entry}\n\
                   namespace eval ::tk::accessible {\n    \
                   foreach wtype {button entry} {\n        \
                   rename ::$wtype ::tk::accessible::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n";
        let r = a.analyse(src, "tcl8.6");
        // Both wrapper redefinitions are registered, at the *same* physical
        // source location (the one templated `proc ::$wtype ...`
        // statement) — genuinely correct: there is only one place in the
        // source that defines either of them.
        let button_def = r.all_procs.get("::button").expect("::button registered");
        let entry_def = r.all_procs.get("::entry").expect("::entry registered");
        assert_eq!(button_def.body_span, entry_def.body_span);
        // No garbled `${wtype}`-named entry left behind.
        assert!(
            !r.all_procs.keys().any(|k| k.contains('$')),
            "{:?}",
            r.all_procs.keys().collect::<Vec<_>>()
        );
        // The old, pre-rename bodies are reachable only via their new
        // `orig_*` names.
        assert_eq!(
            r.renamed_commands.get("::tk::accessible::orig_button"),
            Some(&"::button".to_string())
        );
        assert_eq!(
            r.renamed_commands.get("::tk::accessible::orig_entry"),
            Some(&"::entry".to_string())
        );
    }

    #[test]
    fn handle_foreach_rename_reinstall_idiom_is_not_overfit_to_two_elements() {
        // FP guard — a third element proves the per-iteration simulation
        // isn't hardcoded to exactly the corpus's own two widget names.
        let mut a = Analyser::new();
        let src = "namespace eval ::ns {\n    \
                   foreach wtype {button entry checkbutton} {\n        \
                   rename ::$wtype ::ns::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n";
        let r = a.analyse(src, "tcl8.6");
        for name in ["::button", "::entry", "::checkbutton"] {
            assert!(
                r.all_procs.contains_key(name),
                "{name} missing: {:?}",
                r.all_procs.keys().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn handle_foreach_multi_pair_form_does_not_trigger_the_literal_simulation() {
        // TN — the idx 70 multi-list lock-step form (`foreach v1 l1 v2 l2
        // body`) is a different arg shape; the single-pair-only literal
        // simulation must not misfire on it.
        let mut a = Analyser::new();
        let src = "foreach a {1 2} b {3 4} {\n    rename ::$a ::orig_$a\n}\n";
        let r = a.analyse(src, "tcl8.6");
        assert!(r.renamed_commands.is_empty(), "{:?}", r.renamed_commands);
    }

    #[test]
    fn handle_foreach_dynamic_list_does_not_trigger_the_literal_simulation() {
        // TN — a non-literal list (a variable, not a literal element list)
        // can't be simulated; must abstain exactly as before this fix.
        let mut a = Analyser::new();
        let src = "set items {button entry}\nforeach wtype $items {\n    rename ::$wtype ::orig_$wtype\n}\n";
        let r = a.analyse(src, "tcl8.6");
        assert!(r.renamed_commands.is_empty(), "{:?}", r.renamed_commands);
    }

    #[test]
    fn handle_foreach_non_idiom_body_is_unaffected() {
        // TN — an ordinary literal-list foreach with no rename/proc inside
        // must see no new side effects from the idiom-recognition code.
        let mut a = Analyser::new();
        let src = "foreach x {a b c} {\n    puts $x\n}\n";
        let r = a.analyse(src, "tcl8.6");
        assert!(r.renamed_commands.is_empty());
        assert!(!r.all_procs.keys().any(|k| k.contains('$')));
    }

    // handle_for_command

    #[test]
    fn handle_for_returns_true_for_canonical_shape() {
        let mut a = Analyser::new();
        let handled = a.handle_for_command(
            &[
                "set i 0".to_string(),
                "$i < 10".to_string(),
                "incr i".to_string(),
                "puts $i".to_string(),
            ],
            &[],
            &[],
        );
        assert!(handled);
    }

    #[test]
    fn handle_for_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled =
            a.handle_for_command(&["set i 0".to_string(), "$i < 10".to_string()], &[], &[]);
        assert!(!handled);
    }

    // handle_switch_command

    fn switch_analyser() -> Analyser {
        let mut analyser = Analyser::new();
        analyser.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        analyser
    }

    #[test]
    fn handle_switch_returns_true_for_canonical_shape() {
        let mut a = switch_analyser();
        let handled = a.handle_switch_command(
            "switch",
            &["$x".to_string(), "{a {puts a} b {puts b}}".to_string()],
            &[esc_tok(span(7, 9)), str_tok(span(10, 36))],
            &[],
        );
        assert!(handled);
    }

    #[test]
    fn handle_switch_too_few_args_returns_false() {
        let mut a = switch_analyser();
        let handled = a.handle_switch_command("switch", &["$x".to_string()], &[], &[]);
        assert!(!handled);
    }

    #[test]
    fn handle_switch_form1_walks_each_arm_body() {
        // Form 1: ``switch $x a {set y 1} b {set z 2}``.
        // Each arm body should land its ``set`` in the
        // surrounding scope.  Source layout has the bodies at
        // offsets 13..23 (``{set y 1}``) and 27..37 (``{set z 2}``).
        let mut a = switch_analyser();
        a.handle_switch_command(
            "switch",
            &[
                "$x".to_string(),
                "a".to_string(),
                "set y 1".to_string(),
                "b".to_string(),
                "set z 2".to_string(),
            ],
            &[
                esc_tok(span(7, 9)),
                esc_tok(span(10, 11)),
                str_tok(span(13, 22)),
                esc_tok(span(24, 25)),
                str_tok(span(27, 36)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
        assert!(a.result.global_scope.variables.contains_key("z"));
    }

    #[test]
    fn handle_switch_form2_braced_body_walks_each_arm() {
        // Form 2: ``switch $x { a {set y 1} b {set z 2} }``.
        // The single braced body holds all pattern/body pairs;
        // ``flatten_clause_list_elements`` re-segments to surface
        // each pair, then each body recurses.
        let mut a = switch_analyser();
        let body_text = " a {set y 1} b {set z 2} ".to_string();
        // body span: outer source positions 10..(10 + len(body)+2).
        // body_text has 25 chars, plus surrounding braces → token
        // span 10..37, content_offset = 1 to skip the opening ``{``.
        a.handle_switch_command(
            "switch",
            &["$x".to_string(), body_text],
            &[esc_tok(span(7, 9)), str_tok(span(10, 37))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
        assert!(a.result.global_scope.variables.contains_key("z"));
    }

    #[test]
    fn handle_switch_form1_skips_fallthrough_marker() {
        // ``switch $x a - b {set y 1}`` — the ``-`` body for
        // pattern ``a`` is fall-through (next arm fires); only
        // ``b``'s body should be walked.
        let mut a = switch_analyser();
        a.handle_switch_command(
            "switch",
            &[
                "$x".to_string(),
                "a".to_string(),
                "-".to_string(),
                "b".to_string(),
                "set y 1".to_string(),
            ],
            &[
                esc_tok(span(7, 9)),
                esc_tok(span(10, 11)),
                esc_tok(span(12, 13)),
                esc_tok(span(14, 15)),
                str_tok(span(17, 26)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn handle_switch_recognises_dashdash_options_terminator() {
        // ``switch -- $x a {set y 1}`` — ``--`` ends the option
        // section; the string arg follows.  Walker still finds
        // the arm body and lands ``y``.
        let mut a = switch_analyser();
        a.handle_switch_command(
            "switch",
            &[
                "--".to_string(),
                "$x".to_string(),
                "a".to_string(),
                "set y 1".to_string(),
            ],
            &[
                esc_tok(span(7, 9)),
                esc_tok(span(10, 12)),
                esc_tok(span(13, 14)),
                str_tok(span(16, 25)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn handle_switch_dynamic_form2_body_skips_walk() {
        // Form 2 with a dynamic body (``$body`` instead of a
        // braced literal) yields no elements; the walk no-ops.
        let mut a = switch_analyser();
        let var_tok = Token::new(TokenType::Var, span(10, 15));
        a.handle_switch_command(
            "switch",
            &["$x".to_string(), "$body".to_string()],
            &[esc_tok(span(7, 9)), var_tok],
            &[],
        );
        // No body walked → no vars defined.
        assert!(a.result.global_scope.variables.is_empty());
    }

    // handle_catch_command

    #[test]
    fn handle_catch_canonical_returns_true() {
        let mut a = Analyser::new();
        let handled = a.handle_catch_command(&["body".to_string()], &[esc_tok(span(0, 4))], &[]);
        assert!(handled);
    }

    #[test]
    fn handle_catch_with_result_var_defines_it() {
        let mut a = Analyser::new();
        // The binding positions come from the registry's VarWrite roles.
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.handle_catch_command(
            &["body".to_string(), "res".to_string()],
            &[esc_tok(span(0, 4)), esc_tok(span(5, 8))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("res"));
    }

    #[test]
    fn handle_catch_with_options_var_defines_both() {
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        a.handle_catch_command(
            &["body".to_string(), "res".to_string(), "opts".to_string()],
            &[
                esc_tok(span(0, 4)),
                esc_tok(span(5, 8)),
                esc_tok(span(9, 13)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("res"));
        assert!(a.result.global_scope.variables.contains_key("opts"));
    }

    #[test]
    fn handle_catch_no_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_catch_command(&[], &[], &[]);
        assert!(!handled);
    }

    // handle_try_command

    /// The traits `AnalyserHookId::Try` dispatch threads into
    /// `handle_try_command` in production, resolved through that same path so
    /// these unit tests cannot drift from it.
    fn try_traits(a: &Analyser, args: &[String]) -> tcl_registry::Traits {
        a.resolved_analyser_hook_traits("try", args)
            .expect("`try` resolves the Try analyser hook")
    }

    #[test]
    fn handle_try_canonical_returns_true() {
        let mut a = Analyser::new();
        let args = ["body".to_string()];
        let traits = try_traits(&a, &args);
        let handled = a.handle_try_command(&args, &[str_tok(span(0, 4))], &[], traits);
        assert!(handled);
    }

    #[test]
    fn handle_try_no_args_returns_false() {
        let mut a = Analyser::new();
        let traits = try_traits(&a, &[]);
        let handled = a.handle_try_command(&[], &[], &[], traits);
        assert!(!handled);
    }

    /// The dispatch really does resolve `BRANCH_SELECTED_BODY` for a `try`
    /// call, so the depth bump keys off a fact and not off a default.
    #[test]
    fn try_dispatch_resolves_the_branch_selected_body_trait() {
        let a = Analyser::new();
        let args = ["body".to_string()];
        assert!(
            try_traits(&a, &args).contains(tcl_registry::Traits::BRANCH_SELECTED_BODY),
            "hook dispatch must hand the handler `try`'s own traits"
        );
    }

    #[test]
    fn handle_try_walks_main_body() {
        // ``try {set y 1}`` — main body walks and lands ``y``.
        let mut a = Analyser::new();
        let args = ["set y 1".to_string()];
        let traits = try_traits(&a, &args);
        a.handle_try_command(&args, &[str_tok(span(5, 14))], &[], traits);
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn handle_try_walks_finally_body() {
        // ``try {} finally {set z 1}`` — finally clause body walks.
        let mut a = Analyser::new();
        let args = [String::new(), "finally".to_string(), "set z 1".to_string()];
        let traits = try_traits(&a, &args);
        a.handle_try_command(
            &args,
            &[
                str_tok(span(5, 7)),
                esc_tok(span(8, 15)),
                str_tok(span(16, 25)),
            ],
            &[],
            traits,
        );
        assert!(a.result.global_scope.variables.contains_key("z"));
    }

    #[test]
    fn handle_try_walks_on_handler_body() {
        // ``try {} on error {result options} {set q 1}`` — the
        // handler body at offset i+3 walks; the varList at i+2
        // is *not* defined as a local.
        let mut a = Analyser::new();
        let args = [
            String::new(),
            "on".to_string(),
            "error".to_string(),
            "result options".to_string(),
            "set q 1".to_string(),
        ];
        let traits = try_traits(&a, &args);
        a.handle_try_command(
            &args,
            &[
                str_tok(span(5, 7)),
                esc_tok(span(8, 10)),
                esc_tok(span(11, 16)),
                str_tok(span(17, 33)),
                str_tok(span(34, 43)),
            ],
            &[],
            traits,
        );
        assert!(a.result.global_scope.variables.contains_key("q"));
        // The `on error {result options}` var-list binds the result message +
        // options dict in the handler body — both are defined (so completion
        // offers `$result` / `$options`).
        assert!(a.result.global_scope.variables.contains_key("result"));
        assert!(a.result.global_scope.variables.contains_key("options"));
    }

    #[test]
    fn handle_try_walks_trap_handler_body() {
        // ``try {} trap NONE {result} {set q 1}`` — same shape
        // as ``on``, but the keyword is ``trap``.
        let mut a = Analyser::new();
        let args = [
            String::new(),
            "trap".to_string(),
            "NONE".to_string(),
            "result".to_string(),
            "set q 1".to_string(),
        ];
        let traits = try_traits(&a, &args);
        a.handle_try_command(
            &args,
            &[
                str_tok(span(5, 7)),
                esc_tok(span(8, 12)),
                esc_tok(span(13, 17)),
                str_tok(span(18, 26)),
                str_tok(span(27, 36)),
            ],
            &[],
            traits,
        );
        assert!(a.result.global_scope.variables.contains_key("q"));
    }

    // resolve_proc_call

    #[test]
    fn resolve_proc_call_absolute_qualified_name() {
        // ``::foo`` resolves directly when registered.
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[],
        );
        let resolved = a.resolve_proc_call("::foo", &[]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::foo");
    }

    #[test]
    fn resolve_proc_call_bare_name_walks_namespace_chain() {
        // ``foo`` declared inside ``ns1`` is found when resolved
        // from ``ns1``'s scope.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[0],
        );
        // Resolve from inside ns1 — should find ::ns1::foo.
        let resolved = a.resolve_proc_call("foo", &[0]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::ns1::foo");
    }

    #[test]
    fn resolve_proc_call_falls_back_to_global() {
        // Bare ``foo`` declared at global is found from any scope.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[],
        );
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        // Resolve from inside ns1 — chain misses ::ns1::foo,
        // falls back to ::foo.
        let resolved = a.resolve_proc_call("foo", &[0]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::foo");
    }

    #[test]
    fn resolve_proc_call_qualified_relative_name() {
        // ``a::b`` (qualified but not absolute) prepends ``::``.
        let mut a = Analyser::new();
        a.result.all_procs.insert(
            "::a::b".to_string(),
            super::ProcDef {
                name: "b".to_string(),
                qualified_name: "::a::b".to_string(),
                params: Vec::new(),
                params_computed: false,
                name_span: span(0, 0),
                body_span: span(0, 0),
                doc: String::new(),
                param_traits: std::collections::HashMap::new(),
                caller_frame_params: std::collections::HashSet::new(),
                caller_frame_literals: std::collections::HashMap::new(),
            },
        );
        let resolved = a.resolve_proc_call("a::b", &[]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::a::b");
    }

    #[test]
    fn resolve_proc_call_relative_dotted_name_prefers_current_namespace() {
        // A relative dotted word (`ns2::inner`, containing `::` but not
        // starting with it) must resolve against the current namespace
        // first, not be rooted straight at global (confirmed against tclsh
        // 9.0.4 — see `bareword_resolution_candidates`).
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        for qname in ["::ns2::inner", "::ns::ns2::inner"] {
            a.result.all_procs.insert(
                qname.to_string(),
                super::ProcDef {
                    name: "inner".to_string(),
                    qualified_name: qname.to_string(),
                    params: Vec::new(),
                    params_computed: false,
                    name_span: span(0, 0),
                    body_span: span(0, 0),
                    doc: String::new(),
                    param_traits: std::collections::HashMap::new(),
                    caller_frame_params: std::collections::HashSet::new(),
                    caller_frame_literals: std::collections::HashMap::new(),
                },
            );
        }
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns"));
        let resolved = a.resolve_proc_call("ns2::inner", &[0]);
        assert_eq!(
            resolved.unwrap().qualified_name,
            "::ns::ns2::inner",
            "must prefer the current-namespace proc over the root one",
        );
        // Falls back to the root proc when there is no current-namespace
        // candidate.
        let resolved = a.resolve_proc_call("ns2::inner", &[]);
        assert_eq!(resolved.unwrap().qualified_name, "::ns2::inner");
    }

    #[test]
    fn resolve_proc_call_does_not_walk_ancestor_namespaces() {
        // Real Tcl bareword resolution is exactly two levels — current
        // namespace, then global — absent an explicit `namespace path`. A
        // proc defined in a *grandparent* namespace must not resolve for a
        // bare call from a nested `::a::b::c` scope.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        let mut ns_a = Scope::new(ScopeKind::Namespace, "a");
        let mut ns_b = Scope::new(ScopeKind::Namespace, "b");
        ns_b.children.push(Scope::new(ScopeKind::Namespace, "c"));
        ns_a.children.push(ns_b);
        a.result.global_scope.children.push(ns_a);
        // scope_path [0] = ::a — define `foo` there.
        a.handle_proc_command(
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
            &[0],
        );
        // Resolving from ::a::b::c (scope_path [0, 0, 0]) must NOT reach
        // the grandparent's ::a::foo.
        assert!(
            a.resolve_proc_call("foo", &[0, 0, 0]).is_none(),
            "a grandparent-namespace proc must not resolve for a bare call",
        );
        // Control: resolving directly from ::a still finds it.
        assert!(a.resolve_proc_call("foo", &[0]).is_some());
    }

    #[test]
    fn resolve_proc_call_unknown_name_returns_none() {
        let a = Analyser::new();
        assert!(a.resolve_proc_call("nope", &[]).is_none());
    }

    #[test]
    fn resolve_proc_call_empty_name_returns_none() {
        let a = Analyser::new();
        assert!(a.resolve_proc_call("", &[]).is_none());
    }

    // resolve_expansion_count

    #[test]
    fn resolve_expansion_count_braced_literal() {
        // ``{a b c}`` — Str token; inner content "a b c" splits
        // to three elements.
        let mut a = Analyser::new();
        a.source = "{a b c}".to_string();
        // Span covers ``{a b c`` (5 inner chars + opening brace),
        // content_offset = 1 to skip ``{``.  Closing ``}`` is
        // OUTSIDE the span by lexer convention for non-degenerate
        // STR tokens.
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 6),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), Some(3));
    }

    #[test]
    fn resolve_expansion_count_braced_empty_list() {
        // ``{}`` — degenerate Str case; span extended to include
        // ``}``, token_text returns empty string.
        let mut a = Analyser::new();
        a.source = "{}".to_string();
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 2),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), Some(0));
    }

    #[test]
    fn resolve_expansion_count_var_with_const_value() {
        // ``$xs`` where xs has known constant ``a b c`` — splits
        // to three elements.
        let mut a = Analyser::new();
        a.source = "$xs".to_string();
        a.set_const_string("xs", "a b c".to_string(), span(0, 5), &[]);
        // Var token: span covers ``xs`` (after `$`) by lexer
        // convention; content_offset = 0 because the lexer's
        // ``_start`` for VAR is set after the ``$``.
        // For testing, place the var name at offset 1..3 in source.
        let tok = Token {
            kind: TokenType::Var,
            span: span(1, 3),
            content_offset: 0,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), Some(3));
    }

    #[test]
    fn resolve_expansion_count_var_without_const_value() {
        // Var with no known constant value → None.
        let mut a = Analyser::new();
        a.source = "$xs".to_string();
        let tok = Token {
            kind: TokenType::Var,
            span: span(1, 3),
            content_offset: 0,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), None);
    }

    #[test]
    fn resolve_expansion_count_concatenated_word_returns_none() {
        // ``single_token = false`` short-circuits to None.
        let mut a = Analyser::new();
        a.source = "{a b c}".to_string();
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 6),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, false, &[]), None);
    }

    #[test]
    fn resolve_expansion_count_other_token_kind_returns_none() {
        // Non-Str, non-Var token kinds aren't statically
        // resolvable.
        let a = Analyser::new();
        let tok = esc_tok(span(0, 4));
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), None);
    }

    // handle_interp_alias

    #[test]
    fn handle_interp_alias_records_canonical_form() {
        let mut a = Analyser::new();
        a.handle_interp_alias(
            &[
                "alias".to_string(),
                String::new(),
                "myset".to_string(),
                String::new(),
                "set".to_string(),
            ],
            &[],
            42,
        );
        assert!(a.command_aliases.contains_key("::myset"));
        assert!(a.result.command_aliases.contains_key("::myset"));
        let (target, prepended) = &a.command_aliases["::myset"];
        assert_eq!(target, "set");
        assert!(prepended.is_empty());
        assert_eq!(a.alias_offsets.get("::myset"), Some(&42));
        // The offset is also promoted onto the finalised `AnalysisResult`
        // (not just the in-progress `Analyser`), so a cross-document
        // consumer such as `tcl_lsp_core::WorkspaceIndex` can tell an
        // unconditional alias apart from one nested in a proc/class body.
        assert_eq!(a.result.alias_offsets.get("::myset"), Some(&42));
    }

    #[test]
    fn handle_interp_alias_with_prepended_args() {
        let mut a = Analyser::new();
        a.handle_interp_alias(
            &[
                "alias".to_string(),
                String::new(),
                "logerr".to_string(),
                String::new(),
                "puts".to_string(),
                "stderr".to_string(),
            ],
            &[],
            0,
        );
        let (target, prepended) = &a.command_aliases["::logerr"];
        assert_eq!(target, "puts");
        assert_eq!(prepended, &vec!["stderr".to_string()]);
    }

    #[test]
    fn handle_interp_alias_wrong_shape_no_op() {
        let mut a = Analyser::new();
        a.handle_interp_alias(&["alias".to_string()], &[], 0);
        assert!(a.command_aliases.is_empty());
        assert!(a.alias_offsets.is_empty());
    }

    // resolve_dynamic_word

    #[test]
    fn resolve_dynamic_word_braced_literal_dollar_is_not_mistaken_for_dynamic() {
        // A brace-quoted word containing a literal `$`/`[` character is
        // legitimately STATIC — Tcl suppresses substitution inside
        // `{}` — so this must return it as-is via `resolve_const_word`'s
        // `Str`/`Esc` branch, even though `is_dynamic_word`'s naive text
        // scan alone would call it dynamic (issue #923 idx 3).
        let a = Analyser::new();
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 10),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(
            a.resolve_dynamic_word("::pkg::$c", Some(tok), true, &[]),
            Some("::pkg::$c".to_string())
        );
    }

    #[test]
    fn resolve_dynamic_word_missing_token_is_none() {
        // A dynamic word with no token to inspect (the handful of
        // existing handle_rename unit tests pass `arg_tokens: &[]`)
        // must fall through to None, not panic.
        let a = Analyser::new();
        assert_eq!(a.resolve_dynamic_word("$x", None, false, &[]), None);
    }

    // handle_rename

    #[test]
    fn handle_rename_records_static_move() {
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(
            &["target".to_string(), "target_orig".to_string()],
            &[],
            &[],
            &[],
            42,
        );
        assert!(!dynamic, "a fully static rename is not dynamic");
        assert_eq!(
            a.renamed_commands.get("::target_orig").map(String::as_str),
            Some("::target")
        );
        assert_eq!(
            a.result
                .renamed_commands
                .get("::target_orig")
                .map(String::as_str),
            Some("::target")
        );
        assert_eq!(a.rename_offsets.get("::target_orig"), Some(&42));
        // The old name is recorded as deleted from this offset —
        // confirmed against tclsh 9.0.4: `target` becomes an unknown
        // command, not still callable under its original arity.
        assert_eq!(a.deleted_commands.get("::target"), Some(&42));
    }

    #[test]
    fn handle_rename_deletion_records_old_name_as_deleted() {
        // `rename OLD {}` deletes OLD — no new binding to record (it is
        // not a "dynamic" rename either, nothing to widen for), but OLD
        // itself must be recorded as gone (confirmed against tclsh
        // 9.0.4: also "invalid command name" afterwards).
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(&["target".to_string(), String::new()], &[], &[], &[], 7);
        assert!(!dynamic);
        assert!(a.renamed_commands.is_empty());
        assert_eq!(a.deleted_commands.get("::target"), Some(&7));
    }

    #[test]
    fn handle_rename_dynamic_old_name_reports_dynamic() {
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(&["$x".to_string(), "y".to_string()], &[], &[], &[], 0);
        assert!(dynamic, "rename $x y cannot be resolved statically");
        assert!(a.renamed_commands.is_empty());
        assert!(a.deleted_commands.is_empty());
    }

    #[test]
    fn handle_rename_dynamic_new_name_reports_dynamic() {
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(&["x".to_string(), "y[z]".to_string()], &[], &[], &[], 0);
        assert!(dynamic, "rename x y[z] cannot be resolved statically");
        assert!(a.renamed_commands.is_empty());
        assert!(a.deleted_commands.is_empty());
    }

    #[test]
    fn handle_rename_wrong_shape_no_op() {
        let mut a = Analyser::new();
        let dynamic = a.handle_rename(&["onlyone".to_string()], &[], &[], &[], 0);
        assert!(!dynamic);
        assert!(a.renamed_commands.is_empty());
        assert!(a.deleted_commands.is_empty());
    }

    // handle_source_command

    #[test]
    fn handle_source_command_resolves_a_same_file_constant_variable() {
        // The audit's own "reduced to the simplest possible case" control
        // (issue #923 idx 46): a straight-line `set`, zero branches, zero
        // external input, immediately followed by `source $var` — the real
        // corpus's `set p "e.tcl"; source $p` shape, resolved through the
        // same constant-string lattice already proven for `rename`'s
        // OLD/NEW words (idx 3), via the now-shared `resolve_dynamic_word`.
        let mut a = Analyser::new();
        let r = a.analyse("namespace eval ::z { set p \"e.tcl\"; source $p }\n", "tcl");
        let target = r
            .source_targets
            .iter()
            .find(|s| s.raw_path == "e.tcl")
            .unwrap_or_else(|| panic!("resolved literal not found: {:?}", r.source_targets));
        assert!(target.is_literal, "{target:?}");
    }

    #[test]
    fn handle_source_command_resolves_a_concatenated_constant_variable() {
        // A multi-token word (`${base}.tcl`, a Var fragment concatenated
        // with a literal) goes through `fold_interpolation_single` rather
        // than the single-token `resolve_const_word` fast path — covered
        // separately since it's a different code path inside
        // `resolve_dynamic_word`.
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace eval ::z { set base \"helper\"; source ${base}.tcl }\n",
            "tcl",
        );
        let target = r
            .source_targets
            .iter()
            .find(|s| s.raw_path == "helper.tcl")
            .unwrap_or_else(|| panic!("resolved literal not found: {:?}", r.source_targets));
        assert!(target.is_literal, "{target:?}");
    }

    #[test]
    fn handle_source_command_a_dynamic_variable_with_no_known_value_stays_dynamic() {
        // A proc parameter is never constant-tracked (the same limitation
        // `resolve_dynamic_word` already documents for `rename`, issue
        // #923 idx 3) — `source $p` here must stay conservatively dynamic
        // rather than guess.
        let mut a = Analyser::new();
        let r = a.analyse("proc f {p} { source $p }\n", "tcl");
        assert_eq!(r.source_targets.len(), 1, "{:?}", r.source_targets);
        assert!(!r.source_targets[0].is_literal, "{:?}", r.source_targets[0]);
    }

    #[test]
    fn handle_source_command_leaves_a_bracket_wrapped_variable_dynamic() {
        // Deliberately out of scope (issue #923 idx 46): a same-file
        // constant wrapped inside a `[file join ...]` command
        // substitution — the real corpus's `source [file join $edir
        // extra.tcl]` shape — can't be folded by the same-word constant
        // lattice (`fold_interpolation_single` rejects any word containing
        // `[` outright, by design — a command substitution can have
        // arbitrary side effects), and `evaluate_auto_path_expr`'s
        // separate `[info script]`-anchored folder doesn't evaluate `$var`
        // at all. Pinned here so a future fix that closes this gap updates
        // this test deliberately rather than by surprise.
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace eval ::z { set dir \"lib\"; source [file join $dir extra.tcl] }\n",
            "tcl",
        );
        assert_eq!(r.source_targets.len(), 1, "{:?}", r.source_targets);
        assert!(!r.source_targets[0].is_literal, "{:?}", r.source_targets[0]);
    }

    // The `INSTALLS_NAMED_DEFINITION` re-dispatch contract.

    #[test]
    fn every_installer_spec_lands_on_a_live_redispatch_arm() {
        // Drift gate — the trait is a *promise* that
        // `simulate_remaining_foreach_iterations` re-runs the command once
        // per literal `foreach` element.  A spec that carries it but whose
        // analyser hook has no arm in that match silently promises nothing
        // (Codex review of PR #1074: `oo::objdefine` was exactly that).
        // Registry-driven, so a newly-stamped spec fails here rather than
        // in a corpus months later.
        use tcl_registry::hooks::AnalyserHookId as Hook;
        let redispatched = [Hook::Proc, Hook::Rename, Hook::OoDefine, Hook::OoObjdefine];
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut checked = 0usize;
        for name in registry.command_names() {
            for spec in registry.specs(name) {
                if !spec
                    .traits
                    .contains(tcl_registry::Traits::INSTALLS_NAMED_DEFINITION)
                {
                    continue;
                }
                checked += 1;
                let hook = spec.analyser_hook.unwrap_or_else(|| {
                    panic!("{name} carries INSTALLS_NAMED_DEFINITION but has no analyser hook")
                });
                assert!(
                    redispatched.contains(&hook),
                    "{name} carries INSTALLS_NAMED_DEFINITION but its hook {hook:?} has no \
                     arm in the foreach re-dispatch — the trait promises a re-dispatch \
                     the match does not deliver",
                );
            }
        }
        assert!(
            checked >= 4,
            "expected proc/rename/oo::define/oo::objdefine, saw {checked}"
        );
    }

    #[test]
    fn foreach_objdefine_records_per_object_facts_for_every_literal_element() {
        // TP — issue #923 idx 55's sibling shape.  tclsh 9.0.4 and 8.6.16
        // both run `foreach o {::a ::b} { oo::objdefine $o { method probe
        // {} {…} } }` and report `probe` in `info object methods` for
        // *both* objects, so both must carry the per-object method fact.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             Base create ::b\n\
             foreach o {::a ::b} {\n    oo::objdefine $o {\n        method probe {} { return probed }\n    }\n}\n",
            "tcl9.0",
        );
        for obj in ["::a", "::b"] {
            let methods = r
                .object_methods
                .get(obj)
                .unwrap_or_else(|| panic!("{obj} has per-object methods: {:?}", r.object_methods));
            assert!(
                methods.iter().any(|m| m.def.name == "probe"),
                "{obj} gained probe: {methods:?}",
            );
        }
    }

    #[test]
    fn foreach_objdefine_does_not_double_record_the_first_element() {
        // FP guard — the ordinary body walk covers the first element and
        // the simulation covers the rest, so the first element's site is
        // visited twice under the loop variable's own key.  One source site
        // can never declare the same member twice, so it must appear once.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             Base create ::b\n\
             foreach o {::a ::b} {\n    oo::objdefine $o {\n        method probe {} { return probed }\n    }\n}\n",
            "tcl9.0",
        );
        for (key, methods) in &r.object_methods {
            let probes = methods.iter().filter(|m| m.def.name == "probe").count();
            assert!(
                probes <= 1,
                "{key} recorded probe {probes} times: {methods:?}",
            );
        }
    }

    #[test]
    fn foreach_objdefine_does_not_duplicate_body_diagnostics() {
        // FP guard — re-running the installer per element re-walks the
        // per-object method bodies, so a diagnostic raised inside one must
        // still be reported once per site, never once per iteration.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             Base create ::b\n\
             foreach o {::a ::b} {\n    oo::objdefine $o {\n        method probe {} { return $undefinedVar }\n    }\n}\n",
            "tcl9.0",
        );
        let mut seen = std::collections::HashSet::new();
        for diag in &r.diagnostics {
            assert!(
                seen.insert((diag.code.to_string(), diag.span)),
                "duplicate diagnostic {:?} at {:?}",
                diag.code,
                diag.span,
            );
        }
    }

    #[test]
    fn foreach_objdefine_over_a_dynamic_list_abstains() {
        // TN — a runtime element list names no object the walk can know, so
        // the per-object facts stay under the loop variable's own key and
        // are never attributed to a literal object.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Base { method hello {} { return hi } }\n\
             Base create ::a\n\
             foreach o $objects {\n    oo::objdefine $o {\n        method probe {} { return probed }\n    }\n}\n",
            "tcl9.0",
        );
        assert!(
            !r.object_methods.contains_key("::a"),
            "a runtime receiver must not be attributed to a literal object: {:?}",
            r.object_methods,
        );
        assert!(
            r.object_methods.contains_key("o"),
            "the facts stay under the receiver variable: {:?}",
            r.object_methods,
        );
    }

    // handle_oo_objdefine

    #[test]
    fn handle_oo_objdefine_records_dollar_var() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine(&["$obj".to_string()], &[], &[], &[]);
        assert!(a.objdefined_vars.contains("obj"));
    }

    #[test]
    fn handle_oo_objdefine_records_braced_dollar_var() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine(&["${obj}".to_string()], &[], &[], &[]);
        assert!(a.objdefined_vars.contains("obj"));
    }

    #[test]
    fn handle_oo_objdefine_records_bare_name() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine(&["obj".to_string()], &[], &[], &[]);
        assert!(a.objdefined_vars.contains("obj"));
    }

    #[test]
    fn oo_objdefine_body_methods_are_analysed() {
        // Before: the `oo::objdefine` body was never parsed, so nothing inside
        // a per-object method resolved.  Now the body walks like any method
        // body — the call it makes is recorded as an invocation.
        let mut a = Analyser::new();
        let src = "proc helper {} {}\n\
                   oo::class create Foo {}\n\
                   set o [Foo new]\n\
                   oo::objdefine $o {\n    \
                       method greet {} { helper }\n\
                   }\n";
        let r = a.analyse(src, "tcl8.6");
        assert!(
            r.command_invocations.iter().any(|i| i.name == "helper"),
            "the call inside the per-object method body should be analysed: {:?}",
            r.command_invocations
                .iter()
                .map(|i| &i.name)
                .collect::<Vec<_>>(),
        );
    }

    // resolve_alias

    #[test]
    fn resolve_alias_passthrough_for_non_alias() {
        let mut a = Analyser::new();
        let (target, args) = a.resolve_alias("puts", &["hello".to_string()], &[]);
        assert_eq!(target, "puts");
        assert_eq!(args, vec!["hello".to_string()]);
    }

    #[test]
    fn resolve_alias_substitutes_target_and_prepended_args() {
        let mut a = Analyser::new();
        a.command_aliases.insert(
            "::logerr".to_string(),
            ("puts".to_string(), vec!["stderr".to_string()]),
        );
        let (target, args) = a.resolve_alias("logerr", &["hello".to_string()], &[]);
        assert_eq!(target, "puts");
        assert_eq!(args, vec!["stderr".to_string(), "hello".to_string()]);
    }

    // handle_oo_class_command

    #[test]
    fn handle_oo_class_create_records_class() {
        let mut a = Analyser::new();
        // Metaclass recognition is now registry-trait-driven (`IS_OO_METACLASS`).
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        let handled = a.handle_oo_class_command(
            "oo::class",
            &["create".to_string(), "MyClass".to_string()],
            &[
                esc_tok(span(0, 9)),
                esc_tok(span(10, 16)),
                esc_tok(span(17, 24)),
            ],
            &[],
            esc_tok(span(0, 9)),
        );
        assert!(handled);
        assert!(a.result.all_classes.contains_key("::MyClass"));
        let cls = &a.result.all_classes["::MyClass"];
        assert_eq!(cls.name, "MyClass");
    }

    #[test]
    fn handle_oo_class_create_with_body() {
        // arg_tokens stripped of cmd_name (matching the
        // ``process_command`` dispatch convention).
        let mut a = Analyser::new();
        a.registry = Some(tcl_registry::registry_handle_for_dialect("tcl"));
        let handled = a.handle_oo_class_command(
            "oo::class",
            &[
                "create".to_string(),
                "MyClass".to_string(),
                "method m {} {}".to_string(),
            ],
            &[
                esc_tok(span(10, 16)),
                esc_tok(span(17, 24)),
                str_tok(span(25, 41)),
            ],
            &[],
            esc_tok(span(0, 9)),
        );
        assert!(handled);
        assert_eq!(a.result.all_classes["::MyClass"].body_span, span(25, 41));
    }

    #[test]
    fn handle_oo_class_wrong_subcommand_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_class_command(
            "oo::class",
            &["destroy".to_string(), "MyClass".to_string()],
            &[
                esc_tok(span(0, 9)),
                esc_tok(span(10, 17)),
                esc_tok(span(18, 25)),
            ],
            &[],
            esc_tok(span(0, 9)),
        );
        assert!(!handled);
        assert!(a.result.all_classes.is_empty());
    }

    // handle_oo_class_command body walking

    #[test]
    fn analyse_oo_class_body_records_superclass_and_methods() {
        // End-to-end: ``oo::class create Sub`` with a body
        // declaring a superclass and a method.  After analyse
        // ``::Sub`` should carry both fields.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create Sub { superclass ::Base\nmethod greet {} { puts hi } }",
            "tcl",
        );
        assert!(r.all_classes.contains_key("::Sub"));
        let cls = &r.all_classes["::Sub"];
        assert_eq!(cls.superclasses, vec!["::Base"]);
        assert!(cls.methods.contains_key("greet"));
        assert_eq!(cls.methods["greet"].kind, "method");
    }

    #[test]
    fn analyse_oo_class_body_records_classmethod_and_mixin() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { mixin ::M\nclassmethod build {} { return ok } }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.mixins, vec!["::M"]);
        assert!(cls.class_methods.contains_key("build"));
        assert!(!cls.methods.contains_key("build"));
    }

    // handle_oo_define_command body walking

    #[test]
    fn analyse_oo_define_body_extends_existing_class() {
        // ``oo::class create C {}`` followed by ``oo::define
        // C { method m {} {} }`` — the method ends up in the
        // already-recorded class.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C {}\noo::define C { method m {} {} }",
            "tcl",
        );
        assert!(r.all_classes.contains_key("::C"));
        let cls = &r.all_classes["::C"];
        assert!(cls.methods.contains_key("m"));
    }

    #[test]
    fn oo_define_extending_an_existing_class_adds_its_own_class_body_span() {
        // Issue #923 idx 52: `class_body_spans` must record BOTH the
        // creation site's own span AND the separate `oo::define` block's
        // own span for the same qualified class — a class extended via a
        // *separate* `oo::define ClassName { ... }` block has textually
        // disjoint body spans, not one contiguous range, and `my`-dispatch
        // resolution (a lexical "which class's body am I inside" query)
        // needs every contributing span, not just the first one recorded.
        let mut a = crate::analyser::Analyser::new();
        let src = "oo::class create C {}\noo::define C { method m {} {} }";
        let r = a.analyse(src, "tcl");
        let spans: Vec<tcl_lexer::Span> = r
            .class_body_spans
            .iter()
            .filter(|(name, _)| name == "::C")
            .map(|(_, span)| *span)
            .collect();
        assert_eq!(spans.len(), 2, "{spans:?}");
        // One of the two recorded spans must cover the `oo::define`
        // block's own `method m {} {}` text — the token whose containment
        // `enclosing_class_at` checks for a cursor inside it.
        let inside_define_block = u32::try_from(src.find("method m").unwrap()).unwrap();
        assert!(
            spans
                .iter()
                .any(|span| span.start() <= inside_define_block
                    && inside_define_block < span.end()),
            "no recorded span covers the oo::define block's own body: {spans:?}"
        );
    }

    #[test]
    fn oo_define_extends_class_named_like_a_create_subcommand() {
        // Regression: `oo::define` / `oo::objdefine` carry a TclOO
        // `definition_body` but no `IS_OO_METACLASS` trait, so a class literally
        // named `create` must be *extended* by `handle_oo_define_command`, not
        // stolen by `handle_oo_class_command` (which would record a bogus class
        // named `method`).  The class-creator check requires the metaclass
        // trait, which define/objdefine lack.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("oo::define create method greet {} { return hi }", "tcl");
        // The real class is `create`, with method `greet` — not a bogus class.
        let cls = r
            .all_classes
            .get("::create")
            .expect("class `create` recorded by oo::define");
        assert!(
            cls.methods.contains_key("greet"),
            "method greet must be recorded on class `create`: {:?}",
            cls.methods.keys().collect::<Vec<_>>()
        );
        assert!(
            !r.all_classes.contains_key("::method"),
            "no bogus class named `method` should be created",
        );
    }

    #[test]
    fn analyse_oo_define_inline_form_extends_class() {
        // ``oo::define C method m {} {}`` — inline form,
        // single subcommand.  Works whether or not the class
        // was previously declared (creates a stub if absent).
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("oo::define MyClass method greet {} { puts hi }", "tcl");
        assert!(r.all_classes.contains_key("::MyClass"));
        let cls = &r.all_classes["::MyClass"];
        assert!(cls.methods.contains_key("greet"));
    }

    #[test]
    fn two_dynamic_oo_define_targets_sharing_a_variable_name_never_merge() {
        // TP — regression for a bug found by differential audit against
        // clay.tcl's `current_class`-based Ensemble DSL: two lexically
        // unrelated procs each write `oo::define $class method ... ` with a
        // same-named local `class` (an unremarkable choice for this exact
        // idiom) and each add their own method. Before the fix both calls
        // computed the identical raw-text key `$class`, so the second
        // `remove`+`insert` round-trip silently merged the first proc's
        // method into the second's ClassDef (and vice versa via the
        // dual-registration into `scope.classes`), producing a document
        // symbol whose child method range sits outside its own parent's.
        // The targets here are *parameters*, so no constant dominates
        // either call and both stay genuinely dynamic — the shape this
        // guard is about.  (A target bound to a dominating constant is a
        // different, statically-resolvable case: see
        // `oo_define_resolves_a_dominating_constant_target`.)
        let mut a = Analyser::new();
        let r = a.analyse(
            "proc addFoo {class} {\n    oo::define $class method foo {} { return foo }\n}\n\
             proc addBar {class} {\n    oo::define $class method bar {} { return bar }\n}\n",
            "tcl8.6",
        );
        let synthetic: Vec<_> = r
            .all_classes
            .iter()
            .filter(|(k, _)| k.contains("@dynclass@"))
            .collect();
        assert_eq!(
            synthetic.len(),
            2,
            "each dynamic oo::define call site gets its own ClassDef: {:?}",
            r.all_classes.keys().collect::<Vec<_>>(),
        );
        let has_foo_only = synthetic
            .iter()
            .any(|(_, c)| c.methods.contains_key("foo") && !c.methods.contains_key("bar"));
        let has_bar_only = synthetic
            .iter()
            .any(|(_, c)| c.methods.contains_key("bar") && !c.methods.contains_key("foo"));
        assert!(
            has_foo_only && has_bar_only,
            "neither call site's method may leak into the other's ClassDef: {:?}",
            synthetic
                .iter()
                .map(|(k, c)| (k, c.methods.keys().collect::<Vec<_>>()))
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn dynamic_oo_define_target_does_not_touch_a_same_named_literal_class() {
        // FP guard — a class whose *written* name matches nothing knowable
        // (`$class` bound to a parameter) must never be found/extended
        // through a literal class that happens to share the variable's
        // source text: the synthetic key can never collide with a real
        // qualified name.  A target bound to a dominating constant *is*
        // resolvable and does extend the real class — tclsh 9.0.4/8.6.16
        // agree — which is
        // `oo_define_resolves_a_dominating_constant_target`'s case, not
        // this one.
        let mut a = Analyser::new();
        let r = a.analyse(
            "oo::class create Widget {\n    method real {} {}\n}\n\
             proc addDynamic {class} {\n    oo::define $class method dynamic {} {}\n}\n",
            "tcl8.6",
        );
        let widget = &r.all_classes["::Widget"];
        assert!(widget.methods.contains_key("real"), "{widget:?}");
        assert!(
            !widget.methods.contains_key("dynamic"),
            "the dynamic call must not extend the literal same-named class: {widget:?}",
        );
    }

    // handle_oo_define_command

    #[test]
    fn handle_oo_define_recognises_canonical_form() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_define_command(
            "oo::define",
            &["MyClass".to_string(), "method m {} {}".to_string()],
            &[],
            &[],
            &[],
        );
        assert!(handled);
    }

    #[test]
    fn handle_oo_define_no_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_define_command("oo::define", &[], &[], &[], &[]);
        assert!(!handled);
    }

    // handle_incr_command

    #[test]
    fn handle_incr_defines_var() {
        let mut a = Analyser::new();
        a.handle_incr_command(&["counter".to_string()], &[esc_tok(span(0, 7))], &[]);
        assert!(a.result.global_scope.variables.contains_key("counter"));
        // incr-defined vars warn_if_unused = true (so a `set`-only
        // var pattern still fires; an `incr`-only-no-read does too).
        assert!(a.result.global_scope.variables["counter"].warn_if_unused);
    }

    #[test]
    fn handle_incr_with_amount() {
        let mut a = Analyser::new();
        a.handle_incr_command(
            &["counter".to_string(), "5".to_string()],
            &[esc_tok(span(0, 7)), esc_tok(span(8, 9))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("counter"));
    }

    #[test]
    fn handle_incr_no_args_no_op() {
        let mut a = Analyser::new();
        a.handle_incr_command(&[], &[], &[]);
        assert!(a.result.global_scope.variables.is_empty());
    }

    // ClassDef extended fields + UnknownProcInfo

    #[test]
    fn analyse_oo_class_records_metaclass_from_command_name() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("oo::class create C {}", "tcl");
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.metaclass, "oo::class");
    }

    #[test]
    fn analyse_oo_class_body_records_constructors_and_destructor() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { constructor args { puts ctor }\ndestructor { puts dtor } }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.constructors.len(), 1);
        assert_eq!(cls.constructors[0].kind, "constructor");
        assert!(cls.destructor.is_some());
        assert_eq!(cls.destructor.as_ref().unwrap().kind, "destructor");
    }

    #[test]
    fn analyse_oo_class_body_records_variables_filters_exports() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { variable x y\nfilter log\nexport foo bar\nunexport hidden }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.variables, vec!["x", "y"]);
        assert_eq!(cls.filters, vec!["log"]);
        assert!(cls.exports.contains("foo"));
        assert!(cls.exports.contains("bar"));
        assert!(cls.unexports.contains("hidden"));
    }

    #[test]
    fn analyse_oo_class_body_records_property_def() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { property colour -kind readwrite -get { return red } }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        let pd = cls.properties.get("colour").expect("colour recorded");
        assert_eq!(pd.kind, "readwrite");
        assert!(pd.has_getter);
        assert!(!pd.has_setter);
    }

    #[test]
    fn analyse_unknown_proc_records_dispatch_targets_end_to_end() {
        // End-to-end: a ``proc unknown {cmd args} {...}`` with
        // an exact-match switch should populate
        // ``result.unknown_proc_info`` with the arm labels as
        // dispatch targets.  This is what gates W123.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } bar { return 2 } } }",
            "tcl",
        );
        let info = r.unknown_proc_info.expect("unknown_proc_info populated");
        assert!(!info.empty_stub);
        assert!(info.dispatch_targets.contains("foo"));
        assert!(info.dispatch_targets.contains("bar"));
    }

    #[test]
    fn analyse_without_unknown_proc_leaves_unknown_proc_info_none() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc foo {} { return 1 }", "tcl");
        assert!(r.unknown_proc_info.is_none());
    }

    #[test]
    fn analyse_unknown_proc_with_empty_body_marks_empty_stub() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc unknown {cmd args} {}", "tcl");
        let info = r.unknown_proc_info.expect("unknown_proc_info populated");
        assert!(info.empty_stub);
    }

    /// Pin (gap-review C3) — `conditional_depth` is driven by
    /// `Traits::BRANCH_SELECTED_BODY`, so exactly the branch-selected bodies
    /// mark a `package require` conditional.
    ///
    /// `if` and `try` bodies are branch-selected: at most one runs, chosen at
    /// run time, so nothing inside dominates the code after the command. A
    /// `while` body is skippable *and* repeatable — a different question,
    /// owned by `control_flow_body_depth` — and does **not** mark the require
    /// conditional. A top-level require is unconditional.
    #[test]
    fn package_require_conditionality_follows_the_branch_selected_body_trait() {
        let conditional_flags = |src: &str| -> Vec<bool> {
            let mut a = crate::analyser::Analyser::new();
            a.analyse(src, "tcl8.6")
                .package_requires
                .iter()
                .map(|p| p.conditional)
                .collect()
        };
        assert_eq!(conditional_flags("package require Tcl 8.6\n"), vec![false]);
        assert_eq!(
            conditional_flags("if {$x} { package require Tcl 8.6 }\n"),
            vec![true],
        );
        assert_eq!(
            conditional_flags("while {$x} { package require Tcl 8.6 }\n"),
            vec![false],
            "a loop body is skippable-and-repeatable, a different question",
        );
        // `try` carries the trait too and reaches its bodies through its own
        // analyser hook; that hook now honours the same trait (issue #1065),
        // so the main body is conditional like `if`'s.
        assert_eq!(
            conditional_flags("try { package require Tcl 8.6 } on error {} {}\n"),
            vec![true],
        );
    }

    /// FIX (issue #1065) — `handle_try_command` bumps the branch-selected
    /// depth per clause kind, so a `package require` records the right
    /// conditionality wherever in a `try` it sits.
    ///
    /// Clause semantics are C Tcl's (Tcl 9.0.4 `try(n)`, `TclNRTryObjCmd` in
    /// `generic/tclCmdMZ.c`): the main body may be cut short by an exception a
    /// handler swallows and the `on`/`trap` handlers run only on a match — both
    /// branch-selected — while `finally` always runs, so it is not.  See
    /// `Analyser::analyse_selected_body` for the full reasoning.
    #[test]
    fn package_require_conditionality_per_try_clause_kind() {
        let conditional_flags = |src: &str| -> Vec<bool> {
            let mut a = crate::analyser::Analyser::new();
            a.analyse(src, "tcl8.6")
                .package_requires
                .iter()
                .map(|p| p.conditional)
                .collect()
        };
        // TP — the issue's own repro: the guarded optional-dependency idiom.
        assert_eq!(
            conditional_flags("try { package require Foo } on error {} {}\n"),
            vec![true],
            "a require in the main try body is conditional",
        );
        // TP — an `on` handler body.
        assert_eq!(
            conditional_flags("try { set x 1 } on error {} { package require Foo }\n"),
            vec![true],
            "a require in an `on` handler is conditional",
        );
        // TP — a `trap` handler body.
        assert_eq!(
            conditional_flags(
                "try { set x 1 } trap {POSIX ENOENT} {m o} { package require Foo }\n"
            ),
            vec![true],
            "a require in a `trap` handler is conditional",
        );
        // TN — a `finally` body always runs, so it is not branch-selected.
        assert_eq!(
            conditional_flags("try { set x 1 } finally { package require Foo }\n"),
            vec![false],
            "a `finally` body always runs — the require is unconditional",
        );
        // TN — a top-level require, the control.
        assert_eq!(conditional_flags("package require Foo\n"), vec![false]);
        // FN guard — the depth is restored after the walk, so a require that
        // follows the whole `try` command is still unconditional.
        assert_eq!(
            conditional_flags(
                "try { package require A } on error {} { package require B } finally { package require C }\npackage require D\n"
            ),
            vec![true, true, false, false],
            "main body + handler conditional; finally and the following \
             top-level require unconditional",
        );
        // FP guard — nesting a `try` inside an `if` must not leave the depth
        // stuck: the require after both is still unconditional.
        assert_eq!(
            conditional_flags(
                "if {$x} { try { package require A } finally { set y 1 } }\npackage require B\n"
            ),
            vec![true, false],
        );
    }

    /// FIX (gap-review C2) — a `proc unknown` nested inside `namespace eval`
    /// is an ordinary namespace proc, not the interpreter's handler, so it
    /// must not seed `unknown_proc_info` and suppress W123 file-wide.
    ///
    /// The body is the dynamic (`exec`-dispatching) shape that *would*
    /// suppress W123 file-wide if this were the global handler, so the test
    /// isolates the scope question rather than the body-shape one.
    ///
    /// Oracle (tclsh8.6.14 and tclsh9.0.4): with
    /// `namespace eval ::mylib { proc unknown {args} { return handled } }`,
    /// calling `totallyBogusCommand` still fails
    /// `invalid command name "totallyBogusCommand"` — only a *global*
    /// `proc unknown` makes it return `handled`.
    #[test]
    fn analyse_namespace_local_unknown_proc_does_not_seed_the_global_handler() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "namespace eval ::mylib { proc unknown {cmd args} { exec $cmd {*}$args } }\n\
             totallyBogusCommand\n",
            "tcl9.0",
        );
        assert!(
            r.unknown_proc_info.is_none(),
            "a namespace-local proc named unknown is not the global handler",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W123),
            "so the bogus command is still unresolved; got {:?}",
            r.diagnostics,
        );
    }

    /// TP guard — the global handler still seeds the info (and still
    /// suppresses W123), which is the behaviour the C2 fix must not disturb.
    #[test]
    fn analyse_global_unknown_proc_still_seeds_the_handler() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { exec $cmd {*}$args }\ntotallyBogusCommand\n",
            "tcl9.0",
        );
        assert!(
            r.unknown_proc_info.is_some(),
            "a global proc unknown is the handler",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W123),
            "and it suppresses the unresolved-command warning; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_qualified_unknown_proc_also_populates_info() {
        // ``::tcl::unknown`` (the canonical fully-qualified
        // name) should trigger detection too.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc ::tcl::unknown {cmd args} { exec $cmd }", "tcl");
        let info = r.unknown_proc_info.expect("unknown_proc_info populated");
        assert!(info.has_exec);
    }

    // stray-close-bracket recovery

    #[test]
    fn analyse_top_level_repairs_stray_close_bracket() {
        // ``set x string]`` is a typo for ``set x [string ...]``.
        // The recovery should rewrite the third argv entry into
        // a virtual ``CMD`` token before dispatch so the var
        // record is registered with the recovered shape.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("set x string]", "tcl");
        // ``x`` ends up in scope as a single-arg ``set`` (a var
        // read), not as a two-arg ``set`` with the broken text
        // — recovery yields the synthetic ``[string]`` command
        // word so dispatch sees the intended shape.
        assert!(r.global_scope.variables.contains_key("x"));
    }

    // unknown_proc_info / package require

    #[test]
    fn analyse_records_package_require() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("package require Tcl 8.6", "tcl");
        assert_eq!(r.package_requires.len(), 1);
        let p = &r.package_requires[0];
        assert_eq!(p.name, "Tcl");
        assert_eq!(p.version.as_deref(), Some("8.6"));
        assert!(!p.conditional);
        assert!(!p.exact, "no `-exact` word means the ranged requirement");
    }

    #[test]
    fn analyse_records_package_require_exact_flag() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("package require -exact Tcl 8.6", "tcl");
        let p = &r.package_requires[0];
        assert_eq!(p.name, "Tcl");
        assert_eq!(p.version.as_deref(), Some("8.6"));
        // TP — issue #1090: the flag reaches the record, so the resolver can
        // narrow `8.6` to the degenerate range `8.6-8.6`.
        assert!(p.exact);
        // FP guard — a package *named* `-exact` is not the flag.
        let r = a.analyse("package require -exact", "tcl");
        let p = &r.package_requires[0];
        assert_eq!(p.name, "-exact");
        assert!(!p.exact);
    }

    /// `package ifneeded NAME VER SCRIPT` registers a load script; the
    /// two-word query form registers nothing (issue #1279).
    #[test]
    fn analyse_records_package_ifneeded_registrations_only() {
        let mut a = crate::analyser::Analyser::new();
        // TP — the setter form, the shape every `pkgIndex.tcl` writes.
        let r = a.analyse(
            "package ifneeded base64 2.5 [list source [file join $dir base64.tcl]]",
            "tcl",
        );
        assert_eq!(r.package_ifneededs.len(), 1);
        assert_eq!(r.package_ifneededs[0].name, "base64");
        assert_eq!(r.package_ifneededs[0].version, "2.5");
        // TP — a braced script body is equally a registration.
        let r = a.analyse("package ifneeded mylib 1.0 {source lib.tcl}", "tcl");
        assert_eq!(r.package_ifneededs.len(), 1);
        // TN — the two-word form is a *query*; it registers nothing, so a
        // document that merely asks what is registered must not be read as
        // making the package's loading dynamic.
        let r = a.analyse("package ifneeded base64 2.5", "tcl");
        assert!(r.package_ifneededs.is_empty());
    }

    /// `package prefer latest` is recorded; every other spelling of the
    /// subcommand changes no state and is not (issue #1126 item 1).
    #[test]
    fn analyse_records_package_prefer_latest() {
        let mut a = crate::analyser::Analyser::new();
        // TP — the one transition that exists.
        let r = a.analyse("package prefer latest", "tcl");
        assert_eq!(r.package_prefer_latest.len(), 1);
        assert!(!r.package_prefer_latest[0].conditional);
        // TP — inside a guarded branch the record carries `conditional`, so a
        // consumer can refuse to flip the selection rule on it.
        let r = a.analyse("catch {package prefer latest}", "tcl");
        assert_eq!(r.package_prefer_latest.len(), 1);
        assert!(r.package_prefer_latest[0].conditional);
        // TN — `stable` is a no-op from the default and silently ineffective
        // afterwards; the bare form is a query; a dynamic mode word is not
        // guessed at.
        for src in [
            "package prefer stable",
            "package prefer",
            "package prefer $mode",
            "package prefer [mode]",
        ] {
            let r = a.analyse(src, "tcl");
            assert!(r.package_prefer_latest.is_empty(), "{src}");
        }
    }

    #[test]
    fn analyse_w123_suppressed_when_package_require_seen() {
        // W123 is suppressed when any package require is on
        // file — package may load arbitrary commands.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("package require Foo\nbogus_command arg", "tcl");
        assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
    }

    #[test]
    fn analyse_w123_suppressed_when_unknown_proc_chains_original() {
        // ``proc unknown`` that chains the original handler is
        // a *dynamic* shape — W123 is suppressed entirely
        // because runtime can resolve any command name.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { _original_unknown $cmd {*}$args }\nbogus_command arg",
            "tcl",
        );
        assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
    }

    #[test]
    fn analyse_w123_suppressed_when_unknown_proc_calls_exec() {
        // ``exec $cmd`` inside ``unknown`` is a dynamic shape;
        // any command may be a real binary on PATH.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { exec $cmd {*}$args }\nbogus_command arg",
            "tcl",
        );
        assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
    }

    #[test]
    fn analyse_w123_still_fires_outside_explicit_dispatch_targets() {
        // ``proc unknown`` with ONLY explicit dispatch targets
        // (no exec / auto_load / chain / pattern / case-fold)
        // is *not* dynamic — W123 should still fire for
        // commands not in the explicit target set.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nbogus_command arg",
            "tcl",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "W123 expected for ``bogus_command`` outside explicit dispatch targets; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_suppressed_for_explicit_dispatch_target() {
        // ``foo`` is in the explicit dispatch_targets — even
        // for the non-dynamic shape, the per-invocation loop
        // suppresses W123 for it.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nfoo arg",
            "tcl",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W123 && d.message.contains("'foo'")),
            "W123 should not fire for command listed in dispatch_targets; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_still_fires_for_empty_unknown_stub() {
        // An empty ``unknown`` stub resolves nothing — W123
        // should still emit.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc unknown {cmd args} {}\nbogus_command arg", "tcl");
        // ``bogus_command`` should be flagged.
        assert!(r.diagnostics.iter().any(|d| d.code == DiagCode::W123));
    }

    // deep_param_traits plumbing
    //
    // These tests pin the contract that flipping
    // `Analyser::deep_param_traits` actually changes the
    // `ProcDef.param_traits` surface: the shallow pass alone
    // misses traits hidden inside braced bodies, the deep pass
    // catches them, and the union of both is what the analyser
    // exposes.  The call-graph / symbol-graph / dataflow-graph /
    // semantic-graph builders flip this on.

    #[test]
    fn analyse_with_deep_param_traits_surfaces_nested_eval() {
        // `$body` is buried inside a `foreach` body — the
        // shallow pass walks only top-level commands and misses
        // it.  Flipping `deep_param_traits` on must surface
        // `Eval`.
        let source = "proc f {items body} {\n  foreach item $items {\n    uplevel 1 $body\n  }\n}";
        let mut shallow = crate::analyser::Analyser::new();
        let shallow_r = shallow.analyse(source, "tcl");
        let shallow_proc = shallow_r.all_procs.get("::f").expect("::f proc registered");
        let shallow_body_traits = shallow_proc.param_traits.get("body");
        assert!(
            !shallow_body_traits.is_some_and(|s| s.contains(&crate::analyser::ProcArgTrait::Eval)),
            "shallow pass should miss nested Eval, got {shallow_body_traits:?}",
        );

        let mut deep = crate::analyser::Analyser::new();
        deep.deep_param_traits = true;
        let deep_r = deep.analyse(source, "tcl");
        let deep_proc = deep_r.all_procs.get("::f").expect("::f proc registered");
        let deep_body_traits = deep_proc
            .param_traits
            .get("body")
            .expect("body traits present with deep_param_traits on");
        assert!(
            deep_body_traits.contains(&crate::analyser::ProcArgTrait::Eval),
            "deep pass should surface nested Eval, got {deep_body_traits:?}",
        );
    }

    #[test]
    fn analyse_with_deep_param_traits_off_matches_shallow() {
        // For procs without nested-body usage, deep + shallow
        // produce the same trait map.  This pins that the deep
        // pass doesn't accidentally lose any shallow trait.
        let source = "proc g {body} { uplevel 1 $body }";
        let mut shallow = crate::analyser::Analyser::new();
        let shallow_r = shallow.analyse(source, "tcl");

        let mut deep = crate::analyser::Analyser::new();
        deep.deep_param_traits = true;
        let deep_r = deep.analyse(source, "tcl");

        let shallow_traits = shallow_r
            .all_procs
            .get("::g")
            .expect("::g registered")
            .param_traits
            .clone();
        let deep_traits = deep_r
            .all_procs
            .get("::g")
            .expect("::g registered")
            .param_traits
            .clone();
        assert_eq!(
            shallow_traits, deep_traits,
            "deep + shallow should match for top-level-only bodies",
        );
    }

    #[test]
    fn uplevel_nonzero_isolates_body_vars_from_proc() {
        // `uplevel 1 {set x 5}` runs in the caller's frame, not the enclosing
        // proc's — its `x` must not merge into the proc's locals (they are
        // different variables), and it lands in an isolated `Uplevel` child
        // scope tagged with the level word.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc f {} {\n    uplevel 1 {set x 5}\n    set y 1\n}\n",
            "tcl8.6",
        );
        let f_scope = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name == "f")
            .expect("f proc scope");
        assert!(f_scope.variables.contains_key("y"), "y is a proc local");
        assert!(
            !f_scope.variables.contains_key("x"),
            "uplevel body's `x` must not merge into the proc scope",
        );
        let up = f_scope
            .children
            .iter()
            .find(|c| c.kind == crate::analyser::ScopeKind::Uplevel)
            .expect("isolated uplevel scope");
        assert_eq!(up.name, "1", "the level word tags the frame");
        assert!(up.variables.contains_key("x"));
    }

    #[test]
    fn nested_def_in_qualified_encloser_does_not_overwrite_global() {
        // `proc a::outer { proc helper ... }`: the nested `helper` homes to the
        // encloser's *defining* namespace (`::a::helper`), not the lexical
        // global — so the real global `proc helper` is preserved rather than
        // overwritten in `all_procs`.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc helper {} { puts global }\nproc a::outer {} {\n    proc helper {} { puts nested }\n}\n",
            "tcl8.6",
        );
        assert!(
            r.all_procs.contains_key("::a::helper"),
            "nested helper must home to ::a::helper: {:?}",
            r.all_procs.keys().collect::<Vec<_>>(),
        );
        let global = r
            .all_procs
            .get("::helper")
            .expect("global ::helper preserved");
        assert_eq!(
            global.name_span.start(),
            5,
            "::helper must remain the global declaration (line 0), not the nested one",
        );
        // The same for a nested class.
        let mut b = crate::analyser::Analyser::new();
        let rc = b.analyse(
            "oo::class create Widget {}\nproc a::outer {} {\n    oo::class create Widget {}\n}\n",
            "tcl8.6",
        );
        assert!(
            rc.all_classes.contains_key("::a::Widget"),
            "nested class homes to ::a::Widget: {:?}",
            rc.all_classes.keys().collect::<Vec<_>>(),
        );
        assert!(
            rc.all_classes.contains_key("::Widget"),
            "global ::Widget preserved",
        );
    }

    #[test]
    fn variable_global_upvar_skip_dynamic_names() {
        // A `variable` / `global` / `upvar` / `namespace upvar` whose name word
        // is computed (`$dyn` / `[f]`) is not a static declaration — the literal
        // substitution text must not be recorded as a variable.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "namespace eval ns {\n    variable $dyn\n    variable realvar 1\n}\nproc p {} {\n    global $g\n    upvar 1 other $loc\n}\n",
            "tcl8.6",
        );
        let ns = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name == "ns")
            .expect("ns scope");
        assert!(
            ns.variables.contains_key("realvar"),
            "a static `variable` name is still recorded",
        );
        assert!(
            !ns.variables.contains_key("dyn"),
            "`variable $dyn` must not record `dyn`: {:?}",
            ns.variables.keys().collect::<Vec<_>>(),
        );
        let p = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name == "p")
            .expect("p scope");
        assert!(
            !p.variables.contains_key("g") && !p.variables.contains_key("loc"),
            "dynamic `global`/`upvar` names must not be recorded: {:?}",
            p.variables.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn uplevel_zero_still_resets_to_global_frame() {
        // `uplevel #0` keeps the global-frame tag so variable resolution
        // resolves outward to the global namespace (not the caller-frame
        // abstention a non-`#0` level uses).
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc g {} {\n    uplevel #0 {set z 5}\n}\n", "tcl8.6");
        let g_scope = r
            .global_scope
            .children
            .iter()
            .find(|c| c.name == "g")
            .expect("g proc scope");
        let up = g_scope
            .children
            .iter()
            .find(|c| c.kind == crate::analyser::ScopeKind::Uplevel)
            .expect("uplevel scope");
        assert_eq!(up.name, "#0");
        assert!(up.variables.contains_key("z"));
    }

    // stub-overlay end-to-end
    //
    // These tests pin the contract that the stub overlay built
    // from `# tcl-lsp: stub` directives during `analyse()`
    // propagates into the per-proc `param_traits` map.

    #[test]
    fn analyse_with_stub_overlay_propagates_role_to_param_traits() {
        // The source declares a `# tcl-lsp: stub my_eval
        // {script:body}` directive, then defines a proc that
        // invokes `my_eval $body`.  The body arg's role flows
        // from the stub overlay → `param_traits["body"]
        // .contains(Body)`.
        let source = "\
# tcl-lsp: stubs-begin\n\
# tcl-lsp: stub my_eval {script:body}\n\
# tcl-lsp: stubs-end\n\
proc runs {body} {\n\
    my_eval $body\n\
}\n";
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(source, "tcl");
        let proc = r.all_procs.get("::runs").expect("::runs registered");
        let body_traits = proc
            .param_traits
            .get("body")
            .expect("body param has traits");
        assert!(
            body_traits.contains(&crate::analyser::ProcArgTrait::Body),
            "expected Body trait via stub overlay, got {body_traits:?}",
        );
    }

    #[test]
    fn analyse_without_stub_directive_leaves_body_untyped() {
        // Same proc, no stub directive — without the overlay
        // entry, `my_eval` isn't a known command, so the body
        // arg has no recorded role and `param_traits` is empty
        // for `body`.  This pins that the stub directive (not
        // some background heuristic) is what gives the
        // parameter its `Body` trait.
        let source = "proc runs {body} { my_eval $body }";
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(source, "tcl");
        let proc = r.all_procs.get("::runs").expect("::runs registered");
        assert!(
            !proc
                .param_traits
                .get("body")
                .is_some_and(|s| s.contains(&crate::analyser::ProcArgTrait::Body)),
            "expected NO Body trait without stub directive, got {:?}",
            proc.param_traits.get("body"),
        );
    }

    // -- issue #1001 follow-up: safe-interp visibility survives `analyse_per_item` --

    /// A **separate** gap found while investigating issue #1001, now fixed
    /// alongside it: `analyse_per_item`'s shell/body-pass split (`per_item.rs`)
    /// defers *every* proc/method body — including one nested inside a
    /// tracked `interp eval` safe-interpreter body — to an isolated second
    /// pass (`DeferredBody` / `analyse_proc_body_isolated`). Before this fix
    /// that pass carried no `safe_interp_stack` snapshot at all, so W129
    /// never fired for a hidden call inside *any* proc body nested in a safe
    /// interpreter when analysed incrementally (the live LSP server's
    /// diagnostics path always uses `analyse_per_item`) — even a
    /// directly-written call, with no bracket-substitution indirection
    /// whatsoever. `DeferredBody::safe_interp_ctx` (a flattened snapshot of
    /// the stack's top entry, captured when the body is deferred and
    /// restored in `analyse_proc_body_isolated`) closes this.
    #[test]
    fn safe_interp_w129_reaches_deferred_proc_body_under_per_item_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item(
            "interp create -safe s\ninterp eval s { proc f {} { source foo }; f }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a hidden call inside a deferred proc body warns under \
             incremental (per-item) analysis, matching `Analyser::analyse`: {:?}",
            r.diagnostics
        );
    }

    /// The same fix for a directly-written `apply` lambda body (also
    /// deferred by the shell pass) rather than a `proc` body.
    #[test]
    fn safe_interp_w129_reaches_deferred_apply_body_under_per_item_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item(
            "interp create -safe s\ninterp eval s { apply {{} { source foo }} }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a hidden call inside a deferred apply-lambda body warns under \
             incremental (per-item) analysis: {:?}",
            r.diagnostics
        );
    }

    /// The same fix for a `TclOO` method body (also deferred by the shell pass
    /// via a distinct push site in `oo.rs`).
    #[test]
    fn safe_interp_w129_reaches_deferred_method_body_under_per_item_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item(
            "interp create -safe s\n\
             interp eval s {\n\
                 oo::class create C { method m {} { source foo } }\n\
                 [C new] m\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a hidden call inside a deferred TclOO method body warns under \
             incremental (per-item) analysis: {:?}",
            r.diagnostics
        );
    }

    /// FP guard: a deferred proc body outside any safe interpreter must not
    /// warn — the new `safe_interp_ctx` snapshot must stay `None` and inert
    /// for the overwhelming common case.
    #[test]
    fn deferred_proc_body_outside_any_safe_interp_is_untouched_under_per_item_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item("proc f {} { source foo }\nf\n", "tcl8.6");
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "no safe interpreter is involved, so no W129 can ever fire: {:?}",
            r.diagnostics
        );
    }

    /// A narrower, separate limitation the `safe_interp_ctx` fix above does
    /// **not** cover, found while adding it: a *nested* local redefinition —
    /// `proc source {…}` written **inside** a proc body that is itself
    /// deferred, redefining a name a *later statement in the same body* then
    /// calls — does not suppress W129 the way an identical redefinition at
    /// the top level of a tracked `interp eval` body already does (see
    /// `safe_interp_w129_redefined_command_not_flagged_through_indirection_1001`,
    /// which is unaffected — it never defers a body at all). Root cause:
    /// `mark_locally_defined_in_enclosing_interp` also requires
    /// `self.interp_path_stack` / `self.interpreters` to recognise the
    /// current interpreter, and `safe_interp_ctx` only snapshots
    /// `safe_interp_stack` (sufficient for the gate check itself, per that
    /// field's doc) — not those two, which `analyse_proc_body_isolated`'s
    /// fresh `Analyser` never seeds. Fixing this fully would mean threading
    /// the interpreter identity (not just its visibility snapshot) through
    /// `DeferredBody` too; out of scope here since the primary miss (a
    /// hidden call inside a deferred body not warning *at all*) is what this
    /// fix targets, and this narrower shadowing case is `SAFE_INTERP_HIDDEN`-
    /// specific low-severity — it only means an occasional cosmetic false
    /// positive on a rare pattern (redefining a hidden builtin's name
    /// *inside* the very body that also calls it), never a missed real
    /// violation. Pinned so a future contributor extending
    /// `DeferredBody` has a red test.
    #[test]
    fn safe_interp_w129_nested_redefinition_inside_deferred_body_still_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse_per_item(
            "interp create -safe s\n\
             interp eval s {\n\
                 proc f {} { proc source {} { return ok }; source }\n\
                 f\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "known narrower gap: a redefinition nested inside a deferred \
             body doesn't suppress W129 for a later call in the same body, \
             under per-item analysis specifically; flip this assertion if a \
             future fix threads interpreter identity through `DeferredBody`: {:?}",
            r.diagnostics
        );
    }

    /// Pins issue #1001's own second reported repro case verbatim:
    /// `{*}[list apply {...} $x]` combines *two* indirection mechanisms at
    /// once — `{*}`-expansion of this command's own effective head
    /// (`check_indirect_hiding`'s `{*}[list HEAD ...]` resolution) *and*
    /// the resolved head being `apply` (triggering the lambda-body
    /// recursion into `handle_apply_command`), so the hidden `source`
    /// nested inside the lambda body must still draw W129.
    #[test]
    fn safe_interp_w129_expand_list_quoted_apply_lambda_body_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s { {*}[list apply {dir {source $dir/evil.tcl}} $env(HOME)] }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "hidden `source` nested inside a `{{*}}[list apply ...]`-invoked \
             lambda body warns: {:?}",
            r.diagnostics
        );
    }

    /// Pins issue #1001's third reported repro case verbatim: the
    /// `package ifneeded` deferred script followed by the actual
    /// `package require` that triggers it — confirms the fix holds when the
    /// deferred script is later invoked, not just when it is merely
    /// declared.
    #[test]
    fn safe_interp_w129_list_quoted_apply_package_ifneeded_then_require_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 package ifneeded evil 1.0 [list apply {dir {source $dir/evil.tcl}} $env(HOME)]\n\
                 package require evil\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "the deferred script's hidden `source` warns once `package \
             require` is present too: {:?}",
            r.diagnostics
        );
    }

    // -- issue #1001 follow-up: namespace ensemble -map redirection --------

    /// TP: `namespace ensemble create -command myens -map {go source}` then
    /// `myens go ...` — the ensemble redirects `go` to the hidden `source`,
    /// so the call must warn exactly like a direct `source` call would, even
    /// though `myens` itself is never a registry name.
    #[test]
    fn safe_interp_w129_ensemble_create_map_redirect_to_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go source}\n\
                 myens go pkg.tcl\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an ensemble -map redirect to a hidden command warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: the same redirect declared via `namespace ensemble configure NAME
    /// -map {...}` (previously silently ignored entirely — only `create`
    /// was handled) rather than at `create` time.
    #[test]
    fn safe_interp_w129_ensemble_configure_map_redirect_to_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens\n\
                 namespace ensemble configure myens -map {go source}\n\
                 myens go pkg.tcl\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an ensemble -map redirect declared via `configure` warns: {:?}",
            r.diagnostics
        );
    }

    /// TP: the ensemble's default naming (no `-command`, so the command is
    /// the enclosing namespace's own name) also resolves the redirect.
    #[test]
    fn safe_interp_w129_ensemble_default_name_map_redirect_to_hidden_command_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace eval myns { namespace ensemble create -map {go source} }\n\
                 myns go pkg.tcl\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an ensemble redirect under its default (namespace) name warns: {:?}",
            r.diagnostics
        );
    }

    /// FP guard: an ensemble `-map` redirect to a *safe* command must not
    /// warn — this fix widens W129's recall, it must not start flagging
    /// ordinary ensemble dispatch.
    #[test]
    fn safe_interp_w129_ensemble_map_redirect_to_safe_command_not_flagged_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go puts}\n\
                 myens go hi\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an ensemble redirect to a safe command must not warn: {:?}",
            r.diagnostics
        );
    }

    /// FP guard: the exact same ensemble-redirect shape outside any safe
    /// interpreter must not warn (and, since the whole mechanism is gated on
    /// a non-empty `safe_interp_stack`, must not do anything at all).
    #[test]
    fn ensemble_map_redirect_outside_any_safe_interp_is_untouched_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "namespace ensemble create -command myens -map {go source}\n\
             myens go pkg.tcl\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "no safe interpreter is involved, so no W129 can ever fire: {:?}",
            r.diagnostics
        );
    }

    /// TP: a `-map` target that is itself a multi-word command prefix
    /// (`{source b.tcl}`, real and valid Tcl — tclsh 8.6.14-verified:
    /// `-map {go {string length}}` dispatches `myens go x` to `string
    /// length x`) must still resolve to its *head* command for W129,
    /// not be dropped entirely by a naive whitespace split across the
    /// pair boundary (codex review, #1001 follow-up).
    #[test]
    fn safe_interp_w129_ensemble_map_redirect_multiword_target_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens -map {go {source b.tcl}}\n\
                 myens go\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "an ensemble -map redirect to a multi-word hidden-command \
             prefix warns: {:?}",
            r.diagnostics
        );
    }

    /// TP/FN guard: `configure -map` *replaces* the ensemble's whole
    /// subcommand table in real Tcl (tclsh 8.6.14-verified: a subcommand
    /// dropped from a later `-map` becomes "unknown or ambiguous
    /// subcommand", not a leftover redirect) — a subcommand a later
    /// `-map` omits must stop resolving to its stale, no-longer-mapped
    /// target instead of still drawing W129 (codex review, #1001
    /// follow-up: merging into the cached map instead of replacing it
    /// left the stale entry behind).
    #[test]
    fn safe_interp_w129_ensemble_configure_map_replaces_not_merges_1001() {
        let mut a = Analyser::new();
        let r = a.analyse(
            "interp create -safe s\n\
             interp eval s {\n\
                 namespace ensemble create -command myens \
                     -map {bad source ok puts}\n\
                 namespace ensemble configure myens -map {ok puts}\n\
                 myens bad\n\
             }\n",
            "tcl8.6",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == tcl_core_types::DiagCode::W129),
            "a subcommand dropped by a later `-map` must not keep warning \
             through its stale mapping: {:?}",
            r.diagnostics
        );
    }
}
