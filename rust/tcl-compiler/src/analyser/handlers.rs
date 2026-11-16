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

//! Per-command handlers: the variable-mutation commands, plus `proc`,
//! `namespace eval`, the loop and branch forms, `catch`/`try`, `interp
//! alias` and `oo::objdefine`.
//!
//! The variable bindings:
//!
//! - [`Analyser::bind_value_word_assignment`] — a direct one-target write
//!   of a value word (`set var value`), read off the registry's `CellWrite`
//!   declaration
//! - [`Analyser::apply_state_transitions`] — every scope alias an
//!   invocation's state transitions state (`global`, `variable`, `upvar`,
//!   `namespace upvar`, a pack command's alias facts)
//! - [`Analyser::handle_var_binding_command`] — every loop and written
//!   variable a call's roles name (`foreach`, `dict for`, `incr`, `append`,
//!   `lassign`, …)
//!
//! Plus the remaining structural command handlers: proc-body
//! walking, `namespace eval`/`namespace ensemble`, `foreach`/`for`/
//! `switch`, `catch`/`try`, `interp alias`, `oo::objdefine`, and
//! alias resolution.

mod finite_foreach_source;

pub(super) use finite_foreach_source::FiniteForeachReportingState;
use finite_foreach_source::FiniteNameLookup;
use tcl_core_types::DiagCode;
use tcl_dialect::model::surface_admits;
use tcl_lexer::{Span, Token, TokenType};
use tcl_syntax::list::find_element;

use crate::alias::{command_table_transitions, is_current_interpreter};
use crate::lowering::structured::{braced_word_flags, quoted_word_flags};
use crate::parsing::syntax::descend::descend_token;
use crate::parsing::syntax::segment::segments_from_tree;
use crate::segmenter::SegmentedCommand;
use crate::signature_scan::params::bind_proc_formals;
use crate::signature_scan::types::ParamDef;
use crate::signature_scan::types::SignatureCommandAlias;
use crate::value_transfer::{WordForm, literal_selection};

use super::state::Analyser;
use super::types::{
    ClassDef, ClassFactory, DefinedSymbol, FactoryMember, FactoryWord, MemberSide,
    MetaclassProvenance, ProcDef,
};
use super::utils::{param_name_spans_for_token, parse_param_list};
use crate::registry_invocation::{load_trace_flag, load_trace_option};
use tcl_dialect::model::SpecSurface;

struct ResolvedProcedureReportingName {
    finite: Option<(crate::signature_scan::scope::SignatureSourceCommand, String)>,
    resolved: String,
}

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
/// *written* `name`'s colon runs canonicalise.
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

/// Parse `interp create`'s `?-safe? ?--? ?path?` words:
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
pub(super) fn parse_interp_create_path(words: &[String]) -> Option<&str> {
    let args = words.iter().map(String::as_str).collect::<Vec<_>>();
    parse_interp_create_words(&args).1
}

fn parse_interp_create_words<'a>(words: &[&'a str]) -> (bool, Option<&'a str>) {
    let mut safe = false;
    let mut path: Option<&str> = None;
    let mut past_flags = false;
    let mut too_many_paths = false;
    for &word in words {
        if !past_flags && word.starts_with('-') {
            // registry-axis-ok: options — the interp create flag run; until the option table carries the safe-interpreter flag
            if word == "--" {
                past_flags = true;
            // registry-axis-ok: options — the interp create flag run; until the option table carries the safe-interpreter flag
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

/// The child an invocation's transitions create: its path (`None` for an
/// auto-named child) and whether it starts safe — `None` when the call states
/// no creation (another command, or a shape Tcl rejects). A child whose
/// safety a computed option leaves unknown is not recorded as safe.
fn created_interpreter(
    transitions: &tcl_registry::StateTransitions,
) -> Option<(Option<tcl_registry::TransitionSubject>, bool)> {
    transitions
        .facts()
        .iter()
        .find_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::Interpreter(
                tcl_registry::InterpreterTransition::Create {
                    interpreter,
                    safety,
                },
            ) => Some((
                interpreter.clone(),
                *safety == tcl_registry::ChildInterpreterSafety::Safe,
            )),
            _ => None,
        })
}

/// The first position `roles` assigns `role`.
fn role_position(
    roles: &[(usize, tcl_registry::ArgRole)],
    role: tcl_registry::ArgRole,
) -> Option<usize> {
    roles
        .iter()
        .find(|&&(_, assigned)| assigned == role)
        .map(|&(index, _)| index)
}

/// The words at every position `roles` assigns `role`, in position order.
fn role_texts(
    roles: &[(usize, tcl_registry::ArgRole)],
    role: tcl_registry::ArgRole,
    args: &[String],
) -> Vec<String> {
    roles
        .iter()
        .filter(|&&(_, assigned)| assigned == role)
        .filter_map(|&(index, _)| args.get(index).cloned())
        .collect()
}

/// Whether a word substitutes at all: a braced word never does, whatever it
/// holds; any other word does when it carries a `$` or `[` or is itself a
/// variable or command substitution.
fn word_substitutes(text: &str, tok: Token) -> bool {
    tok.kind != TokenType::Str && super::diagnostics::helpers::has_substitution(text, &tok)
}

/// Whether a written-variable word names a static variable: a non-empty word
/// that substitutes nothing, or an element of an array whose name is written
/// literally — the key may substitute (`incr hits($word)`), the base may not.
fn names_static_variable(text: &str, tok: Token) -> bool {
    if text.is_empty() {
        return false;
    }
    if !word_substitutes(text, tok) {
        return true;
    }
    crate::naming::split_element_ref(text)
        .is_some_and(|(base, _)| !base.is_empty() && !crate::naming::is_dynamic_word(base))
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
    /// Original relation naming policy and selected root candidate.
    root_policy: tcl_syntax::naming::NamePolicyProtocol,
    root_slot: tcl_core_types::ByteCommandSlot,
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
#[derive(Clone, Copy)]
struct SelectedObjdefineBodyLayout {
    grammar: Option<&'static tcl_registry::definer::DefinitionBodyGrammar>,
    body_argument: Option<usize>,
    definer_disabled: bool,
}

// Immutable original source-body query shared by the actual handler and
// direct source-capability controls. This supplies no handler/body entry.
fn original_class_definition_body_in_analysis(
    source: &str,
    analysis: &super::types::AnalysisResult,
    offset: u32,
    token: Token,
    grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
) -> Option<std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBody>> {
    let input = analysis.resolved_input.as_ref()?;
    let context = input.context_registry();
    let original = crate::registry_invocation::source_structure::source_registry_words_at(
        source, analysis, offset,
    );
    let words = match original {
        Some(words)
            if matches!(words.source(), crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(_)) => words,
        original => crate::registry_invocation::source_structure::source_class_declaration_at(
            source, analysis, offset,
        )
        .and_then(|declaration| {
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                source,
                input.lexer_config(),
                &context,
                declaration.factory().clone(),
            )
        })
        .or(original)?,
    };
    (words.with_source_schema(&context, |schema| {
        schema
            .authored_source_definition_body_grammar()
            .is_some_and(|selected| std::ptr::eq(selected, grammar))
    }) == Some(true))
    .then_some(())?;
    let mut bodies = words
        .source_script_bodies(&context)
        .into_iter()
        .filter(|body| body.original_container().tokens().first() == Some(&token));
    let body = bodies.next()?;
    bodies.next().is_none().then(|| std::sync::Arc::new(body))
}

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
    /// Descriptor selected by the shared catalogue owner for this written head.
    registry_metaclass: Option<&'static tcl_registry::CommandSpec>,
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

/// How many unclassifiable creation calls one document keeps for the
/// post-pass retry — see [`Analyser::defer_class_creation`].
///
/// Only a call whose head names nothing the document knows is kept, so a real
/// file holds a handful: the shape exists to catch a metaclass proved after
/// the walk, and a document proves few. The cap bounds a generated or
/// half-edited file whose calls are all to unknown commands; past it the
/// document behaves exactly as it did before this pass existed, which is the
/// abstaining direction.
const MAX_DEFERRED_CLASS_CREATIONS: usize = 4096;

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
fn substitute_bound_words(
    word: &str,
    params: &[String],
    args: &[String],
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    if word.contains('\\') {
        return None;
    }
    let length = u32::try_from(word.len()).ok()?;
    let arena = tcl_lexer::ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::document(word),
        Span::new(0, length),
        tcl_lexer::word_parts::SubstFlags::default(),
        config,
    )
    .ok()?;
    let mut out = String::with_capacity(word.len());
    for part in arena.list(arena.root()) {
        match part.part {
            tcl_lexer::ExecutablePart::Text(_) => {
                let text = std::str::from_utf8(arena.bytes(part.span)?).ok()?;
                if text.contains('$') {
                    return None;
                }
                out.push_str(text);
            }
            tcl_lexer::ExecutablePart::Variable {
                name, index: None, ..
            } => {
                let name = std::str::from_utf8(arena.bytes(name)?).ok()?;
                let index = params.iter().position(|parameter| parameter == name)?;
                let value = args.get(index)?;
                if tcl_syntax::naming::is_dynamic_word(value) {
                    return None;
                }
                out.push_str(value);
            }
            tcl_lexer::ExecutablePart::Variable { index: Some(_), .. }
            | tcl_lexer::ExecutablePart::Command { .. }
            | tcl_lexer::ExecutablePart::Expression { .. }
            | tcl_lexer::ExecutablePart::ParseError(_) => return None,
        }
    }
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
///
/// **Only facts a walk actually established take part in the conflict check.**
/// Both sides here are `ClassDef`s, and `ClassDef::default()` fabricates one
/// concrete-looking value — `metaclass: "oo::class"` — that no walk need have
/// read. A record that never saw a creation command's head word carries it
/// while having observed nothing, so it is compared only when
/// [`ClassDef::metaclass_provenance`] says it was read; otherwise the observing
/// side's answer is simply adopted. Without that gate an `oo::define` stub for
/// a class made by a user metaclass contradicted the real metaclass and cost
/// the record its factory and superclasses.
fn join_parameterised_class_observations(
    existing: &ClassDef,
    observed: &ClassDef,
) -> Option<ClassDef> {
    let metaclasses_conflict = existing.metaclass_provenance == MetaclassProvenance::Observed
        && observed.metaclass_provenance == MetaclassProvenance::Observed
        && crate::naming::normalise_qualified_name(&existing.metaclass)
            != crate::naming::normalise_qualified_name(&observed.metaclass);
    if existing.qualified_name != observed.qualified_name
        || metaclasses_conflict
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
    // The stand-in loses to a real reading whichever side won above, so the
    // joined record names the metaclass the source actually wrote.
    if joined.metaclass_provenance != MetaclassProvenance::Observed
        && other.metaclass_provenance == MetaclassProvenance::Observed
    {
        joined.metaclass.clone_from(&other.metaclass);
        joined.metaclass_provenance = MetaclassProvenance::Observed;
    }
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
    // One of these observations *is* the creation, so the joined record is
    // not a cross-file extension stub even when the stub's side won above —
    // go-to-definition must still prefer this file's creation site.
    joined.via_define &= other.via_define;
    // Whichever side lost, its declarations are still declarations: an
    // `oo::define` stub's members belong to this class as surely as the
    // creation body's do. Deliberately after the line above:
    // `absorb_declarations` reads `via_define` on both records to decide who
    // wins a name they both declare, and by here `joined`'s answers "were
    // *both* of these observations stubs" — which is exactly the question
    // that rule needs asked.
    joined.absorb_declarations(other);
    Some(joined)
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

/// One piece of a manufacturer prologue word, as
/// [`Analyser::manufacturer_injected_template`] accounts for it.
#[derive(Debug, PartialEq, Eq)]
enum ProloguePiece<'a> {
    /// A `[…]` command substitution — one prologue member to read.
    Substitution(Token),
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

/// Read original executable components without treating literal text as a
/// substituted parameter. Unsupported components withdraw the whole template.
fn prologue_pieces(original: &tcl_lexer::NativeWord) -> Vec<ProloguePiece<'_>> {
    let arena = original.executable_parts();
    let source_map = tcl_lexer::SourceMap::from_image(original.image());
    let mut pieces = Vec::new();
    for part in arena.list(arena.root()) {
        let piece = match part.part {
            tcl_lexer::ExecutablePart::Text(_) => {
                if arena.text(part).is_some_and(|text| {
                    text.iter()
                        .all(|byte| byte.is_ascii_whitespace() || *byte == b';')
                }) {
                    ProloguePiece::Separator
                } else {
                    ProloguePiece::Opaque
                }
            }
            tcl_lexer::ExecutablePart::Variable { name, index: None } => arena
                .bytes(name)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map_or(ProloguePiece::Opaque, ProloguePiece::VarRead),
            tcl_lexer::ExecutablePart::Command { body } => {
                let mut groups = original.tokens().iter().filter(|token| {
                    token.kind == TokenType::Cmd
                        && token.span.start() == part.span.start()
                        && Some(source_map.token_text(**token).as_bytes()) == arena.bytes(body)
                });
                match (groups.next(), groups.next()) {
                    (Some(group), None) => ProloguePiece::Substitution(*group),
                    _ => ProloguePiece::Opaque,
                }
            }
            tcl_lexer::ExecutablePart::Variable { index: Some(_), .. }
            | tcl_lexer::ExecutablePart::Expression { .. }
            | tcl_lexer::ExecutablePart::ParseError(_) => ProloguePiece::Opaque,
        };
        let opaque = piece == ProloguePiece::Opaque;
        if piece != ProloguePiece::Separator || pieces.last() != Some(&piece) {
            pieces.push(piece);
        }
        if opaque {
            break;
        }
    }
    pieces
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
    original_arguments: Option<&[tcl_lexer::NativeWord]>,
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
                let original = original_arguments?.get(*arg_index)?;
                for element in tcl_syntax::word_rules::original_static_word_list_elements(original)?
                {
                    // This token is a display anchor for an actual list field,
                    // never an independently captured word or naming input.
                    let span = element.source_span()?;
                    member_words.push(element.value().to_owned());
                    member_tokens.push(Token {
                        kind: TokenType::Esc,
                        span,
                        content_offset: 0,
                        in_quote: false,
                    });
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
    original_arguments: Option<&[tcl_lexer::NativeWord]>,
) -> Option<ManufacturerLayout> {
    let Some(spec) = factory.overrides.get(&args[0]) else {
        return ManufacturerLayout::builtin(builtin);
    };
    let injected = spec.injected.as_ref().and_then(|members| {
        members
            .iter()
            .map(|m| resolve_factory_member(m, original_arguments))
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
    /// The parameter-list word's token.
    params_tok: Token,
    name_tok: Token,
    params: &'a [crate::signature_scan::types::ParamDef],
    /// The static-variable list word and its token, when the definer takes one
    /// and the call supplies it.
    statics: Option<(&'a str, Token)>,
    body_text: &'a str,
    body_tok: Token,
    ns_prefix: &'a str,
}

/// One element of a static-variable list.
struct StaticVariable {
    /// The name the body declares.
    name: String,
    /// Whether the element refers to the enclosing scope's variable of that
    /// name, which is read when the procedure is defined: a bare `name` or
    /// `&name`. A `{name value}` pair names its own initial value.
    reads_enclosing: bool,
}

/// The variables a static-variable list declares: each element is a bare
/// `name`, a `{name value}` pair, or `&name`, which refers to the enclosing
/// scope's variable of that name and declares the same name in the body.
fn static_variables(
    text: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Vec<StaticVariable> {
    parse_param_list(text, rules)
        .into_iter()
        .map(|param| {
            let reads_enclosing = !param.has_default;
            match param.name.strip_prefix('&') {
                Some(referenced) => StaticVariable {
                    name: referenced.to_owned(),
                    reads_enclosing,
                },
                None => StaticVariable {
                    name: param.name,
                    reads_enclosing,
                },
            }
        })
        .collect()
}

struct ApplyLambdaSourceFacts {
    body_ns: String,
    original_namespace: Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        Option<std::sync::Arc<super::interp_visibility::OriginalInterpreterSourceDomain>>,
    )>,
    logical_formals: Option<(Vec<ParamDef>, Vec<Span>)>,
    logical_namespace_source:
        Option<std::sync::Arc<super::interp_visibility::OriginalInterpreterLambdaBody>>,
}

struct ApplyLambdaBody {
    params: Vec<ParamDef>,
    original_param_spans: Option<Vec<Span>>,
    params_tok: Token,
    body_text: std::sync::Arc<str>,
    body_tok: Token,
    body_ns: String,
    scope_name: String,
    original_namespace: Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        Option<std::sync::Arc<super::interp_visibility::OriginalInterpreterSourceDomain>>,
    )>,
    logical_namespace_source:
        Option<std::sync::Arc<super::interp_visibility::OriginalInterpreterLambdaBody>>,
}

impl Analyser {
    /// Bind the variable a direct one-target write names to the value word
    /// it stores — `set name value` — for any invocation whose declared
    /// semantics is that write
    /// ([`tcl_registry::value_transfer::ResolvedSemantics::writes_value_word`]),
    /// the registry's declaration rather than the command's spelling or an
    /// analyser hook:
    ///
    /// - the written name is an assignment the unused-variable hint may
    ///   report, so its definition escalates `warn_if_unused` over the one
    ///   the generic role binding ([`Self::handle_var_binding_command`])
    ///   gave the same word;
    /// - the constant-string environment reads the value word's `CellWrite`
    ///   evaluation (`value_word_write`) when the word is one literal
    ///   token, so a regex source or a namespace a later word names
    ///   resolves through the variable;
    /// - a value word that is one `[…]` substitution creating a child
    ///   interpreter binds the name to that interpreter's key, the
    ///   interpreter domain reached from this binding;
    /// - a value word that is one `[…]` substitution the registry folds
    ///   binds the folded constant, and anything else clears both;
    /// - `set auto_path …` records the search path the value word names.
    ///
    /// The one-word read form (`set name`) binds nothing: the walk's
    /// `VarRead`-role reference recording
    /// ([`Self::record_arg_var_reads`]) takes it.
    pub fn bind_value_word_assignment(
        &mut self,
        command_token: Token,
        argument_tokens: &[Token],
        scope_path: &[usize],
    ) {
        use tcl_registry::arg_role::ArgRole;
        let Some(original) = self.original_diagnostic_call_at(command_token, argument_tokens)
        else {
            return;
        };
        let Some(Some((Some(roles), semantics, canonical))) = original.with_schema(|call| {
            call.semantics.value.writes_value_word().then(|| {
                (
                    call.arg_roles(),
                    call.semantics.value,
                    call.canonical_command.to_owned(),
                )
            })
        }) else {
            return;
        };
        let Some((arguments, tokens, single_token_word)) = original.source_arguments() else {
            return;
        };
        let args = arguments.as_slice();
        let arg_tokens = tokens.as_slice();
        // The write's shape (`CellWriteSemantics`): one `VarWrite` word, and
        // the value word after it, last.
        let Some(target) = role_position(&roles, ArgRole::VarWrite) else {
            return;
        };
        let value = target + 1;
        let (Some(name), Some(&name_tok), Some(value_text), Some(&value_tok)) = (
            args.get(target),
            arg_tokens.get(target),
            args.get(value),
            arg_tokens.get(value),
        ) else {
            return;
        };
        if value + 1 != args.len() {
            return;
        }
        if names_static_variable(name, name_tok) {
            self.define_var(name, name_tok, scope_path, true, None);
        }
        if original.literal(target).is_none() {
            return;
        }
        self.record_search_path_write(
            args,
            arg_tokens,
            target,
            value..value + 1,
            super::types::AutoPathForm::Assign,
        );
        let single = single_token_word.get(value).copied().unwrap_or(false);
        let written = single
            .then(|| {
                self.value_word_write(
                    semantics,
                    &canonical,
                    (args, roles.as_slice()),
                    (target, value),
                    original.literal(value)?,
                )
            })
            .flatten();
        if let Some(written) = written {
            self.set_const_string(name, written, value_tok.span, scope_path);
            self.clear_interp_var_binding(name, scope_path);
        } else if let Some((path, safe)) = original
            .word(value)
            .and_then(|word| self.original_substitution_call(word))
            .and_then(|call| {
                call.with_schema(|schema| created_interpreter(&schema.state_transitions()))
                    .flatten()
            })
        {
            self.bind_created_interpreter_value(path, safe, name, value_tok, scope_path);
        } else if single
            && value_tok.kind == TokenType::Cmd
            && let Some(folded) =
                self.try_fold_const_cmd_subst_rhs(value_text, value_tok, scope_path)
        {
            // `set VAR [cmd …]` whose substitution is a compile-time
            // constant: the registry `const_fold` / frame-fact engine proves
            // the value, so VAR enters the same constant-string lattice a
            // literal value does — unblocking the `${ns}::setdef`-style
            // navigation chain (`resolve_dynamic_command_head`) for the
            // `set ns [namespace qualifiers ::tc::X]` shape.
            self.set_const_string(name, folded, value_tok.span, scope_path);
            self.clear_interp_var_binding(name, scope_path);
        } else {
            self.clear_const_string(name, scope_path);
            self.clear_interp_var_binding(name, scope_path);
        }
    }

    fn bind_created_interpreter_value(
        &mut self,
        path: Option<tcl_registry::TransitionSubject>,
        safe: bool,
        name: &str,
        value_tok: Token,
        scope_path: &[usize],
    ) {
        // The substitution's registry transition, not its spelling, names the
        // created interpreter. Keep the variable binding in the scope chain.
        self.clear_const_string(name, scope_path);
        match path {
            Some(tcl_registry::TransitionSubject::Unknown { .. }) => {
                self.dynamic_interp_ops = true;
                self.clear_interp_var_binding(name, scope_path);
            }
            path => {
                let key = match path {
                    Some(tcl_registry::TransitionSubject::Literal(p)) => {
                        self.qualified_interp_key(&p)
                    }
                    _ => self.mint_synthetic_offset_name("@autoname@", value_tok.span.start()),
                };
                self.interpreters.insert(
                    key.clone(),
                    super::state::InterpState {
                        safe,
                        ..Default::default()
                    },
                );
                self.set_original_logical_interp_var_binding(name, key, value_tok, scope_path);
            }
        }
    }

    /// The value a direct one-target write stores when its value word is
    /// one literal token: the declared semantics' `CellWrite` evaluation
    /// over the call's words, the value word cooked as Tcl reads it
    /// (backslashes substituted in a bare or quoted word, the braces' content
    /// in a braced one), the resolver's roles on its operands. `None` for a
    /// word that is no literal, or an evaluation that writes no exact value
    /// to the target.
    fn value_word_write(
        &self,
        semantics: tcl_registry::value_transfer::ResolvedSemantics,
        canonical: &str,
        (args, roles): (&[String], &[(usize, tcl_registry::ArgRole)]),
        (target, value): (usize, usize),
        value_literal: &str,
    ) -> Option<String> {
        use tcl_registry::value_transfer::{
            Budget, EvalAnswer, LiteralInputs, OperandId, StoreOutcome,
        };
        let words: Vec<&str> = args
            .iter()
            .enumerate()
            .map(|(index, word)| {
                if index == value {
                    value_literal
                } else {
                    word.as_str()
                }
            })
            .collect();
        let input = self.resolved_analysis_input();
        let generation = input.context_registry();
        let mut context =
            tcl_registry::value_transfer::AnalysisContext::detached(Some(input.unit_profile()));
        context.grammar = self
            .lexer_config()
            .grammar_over(input.unit_profile().grammar);
        context.script_encoding = input.script_encoding();
        context.registry_generation = generation.commands().generation();
        context.overlay_generation = generation.commands().overlay_generation();
        let mut inputs = LiteralInputs::new(canonical, None, &words, Some(input.unit_profile()))
            .with_context(context);
        for &(index, role) in roles {
            inputs = inputs.with_role(OperandId(index), role);
        }
        match semantics
            .semantics()?
            .evaluate(&inputs, &mut Budget::evaluation())
        {
            EvalAnswer::Evaluated(outcome) => {
                outcome
                    .ordered_stores
                    .into_iter()
                    .find_map(|store| match store {
                        StoreOutcome::Write { target: t, value } if t.0 == OperandId(target) => {
                            String::from_utf8(value.bytes).ok()
                        }
                        _ => None,
                    })
            }
            EvalAnswer::Pending | EvalAnswer::Declined(_) => None,
        }
    }

    /// Registry folding keeps whole-module command trust and exact original variable sources.
    /// Declaration labels cannot supply entered object-frame facts.
    fn try_fold_const_cmd_subst_rhs(
        &mut self,
        value_text: &str,
        value_token: Token,
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
        let version = (!self.result.dialect.is_empty())
            .then(|| self.profile.const_fold_version())
            .flatten();
        let trusts = |name: &str| trust.trusts(name);
        let lookup = |arena: &tcl_lexer::ExecutablePartArena,
                      component: &tcl_lexer::SpannedExecutablePart| {
            self.lookup_logical_const_component(arena, component)
        };
        let operand = self.retained_value_word(value_token)?;
        crate::const_subst::ConstSubstCtx {
            registry: &registry,
            resolution_namespace: &self.command_resolution_namespace(scope_path),
            namespace_context: None,
            version,
            defining_class: None,
            trusts: &trusts,
            lookup_var: &|_| None,
            script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
        }
        .fold_original_source_substitution(&operand, &lookup)
    }

    /// Apply the scope aliases one invocation's state transitions state: each
    /// [`tcl_registry::VariableCellAliasTransition`] defines its local and
    /// links it to the cell it aliases, whichever command — `global`,
    /// `variable`, `upvar`, `namespace upvar`, or a pack command whose
    /// `state_transitions` resolver states alias facts — said so. Nothing
    /// here knows a command's layout: the registry resolver located every
    /// word, and [`tcl_registry::AliasWords`] says where each is spelled.
    ///
    /// Called from the dispatch tail for every command the walk's context
    /// proves, with `invocation` resolved over the words' source facts, so a
    /// computed word reaches the resolver as computed and the fact abstains
    /// on it (`alias_and_binding_resolvers_abstain_on_a_dynamic_word` pins
    /// the registry side).
    pub fn apply_state_transitions(
        &mut self,
        transitions: &tcl_registry::StateTransitions,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        for fact in transitions.facts() {
            if let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition {
                self.apply_variable_alias(alias, args, arg_tokens, scope_path);
            }
        }
    }

    /// Define one alias fact's local and link it to its cell.
    ///
    /// The local is a declaration (`warn_if_unused = false`), never a "set
    /// but unused" write. A computed local names no static variable, so the
    /// fact binds nothing. The link unifies every alias of one cell for
    /// Find-References and Rename; `target_span` is the word that names the
    /// cell — the declaration word itself for `global v`, one word earlier
    /// for `upvar #0 v local` — see [`super::types::VarDef::link_target_span`].
    fn apply_variable_alias(
        &mut self,
        alias: &tcl_registry::VariableCellAliasTransition,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let Some(local) = alias.local.literal() else {
            return;
        };
        let Some(local_tok) = arg_tokens.get(alias.words.local).copied() else {
            return;
        };
        self.define_var(local, local_tok, scope_path, false, None);
        let Some(target_tok) = arg_tokens.get(alias.words.target).copied() else {
            return;
        };
        let namespace =
            self.original_alias_target_namespace((local_tok.span, target_tok.span), scope_path);
        let original = self.record_original_variable_alias_target(
            local_tok.span,
            target_tok.span,
            namespace,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias,
        );
        if let Some(report) = self.alias_target_report(&alias.target, args, scope_path) {
            self.set_var_link_report(local, scope_path, report, target_tok.span);
        }
        if matches!(
            alias.target,
            tcl_registry::VariableAliasTarget::CallerSelectedFrame { .. }
        ) && let Some(original) = original
        {
            self.define_original_namespace_alias_target(&original);
        }
    }

    fn original_alias_target_namespace(
        &self,
        operands: (Span, Span),
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        let input = self.resolved_analysis_input();
        let target_input = self.original_qualified_variable_input(operands.1, false)?;
        let current = if input.has_logical_source_name_context() {
            self.head_identities
                .original_logical_variable_scope_at(
                    &input,
                    &tcl_lexer::SourceImage::document(&self.source),
                    operands.1,
                    &target_input,
                )?
                .namespace()
        } else {
            self.original_variable_namespace_at(scope_path)?
        };
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                &input,
                self.source_analysis_entry.as_deref(),
                self.declaration_name_policy()?,
            );
        let mut selected = None;
        for source in &self.result.original_variable_alias_source_operands {
            if !source
                .operands()
                .iter()
                .any(|&(local, target, _)| (local, target) == operands)
            {
                continue;
            }
            let namespace = source.target_namespace_scope(
                &self.source,
                &self.result,
                operands,
                &target_input,
                &current,
                purpose,
            )?;
            if selected.as_ref().is_some_and(|prior| prior != &namespace) {
                return None;
            }
            selected = Some(namespace);
        }
        selected
    }

    /// Reporting spelling and protective unknown markers. No symbol or target
    /// definition is constructed from this text.
    pub(super) fn alias_target_report(
        &self,
        target: &tcl_registry::VariableAliasTarget,
        args: &[String],
        scope_path: &[usize],
    ) -> Option<String> {
        use tcl_registry::VariableAliasTarget as Target;
        /// A subject's value when the resolver knew it, else its source word
        /// as written — the spelling, substitution marker included, never
        /// taken for the value it computes.
        fn spelt<'a>(
            subject: &'a tcl_registry::TransitionSubject,
            args: &'a [String],
        ) -> Option<&'a str> {
            match subject {
                tcl_registry::TransitionSubject::Literal(name)
                | tcl_registry::TransitionSubject::LocatedLiteral { value: name, .. } => Some(name),
                tcl_registry::TransitionSubject::LocatedNativeBytes { value, .. } => {
                    std::str::from_utf8(value).ok()
                }
                tcl_registry::TransitionSubject::Unknown { argument_index, .. } => {
                    args.get(*argument_index).map(String::as_str)
                }
            }
        }
        let path = match target {
            Target::Global { variable } => crate::naming::qualify("::", variable.literal()?),
            Target::CurrentNamespace { variable } => crate::naming::qualify(
                &self.command_resolution_namespace(scope_path),
                variable.literal()?,
            ),
            Target::Namespace {
                namespace,
                variable,
            } => {
                let namespace = crate::naming::qualify_namespace(
                    &self.command_resolution_namespace(scope_path),
                    spelt(namespace, args)?,
                );
                crate::naming::qualify(&namespace, spelt(variable, args)?)
            }
            Target::CallerSelectedFrame { frame, variable } => {
                // The array an element word lives in is named by the word's
                // base whatever its key, so a computed key still leaves the
                // written base readable: the source spelling is read for that
                // base alone, never taken as the variable's value.
                let other = spelt(variable, args)?;
                let path = Self::upvar_link_target(other, self.alias_frame_level(frame))?;
                return Some(path);
            }
        };
        Some(path)
    }

    /// The frame an alias's caller-frame selection reaches, in the analysed
    /// release's level grammar (`uplevel 010` is 8 frames up on 8.6 and 10
    /// on 9.0). With no registry loaded there is no release to name, so the
    /// parse abstains with `Dynamic` wherever the releases disagree, as does
    /// a computed level word — the direction every consumer of this level
    /// already treats conservatively.
    fn alias_frame_level(
        &self,
        frame: &tcl_registry::CallerFrameSelection,
    ) -> tcl_registry::frame_effect::FrameLevel {
        use tcl_registry::frame_effect::FrameLevel;
        match frame {
            tcl_registry::CallerFrameSelection::DefaultCaller => FrameLevel::DEFAULT,
            tcl_registry::CallerFrameSelection::Explicit(level) => level
                .literal()
                .and_then(|level| {
                    self.registry.as_ref().map_or_else(
                        || FrameLevel::parse(level),
                        |r| FrameLevel::parse_in(level, r),
                    )
                })
                .unwrap_or(FrameLevel::Dynamic),
        }
    }

    /// Handle a `global` declaration: a flat list of names; each gets
    /// a var binding with `warn_if_unused = false` (declared, not
    /// "set but unused").
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Global`].
    #[cfg(test)]
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
            let Some(local) = self.declaration_local_alias(
                name,
                i,
                tcl_registry::state_transition::VariableAliasNamePurpose::Global,
            ) else {
                continue;
            };
            // A dynamic name (`global $dyn`) computes its name at runtime — not
            // a static declaration.
            if let Some(tok) = arg_tokens.get(i)
                && !crate::naming::is_dynamic_word(name)
            {
                self.define_var(&local, *tok, scope_path, false, None);
                // `global v` aliases the global cell `::v`; `global ::ns::v`
                // aliases `::ns::v` as written.  Record the target so every
                // `global` alias and the global declaration unify.
                let Some(target) = self.declaration_alias_target("::", name) else {
                    continue;
                };
                // The declaration word *is* the cell's name here (`global v`
                // / `global ::ns::v`), so a rename of the cell rewrites this
                // very word — see `VarDef::link_target_span`.
                self.set_var_link_target(
                    &local,
                    scope_path,
                    super::scope::VariableLinkTarget {
                        target,
                        target_span: tok.span,
                        local_span: tok.span,
                        original_target_namespace: self.original_variable_root_namespace(),
                        original_target_receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias,
                    },
                );
            }
        }
    }

    /// Handle a `variable` declaration: alternating `name ?value?`
    /// pairs; only the names get bindings. The optional value words
    /// are skipped (the IR pass handles their assignment if the value
    /// form actually fires).
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Variable`].
    #[cfg(test)]
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
                let Some(local) = self.declaration_local_alias(
                    &args[i],
                    i,
                    tcl_registry::state_transition::VariableAliasNamePurpose::NamespaceVariable,
                ) else {
                    i += if i + 1 < args.len() { 2 } else { 1 };
                    continue;
                };
                self.define_var(&local, *tok, scope_path, false, None);
                let Some(target) = self.declaration_alias_target(&ns, &args[i]) else {
                    i += if i + 1 < args.len() { 2 } else { 1 };
                    continue;
                };
                // As with `global`, the declaration word names the cell.
                self.set_var_link_target(
                    &local,
                    scope_path,
                    super::scope::VariableLinkTarget {
                        target,
                        target_span: tok.span,
                        local_span: tok.span,
                        original_target_namespace: self.original_variable_namespace_at(scope_path),
                        original_target_receiver: if self.head_identities.source_bindings_ref().original_variable_uses_global_frame_at_span(tok.span, self.file_lexer_config())
                        && self.original_variable_namespace_at(scope_path).is_some_and(|namespace| match namespace {
                            crate::signature_scan::scope::SignatureNamespaceScope::C(path) => path.is_root(),
                            crate::signature_scan::scope::SignatureNamespaceScope::Jim(object) => object.is_empty(),
                            crate::signature_scan::scope::SignatureNamespaceScope::Symbolic(_) => false,
                        })
                    {
                        crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceVariableWithoutLink
                    } else {
                        crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias
                    },
                    },
                );
            }
            i += if i + 1 < args.len() { 2 } else { 1 };
        }
    }

    #[cfg(test)]
    fn declaration_local_alias(
        &self,
        name: &str,
        argument_index: usize,
        purpose: tcl_registry::state_transition::VariableAliasNamePurpose,
    ) -> Option<String> {
        let dialect = self.declaration_name_dialect();
        let subject = tcl_registry::state_transition::local_alias_name(
            &tcl_registry::TransitionSubject::Literal(name.to_owned()),
            argument_index,
            purpose,
            Some(dialect),
        )?;
        subject.literal().map(str::to_owned)
    }

    fn declaration_name_dialect(&self) -> tcl_registry::InvocationDialect {
        self.source_analysis_entry
            .as_ref()
            .and_then(|entry| entry.invocation_dialect)
            .unwrap_or_else(|| tcl_registry::InvocationDialect::of_profile(self.profile))
    }

    /// Independently selected Logical source advice. The authored declaration
    /// simulation can have a native-shaped naming policy without selecting a
    /// Native editing domain; actual entry and recipe remain separate barriers.
    pub(super) fn selected_logical_declaration_advice(&self) -> bool {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        crate::signature_scan::original_name::LogicalSourceNamePurpose::for_input(
            &self.resolved_analysis_input(),
            self.source_analysis_entry.as_deref(),
        )
        .is_some()
    }

    pub(super) fn declaration_name_policy(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        if self.vendor_source_name_policy().is_some()
            && self.source_analysis_entry.as_ref().is_none_or(|entry| {
                entry.native_entry.is_none()
                    && !matches!(
                        entry.execution_name_policy,
                        Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(_))
                    )
            })
        {
            // A hosted document supplied without an entry is a fresh rule
            // load (user decision 2026-10-21T00:10Z, "realm-aware command
            // world"): its loader defines each `proc` at load time under the
            // authored naming recipe of that entry.
            if self.source_analysis_entry.is_some() {
                return None;
            }
            return crate::command_binding::SourceAnalysisEntry::for_fresh_hosted_source(
                &self.resolved_analysis_input(),
            )
            .filter(|entry| !entry.unknown_entry)
            .and_then(|entry| match entry.execution_name_policy {
                Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(policy)) => Some(policy),
                _ => None,
            });
        }
        match self.source_analysis_entry.as_deref() {
            Some(entry) => match entry.native_entry.as_deref() {
                Some(native) => native.command_name_policy(),
                None => match entry.execution_name_policy {
                    Some(tcl_syntax::naming::ExecutionNamePolicy::ObservedBigIp(_)) => {
                        entry.invocation_dialect?.authored_name_policy()
                    }
                    Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(policy)) => {
                        Some(policy)
                    }
                    None => self.declaration_name_dialect().authored_name_policy(),
                },
            },
            None => self.declaration_name_dialect().authored_name_policy(),
        }
    }

    pub(super) fn declaration_namespace_scope(
        &self,
        path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        use crate::signature_scan::scope::SignatureNamespaceScope;
        let policy = self.declaration_name_policy()?;
        let mut current = match self
            .source_analysis_entry
            .as_deref()
            .and_then(|entry| entry.native_entry.as_deref())
        {
            Some(entry) => {
                let row = entry.namespace_context(entry.current_namespace).ok()?;
                match policy.recipe() {
                    tcl_syntax::naming::NativeNameProtocol::C(_) => {
                        SignatureNamespaceScope::C(row.path.clone())
                    }
                    tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                        SignatureNamespaceScope::Jim(row.jim_namespace_object.clone()?)
                    }
                }
            }
            None => SignatureNamespaceScope::root(Some(policy)),
        };
        let mut scope = &self.result.global_scope;
        for &index in path {
            scope = scope.children.get(index)?;
        }
        // Each receipt owns a complete namespace. Synthetic reporting wrappers
        // need not have their own recipe when the final lambda/declaration
        // scope independently retains one. An unknown final scope stays unknown.
        if !path.is_empty() {
            current = scope.naming_scope.clone()?;
        }
        Some(current)
    }

    #[cfg(test)]
    fn declaration_alias_target(&self, namespace: &str, name: &str) -> Option<String> {
        let dialect = self.declaration_name_dialect();
        if dialect.native_name_protocol().is_none()
            && dialect.family() == Some(tcl_dialect::model::Family::Tcl)
            && !name.contains('\0')
        {
            return Some(crate::naming::qualify(namespace, name));
        }
        Some(match dialect.native_name_protocol()? {
            tcl_syntax::naming::NativeNameProtocol::C(_) => crate::naming::qualify(namespace, name),
            tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                format!(
                    "::{}",
                    crate::naming::jim_global_variable_key(namespace, name)
                )
            }
        })
    }

    /// Record a lightweight named definition for a registry *symbol-definer*
    /// command (`tcltest::test NAME …`, …) so it appears in the document /
    /// workspace outline alongside procs, classes, and variables.
    ///
    /// Everything command-specific is registry data: which argument holds the
    /// name, which holds the description, and the outline category all come from
    /// the command's [`tcl_registry::SymbolDef`] — there is no `cmd == "test"`
    /// arm here, so registering the next such command (a benchmark runner, a
    /// custom test wrapper) is a one-line spec change.
    ///
    /// Names use original-word values and checked Logical declaration provenance.
    /// Unresolved substitutions cannot supply outline identities.
    pub(super) fn handle_defines_symbol(
        &mut self,
        advice: &crate::registry_invocation::OriginalSymbolDeclarationAdvice,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        command_token: Token,
        scope_path: &[usize],
    ) {
        if !self.registry_symbol_placement_is_valid(advice, args, arg_tokens, arg_single) {
            return;
        }
        let sym = advice.symbol;
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
        let vendor_name = self
            .result
            .original_vendor_source_name_in_source(
                &tcl_lexer::SourceImage::document(&self.source),
                self.file_lexer_config(),
                name_tok.span,
            )
            .cloned();
        let original_name_input = self
            .head_identities
            .original_written_name_input_at_span(name_tok.span, self.file_lexer_config())
            .or_else(|| {
                Some(
                    crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                        self.original_static_source_name_at_span(name_tok.span)?
                            .name_input()
                            .clone(),
                    ),
                )
            });
        let name = self.defined_symbol_reporting_name(
            original_name_input.as_ref(),
            vendor_name.as_ref(),
            name_text,
            name_tok,
            name_single,
            scope_path,
        );
        let Some(name) = name else {
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
            self.resolve_const_word(text, tok, single)
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
        // command-resolution rule, not the lexical one.
        let ns_prefix = self.command_resolution_namespace(scope_path);
        let qualified = qualify(&ns_prefix, &name);

        let symbol = DefinedSymbol {
            original_name_input,
            name,
            qualified_name: qualified,
            kind: sym.kind,
            name_span: name_tok.span,
            full_span,
            detail,
        };
        self.retain_vendor_symbol_metadata(vendor_name, &symbol);
        self.retain_defined_symbol_metadata(advice, command_token, scope_path, &symbol);
        self.result.all_defined_symbols.push(symbol.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.defined_symbols.push(symbol);
        }
    }

    fn procedure_definition_doc(&mut self, body: Option<&str>) -> String {
        let mut doc = std::mem::take(&mut self.last_comment);
        if doc.is_empty()
            && let Some(body) = body
        {
            doc = super::utils::extract_body_docstring(body);
        }
        doc
    }

    fn retain_unknown_procedure_info(
        &mut self,
        qualified: &str,
        body: &str,
        params: &[ParamDef],
        body_tok: Token,
    ) {
        if self.defines_global_unresolved_handler(qualified) {
            let info = self.extract_unknown_proc_info_at(
                body,
                params,
                Some(body_tok.span.start() + u32::from(body_tok.content_offset)),
            );
            self.result.unknown_proc_info = Some(info);
        }
    }

    fn bind_opt_proc_locals(
        &mut self,
        params_tok: Token,
        name_tok: Token,
        child_path: &[usize],
        opt_locals: &[ParamDef],
    ) {
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
        let args_span = Span::new(params_tok.span.start(), params_tok.span.start());
        self.define_var("args", params_tok, child_path, false, Some(args_span));
        // Bind every optlist-derived local, anchored to its own
        // descriptor's span (dash included — the written token, not a
        // byte-sliced substring) so go-to-definition / references /
        // rename on the parameter land on the real declaration.
        let param_spans = param_name_spans_for_token(&self.source, params_tok);
        for (i, p) in opt_locals.iter().enumerate() {
            self.define_var(
                &p.name,
                name_tok,
                child_path,
                false,
                param_spans.get(i).copied(),
            );
        }
    }

    fn retain_manufactured_class_declaration(
        &mut self,
        original_name: Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence>,
        qualified: String,
        class: ClassDef,
        scope_path: &[usize],
    ) {
        self.retain_original_provider_class(original_name, &class);
        let simple_key = class.name.clone();
        // The creation site's own body span, for `my`-dispatch resolution
        // to find — see `class_body_spans`'s doc.
        self.result
            .class_body_spans
            .push((qualified.clone(), class.body_span));
        self.result
            .retain_class_declaration(qualified, class.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.classes.insert(simple_key, class);
        }
    }

    fn procedure_reporting_names(
        &self,
        original_name: Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence>,
        name_word: Token,
        scope_path: &[usize],
        ns_prefix: &str,
        resolved_name: &str,
    ) -> (
        Option<crate::signature_scan::scope::SignatureSourceCommand>,
        String,
        String,
    ) {
        let source_name = original_name.and_then(|original| {
            crate::signature_scan::scope::SignatureSourceCommand::procedure_from_key(
                &self.declaration_namespace_scope(scope_path)?,
                original.name_input(),
            )
        });
        let source_name = source_name
            .or_else(|| self.original_interp_procedure_source_name(name_word, scope_path));
        let qualified = source_name
            .as_ref()
            .and_then(|name| {
                match super::scope::scope_at(&self.result.global_scope, scope_path)
                    .and_then(|scope| scope.original_interpreter_source_domain.as_ref())
                {
                    Some(domain) => domain.reported_command(name.slot()),
                    None => name.reported_full_name(),
                }
            })
            .unwrap_or_else(|| qualify(ns_prefix, resolved_name));
        let simple = source_name
            .as_ref()
            .and_then(crate::signature_scan::scope::SignatureSourceCommand::simple_name)
            .unwrap_or_else(|| crate::naming::key_tail(&qualified).to_string());
        (source_name, qualified, simple)
    }

    fn declared_procedure_parameters(
        &self,
        text: &str,
        token: Token,
        single: bool,
    ) -> (Vec<ParamDef>, bool) {
        let params_computed =
            !crate::signature_scan::params::param_word_is_literal(token.kind, single);
        let params = if params_computed {
            Vec::new()
        } else {
            parse_param_list(text, self.word_rules())
        };
        (params, params_computed)
    }

    fn declared_procedure_formal_count(
        &self,
        token: Token,
    ) -> crate::signature_scan::formal_count::SourceFormalCount {
        use crate::signature_scan::formal_count::SourceFormalCount;
        let Some(dialect) = self.original_source_invocation_dialect() else {
            return SourceFormalCount::Unknown;
        };
        if dialect.authored_name_policy().is_none() {
            return SourceFormalCount::Authored(
                dialect
                    .parameter_grammar()
                    .unwrap_or(tcl_dialect::ParameterGrammar::Tcl),
            );
        }
        self.original_static_source_name_at_span(token.span)
            .map_or(SourceFormalCount::Unknown, |original| {
                SourceFormalCount::from_original_input(original.name_input(), dialect)
            })
    }

    fn retain_vendor_symbol_metadata(
        &mut self,
        vendor_name: Option<crate::signature_scan::vendor_name::VendorSourceNameOccurrence>,
        symbol: &DefinedSymbol,
    ) {
        if let Some(original) = vendor_name {
            let record = crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata::new(
                &original,
                tcl_syntax::naming::VendorSourceNamePurpose::SourceName,
                symbol.clone(),
            );
            if !self
                .result
                .original_vendor_symbol_metadata
                .contains(&record)
            {
                self.result.original_vendor_symbol_metadata.push(record);
            }
        }
    }

    fn original_procedure_declaration_name(
        &self,
        name_tok: Token,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        self.declaration_name_policy().and_then(|policy| {
            self.head_identities
                .original_source_name_at_span(
                    name_tok.span,
                    self.file_lexer_config(),
                    self.word_rules(),
                    policy,
                )
                .or_else(|| {
                    // The selected procedure role supplies declaration advice;
                    // retaining its original static word requires no completed
                    // command world or reached installation.
                    let original = self.original_static_source_name_at_span(name_tok.span)?;
                    let key = original.name_input();
                    (key.policy() == policy
                        && key.lexer_config() == self.file_lexer_config()
                        && key.word_value_rules() == self.word_rules())
                    .then(|| original.clone())
                })
        })
    }

    fn retain_procedure_declaration_metadata(
        &mut self,
        original_name: Option<&crate::signature_scan::original_name::SourceOriginalNameOccurrence>,
        proc: &ProcDef,
        name_span: Span,
    ) {
        if self.selected_logical_declaration_advice()
            && let Some(declaration) = super::types::LogicalSourceProcedureDeclaration::capture(
                &self.result,
                &self.source,
                self.resolved_analysis_input(),
                proc,
            )
        {
            self.result.logical_procedure_declarations.push(declaration);
        }
        if let (Some(original), Some(name)) = (
            crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.proc.original_name",
                original_name,
            ),
            crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.proc.source_name",
                proc.source_name.as_ref(),
            ),
        ) && let Some(record) =
            crate::registry_invocation::source_structure::trace_jim_source_option(
                "jim.proc.declaration_record",
                crate::signature_scan::original_name::SourceDeclarationMetadata::new(
                    original,
                    name.clone(),
                    proc.clone(),
                ),
            )
            && !self.result.original_procedure_metadata.contains(&record)
        {
            self.result.original_procedure_metadata.push(record);
        }
        if let Some(original) = self.result.original_vendor_source_name_in_source(
            &tcl_lexer::SourceImage::document(&self.source),
            self.file_lexer_config(),
            name_span,
        ) {
            let record = crate::signature_scan::vendor_name::VendorSourceDeclarationMetadata::new(
                original,
                tcl_syntax::naming::VendorSourceNamePurpose::ProcedureName,
                proc.clone(),
            );
            if !self
                .result
                .original_vendor_procedure_metadata
                .contains(&record)
            {
                self.result.original_vendor_procedure_metadata.push(record);
            }
        }
    }

    fn record_apply_namespace_reference(
        &mut self,
        elements: &[(Token, String)],
        body_ns: &str,
        arg_tokens: &[Token],
    ) {
        // That element is also a first-class *reference* to the namespace it
        // names, not merely a semantic input: it sits
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
                self.result.namespace_refs.push(
                    super::types::NamespaceRef::from_original(
                        self.declaration_name_policy().map(|policy| {
                            crate::signature_scan::scope::SignatureNamespaceScope::root(Some(
                                policy,
                            ))
                        }),
                        self.declaration_name_policy(),
                        ns_text,
                        span,
                        0,
                        false,
                        body_ns.to_owned(),
                    )
                    .with_original_input(arg_tokens.first().and_then(
                        |token| {
                            self.head_identities
                                .original_written_name_input_at_span(
                                    token.span,
                                    self.file_lexer_config(),
                                )?
                                .original_list_element(2)
                        },
                    )),
                );
            }
        }
    }

    fn record_resolved_namespace_reference(
        &mut self,
        resolved_dynamic: Option<&str>,
        token: Option<&Token>,
        scope_path: &[usize],
        invocation_offset: Option<u32>,
    ) {
        if let (Some(resolved), Some(tok)) = (resolved_dynamic, token)
            && !self
                .result
                .namespace_refs
                .iter()
                .any(|reference| reference.span == tok.span)
        {
            // The general namespace role collector already retains genuine
            // computed inputs. Keep this presentation-only legacy record when
            // it has no independently retained input at the same source word.
            let here = self.command_resolution_namespace(scope_path);
            self.result.namespace_refs.push(
                super::types::NamespaceRef::from_original(
                    self.declaration_namespace_scope(scope_path),
                    self.declaration_name_policy(),
                    resolved,
                    tok.span,
                    u16::from(tok.content_offset),
                    true,
                    crate::naming::qualify(&here, resolved),
                )
                .with_original_input(match invocation_offset {
                    Some(offset) => self
                        .head_identities
                        .original_written_name_input_at_invocation_span(
                            &tcl_lexer::SourceImage::document(&self.source),
                            self.file_lexer_config(),
                            offset,
                            tok.span,
                        ),
                    None => self
                        .head_identities
                        .original_written_name_input_at_span(tok.span, self.file_lexer_config()),
                }),
            );
        }
    }

    fn apply_manufactured_class_members(
        &self,
        grammar: Option<&tcl_registry::definer::DefinitionBodyGrammar>,
        injected_members: &[InjectedMember],
        class: &mut ClassDef,
    ) {
        if let Some(grammar) = grammar {
            for injected in injected_members {
                super::oo::apply_oo_subcommand_in(
                    grammar,
                    &injected.texts,
                    &injected.argv,
                    class,
                    super::oo::MemberDialect {
                        surface: Some(self.analysis_context().context().authoring_query()),
                        rules: self.word_rules(),
                    },
                );
            }
        }
    }

    fn defined_symbol_reporting_name(
        &self,
        original_name_input: Option<&crate::signature_scan::scope::SignatureSourceNameInput>,
        vendor_name: Option<&crate::signature_scan::vendor_name::VendorSourceNameOccurrence>,
        name_text: &str,
        name_tok: Token,
        name_single: bool,
        _scope_path: &[usize],
    ) -> Option<String> {
        if let Some(input) = original_name_input {
            std::str::from_utf8(input.bytes())
                .ok()
                .map(str::to_owned)
                .or_else(|| {
                    tcl_syntax::backslash::native_literal_source_word(
                        input.bytes(),
                        tcl_lexer::SourceChannel::Document,
                        self.lexer_config(),
                        input.policy().string_protocol(),
                    )
                })
        } else if let Some(original) = vendor_name {
            // Registry-selected declaration advice retains the original hosted
            // source spelling when materialisation is unavailable. The label
            // supplies no canonical units or runtime identity.
            let input = original.name_input();
            input
                .literal_units(tcl_syntax::naming::VendorSourceNamePurpose::SourceName)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(str::to_owned)
                .or_else(|| {
                    let span = input.original_word().content_span().ok()?;
                    std::str::from_utf8(input.source_image().bytes().get(span.as_range())?)
                        .ok()
                        .map(str::to_owned)
                })
        } else if self.result.allows_lexical_declaration_advice() {
            // Explicit logical-only declaration advice has no native recipe.
            self.resolve_const_word(name_text, name_tok, name_single)
        } else {
            None
        }
    }

    fn retain_defined_symbol_metadata(
        &mut self,
        advice: &crate::registry_invocation::OriginalSymbolDeclarationAdvice,
        command_token: Token,
        scope_path: &[usize],
        symbol: &DefinedSymbol,
    ) {
        if let (Some(input), Some(head)) = (
            symbol.original_name_input.as_ref(),
            self.original_static_source_name_at_span(command_token.span),
        ) && let Some(registry) = self.result.resolved_registry().or(self.registry.as_deref())
            && let Some(record) =
                crate::signature_scan::symbol_name::OriginalSourceSymbolDeclaration::new(
                    head,
                    input,
                    advice.symbol,
                    super::scope::scope_at(&self.result.global_scope, scope_path)
                        .and_then(|scope| scope.naming_scope.as_ref()),
                    advice.conditional_metadata.as_ref(),
                    symbol,
                    registry,
                )
            && !self.result.original_symbol_metadata.contains(&record)
        {
            self.result.original_symbol_metadata.push(record);
        }
    }

    fn registry_symbol_placement_is_valid(
        &self,
        advice: &crate::registry_invocation::OriginalSymbolDeclarationAdvice,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> bool {
        // A registry symbol definer that is top-level-only in iRules does not
        // become a definition merely because its invalid nested spelling was
        // parsed. In particular, nested event-handler syntax must not leak
        // into document/workspace symbol inventories. Keep this driven by the
        // placement trait shared with IRULE5006, never by a command name.
        let cmd_name = advice.command.as_str();
        if self.profile.is_irules()
            && self.body_depth > 0
            && advice
                .traits
                .contains(tcl_registry::Traits::IRULES_TOP_LEVEL_ONLY)
        {
            return false;
        }
        let sym = advice.symbol;
        // An iRules event symbol is a declaration only when the shared
        // source-aware owner accepts its complete top-level shape. In
        // particular, a bare or quoted body must not turn an otherwise
        // ordinary command into an outline event.
        if self.profile.is_irules() && sym.kind == tcl_registry::DefinedSymbolKind::Event {
            let words: Vec<&str> = args.iter().map(String::as_str).collect();
            let valid = self.body_depth == 0
                && self.registry.as_deref().is_some_and(|registry| {
                    tcl_registry::events::closed_braced_argument_words(
                        &self.source,
                        arg_tokens,
                        arg_single,
                    )
                    .as_deref()
                    .and_then(|closed| {
                        tcl_registry::events::IrulesDeclarationArguments::new(
                            &words, arg_tokens, arg_single, closed,
                        )
                    })
                    .and_then(|arguments| {
                        registry.irules_top_level_declaration_shape(cmd_name, arguments)
                    })
                    .is_some_and(|declaration| {
                        matches!(
                            declaration,
                            tcl_registry::events::IrulesTopLevelDeclaration::Event { .. }
                        )
                    })
                });
            if !valid {
                return false;
            }
        }
        true
    }

    /// Resolve a single literal or original scalar substitution through checked value provenance.
    fn resolve_const_word(&self, text: &str, tok: Token, is_single: bool) -> Option<String> {
        if !is_single {
            return None;
        }
        match tok.kind {
            TokenType::Str | TokenType::Esc => Some(text.to_string()),
            TokenType::Var => self
                .lookup_logical_const_substitution(tok)
                .map(|(value, _)| value.to_owned())
                .or_else(|| self.frozen_value_word(tok)),
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
    /// name`).  Computed here, from the same
    /// body text and the same dialect config, so the three can never be
    /// derived from different views of the proc.
    fn infer_proc_param_traits(
        &self,
        params: &[crate::signature_scan::types::ParamDef],
        body_text: &str,
        body_span: tcl_lexer::Span,
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
        // different views of the proc *or* of the document's command bindings.
        let input = self.resolved_analysis_input();
        let original_logical_body = self
            .head_identities
            .original_logical_procedure_body_cards(&input)
            .and_then(|cards| {
                let mut matching = cards.into_iter().filter(|body| {
                    body.matches_source(&tcl_lexer::SourceImage::document(&self.source), &input)
                        && body.scope().scope_body().original_container().group().span == body_span
                        && body
                            .parameters()
                            .iter()
                            .map(|formal| formal.name.as_str())
                            .eq(param_names.iter().copied())
                });
                let body = matching.next()?;
                matching.next().is_none().then_some(body)
            });
        let env = super::param_traits::TraitScanEnv {
            surface: tcl_registry::model::DocumentCommandSurface::new(
                registry,
                self.declared_commands.as_ref(),
            ),
            config: self.file_lexer_config(),
            identities: &self.head_identities,
            executed_source: self.head_identities.executed_script_for_word(body_span),
            original_logical_body: original_logical_body.as_ref(),
        };
        let original_body_offset = self.result.retained_command_realm().and_then(|realm| {
            let config = self.file_lexer_config();
            let image = realm
                .original_source_image()
                .filter(|image| image.bytes() == self.source.as_bytes())?;
            let (word, _) =
                realm.original_written_word_at_span_in_source(image, body_span, config)?;
            word.content_span().ok().map(tcl_lexer::Span::start)
        });
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_PARAM_ROLES").is_some() {
            eprintln!(
                "ORIGINAL_PARAM_ROLES begin body={}..{} parameters={} executed_source={} deep={}",
                body_span.start(),
                body_span.end(),
                params.len(),
                env.executed_source.is_some(),
                self.deep_param_traits
            );
        }
        let body_text = match env.executed_source {
            Some(source) => {
                let Ok(text) = source.try_text() else {
                    return (
                        std::collections::HashMap::default(),
                        std::collections::HashSet::default(),
                        std::collections::HashMap::default(),
                    );
                };
                text
            }
            None => body_text,
        };
        let (caller_frame_params, caller_frame_literals) =
            super::param_traits::caller_frame_facts(&param_names, body_text, env);
        let shallow = super::param_traits::infer_param_traits(&param_names, body_text, env);
        let traits = if self.deep_param_traits {
            let deep = super::param_traits::infer_param_traits_deep_in_document(
                &param_names,
                body_text,
                env,
                &self.source,
                &self.result,
                original_body_offset,
            );
            super::param_traits::merge_traits(shallow, deep)
        } else {
            shallow
        };
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_PARAM_ROLES").is_some() {
            eprintln!(
                "ORIGINAL_PARAM_ROLES end body={} traits={} caller_parameters={} caller_literals={}",
                body_span.start(),
                traits.len(),
                caller_frame_params.len(),
                caller_frame_literals.len()
            );
        }
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
    /// proc, …) is excluded too: `registry.command_names()`
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
    fn emit_w113_proc_shadows_builtin(
        &mut self,
        raw_name: &str,
        qualified: &str,
        name_span: Span,
        source_name: Option<&crate::signature_scan::scope::SignatureSourceCommand>,
    ) {
        // Original structured publication geometry takes precedence over
        // reporting names, including literal colons and isolated interpreters.
        let original_global = match source_name {
            Some(name) if !name.slot().namespace.is_root() => return,
            Some(name) => {
                let Ok(simple) = name.slot().simple.try_utf8() else {
                    return;
                };
                Some(simple)
            }
            None => None,
        };
        let rooted_written = qualify("::", raw_name);
        let normalised_proc = tcl_syntax::naming::unroot_rooted_key(&rooted_written)
            .unwrap_or(&rooted_written)
            .to_owned();
        let normalised_qual = tcl_syntax::naming::unroot_rooted_key(qualified)
            .unwrap_or(qualified)
            .to_owned();
        let shadow_name: Option<String> = {
            let builtins = self.builtin_command_names();
            // A `tcl::mathop` operator (`%`, `+`, `eq`, …) carries a bare-name
            // registry entry only so an `import`ed / `namespace path`-reachable
            // call resolves; the command itself lives in `::tcl::mathop`, not the
            // global namespace, so `proc %` shadows nothing reachable. Detect it
            // by its qualified spelling and treat it like the other namespaced
            // (non-core-global) commands the check already skips.
            let is_mathop =
                |n: &str| builtins.contains(&crate::naming::unrooted_member_key("tcl::mathop", n));
            if let Some(simple) = original_global {
                (builtins.contains(simple) && !is_mathop(simple)).then(|| simple.to_owned())
            } else if builtins.contains(&normalised_proc) && !is_mathop(&normalised_proc) {
                Some(normalised_proc.clone())
            } else if builtins.contains(&normalised_qual) && !is_mathop(&normalised_qual) {
                Some(normalised_qual.clone())
            } else {
                None
            }
        };
        let shadow_name = shadow_name.filter(|name| {
            !tcl_syntax::naming::is_qualified(name.as_bytes())
                && !overridable_library_procs().contains(name.as_str())
                && !self.is_package_gated_non_ambient(name)
        });
        if shadow_name.is_some() {
            // The permissive fallback profile means "no specific dialect" —
            // no parenthetical label.
            let dialect_label = if self.profile.is_fallback() {
                String::new()
            } else {
                format!(" ({})", self.dialect())
            };
            let message = format!("Procedure '{raw_name}' shadows built-in command{dialect_label}");
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W113,
                    name_span,
                    message,
                    super::types::Severity::Warning,
                ));
        }
    }

    /// Whether `name` (a bare or fully-qualified command name already known
    /// to be in `registry.command_names()`) resolves to a `CommandSpec` gated
    /// behind a `required_package` / `tcllib_package` (`CommandSpec::
    /// owning_package`) that this profile does **not** ship ambiently.
    ///
    /// Data-driven, not a hardcoded package-name list: the answer comes
    /// straight from the resolved spec's package attribution and the
    /// registry's own `is_ambient_package` query (the same query
    /// `emit_missing_package_require_diagnostics` / W120 already uses for the
    /// converse fact), so a new package-gated spec is covered automatically —
    /// no per-package entry to remember to add here. The registry is asked
    /// rather than the profile so a pack's `ambient_package` row counts too.
    fn is_package_gated_non_ambient(&self, name: &str) -> bool {
        // Resolution reads the un-overlaid generation's store (as the old
        // `registry_for_profile` call did); the ambient answer reads this
        // walk's own — possibly pack-overlaid — context, the same split
        // W120 uses.
        let base = crate::environment_ingress::context_for_profile(self.profile);
        base.context()
            .resolve_spec(base.commands(), name)
            .and_then(tcl_registry::CommandSpec::owning_package)
            .is_some_and(|pkg| !self.analysis_context().context().ambient_package(pkg))
    }

    /// **W314.** The definition's name has no absolute (fully-qualified)
    /// written form.  A written colon run of two or more is a
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
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::W314,
                name_span,
                format!(
                    "'{raw_name}' has no absolute (fully-qualified) name — a written colon run \
                 is a namespace separator, so this definition is reachable only by \
                 unqualified/relative lookup ([namespace which] output will not resolve, \
                 and ensembles cannot dispatch it)"
                ),
                super::types::Severity::Warning,
            ));
    }

    /// Logical declaration compatibility for class definition-abort reports.
    /// The source member tables retain authored order, without a selected Native
    /// definition worker or closed current own-method table. Every candidate is
    /// consumed; only positively retained Logical input permits reporting advice.
    pub(super) fn emit_w315_definition_cannot_run(&mut self, class: &mut super::types::ClassDef) {
        let aborts = core::mem::take(&mut class.definition_aborts);
        // naming.diagnostics.original-definition-abort-admission
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-definition-abort-admission.md
        if !self.result.allows_retained_logical_declaration_advice() || class.via_define {
            // A missing Logical admission or a cross-file extension cannot
            // justify an abort from these reporting tables. Retain its source
            // tombstone without claiming a successful Native retraction.
            return;
        }
        // Within Logical compatibility, an abort report replaces the source
        // tombstone for that same retraction. Neither reporting form describes
        // a successful Native operation or its current method table.
        if !aborts.is_empty() {
            class.retracted_members.retain(|record| {
                !aborts.iter().any(|a| {
                    a.kind == super::types::DefinitionAbortKind::MissingMember
                        && a.member == record.member
                })
            });
        }
        for abort in aborts {
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W315,
                    abort.span,
                    abort.message(),
                    super::types::Severity::Warning,
                ));
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
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::W314,
                span,
                format!(
                    "namespace '{written_ns}' has no absolute (fully-qualified) path — a \
                 written colon run is a namespace separator, so everything defined \
                 inside is reachable only by relative lookup"
                ),
                super::types::Severity::Warning,
            ));
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
            // registry-axis-ok: irreducible — the variadic `args` formal is Tcl's proc grammar (`VAR_IS_ARGS` on the last formal, `tclProc.c`), no registry fact; until never
            if i == last || p.name != "args" {
                continue;
            }
            let span = param_spans.get(i).copied().unwrap_or(fallback_tok.span);
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W218,
                    span,
                    "`args` here is an ordinary parameter — it only has its special \
                     collect-the-rest meaning as the final parameter. Move it last, or \
                     rename it if a plain parameter is intended."
                        .to_string(),
                    super::types::Severity::Warning,
                ));
        }
    }

    fn procedure_words_for_analyser(
        &self,
        command: &str,
        args: &[String],
        arg_tokens: &[Token],
        source_head: Option<Token>,
    ) -> Option<tcl_registry::ProcedureWords> {
        if let Some(head) = source_head {
            let words = self.original_procedure_source_words_at(head, args, arg_tokens);
            // The interpretation never enters a child interpreter's script
            // or the recovered tail of a document that did not parse, so a
            // declaration there has no retained words: its layout is the
            // written head's in the registry. A computed name stays refused,
            // since its value may name another namespace.
            let unreached_script = (!self.interp_path_stack.is_empty() || self.recovered_source)
                && args
                    .first()
                    .is_some_and(|name| !crate::naming::is_dynamic_word(name))
                && !self
                    .head_identities
                    .source_bindings_ref()
                    .reached_site(head.span.start());
            if words.is_some() || !unreached_script {
                return words;
            }
        }
        // Legacy direct callers retain their existing Native-shape projection.
        let arg_words: Vec<&str> = args.iter().map(String::as_str).collect();
        Some(
            self.registry
                .as_deref()
                .and_then(|registry| registry.procedure_definition_words(command, &arg_words))
                .unwrap_or(tcl_registry::ProcedureWords::TCL_PROC),
        )
    }

    fn original_procedure_source_words_at(
        &self,
        head: Token,
        args: &[String],
        arg_tokens: &[Token],
    ) -> Option<tcl_registry::ProcedureWords> {
        use crate::registry_invocation::source_structure::trace_jim_source_option as observed;
        let words = observed(
            "analyser.proc.words",
            crate::registry_invocation::source_structure::source_registry_words_at(
                &self.source,
                &self.result,
                head.span.start(),
            ),
        )?;
        Self::original_procedure_source_arguments_match(&words, args, arg_tokens)?;
        let context = self.analysis_context();
        let positions = observed(
            "analyser.proc.schema",
            words.with_source_schema(&context, |schema| {
                observed(
                    "analyser.proc.hook",
                    (schema.semantics.analyser_hook
                        == Some(tcl_registry::hooks::AnalyserHookId::Proc))
                    .then_some(()),
                )?;
                observed(
                    "analyser.proc.shape",
                    schema.authored_source_procedure_arguments(),
                )
            }),
        )??;
        let mut statics = observed("analyser.proc.roles", words.roles())?
            .iter()
            .filter_map(|&(position, role)| {
                (role == tcl_registry::ArgRole::StaticVarList).then_some(position)
            });
        let static_at = statics.next();
        if statics.next().is_some() {
            observed::<()>("analyser.proc.statics", None);
            return None;
        }
        let required = [positions.name, positions.parameters, positions.body];
        if required.iter().any(|&index| index >= args.len())
            || required[0] == required[1]
            || required[0] == required[2]
            || required[1] == required[2]
            || static_at.is_some_and(|index| index >= args.len() || required.contains(&index))
        {
            observed::<()>("analyser.proc.positions", None);
            return None;
        }
        observed(
            "analyser.proc.complete",
            Some(tcl_registry::ProcedureWords {
                name: positions.name,
                params: positions.parameters,
                statics: static_at,
                body: positions.body,
            }),
        )
    }

    fn original_procedure_source_arguments_match(
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        args: &[String],
        arg_tokens: &[Token],
    ) -> Option<()> {
        use crate::registry_invocation::source_structure::trace_jim_source_option as observed;
        if words.arguments().len() != args.len() || arg_tokens.len() != args.len() {
            observed::<()>("analyser.proc.argument_count", None);
            return None;
        }
        for (index, (text, token)) in args.iter().zip(arg_tokens).enumerate() {
            if words.origins().get(index + 1)
                != Some(&crate::registry_invocation::InvocationWordOrigin::Written(
                    index + 1,
                ))
            {
                observed::<()>("analyser.proc.origin", None);
                return None;
            }
            let word = observed("analyser.proc.word", words.original_argument_word(index))?;
            if word.group().expand {
                observed::<()>("analyser.proc.expansion", None);
                return None;
            }
            let projection = crate::segmenter::segment_commands_with_offset_and_config(
                observed("analyser.proc.text", word.try_text().ok())?,
                word.word_span().start(),
                word.config().nested(),
            );
            let [original] = projection.as_slice() else {
                observed::<()>("analyser.proc.projection", None);
                return None;
            };
            if observed(
                "analyser.proc.argv",
                (original.argv == [*token]).then_some(()),
            )
            .is_none()
                || observed(
                    "analyser.proc.value",
                    (original.texts == [text.as_str()]).then_some(()),
                )
                .is_none()
                || observed(
                    "analyser.proc.tokens",
                    (original.all_tokens == word.tokens()).then_some(()),
                )
                .is_none()
            {
                return None;
            }
        }
        Some(())
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
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        self.handle_proc_command_with_source_head(
            cmd_name, args, arg_tokens, arg_single, scope_path, None,
        )
    }

    pub(super) fn handle_proc_command_at(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        source_head: Token,
    ) -> bool {
        self.handle_proc_command_with_source_head(
            cmd_name,
            args,
            arg_tokens,
            arg_single,
            scope_path,
            Some(source_head),
        )
    }

    fn resolve_proc_name(
        &self,
        raw_name: &str,
        name_tok: Token,
        arg_single: &[bool],
        scope_path: &[usize],
        source_head: Option<Token>,
    ) -> Option<ResolvedProcedureReportingName> {
        let finite_name = match self.finite_foreach_names.lookup(
            source_head,
            name_tok,
            scope_path,
            self.body_depth,
        ) {
            FiniteNameLookup::Ordinary => None,
            FiniteNameLookup::Refused => return None,
            FiniteNameLookup::Candidate(receipt) => {
                Some(receipt.reborrow(self, name_tok, scope_path)?)
            }
        };
        // A constant-foldable dynamic name (`proc ::$wtype {args} {...}` with
        // `wtype` a known constant) resolves the same way `rename`'s operands
        // already do (`tk/library/accessibility.tcl`'s
        // rename-away-and-reinstall idiom names its wrapper proc this way).
        // An unresolvable dynamic name falls back to the raw written text
        // unchanged — this only *improves* the resolvable case, the same
        // scope boundary `resolve_dynamic_word`'s other callers keep.
        let resolved_name = finite_name.as_ref().map_or_else(
            || {
                self.resolve_dynamic_word(
                    raw_name,
                    Some(name_tok),
                    arg_single.first().copied().unwrap_or(false),
                    scope_path,
                )
                .unwrap_or_else(|| raw_name.to_owned())
            },
            |(_, text)| text.clone(),
        );
        Some(ResolvedProcedureReportingName {
            finite: finite_name,
            resolved: resolved_name,
        })
    }

    fn procedure_reporting_body_span(&self, body: Token, finite: bool) -> Span {
        if finite {
            tcl_lexer::word_span_at(&*self.source, body.span)
        } else {
            body.span
        }
    }

    fn handle_proc_command_with_source_head(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        source_head: Option<Token>,
    ) -> bool {
        // iRules procedures are declaration-only, exact three-word forms at
        // file scope. An invalid declaration is consumed but deliberately
        // contributes no symbol, binding, scope, or executable body facts.
        if self.profile.is_irules()
            && !self.is_valid_irules_procedure_declaration(args, arg_tokens, arg_single)
        {
            return true;
        }
        let Some(words) =
            self.procedure_words_for_analyser(cmd_name, args, arg_tokens, source_head)
        else {
            return false;
        };
        let last = words.name.max(words.params).max(words.body);
        if args.len() <= last || arg_tokens.len() <= last {
            return false;
        }

        let raw_name = &args[words.name];
        let name_tok = arg_tokens[words.name];
        let Some(reporting) =
            self.resolve_proc_name(raw_name, name_tok, arg_single, scope_path, source_head)
        else {
            return false;
        };
        let ResolvedProcedureReportingName {
            finite: finite_name,
            resolved: resolved_name,
        } = reporting;
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
        // an `rsplit("::")` here would collapse a lone-colon name.
        let original_name = self.original_procedure_declaration_name(name_tok);
        let (mut source_name, mut qualified, mut simple) = self.procedure_reporting_names(
            original_name.as_ref(),
            name_tok,
            scope_path,
            &ns_prefix,
            &resolved_name,
        );
        let finite_reporting = finite_name.is_some();
        if let Some((name, _)) = finite_name {
            let (Some(full), Some(tail)) = (name.reported_full_name(), name.simple_name()) else {
                return false;
            };
            source_name = Some(name);
            qualified = full;
            simple = tail;
        }
        let name_span = name_tok.span;
        let body_tok = arg_tokens[words.body];
        let body_span = self.procedure_reporting_body_span(body_tok, finite_reporting);

        // **W113** — proc name shadows a built-in command.
        self.emit_w113_proc_shadows_builtin(
            &resolved_name,
            &qualified,
            name_span,
            source_name.as_ref(),
        );
        // **W314** — the name has no absolute written form.
        self.emit_w314_no_absolute_name(raw_name, name_span);

        // A **computed** parameter-list word (`proc p [makeargs] {…}`,
        // `proc q $params {…}`) builds the formals from a run-time value, so
        // nothing about them is knowable: the proc's params are unmodelled.
        // Reading the unresolved word as a one-parameter
        // literal registered a `VarDef` literally named `"[makeargs]"` and
        // made the call-site arity checker demand exactly one argument, which
        // tclsh 9.0.4 / 8.6.16 contradict (`proc makeargs {} {return {a b}}`;
        // `proc p [makeargs] {…}`; `info args p` → `a b`; `p 1 2` runs).
        //
        // The literalness rule itself lives in
        // `signature_scan::params::param_word_is_literal` — the one predicate
        // this tier, the signature-scan tier, and the LSP's cursor classifier
        // all share.
        let (params, params_computed) = self.declared_procedure_parameters(
            &args[words.params],
            arg_tokens[words.params],
            arg_single.get(words.params).copied().unwrap_or(true),
        );
        // Doc string: prefer the preceding-comment harvest from
        // the segmenter; fall back to ``extract_body_docstring``
        // (leading comment block at the top of the body).
        let doc = self.procedure_definition_doc(Some(&args[words.body]));

        // When a user defines the *global* unresolved-command handler,
        // inspect the body to determine which commands it can resolve. The
        // result gates W123 (unresolved command) file-wide — with a
        // user-supplied handler in place we cannot statically prove a
        // command is truly unresolved.
        self.retain_unknown_procedure_info(&qualified, &args[words.body], &params, body_tok);

        let body_text = &args[words.body];
        let (param_traits, caller_frame_params, caller_frame_literals) =
            self.infer_proc_param_traits(&params, body_text, body_tok.span);

        let proc = ProcDef {
            source_name,
            name: simple,
            qualified_name: qualified.clone(),
            params: params.clone(),
            params_computed,
            formal_count: self.declared_procedure_formal_count(arg_tokens[words.params]),
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
        self.retain_procedure_declaration_metadata(original_name.as_ref(), &proc, name_span);
        self.register_proc_definition(&qualified, &proc, name_span);
        self.record_two_word_proc_member(&resolved_name, &proc, scope_path);
        let simple_key = proc.name.clone();
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.procs.insert(simple_key.clone(), proc);
        }
        // Original command-transition source advice tracks this definition
        // independently of the reporting procedure table.

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
                params_tok: arg_tokens[words.params],
                name_tok,
                params: &params,
                statics: words.statics.map(|at| (args[at].as_str(), arg_tokens[at])),
                body_text: &args[words.body],
                body_tok,
                ns_prefix: &ns_prefix,
            });
        }

        true
    }

    fn is_valid_irules_procedure_declaration(
        &self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
    ) -> bool {
        if self.body_depth != 0 {
            return false;
        }
        let words: Vec<&str> = args.iter().map(String::as_str).collect();
        let generation;
        let registry = if let Some(stashed) = self.registry.as_deref() {
            stashed
        } else {
            generation = self.analysis_context();
            generation.commands()
        };
        let closed = tcl_registry::events::closed_braced_argument_words(
            &self.source,
            arg_tokens,
            arg_single,
        );
        closed
            .as_deref()
            .and_then(|closed| {
                tcl_registry::events::IrulesDeclarationArguments::new(
                    &words, arg_tokens, arg_single, closed,
                )
            })
            .and_then(|arguments| {
                registry.irules_top_level_declaration(
                    "proc",
                    arguments,
                    &tcl_registry::events::EventRegistry::build(),
                )
            })
            .is_some_and(|declaration| {
                matches!(
                    declaration,
                    tcl_registry::events::IrulesTopLevelDeclaration::Procedure { .. }
                )
            })
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
    /// and left its arity unknowable.
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

    /// Register the same physical procedure scope for ordinary and optlist
    /// definers, before either an inline walk or a deferred body is scheduled.
    fn push_proc_body_scope(&mut self, path: &[usize], name: &str, span: Span) -> Vec<usize> {
        let parent = super::scope::scope_at_mut(&mut self.result.global_scope, path)
            .expect("scope_path resolved when registering proc must still resolve");
        let mut child = super::types::Scope::new(super::types::ScopeKind::Proc, name.to_owned());
        child.body_span = Some(span);
        child
            .original_interpreter_source_domain
            .clone_from(&parent.original_interpreter_source_domain);
        child.naming_scope = parent
            .procs
            .values()
            .find(|proc| proc.body_span == span)
            .and_then(|proc| proc.source_name.as_ref())
            .and_then(crate::signature_scan::scope::SignatureSourceCommand::body_scope);
        let index = parent.children.len();
        parent.children.push(child);
        let mut child_path = path.to_vec();
        child_path.push(index);
        child_path
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
            params_tok,
            name_tok,
            params,
            statics,
            body_text,
            body_tok,
            ns_prefix,
        } = ctx;
        // One scope key for both the inline and the deferred path — a
        // divergence here would make the per-item and whole-file walks
        // disagree about which namespace the body runs in.
        let scope_name = scope_name_for_routine(resolved_name);
        let child_path = self.push_proc_body_scope(path, scope_name, body_span);
        self.record_original_vendor_variable_body(
            name_tok.span,
            body_tok.span,
            Some(params_tok.span),
            crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind::Procedure,
        );

        // Parameters become locals in the proc scope. Each param's
        // definition range is anchored to its *name* in the param-list
        // literal so go-to-definition / references / rename on
        // a formal parameter resolve to the parameter, not the proc name.
        // The spans are recovered from the raw param-list word token; any
        // param whose name can't be located falls back to the proc name
        // token.
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

        // Static variables are locals of the body too: declared once when the
        // procedure is defined, kept between calls, and never an argument.
        let static_names = statics.map_or_else(Vec::new, |(text, tok)| {
            let variables = static_variables(text, self.word_rules());
            let spans = param_name_spans_for_token(&self.source, tok);
            for (i, variable) in variables.iter().enumerate() {
                let span = spans.get(i).copied();
                self.define_var(&variable.name, tok, &child_path, false, span);
                // The definition reads the enclosing variable, so it is used
                // there whether or not the body reads its own copy.
                if variable.reads_enclosing
                    && let Some(span) = span
                {
                    let named = self.source.get(span.as_range()).unwrap_or_default();
                    let skipped = u32::try_from(named.len() - named.trim_start_matches('&').len())
                        .unwrap_or(0);
                    self.record_var_read(
                        &variable.name,
                        Span::new(span.start() + skipped, span.end()),
                        path,
                    );
                }
            }
            variables
                .into_iter()
                .map(|variable| variable.name)
                .collect()
        });

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
        let body_text: std::sync::Arc<str> = std::sync::Arc::from(body_text);
        if self.defer_proc_bodies {
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies.push(super::per_item::DeferredBody {
                resolved_input: Some(self.resolved_analysis_input()),
                body_text,
                body_tok,
                scope_path: child_path.clone(),
                is_method: false,
                oo_global_resolution: false,
                namespace: ns_prefix.to_string(),
                scope_name: scope_name.to_string(),
                params: params.to_vec(),
                seeded_variables: static_names,
                // Attached later by `fill_deferred_bodies` for bodies with a
                // fold candidate.
                command_trust: None,
                ensemble_targets: Vec::new(),
                prefixless_ensembles: Vec::new(),
                oo_defining_class: None,
                safe_interp_ctx,
            });
        } else {
            self.analyse_body(&body_text, body_tok, &child_path);
        }

        (self.last_comment, self.current_event) = (saved_comment, saved_event);
    }

    /// `tcl::OptProc name optlist body` — the `opt` package's automatic-
    /// option-parsing proc definer.
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
        let original_name = self.original_procedure_declaration_name(name_tok);
        let (source_name, qualified, simple) = self.procedure_reporting_names(
            original_name.as_ref(),
            name_tok,
            scope_path,
            &ns_prefix,
            &resolved_name,
        );
        let name_span = name_tok.span;
        let body_tok = arg_tokens[2];

        self.emit_w113_proc_shadows_builtin(
            &resolved_name,
            &qualified,
            name_span,
            source_name.as_ref(),
        );
        self.emit_w314_no_absolute_name(raw_name, name_span);

        let (real_params, opt_locals) = Self::opt_proc_params(&args[1], self.word_rules());

        let doc = self.procedure_definition_doc(args.get(2).map(String::as_str));

        // Combined list — the real `args` catch-all plus every
        // optlist-derived local — feeds hover/param-trait inference and
        // the body's own local-variable scope (never `ProcDef.params`,
        // which stays `[args]`-only for correct arity).
        let mut combined_params = real_params.clone();
        combined_params.extend(opt_locals.iter().cloned());

        let body_text = &args[2];
        let (param_traits, caller_frame_params, caller_frame_literals) =
            self.infer_proc_param_traits(&combined_params, body_text, body_tok.span);

        let proc = ProcDef {
            source_name,
            name: simple,
            qualified_name: qualified.clone(),
            params: real_params,
            // `tcl::OptProc`'s Tcl-level signature is always the single
            // catch-all `args`, derived above — a known list, not a computed
            // one.
            params_computed: false,
            formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                tcl_dialect::ParameterGrammar::Tcl,
            ),
            name_span,
            body_span: body_tok.span,
            doc,
            param_traits,
            caller_frame_params,
            caller_frame_literals,
        };

        // The optlist's locals remain distinct from native formal syntax;
        // this record retains only its genuine declaration name and report.
        self.retain_procedure_declaration_metadata(original_name.as_ref(), &proc, name_span);
        self.register_proc_definition(&qualified, &proc, name_span);
        let simple_key = proc.name.clone();
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.procs.insert(simple_key.clone(), proc);
        }

        if body_tok.kind == TokenType::Str {
            // The *resolved* name keys the scope — see
            // `scope_name_for_routine`; the raw written word would invent a
            // namespace out of a substitution.
            let scope_name = scope_name_for_routine(&resolved_name).to_string();
            let child_path = self.push_proc_body_scope(&path, &scope_name, body_tok.span);

            self.bind_opt_proc_locals(arg_tokens[1], name_tok, &child_path, &opt_locals);

            let saved_comment = std::mem::take(&mut self.last_comment);
            let body_text: std::sync::Arc<str> = std::sync::Arc::from(args[2].as_str());
            if self.defer_proc_bodies {
                let safe_interp_ctx = self.safe_interp_ctx_snapshot();
                self.deferred_bodies.push(super::per_item::DeferredBody {
                    resolved_input: Some(self.resolved_analysis_input()),
                    body_text,
                    body_tok,
                    scope_path: child_path.clone(),
                    is_method: false,
                    oo_global_resolution: false,
                    namespace: ns_prefix.clone(),
                    scope_name,
                    params: combined_params,
                    seeded_variables: Vec::new(),
                    // Attached later by `fill_deferred_bodies` for bodies
                    // with a fold candidate.
                    command_trust: None,
                    ensemble_targets: Vec::new(),
                    prefixless_ensembles: Vec::new(),
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
    /// opt_locals)`: the real, arity-relevant Tcl-level
    /// signature — always the single catch-all `args`, regardless of what
    /// `optlist` declares — and `optlist`'s own descriptors, dash-stripped
    /// to the LOCAL VARIABLE name `::tcl::OptKeyParse` actually binds at
    /// runtime (used only for local-variable binding in the body, never
    /// for arity). A pure text transform — no analyser state needed — kept
    /// out of [`Self::handle_opt_proc_command`] purely to stay within the
    /// line-count lint.
    fn opt_proc_params(
        optlist_text: &str,
        rules: tcl_syntax::word_rules::WordValueRules,
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
            parse_param_list(optlist_text, rules)
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

    /// One-hop lambda inspection from original list builders or checked Logical values.
    /// Element spans retain their defining source; unavailable receivers stay opaque.
    fn resolve_dynamic_apply_lambda(&self, arg_tok: Token) -> Option<Vec<(Token, String)>> {
        let one_hop = |word_tok: Token| -> Option<(Token, String)> {
            if word_tok.kind != TokenType::Var {
                let word_text = Analyser::source_slice(
                    &self.source,
                    word_tok.span.start() as usize,
                    word_tok.span.end() as usize,
                )?;
                return Some((word_tok, word_text.to_string()));
            }
            let (value, span) = self.lookup_logical_const_substitution(word_tok)?;
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
                let segmented = crate::segmenter::segment_commands_with_offset_and_config(
                    inner,
                    base,
                    self.file_lexer_config().nested(),
                );
                let [cmd] = segmented.as_slice() else {
                    return None;
                };
                // A lambda built word by word: the head is the command whose
                // result quotes each of its words as one element — the
                // registry's `BUILDS_COMMAND_PREFIX` reading, not a spelling.
                let original =
                    self.original_diagnostic_call_at(*cmd.argv.first()?, &cmd.argv[1..])?;
                if !original.with_schema(|schema| {
                    schema
                        .semantics
                        .traits
                        .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
                })? || original.words().arguments().len() != cmd.argv.len().checked_sub(1)?
                    || (0..original.words().arguments().len())
                        .any(|ordinal| original.written_index(ordinal) != Some(ordinal))
                {
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
                        // and the real `cleanup` call is missed.
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
            self.file_lexer_config().nested(),
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
        _scope_path: &[usize],
    ) -> bool {
        self.handle_apply_command_with_source_namespace(args, arg_tokens, None)
    }

    pub(super) fn handle_apply_command_with_source_namespace(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        original: Option<super::interp_visibility::OriginalInterpreterLambdaBody>,
    ) -> bool {
        // A literal braced lambda parses directly; a dynamic one (`apply
        // $lambda`, `apply [list {params} $body ns]`) gets one hop through
        // the constant-value lattice before giving up
        // — same downstream shape either way, so nothing past this point
        // needs to know which path supplied `elements`.
        let elements = if let Some(elements) = self.parse_apply_lambda_elements(args, arg_tokens) {
            elements
        } else {
            let Some(&arg_tok) = arg_tokens.first() else {
                return false;
            };
            let Some(elements) = self.resolve_dynamic_apply_lambda(arg_tok) else {
                return false;
            };
            elements
        };
        // When a source-selected lambda receipt is present, the token-derived
        // list projection must name that exact original body in the current
        // input. A cooked or shifted value cannot borrow this source owner.
        if let Some(original) = original.as_ref() {
            let (Some((parameter_token, _)), Some((body_token, _))) =
                (elements.first(), elements.get(1))
            else {
                return true;
            };
            if original
                .for_lambda_tokens(
                    &self.source,
                    &self.resolved_analysis_input(),
                    *parameter_token,
                    *body_token,
                )
                .is_none()
            {
                return true;
            }
        }
        let Some((params_tok, body_tok)) = self.apply_lambda_body_tokens(&elements) else {
            return true;
        };
        let params_text = elements[0].1.as_str();
        let body_text = elements[1].1.as_str();

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
        let source_facts = self.resolve_apply_lambda_source_facts(
            &elements, arg_tokens, original, body_tok, params_tok, body_ns,
        );
        let ApplyLambdaSourceFacts {
            body_ns,
            original_namespace,
            logical_formals,
            logical_namespace_source,
        } = source_facts;
        let (params, original_param_spans) = match logical_formals {
            Some((parameters, spans)) => (parameters, Some(spans)),
            None => (parse_param_list(params_text, self.word_rules()), None),
        };
        let body_text: std::sync::Arc<str> = std::sync::Arc::from(body_text);
        // Anonymous, but keyed by source position so two lambdas never collide
        // in `all_variables` (keyed `"<scope_name>::<var>"`).
        let scope_name = format!("apply@{}", arg_tokens[0].span.start());

        self.finish_apply_lambda_body(ApplyLambdaBody {
            params,
            original_param_spans,
            params_tok,
            body_text,
            body_tok,
            body_ns,
            scope_name,
            original_namespace,
            logical_namespace_source,
        })
    }

    fn apply_lambda_body_tokens(&self, elements: &[(Token, String)]) -> Option<(Token, Token)> {
        if elements.len() < 2 {
            return None;
        }
        let params_tok = elements[0].0;
        let body_tok = elements[1].0;
        if body_tok.kind != TokenType::Str
            || self.source.as_bytes().get(body_tok.span.start() as usize) != Some(&b'{')
        {
            return None;
        }
        Some((params_tok, body_tok))
    }

    fn resolve_apply_lambda_source_facts(
        &mut self,
        elements: &[(Token, String)],
        arg_tokens: &[Token],
        original: Option<super::interp_visibility::OriginalInterpreterLambdaBody>,
        body_tok: Token,
        params_tok: Token,
        body_ns: String,
    ) -> ApplyLambdaSourceFacts {
        let original_namespace = original.as_ref().and_then(|original| {
            original.for_body_token(&self.source, &self.resolved_analysis_input(), body_tok)
        });
        let logical_formals = original.as_ref().and_then(|original| {
            original
                .logical_formals_for_body_token(
                    &self.source,
                    &self.resolved_analysis_input(),
                    body_tok,
                    params_tok,
                )
                .and_then(|formals| {
                    Some((
                        formals
                            .parameters()
                            .iter()
                            .map(|parameter| ParamDef {
                                name: parameter.name.clone(),
                                has_default: parameter.default.is_some(),
                                default_value: parameter.default.clone(),
                            })
                            .collect::<Vec<_>>(),
                        formals
                            .name_sources()
                            .iter()
                            .map(crate::signature_scan::formal_parameters::OriginalSourceFormalName::source_span)
                            .collect::<Option<Vec<_>>>()?,
                    ))
                })
        });
        if let Some(original) = original
            .as_ref()
            .filter(|original| original.has_logical_source_namespace())
        {
            if let Some(field) = original.logical_namespace_field_for_body_token(
                &self.source,
                &self.resolved_analysis_input(),
                body_tok,
            ) && let Some((namespace, _)) = original_namespace.as_ref()
                && !namespace.is_root()
                && let Some(span) = field.original_field().source_span()
                && !span.is_empty()
            {
                self.result
                    .namespace_refs
                    .push(super::types::NamespaceRef::from_original(
                        Some(crate::signature_scan::scope::SignatureNamespaceScope::root(
                            Some(field.policy()),
                        )),
                        Some(field.policy()),
                        field.original_field().value(),
                        span,
                        0,
                        false,
                        namespace
                            .source_spelling(Some(field.policy()))
                            .unwrap_or_else(|| body_ns.clone()),
                    ));
            }
        } else {
            self.record_apply_namespace_reference(elements, &body_ns, arg_tokens);
        }
        let logical_namespace_source = original
            .filter(|original| {
                original_namespace.is_some() && original.has_logical_source_namespace()
            })
            .map(std::sync::Arc::new);
        let body_ns = original_namespace
            .as_ref()
            .and_then(|(namespace, domain)| {
                domain.as_ref().map_or_else(
                    || namespace.display(),
                    |domain| domain.reported_namespace(namespace),
                )
            })
            .unwrap_or(body_ns);
        ApplyLambdaSourceFacts {
            body_ns,
            original_namespace,
            logical_formals,
            logical_namespace_source,
        }
    }

    fn finish_apply_lambda_body(&mut self, apply: ApplyLambdaBody) -> bool {
        let ApplyLambdaBody {
            params,
            original_param_spans,
            params_tok,
            body_text,
            body_tok,
            body_ns,
            scope_name,
            original_namespace,
            logical_namespace_source,
        } = apply;
        let body_span = body_tok.span;
        // Record the namespace override for command-resolution lookups.
        self.result
            .namespace_overrides
            .push((body_span, body_ns.clone()));
        let child_path = super::per_item::reconstruct_proc_scope(
            &mut self.result.global_scope,
            &body_ns,
            &scope_name,
            super::types::ScopeKind::Proc,
        );
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &child_path)
        {
            scope.body_span = Some(body_span);
            scope.requires_original_lambda_source_context = true;
            if let Some((namespace, domain)) = original_namespace {
                scope.naming_scope = Some(namespace);
                scope.original_interpreter_source_domain = domain;
                scope.original_logical_lambda_namespace = logical_namespace_source;
            }
        }
        let param_spans = original_param_spans
            .unwrap_or_else(|| param_name_spans_for_token(&self.source, params_tok));
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
        let saved_comment = std::mem::take(&mut self.last_comment);
        if self.defer_proc_bodies {
            let safe_interp_ctx = self.safe_interp_ctx_snapshot();
            self.deferred_bodies.push(super::per_item::DeferredBody {
                resolved_input: Some(self.resolved_analysis_input()),
                body_text,
                body_tok,
                scope_path: child_path.clone(),
                is_method: false,
                oo_global_resolution: false,
                namespace: body_ns,
                scope_name,
                params,
                seeded_variables: Vec::new(),
                command_trust: None,
                ensemble_targets: Vec::new(),
                prefixless_ensembles: Vec::new(),
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
        self.handle_namespace_eval_command_at_invocation(
            args, arg_tokens, arg_single, scope_path, None,
        )
    }

    /// Source-only namespace geometry from the actual command point, when
    /// dispatch retains one. Direct compatibility calls keep their ordinary
    /// span-based lookup without inventing an invocation offset.
    pub(in crate::analyser) fn handle_namespace_eval_command_at_invocation(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        invocation_offset: Option<u32>,
    ) -> bool {
        // Implementation contract: naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        if args.len() < 2 {
            return false;
        }
        let ns_name = args[1].clone();
        // **W314** — an all-colon segment (`namespace eval :`) creates a
        // namespace no absolute path can spell.
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
        // nothing for that read.
        let body_tok = arg_tokens.get(2).copied();
        let body_span = body_tok
            .filter(|t| t.kind == TokenType::Str)
            .map(|t| t.span);
        let body_text = args.get(2).cloned();
        // `namespace eval $ns [list namespace unknown $handler]` — the
        // list-wrapped installer idiom `analyse_body`'s literal-`{...}`
        // -only gate below never sees.
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
        // helper the command head, `source`, `rename`, and
        // `oo::define`'s target word already use — one lattice answers "what
        // does this word name" everywhere, and its dominance requirement keeps
        // a branch-conditional binding out.
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
        self.record_resolved_namespace_reference(
            resolved_dynamic.as_deref(),
            arg_tokens.get(1),
            scope_path,
            invocation_offset,
        );
        let scope_name = match resolved_dynamic {
            Some(resolved) => resolved,
            None if crate::naming::is_dynamic_word(&ns_name) => match arg_tokens.get(1) {
                Some(tok) => self.mint_synthetic_offset_name("@dynns@", tok.span.start()),
                None => ns_name.clone(),
            },
            None => ns_name,
        };

        let naming_scope = arg_tokens
            .get(1)
            .and_then(|token| {
                let input = match invocation_offset {
                    Some(offset) => {
                        self.original_variable_operand_input_at_invocation(offset, token.span)
                    }
                    None => self.original_variable_operand_input(token.span),
                }?;
                self.declaration_namespace_scope(scope_path)?
                    .child_from_input(&input)
            })
            .or_else(|| {
                self.original_interp_namespace_source_scope(
                    invocation_offset?,
                    arg_tokens,
                    scope_path,
                )
            });
        let path = scope_path.to_vec();
        let child_scope_idx = {
            let mut child =
                super::types::Scope::new(super::types::ScopeKind::Namespace, scope_name);
            child.body_span = body_span;
            child.naming_scope = naming_scope;
            child.original_interpreter_source_domain =
                super::scope::scope_at(&self.result.global_scope, &path)
                    .and_then(|scope| scope.original_interpreter_source_domain.clone());
            // The written `NAME` word, so the outline can point its
            // `selectionRange` at the name rather than the whole body.
            // Recorded even for a dynamic `$ns` target: the
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
        // concatenation or not at all — never as its first
        // word alone, which invented an E002 on the trailing words'
        // arguments and lost every write they performed.
        if args.len() == 3
            && let (Some(name), Some(body)) = (arg_tokens.get(1), body_tok)
        {
            self.record_original_vendor_variable_body(
                name.span,
                body.span,
                None,
                crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind::Namespace,
            );
        }
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
        self.handle_interp_eval_command_original(args, arg_tokens, scope_path, None)
    }

    pub(super) fn handle_interp_eval_command_original(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        head: Option<Token>,
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
        // create ...]` binding — that key is already
        // fully qualified at bind time, so it's used as-is, not
        // requalified.
        let literal_path = !crate::naming::is_dynamic_word(&args[1]);
        let resolved_dynamic = (!literal_path)
            .then(|| self.resolve_dynamic_interp_path(arg_tokens.get(1).copied()))
            .flatten();
        let original_key =
            head.and_then(|head| self.original_interp_eval_state_key(head, arg_tokens));
        let key = original_key
            .or_else(|| resolved_dynamic.clone())
            .unwrap_or_else(|| self.qualified_interp_key(&args[1]));
        if literal_path || resolved_dynamic.is_some() {
            // Interpreter existence: evaluating into a
            // known-but-never-created child raises `could not find
            // interpreter` at run time.  Abstains when any interp operation
            // in the file used a dynamic, *unresolvable* path (existence
            // then unknowable).
            if !self.interpreters.contains_key(&key)
                && !self.dynamic_interp_ops
                && let Some(tok) = arg_tokens.get(1)
            {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        tcl_core_types::DiagCode::W140,
                        tok.span,
                        format!(
                            "interpreter '{}' is never created in this file — \
                         `interp eval` will raise `could not find interpreter`",
                            args[1]
                        ),
                        super::types::Severity::Warning,
                    ));
            }
        }
        // Multiple script words concatenate at run time (`Tcl_ConcatObj`),
        // so commands can span word boundaries — no per-word walk is sound.
        // Consume the command *without* walking, keeping the parent domain
        // clean (a fall-through would analyse the words in the parent
        // scope); W312 separately flags the shape.
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
        let visibility = head.and_then(|head| {
            self.original_interp_visibility_body(&key, body_tok, head, arg_tokens)
        });
        self.isolate_interp_eval_body(&key, resolved, &args[2], body_tok, scope_path, visibility);
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
    /// state*, such as the Tk widget/geometry hierarchy — can
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
        visibility: Option<super::SourceInterpreterVisibilitySnapshot>,
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
            // domain that never merges with its predecessor's definitions.
            let mut child =
                super::types::Scope::new(super::types::ScopeKind::Namespace, domain.clone());
            child.body_span = Some(body_tok.span);
            child.interpreter_source_body = super::types::InterpreterSourceBody::Independent;
            child.original_interpreter_source_domain = visibility
                .as_ref()
                .map(|snapshot| snapshot.reporting_domain(domain.clone()));
            child.naming_scope = visibility.as_ref().map(|_| {
                crate::signature_scan::scope::SignatureNamespaceScope::C(
                    tcl_core_types::ByteNamespacePath::root(),
                )
            });
            let Some(parent) = super::scope::scope_at_mut(&mut self.result.global_scope, &outer)
            else {
                return false;
            };
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = outer;
        child_path.push(child_scope_idx);

        // Each child source has an independent context, including a child
        // whose original declaration cannot be retained. The enclosing
        // child's snapshot must not leak into that unknown body.
        let outer_visibility = std::mem::take(&mut self.safe_interp_stack);
        if let Some(context) = visibility {
            self.safe_interp_stack.push(context);
        }
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
        self.safe_interp_stack = outer_visibility;
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
    /// Handles the `$handle eval { … }` spelling too: `cmd_name` is the
    /// literal, unsubstituted head text, so a `$`-prefixed handle only
    /// resolves through a tracked `set VAR [interp create ...]` binding
    /// ([`Self::resolve_dynamic_interp_path`]) — a handle sourced any
    /// other way (a proc parameter, a value read from elsewhere) still
    /// falls through untouched, the same conservative fallback used for
    /// an untracked head.
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
        self.handle_interp_handle_eval_command_original(
            cmd_name, args, arg_tokens, scope_path, None,
        )
    }

    pub(super) fn handle_interp_handle_eval_command_original(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        head: Option<Token>,
    ) -> bool {
        // registry-axis-ok: subcommands — a namespace eval wrapper's subcommand; until the resolved invocation's selected subcommand is read
        if args.len() != 2 || args[0] != "eval" {
            return false;
        }
        let Some(body_tok) = arg_tokens.get(1).copied() else {
            return false;
        };
        if let Some((key, visibility)) = head.and_then(|head| {
            self.original_interp_handle_visibility_body(body_tok, head, arg_tokens)
        }) {
            return self.isolate_interp_eval_body(
                &key,
                true,
                &args[1],
                body_tok,
                scope_path,
                Some(visibility),
            );
        }
        let key = if crate::naming::is_dynamic_word(cmd_name) {
            let Some(resolved) = self.resolve_dynamic_interp_path(head) else {
                return false;
            };
            resolved
        } else {
            self.qualified_interp_key(cmd_name)
        };
        if !self.interpreters.contains_key(&key) {
            return false;
        }
        // Compatibility reporting retains the tracked interpreter path, but
        // no genuine source receipt or visibility advice follows from it.
        self.isolate_interp_eval_body(&key, true, &args[1], body_tok, scope_path, None)
    }

    /// Qualify a literal `interp` path operand against the enclosing
    /// `interp eval` bodies on the walk stack: paths are relative to the
    /// *current* interpreter, so `interp create t` inside
    /// `interp eval s {…}` names `s t`.  A dynamic operand is returned
    /// key-normalised only (its stack context cannot make it literal).
    pub(super) fn qualified_interp_key(&self, path: &str) -> String {
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
            .map_or_else(String::new, |frame| {
                crate::naming::root_unrooted_key(&frame.domain)
            })
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

    /// Current original scalar syntax selects independently recorded interpreter-path advice.
    fn resolve_dynamic_interp_path(&self, token: Option<Token>) -> Option<String> {
        self.lookup_logical_interp_path(token?)
    }

    /// Resolve an `interp alias` path operand to the domain prefix its
    /// alias/target command name qualifies under: the
    /// plain-current-interpreter sentinel (`""`/`"{}"`) is the empty
    /// prefix; a literal word resolves via [`Self::qualified_interp_key`]
    /// and then [`Self::interp_domain_name`]; a dynamic word resolves
    /// ONLY through [`Self::resolve_dynamic_interp_path`] — anything else
    /// (an untracked dynamic word) returns `None`, aborting the whole
    /// cross-domain alias (unchanged conservative behaviour).
    fn resolve_alias_domain_prefix(&self, path: &str, token: Option<Token>) -> Option<String> {
        if matches!(path, "" | "{}") {
            // The empty path is *this* interpreter — which is the child when
            // the command is written inside a child's eval body, not always
            // the main one.
            return Some(self.current_interp_domain_prefix());
        }
        let key = if crate::naming::is_dynamic_word(path) {
            self.resolve_dynamic_interp_path(token)?
        } else {
            self.qualified_interp_key(path)
        };
        Some(crate::naming::root_unrooted_key(
            &self.interp_domain_name(&key),
        ))
    }

    /// Handle `interp create ?-safe? ?--? ?path?` — record the child
    /// interpreter's existence and safe state in the interpreter-domain
    /// map, read off the invocation's `InterpreterTransition::Create`: the
    /// registry resolver scans the options as Tcl does (the scan continues
    /// past the path, `-safe` and `--` match by prefix, and a shape Tcl
    /// rejects states no creation). A computed path makes interpreter
    /// existence unknowable file-wide; a missing path auto-generates a name
    /// this file cannot reference literally, so nothing is recorded.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpCreate`],
    /// with the transitions the call states over its source words.
    pub fn handle_interp_create_command(&mut self, transitions: &tcl_registry::StateTransitions) {
        if let Some((Some(subject), safe)) = created_interpreter(transitions) {
            let path = subject.literal().or_else(|| {
                subject
                    .native_bytes()
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
            });
            let Some(path) = path else {
                self.dynamic_interp_ops = true;
                return;
            };
            let state = super::state::InterpState {
                safe,
                ..Default::default()
            };
            let key = self.qualified_interp_key(path);
            self.interpreters.insert(key, state);
        }
    }

    /// Handle `interp delete ?path ...?` — remove the recorded state and
    /// bump the path's **epoch**, so a later re-creation is a fresh
    /// domain.
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

    /// Compatibility entry without original operands. It withdraws source
    /// visibility advice; the complete invocation entry retains hidden tokens
    /// separately from scoped visible slots.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpHide`].
    pub fn handle_interp_hide_command(&mut self, args: &[String]) {
        self.apply_interp_visibility_delta(args, true);
    }

    /// Compatibility entry without original operands. It withdraws source
    /// visibility advice rather than constructing hidden-token/name receipts.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpExpose`].
    pub fn handle_interp_expose_command(&mut self, args: &[String]) {
        self.apply_interp_visibility_delta(args, false);
    }

    /// Compatibility handlers without original operands cannot issue visibility.
    fn apply_interp_visibility_delta(&mut self, args: &[String], _hide: bool) {
        let Some(path) = args.get(1) else {
            return;
        };
        let key = self.qualified_interp_key(path);
        if let Some(state) = self.interpreters.get_mut(&key) {
            state.tainted = true;
            state.visibility = None;
        }
    }

    pub(super) fn handle_interp_hide_command_original(
        &mut self,
        args: &[String],
        head: Token,
        tokens: &[Token],
    ) {
        if let Some(path) = args.get(1) {
            self.apply_original_interp_visibility_delta(path, head, tokens, true);
        }
    }

    pub(super) fn handle_interp_expose_command_original(
        &mut self,
        args: &[String],
        head: Token,
        tokens: &[Token],
    ) {
        if let Some(path) = args.get(1) {
            self.apply_original_interp_visibility_delta(path, head, tokens, false);
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
    /// [`NamespaceRef`](super::types::NamespaceRef) at its own source span.
    /// Whole-word `ArgRole`s cannot express that — the
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
        if surface_admits(
            SpecSurface::TCL85_PLUS,
            Some(&self.analysis_context().context().authoring_query()),
        ) && let Some(token) = arg_tokens.get(1)
        {
            let config = self.file_lexer_config();
            let original = self
                .head_identities
                .original_written_name_input_at_span(token.span, config)
                .or_else(|| {
                    self.declaration_name_policy()
                        .and_then(|policy| {
                            self.head_identities.original_source_name_at_span(
                                token.span,
                                config,
                                self.word_rules(),
                                policy,
                            )
                        })
                        .map(|occurrence| {
                            crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                                occurrence.name_input().clone(),
                            )
                        })
                });
            if let Some(parent) = original {
                let here = self.command_resolution_namespace(scope_path);
                if let Some(entries) = self
                    .record_original_namespace_path_element_refs(&parent, token.span, scope_path)
                {
                    self.namespace_paths.insert(here, entries);
                } else {
                    // Byte names with no UTF-8 report cannot leave an older
                    // String path active. The original reference inventory is
                    // independent of this compatibility map.
                    self.namespace_paths.remove(&here);
                }
                return;
            }
            if self.declaration_name_policy().is_some() {
                self.result.namespace_path_computed.push(token.span);
                return;
            }
        }
        // The word arrives with its braces already stripped, so a text-only
        // dynamism scan mistakes `namespace path {$ns ::a}` for a computed
        // path and abstains from the whole command. Braces suppress
        // substitution, so the token kind is the authority.
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
            // from one that declares no path at all.
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
        if !surface_admits(
            SpecSurface::TCL85_PLUS,
            Some(&self.analysis_context().context().authoring_query()),
        ) {
            return;
        }
        let ns = self.command_resolution_namespace(scope_path);
        let Ok(elements) = self.word_rules().split_list(&args[1]) else {
            // A malformed list (an unbalanced brace) is not a path Tcl would
            // accept either; record nothing rather than half of one.
            return;
        };
        let entries: Vec<String> = elements.iter().map(ToString::to_string).collect();
        self.record_namespace_path_element_refs(
            &args[1],
            arg_tokens.get(1),
            braced,
            &ns,
            scope_path,
        );
        self.namespace_paths.insert(ns, entries);
    }

    fn record_original_namespace_path_element_refs(
        &mut self,
        parent: &crate::signature_scan::scope::SignatureSourceNameInput,
        parent_span: tcl_lexer::Span,
        scope_path: &[usize],
    ) -> Option<Vec<String>> {
        let Some(children) = parent.original_list_elements_with_source_spans() else {
            self.result.namespace_path_computed.push(parent_span);
            return None;
        };
        let context = self.declaration_namespace_scope(scope_path);
        let mut reports = Some(Vec::with_capacity(children.len()));
        for (input, extent) in children {
            let report = std::str::from_utf8(input.bytes()).ok();
            if let Some(value) = report {
                if let Some(reports) = &mut reports {
                    reports.push(value.to_owned());
                }
            } else {
                reports = None;
            }
            let span = extent.unwrap_or(parent_span);
            self.result.namespace_refs.push(
                super::types::NamespaceRef::from_original(
                    context.clone(),
                    Some(input.policy()),
                    report.unwrap_or(""),
                    span,
                    0,
                    false,
                    String::new(),
                )
                .with_original_input(Some(input)),
            );
        }
        reports
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
    /// the whole-word one already is — otherwise `namespace
    /// path {::$ns ::a}` records `::$ns` as a path entry but not as a
    /// reference, and the same function disagrees with itself.
    fn record_namespace_path_element_refs(
        &mut self,
        raw: &str,
        token: Option<&Token>,
        word_braced: bool,
        here: &str,
        scope_path: &[usize],
    ) {
        let Some(token) = token else { return };
        let Ok(decoded) = self.word_rules().split_list(raw) else {
            return;
        };
        let mut decoded = decoded.into_iter();
        let base = token.span.start() + u32::from(token.content_offset);
        let original_parent = self
            .head_identities
            .original_written_name_input_at_span(token.span, self.file_lexer_config());
        let mut scan = 0usize;
        let mut ordinal = 0usize;
        while let Ok(Some(el)) = tcl_syntax::list::find_element(raw, scan) {
            let Some(text) = decoded.next() else { break };
            let original_input = original_parent
                .as_ref()
                .and_then(|parent| parent.original_list_element(ordinal));
            ordinal += 1;
            let (start, end) = (el.value.start, el.value.end);
            if el.next <= scan {
                break;
            }
            scan = el.next;
            if text.is_empty() || tcl_syntax::naming::word_is_dynamic(&text, word_braced) {
                continue;
            }
            let Ok(start_u32) = u32::try_from(start) else {
                continue;
            };
            let Ok(end_u32) = u32::try_from(end) else {
                continue;
            };
            self.result.namespace_refs.push(
                super::types::NamespaceRef::from_original(
                    self.declaration_namespace_scope(scope_path),
                    self.declaration_name_policy(),
                    &text,
                    tcl_lexer::Span::new(base + start_u32, base + end_u32),
                    0,
                    false,
                    crate::naming::qualify(here, &text),
                )
                .with_original_input(original_input),
            );
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
        // it opens the same frame scope.  Any other shape falls
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
            // parameter, not stop at this frame.
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
        head: Option<u32>,
    ) {
        if args.len() < 2 {
            return;
        }
        // `create` operates on the *current* namespace, with no explicit name
        // argument; `configure NAME ...` (the ensemble side of the
        // `namespace ensemble configure -map` gap) names an *existing* ensemble
        // command explicitly, resolved like any other command reference.
        // Both share the same `-command`/`-map`/`-subcommands` option set.
        // registry-axis-ok: subcommands — namespace ensemble create and configure; until the resolved invocation's selected subcommand is read
        let is_create = args[1] == "create";
        // registry-axis-ok: subcommands — namespace ensemble create and configure; until the resolved invocation's selected subcommand is read
        let is_configure = args[1] == "configure";
        if !is_create && !is_configure {
            return;
        }
        // `namespace ensemble create` names the ensemble after the namespace
        // the command *resolves in*, not the lexically enclosing one: run
        // inside the body of `proc ::tk::foo {} {...}` declared at top level,
        // the current namespace is `::tk`, so the ensemble is `::tk` — the
        // purely lexical namespace walk skips proc scopes and homes it
        // to `::` instead, losing every `<ns> sub` call site.
        let ns = self.command_resolution_namespace(scope_path);
        let ns_prefix = ns.clone();

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

        // The ensemble's own resolved command name:
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
                    // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
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
            .map_or_default(|(spec, sub)| {
                self.analysis_context()
                    .context()
                    .available_sub_option_specs(spec, sub)
            });

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
                // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
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
                // this redirect to the target.
                // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
                "-map" => {
                    if let (Some(value), Some(tok)) = (value, value_tok)
                        && let Some((text, text_tok)) = self.ensemble_list_literal(value, tok)
                    {
                        self.record_ensemble_map_targets(
                            &text,
                            text_tok,
                            scope_path,
                            ensemble_key.as_deref(),
                            head,
                        );
                    }
                }
                // `-subcommands {a b c}` — each subcommand `a` dispatches to
                // the command `<ns>::a` in the ensemble's namespace.
                // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
                "-subcommands" => {
                    if let (Some(value), Some(tok)) = (value, value_tok)
                        && let Some((text, text_tok)) = self.ensemble_list_literal(value, tok)
                    {
                        self.record_ensemble_subcommands(
                            &text,
                            text_tok,
                            &ns_prefix,
                            ensemble_key.as_deref(),
                            head,
                        );
                    }
                }
                // registry-axis-ok: options — namespace ensemble configuration options; until the ensemble option table drives the walk
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
            self.result.prefixless_ensembles.insert(key.to_owned());
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
    /// needs; anything else abstains.
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
    /// `{...}` value of its own: `namespace ensemble
    /// configure tk -map [dict merge [namespace ensemble configure tk -map]
    /// {systray ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]`.
    /// `EXISTING` (whatever it evaluates to — typically a self-referential
    /// query of the ensemble's own current map) is left unknown, but the
    /// spliced literal tail is a statically known fact regardless of what
    /// `EXISTING` is.
    ///
    /// Deliberately narrow: exactly `dict merge ARG {literal}` (2 dict-merge operands, the
    /// second a literal word) — does not recognise `dict set`/`dict
    /// replace`/`concat`/a list-building helper proc, or a `dict merge`
    /// with more than 2 operands. A documented scope boundary, not an
    /// oversight (no attested real-world instance of those forms).
    fn dict_merge_literal_tail(&self, value_tok: Token) -> Option<(String, Token)> {
        if value_tok.kind != TokenType::Cmd {
            return None;
        }
        let config = self.file_lexer_config();
        let sm = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let descended = descend_token(&sm, value_tok, config);
        let segs = segments_from_tree(descended.tree(), &sm);
        let [seg] = segs.as_slice() else { return None };
        // registry-axis-ok: subcommands — a dict merge value shape; until the resolved invocation's selected subcommand is read
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
    /// that no longer line up in pairs (a naive `split_whitespace` turns
    /// `-map {go {source b.tcl}}` into `["go", "{source", "b.tcl}"]`, an
    /// unmatched three-way split that silently drops the pairing entirely).
    /// Each element's span is
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
    /// redirect (`None` when the ensemble's own name
    /// couldn't be resolved statically, matching every other guard in this
    /// handler), *and* — when the paired subcommand word is also static —
    /// file the same `sub -> target` fact under `ensemble_key` in
    /// [`AnalysisResult::ensemble_subcommand_targets`] so `definition`/
    /// `hover`/`references` can resolve a `<ensemble> <subcommand>` call site
    /// — a distinct field from `ensemble_command_maps`
    /// above, serving navigation rather than the safe-interpreter gate.
    ///
    /// The navigation entry is tagged
    /// [`EnsembleSubcommandProvenance::Map`](crate::signature_scan::types::EnsembleSubcommandProvenance::Map):
    /// a `-map` key is an arbitrary name, so a consumer that
    /// *rewrites* the subcommand word — rename — must leave it alone, unlike
    /// the `-subcommands` sibling below whose entry is the target's own tail.
    ///
    /// A `-map` target is a command name **or a command prefix** in real
    /// Tcl (tclsh 8.6.14-verified: `-map {go {string length}}` dispatches
    /// `myens go hello` to `string length hello`) — the command actually
    /// invoked is the prefix's *head*; the rest are baked-in arguments, not
    /// part of the command's identity. [`Self::command_prefix_head`]
    /// extracts just that head (recording the whole multi-word target text
    /// verbatim, or worse, splitting it on whitespace before pairing it with
    /// its subcommand at all, means
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
    /// leftover redirect to `source`).  Merging new pairs into the existing
    /// cached map instead of replacing it would leave a subcommand a later
    /// `-map` dropped still resolving to its stale target, a false-positive
    /// risk. The cached
    /// map for `ensemble_key` is cleared before any of its new pairs are
    /// inserted.
    ///
    /// The reporting map supports navigation only. A selector token does not
    /// own the mapped target's original source/value producer and cannot issue
    /// an interpreter visibility subject or suppress invocation effects.
    fn record_ensemble_map_targets(
        &mut self,
        list_text: &str,
        tok: Token,
        scope_path: &[usize],
        ensemble_key: Option<&str>,
        command_head: Option<u32>,
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
            self.push_command_reference_with_policy(
                head,
                head_span,
                resolved,
                None,
                false,
                Self::ensemble_target_lookup(command_head, tok),
            );
        }
    }

    /// A target word of a configuring command with a known head is looked up
    /// at the ensemble's dispatches; without one it stays deferred.
    fn ensemble_target_lookup(
        head: Option<u32>,
        container: Token,
    ) -> crate::signature_scan::types::SignatureCommandLookup {
        head.map_or(
            crate::signature_scan::types::SignatureCommandLookup::DeferredReference,
            |invocation_offset| {
                crate::signature_scan::types::SignatureCommandLookup::EnsembleTarget {
                    invocation_offset,
                    container_start: container.span.start(),
                    container_end: container.span.end(),
                }
            },
        )
    }

    /// Record each `-subcommands` name as a reference to the command
    /// `<ns>::<name>` the ensemble maps it to, and file the same
    /// `subcommand → resolved target` fact under `ensemble_key` in
    /// [`AnalysisResult::ensemble_subcommand_targets`] — the `-subcommands`
    /// sibling of [`Self::record_ensemble_map_targets`] (same
    /// one-directional-only gap, same shape).
    ///
    /// Tagged
    /// [`EnsembleSubcommandProvenance::Subcommands`](crate::signature_scan::types::EnsembleSubcommandProvenance::Subcommands):
    /// here the subcommand word *is* the target's tail — the
    /// ensemble derives `<ns>::<name>` from it — so renaming the target must
    /// rewrite the entry and the dispatch word with it.
    fn record_ensemble_subcommands(
        &mut self,
        list_text: &str,
        tok: Token,
        ns_prefix: &str,
        ensemble_key: Option<&str>,
        command_head: Option<u32>,
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
            self.push_command_reference_with_policy(
                elem,
                span,
                resolved,
                None,
                false,
                Self::ensemble_target_lookup(command_head, tok),
            );
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
    pub(super) fn define_vars_from_list(
        &mut self,
        var_list_text: &str,
        tok: Token,
        scope_path: &[usize],
    ) {
        let Ok(names) = self.word_rules().split_list(var_list_text) else {
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
            // The element is a written variable name of its own, declared
            // here as a whole-word receiver is.
            self.record_original_variable_receiver(
                Span::new(start, end),
                scope_path,
                tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
                false,
            );
            pos = element.next;
        }
    }

    /// Handle `foreach varList1 list1 ?varList2 list2 ...? body` (and the
    /// `foreach_in_collection` dialect variant, whose spec carries the same
    /// hook).
    ///
    /// Defines every `varListN` the registry's roles name (its repeated
    /// `LoopVarList` layout covers an unlimited number of `varList`/`list`
    /// pairs, tclsh 8.6/9.0-verified) in the active scope, through the one
    /// binder [`Self::handle_var_binding_command`], then recurses into the
    /// body so vars defined inside the loop land in the enclosing scope. What
    /// stays here is the literal-iteration simulation, the page's residue.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Foreach`].
    pub fn handle_foreach_command(
        &mut self,
        _cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if args.len() < 3 {
            return false;
        }
        // A single-pair `foreach VAR {literal list} { rename ::$VAR ... ;
        // proc ::$VAR ... }` loop over a fully literal list (the real
        // `tk/library/accessibility.tcl` rename-away-and-reinstall idiom)
        // binds `VAR` to a *different* value each
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
            .then(|| {
                Self::literal_foreach_binding(&args[0], &args[1], list_braced, self.word_rules())
            })
            .flatten();
        if let Some((var, elements)) = &literal_binding
            && let Some(first) = elements.first()
        {
            self.set_const_string(var, first.clone(), arg_tokens[1].span, scope_path);
        }
        // The body is always the last argument; recurse so vars
        // defined inside the loop land in the enclosing scope.
        if let (Some(body_text), Some(body_tok)) = (args.last(), arg_tokens.last().copied()) {
            self.control_flow_body_depth += 1;
            self.analyse_body(body_text, body_tok, scope_path);
            if let Some((var, elements)) = &literal_binding {
                self.simulate_remaining_foreach_iterations(var, elements, body_tok, scope_path);
            }
            self.control_flow_body_depth -= 1;
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
    /// made one odd element abstain from the whole simulation.
    /// tclsh-proof (8.6.16 / 9.0.4):
    ///   foreach n {aa {$b} cc} { puts $n }   ->  aa / $b / cc
    /// i.e. the loop runs three times over literal values, and no read of
    /// `b` happens at any point.
    fn literal_foreach_binding(
        var_list_text: &str,
        list_text: &str,
        list_braced: bool,
        rules: tcl_syntax::word_rules::WordValueRules,
    ) -> Option<(String, Vec<String>)> {
        let vars = rules.split_list(var_list_text).ok()?;
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
        // iteration. A malformed list (unbalanced
        // brace/quote) is not a valid `foreach` value at all — real Tcl
        // errors on it — so `split_list`'s `Err` means abstain entirely.
        let elements: Vec<String> = rules
            .split_list(list_text)
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
    /// *different* definition (`proc` / `rename`, and `oo::define`'s class
    /// target). Which commands those are is
    /// registry data, never a name list here — a newly-stamped spec joins
    /// the simulation with no edit to this walker.
    ///
    /// Every *other* command in the body keeps the single evaluation the
    /// normal walk above already gave it — deliberately narrow: this does
    /// not generally re-walk
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
        let config = self.file_lexer_config();
        let segs: Vec<SegmentedCommand> = {
            let sm = Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
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
        for (index, element) in elements.iter().enumerate().skip(1) {
            let _reporting_iteration = self
                .finite_foreach_names
                .advance(var, body_tok, scope_path, index, element);
            self.set_const_string(var, element.clone(), body_tok.span, scope_path);
            for (seg, hook) in &installers {
                let args = seg.args();
                let arg_tokens = seg.arg_tokens();
                let arg_single = seg.arg_single_token();
                match hook {
                    Hook::Proc => {
                        self.handle_proc_command_at(
                            seg.name(),
                            args,
                            arg_tokens,
                            arg_single,
                            scope_path,
                            seg.argv[0],
                        );
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
                        if let Some(cmd_tok) = seg.argv.first().copied() {
                            self.handle_oo_objdefine_at(
                                cmd_tok, args, arg_tokens, arg_single, scope_path,
                            );
                        }
                    }
                    // Unreachable for a spec carrying the trait —
                    // `installer_hook_is_redispatched` pins that every one
                    // of them lands on an arm above, so a newly-stamped
                    // spec cannot silently promise a re-dispatch this match
                    // never delivers.
                    _ => {}
                }
            }
        }
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
    ///    block.  See [`crate::segmenter::flatten_clause_list_elements_with_config`]
    ///    for how that braced form is split.
    ///
    /// Bodies that are literally ``-`` are fall-through markers
    /// (the next arm's body fires) and are skipped — recursing
    /// into the literal ``-`` would produce a useless command.
    ///
    /// `-regexp` pattern recording: each non-`default`
    /// pattern arm is recorded as a `RegexPattern` with
    /// ``command = "switch"`` when its token is a literal
    /// (`Esc` / `Str`) or a `Var` token whose original substitution resolves
    /// through checked Logical declaration and value provenance.  Command-substitution patterns
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
        let Some(registry) = self.registry.as_deref() else {
            return true;
        };
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Some((case, invocation)) = registry.case_invocation(
            cmd_name,
            &arg_refs,
            Some(self.analysis_context().context().authoring_query()),
        ) else {
            // Preserve malformed-switch handling while allowing valid
            // subjectless Expect clause lists through the registry resolver.
            return args.len() >= 2;
        };
        let invocation_is_regexp = invocation.mode == tcl_registry::spec::CaseMatchMode::Regexp;
        if let Some(i) = invocation.clause_list_index {
            // Form 2 — single braced body containing all pairs.
            let body_text = args[i].clone();
            let Some(body_tok) = arg_tokens.get(i).copied() else {
                return true;
            };
            let clauses = crate::segmenter::flatten_case_list_clauses(
                &self.source,
                &body_text,
                body_tok,
                &case,
                self.lexer_config(),
            );
            let shape = tcl_syntax::case_list::CaseListShape {
                clause_flags: case.clause_flags,
                clause_value_flags: case.clause_value_flags,
            };
            let clause_regexp: Vec<bool> =
                tcl_syntax::case_list::split_case_list(&body_text, &shape)
                    .iter()
                    .map(|clause| {
                        clause.flags.iter().any(|flag| {
                            body_text
                                .get(flag.start..flag.end)
                                .and_then(|word| shape.resolve_flag(word))
                                == case.clause_regex_flag
                        })
                    })
                    .collect();
            let clause_count = clauses.len();
            for (clause_index, ((pat_text, pat_tok), (body_text, body_tok))) in
                clauses.iter().enumerate()
            {
                if invocation_is_regexp || clause_regexp.get(clause_index) == Some(&true) {
                    self.record_switch_regexp_pattern(
                        cmd_name,
                        &case,
                        pat_text,
                        *pat_tok,
                        (clause_index, clause_count),
                        scope_path,
                    );
                }
                if case.fallthrough_body != Some(body_text.as_str()) {
                    self.analyse_control_flow_body(body_text, *body_tok, scope_path);
                }
            }
        } else if let Some(i) = invocation.inline_clause_start {
            // Form 1 — pattern/body pairs inline in args/arg_tokens.
            let Some(clauses) = case.inline_clauses(&arg_refs, i) else {
                return true;
            };
            let clause_count = clauses.len();
            for (clause_index, clause) in clauses.iter().enumerate() {
                let pattern_index = clause.pattern_index;
                let body_index = clause.body_index;
                if (invocation_is_regexp
                    || clause.mode == tcl_registry::spec::CaseMatchMode::Regexp)
                    && let Some(pat_tok) = arg_tokens.get(pattern_index).copied()
                {
                    self.record_switch_regexp_pattern(
                        cmd_name,
                        &case,
                        &args[pattern_index],
                        pat_tok,
                        (clause_index, clause_count),
                        scope_path,
                    );
                }
                let Some(body_index) = body_index else {
                    continue;
                };
                let body_text = &args[body_index];
                if let Some(body_tok) = arg_tokens.get(body_index).copied()
                    && case.fallthrough_body != Some(body_text.as_str())
                {
                    self.analyse_control_flow_body(body_text, body_tok, scope_path);
                }
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
        case: &tcl_registry::spec::CaseListSpec,
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

    /// Bind the loop and output variables a call's argument roles name — the
    /// one binder behind every command that binds a variable by position,
    /// read off the registry's roles and never a per-command layout:
    ///
    /// - an [`tcl_registry::ArgRole::LoopVarList`] word is a Tcl list of
    ///   names bound before a body runs — `foreach` / `lmap` var lists,
    ///   `dict for` / `dict map` / `array for` key-value pairs, `dict
    ///   update`'s locals — each name defined at its own element's span
    ///   ([`Self::define_vars_from_list`]);
    /// - an [`tcl_registry::ArgRole::VarWrite`] word names a variable the
    ///   command writes — `lassign`, `scan`, `regexp`, `regsub`, `gets`,
    ///   `binary scan`, `vwait`, `incr`, `append`, `lappend`, `dict set`,
    ///   `info default`, … — bound with `warn_if_unused = false`: like
    ///   `catch`'s result variable the binding is a documented side effect,
    ///   and a read-modify-write command (`incr`, `append`, `lappend`) reads
    ///   the target before it writes it.
    ///
    /// Only a word that names a static variable binds: one that substitutes
    /// nothing, or (for a written variable) an element of an array whose name
    /// is written literally — `incr hits($word)` writes an element of `hits`
    /// whatever the key. A computed name (`scan $s $fmt $dyn`, `foreach
    /// $names …`) names nothing statically.
    ///
    /// The roles are read over the words' spellings, a computed word taking
    /// one position and never standing for an option (the reading the editor
    /// has always given a call), under the invocation the walk's context
    /// proves. Two trait families opt out:
    ///
    /// - [`tcl_registry::Traits::CREATES_SCOPE_ALIAS`] (`global` /
    ///   `variable` / `upvar`): the alias an invocation states is
    ///   [`Self::apply_state_transitions`]'s — tail-stripping (`global
    ///   ::ns::v` binds the local alias `v`, not the qualified name) and
    ///   name/value pairing are the resolver's, which a flat `VarWrite` walk
    ///   would get wrong. The opt-out reads the command's trait only:
    ///   `SubCommand::creates_scope_alias` is not folded into the
    ///   invocation's traits, because `dict update`, `dict with` and `my
    ///   variable` carry it for aliases no `VariableCellAliasTransition`
    ///   states — this binder is what binds their names. So `namespace
    ///   upvar`, the one flagged subcommand whose aliases the resolver does
    ///   state, has its strided `VarWrite` locals bound here as well as
    ///   there: the same name at the same word, which [`Self::define_var`]
    ///   re-defines idempotently, and the link the alias set is kept.
    /// - [`tcl_registry::Traits::DESTROYS_VARIABLE`] (`unset`): its
    ///   `VarWrite` role marks a *removal* target (an SSA def that kills the
    ///   value), not a binding to record.
    ///
    /// A name [`Self::bind_value_word_assignment`] also binds (`set`'s) is
    /// bound twice for one word: [`Self::define_var`]'s re-definition path is
    /// idempotent for the same token span, and the assignment's
    /// `warn_if_unused = true` escalates this binding's `false`.
    ///
    /// Void-returning (self-guards on the role set) so it composes with the
    /// other side-effect handlers — `regexp` / `regsub` also feed
    /// `handle_regex_pattern_capture`, which must still run.
    pub fn handle_var_binding_command(
        &mut self,
        command_token: Token,
        argument_tokens: &[Token],
        scope_path: &[usize],
    ) {
        use tcl_registry::arg_role::ArgRole;
        let Some(original) = self.original_diagnostic_call_at(command_token, argument_tokens)
        else {
            return;
        };
        let Some((Some(roles), appends_from)) = original.with_schema(|call| {
            let opted_out = call.semantics.traits.intersects(
                tcl_registry::Traits::CREATES_SCOPE_ALIAS | tcl_registry::Traits::DESTROYS_VARIABLE,
            );
            let appends_from = match call.semantics.var_elements_effect {
                Some(tcl_registry::VarElementsEffect::AppendsListElements { values_from }) => {
                    Some(call.semantics.argument_offset + usize::from(values_from))
                }
                _ => None,
            };
            (
                (!opted_out).then(|| call.arg_roles()).flatten(),
                appends_from,
            )
        }) else {
            return;
        };
        let Some((arguments, tokens, _single)) = original.source_arguments() else {
            return;
        };
        let args = arguments.as_slice();
        let arg_tokens = tokens.as_slice();
        // A list append to the package search path is a new directory per
        // word (`lappend auto_path DIR…`).
        if let (Some(values_from), Some(written)) =
            (appends_from, role_position(&roles, ArgRole::VarWrite))
        {
            self.record_search_path_write(
                args,
                arg_tokens,
                written,
                values_from..args.len(),
                super::types::AutoPathForm::Append,
            );
        }
        for (index, role) in roles {
            let (Some(text), Some(&tok)) = (args.get(index), arg_tokens.get(index)) else {
                continue;
            };
            match role {
                ArgRole::LoopVarList if !word_substitutes(text, tok) => {
                    self.define_vars_from_list(text, tok, scope_path);
                }
                ArgRole::VarWrite if names_static_variable(text, tok) => {
                    self.define_var(text, tok, scope_path, false, None);
                }
                _ => {}
            }
        }
    }

    // Registry clause plans supply body depths and per-clause variable bindings.
    // Selected handler bodies retain conditional control flow for name mutations.

    /// The fixed cell a frame-crossing alias's `otherVar` word names, or
    /// `None` when it names a frame-relative variable with no stable path.
    ///
    /// `otherVar` normally names a variable in *another frame*, which has no
    /// namespace path to link to — a bare `x` at level 1 is whatever local
    /// the caller happens to have. Two spellings escape that and name one
    /// fixed cell whatever the call depth (without them `upvar
    /// ::tk::FocusGrab($index) data` leaves the array completely
    /// unregistered):
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
            // The level word's grammar is the analysed release's (`uplevel 010`
            // is 8 frames up on 8.6 and 10 on 9.0). With no registry loaded
            // there is no release to name, so `parse` abstains with `Dynamic`
            // wherever the releases disagree — the direction every consumer of
            // this level already treats conservatively.
            self.registry
                .as_ref()
                .map_or_else(
                    || FrameLevel::parse(&args[0]),
                    |r| FrameLevel::parse_in(&args[0], r),
                )
                .unwrap_or(FrameLevel::Dynamic)
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
                    let original_namespace = self
                        .original_qualified_variable_input(other_tok.span, false)
                        .and_then(|input| {
                            let absolute = input
                                .policy()
                                .recipe()
                                .variable_root_input(input.bytes())
                                .qualification()
                                == tcl_syntax::naming::NativeNameQualification::Absolute;
                            let global = if pair_start == 1 {
                                self.original_variable_operand_input(arg_tokens[0].span)
                                    .and_then(|level| {
                                        let dialect = tcl_registry::InvocationDialect::of_profile(
                                            self.profile,
                                        );
                                        FrameLevel::parse_native_bytes(level.bytes(), dialect)?
                                            .map(FrameLevel::is_global_frame)
                                    })
                                    == Some(true)
                            } else {
                                false
                            };
                            (absolute || global)
                                .then(|| self.original_variable_root_namespace())
                                .flatten()
                        });
                    self.set_var_link_target(
                        &args[i],
                        scope_path,
                        super::scope::VariableLinkTarget {
                            target: target.clone(),
                            target_span: other_tok.span,
                            local_span: arg_tokens[i].span,
                            original_target_namespace: original_namespace,
                            original_target_receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::NamespaceAlias,
                        },
                    );
                    // The fixed cell itself gets a definition at the
                    // `otherVar` word: `upvar ::tk::FocusGrab($index) data`
                    // is the
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
        let base = crate::naming::split_element_ref(other).map_or(other, |(base, _)| base);
        if base.is_empty() || crate::naming::is_dynamic_word(base) {
            return None;
        }
        (tcl_syntax::naming::is_absolute(base) || level.is_global_frame())
            .then(|| crate::naming::qualify("::", base))
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
        let Some((value, value_span)) = self
            .lookup_logical_const_string_at(&args[1], arg_tokens[1].span)
            .map(|(value, span)| (value.to_owned(), span))
        else {
            return;
        };
        self.retain_readonly_dictionary_keys(&value, value_span, arg_tokens[1].span, scope_path);
    }

    /// Handle `interp alias {} ALIAS {} TARGET ?ARG ...?` —
    /// records the alias for later argument-role resolution.
    ///
    /// Reads the registry's one command-table transition vocabulary
    /// (`CommandBindingTransition`) rather than destructuring
    /// `interp alias`'s word layout here. `offset` is the command token's
    /// start, recorded in [`Analyser::alias_offsets`] for the same-file
    /// arity resolver's top-level order gate.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::InterpAlias`] (stamped on
    /// `interp`'s `alias` subcommand); `args[0]` is still the subcommand
    /// word, which is what selects the alias descriptor.
    pub fn handle_interp_alias(&mut self, args: &[String], scope_path: &[usize], offset: u32) {
        self.handle_interp_alias_with_original_arguments(args, &[], scope_path, offset);
    }

    pub(super) fn handle_interp_alias_with_original_arguments(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        _scope_path: &[usize],
        offset: u32,
    ) {
        // What this call did to the command table is registry data, read
        // through the one transition vocabulary. The
        // interpreter *paths* are not — a path may be a tracked
        // `set VAR [interp create …]` binding this analyser resolves
        // itself — so the cross-domain arm below still reads the path
        // words; the alias and target *command* names come from the facts.
        let registry = self.registry.as_deref();
        let facts: Vec<tcl_registry::CommandBindingTransition> =
            registry.map_or_else(Vec::new, |registry| {
                command_table_transitions(registry, "interp", args)
                    .command_bindings()
                    .cloned()
                    .collect()
            });
        if let Some(tcl_registry::CommandBindingTransition::Delete { interpreter, name }) =
            facts.first()
            && interpreter.as_ref().is_none_or(is_current_interpreter)
            && let Some(deleted) = name.literal()
        {
            // Deleting an alias destroys the command object, so every
            // `namespace import` edge pointing at it dies too.
            // An empty srcPath means the interpreter this command *runs in*,
            // so inside a child's eval body the deletion homes under that
            // child's domain rather than the parent's command table.
            let deleted = format!(
                "{}{}",
                self.current_interp_domain_prefix(),
                crate::naming::normalise_qualified_name(deleted)
            );
            self.result
                .destroyed_commands
                .insert(deleted.clone(), offset);
            self.deleted_commands.insert(deleted, offset);
            return;
        }
        let Some(tcl_registry::CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        }) = facts.first()
        else {
            return;
        };
        if *target_lookup != tcl_registry::AliasTargetLookup::Global {
            // Caller-dependent aliases are represented by the source kernel.
            // This legacy global-only presentation cannot invent a target.
            return;
        }
        // The `interp alias {} A {} B` fast path homes both names at the
        // global root, which is only right in the main interpreter: inside a
        // child's eval body `{}` names *that child*. There, fall through to
        // the domain-aware branch below, which qualifies both sides through
        // `resolve_alias_domain_prefix` exactly as the explicit-path form
        // already does.
        if self.interp_path_stack.is_empty()
            && is_current_interpreter(source_interpreter)
            && is_current_interpreter(target_interpreter)
            && let (Some(alias_name), Some(target_cmd)) = (alias.literal(), target.literal())
        {
            let qualified = if alias_name.is_empty() {
                String::new()
            } else {
                crate::naming::normalise_qualified_name(alias_name)
            };
            let prepended = arguments
                .iter()
                .filter_map(tcl_registry::TransitionSubject::literal)
                .map(str::to_owned)
                .collect();
            self.record_interp_alias(
                qualified,
                self.registry
                    .as_deref()
                    .and_then(tcl_registry::CommandRegistry::profile)
                    .and_then(|profile| {
                        tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
                    })
                    .and_then(|policy| {
                        crate::signature_scan::scope::SignatureSourceCommand::alias_at_root(
                            policy, alias_name,
                        )
                    }),
                crate::signature_scan::types::SignatureCommandAliasTarget::WrittenGlobal(
                    target_cmd.to_owned(),
                ),
                prepended,
                offset,
            );
            return;
        }
        // Cross-domain alias: `interp alias PATH name
        // TPATH target ?arg…?` deliberately crosses interpreter domains —
        // the alias `name` becomes callable in the *source* interpreter,
        // running `target` resolved in the *target* interpreter.  With
        // literal paths both sides home under their `@interp@` domains, so
        // calls of the alias inside the child's eval bodies resolve to the
        // target through the ordinary alias link machinery. A path need
        // not itself be a source literal: it also
        // resolves through a tracked `set VAR [interp create ...]`
        // binding — only the alias/target *command* names stay hard
        // literal requirements, which is exactly what the registry's typed
        // subjects already say.
        let (Some(alias_name), Some(target_cmd)) = (alias.literal(), target.literal()) else {
            return;
        };
        let (Some(src_path), Some(target_path)) = (args.get(1), args.get(3)) else {
            return;
        };
        if let Some(src_prefix) =
            self.resolve_alias_domain_prefix(src_path, arg_tokens.get(1).copied())
            && let Some(target_prefix) =
                self.resolve_alias_domain_prefix(target_path, arg_tokens.get(3).copied())
            && !(src_prefix.is_empty() && target_prefix.is_empty())
        {
            let qualified =
                crate::signature_scan::types::SignatureCommandAliasTarget::in_interpreter(
                    &src_prefix,
                    alias_name,
                )
                .constructed_key()
                .into_owned();
            let target = crate::signature_scan::types::SignatureCommandAliasTarget::in_interpreter(
                &target_prefix,
                target_cmd,
            );
            let prepended: Vec<String> = args[5..].to_vec();
            self.record_interp_alias(qualified, None, target, prepended, offset);
        }
    }

    /// Record one resolved `interp alias` fact into the three alias
    /// tables (shared by the current-interp and cross-domain forms).
    fn record_interp_alias(
        &mut self,
        qualified: String,
        source_name: Option<crate::signature_scan::scope::SignatureSourceCommand>,
        target_cmd: crate::signature_scan::types::SignatureCommandAliasTarget,
        prepended: Vec<String>,
        offset: u32,
    ) {
        // Binding a second name onto `source` (or another external-unit
        // loader) means calls through that name never reach the `Source` hook,
        // so the files they pull in are invisible — see
        // [`Self::note_external_unit_command_moved`].
        let policy = self
            .registry
            .as_deref()
            .and_then(tcl_registry::CommandRegistry::profile)
            .and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
            });
        self.command_aliases.remove(&qualified);
        if let Some(target_key) = target_cmd
            .checked_global_key(policy)
            .map(std::borrow::Cow::into_owned)
        {
            self.note_external_unit_command_moved(&target_key);
            self.command_aliases
                .insert(qualified.clone(), (target_key, prepended.clone()));
        }
        self.alias_offsets.insert(qualified.clone(), offset);
        self.result.alias_offsets.insert(qualified.clone(), offset);
        self.result.command_aliases.insert(
            qualified.clone(),
            SignatureCommandAlias {
                source_name,
                qualified_name: qualified,
                target: target_cmd,
                extras: prepended,
            },
        );
    }

    /// Static words and dominating interpolations supply constant argument values.
    /// Command substitutions and unavailable variable values stay unresolved.
    /// Shared by any command whose argument names something else purely
    /// as data rather than computing it — [`Self::handle_rename`]'s
    /// `OLD`/`NEW` words, and [`Self::handle_source_command`]'s path word
    /// (`set p "e.tcl"; source $p`, the same shape `rename $old new`
    /// already resolves), the `oo::define`
    /// target word, and the command head itself
    /// ([`Self::resolve_dynamic_command_head`]).
    pub(super) fn resolve_dynamic_word(
        &self,
        text: &str,
        tok: Option<Token>,
        is_single: bool,
        _scope_path: &[usize],
    ) -> Option<String> {
        if !crate::naming::is_dynamic_word(text) {
            return Some(text.to_string());
        }
        let tok = tok?;
        if is_single && matches!(tok.kind, TokenType::Str | TokenType::Esc) {
            return Some(text.to_owned());
        }
        self.resolve_logical_value_word(tok)
            .or_else(|| self.frozen_value_word(tok))
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
        let old = crate::naming::normalise_qualified_name(&old_resolved);
        if old.is_empty() {
            return false;
        }
        // Required-name navigation and advice use the post-argv lookup site.
        // Recording the source operand does not require the destination to be
        // known, and does not prove that the rename itself succeeds.
        if let Some(tok) = arg_tokens.first() {
            self.push_command_reference_with_policy(
                args[0].clone(),
                tok.span,
                old.clone(),
                None,
                false,
                crate::signature_scan::types::SignatureCommandLookup::ConsumedName {
                    invocation_offset: offset,
                },
            );
        }
        let Some(new_resolved) = self.resolve_dynamic_word(
            &args[1],
            arg_tokens.get(1).copied(),
            arg_single.get(1).copied().unwrap_or(false),
            scope_path,
        ) else {
            return true;
        };
        let new = crate::naming::normalise_qualified_name(&new_resolved);
        // Moving an external-unit loader takes its loading out of static view.
        self.note_external_unit_command_moved(&old);
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
        // interpreter tracked under that name before.
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
        // Child-interpreter renames cannot mutate the parent's command facts.
        if !self.interp_path_stack.is_empty() {
            return false;
        }
        // A rename nested inside a control-flow body may not run at all, so it
        // is not evidence that `OLD` is gone. Recording it anyway draws a
        // W123 on a command that is demonstrably still callable, and — since
        // class liveness consults the same facts — withdraws the
        // W308 on objects of that class too.
        //
        // Oracle (tclsh8.6): with `if {0} { rename
        // Dog {} }`, `Dog new` succeeds and `$d fly` fails with `unknown
        // method "fly"`. The analyser proves the same branch dead in the same
        // run — it emits I230 on it — so honouring the deletion would
        // contradict its own verdict.
        //
        // Only straight-line deletions count. This is the *syntactic* rule:
        // `if {1} { rename Dog {} }` is equally not recorded, even though it
        // does run. Reading the branch's real value needs SCCP, which runs
        // later over the IR this walk feeds, so it is not available here. The
        // narrower rule errs towards treating commands as live, matching the
        // existing "a deletion inside a definition body does not count"
        // rule in `fact_live_for_call`.
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
            // NEW`, which hands it over and leaves every import edge alive.
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
    pub fn handle_oo_objdefine_at(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        let Some(words) = crate::registry_invocation::source_structure::source_registry_words_at(
            &self.source,
            &self.result,
            cmd_tok.span.start(),
        ) else {
            return true;
        };
        let context = self.analysis_context();
        let Some((grammar, definer_disabled)) = words
            .with_source_schema(&context, |schema| {
                let command = schema.authored_source_descriptors().command;
                // A script operand enters the definition vocabulary only through
                // the selected configure layout. The inline member form instead
                // names a member of the selected descriptor's own grammar under
                // its complete selected layout; that member's body stays Tcl.
                let grammar = schema
                    .authored_source_definition_body_grammar()
                    .or_else(|| {
                        let declared = command.definition_body?;
                        (schema.authored_source_argument_roles().1
                            && args
                                .get(1)
                                .is_some_and(|keyword| declared.is_member(keyword)))
                        .then_some(declared)
                    })?;
                Some((grammar, !context.context().spec_available(command)))
            })
            .flatten()
        else {
            return true;
        };
        let body_argument = if args
            .get(1)
            .is_some_and(|subcommand| grammar.is_member(subcommand))
        {
            None
        } else {
            let body_arguments: Vec<_> = words
                .written_argument_roles()
                .into_iter()
                .filter_map(|(argument, role)| {
                    (role == tcl_registry::ArgRole::Body).then_some(argument)
                })
                .collect();
            let [body_argument] = body_arguments.as_slice() else {
                return true;
            };
            let Some(source_body) = words.original_argument_word(*body_argument) else {
                return true;
            };
            let Some(body_token) = arg_tokens.get(*body_argument) else {
                return true;
            };
            if !words.matches_source(source_body.image(), source_body.config())
                || !source_body
                    .tokens()
                    .iter()
                    .any(|token| token.span == body_token.span)
            {
                return true;
            }
            Some(*body_argument)
        };
        self.handle_oo_objdefine_with_layout(
            args,
            arg_tokens,
            arg_single,
            scope_path,
            SelectedObjdefineBodyLayout {
                grammar: Some(grammar),
                body_argument,
                definer_disabled,
            },
        )
    }

    /// Apply the standalone object-definition layout used by direct test fixtures.
    #[cfg(test)]
    pub fn handle_oo_objdefine(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        let grammar = self.definition_grammar("oo::objdefine");
        self.handle_oo_objdefine_with_layout(
            args,
            arg_tokens,
            arg_single,
            scope_path,
            SelectedObjdefineBodyLayout {
                grammar,
                body_argument: Some(1),
                definer_disabled: false,
            },
        )
    }

    fn handle_oo_objdefine_with_layout(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        layout: SelectedObjdefineBodyLayout,
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        let (obj_name, resolved_obj) =
            self.oo_objdefine_receiver(args, arg_tokens, arg_single, scope_path);
        if let Some(name) = &obj_name {
            self.objdefined_vars.insert(name.clone());
        }
        // A receiver that resolves to no static name (`$objs($i)`, `[pick]`)
        // may define members on *any* object, so the per-object W315 gates
        // abstain file-wide.
        if obj_name.is_none() && resolved_obj.is_none() {
            self.objdefine_unresolved_receiver = true;
            for state in self.result.object_member_state.values_mut().flatten() {
                state.logical_source_members = None;
            }
        }

        // `oo::objdefine $obj` with no definition script — the object variable
        // is recorded above; there is nothing more to walk.
        if args.len() < 2 {
            return true;
        }

        if let Some(target) = arg_tokens.first() {
            self.retain_original_class_configuration_metadata(target.span);
        }

        // A throwaway holder for the walked members.  The method bodies home
        // under a private synthetic name (`@objdefine@…`, unrepresentable in
        // real Tcl) so a per-object `greet` never collides with the same-named
        // class method in the duplicate detector or the scope-name key.
        let synthetic = obj_name.as_ref().map_or_else(
            || "::@objdefine@".to_string(),
            |name| format!("::@objdefine@::{name}"),
        );
        let mut object_class = super::types::ClassDef {
            name: obj_name.clone().unwrap_or_default(),
            qualified_name: synthetic,
            ..Default::default()
        };

        // The receiver keys this block records under, and the binding the
        // block belongs to: the innermost proc/method frame is
        // the walk-time spelling of the binding identity consumers re-derive
        // from `ObjectMemberState::anchor_offset`.
        let keys: Vec<String> = [obj_name.clone(), resolved_obj.clone()]
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
            .map_or_default(|st| (st.methods.clone(), st.conditional));
        object_class.methods = seed;
        if let Some(source) = self.logical_object_member_source_input()
            && let Some(previous) = keys
                .first()
                .and_then(|key| self.objdefine_binding_state(key, frame))
            && let Some(owner) = previous.logical_source_members.as_ref()
            && owner.matches_state(previous, &source.0, &source.1)
        {
            object_class
                .original_members
                .absorb(owner.original_members(), true);
        }

        self.walk_oo_objdefine_form(args, arg_tokens, scope_path, &mut object_class, layout);

        // **W315** candidates. `oo::objdefine` aborts the same
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
    ) -> (Option<String>, Option<String>) {
        let Some(token) = arg_tokens.first() else {
            return (None, None);
        };
        let config = self.file_lexer_config();
        let Some(realm) = self.result.retained_command_realm() else {
            return (None, None);
        };
        let Some(image) = realm
            .original_source_image()
            .filter(|image| image.bytes() == self.source.as_bytes())
        else {
            return (None, None);
        };
        let Some((word, _)) =
            realm.original_written_word_at_span_in_source(image, token.span, config)
        else {
            return (None, None);
        };
        let operand = crate::registry_invocation::original_source_receiver_name_operand(&word);
        let written = match operand {
            Some(crate::registry_invocation::SourceReceiverNameOperand::Literal(value)) => {
                if self.selected_logical_declaration_advice() && value.as_bytes().contains(&0) {
                    return (None, None);
                }
                return (Some(value), None);
            }
            Some(crate::registry_invocation::SourceReceiverNameOperand::ScalarVariable(name)) => {
                name
            }
            None => return (None, None),
        };
        if self.selected_logical_declaration_advice() && written.as_bytes().contains(&0) {
            return (None, None);
        }
        let resolved = self
            .resolve_dynamic_word(
                &args[0],
                Some(*token),
                arg_single.first().copied().unwrap_or(false),
                scope_path,
            )
            .filter(|name| {
                !name.is_empty() && *name != written && !crate::naming::is_dynamic_word(name)
            });
        (Some(written), resolved)
    }

    /// Walk an `oo::objdefine` inline member or its braced definition body
    /// through the same registry grammar used by `oo::define`.
    fn walk_oo_objdefine_form(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        object_class: &mut super::types::ClassDef,
        layout: SelectedObjdefineBodyLayout,
    ) {
        let Some(grammar) = layout.grammar else {
            return;
        };
        let inline_form = args
            .get(1)
            .is_some_and(|subcommand| grammar.is_member(subcommand));
        if inline_form {
            let inline_args: Vec<String> = args[1..].to_vec();
            let inline_tokens: Vec<Token> = arg_tokens.iter().skip(1).copied().collect();
            {
                if let Some(member) = inline_args
                    .first()
                    .and_then(|subcmd| grammar.member(subcmd))
                {
                    self.emit_w002_oo_member_option_disabled(
                        member,
                        &inline_args[1..],
                        &inline_tokens[1..],
                        layout.definer_disabled,
                    );
                }
                super::oo::parse_oo_define_inline_in(
                    self,
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    object_class,
                    Some(self.analysis_context().context().authoring_query()),
                    scope_path,
                );
                self.record_member_command_references(
                    grammar,
                    &inline_args,
                    &inline_tokens,
                    scope_path,
                );
            }
        } else if let (Some(body), Some(body_tok)) = (
            layout.body_argument.and_then(|argument| args.get(argument)),
            layout
                .body_argument
                .and_then(|argument| arg_tokens.get(argument).copied()),
        ) {
            self.parse_oo_definition_body(
                body,
                body_tok,
                object_class,
                scope_path,
                Some(grammar),
                layout.definer_disabled,
            );
        }
    }

    /// Record one walked `oo::objdefine` block's member declarations and fold
    /// it into each receiver key's durable [`super::types::ObjectMemberState`]
    /// — the durable home for per-object visibility.
    ///
    /// Declarations feed `object_methods` so `$obj m` navigation resolves the
    /// per-object override ahead of a same-named class method, keyed by the
    /// objdefine site's receiver offset — the receiver's *binding identity*,
    /// never the textual tail alone.  The diff runs per
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
        let source = self.logical_object_member_source_input();
        for key in keys {
            let previous_source_complete =
                self.objdefine_binding_state(key, frame)
                    .is_none_or(|previous| {
                        source.as_ref().is_some_and(|(image, input)| {
                            previous
                                .logical_source_members
                                .as_ref()
                                .is_some_and(|owner| owner.matches_state(previous, image, input))
                        })
                    });
            let key_seed = self
                .objdefine_binding_state(key, frame)
                .map_or_default(|st| st.methods.clone());
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
            state.logical_source_members = source
                .as_ref()
                .filter(|_| previous_source_complete && !object_class.member_set_incomplete)
                .and_then(|(image, input)| {
                    super::types::LogicalSourceObjectMemberState::capture(
                        image.clone(),
                        input.clone(),
                        state,
                        &object_class.original_members,
                    )
                });
        }
    }

    fn logical_object_member_source_input(
        &self,
    ) -> Option<(tcl_lexer::SourceImage, super::ResolvedAnalysisInput)> {
        (self.selected_logical_declaration_advice() && !self.objdefine_unresolved_receiver)
            .then_some(())?;
        let input = self.resolved_analysis_input();
        let realm = self.result.retained_command_realm()?;
        realm
            .matches_resolved_analysis_input(&input)
            .then_some(())?;
        let image = realm.original_source_image()?;
        (image.bytes() == self.source.as_bytes()).then_some((image.clone(), input))
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

    /// Logical source compatibility for per-object definition-abort advice.
    /// Candidates retain reporting member/receiver labels, without a genuine
    /// selected operation or a closed current Native own-method table. Native,
    /// hosted and unknown input therefore supplies no diagnosis from these rows.
    /// Original member ledgers describe authored source order; constructed
    /// instance source carriers supply neither allocation nor absence closure.
    pub(super) fn flush_objdefine_abort_diagnostics(&mut self) {
        let candidates = std::mem::take(&mut self.objdefine_abort_candidates);
        // naming.diagnostics.original-object-abort-admission
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-object-abort-admission.md
        if !self.result.allows_retained_logical_declaration_advice()
            || candidates.is_empty()
            || self.objdefine_unresolved_receiver
        {
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
                diagnostics.push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W315,
                    cand.abort.span,
                    cand.abort.object_message(),
                    super::types::Severity::Warning,
                ));
            }
        }
        self.result.diagnostics.extend(diagnostics);
    }

    /// Handle ``package require ?-exact? NAME ?requirement ...?`` — record
    /// the package dependency so later passes can gate W123
    /// (unresolved-command) suppression and dynamic-provider detection.
    ///
    /// Appends a ``SignaturePackageRequire`` record to
    /// ``result.package_requires`` and flips ``has_dynamic_providers`` when
    /// the name is a ``$``-substitution / ``[…]``-substitution. Where each
    /// word sits is the registry's: the name and the requirements are the
    /// roles `require` declares after its option run, and ``exact`` is
    /// whether that run is non-empty — the subcommand's one option,
    /// ``-exact``, narrows each requirement to an exact version for the
    /// resolver (the reading `signature_scan`'s handler gives the same
    /// call). A call too short to carry a name records nothing.
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
    /// on `package`'s `require` subcommand); `args[0]` is still the
    /// subcommand word.
    fn original_package_operand(
        &self,
        token: Option<&Token>,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        let token = token?;
        let policy = self.declaration_name_policy()?;
        if let Some(input) = self
            .head_identities
            .original_written_name_input_at_span(token.span, self.file_lexer_config())
        {
            return (input.policy() == policy
                && !matches!(
                    input,
                    crate::signature_scan::scope::SignatureSourceNameInput::OriginalVariableRoot(_)
                ))
            .then_some(input);
        }
        let original = self.head_identities.original_source_name_at_span(
            token.span,
            self.file_lexer_config(),
            self.word_rules(),
            policy,
        )?;
        Some(
            crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                original.name_input().clone(),
            ),
        )
    }

    fn original_package_name(
        &self,
        token: Option<&Token>,
    ) -> Option<crate::signature_scan::original_name::SourcePackageName> {
        self.original_package_operand(token)
            .map(crate::signature_scan::original_name::SourcePackageName::from_input)
    }

    /// Record an original package requirement and its independently selected
    /// byte key, without inferring that its loader has executed.
    pub fn handle_package_require(
        &mut self,
        _cmd_name: &str,
        cmd_tok: Token,
        _args: &[String],
        arg_tokens: &[Token],
    ) {
        use tcl_registry::arg_role::ArgRole;
        let Some(original) = self.original_diagnostic_call_at(cmd_tok, arg_tokens) else {
            return;
        };
        let Some((Some(roles), exact)) = original.with_schema(|call| {
            (
                call.arg_roles(),
                call.option_effects().option_end > call.semantics.argument_offset,
            )
        }) else {
            return;
        };
        let Some((arguments, tokens, _single)) = original.source_arguments() else {
            return;
        };
        let args = arguments.as_slice();
        let arg_tokens = tokens.as_slice();
        let Some(name_idx) = role_position(&roles, ArgRole::Name) else {
            return;
        };
        let name_text = args[name_idx].clone();
        let requirements = role_texts(&roles, ArgRole::Value, args);
        let version = requirements.first().cloned();

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

        let original_name = self.original_package_name(arg_tokens.get(name_idx));
        let original_requirements = roles
            .iter()
            .filter_map(|&(ordinal, role)| (role == ArgRole::Value).then_some(ordinal))
            .map(|ordinal| self.original_package_operand(arg_tokens.get(ordinal)))
            .collect();
        self.result
            .package_requires
            .push(crate::signature_scan::types::SignaturePackageRequire {
                original_name,
                original_requirements,
                name: name_text,
                version,
                requirements,
                exact,
                range: cmd_tok.span,
                conditional: self.conditional_depth > 0,
                control_flow: self.control_flow_body_depth > 0,
                origin: crate::signature_scan::types::PackageRequireOrigin::Source,
            });
    }

    /// Record ``package provide NAME ?VERSION?`` — the name and the version
    /// are the roles `provide` declares.
    ///
    /// Dispatched via
    /// [`tcl_registry::hooks::AnalyserHookId::PackageProvide`] (stamped
    /// on `package`'s `provide` subcommand).  See
    /// [`Self::handle_package_require`] for the `cmd_tok` anchoring
    /// convention.
    pub fn handle_package_provide(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        if args.len() < 2 {
            return;
        }
        let original_name = self.original_package_name(arg_tokens.get(1));
        let name = args[1].clone();
        let version = args.get(2).cloned();
        self.result
            .package_provides
            .push(super::types::PackageProvide {
                original_name,
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
    /// what was registered. The name, the version and the script are the
    /// roles `ifneeded` declares; the setter form is the one with a script.
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
    pub fn handle_package_ifneeded(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        // args[0] is the `ifneeded` subcommand word; the setter form
        // is NAME + VERSION + SCRIPT.
        if args.len() < 4 {
            return;
        }
        let original_name = self.original_package_name(arg_tokens.get(1));
        self.result
            .package_ifneededs
            .push(super::types::PackageIfneeded {
                original_name,
                name: args[1].clone(),
                version: args[2].clone(),
                range: cmd_tok.span,
            });
    }

    /// Record ``package prefer latest`` — the one form of ``package
    /// prefer`` that changes the interpreter's version-selection rule.
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

    /// The definition-body grammar attached to `cmd_name`'s spec, if it is a
    /// definer command.  Registry-sourced, so member recognition / argument
    /// structure in the body walkers never hardcodes a keyword list — a new
    /// definer is picked up the moment its spec carries a `definition_body`.
    pub(super) fn definition_grammar(
        &self,
        cmd_name: &str,
    ) -> Option<&'static tcl_registry::definer::DefinitionBodyGrammar> {
        let context = self.analysis_context();
        context
            .context()
            .resolve_spec(context.commands(), cmd_name)?
            .definition_body
    }

    /// The [`ClassFactory`] `class` publishes, when `class` is itself a
    /// `TclOO` class factory — a class whose superclass chain reaches a
    /// registry metaclass.
    ///
    /// Derived **once, where the metaclass is written**, so every consumer
    /// (this file's own `Meta create …` calls and, through the workspace
    /// factory index, another file's) classifies a creation call from the
    /// same proved fact instead of re-deriving it — or, cross-file,
    /// abstaining for want of it.
    ///
    /// A user metaclass that does not override a manufacturer subcommand
    /// inherits `oo::class`'s own `create Name Body` shape, so that
    /// subcommand simply has no entry and the builtin layout applies.  One
    /// that *does* override it declares its own shape in the override's
    /// parameter list, and that override is read rather than guessed: Tk's
    /// `self method create {name superclasses body}` puts the body at
    /// argument 3, not 2, and splices a superclass the caller never wrote.
    fn class_factory_of(&self, class: &ClassDef) -> Option<ClassFactory> {
        let meta = self.user_metaclass_of_class(class)?;
        let members =
            super::class_hierarchy::original_metadata::original_definition_members_for_class(
                &self.result,
                class,
                MemberSide::ClassObject,
            )?;
        let mut overrides = std::collections::BTreeMap::new();
        let mut exported_manufacturers = std::collections::BTreeSet::new();
        for builtin in meta.grammar.manufacturers {
            let own = members.iter().find(|method| {
                method.original_name_input().policy() == meta.root_policy
                    && method.original_name_input().bytes() == builtin.keyword.as_bytes()
            });
            let exported = if let Some(method) = own {
                let override_def = method.original_parameter_names_and_body(&self.result)?;
                let lexical =
                    super::types::OriginalLexicalMemberContext::from_original_definition_method(
                        &self.result,
                        class,
                        method,
                    )?;
                let next = self.manufacturer_next_call(&override_def, &lexical)?;
                let arguments = next.original_direct_argument_words()?;
                let (name_arg, body_arg) =
                    Self::manufacturer_word_positions(&override_def, builtin, &arguments)?;
                let body_argument = usize::from(builtin.definition_body_at?).checked_sub(1)?;
                let prologue = arguments.get(body_argument)?;
                overrides.insert(
                    builtin.keyword.to_owned(),
                    super::types::ManufacturerSpec {
                        name_arg,
                        body_arg,
                        injected: self.manufacturer_injected_template(
                            &meta,
                            &override_def,
                            prologue,
                        ),
                    },
                );
                method.exported()
            } else {
                class
                    .original_members
                    .inherited_visibility_for_metadata_keyword(
                        MemberSide::ClassObject,
                        meta.root_policy,
                        builtin.keyword,
                        builtin.visibility == tcl_registry::definer::MemberVisibility::Exported,
                    )?
            };
            if exported {
                exported_manufacturers.insert(builtin.keyword.to_owned());
            }
        }
        let unknown_binds_instance = self.unknown_dispatch_binds_instance(&meta, class);
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
    /// `::tk::IconList .il` idiom.
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
        let Some(members) =
            super::class_hierarchy::original_metadata::original_definition_members_for_class(
                &self.result,
                class,
                MemberSide::Instance,
            )
        else {
            return false;
        };
        let Some(original) = members.iter().find(|method| {
            method.original_name_input().policy() == meta.root_policy
                && method.original_name_input().bytes() == keyword.as_bytes()
        }) else {
            return false;
        };
        let Some(member) = original.original_parameter_names_and_body(&self.result) else {
            return false;
        };
        let Some(word_param) = member.parameter_names().next() else {
            return false;
        };
        let Some(body_token) = member.body_word().tokens().first() else {
            return false;
        };
        let Some(proof) = self.unknown_script_path_proof(
            meta,
            class,
            body_token.span,
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
        let context = self.retained_static_source_context()?;
        let mut proof = UnknownPathProof::default();
        for seg in self.direct_statements_in_span(body_span)? {
            if states.is_empty() {
                break;
            }
            let words = crate::registry_invocation::source_structure::source_registry_words_at(
                &self.source,
                &self.result,
                seg.argv.first()?.span.start(),
            )?;
            let hook =
                words.with_source_schema(&context, |schema| schema.semantics.lowering_hook)?;
            if hook == Some(tcl_registry::hooks::LoweringHookId::If) {
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
            if hook.is_some_and(|hook| {
                matches!(
                    hook,
                    tcl_registry::hooks::LoweringHookId::Switch
                        | tcl_registry::hooks::LoweringHookId::Try
                )
            }) {
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

    fn logical_segment_invocation_completion(
        registry: &tcl_registry::CommandRegistry,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        tokens: &crate::ir::CommandTokens,
        #[cfg(test)] trace: &impl Fn(&str, bool),
    ) -> Option<(
        crate::registry_invocation::ResolvedStatementInvocation,
        tcl_registry::registry::InvocationCompletion,
    )> {
        let invocation =
            crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                registry, metadata, tokens,
            );
        #[cfg(test)]
        trace("operation.selected", invocation.is_some());
        let invocation = invocation?;
        let realm = tokens.source_binding.as_ref().and_then(|binding| {
            binding.original_source_realm_for_input(
                tokens,
                metadata.source_analysis_input()?,
                registry,
            )
        });
        #[cfg(test)]
        trace("realm.original", realm.is_some());
        let realm = realm?;
        let completion = invocation.with_metadata_schema(registry, metadata, realm, |selected| {
            Some(selected.authored_source_invocation_completion())
        });
        #[cfg(test)]
        trace("completion.schema", completion.is_some());
        #[cfg(test)]
        trace(
            "completion.known",
            completion.is_some_and(|completion| {
                completion != tcl_registry::registry::InvocationCompletion::Unknown
            }),
        );
        let completion = completion?;
        Some((invocation, completion))
    }

    fn written_segment_completion(
        effective: &crate::registry_invocation::EffectiveCommandWords,
        completion: tcl_registry::registry::InvocationCompletion,
    ) -> tcl_registry::registry::InvocationCompletion {
        use crate::registry_invocation::InvocationWordOrigin;
        use tcl_registry::registry::InvocationCompletion;
        match completion {
            InvocationCompletion::ReturnsResult(Some(index)) => {
                match index
                    .checked_add(1)
                    .and_then(|index| effective.origins.get(index))
                {
                    Some(InvocationWordOrigin::Written(index)) => index
                        .checked_sub(1)
                        .map_or(InvocationCompletion::Unknown, |index| {
                            InvocationCompletion::ReturnsResult(Some(index))
                        }),
                    _ => InvocationCompletion::Unknown,
                }
            }
            completion => completion,
        }
    }

    #[cfg(test)]
    fn trace_segment_source_guards(
        &self,
        logical_source: bool,
        retained: Option<&crate::analyser::ResolvedAnalysisInput>,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        trace: &impl Fn(&str, bool),
    ) {
        trace("source.logical", logical_source);
        trace("source.retained", retained.is_some());
        trace("metadata.request", metadata.is_some());
        trace("tokens.binding", tokens.source_binding.is_some());
        let binding = tokens.source_binding.as_ref();
        trace(
            "tokens.config",
            binding.and_then(|binding| binding.original_lexer_config_for_tokens(tokens))
                == Some(self.file_lexer_config()),
        );
        trace(
            "metadata.original",
            binding.zip(retained).is_some_and(|(binding, input)| {
                binding
                    .original_invocation_metadata_for_input(tokens, input, registry)
                    .is_some()
            }),
        );
        let advice = binding.and_then(|binding| binding.declaration_operand_layout_advice(tokens));
        trace("declaration.layout", advice.is_some());
        trace(
            "declaration.closed",
            advice.is_some_and(|advice| advice.closed_logical_source_lookup()),
        );
    }

    /// Completion follows the actual selected implementation and frozen argv.
    /// A captured alias operand has no original segment index to return.
    fn segment_invocation_completion(
        &self,
        seg: &SegmentedCommand,
    ) -> tcl_registry::registry::InvocationCompletion {
        use tcl_registry::registry::InvocationCompletion;
        let context = self.analysis_context();
        let registry = context.commands();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &self.cached_source_map(),
            self.file_lexer_config(),
            seg,
        );
        self.head_identities
            .source_bindings_ref()
            .stamp_original_tokens(&mut tokens);
        let retained = self
            .resolved_input
            .as_ref()
            .or(self.result.resolved_input.as_ref());

        #[cfg(test)]
        let trace_enabled =
            tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some();
        #[cfg(test)]
        let trace = |label: &str, passed: bool| {
            if trace_enabled {
                eprintln!("ORIGINAL_COMPLETION_GUARD label={label} passed={passed}");
            }
        };
        let metadata = retained.and_then(|input| {
            crate::registry_invocation::InvocationMetadataContext::for_source_input(
                registry,
                input,
                self.file_lexer_config(),
                Some(input.unit_profile()),
            )
        });
        let purpose = metadata.and_then(|metadata| {
            tokens
                .source_binding
                .as_ref()?
                .original_source_advice_purpose(&tokens, metadata, registry)
        });
        let Some(purpose) = purpose else {
            return InvocationCompletion::Unknown;
        };
        let logical_source =
            purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
        #[cfg(test)]
        if trace_enabled {
            self.trace_segment_source_guards(
                logical_source,
                retained,
                metadata,
                &tokens,
                registry,
                &trace,
            );
        }
        let selected = if logical_source {
            let Some(metadata) = metadata else {
                return InvocationCompletion::Unknown;
            };
            let Some(selected) = Self::logical_segment_invocation_completion(
                registry,
                metadata,
                &tokens,
                #[cfg(test)]
                &trace,
            ) else {
                return InvocationCompletion::Unknown;
            };
            selected
        } else {
            // Native completion retains its independently proved handler path.
            let Some(invocation) =
                crate::registry_invocation::resolved_tokens_invocation_in_context(
                    &context, &tokens,
                )
            else {
                return InvocationCompletion::Unknown;
            };
            let completion = invocation.with_argument_words(|words| {
                registry.invocation_completion_words(
                    &invocation.facts.canonical_command,
                    words.arguments(),
                    invocation
                        .dialect
                        .and_then(tcl_registry::InvocationDialect::authoring_query),
                )
            });
            (invocation, completion)
        };
        let (invocation, completion) = selected;
        Self::written_segment_completion(&invocation.effective, completion)
    }

    fn possible_if_body_spans(
        &self,
        seg: &SegmentedCommand,
        _word_param: &str,
    ) -> Option<(Vec<Span>, bool)> {
        let context = self.retained_static_source_context()?;
        let input = self.result.resolved_input.as_ref()?;
        if self.segment_invocation_completion(seg)
            != tcl_registry::registry::InvocationCompletion::FallsThrough
        {
            return None;
        }
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            &self.source,
            &self.result,
            seg.argv.first()?.span.start(),
        )?;
        if words.origins().len() != seg.argv.len()
            || words.origins().iter().enumerate().any(|(ordinal, origin)| {
                *origin != crate::registry_invocation::InvocationWordOrigin::Written(ordinal)
            })
        {
            return None;
        }
        let clauses = words.with_source_schema(&context, |schema| {
            if schema.semantics.lowering_hook != Some(tcl_registry::hooks::LoweringHookId::If) {
                return None;
            }
            let plan = schema.clause_plan()?;
            if plan.defect.is_some() || plan.clauses.is_empty() {
                return None;
            }
            let clauses = plan
                .clauses
                .iter()
                .map(|clause| {
                    if clause.timing != tcl_registry::clause_grammar::ClauseTiming::Selected
                        || clause.operands(tcl_registry::ArgRole::Body).count() != 1
                        || clause.operands(tcl_registry::ArgRole::Expr).count() > 1
                    {
                        return None;
                    }
                    let body = clause.operand(tcl_registry::ArgRole::Body)?;
                    let semantics = if input.has_logical_source_name_context() {
                        schema.authored_logical_source_control_arm_semantics(body)
                    } else {
                        schema.authored_source_control_arm_semantics(body)
                    };
                    (semantics == Some(tcl_registry::registry::ControlArmSemantics::Selected))
                        .then_some(())?;
                    Some((clause.operand(tcl_registry::ArgRole::Expr), body))
                })
                .collect::<Option<Vec<_>>>()?;
            let (mut scripts, prefixes, mut expressions) =
                if input.has_logical_source_name_context() {
                    (
                        schema.authored_logical_source_script_arguments()?,
                        schema.authored_logical_source_command_prefix_arguments()?,
                        schema
                            .authored_logical_source_expression_arguments()?
                            .arguments,
                    )
                } else {
                    (
                        schema.authored_source_script_arguments()?,
                        schema.authored_source_command_prefix_arguments()?,
                        schema.authored_source_expression_arguments()?.arguments,
                    )
                };
            let mut expected_scripts = clauses.iter().map(|(_, body)| *body).collect::<Vec<_>>();
            let mut expected_expressions = clauses
                .iter()
                .filter_map(|(condition, _)| *condition)
                .collect::<Vec<_>>();
            scripts.sort_unstable();
            expressions.sort_unstable();
            expected_scripts.sort_unstable();
            expected_expressions.sort_unstable();
            (prefixes.is_empty()
                && scripts == expected_scripts
                && expressions == expected_expressions)
                .then_some(clauses)
        })??;
        let branches = Self::original_if_body_branches(&words, clauses, &context)?;
        self.resolve_possible_if_branches(seg, &branches)
    }

    fn original_if_body_branches(
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        clauses: Vec<(Option<usize>, usize)>,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<Vec<(Option<usize>, Span)>> {
        let bodies = words.source_script_bodies_for_mutation_coverage(context)?;
        if bodies.len() != clauses.len() {
            return None;
        }
        let mut branches = Vec::with_capacity(clauses.len());
        for (condition, ordinal) in clauses {
            let original = words.original_argument_word(ordinal)?;
            let mut matches = bodies
                .iter()
                .filter(|body| body.original_container() == original);
            let body = matches.next()?;
            if matches.next().is_some()
                || branches.iter().any(|(_, span)| *span == body.extent_span())
            {
                return None;
            }
            branches.push((condition, body.extent_span()));
        }
        Some(branches)
    }

    fn resolve_possible_if_branches(
        &self,
        seg: &SegmentedCommand,
        branches: &[(Option<usize>, Span)],
    ) -> Option<(Vec<Span>, bool)> {
        for &(condition, body) in branches {
            match condition {
                None => return Some((vec![body], false)),
                Some(condition) => {
                    match self.original_condition_boolean(seg, condition.checked_add(1)?) {
                        Some(true) => return Some((vec![body], false)),
                        Some(false) => {}
                        None => {
                            return Some((
                                branches.iter().map(|(_, span)| *span).collect(),
                                branches.iter().all(|(condition, _)| condition.is_some()),
                            ));
                        }
                    }
                }
            }
        }
        Some((Vec::new(), true))
    }

    fn original_condition_boolean(&self, seg: &SegmentedCommand, written: usize) -> Option<bool> {
        let context = self.retained_static_source_context()?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &self.cached_source_map(),
            self.file_lexer_config(),
            seg,
        );
        self.head_identities
            .source_bindings_ref()
            .stamp_original_tokens(&mut tokens);
        let original = crate::registry_invocation::original_expression_operand_advice_for_word(
            context.commands(),
            &tokens,
            written,
        )?;
        if !original.lookup_closed
            || original.expression.evaluation_nodes().iter().any(|node| {
                !matches!(
                    node,
                    crate::expr_ast::ExprNode::Literal { .. }
                        | crate::expr_ast::ExprNode::Unary { .. }
                        | crate::expr_ast::ExprNode::Binary { .. }
                        | crate::expr_ast::ExprNode::Ternary { .. }
                )
            })
        {
            return None;
        }
        let mut policy = crate::tcl_expr_eval::FoldPolicy::from_registry(context.commands())
            .with_invocation_dialect(original.dialect);
        policy.word_rules =
            tcl_syntax::word_rules::WordValueRules::from_config(&self.file_lexer_config());
        crate::tcl_expr_eval::evaluate_expr_with_constants(
            &original.expression,
            &crate::tcl_expr_eval::Env::new(),
            policy,
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
            let (namespace, _) = crate::naming::key_holder_and_tail(&class.qualified_name);
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
            let lookup = hierarchy.original_unresolved_relation(ancestor)?;
            if lookup.policy() == meta.root_policy
                && lookup
                    .candidates()?
                    .iter()
                    .any(|slot| slot == &meta.root_slot)
                && !classes.values().any(|class| {
                    class.source_name.as_ref().is_some_and(|source| {
                        source.policy() == lookup.policy() && source.slot() == &meta.root_slot
                    })
                })
            {
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
        self.registry.as_deref()?;
        for seg in self.direct_statements_in_span(method.body_span)? {
            match self.segment_invocation_completion(&seg) {
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
            let config = self.file_lexer_config();
            let bindings = self.head_identities.source_bindings_ref();
            let Some(origin) = bindings.source_origin() else {
                return UnknownBodyEvidence::Nothing;
            };
            let image = origin.source_image();
            if image.bytes() != self.source.as_bytes()
                || !bindings.matches_original_source_image(image, config)
            {
                return UnknownBodyEvidence::Nothing;
            }
            let tokens = crate::ir::CommandTokens::from_segmented(&image.source_map(), config, seg);
            let original = crate::registry_invocation::original_native_compiler_words(
                image,
                tokens.words(),
                seg.span.start(),
                config,
            );
            return if original
                .as_ref()
                .and_then(|words| {
                    tcl_syntax::word_rules::original_scalar_variable_with_prefix(
                        words.get(usize::from(name_arg).checked_add(1)?)?,
                        &[b"", b"::"],
                    )
                })
                .is_some_and(|name| name == word_param.as_bytes())
            {
                UnknownBodyEvidence::Constructs
            } else {
                UnknownBodyEvidence::Nothing
            };
        }
        let completion = self.segment_invocation_completion(seg);
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
                return if crate::value_shapes::whole_word_scalar_var_name(
                    word,
                    self.file_lexer_config(),
                ) == Some(word_param)
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
        let Some(context) = self.retained_static_source_context() else {
            return;
        };
        let registry = context.commands().as_ref();
        let block = crate::segmenter::segment_commands_with_offset_and_config(
            text,
            base,
            self.file_lexer_config().nested(),
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
    /// value a literal call-site argument proves.
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
    /// Keep a creation call the walk declined, in case a metaclass proved
    /// after the walk turns it into one.
    ///
    /// The gate is "nothing this document knows makes the head's verdict
    /// final": a registry command has its own classification, and a recorded
    /// **proc** is a command that is not a class, so neither can become a
    /// factory later and neither is kept.
    ///
    /// A recorded **class** deliberately *is* kept, even though the walk knew
    /// the name. A class can be recorded without a factory and gain one in the
    /// post-pass — an `oo::define` on a computed-name metaclass makes exactly
    /// such a stub, and the join then proves it — so "known" is not the same
    /// question as "judged".
    ///
    /// A call needs at least a manufacturer word and a name word to be a
    /// creation at all, so shorter ones are never kept. Buffering is bounded:
    /// past [`MAX_DEFERRED_CLASS_CREATIONS`] a document keeps the behaviour it
    /// had before this pass existed, which is the abstaining direction.
    fn defer_class_creation(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        cmd_tok: Token,
    ) {
        // Cheapest questions first: this runs for every command the generic
        // handler declines, which is most of a document.
        if args.len() < 2
            || arg_tokens.len() < 2
            || self.deferred_class_creations.len() >= MAX_DEFERRED_CLASS_CREATIONS
            || crate::naming::is_dynamic_word(cmd_name)
            || self
                .registry
                .as_deref()
                .is_some_and(|registry| registry.get(cmd_name).is_some())
        {
            return;
        }
        let namespace = self.command_resolution_namespace(scope_path);
        if crate::naming::bareword_resolution_candidates(&namespace, cmd_name)
            .into_iter()
            .any(|candidate| self.result.all_procs.contains_key(&candidate))
        {
            return;
        }
        self.deferred_class_creations
            .push(super::types::DeferredClassCreation {
                cmd_name: cmd_name.to_owned(),
                args: args.to_vec(),
                arg_tokens: arg_tokens.to_vec(),
                scope_path: scope_path.to_vec(),
                cmd_tok,
            });
    }

    /// Form the deferred verdicts, now that
    /// [`Self::record_literal_parameter_definitions`] has settled the
    /// parameterised-class join.
    ///
    /// Replayed in document order, so a chain settles in one pass: a class
    /// this replay records is in the index before the next call is retried,
    /// which is what lets `Meta create ::A::sub` and then `::A::sub create
    /// ::A::leaf` both land when `Meta` itself was only proved post-pass.
    ///
    /// Each call goes back through the *same* handler the walk used, so a
    /// replayed creation is recorded by one code path with the walk's own
    /// rules — the manufacturer layout, the injected members, the body walk,
    /// the W314/W315 emissions. The retry is gated on the head now naming a
    /// factory, so a head that stayed unknown costs one lookup and records
    /// nothing.
    pub(super) fn replay_deferred_class_creations(&mut self) {
        if self.deferred_class_creations.is_empty() {
            return;
        }
        for deferred in std::mem::take(&mut self.deferred_class_creations) {
            // What the walk already recorded under the name this call
            // creates — an `oo::define` stub it made *because* the creation
            // was invisible. Captured before the replay, which overwrites it.
            let prior = self.deferred_creation_prior(&deferred);
            if !self.handle_oo_class_command(
                &deferred.cmd_name,
                &deferred.args,
                &deferred.arg_tokens,
                &deferred.scope_path,
                deferred.cmd_tok,
            ) {
                continue;
            }
            if let Some((qualified, prior)) = prior {
                self.rejoin_replayed_class(&qualified, &prior, &deferred.scope_path);
            }
        }
        // A replayed call has had its verdict, whichever way it went. The
        // handler's decline path buffers again — it cannot tell a first
        // hearing from a second — so the buffer is emptied here rather than
        // left to be replayed once more against the same evidence.
        self.deferred_class_creations.clear();
    }

    /// The class name a deferred call would record, paired with whatever the
    /// walk already holds under it.
    ///
    /// `None` when the head still names no factory, when the layout puts no
    /// name where this call has a word, or — the ordinary case — when nothing
    /// was recorded under that name at all.
    fn deferred_creation_prior(
        &self,
        deferred: &super::types::DeferredClassCreation,
    ) -> Option<(String, ClassDef)> {
        let resolved =
            self.resolve_class_creation(&deferred.args, &deferred.arg_tokens, deferred.cmd_tok)?;
        let raw_name = deferred.args.get(resolved.layout.name_arg)?;
        let namespace = self.command_resolution_namespace(&deferred.scope_path);
        let qualified = qualify(&namespace, raw_name);
        Some((
            qualified.clone(),
            self.result.all_classes.get(&qualified)?.clone(),
        ))
    }

    /// Fold a record the walk made for a class whose creation call it could
    /// not see back into the replayed creation.
    ///
    /// Because the creation was invisible, an `oo::define` on the same class
    /// made a *stub* — and that stub is where its members went. The replay
    /// then records the creation under the same name, so without this the
    /// members the walk did record would be lost: strictly worse than before
    /// the replay existed.
    ///
    /// The union is the one [`join_parameterised_class_observations`] already
    /// owns, used exactly as the parameterised-class post-pass uses it, so a
    /// stub joins a creation by one rule wherever the two meet. Two
    /// observations that contradict each other publish the abstaining record
    /// the house rule calls for — no factory, inheritance unknown — while
    /// still keeping both sides' declarations, which are declarations of this
    /// class either way.
    fn rejoin_replayed_class(&mut self, qualified: &str, prior: &ClassDef, scope_path: &[usize]) {
        let Some(observed) = self.result.all_classes.get(qualified) else {
            return;
        };
        let joined = join_parameterised_class_observations(prior, observed).unwrap_or_else(|| {
            let mut abstaining = observed.clone();
            abstaining.factory = None;
            abstaining.inheritance_unknown = true;
            abstaining.absorb_declarations(prior);
            abstaining
        });
        let simple = joined.name.clone();
        self.result
            .all_classes
            .insert(qualified.to_owned(), joined.clone());
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, scope_path) {
            scope.classes.insert(simple, joined);
        }
    }

    pub(super) fn record_literal_parameter_definitions(&mut self) {
        self.record_parameterised_class_definitions();
        // The join has settled, so the heads the walk could not judge can be
        // judged now. Paired here rather than at each of the
        // four entry points that run this tail: the order matters — a
        // creation call can only be classified after the metaclass it names
        // is proved — and pairing them is what makes that order impossible to
        // get wrong. It also guarantees the buffer is drained on every path,
        // so a reused analyser never carries one document's deferrals into
        // the next.
        self.replay_deferred_class_creations();
    }

    fn record_parameterised_class_definitions(&mut self) {
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
                if !tcl_syntax::naming::is_absolute(&resolved) || !seen.insert(resolved.clone()) {
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
            if !self.static_prefix_falls_through(body_span, arm.controller.span.start(), 0)
                || self.control_body_is_selected("", arm, &env, 0) != Some(true)
            {
                return false;
            }
            body_span = arm.body_span;
        }
        self.static_prefix_falls_through(body_span, call.call_off, 0)
    }

    /// Whether the statements of `body_span` before `stop_off` all fall
    /// through, so control really reaches `stop_off`.
    ///
    /// `depth` is the nesting of *control bodies* this walk has descended
    /// into — [`Self::static_statement_falls_through`] re-enters here for each
    /// arm whose body may run, so nested control flow recurses one native
    /// frame per level and has to be bounded like every other recursive walk
    /// in this file ([`MAX_UNKNOWN_BODY_DEPTH`]). Past
    /// the cap the walk abstains, which is the direction it already takes for
    /// anything it cannot read. Entry points that are not themselves inside a
    /// body pass `0`.
    fn static_prefix_falls_through(&mut self, body_span: Span, stop_off: u32, depth: u32) -> bool {
        if self.retained_static_source_context().is_none() {
            return false;
        }
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return false;
        };
        for seg in statements
            .into_iter()
            .take_while(|seg| seg.span.start() < stop_off)
        {
            if !self.static_statement_falls_through(&seg, depth) {
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
        let proc_def = if self.result.allows_lexical_declaration_advice() {
            let qname = self.resolve_static_proc_name(caller, seg.name())?;
            self.result.all_procs.get(&qname)?.clone()
        } else {
            self.original_static_procedure_at(seg)?
        };
        let qname = proc_def.qualified_name.clone();
        if stack.iter().any(|active| active == &qname) {
            return None;
        }
        if proc_def.params_computed {
            return None;
        }
        let actuals: Vec<Option<String>> = seg.args().iter().cloned().map(Some).collect();
        bind_proc_formals(&proc_def.params, &actuals, self.word_rules())?;
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
        let context = self.retained_static_source_context()?;
        let registry = context.commands();
        for seg in self.direct_statements_in_span(body_span)? {
            match self.segment_invocation_completion(&seg) {
                tcl_registry::registry::InvocationCompletion::ReturnsResult(_) => {
                    return Some(true);
                }
                tcl_registry::registry::InvocationCompletion::Terminates => return Some(false),
                tcl_registry::registry::InvocationCompletion::Unknown => {
                    if registry.get(seg.name()).is_some() {
                        return None;
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

    fn static_statement_falls_through(&mut self, seg: &SegmentedCommand, depth: u32) -> bool {
        if depth >= MAX_UNKNOWN_BODY_DEPTH {
            return false;
        }
        let Some(context) = self.retained_static_source_context() else {
            return false;
        };
        let registry = context.commands().as_ref();
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        match self.segment_invocation_completion(seg) {
            tcl_registry::registry::InvocationCompletion::FallsThrough => {}
            tcl_registry::registry::InvocationCompletion::Unknown
                if registry.get(seg.name()).is_none() =>
            {
                return self
                    .static_user_proc_call_falls_through("", seg, depth + 1, &mut Vec::new())
                    .unwrap_or(false);
            }
            _ => return false,
        }
        let is_control = registry
            .invocation_traits(seg.name(), &args, Some(context.context().authoring_query()))
            .contains(tcl_registry::Traits::CONTROL_FLOW);
        if is_control
            && registry.control_invocation_valid(
                seg.name(),
                &args,
                Some(context.context().authoring_query()),
            ) != Some(true)
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
                // Untyped is not the same as irrelevant. The
                // registry types the arms whose *selection* it models; a body
                // it does not type is still a body, and unless the command
                // declared that it stores one (`DEFERS_BODY`), it runs it
                // here — so `uplevel 1 {error stop}`, `eval {error stop}` and
                // `oo::define C {error stop}` cannot be walked past on the
                // strength of having no descriptor.
                let body_runs_now = self
                    .control_arm_body_index(&arm)
                    .is_none_or(|index| self.body_runs_now(&arm.controller, index));
                if body_runs_now && self.untyped_body_provably_blocks(arm.body_span, depth + 1) {
                    return false;
                }
                continue;
            };
            saw_typed_arm = true;
            match semantics {
                tcl_registry::registry::ControlArmSemantics::Always
                | tcl_registry::registry::ControlArmSemantics::FrameBoundary => {
                    saw_propagating_arm = true;
                    if !self.static_prefix_falls_through(
                        arm.body_span,
                        arm.body_span.end(),
                        depth + 1,
                    ) {
                        return false;
                    }
                }
                tcl_registry::registry::ControlArmSemantics::Selected => selected.push(arm),
                tcl_registry::registry::ControlArmSemantics::CompletionBoundary => {}
                // A collection loop runs its body a finite number of times, so
                // control reaches the next statement exactly when the body
                // falls through — the same question, and the same answer, as
                // an `if` arm whose selection is unknown (below). Nothing is
                // *read* out of the loop: it stays unselectable, and the
                // provenance walk still clears every fact the iteration
                // variables or the body could write.
                tcl_registry::registry::ControlArmSemantics::BoundedIteration => {
                    if !self.static_prefix_falls_through(
                        arm.body_span,
                        arm.body_span.end(),
                        depth + 1,
                    ) {
                        return false;
                    }
                }
                // A condition loop's trip count is not bounded by anything
                // readable here — it may run forever, so no claim is made
                // about the statement after it.
                tcl_registry::registry::ControlArmSemantics::Uncertain => return false,
            }
        }
        if !saw_typed_arm || selected.is_empty() {
            return true;
        }
        if saw_propagating_arm {
            return false;
        }

        self.selected_arms_fall_through(&selected, depth)
    }

    /// The `Selected`-arm half of [`Self::static_statement_falls_through`]:
    /// at most one arm of an `if` / `switch` runs, so control reaches the
    /// next statement when *that* arm does. An arm whose selection cannot be
    /// decided is treated as one that may run, which is the same requirement
    /// a [`ControlArmSemantics::BoundedIteration`] body carries.
    ///
    /// [`ControlArmSemantics::BoundedIteration`]: tcl_registry::registry::ControlArmSemantics::BoundedIteration
    fn selected_arms_fall_through(&mut self, selected: &[ControlArm], depth: u32) -> bool {
        let env = std::collections::HashMap::new();
        let mut selected_body = None;
        let mut selection_unknown = false;
        for arm in selected {
            match self.control_body_is_selected("", arm, &env, depth + 1) {
                Some(true) if selected_body.is_none() => selected_body = Some(arm.body_span),
                Some(true) => return false,
                Some(false) => {}
                None => {
                    selection_unknown = true;
                    if !self.static_prefix_falls_through(
                        arm.body_span,
                        arm.body_span.end(),
                        depth + 1,
                    ) {
                        return false;
                    }
                }
            }
        }
        if selection_unknown {
            return true;
        }
        selected_body
            .is_none_or(|span| self.static_prefix_falls_through(span, span.end(), depth + 1))
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
            class.factory = self.class_factory_of(&class);
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
            bind_proc_formals(&candidate.params, &actuals, self.word_rules())?
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
        let previous = self.static_eval_site;
        let mut applied = Some(());
        for seg in self.direct_statements_in_span(body_span)? {
            if seg.span.start() >= stop_off {
                break;
            }
            self.static_eval_site = Some(seg.span);
            if !self.static_statement_falls_through(&seg, 0) {
                applied = None;
                break;
            }
            if self
                .apply_static_provenance_statement(proc_qname, &seg, env, 0)
                .is_none()
            {
                applied = None;
                break;
            }
        }
        self.static_eval_site = previous;
        applied
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
        let context = self.retained_static_source_context()?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &arm.controller,
        )?;
        match self.control_arm_semantics_for(arm)? {
            tcl_registry::registry::ControlArmSemantics::Always => return Some(true),
            tcl_registry::registry::ControlArmSemantics::Selected => {}
            // A bounded collection loop is no more *selectable* than an
            // unbounded one — which iteration a body run belongs to, or
            // whether the collection was empty, is not decidable here.
            tcl_registry::registry::ControlArmSemantics::FrameBoundary
            | tcl_registry::registry::ControlArmSemantics::BoundedIteration
            | tcl_registry::registry::ControlArmSemantics::Uncertain => return None,
            tcl_registry::registry::ControlArmSemantics::CompletionBoundary => {
                return Some(true);
            }
        }
        let hook =
            words.with_source_schema(&context, |schema| schema.semantics.lowering_hook)??;
        let previous = self.static_eval_site.replace(arm.controller.span);
        let selected = match hook {
            tcl_registry::hooks::LoweringHookId::If => {
                self.if_body_is_selected(proc_qname, arm, env, depth)
            }
            tcl_registry::hooks::LoweringHookId::Switch => {
                self.switch_body_is_selected(proc_qname, arm, env, depth)
            }
            _ => None,
        };
        self.static_eval_site = previous;
        selected
    }

    /// Resolve typed semantics for either a direct body argument or one body
    /// nested inside a registry-described case-list argument. The latter's
    /// token span belongs to a list element, not the controller argv, so its
    /// owning clause-list position must be recovered through `CaseListSpec`.
    fn control_arm_semantics_for(
        &self,
        arm: &ControlArm,
    ) -> Option<tcl_registry::registry::ControlArmSemantics> {
        let input = self.result.resolved_input.as_ref()?;
        let context = self.retained_static_source_context()?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &arm.controller,
        )?;
        let ordinal = self.control_arm_body_index(arm)?;
        words.with_source_schema(&context, |schema| {
            if input.has_logical_source_name_context() {
                schema.authored_logical_source_control_arm_semantics(ordinal)
            } else {
                schema.authored_source_control_arm_semantics(ordinal)
            }
        })?
    }

    /// Recover the owning argv position for a direct body, a case-list body,
    /// or a body nested inside a lambda literal.
    ///
    /// Timing and control-arm semantics are both per script position, so they
    /// must share this mapping rather than falling back to an invocation-wide
    /// trait when a command mixes immediate and deferred scripts.
    fn control_arm_body_index(&self, arm: &ControlArm) -> Option<usize> {
        let context = self.retained_static_source_context()?;
        let config = self.file_lexer_config();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &self.cached_source_map(),
            config,
            &arm.controller,
        );
        let bindings = self.head_identities.source_bindings_ref();
        let origin = bindings.source_origin()?;
        let original = origin.original_lexical_command(
            tokens.words(),
            arm.controller.argv.first()?.span.start(),
            config,
        )?;
        let actual = original.command();
        if actual.span != arm.controller.span
            || actual.argv != arm.controller.argv
            || actual.texts != arm.controller.texts
            || actual.word_fragments != arm.controller.word_fragments
            || actual.single_token_word != arm.controller.single_token_word
            || actual.expand_word != arm.controller.expand_word
            || actual.is_partial != arm.controller.is_partial
            || actual.partial_delimiter != arm.controller.partial_delimiter
        {
            return None;
        }
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &arm.controller,
        )?;
        if words.origins().len() != arm.controller.argv.len()
            || words.origins().iter().enumerate().any(|(ordinal, origin)| {
                // A proved dispatch states the head as its implementation;
                // with no alias prefix it is still the written head word.
                use crate::registry_invocation::InvocationWordOrigin as Origin;
                !(*origin == Origin::Written(ordinal)
                    || (ordinal == 0 && *origin == Origin::ResolvedHead))
            })
        {
            return None;
        }
        let mut bodies = words
            .source_script_bodies_for(
                &context,
                crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
            )
            .into_iter()
            .chain(words.original_lambda_body_geometry(&context))
            .filter(|body| body.extent_span() == arm.body_span);
        let body = bodies.next()?;
        if bodies.next().is_some() {
            return None;
        }
        let mut ordinals = words
            .operands()
            .iter()
            .enumerate()
            .filter_map(|(ordinal, operand)| {
                (operand.as_ref()?.word()? == body.original_container()).then_some(ordinal)
            });
        let ordinal = ordinals.next()?;
        ordinals.next().is_none().then_some(ordinal)
    }

    /// Whether a body argument this walk could **not** read makes the arm set
    /// incomplete.
    ///
    /// Two registry facts, in order, and the walk abstains unless both say the
    /// body is irrelevant to it:
    ///
    /// 1. **Typed control semantics.** A body position the registry types
    ///    carries a fact the consumers below consume directly — whether it is
    ///    one arm of a selection (`if {$c} $script`), a frame boundary that
    ///    runs now (`namespace eval $ns $script`), a bounded iteration. An
    ///    unreadable body there is a missing answer, so the arm set really is
    ///    incomplete.
    /// 2. **[`ScriptTiming`].** With no typed semantics the question left is
    ///    whether this exact body argument runs as part of *this* invocation,
    ///    and that is registry data, never an inference. `proc ${ns}::define
    ///    {a} [string map … {…}]` stores its body — nothing in it executes
    ///    here, so an unreadable body word costs the walk nothing about the
    ///    declaration, which falls through like any other command. Legacy
    ///    positional bodies without an exact timing descriptor fall back to
    ///    the command-level [`Traits::DEFERS_BODY`] fact.
    ///
    /// The default is deliberately the abstaining one: a command that has not
    /// declared deferred timing is assumed to run its body now. The first
    /// version of this helper read the *absence* of control-arm semantics as
    /// proof of dormancy, which is not what that absence means —
    /// `uplevel 1 $script` and `oo::define $c $script` have no control-arm
    /// semantics either, and both execute their argument immediately and can
    /// raise or return before the enclosing walk reaches the statement it
    /// cares about. That let the computed-metaclass walk materialise a class
    /// created after `uplevel 1 $script`.
    /// `BodyKind` cannot answer it either: `proc` and `uplevel` are both
    /// `Structural`, because that descriptor answers *which frame*, not
    /// *when*.
    ///
    /// `body_index` may name an [`ArgRole::LambdaLiteral`] word as well as an
    /// [`ArgRole::Body`] one — an unreadable lambda is unreadable script on
    /// the same terms, and `apply` types that position, so it
    /// stops at the first check.
    ///
    /// [`ArgRole::Body`]: tcl_registry::ArgRole::Body
    /// [`ArgRole::LambdaLiteral`]: tcl_registry::ArgRole::LambdaLiteral
    /// [`ScriptTiming`]: tcl_registry::ScriptTiming
    /// [`Traits::DEFERS_BODY`]: tcl_registry::Traits::DEFERS_BODY
    fn unreadable_body_is_material(&self, seg: &SegmentedCommand, body_index: usize) -> bool {
        let Some(context) = self.retained_static_source_context() else {
            return true;
        };
        let registry = context.commands().as_ref();
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        if registry
            .control_arm_semantics(seg.name(), &args, body_index)
            .is_some()
        {
            return true;
        }
        self.body_runs_now(seg, body_index)
    }

    /// Whether `seg` runs its body argument as part of **this** invocation.
    ///
    /// The one place that answers it, for both the unreadable-body question
    /// above and the untyped-arm question in
    /// [`Self::static_statement_falls_through`] — the two were
    /// the same question asked in two places, and only one of them was asking
    /// it.
    ///
    /// Registry data, never an inference: an option or argument can declare
    /// its exact [`ScriptTiming`], so a command with both immediate and stored
    /// scripts does not force one answer onto both. Legacy positional bodies
    /// fall back to command-level [`Traits::DEFERS_BODY`]. Everything else is
    /// assumed to run now. That default is the abstaining direction — a body
    /// that runs can raise or return before the walk reaches the statement it
    /// cares about, while a body that is merely stored cannot.
    ///
    /// [`Traits::DEFERS_BODY`]: tcl_registry::Traits::DEFERS_BODY
    /// [`ScriptTiming`]: tcl_registry::ScriptTiming
    fn body_runs_now(&self, seg: &SegmentedCommand, body_index: usize) -> bool {
        let Some(context) = self.retained_static_source_context() else {
            return true;
        };
        let registry = context.commands().as_ref();
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        registry
            .script_timing(
                seg.name(),
                &args,
                body_index,
                Some(context.context().authoring_query()),
            )
            .is_none_or(|timing| timing == tcl_registry::ScriptTiming::SameInvocation)
    }

    /// Whether a body the registry gives **no** control-arm semantics for is
    /// proved to stop control before its own end.
    ///
    /// The typed arms above ask the opposite question — *prove* the body falls
    /// through, or abstain — because a descriptor says what the body is for. An
    /// untyped body carries no such promise, and asking it to prove
    /// fall-through would abstain on every script written in a vocabulary this
    /// walk does not model. A class definition script is the case that settles
    /// it: `oo::class create C { superclass D }` is an untyped body running
    /// now, and `superclass` is a definition command, not a command — nothing
    /// here can type it, so demanding a proof of fall-through would abstain on
    /// the one construct the computed-metaclass walk exists to read (the tcllib
    /// clay corpus is built out of exactly these).
    ///
    /// So the burden runs the other way: the enclosing statement is walked past
    /// unless *this* body proves it blocks. Both proofs are registry-owned and
    /// oracle-checked against tclsh 8.6.16 and 9.0.4, which agree exactly:
    ///
    /// * [`InvocationCompletion::Terminates`] — `uplevel 1 {error stop}`,
    ///   `eval {error stop}`, `oo::define C {error stop}`: the statement after
    ///   them never runs;
    /// * [`InvocationCompletion::ReturnsResult`] — `oo::define C {return}`,
    ///   `eval {return}`, `uplevel 1 {return}`: likewise, the completion
    ///   propagates out of the body rather than ending at it.
    ///
    /// A statement whose completion is [`InvocationCompletion::Unknown`] ends
    /// the scan with no proof, which leaves the enclosing walk exactly where it
    /// was before this rule existed. That is the honest limit of an untyped
    /// body, and it is why nested control inside one (`oo::define C {if {$x}
    /// {error stop} else {error stop}}`) is not detected: the `if` itself
    /// completes normally as far as the registry is concerned, and reading its
    /// arms requires control-flow facts unavailable to this lexical walk.
    ///
    /// [`InvocationCompletion::Terminates`]: tcl_registry::registry::InvocationCompletion::Terminates
    /// [`InvocationCompletion::ReturnsResult`]: tcl_registry::registry::InvocationCompletion::ReturnsResult
    /// [`InvocationCompletion::Unknown`]: tcl_registry::registry::InvocationCompletion::Unknown
    fn untyped_body_provably_blocks(&mut self, body_span: Span, depth: u32) -> bool {
        if depth >= MAX_UNKNOWN_BODY_DEPTH || self.retained_static_source_context().is_none() {
            return false;
        }
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return false;
        };
        for seg in statements {
            if self.static_statement_falls_through(&seg, depth) {
                continue;
            }
            let completion = match self.segment_invocation_completion(&seg) {
                // An untyped body's statements may have no source point of
                // their own on a release profile. Reading the written head
                // there can only make this walk abstain, never claim.
                tcl_registry::registry::InvocationCompletion::Unknown => {
                    self.written_head_completion(&seg)
                }
                completion => completion,
            };
            return matches!(
                completion,
                tcl_registry::registry::InvocationCompletion::Terminates
                    | tcl_registry::registry::InvocationCompletion::ReturnsResult(_)
            );
        }
        false
    }

    /// The registry's completion for the statement's written words, with no
    /// lookup behind it. Only the abstaining direction may read it.
    fn written_head_completion(
        &self,
        seg: &SegmentedCommand,
    ) -> tcl_registry::registry::InvocationCompletion {
        let Some(context) = self.retained_static_source_context() else {
            return tcl_registry::registry::InvocationCompletion::Unknown;
        };
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        context.commands().invocation_completion(
            seg.name(),
            &args,
            Some(context.context().authoring_query()),
        )
    }

    fn control_arms_for_segment(&self, seg: &SegmentedCommand) -> ControlArms {
        let Some(context) = self.retained_static_source_context() else {
            return ControlArms::default();
        };
        let registry = context.commands().as_ref();
        let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
        let mut indices =
            registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
        indices.sort_unstable();
        indices.dedup();
        let case_call = registry
            .case_invocation(seg.name(), &args, Some(context.context().authoring_query()))
            .and_then(|(_, invocation)| invocation.clause_list_index);
        let mut arms = Vec::new();
        let mut complete = true;
        for idx in indices {
            let (Some(word), Some(token)) =
                (seg.args().get(idx), seg.arg_tokens().get(idx).copied())
            else {
                complete &= !self.unreadable_body_is_material(seg, idx);
                continue;
            };
            if case_call == Some(idx) {
                let Some((case, _)) = registry.case_invocation(
                    seg.name(),
                    &args,
                    Some(context.context().authoring_query()),
                ) else {
                    complete = false;
                    continue;
                };
                for (_, (body_word, body_token)) in crate::segmenter::flatten_case_list_clauses(
                    &self.source,
                    word,
                    token,
                    &case,
                    self.lexer_config(),
                ) {
                    if body_token.kind == TokenType::Str
                        || (body_token.kind == TokenType::Esc
                            && !tcl_syntax::naming::is_dynamic_word(&body_word)
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
                complete &= !self.unreadable_body_is_material(seg, idx);
            }
        }
        self.collect_lambda_arms(seg, &args, &mut arms, &mut complete);
        ControlArms { arms, complete }
    }

    /// The [`ArgRole::LambdaLiteral`] half of [`Self::control_arms_for_segment`].
    ///
    /// A lambda argument carries a script that runs *now* — `apply $lambda`
    /// can raise before control ever reaches the next statement — but it is
    /// not an [`ArgRole::Body`], so the loop above never sees it. The walk
    /// therefore neither read such a lambda nor abstained on it, and claimed
    /// past an aborting one: the same soundness class as the `uplevel` shape
    /// above, reached through the other role.
    ///
    /// The argument is one level removed from a body — a `{argList body ?ns?}`
    /// **list** — so a readable one contributes its *body element*'s span,
    /// split by the shared [`crate::lambda_literal`] owner rather than
    /// re-segmented whole. A lambda whose
    /// body cannot be located at exact source offsets — a computed word, a
    /// malformed list, a body element that is not `{braced}` and so has its
    /// escapes collapsed before `apply` sees it — is unreadable, and falls to
    /// the same materiality question a `Body` word does.
    ///
    /// [`ArgRole::Body`]: tcl_registry::ArgRole::Body
    /// [`ArgRole::LambdaLiteral`]: tcl_registry::ArgRole::LambdaLiteral
    fn collect_lambda_arms(
        &self,
        seg: &SegmentedCommand,
        args: &[&str],
        arms: &mut Vec<ControlArm>,
        complete: &mut bool,
    ) {
        let Some(context) = self.retained_static_source_context() else {
            return;
        };
        let registry = context.commands().as_ref();
        let mut indices =
            registry.arg_indices_for_role(seg.name(), args, tcl_registry::ArgRole::LambdaLiteral);
        indices.sort_unstable();
        indices.dedup();
        for idx in indices {
            // Read the body only where the registry says what running it
            // means. `apply` types this position (a frame boundary that runs
            // now); a command that carries the role without typing it has told
            // the walk nothing, and an untyped script that runs now is exactly
            // what `unreadable_body_is_material` abstains on.
            let body = registry
                .control_arm_semantics(seg.name(), args, idx)
                .and_then(|_| seg.arg_tokens().get(idx).copied())
                .and_then(|token| crate::lambda_literal::split_lambda_literal(&self.source, token))
                .and_then(|elements| elements.braced_body());
            if let Some(body_span) = body {
                arms.push(ControlArm {
                    controller: seg.clone(),
                    body_span,
                });
            } else {
                *complete &= !self.unreadable_body_is_material(seg, idx);
            }
        }
    }

    fn if_body_is_selected(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<bool> {
        let context = self.retained_static_source_context()?;
        let words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &arm.controller,
        )?;
        let wanted = self.control_arm_body_index(arm)?;
        let clauses = words.with_source_schema(&context, |schema| {
            if schema.semantics.lowering_hook != Some(tcl_registry::hooks::LoweringHookId::If) {
                return None;
            }
            let plan = schema.clause_plan()?;
            if plan.defect.is_some() || plan.clauses.is_empty() {
                return None;
            }
            plan.clauses
                .iter()
                .map(|clause| {
                    if clause.timing != tcl_registry::clause_grammar::ClauseTiming::Selected
                        || clause.operands(tcl_registry::ArgRole::Body).count() != 1
                        || clause.operands(tcl_registry::ArgRole::Expr).count() > 1
                    {
                        return None;
                    }
                    Some((
                        clause.operand(tcl_registry::ArgRole::Expr),
                        clause.operand(tcl_registry::ArgRole::Body)?,
                    ))
                })
                .collect::<Option<Vec<_>>>()
        })??;
        for (condition, body) in clauses {
            let Some(condition) = condition else {
                return Some(body == wanted);
            };
            let original = words.original_argument_word(condition)?;
            let source = original.image().try_text().ok()?;
            let expression = source.get(original.content_span().ok()?.as_range())?;
            if self.static_expr_value(expression, proc_qname, env, &mut Vec::new(), depth)? {
                return Some(body == wanted);
            }
        }
        Some(false)
    }

    /// Whether the body at `arm.body_span` is the one the `switch` runs, when
    /// the environment states the words the registry's selection reads: the
    /// selection is the command's own (mode, case folding, ordered first
    /// match, the final `default`, fall-through, the regexp engine), never a
    /// match made here. `None` where any word is not stated, the selection
    /// declines, or the subject's members would run different bodies.
    fn switch_body_is_selected(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<bool> {
        let context = self.retained_static_source_context()?;
        let registry = context.commands();
        let args: Vec<&str> = arm.controller.args().iter().map(String::as_str).collect();
        let (case, invocation) = registry.case_invocation(
            arm.controller.name(),
            &args,
            Some(context.context().authoring_query()),
        )?;
        if usize::from(case.subject_args) != 1 {
            return None;
        }
        let values = self.switch_words(proc_qname, arm, &invocation, env, depth)?;
        let words: Vec<(&str, WordForm)> = values
            .iter()
            .map(|(text, form)| (text.as_str(), *form))
            .collect();
        let selection = literal_selection(registry, arm.controller.name(), &words, None)?;
        let bodies = self.switch_clause_bodies(arm, &case, &invocation)?;
        if bodies.len() != selection.clauses || Some(selection.subject) != invocation.subject_index
        {
            return None;
        }
        let mut verdicts = selection.fact.bodies.iter().map(|body| match body {
            None => Some(false),
            Some(clause) => bodies.get(*clause).map(|span| *span == arm.body_span),
        });
        let first = verdicts.next()??;
        verdicts
            .all(|verdict| verdict == Some(first))
            .then_some(first)
    }

    /// The words of a `switch` as its selection reads them, each with how it
    /// was written: the subject and the inline patterns as the environment
    /// states them (a braced one is its own text), every other word as
    /// written. A word left to substitute that is not the subject or a
    /// pattern is unproven, so a `-` body it might spell declines the
    /// selection.
    fn switch_words(
        &mut self,
        proc_qname: &str,
        arm: &ControlArm,
        invocation: &tcl_registry::spec::CaseInvocation,
        env: &std::collections::HashMap<String, String>,
        depth: u32,
    ) -> Option<Vec<(String, WordForm)>> {
        let controller = &arm.controller;
        let count = controller.args().len();
        let braced = braced_word_flags(
            controller.arg_tokens(),
            controller.arg_single_token(),
            count,
        );
        let quoted = quoted_word_flags(controller.arg_tokens(), count);
        let is_pattern = |index: usize| {
            invocation
                .inline_clause_start
                .is_some_and(|start| index >= start && (index - start).is_multiple_of(2))
        };
        let mut words = Vec::with_capacity(count);
        for (index, text) in controller.args().iter().enumerate() {
            let stated = invocation.subject_index == Some(index) || is_pattern(index);
            let form = if braced[index] {
                WordForm::Braced
            } else if quoted[index] {
                WordForm::Quoted
            } else {
                WordForm::Bare
            };
            words.push(if braced[index] {
                (
                    self.word_rules().collapse_braced_word(text).into_owned(),
                    form,
                )
            } else if stated {
                let value =
                    self.static_word_value(text, proc_qname, env, &mut Vec::new(), depth)?;
                (value, form)
            } else if ['$', '[', '\\'].iter().any(|mark| text.contains(*mark)) {
                (text.clone(), WordForm::Unproven)
            } else {
                (text.clone(), form)
            });
        }
        Some(words)
    }

    /// The body span of each clause of a `switch`, in the command's order:
    /// the clause-list word's bodies, or every second inline word after the
    /// patterns begin.
    fn switch_clause_bodies(
        &self,
        arm: &ControlArm,
        case: &tcl_registry::spec::CaseListSpec,
        invocation: &tcl_registry::spec::CaseInvocation,
    ) -> Option<Vec<Span>> {
        let args = arm.controller.args();
        let tokens = arm.controller.arg_tokens();
        if let Some(list_index) = invocation.clause_list_index {
            let clauses = crate::segmenter::flatten_case_list_clauses(
                &self.source,
                args.get(list_index)?,
                *tokens.get(list_index)?,
                case,
                self.lexer_config(),
            );
            return Some(
                clauses
                    .into_iter()
                    .map(|(_, (_, body))| body.span)
                    .collect(),
            );
        }
        let start = invocation.inline_clause_start?;
        (start..args.len().saturating_sub(1))
            .step_by(2)
            .map(|pattern| tokens.get(pattern + 1).map(|token| token.span))
            .collect()
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
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
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
        for seg in crate::segmenter::segment_commands_with_offset_and_config(
            body,
            0,
            self.file_lexer_config().nested(),
        ) {
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
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
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
                    let Ok(names) = self.word_rules().split_list(word) else {
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
                    let base = tcl_syntax::naming::split_element_ref(&name)
                        .map_or(name.as_str(), |(base, _)| base);
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
                .filter(|sub| !sub.side_effects.lower_bound().is_empty())
                .map_or(spec.side_effects, |sub| sub.side_effects);
            if effects.lower_bound().iter().any(|effect| {
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
        if let Some((head, args)) = crate::value_shapes::parse_command_substitution_with_config(
            unquoted,
            self.file_lexer_config().nested(),
        ) {
            if !self.result.allows_lexical_declaration_advice() {
                let seg = self.static_substitution_segment(unquoted)?;
                return self.static_segment_value(&seg, proc_qname, env, stack, depth);
            }
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
        substitute_bound_words(unquoted, &names, &values, self.file_lexer_config())
    }

    /// The actual source input held by this walk, including availability.
    /// Transient walk caches and reconstructed environment labels cannot
    /// replace its original source, full configuration or realm.
    fn retained_static_source_context(
        &self,
    ) -> Option<std::sync::Arc<tcl_registry::model::ContextRegistry>> {
        let input = self.result.resolved_input.as_ref()?;
        let config = self.file_lexer_config();
        if self
            .resolved_input
            .as_ref()
            .is_some_and(|current| current != input)
            || self.profile.cache_key() != input.analyser_profile().cache_key()
            || self.unit_profile.unwrap_or(self.profile).cache_key()
                != input.unit_profile().cache_key()
            || input.lexer_config() != config
            || !self.result.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&self.source),
                config,
            )
            || !self.head_identities.matches_resolved_analysis_input(input)
            || self.retained_source_advice_purpose().is_none()
        {
            return None;
        }
        Some(input.context_registry())
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
        // A materialised String command has no genuine original call point,
        // occupied implementation or captured prefix. Registry folds here
        // belong only to the explicit Logical assistance domain.
        if !self.result.allows_lexical_declaration_advice() || depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
        if registry.get(head).is_some() {
            if !self.static_provenance_command_is_trusted(&crate::naming::qualify("::", head)) {
                return None;
            }
            return self.static_registry_fold(head, args);
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

    /// Fold a registry command whose occupied implementation the caller has
    /// already established, lexically or at its source position.
    fn static_registry_fold(&mut self, head: &str, args: &[Option<String>]) -> Option<String> {
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
        let spec = context.context().resolve_spec(registry, head)?;
        let values: Vec<&str> = args
            .iter()
            .map(|value| value.as_deref())
            .collect::<Option<_>>()?;
        let dialect = self
            .source_analysis_entry
            .as_ref()
            .and_then(|entry| entry.invocation_dialect)
            .map(|mut dialect| {
                dialect.lexer_grammar = self.grammar();
                dialect.word_values = self.word_rules();
                dialect
            });
        if spec.subcommands.is_empty() {
            return match dialect {
                Some(dialect) => spec.run_const_fold_in(&values, dialect),
                None => spec.run_const_fold(&values, None),
            };
        }
        let (sub, rest) = values.split_first()?;
        let subcommand = spec.resolve_subcommand(sub)?;
        match dialect {
            Some(dialect) => subcommand.run_const_fold_in(rest, dialect),
            None => subcommand.run_const_fold(rest, None),
        }
    }

    /// The one command of a `[…]` substitution written inside the statement
    /// under evaluation, at its own source position. Two identical
    /// substitutions in one statement abstain.
    fn static_substitution_segment(&self, substitution: &str) -> Option<SegmentedCommand> {
        let site = self.static_eval_site?;
        let statement = self
            .source
            .get(site.start() as usize..site.end() as usize)?;
        let mut found = statement.match_indices(substitution);
        let (at, _) = found.next()?;
        if found.next().is_some() {
            return None;
        }
        let start = site
            .start()
            .checked_add(u32::try_from(at).ok()?)?
            .checked_add(1)?;
        let end = start.checked_add(u32::try_from(substitution.len()).ok()?.checked_sub(2)?)?;
        let mut commands = self.direct_statements_in_span(Span::new(start, end))?;
        if commands.len() != 1 {
            return None;
        }
        commands.pop()
    }

    /// Evaluate one command at its source position. The head is the
    /// procedure or registry builtin that position is proved to dispatch to;
    /// its spelling selects nothing.
    fn static_segment_value(
        &mut self,
        seg: &SegmentedCommand,
        proc_qname: &str,
        env: &std::collections::HashMap<String, String>,
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        if depth > MAX_UNKNOWN_BODY_DEPTH {
            return None;
        }
        let previous = self.static_eval_site.replace(seg.span);
        let values: Vec<Option<String>> = seg
            .args()
            .iter()
            .map(|arg| self.static_word_value(arg, proc_qname, env, stack, depth))
            .collect();
        let result = self.static_positioned_command_value(seg, &values, stack, depth);
        self.static_eval_site = previous;
        result
    }

    fn static_positioned_command_value(
        &mut self,
        seg: &SegmentedCommand,
        args: &[Option<String>],
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        if let Some(proc_def) = self.original_static_procedure_at(seg) {
            // The position's own lookup proves the implementation. The
            // head must still be that procedure's own name (an alias does
            // not lend it), and a command-table mutation that may name it
            // anywhere in the document blocks the fold, as on the lexical
            // path.
            let qname = proc_def.qualified_name.clone();
            if tcl_syntax::naming::key_tail(seg.name()) != tcl_syntax::naming::key_tail(&qname)
                || !self.static_provenance_command_is_trusted(&qname)
                || stack.iter().any(|seen| seen == &qname)
            {
                return None;
            }
            stack.push(qname);
            let result = self.static_proc_def_result(&proc_def, args, stack, depth + 1);
            stack.pop();
            return result;
        }
        (self.original_static_builtin_at(seg)
            && self.static_provenance_command_is_trusted(&crate::naming::qualify("::", seg.name())))
        .then(|| self.static_registry_fold(seg.name(), args))?
    }

    /// Whether this position is proved to dispatch to the registry builtin
    /// its head spells, with no alias prefix.
    fn original_static_builtin_at(&self, segment: &SegmentedCommand) -> bool {
        let config = self.file_lexer_config();
        if !self
            .result
            .matches_original_source_image(&tcl_lexer::SourceImage::document(&self.source), config)
        {
            return false;
        }
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            ),
            config,
            segment,
        );
        self.head_identities.stamp_original_tokens(&mut tokens);
        tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.proved_execution_target())
            .is_some_and(|target| {
                target.registry_backed
                    && target.kind == crate::command_binding::BindingKind::Builtin
                    && target.prepended.is_empty()
                    && crate::naming::qualify("::", segment.name()) == target.command
            })
    }

    /// On a release profile a call of a command the registry does not
    /// describe has no schema to read; its body, if the document declares
    /// one, is a definition body the trust scan skips.
    fn release_call_has_no_schema(
        &self,
        registry: &tcl_registry::CommandRegistry,
        seg: &SegmentedCommand,
    ) -> bool {
        self.result
            .resolved_input
            .as_ref()
            .is_some_and(|input| !input.has_logical_source_name_context())
            && registry.get(seg.name()).is_none()
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
        let Some(context) = self.retained_static_source_context() else {
            return false;
        };
        let registry = context.commands().as_ref();
        let Some(statements) = self.statements_in_span(Span::new(
            0,
            u32::try_from(self.source.len()).unwrap_or(u32::MAX),
        )) else {
            return false;
        };
        let expected_declarations = usize::from(self.result.all_procs.contains_key(query));
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
            let Some(original) =
                crate::registry_invocation::source_structure::source_registry_words(
                    &self.source,
                    &self.result,
                    &seg,
                )
            else {
                if self.release_call_has_no_schema(registry, &seg) {
                    continue;
                }
                return false;
            };
            let Some(transitions) = original.with_source_schema(&context, |schema| {
                schema
                    .semantics
                    .state_transitions
                    .resolve_after_success_with_effect_coverage(schema.words.arguments())
                    .0
            }) else {
                return false;
            };
            let input = self
                .result
                .resolved_input
                .as_ref()
                .expect("checked original source input");
            let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
                registry,
                input,
                self.file_lexer_config(),
                Some(input.unit_profile()),
            );
            let overlap = |subject: &tcl_registry::TransitionSubject| {
                let word = original.original_argument_word(subject.argument_index()?)?;
                crate::dynamic_names::original_command_name_overlap(word, query, metadata).or_else(
                    || {
                        // Outside the Logical domain no fragment of a
                        // computed word is read: it may name anything.
                        (!input.has_logical_source_name_context()).then(|| {
                            crate::dynamic_names::release_command_name_overlap(word, query)
                        })
                    },
                )
            };
            for transition in transitions.command_bindings() {
                match transition {
                    tcl_registry::CommandBindingTransition::Define { name, .. } => {
                        let Some(name) = overlap(name) else {
                            return false;
                        };
                        if name.may_equal {
                            if name.dynamic {
                                return false;
                            }
                            matching_declarations += 1;
                            if matching_declarations > expected_declarations {
                                return false;
                            }
                        }
                    }
                    tcl_registry::CommandBindingTransition::Move { from, to } => {
                        let (Some(old), Some(new)) = (overlap(from), overlap(to)) else {
                            return false;
                        };
                        if old.may_equal || (!new.empty && new.may_equal) {
                            return false;
                        }
                    }
                    tcl_registry::CommandBindingTransition::Delete { name, .. }
                    | tcl_registry::CommandBindingTransition::Alias { alias: name, .. } => {
                        let Some(name) = overlap(name) else {
                            return false;
                        };
                        if name.may_equal {
                            return false;
                        }
                    }
                    // A mutation whose very shape is unknown could name
                    // anything.
                    tcl_registry::CommandBindingTransition::Unknown { .. } => return false,
                }
            }
        }
        matching_declarations == expected_declarations
    }

    /// A genuine positioned occupied implementation for static continuation.
    /// Metadata joins its original allocation; a final report name cannot
    /// supply a replacement, captured prefix or current command identity.
    fn original_static_procedure_at(&self, segment: &SegmentedCommand) -> Option<ProcDef> {
        let config = self.file_lexer_config();
        self.result
            .matches_original_source_image(&tcl_lexer::SourceImage::document(&self.source), config)
            .then_some(())?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &Self::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            ),
            config,
            segment,
        );
        self.head_identities.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref()?;
        let target = binding.proved_execution_target()?;
        if target.registry_backed
            || target.kind != crate::command_binding::BindingKind::Proc
            || !target.prepended.is_empty()
        {
            return None;
        }
        let reference = binding.evaluated_command_reference()?;
        let definition = reference
            .linked_definition()
            .or_else(|| reference.definition())?;
        if Some(definition.allocation()) != target.implementation_allocation.as_ref() {
            return None;
        }
        self.result
            .proc_for_definition(definition, &self.source)
            .cloned()
    }

    /// Explicit Logical assistance only. This unpositioned String interface
    /// has no original call/allocation point from which Native or hosted
    /// occupied implementation could be proved.
    fn resolve_static_proc_name(&self, caller: &str, head: &str) -> Option<String> {
        if !self.result.allows_lexical_declaration_advice() {
            return None;
        }
        let absolute = if tcl_syntax::naming::is_absolute(head) {
            crate::naming::qualify("", head)
        } else {
            let (ns, _) = crate::naming::key_holder_and_tail(caller);
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
        // Materialised substitution strings have no original operation point.
        // A positioned caller must use original_static_procedure_at instead.
        if !self.result.allows_lexical_declaration_advice() {
            return None;
        }
        let proc_def = self.result.all_procs.get(qname)?.clone();
        self.static_proc_def_result(&proc_def, args, stack, depth)
    }

    fn static_proc_def_result(
        &mut self,
        proc_def: &ProcDef,
        args: &[Option<String>],
        stack: &mut Vec<String>,
        depth: u32,
    ) -> Option<String> {
        self.retained_static_source_context()?;
        if proc_def.params_computed {
            return None;
        }
        let mut env = std::collections::HashMap::new();
        for (name, value) in bind_proc_formals(&proc_def.params, args, self.word_rules())? {
            if let Some(value) = value {
                env.insert(name, value);
            }
        }
        let qname = proc_def.qualified_name.as_str();
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
        let previous = self.static_eval_site;
        let outcome = self.static_script_statements(qname, body_span, env, stack, depth);
        self.static_eval_site = previous;
        outcome
    }

    fn static_script_statements(
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
        let context = self.retained_static_source_context()?;
        let registry = context.commands().as_ref();
        let statements = self.direct_statements_in_span(body_span)?;
        let mut result = String::new();
        for seg in statements {
            self.static_eval_site = Some(seg.span);
            let raw_args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            match self.segment_invocation_completion(&seg) {
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
                    Some(context.context().authoring_query()),
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
            result = if self.result.allows_lexical_declaration_advice() {
                self.static_command_value(seg.name(), &values, qname, env, stack, depth)?
            } else {
                self.static_positioned_command_value(&seg, &values, stack, depth)?
            };
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
        let context = self.retained_static_source_context()?;
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
        // Each value is its text, which the expression engine reads as Tcl
        // reads a literal operand.
        let constants: crate::tcl_expr_eval::Env = env
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    crate::tcl_expr_eval::EnvValue::Str(value.clone()),
                )
            })
            .collect();
        crate::tcl_expr_eval::evaluate_expr_with_constants(
            &crate::parse_expr_for_profile(&expr, Some(self.profile)),
            &constants,
            crate::tcl_expr_eval::FoldPolicy::from_registry(context.commands()),
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
            self.file_lexer_config().nested(),
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
        let (owner_ns, _) = crate::naming::key_holder_and_tail(&candidate.proc_qname);
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
                && !crate::var_refs::scan_var_ref_forms_with_config(name_word, self.lexer_config())
                    .is_empty()
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
                .case_invocation(
                    seg.name(),
                    &args,
                    Some(self.analysis_context().context().authoring_query()),
                )
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
                    let Some((case, _)) = registry.case_invocation(
                        seg.name(),
                        &args,
                        Some(self.analysis_context().context().authoring_query()),
                    ) else {
                        continue;
                    };
                    for (_, (_, body_token)) in crate::segmenter::flatten_case_list_clauses(
                        &self.source,
                        word,
                        token,
                        &case,
                        self.lexer_config(),
                    ) {
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
        let Some(context) = self.retained_static_source_context() else {
            return;
        };
        let registry = context.commands().as_ref();
        let Some(statements) = self.direct_statements_in_span(body_span) else {
            return;
        };
        for seg in statements {
            if !self
                .result
                .offset_is_inside_any_definition_body(seg.span.start())
                && !seg.name().is_empty()
            {
                let qualified = crate::naming::qualify("::", seg.name());
                out.entry(qualified).or_default().push(LoadTimeCall {
                    args: seg.args().to_vec(),
                    control_path: control_path.clone(),
                    call_off: seg.span.start(),
                });
            }

            let args: Vec<&str> = seg.args().iter().map(String::as_str).collect();
            let mut body_indices =
                registry.arg_indices_for_role(seg.name(), &args, tcl_registry::ArgRole::Body);
            body_indices.sort_unstable();
            body_indices.dedup();
            let case_call = registry
                .case_invocation(seg.name(), &args, Some(context.context().authoring_query()))
                .and_then(|(_, invocation)| invocation.clause_list_index);
            for body_idx in body_indices {
                let (Some(word), Some(token)) = (
                    seg.args().get(body_idx),
                    seg.arg_tokens().get(body_idx).copied(),
                ) else {
                    continue;
                };
                if case_call == Some(body_idx) {
                    let Some((case, _)) = registry.case_invocation(
                        seg.name(),
                        &args,
                        Some(context.context().authoring_query()),
                    ) else {
                        continue;
                    };
                    for (_, (_, body_token)) in crate::segmenter::flatten_case_list_clauses(
                        &self.source,
                        word,
                        token,
                        &case,
                        self.lexer_config(),
                    ) {
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

    /// Original formal arguments supplying the selected manufacturer's name
    /// and body roles. Unrepresented or transformed contributions refuse.
    fn manufacturer_word_positions(
        original: &super::types::OriginalSourceMethodNameBody<'_>,
        builtin: &tcl_registry::definer::ManufacturerMethod,
        arguments: &[&tcl_lexer::NativeWord],
    ) -> Option<(usize, usize)> {
        let name = usize::from(builtin.names_instance_at?).checked_sub(1)?;
        let body = usize::from(builtin.definition_body_at?).checked_sub(1)?;
        let name_arg = original
            .scalar_parameter_read(arguments.get(name)?)?
            .checked_add(1)?;
        let body_arg = original
            .body_parameter_read(arguments.get(body)?)?
            .checked_add(1)?;
        (name_arg != body_arg).then_some((name_arg, body_arg))
    }

    fn manufacturer_next_call(
        &self,
        original: &super::types::OriginalSourceMethodNameBody<'_>,
        context: &super::types::OriginalLexicalMemberContext,
    ) -> Option<super::types::OriginalLexicalNextCall> {
        if context.body_word() != original.body_word() {
            return None;
        }
        let word = original.body_word();
        let body_span = word.content_span().ok()?;
        let body = word.image().try_text().ok()?.get(body_span.as_range())?;
        crate::segmenter::segment_commands_with_offset_and_config(
            body,
            body_span.start(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript.config(word.config()),
        )
        .into_iter()
        .filter_map(|command| {
            let head = command.argv.first()?;
            super::types::OriginalLexicalNextCall::from_source(
                context,
                &self.result,
                &self.source,
                head,
            )
        })
        .find(|call| !call.has_target_class())
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
        original: &super::types::OriginalSourceMethodNameBody<'_>,
        prologue: &tcl_lexer::NativeWord,
    ) -> Option<Vec<FactoryMember>> {
        let params = original.parameter_names().collect::<Vec<_>>();
        let mut injected: Vec<FactoryMember> = Vec::new();
        for piece in prologue_pieces(prologue) {
            match piece {
                ProloguePiece::Separator => {}
                ProloguePiece::VarRead(name) if params.contains(&name) => {}
                ProloguePiece::Substitution(group) => {
                    injected.push(self.injected_member_from_group(meta, group, original)?);
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
    /// reference-only member built by a source-selected list-argument constructor.
    fn injected_member_from_group(
        &self,
        meta: &UserMetaclass,
        group: Token,
        original: &super::types::OriginalSourceMethodNameBody<'_>,
    ) -> Option<FactoryMember> {
        let sm = Self::source_map(
            &self.source,
            &self.cached_line_index,
            self.cached_line_index_source_len,
        );
        let descended = descend_token(&sm, group, self.file_lexer_config());
        let mut segs = segments_from_tree(descended.tree(), &sm).into_iter();
        let seg = segs.next()?;
        // One group builds one member; a `[a; b]` compound is not a shape
        // the prologue reader accounts for.
        if segs.next().is_some() {
            return None;
        }
        // The registry words confirm the selected list constructor. Their
        // argument vector is the one recorded when the body was entered,
        // where a `{*}` splice has already expanded to that call's element
        // count, so the template itself is read from the written words.
        let source_words = crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            &seg,
        )?;
        let context = self.result.resolved_input.as_ref()?.context_registry();
        source_words.with_source_schema(&context, |schema| {
            schema.authored_source_list_operand_templates()
        })??;
        if source_words.origins().iter().skip(1).any(|origin| {
            !matches!(
                origin,
                crate::registry_invocation::InvocationWordOrigin::Written(_)
            )
        }) {
            return None;
        }
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.file_lexer_config();
        let mut words = Vec::new();
        for token in seg.arg_tokens() {
            let (word, _) = [tcl_lexer::word_span(&sm, *token), token.span]
                .into_iter()
                .find_map(|span| {
                    self.head_identities
                        .original_written_word_at_span_in_source(&image, span, config)
                })?;
            let value = if word.group().expand {
                FactoryWord::CallerSplice(original.scalar_parameter_splice(&word)?.checked_add(1)?)
            } else {
                FactoryWord::Literal {
                    text: String::from_utf8(
                        tcl_syntax::word_rules::original_static_word_unicode_value(&word)?,
                    )
                    .ok()?,
                    token: *token,
                }
            };
            words.push(value);
        }
        let FactoryWord::Literal { text: keyword, .. } = words.first()? else {
            return None;
        };
        meta.grammar.member(keyword)?.all_args_ref?;
        Some(FactoryMember { words })
    }

    /// Readonly factory metadata and its same original call arguments.
    /// Foreign headers require the source graph's separate occupancy facet;
    /// neither tier proves that a custom manufacturer executes or completes.
    fn class_factory_for_command(
        &self,
        args: &[String],
        arg_tokens: &[Token],
        call_off: u32,
    ) -> Option<(ClassFactory, Vec<tcl_lexer::NativeWord>)> {
        if let Some(call) = crate::registry_invocation::source_structure::source_constructor_call_at(
            &self.source,
            &self.result,
            call_off,
        ) {
            if (0..arg_tokens.len()).any(|index| call.written_argument(index) != Some(index))
                || !super::original_definer::constructor_arguments_match(&call, args, arg_tokens)
            {
                return None;
            }
            let declaration = call.class_declaration();
            let class = if self.result.allows_retained_logical_declaration_advice() {
                declaration.logical_source_class(&self.result)?
            } else {
                declaration.source_class(&self.result)?.metadata()
            };
            let original_arguments = (0..arg_tokens.len())
                .map(|ordinal| call.argument_word(ordinal).cloned())
                .collect::<Option<Vec<_>>>()?;
            return Some((class.factory.clone()?, original_arguments));
        }
        let request =
            crate::registry_invocation::source_structure::source_logical_command_candidates_at(
                &self.source,
                &self.result,
                call_off,
            )?;
        if request.has_earlier_unknown_mutations()
            || request.has_lookup_operand_effects()
            || request.argument_count() != Some(arg_tokens.len())
            || !super::original_definer::logical_arguments_match(
                request.original_words(),
                args,
                arg_tokens,
            )
        {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let policy = tcl_registry::InvocationDialect::of_profile(input.unit_profile())
            .authored_name_policy()?;
        let index = self.workspace_class_factories.as_ref()?;
        let foreign = request
            .external_header_coordinates()?
            .iter()
            .find_map(|coordinate| {
                // Project an existing slot outward to the legacy index only
                // when the shared recipe preserves every component boundary.
                let key = tcl_syntax::naming::native_command_source_spelling(
                    policy.recipe(),
                    coordinate,
                )?;
                index.get(&key)
            })?;
        Some((
            self.foreign_factory_at_call(foreign, arg_tokens)?,
            request.original_words().iter().skip(1).cloned().collect(),
        ))
    }

    fn foreign_factory_at_call(
        &self,
        foreign: &ClassFactory,
        arg_tokens: &[Token],
    ) -> Option<ClassFactory> {
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
    /// Superclass words retain the original definition caller's namespace,
    /// independently of the manufactured class's holder or reported name.
    /// Missing relation context supplies no superclass identity.
    ///
    /// Chain walking is depth-bounded and visited-checked, so a cyclic
    /// `superclass` declaration (rejected by real Tcl, but writable in a
    /// half-edited buffer) terminates.
    fn user_metaclass_of_class(&self, seed: &ClassDef) -> Option<UserMetaclass> {
        let context = self.analysis_context();
        let provider =
            super::class_hierarchy::original_metadata::original_registry_metaclass_provider(
                &self.result,
                seed,
                &context,
            )?;
        let descriptor = provider.registry_descriptor(&context)?;
        Some(UserMetaclass {
            root_command: descriptor.name.to_owned(),
            root_policy: provider.policy(),
            root_slot: provider.source_slot().clone(),
            grammar: descriptor.definition_body?,
        })
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
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) -> Option<ResolvedClassCreation> {
        if args.is_empty() || arg_tokens.is_empty() {
            return None;
        }
        let context = self.analysis_context();
        let original_definer = super::original_definer::OriginalClassDefinerSource::capture(
            &self.source,
            &self.result,
            cmd_tok.span.start(),
        );
        let registry_definer = if let Some(original) = &original_definer {
            original.matches_arguments(args, arg_tokens).then_some(())?;
            let descriptor = original.registry_descriptor(&context)?;
            (descriptor
                .traits
                .contains(tcl_registry::Traits::IS_OO_METACLASS)
                && descriptor.definition_body.is_some_and(|grammar| {
                    grammar.family == tcl_registry::definer::DefinerFamily::TclOo
                }))
            .then_some(())?;
            Some(descriptor)
        } else {
            None
        };
        let user_factory = if registry_definer.is_some() {
            None
        } else {
            Some(self.class_factory_for_command(args, arg_tokens, cmd_tok.span.start())?)
        };
        let manufacturer = match user_factory.as_ref() {
            None => tcl_registry::CommandRegistry::manufacturer_method_for_descriptor(
                registry_definer?,
                &args[0],
            )
            .filter(|method| {
                method.visibility == tcl_registry::definer::MemberVisibility::Exported
            }),
            Some((factory, _)) => self
                .definition_grammar(&factory.root_metaclass)
                .and_then(|grammar| grammar.manufacturer(&args[0])),
        }?;
        if user_factory.as_ref().is_some_and(|(factory, _)| {
            !factory
                .exported_manufacturers
                .contains(manufacturer.keyword)
        }) {
            return None;
        }
        let layout = match user_factory.as_ref() {
            None => ManufacturerLayout::builtin(manufacturer),
            Some((factory, original_arguments)) => {
                manufacturer_layout(factory, manufacturer, args, Some(original_arguments))
            }
        }?;
        Some(ResolvedClassCreation {
            user_factory: user_factory.map(|(factory, _)| factory),
            registry_metaclass: registry_definer,
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
        self.handle_oo_class_source(super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok: Some(cmd_tok),
            original: None,
        })
    }

    pub(super) fn handle_oo_class_source(
        &mut self,
        call: super::original_definer::ClassDefinerCall<'_>,
    ) -> bool {
        let super::original_definer::ClassDefinerCall {
            cmd_name,
            args,
            arg_tokens,
            scope_path,
            cmd_tok,
            original,
        } = call;
        let Some(cmd_tok) = cmd_tok else {
            return false;
        };
        // Keep the written head for original source and factory lookups.
        let Some(ResolvedClassCreation {
            user_factory,
            registry_metaclass,
            layout,
        }) = self.resolve_class_creation(args, arg_tokens, cmd_tok)
        else {
            // Anonymous class manufacturers have no source-level class name
            // to put in `all_classes`; ordinary instance manufacturers have
            // no class-definition body. Both are handled by their respective
            // object-value paths instead.
            //
            // …and a third case, which is not a verdict at all: the head may
            // name a metaclass this document proves only *after* the walk.
            // Keep it for the post-pass.
            self.defer_class_creation(cmd_name, args, arg_tokens, scope_path, cmd_tok);
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
        let original_name = original
            .and_then(super::original_definer::OriginalClassDefinerSource::original_name)
            .or_else(|| self.original_provider_class_name(arg_tokens[layout.name_arg].span));
        let source_name = original_name.as_ref().and_then(|original| {
            crate::signature_scan::scope::SignatureSourceCommand::object_from_key(
                &self.declaration_namespace_scope(scope_path)?,
                original.name_input(),
            )
        });
        let qualified = source_name
            .as_ref()
            .and_then(crate::signature_scan::scope::SignatureSourceCommand::reported_full_name)
            .unwrap_or_else(|| qualify(&ns_prefix, raw_name));
        let simple = source_name
            .as_ref()
            .and_then(crate::signature_scan::scope::SignatureSourceCommand::simple_name)
            .unwrap_or_else(|| crate::naming::key_tail(&qualified).to_string());
        let name_span = arg_tokens[layout.name_arg].span;
        // **W314** — the class name has no absolute written form.
        self.emit_w314_no_absolute_name(raw_name, name_span);
        let body_tok_opt = arg_tokens.get(layout.body_arg).copied();
        let body_span = body_tok_opt.map_or(name_span, |t| t.span);
        let doc = std::mem::take(&mut self.last_comment);
        let metaclass_lookup = self.declaration_name_policy().and_then(|policy| {
            crate::signature_scan::scope::SignatureSourceLookup::new(
                policy,
                self.declaration_namespace_scope(scope_path)?,
                cmd_name.to_owned(),
            )
        });
        let mut class = super::types::ClassDef {
            source_name,
            metaclass_lookup,
            name: simple,
            qualified_name: qualified.clone(),
            name_span,
            body_span,
            metaclass: registry_metaclass.map_or_else(
                || cmd_name.to_owned(),
                |descriptor| descriptor.name.to_owned(),
            ),
            metaclass_provenance: MetaclassProvenance::Observed,
            doc,
            inheritance_unknown: layout.inheritance_unknown,
            ..Default::default()
        };
        // A user metaclass has no spec of its own, so the bodies it makes are
        // governed by the definition grammar of the registry metaclass at the
        // root of its superclass chain — the same `TclOO` grammar, reached
        // without naming it here.
        let grammar = original
            .map(super::original_definer::OriginalClassDefinerSource::grammar)
            .or_else(|| {
                user_factory.as_ref().map_or_else(
                    || registry_metaclass.and_then(|descriptor| descriptor.definition_body),
                    |factory| self.definition_grammar(&factory.root_metaclass),
                )
            });
        self.apply_original_created_class_body(&mut class, &layout, grammar, call, body_tok_opt);
        // **W315** — a retraction the body could not legally make. Drained
        // here, after the whole body walk, so a class extended by several
        // `oo::define` blocks reports each block's own aborts once.
        self.emit_w315_definition_cannot_run(&mut class);
        // Is the class we just recorded *itself* a class factory?  Answered
        // here, once, so every later `ThisClass create …` — in this file or,
        // via the workspace factory index, in another — reads a derived fact
        // instead of re-walking the superclass chain and re-segmenting the
        // manufacturer override per call site.
        class.factory = self.class_factory_of(&class);
        // …and does the *manufacturer* let this class be constructed by a
        // bare word (Tk's `::tk::IconList .il`)?  Settled here too, for the
        // same reason: the proof lives on the metaclass, which a consuming
        // document may never see, so the answer travels on the class rather
        // than the question.
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
        self.retain_manufactured_class_declaration(
            original_name.as_ref(),
            qualified,
            class,
            scope_path,
        );
        true
    }

    fn apply_original_created_class_body(
        &mut self,
        class: &mut super::types::ClassDef,
        layout: &ManufacturerLayout,
        grammar: Option<&'static tcl_registry::definer::DefinitionBodyGrammar>,
        call: super::original_definer::ClassDefinerCall<'_>,
        body_token: Option<Token>,
    ) {
        self.apply_manufactured_class_members(grammar, &layout.injected, class);
        if let (Some(body_text), Some(body_tok)) = (call.args.get(layout.body_arg), body_token) {
            let disabled = self.command_dialect_disabled(call.cmd_name);
            let body = call.cmd_tok.zip(grammar).and_then(|(head, grammar)| {
                self.original_class_definition_body_at(head.span.start(), body_tok, grammar)
            });
            let enclosing = std::mem::replace(&mut self.original_class_definition_body, body);
            self.parse_oo_definition_body(
                body_text,
                body_tok,
                class,
                call.scope_path,
                grammar,
                disabled,
            );
            self.original_class_definition_body = enclosing;
        }
    }

    pub(super) fn original_class_definition_body_at(
        &self,
        offset: u32,
        token: Token,
        grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    ) -> Option<std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBody>> {
        original_class_definition_body_in_analysis(
            &self.source,
            &self.result,
            offset,
            token,
            grammar,
        )
    }

    /// Class definition advice from the selected definition grammar.
    /// Native source targets require a genuine original class/configuration
    /// receipt; unjoined targets retain only lexical body analysis. Positively
    /// retained Logical inputs can also publish cross-document advisory stubs.
    /// Reporting tables establish neither Native identity nor definition entry.
    pub fn handle_oo_define_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) -> bool {
        let enclosing = self.original_class_definition_body.take();
        let result = self
            .handle_oo_define_with_source(cmd_name, args, arg_tokens, arg_single, scope_path, None);
        self.original_class_definition_body = enclosing;
        result
    }

    pub(super) fn handle_oo_define_source(
        &mut self,
        original: &super::original_definer::OriginalClassConfigurationSource,
        scope_path: &[usize],
    ) -> bool {
        let body = original
            .tokens
            .get(1)
            .zip(
                original
                    .target
                    .class_declaration()
                    .grammar(&self.analysis_context()),
            )
            .and_then(|(token, grammar)| {
                self.original_class_definition_body_at(
                    original.target.consumer().site().offset,
                    *token,
                    grammar,
                )
            });
        let enclosing = std::mem::replace(&mut self.original_class_definition_body, body);
        let result = self.handle_oo_define_with_source(
            original.target.consumer().command(),
            &original.arguments,
            &original.tokens,
            &original.single,
            scope_path,
            Some(original),
        );
        self.original_class_definition_body = enclosing;
        result
    }

    fn handle_oo_define_with_source(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
        source: Option<&super::original_definer::OriginalClassConfigurationSource>,
    ) -> bool {
        if args.is_empty() {
            return false;
        }
        // Distinguish body-form from inline-form by inspecting
        // ``args[1]``.  Body-form: ``oo::define Class { ... }``
        // — args[1] is a single body argument.  Inline-form:
        // ``oo::define Class method foo {} {}`` — args[1] is a
        // known define subcommand.
        if args.len() < 2 {
            return true;
        }

        let source_class = source.map(|source| source.target.class_declaration());
        if let Some(target) = arg_tokens.first() {
            self.retain_original_class_configuration_metadata_with_source(
                target.span,
                source_class,
            );
        }
        let original_class = source_class.cloned().or_else(|| {
            arg_tokens
                .first()
                .and_then(|token| self.original_oo_configuration_class_declaration(*token))
        });
        let selected = original_class.as_ref().and_then(|declaration| {
            declaration
                .source_class(&self.result)
                .map(|record| record.metadata().qualified_name.clone())
                .or_else(|| {
                    declaration
                        .logical_source_class(&self.result)
                        .map(|class| class.qualified_name.clone())
                })
        });
        let Some(qualified) = selected
            .or_else(|| self.oo_define_qualified_target(args, arg_tokens, arg_single, scope_path))
        else {
            // An unjoined target still owns its lexical definition body. This
            // temporary holder is not published under an invented class key.
            let mut holder = super::types::ClassDef {
                via_define: true,
                ..Default::default()
            };
            self.walk_oo_define_form(cmd_name, args, arg_tokens, scope_path, &mut holder);
            return true;
        };

        let (name_span, this_call_span) = Self::oo_definition_source_spans(arg_tokens, source);
        self.result
            .class_body_spans
            .push((qualified.clone(), this_call_span));
        let mut class_def = self.oo_definition_source_class(
            &qualified,
            original_class.as_ref(),
            name_span,
            this_call_span,
        );

        self.walk_oo_define_form(cmd_name, args, arg_tokens, scope_path, &mut class_def);

        // **W315** — as on the creation path, except that a `via_define` stub's
        // aborts are dropped rather than reported: a class created in another
        // file leaves this record with no member tables to judge against.
        self.emit_w315_definition_cannot_run(&mut class_def);
        // `oo::define C { superclass oo::class }` turns an ordinary class into
        // a factory, and `self method create …` changes the shape of the
        // classes it makes, so the derived factory record is recomputed here
        // exactly as it is on the creation path.
        class_def.factory = self.class_factory_of(&class_def);
        if let Some(original) = &original_class {
            self.retain_original_source_class_update(original, &class_def);
        }
        self.register_defined_class(qualified, class_def, scope_path);
        true
    }

    fn oo_definition_source_class(
        &mut self,
        qualified: &str,
        original: Option<&crate::command_binding::OriginalSourceClassDeclaration>,
        name_span: tcl_lexer::Span,
        body_span: tcl_lexer::Span,
    ) -> super::types::ClassDef {
        let canonical = original.and_then(|original| {
            original
                .source_class(&self.result)
                .map(|record| record.metadata().clone())
                .or_else(|| original.logical_source_class(&self.result).cloned())
        });
        canonical
            .or_else(|| {
                self.result
                    .allows_retained_logical_declaration_advice()
                    .then(|| self.result.all_classes.remove(qualified))
                    .flatten()
            })
            .unwrap_or_else(|| super::types::ClassDef {
                name: crate::naming::key_tail(qualified).to_owned(),
                qualified_name: qualified.to_owned(),
                name_span,
                body_span,
                via_define: true,
                ..Default::default()
            })
    }

    /// Report geometry follows the actual whole selected installer. A
    /// captured target's producer does not become the configuration site.
    fn oo_definition_source_spans(
        arguments: &[Token],
        source: Option<&super::original_definer::OriginalClassConfigurationSource>,
    ) -> (tcl_lexer::Span, tcl_lexer::Span) {
        let name = arguments
            .first()
            .map_or(tcl_lexer::Span::empty(0), |word| word.span);
        let original = source.and_then(|source| {
            let words = source.target.consumer().original_words();
            Some(tcl_lexer::Span::new(
                words.first()?.word_span().start(),
                words.last()?.word_span().end(),
            ))
        });
        let call = original.unwrap_or_else(|| {
            arguments.last().map_or(name, |last| {
                tcl_lexer::Span::new(name.start(), last.span.end())
            })
        });
        (name, call)
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
    /// `namespace unknown` handler produces.
    ///
    /// The test is [`Traits::LOADS_EXTERNAL_UNIT`](tcl_registry::Traits), read
    /// off the spec: no command name appears here, so `load` and `auto_load`
    /// are covered by the same code that covers `source`, and a dialect that
    /// adds another file-loading command is covered by declaring the trait.
    fn note_external_unit_command_moved(&mut self, name: &str) {
        if self
            .registry
            .as_deref()
            .and_then(|r| r.get(name))
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
        let Some(spec) = self.registry.as_deref().and_then(|r| r.get(cmd_name)) else {
            return 0;
        };
        let options = self
            .analysis_context()
            .context()
            .available_option_specs(spec);
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
    pub(super) fn register_defined_class(
        &mut self,
        qualified: String,
        class_def: super::types::ClassDef,
        scope_path: &[usize],
    ) {
        let simple = class_def.name.clone();
        self.result
            .retain_class_declaration(qualified, class_def.clone());
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, scope_path) {
            scope.classes.insert(simple, class_def);
        }
    }

    fn original_oo_configuration_class_declaration(
        &self,
        token: Token,
    ) -> Option<crate::command_binding::OriginalSourceClassDeclaration> {
        let configuration = self
            .result
            .original_class_configurations()
            .find(|configuration| {
                configuration
                    .target()
                    .name_input()
                    .original_word_key()
                    .is_some_and(|key| {
                        key.span() == token.span
                            || key
                                .original_word()
                                .tokens()
                                .first()
                                .is_some_and(|original| original.span == token.span)
                    })
            })?;
        let allocation = configuration.target().local_allocation()?;
        let mut originals = self
            .result
            .original_class_declarations()
            .filter(|declaration| declaration.declaration_site() == &allocation.site);
        let original = originals.next()?;
        if originals.next().is_some() {
            return None;
        }
        let declaration =
            crate::registry_invocation::source_structure::source_class_declaration_at(
                &self.source,
                &self.result,
                original.declaration_site().offset,
            )?;
        (declaration.source_class(&self.result)?.declaration_site() == original.declaration_site())
            .then_some(declaration)
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
    ) -> Option<String> {
        if let Some(token) = arg_tokens.first()
            && let Some(configuration) =
                self.result
                    .original_class_configurations()
                    .find(|configuration| {
                        configuration
                            .target()
                            .name_input()
                            .original_word_key()
                            .is_some_and(|key| {
                                key.span() == token.span
                                    || key
                                        .original_word()
                                        .tokens()
                                        .first()
                                        .is_some_and(|written| written.span == token.span)
                            })
                    })
        {
            if let Some(allocation) = configuration.target().local_allocation() {
                let mut declarations = self
                    .result
                    .original_class_declarations()
                    .filter(|declaration| declaration.declaration_site() == &allocation.site);
                let declaration = declarations.next()?;
                if declarations.next().is_some() {
                    return None;
                }
                return Some(declaration.metadata().qualified_name.clone());
            }
            if !configuration.target().is_external_candidate() {
                return None;
            }
        }
        // Native targets require the genuine original configuration issuer.
        // Missing source category/layout never re-enters a reporting-name map.
        if !self.result.allows_retained_logical_declaration_advice() {
            return None;
        }
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
        if dynamic_key.is_none()
            && let Some(selected) = self.resolve_user_class_in(raw, scope_path)
        {
            return Some(selected);
        }
        let namespace = self.command_resolution_namespace(scope_path);
        let presentation = qualify(&namespace, dynamic_key.as_deref().unwrap_or(raw));
        if self.result.all_classes.contains_key(&presentation) {
            return None;
        }
        Some(presentation)
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
                    Some(self.analysis_context().context().authoring_query()),
                    scope_path,
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
    /// option carries `SpecSurface::TCL85_PLUS` in the registry, so the
    /// dialect gating comes along for free: under an 8.4 profile — which has
    /// no `-encoding` — the word is not recognised as an option and the path
    /// word is read at index 0, matching that spec's own declaration rather
    /// than a hardcoded assumption here.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::Source`].
    pub fn handle_source_command(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) {
        let current_interpreter_scope = (0..=scope_path.len()).all(|depth| {
            super::scope::scope_at(&self.result.global_scope, &scope_path[..depth]).is_some_and(
                |scope| {
                    scope.original_interpreter_source_domain.is_none()
                        && !scope.is_independent_interpreter_source_body()
                },
            )
        });
        let original_project_source_load = (load_trace_flag(
            "eligible.scope", None, current_interpreter_scope,
        ) && load_trace_flag("eligible.interpreter-stack", None, self.interp_path_stack.is_empty())
            && load_trace_flag("eligible.safe-stack", None, self.safe_interp_stack.is_empty()))
        .then(|| {
            load_trace_option("capture", None,
                crate::registry_invocation::source_structure::capture_original_project_source_load_at(
                    &self.source,
                    &self.result,
                    cmd_tok.span.start(),
                ),
            )
        })
        .flatten()
        .map(std::sync::Arc::new);
        let file_idx = original_project_source_load.as_ref().map_or_else(
            || self.leading_option_words("source", args),
            |load| load.path_argument(),
        );
        if file_idx >= args.len() || file_idx >= arg_tokens.len() {
            self.result
                .source_targets
                .push(crate::signature_scan::types::SignatureSource {
                    original_project_source_load: None,
                    original_interpreter_source_load: None,
                    raw_path: String::new(),
                    range: cmd_tok.span,
                    is_literal: false,
                    site_namespace: self.command_resolution_namespace(scope_path),
                });
            return;
        }
        let st = arg_tokens[file_idx];
        let original_project_source_load = original_project_source_load.filter(|load| {
            load_trace_flag(
                "path-span",
                None,
                load.path_word().is_some_and(|path| {
                    path.span() == st.span
                        || path
                            .tokens()
                            .first()
                            .is_some_and(|written| written.span == st.span)
                }),
            )
        });
        let path = &args[file_idx];
        let is_single = arg_single.get(file_idx).copied().unwrap_or(false);
        // A dynamic path — a bare `$var` or a concatenation like
        // `${dir}lit.tcl` — is first tried against the same last-write-wins
        // constant-string lattice already proven for `rename`'s OLD/NEW
        // words: the real corpus idiom is `set p
        // "e.tcl"; source $p`, a few lines apart in the same file, which
        // should resolve exactly like the literal `source e.tcl` it's
        // equivalent to — instead of going untracked and leaking the
        // sourced file's definitions into every caller as if unconditionally
        // global. A `[...]` command substitution
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
        // `source` evaluates the file in the caller's current namespace:
        // record the command-resolution namespace at this call site so the
        // workspace index can re-home the sourced document's definitions.
        let site_namespace = self.command_resolution_namespace(scope_path);
        if let Some(source) = self
            .result
            .source_targets
            .iter_mut()
            .find(|source| source.range == st.span)
        {
            source.original_project_source_load = original_project_source_load;
            source.raw_path = path;
            source.is_literal = is_lit;
            source.site_namespace = site_namespace;
            return;
        }
        self.result
            .source_targets
            .push(crate::signature_scan::types::SignatureSource {
                original_project_source_load,
                raw_path: path,
                range: st.span,
                is_literal: is_lit,
                site_namespace,
                original_interpreter_source_load: None,
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
        let Some(first) = arg_tokens
            .get(1)
            .and_then(|token| self.original_source_namespace_operand(token.span, scope_path))
        else {
            return;
        };
        let first_bytes = [first.name_input().bytes()];
        let Some((forced, _)) = tcl_registry::NamespaceTransition::import_pattern_byte_operands(
            &first_bytes,
            first.name_input().policy(),
        ) else {
            return;
        };
        let mut idx = 1 + usize::from(forced);
        // `namespace import` imports into the namespace current at the call —
        // the command-resolution namespace, so an import inside
        // `proc ::ns::p {}` lands in `::ns`.
        let importing_ns = self.command_resolution_namespace(scope_path);
        while idx < args.len() && idx < arg_tokens.len() {
            let pat_raw = args[idx].clone();
            let original = self
                .original_source_namespace_operand(arg_tokens[idx].span, scope_path)
                .and_then(|operand| {
                    crate::signature_scan::original_name::SourceNamespacePattern::from_original(
                        operand,
                        tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
                        arg_tokens[idx].span,
                        forced,
                    )
                });
            let Some(original) = original else {
                if pat_raw.contains(['$', '[']) {
                    self.result.has_dynamic_providers = true;
                }
                idx += 1;
                continue;
            };
            let source =
                crate::signature_scan::types::SignatureNamespaceImportSource::from_native_pattern(
                    original.parts(),
                );
            self.result.original_namespace_patterns.push(original);
            let Some(source) = source else {
                idx += 1;
                continue;
            };
            let pat = source.constructed_pattern();
            self.result.namespace_imports.push(
                crate::signature_scan::types::SignatureNamespaceImport {
                    ns: importing_ns.clone(),
                    pattern: pat,
                    source: Some(source),
                    range: arg_tokens[idx].span,
                    conjectured: false,
                    forced,
                },
            );
            idx += 1;
        }
    }

    /// Record `namespace forget ?PATTERN ...?` events — the removal half of
    /// the import edge's lifecycle log.
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
            let original = self
                .original_source_namespace_operand(arg_tokens[idx].span, scope_path)
                .and_then(|operand| {
                    crate::signature_scan::original_name::SourceNamespacePattern::from_original(
                        operand,
                        tcl_syntax::naming::NativeNamePurpose::NamespaceForgetPattern,
                        arg_tokens[idx].span,
                        false,
                    )
                });
            let Some(original) = original else {
                idx += 1;
                continue;
            };
            let parts = original.parts().clone();
            self.result.original_namespace_patterns.push(original);
            let source_ns =
                crate::signature_scan::types::SignatureNamespaceImportSource::from_native_pattern(
                    &parts,
                )
                .map(|source| source.namespace);
            let Ok(pattern) = parts.tail.try_utf8() else {
                idx += 1;
                continue;
            };
            self.result.namespace_forgets.push(
                crate::signature_scan::types::SignatureNamespaceForget {
                    ns: forgetting_ns.clone(),
                    source_ns,
                    pattern: pattern.to_owned(),
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
    /// source namespace actually exports it.
    ///
    /// `-clear` is recorded as an ordered **tombstone** entry rather than
    /// applied by dropping the namespace's earlier entries: an import
    /// snapshots the export list as it stood when the import ran, so a
    /// `-clear` written *after* an import must not revoke what that import
    /// already bound (oracle in
    /// [`crate::signature_scan::types::SignatureNamespaceExport`]).
    /// Collapsing it eagerly destroys exactly the ordering the snapshot needs. Consumers reconstruct the
    /// export set as of an offset with
    /// `tcl_lsp_core::namespace_import::exported_at_import_site`.
    ///
    /// A dynamic pattern (`$`/`[` substitution) can't be statically resolved
    /// to a glob text, so it is silently skipped — the wildcard-import
    /// resolver then correctly abstains for names it might have covered,
    /// rather than guessing. A dynamic word cannot hide a `-clear`, which is
    /// a literal flag word or nothing. It is skipped rather than treated as
    /// the abort below, for the same reason: whether it resolves to a
    /// qualified pattern is unknowable here.
    ///
    /// A **qualified** pattern is a guaranteed runtime error (`Tcl_Export`
    /// rejects it), and C `return`s from the pattern loop on that failure, so
    /// recording stops there — see the loop for the oracle.
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
        self.record_original_namespace_exports(args, arg_tokens, scope_path);
        let mut idx = 1;
        // As for `import`: the exporting namespace is the one current at the
        // call, not the lexically enclosing one.
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
            let selected = self
                .declaration_name_dialect()
                .authored_name_policy()
                .and_then(|policy| {
                    policy
                        .recipe()
                        .namespace_pattern_input(
                            pattern.as_bytes(),
                            tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
                        )
                        .ok()
                        .and_then(|projection| {
                            std::str::from_utf8(projection.selected())
                                .ok()
                                .map(str::to_owned)
                        })
                });
            let Some(pattern) = selected else {
                idx += 1;
                continue;
            };
            // `NamespaceExportCmd` calls `Tcl_Export` per pattern and
            // `return`s on the first failure, so a qualified pattern aborts
            // the loop: every *later* word is never exported at all. The
            // earlier ones stay — C commits each pattern as it goes.
            // Oracle (tclsh 8.6.16 / 9.0.4): `namespace export one ::bad
            // three` leaves exactly `one` exported, and `namespace export
            // ::bad ok` leaves nothing. Recording past the bad word let a
            // wildcard-import bareword resolve where real Tcl would not.
            if tcl_syntax::naming::is_qualified(pattern.as_bytes()) {
                break;
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

    fn original_source_namespace_operand(
        &self,
        span: tcl_lexer::Span,
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNamespaceOperand> {
        self.head_identities
            .original_namespace_operand_at_span_in_source(
                &tcl_lexer::SourceImage::document(&self.source),
                span,
                self.file_lexer_config(),
            )
            .or_else(|| {
                let original = self.original_static_source_name_at_span(span)?;
                (original.name_input().policy() == self.declaration_name_policy()?)
                    .then_some(())?;
                crate::signature_scan::original_name::SourceOriginalNamespaceOperand::from_conditional_declaration(
                    original,
                    self.conditional_source_namespace_scope(scope_path)?,
                )
            })
    }

    fn conditional_source_namespace_scope(
        &self,
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        let policy = self.declaration_name_policy()?;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let mut selected = self.declaration_namespace_scope(&[])?;
        let mut scope = &self.result.global_scope;
        for &index in scope_path {
            scope = scope.children.get(index)?;
            if scope.kind != super::types::ScopeKind::Namespace {
                return None;
            }
            let name = self.original_static_source_name_at_span(scope.name_span?)?;
            let key = name.name_input();
            if key.policy() != policy
                || key.lexer_config() != self.file_lexer_config()
                || key.source_image() != &image
            {
                return None;
            }
            selected =
                selected.child_from_input(&SignatureSourceNameInput::OriginalWord(key.clone()))?;
            if scope.naming_scope.as_ref() != Some(&selected) {
                return None;
            }
        }
        Some(selected)
    }

    fn record_original_namespace_exports(
        &mut self,
        args: &[String],
        tokens: &[Token],
        scope_path: &[usize],
    ) {
        use crate::signature_scan::original_name::SourceNamespaceExport;
        let mut operands = Vec::new();
        for token in tokens.iter().take(args.len()).skip(1) {
            let Some(operand) = self.original_source_namespace_operand(token.span, scope_path)
            else {
                self.result
                    .original_namespace_export_unknowns
                    .push(token.span);
                return;
            };
            operands.push((operand, token.span));
        }
        let Some((first, _)) = operands.first() else {
            return;
        };
        let policy = first.name_input().policy();
        if operands.iter().any(|(operand, _)| {
            operand.name_input().policy() != policy
                || operand.context() != first.context()
                || operand.site() != first.site()
        }) {
            return;
        }
        let bytes = operands
            .iter()
            .map(|(operand, _)| operand.name_input().bytes())
            .collect::<Vec<_>>();
        let Some((clear, _)) =
            tcl_registry::NamespaceTransition::export_pattern_byte_operands(&bytes, policy)
        else {
            return;
        };
        for (index, (operand, span)) in operands.into_iter().enumerate() {
            let Some(event) =
                SourceNamespaceExport::from_original(operand, span, clear && index == 0)
            else {
                // A qualified pattern aborts this handler after committing its
                // preceding pattern events; it supplies no enclosing Normal.
                break;
            };
            self.result.original_namespace_exports.push(event);
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
        let Some((spec, sub_spec)) = self
            .registry
            .as_deref()
            .and_then(|r| r.get("namespace"))
            .and_then(|spec| spec.subcommand(sub).map(|s| (spec, s)))
        else {
            return 0;
        };
        let options = self
            .analysis_context()
            .context()
            .available_sub_option_specs(spec, sub_spec);
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
    /// The wrapper bodies are not statically detected, but
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
        let [alias] = args else {
            return;
        };
        let current_ns = self.command_resolution_namespace(scope_path);
        let Some(hint) = crate::naming::authored_import_wrapper_hint(
            cmd_name,
            alias,
            &current_ns,
            crate::naming::AuthoredImportAliasSpelling::Written,
        ) else {
            return;
        };
        self.result.namespace_imports.push(
            crate::signature_scan::types::SignatureNamespaceImport {
                ns: hint.alias_namespace,
                pattern: hint.pattern,
                source: Some(
                    crate::signature_scan::types::SignatureNamespaceImportSource {
                        native_source: None,
                        namespace: hint.source_namespace,
                        tail_pattern: "*".to_owned(),
                    },
                ),
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

    /// Record a write to the package search path: `lappend auto_path
    /// PATH...` ([`AutoPathForm::Append`], one entry per word) or `set
    /// auto_path PATH` ([`AutoPathForm::Assign`], one entry for the list).
    ///
    /// The search path is the special variable the registry names
    /// `auto_path`, where the document's dialect provides it for user code to
    /// write ([`tcl_registry::VarAccess::ReadWrite`]) — one read of the
    /// special-variable registry, so a dialect with no auto-loader (iRules)
    /// records nothing. The check is on the written *variable*, not a
    /// command guard. Any such mutation flips ``has_dynamic_providers``:
    /// packages discovered at runtime can register commands the static
    /// analyser can't see, so W123 unknown-command diagnostics suppress on
    /// the document.
    ///
    /// `lappend` appends **one list element per argument word**, so an
    /// append records one entry per word and each is a whole directory —
    /// `lappend auto_path {p q}` names the single directory `p q`, not two.
    /// An assignment's right-hand side is a **list**, so its one record may
    /// name several directories; the split is deliberately *not* done here:
    /// the record keeps the source word verbatim (its `range` is the whole
    /// argument), and [`crate::auto_path_eval::evaluate_auto_path_entry`]
    /// applies the list grammar at consumption.
    ///
    /// `written` is the position of the written variable and `values` the
    /// words recorded. The assignment is recorded by
    /// [`Self::bind_value_word_assignment`] for any command whose declared
    /// semantics stores its value word into the variable it names; an
    /// append by the binder ([`Self::handle_var_binding_command`]) for any
    /// command whose descriptor appends list elements to the variable it
    /// writes ([`tcl_registry::VarElementsEffect::AppendsListElements`]).
    pub fn record_search_path_write(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        written: usize,
        values: std::ops::Range<usize>,
        form: super::types::AutoPathForm,
    ) {
        let Some(written) = args.get(written) else {
            return;
        };
        let context = self.analysis_context();
        let dialect = Some(context.context().authoring_query());
        if !context
            .commands()
            .special_var_in_dialect("auto_path", dialect)
            .is_some_and(|search_path| {
                search_path.access == tcl_registry::VarAccess::ReadWrite
                    && search_path.name == written
            })
        {
            return;
        }
        // An assignment with no value reads the path; it writes nothing. An
        // append of nothing still names the path as written.
        if values.is_empty() && form == super::types::AutoPathForm::Assign {
            return;
        }
        for i in values {
            let Some(tok) = arg_tokens.get(i) else {
                continue;
            };
            self.result
                .auto_path_entries
                .push(super::types::AutoPathEntry {
                    raw_path: args[i].clone(),
                    range: tok.span,
                    form,
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
        // registry-axis-ok: special_vars — the auto_path special variable; until the special-variable table marks the package search path
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

    /// Registry-owned pattern layout over original typed argument words.
    pub(in crate::analyser) fn regex_pattern_source_index(
        &self,
        command: &str,
        arg_tokens: &[Token],
        command_token: Token,
    ) -> Option<usize> {
        let context = self.analysis_context();
        let mut dialect = self
            .source_analysis_entry
            .as_ref()
            .and_then(|entry| entry.invocation_dialect)
            .unwrap_or_else(|| tcl_registry::InvocationDialect::of_profile(self.profile));
        dialect.lexer_grammar = self.grammar();
        dialect.word_values = self.word_rules();
        let source_map = self.cached_source_map();
        let end = tcl_lexer::word_span(&source_map, *arg_tokens.last()?).end();
        let start = command_token.span.start();
        let text = self.source.get(start as usize..end as usize)?;
        let segmented = crate::segmenter::segment_commands_with_offset_and_config(
            text,
            start,
            self.file_lexer_config().nested(),
        )
        .into_iter()
        .next()?;
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &source_map,
            self.file_lexer_config(),
            &segmented,
        );
        self.head_identities
            .source_bindings_ref()
            .stamp_original_tokens(&mut tokens);
        if self.result.allows_lexical_declaration_advice() {
            let words = crate::registry_invocation::source_structure::source_registry_words(
                &self.source,
                &self.result,
                &segmented,
            )?;
            let patterns = words.with_source_schema(&context, |schema| {
                schema.authored_source_pattern_arguments()
            })??;
            let mut selected = patterns
                .into_iter()
                .filter(|pattern| pattern.kind == tcl_registry::patterns::PatternType::Regex);
            let pattern = selected.next()?;
            if selected.next().is_some() {
                return None;
            }
            let crate::registry_invocation::InvocationWordOrigin::Written(written) = words
                .origins()
                .get(usize::from(pattern.index).checked_add(1)?)?
            else {
                return None;
            };
            return written.checked_sub(1);
        }
        if tokens.source_binding.is_some() {
            return crate::registry_invocation::normal_representation_invocation_in_context(
                &context, &tokens,
            )?
            .pattern_source_argument_index(context.commands());
        }
        crate::regex_source::source_pattern_index(
            &self.source,
            context.commands(),
            command,
            arg_tokens,
            self.file_lexer_config(),
            dialect,
        )
    }

    /// Record the pattern arguments of regex-pattern commands
    /// (`PatternType::Regex` specs — `regexp` / `regsub`) for syntax
    /// highlighting.
    ///
    /// Literal patterns (`Esc` / `Str` tokens) are recorded
    /// verbatim; variable patterns use checked Logical declaration and value
    /// provenance to retain the original value and its source geometry.
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
        command_token: Token,
    ) {
        if args.is_empty() {
            return;
        }
        let Some(idx) = self.regex_pattern_source_index(cmd_name, arg_tokens, command_token) else {
            return;
        };
        if idx >= arg_tokens.len() {
            return;
        }
        let tok = arg_tokens[idx];
        self.record_regex_pattern_token(&args[idx], tok, cmd_name, scope_path);
    }

    /// Record one regex-pattern token for `regexp` / `regsub` /
    /// `switch -regexp`. Variables use checked original value provenance;
    /// command substitutions supply no static pattern, and literals retain their bytes.
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
                if let Some((const_val, def_span)) = self.lookup_logical_const_substitution(tok) {
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
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_native_namespace_path_tests;

#[cfg(test)]
mod original_static_declaration_tests;

#[cfg(test)]
mod original_optlist_declaration_tests;

#[cfg(test)]
mod original_registry_symbol_tests;

#[cfg(test)]
mod original_parent_purpose_tests;

#[cfg(test)]
mod retained_static_context_tests;

#[cfg(test)]
mod constructed_projection_tests;

#[cfg(test)]
mod original_proc_source_layout_tests;

#[cfg(test)]
mod direct_original_class_body_tests;
