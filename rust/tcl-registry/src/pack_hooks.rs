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

//! Pack-declared hooks: the registry side of the `SpecTcl` hook host.
//!
//! A shipped spec's `const_fold` / `arg_role_resolver` / … is a Rust function
//! pointer.  A **pack**-declared one is a Tcl body running on a sandboxed VM,
//! which the registry must not know about — the hook host owns that
//! (`docs/design/registry/spec-packs.md`, "Two layers, deliberately").  This module is
//! the seam between them, and it holds exactly three things:
//!
//! - **Slots and thunks.** A pack hook is allocated a [`HookSlot`] and the
//!   spec is built with the matching thunk from [`const_fold_fn`] and friends.
//!   The thunk is an ordinary function pointer of the family's shipped type,
//!   so every consumer — the optimiser's const-subst engine, the analyser's
//!   role walk, `hover.rs`'s option scanner — keeps calling
//!   [`CommandSpec::run_const_fold`](crate::spec::CommandSpec::run_const_fold)
//!   and the rest with no idea a pack is involved.
//! - **The host seam.** [`PackHookHost`] is what the thunk calls. It is
//!   installed per **thread** ([`install_host`]), because a hook host owns
//!   VM instances that are not `Send`; a thread with no host installed
//!   abstains, which is the same answer a crashed or quarantined hook gives.
//! - **The shape-keyed cache.** The measured budget in `spec-packs.md` only
//!   closes if a resolver whose declared inputs are shape-only is answered
//!   from a cache keyed by command + word shape rather than re-entering the
//!   VM.  [`HookInputs`] is that declaration and [`HookInputs::shape_only`]
//!   is the cacheability rule; the cache itself is thread-local, like the
//!   host. A content-keyed entry keeps the call's content and a hit compares
//!   it: the hash is the bucket, never the proof.
//! - **The evaluator generation.** [`evaluator_generation`] names this
//!   thread's host and its health, changing wherever the cache is cleared
//!   for a host change, so an analysis memo keyed by it never serves one
//!   worker's answers to a worker whose evaluators differ.
//!
//! Nothing here evaluates anything. If no host is installed — the default in
//! every process that has not loaded a pack — every thunk answers its family's
//! documented silence, and the cost is one relaxed atomic load.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};

use tcl_dialect::TclVersion;

use crate::arg_role::{AppendedArity, ArgRole};
use crate::clause_shape::{ClauseShapeChecker, ClauseShapeError};
use crate::hooks::{ConstFoldFn, VersionedConstFoldFn};
use crate::hover::{OptionValueHook, OptionValueOutcome, ScriptTiming};
use crate::invocation_words::{
    CommandPrefixArguments, InvocationArguments, InvocationWord, InvocationWordKind,
};
use crate::literal_validation::{LiteralArgumentValidation, LiteralArgumentValidator};
use crate::spec::{
    ArgRoleResolver, CommandPrefixResolver, ConstraintsHook, ContextGate, ScriptTimingResolver,
};
use crate::value_transfer::{
    ContextDependency, DeclaredStructure, EvalRoute, EvaluatorGeneration, ImplementationBudget,
};

/// How many hooks of one family a process may install.
///
/// The thunk tables are static arrays of distinct monomorphised functions, so
/// this is a compile-time bound, not a budget: the shipped registry sets 45
/// const-folders and 44 role resolvers across ~2,210 commands, so 64 per
/// family is generous for the pack that motivates it. Allocation past the end
/// returns `None` and the loader installs no hook — a pack that declares 65
/// folders loses the 65th's behaviour, never its facts.
pub const SLOTS_PER_FAMILY: usize = 64;

/// The ten hook families, which is what fixes a hook's calling convention:
/// its emitter verbs, what silence means, and whether it may run on a call
/// carrying a non-literal word.
///
/// Spelled here rather than in the loader because the vocabulary is the
/// registry's — the same reason trait, role, and hook-id names live here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HookFamily {
    /// `arg_role_resolver` — emits `role IDX ROLE`.
    ArgRoleResolver,
    /// `command_prefix_resolver` — emits `prefix IDX {Exactly N}`.
    CommandPrefixResolver,
    /// `script_timing_resolver` — emits
    /// `timing IDX SameInvocation|Deferred|ReferenceOnly`.
    ScriptTimingResolver,
    /// `const_fold` — emits `fold VALUE`.
    ConstFold,
    /// `const_fold_versioned` — `const_fold` with `tcl-version` in `ctx`.
    ConstFoldVersioned,
    /// `taint_sink_gate` — emits `sink-applies` / `sink-suppressed`.
    TaintSinkGate,
    /// `context_gate` — emits `reject MESSAGE`.
    ContextGate,
    /// `literal_argument_validator` — emits `invalid …` / `abstain REASON`.
    LiteralArgumentValidator,
    /// `clause_shape_check` — emits `missing-expr` / `missing-body` /
    /// `extra-words`.
    ClauseShapeCheck,
    /// The option-arity hook, written `-arity-hook` inside an option row —
    /// emits `consume N ?-invalid MESSAGE?`.
    OptionArity,
    /// `constraints` — E-R14's escape hatch. Reads the whole invocation
    /// through `option-present` / `option-value` / `arg-count` / `literal`
    /// and emits `invalid SLOT MESSAGE ?-conflict?` / `abstain REASON`.
    Constraints,
    /// `evaluate -implementation` — a declared implementation's body. Its
    /// parameters are the declared inputs, in order, and it emits `fold
    /// VALUE`, `write TARGET VALUE` and `preserve TARGET`
    /// (`docs/design/compiler/value-evaluation.md` § *The body verbs*).
    Evaluate,
}

/// Every family, in declaration order — the index a slot's family contributes
/// to the per-family tables.
pub const HOOK_FAMILIES: [HookFamily; 12] = [
    HookFamily::ArgRoleResolver,
    HookFamily::CommandPrefixResolver,
    HookFamily::ScriptTimingResolver,
    HookFamily::ConstFold,
    HookFamily::ConstFoldVersioned,
    HookFamily::TaintSinkGate,
    HookFamily::ContextGate,
    HookFamily::LiteralArgumentValidator,
    HookFamily::ClauseShapeCheck,
    HookFamily::OptionArity,
    HookFamily::Constraints,
    HookFamily::Evaluate,
];

impl HookFamily {
    /// The emitter verbs this family injects into the sandbox.
    #[must_use]
    pub fn verbs(self) -> &'static [&'static str] {
        match self {
            Self::ArgRoleResolver => &["role"],
            Self::CommandPrefixResolver => &["prefix"],
            Self::ScriptTimingResolver => &["timing"],
            Self::ConstFold | Self::ConstFoldVersioned => &["fold"],
            Self::TaintSinkGate => &["sink-applies", "sink-suppressed"],
            Self::ContextGate => &["reject"],
            Self::LiteralArgumentValidator => &["invalid", "abstain"],
            Self::ClauseShapeCheck => &["missing-expr", "missing-body", "extra-words"],
            Self::OptionArity => &["consume"],
            // Four readers and two emitters: the reading half is how a body
            // reaches the invocation it is judging, and every spelling is one
            // another family already uses (`invalid`, `abstain`, `literal`).
            Self::Constraints => &[
                "invalid",
                "abstain",
                "option-present",
                "option-value",
                "literal",
                "arg-count",
            ],
            Self::Evaluate => &["fold", "write", "preserve"],
        }
    }

    /// What calling no verb at all means — the per-field conservative answer.
    #[must_use]
    pub fn silence(self) -> &'static str {
        match self {
            Self::ArgRoleResolver => "no roles (fall back to arg_roles)",
            Self::CommandPrefixResolver => "no prefix positions",
            Self::ScriptTimingResolver => "no timing override",
            Self::ConstFold | Self::ConstFoldVersioned => "no fold",
            // The one family whose silence is not "no opinion": a security
            // finding must survive a hook that says nothing.
            Self::TaintSinkGate => "the sink applies",
            Self::ContextGate => "the call is allowed",
            Self::LiteralArgumentValidator => "valid",
            Self::ClauseShapeCheck => "the shape is accepted",
            Self::OptionArity => "consume one word",
            // The declarative relations already answered; a silent hook adds
            // nothing to their verdict.
            Self::Constraints => "no report",
            // Silence establishes nothing: the evaluation declines.
            Self::Evaluate => "a decline",
        }
    }

    /// Whether the family may only run when **every** word's `kinds` entry is
    /// `literal`.
    ///
    /// Normative (`docs/design/spec-dsl-examples/README.md`, "When a hook
    /// runs"): the fold families and the option-arity hook are literal-only.
    /// The DSL's own convention is that a non-literal word arrives as the
    /// empty string, so a fold body that never consults `kinds` would
    /// otherwise fold `string length $x` to `0`. The precondition is the
    /// loader's, stated once, rather than a guard every author must remember.
    #[must_use]
    pub fn requires_all_literal(self) -> bool {
        matches!(
            self,
            Self::ConstFold | Self::ConstFoldVersioned | Self::OptionArity | Self::Evaluate
        )
    }

    /// The DSL property word this family fills.
    #[must_use]
    pub fn field(self) -> &'static str {
        match self {
            Self::ArgRoleResolver => "arg_role_resolver",
            Self::CommandPrefixResolver => "command_prefix_resolver",
            Self::ScriptTimingResolver => "script_timing_resolver",
            Self::ConstFold => "const_fold",
            Self::ConstFoldVersioned => "const_fold_versioned",
            Self::TaintSinkGate => "taint_sink_gate",
            Self::ContextGate => "context_gate",
            Self::LiteralArgumentValidator => "literal_argument_validator",
            Self::ClauseShapeCheck => "clause_shape_check",
            Self::OptionArity => "options.arity_hook",
            Self::Constraints => "constraints",
            Self::Evaluate => "evaluate",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::ArgRoleResolver => 0,
            Self::CommandPrefixResolver => 1,
            Self::ScriptTimingResolver => 2,
            Self::ConstFold => 3,
            Self::ConstFoldVersioned => 4,
            Self::TaintSinkGate => 5,
            Self::ContextGate => 6,
            Self::LiteralArgumentValidator => 7,
            Self::ClauseShapeCheck => 8,
            Self::OptionArity => 9,
            Self::Constraints => 10,
            Self::Evaluate => 11,
        }
    }
}

/// A native implementation, named `SCOPE::FIELD`
/// (`docs/design/compiler/value-evaluation.md` § *`-native ID`, and the
/// per-family catalogues*). `SCOPE` is the command name, `command::subcommand`
/// for a subcommand-scoped field, or `command::subcommand::-option` for an
/// option-scoped one; `FIELD` is the field's own DSL keyword
/// ([`HookFamily::field`], or `semantics` / `evaluate` / `facts`, the two
/// statements no family owns and the one that is also `HookFamily::Evaluate`
/// itself).
///
/// Every family and those two extra fields gets one of these tables below,
/// each keyed by the full id, holding the shipped Rust value the id names.
/// An empty table means nothing this build ships is reachable by id yet —
/// not that the field cannot be declared natively — and every `-native ID`
/// still keeps loading; it only ever fails to resolve.
/// `rust/tcl-spectcl/src/loader.rs`'s `native_hook_tables_cover_their_catalogues`
/// holds each table's id set level with `tcl_spectcl::catalogue`'s picker
/// list of the same name, so a table that gains a shipped entry and a
/// catalogue that does not name it fail the same assertion, from either
/// side.
pub const ARG_ROLE_RESOLVER_NATIVE: &[(&str, ArgRoleResolver)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const COMMAND_PREFIX_RESOLVER_NATIVE: &[(&str, CommandPrefixResolver)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const SCRIPT_TIMING_RESOLVER_NATIVE: &[(&str, ScriptTimingResolver)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. The shipped folders' unversioned
/// constant folders, keyed by the command or subcommand each folds
/// (`docs/design/compiler/value-evaluation.md`'s worked example).
pub const CONST_FOLD_NATIVE: &[(&str, ConstFoldFn)] = &[
    // `fold_range` itself is `VersionedConstFoldFn`-shaped (it reads the
    // release); the unversioned slot `string range` actually ships is the
    // unanimous-answer wrapper around it.
    (
        "string::range::const_fold",
        crate::commands::tcl::fold_range_unanimous,
    ),
    (
        "string::replace::const_fold",
        crate::commands::tcl::fold_replace,
    ),
    ("regsub::const_fold", crate::commands::tcl::fold_regsub),
    ("scan::const_fold", crate::commands::tcl::fold_scan),
    ("list::const_fold", crate::const_fold::fold_list),
    ("lindex::const_fold", crate::const_fold::fold_lindex),
    ("concat::const_fold", crate::const_fold::fold_concat),
    ("llength::const_fold", crate::const_fold::fold_llength),
    ("lreverse::const_fold", crate::const_fold::fold_lreverse),
    ("join::const_fold", crate::const_fold::fold_join),
    ("split::const_fold", crate::const_fold::fold_split),
    ("lrepeat::const_fold", crate::const_fold::fold_lrepeat),
    ("lrange::const_fold", crate::const_fold::fold_lrange),
    ("dict::get::const_fold", crate::const_fold::fold_dict_get),
    (
        "dict::exists::const_fold",
        crate::const_fold::fold_dict_exists,
    ),
    ("dict::size::const_fold", crate::const_fold::fold_dict_size),
    ("dict::keys::const_fold", crate::const_fold::fold_dict_keys),
    (
        "dict::values::const_fold",
        crate::const_fold::fold_dict_values,
    ),
    (
        "dict::create::const_fold",
        crate::const_fold::fold_dict_create,
    ),
    (
        "dict::merge::const_fold",
        crate::const_fold::fold_dict_merge,
    ),
    // The `::tcl::dict::` spellings are commands of their own
    // (`qualified_specs()`), each carrying its subcommand's folder.
    (
        "::tcl::dict::get::const_fold",
        crate::const_fold::fold_dict_get,
    ),
    (
        "::tcl::dict::exists::const_fold",
        crate::const_fold::fold_dict_exists,
    ),
    (
        "::tcl::dict::size::const_fold",
        crate::const_fold::fold_dict_size,
    ),
    (
        "::tcl::dict::keys::const_fold",
        crate::const_fold::fold_dict_keys,
    ),
    (
        "::tcl::dict::values::const_fold",
        crate::const_fold::fold_dict_values,
    ),
    (
        "::tcl::dict::create::const_fold",
        crate::const_fold::fold_dict_create,
    ),
    (
        "::tcl::dict::merge::const_fold",
        crate::const_fold::fold_dict_merge,
    ),
    ("string::cat::const_fold", crate::commands::tcl::fold_cat),
    (
        "string::compare::const_fold",
        crate::commands::tcl::fold_compare,
    ),
    (
        "string::equal::const_fold",
        crate::commands::tcl::fold_equal,
    ),
    (
        "string::first::const_fold",
        crate::commands::tcl::fold_first,
    ),
    (
        "string::index::const_fold",
        crate::commands::tcl::fold_index,
    ),
    ("string::last::const_fold", crate::commands::tcl::fold_last),
    (
        "string::length::const_fold",
        crate::commands::tcl::fold_length,
    ),
    (
        "string::map::const_fold",
        crate::commands::tcl::fold_string_map,
    ),
    (
        "string::match::const_fold",
        crate::commands::tcl::fold_match,
    ),
    (
        "string::repeat::const_fold",
        crate::commands::tcl::fold_repeat,
    ),
    (
        "string::reverse::const_fold",
        crate::commands::tcl::fold_reverse,
    ),
    (
        "string::tolower::const_fold",
        crate::commands::tcl::fold_tolower,
    ),
    (
        "string::totitle::const_fold",
        crate::commands::tcl::fold_totitle,
    ),
    (
        "string::toupper::const_fold",
        crate::commands::tcl::fold_toupper,
    ),
    ("string::trim::const_fold", crate::commands::tcl::fold_trim),
    (
        "string::trimleft::const_fold",
        crate::commands::tcl::fold_trimleft,
    ),
    (
        "string::trimright::const_fold",
        crate::commands::tcl::fold_trimright,
    ),
    (
        "namespace::qualifiers::const_fold",
        crate::commands::tcl::fold_qualifiers,
    ),
    (
        "namespace::tail::const_fold",
        crate::commands::tcl::fold_tail,
    ),
    ("subst::const_fold", crate::commands::tcl::fold_subst),
];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. The shipped folders' release-aware
/// constant folders.
pub const CONST_FOLD_VERSIONED_NATIVE: &[(&str, VersionedConstFoldFn)] = &[
    (
        "string::is::const_fold_versioned",
        crate::commands::tcl::fold_is,
    ),
    (
        "string::range::const_fold_versioned",
        crate::commands::tcl::fold_range,
    ),
    (
        "format::const_fold_versioned",
        crate::commands::tcl::fold_format,
    ),
];

/// [`crate::spec::CommandSpec::taint_sink_gate`]'s function-pointer shape,
/// named so [`TAINT_SINK_GATE_NATIVE`]'s element type stays under clippy's
/// `type_complexity` threshold.
pub type TaintSinkGateFn = fn(&[&str]) -> bool;

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const TAINT_SINK_GATE_NATIVE: &[(&str, TaintSinkGateFn)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const CONTEXT_GATE_NATIVE: &[(&str, ContextGate)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const LITERAL_ARGUMENT_VALIDATOR_NATIVE: &[(&str, LiteralArgumentValidator)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const CLAUSE_SHAPE_CHECK_NATIVE: &[(&str, ClauseShapeChecker)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const OPTION_ARITY_NATIVE: &[(&str, OptionValueHook)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`].
pub const CONSTRAINTS_NATIVE: &[(&str, ConstraintsHook)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. `semantics -native ID`: the shipped
/// structural plan, by name.
pub const SEMANTICS_NATIVE: &[(&str, DeclaredStructure)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. `evaluate -native ID`: a shipped
/// evaluator whose route the catalogue entry itself names. `-direct` and
/// `-expression` are different, already-closed catalogues of their own
/// (`NativeEvalId::ALL`, `LanguageProfileId::ALL`), not `SCOPE::FIELD` ids.
pub const EVALUATE_NATIVE: &[(&str, EvalRoute)] = &[];

/// See [`ARG_ROLE_RESOLVER_NATIVE`]. `facts -native ID`: checked, not
/// stored — nothing reads a pack's facts yet — so the table records only
/// which ids are shipped.
pub const FACTS_NATIVE: &[(&str, ())] = &[];

/// One input a hook body declares it reads.
///
/// The DSL spelling is `-inputs {nwords kinds}` on the hook statement. An
/// undeclared hook depends on everything ([`HookInputs::unrestricted`]), which
/// is always sound and never cacheable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HookInput {
    /// The words' *values* — the one input that makes a hook uncacheable.
    Words,
    /// `[llength $words]`.
    Nwords,
    /// The per-word `literal`/`dynamic`/`expanded`/`opaque` classification.
    Kinds,
    /// The resolved command name (fixed per slot).
    Command,
    /// The resolved subcommand word (fixed per slot).
    Subcommand,
    /// `tcl-version`.
    TclVersion,
    /// `dialect`.
    Dialect,
    /// `in-event-body`.
    InEventBody,
    /// The option-arity family's `option` / `option-index` /
    /// `option-value-start`.
    Option,
    /// The `constraints` family's structured view of the whole invocation —
    /// which options were supplied, their literal values, and the positional
    /// words after them. Content, so it keys the cache by content rather than
    /// by shape.
    Invocation,
}

/// The inputs a hook declared it reads — the cacheability rule's input.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HookInputs {
    declared: Option<Vec<HookInput>>,
}

impl HookInputs {
    /// The default: the hook declared nothing, so it may read anything.
    /// Fully legal, never cacheable — `spec-packs.md`'s "a hook that declares
    /// broader dependencies stays fully legal but uncacheable".
    #[must_use]
    pub fn unrestricted() -> Self {
        Self { declared: None }
    }

    /// The hook declared exactly these inputs.
    #[must_use]
    pub fn declared(inputs: impl IntoIterator<Item = HookInput>) -> Self {
        let mut inputs: Vec<HookInput> = inputs.into_iter().collect();
        inputs.sort_unstable();
        inputs.dedup();
        Self {
            declared: Some(inputs),
        }
    }

    /// The declared inputs, or `None` when the hook declared none.
    #[must_use]
    pub fn inputs(&self) -> Option<&[HookInput]> {
        self.declared.as_deref()
    }

    /// Parse the DSL's `-inputs {…}` word list. An unknown word makes the
    /// whole declaration unrestricted rather than dropping a dependency the
    /// hook actually has — degrading to "slower but correct" is the only safe
    /// direction for a tolerance rule that governs caching.
    #[must_use]
    pub fn parse(words: &[&str]) -> Self {
        let mut inputs = Vec::with_capacity(words.len());
        for word in words {
            let input = match *word {
                "words" => HookInput::Words,
                "nwords" => HookInput::Nwords,
                "kinds" => HookInput::Kinds,
                "command" => HookInput::Command,
                "subcommand" => HookInput::Subcommand,
                "tcl-version" => HookInput::TclVersion,
                "dialect" => HookInput::Dialect,
                "in-event-body" => HookInput::InEventBody,
                "option" | "option-index" | "option-value-start" => HookInput::Option,
                "invocation" => HookInput::Invocation,
                _ => return Self::unrestricted(),
            };
            inputs.push(input);
        }
        Self::declared(inputs)
    }

    /// Whether the hook's body is given the `words` parameter at all.
    ///
    /// **This is what makes `-inputs` a declaration rather than a hint.** The
    /// shape cache keys on the words' *shape* — count and kinds — and not
    /// their content, so a hook that reads content while claiming not to would
    /// be served another call's answer whenever the shapes matched. Binding
    /// `words` only when it was declared turns that silent miscompile into a
    /// loud "no such variable" from the sandbox on the hook's first call, which
    /// the host reports and quarantines like any other hook failure.
    ///
    /// An undeclared (unrestricted) hook still gets `words`: it made no claim,
    /// and it is never cached.
    #[must_use]
    pub fn binds_words(&self) -> bool {
        self.declared
            .as_ref()
            .is_none_or(|inputs| inputs.iter().any(|input| matches!(input, HookInput::Words)))
    }

    /// **The cacheability rule.** A hook is shape-cacheable when it declared
    /// its inputs and none of them is the words' content: everything else it
    /// may read is either fixed for the slot (`command`, `subcommand`) or part
    /// of the shape key (`nwords`, `kinds`, `tcl-version`, `in-event-body`).
    ///
    /// `option` is *not* in the key, so declaring it is uncacheable today.
    /// `dialect` is in the key — a release-pinned body runs on an engine
    /// pinned to it — but declaring it still makes a hook uncacheable;
    /// widening this rule is the change to make if a pack needs it.
    ///
    /// [`Self::binds_words`] is what keeps this honest: a hook that is
    /// shape-cacheable by this rule is not handed the words at all.
    #[must_use]
    pub fn shape_only(&self) -> bool {
        self.declared
            .as_ref()
            .is_some_and(|inputs| inputs.iter().all(|input| Self::is_shape_input(*input)))
    }

    const fn is_shape_input(input: HookInput) -> bool {
        matches!(
            input,
            HookInput::Nwords
                | HookInput::Kinds
                | HookInput::Command
                | HookInput::Subcommand
                | HookInput::TclVersion
                | HookInput::InEventBody
        )
    }

    /// **The content-cacheability rule** (E-R14, principle P-B). A hook that
    /// declared its inputs and reads the *content* of the call — the words'
    /// values, or the `constraints` family's structured invocation view — is
    /// still memoisable: the sandbox is deterministic (E-R2), so the same
    /// content on the same slot has the same answer. It is keyed by a hash of
    /// that content instead of by the shape alone.
    ///
    /// This is what makes "an edit elsewhere in the document must not re-run
    /// hooks for unrelated call sites" true: an unchanged call site hashes the
    /// same and is answered from the cache.
    ///
    /// An **undeclared** hook stays uncacheable — it made no claim about what
    /// it reads, so nothing can be hashed on its behalf.
    #[must_use]
    pub fn content_cacheable(&self) -> bool {
        self.declared.as_ref().is_some_and(|inputs| {
            inputs.iter().all(|input| {
                Self::is_shape_input(*input)
                    || matches!(input, HookInput::Words | HookInput::Invocation)
            })
        })
    }
}

/// How a slot's answers may be memoised, decided once at allocation from its
/// declared inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    /// Never cached: the hook declared no inputs, so it may read anything.
    None,
    /// Keyed on the call's shape — word count, kinds, version, event flag.
    Shape,
    /// Keyed on the shape **and** a hash of the call's content.
    Content,
}

impl CacheMode {
    /// The mode `inputs` earn.
    #[must_use]
    pub fn of(inputs: &HookInputs) -> Self {
        if inputs.shape_only() {
            Self::Shape
        } else if inputs.content_cacheable() {
            Self::Content
        } else {
            Self::None
        }
    }

    const fn as_u8(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Shape => 1,
            Self::Content => 2,
        }
    }

    const fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Shape,
            2 => Self::Content,
            _ => Self::None,
        }
    }
}

/// The DSL's spelling of a word kind — the `kinds` entry a hook body reads.
///
/// One place, because the vocabulary is the registry's: a body compares
/// against `literal` and the loader documents `literal`, so the two cannot
/// drift apart into `Literal` versus `literal`.
#[must_use]
pub fn kind_word(kind: InvocationWordKind) -> &'static str {
    match kind {
        InvocationWordKind::Literal => "literal",
        InvocationWordKind::Dynamic => "dynamic",
        InvocationWordKind::Expanded => "expanded",
        InvocationWordKind::Opaque => "opaque",
    }
}

/// A pack hook's process-wide identity: which family, and which of that
/// family's slots.
///
/// Allocated once at pack load ([`allocate`]) and baked into the leaked
/// `CommandSpec` as a function pointer, so it must stay valid for the
/// process's life — hence a slot is never freed. Quarantine unbinds the
/// *behaviour* (the host stops answering for it), not the slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HookSlot {
    family: HookFamily,
    index: u16,
}

impl HookSlot {
    /// The family whose calling convention this slot obeys.
    #[must_use]
    pub const fn family(self) -> HookFamily {
        self.family
    }

    /// The slot's index within its family.
    #[must_use]
    pub const fn index(self) -> u16 {
        self.index
    }
}

/// One word as a hook body sees it: its value when literal, and its kind
/// always. Mirrors [`InvocationWord`] without borrowing, because a hook call
/// crosses into an engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HookWord<'w> {
    /// The word's Tcl value, or `""` for a non-literal word — the DSL's own
    /// convention, so a body that forgets to consult `kinds` sees nothing
    /// rather than source spelling.
    pub value: &'w str,
    /// The word's classification.
    pub kind: InvocationWordKind,
}

/// The `constraints` family's structured view of the invocation — what its
/// reading verbs answer from.
///
/// Borrowed straight from the analyser's leading-option walk, so building it
/// costs nothing beyond the slices that walk already produced.
#[derive(Debug, Clone, Copy)]
pub struct ConstraintCallCtx<'w> {
    /// Canonical option name → its first literal value, in call order.
    pub options: &'w [(&'static str, Option<&'w str>)],
    /// The positional words after the leading option run, `None` where not
    /// statically known.
    pub positionals: &'w [Option<&'w str>],
    /// Whether the invocation was read to its end — the hook's `complete`.
    pub complete: bool,
}

/// The option-arity family's extra context — the one family whose Rust
/// signature takes more than `args`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptionCallCtx<'w> {
    /// The option word as written (`-errorstack`).
    pub option: &'w str,
    /// Its 0-based index in `words`.
    pub index: usize,
    /// The index of the first value word — the shipped hook's `start`.
    pub value_start: usize,
}

/// Everything a hook invocation carries across the seam: the DSL's `words`
/// and the `ctx` keys that vary per call. The keys fixed per slot (`command`,
/// `subcommand`) are the host's, which knows them from the declaration.
#[derive(Debug, Clone, Copy)]
pub struct HookCall<'w> {
    /// The call's argument words after the command (or subcommand) name.
    pub words: &'w [HookWord<'w>],
    /// `tcl-version`, or `None` when the profile names no release.
    pub version: Option<TclVersion>,
    /// `in-event-body`.
    pub in_event_body: bool,
    /// The option-arity family's extra keys.
    pub option: Option<OptionCallCtx<'w>>,
    /// The `constraints` family's structured invocation view.
    pub constraints: Option<ConstraintCallCtx<'w>>,
    /// `dialect` — the profile name the call is being analysed under
    /// (`f5-irules`, `tcl9.0`, …), or [`None`] when nothing set one.
    ///
    /// **Not** derived from `version`: a dialect is not a release. Deriving it
    /// was the bug — an iRules document reported `tcl9.0`, so a hook could
    /// never tell the two apart.
    pub dialect: Option<&'static str>,
    /// The `evaluate` family's declared store targets, as the body names
    /// them (`write TARGET VALUE`); empty for every other family.
    pub targets: &'w [usize],
    /// The `evaluate` family's own budget, which narrows the host's for this
    /// call; the default narrows nothing.
    pub budget: ImplementationBudget,
    /// The `evaluate` family's declared context dependencies, in
    /// declaration order: part of what a cached answer is compared on;
    /// empty for every other family.
    pub depends: &'w [ContextDependency],
}

impl HookCall<'_> {
    /// Whether every word is literal — the precondition the fold families and
    /// the option-arity hook run under.
    #[must_use]
    pub fn all_literal(&self) -> bool {
        self.words
            .iter()
            .all(|word| word.kind == InvocationWordKind::Literal)
    }
}

/// What an `evaluate` body stated: the result `fold` named, and one store
/// per declared target in call order — `write TARGET VALUE` as the value,
/// `preserve TARGET` as `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationAnswer {
    /// The invocation's result, when the body called `fold`.
    pub result: Option<String>,
    /// `(target, Some(value))` for a `write`, `(target, None)` for a
    /// `preserve`, in call order.
    pub stores: Vec<(usize, Option<String>)>,
}

/// What a hook invocation produced: the emitter verb it called, or an
/// abstention.
///
/// One enum for all ten families because the *protocol* is one protocol; the
/// thunk that receives it knows its family and converts, applying that
/// family's silence to [`Self::Abstain`] and to any answer of the wrong shape
/// (a host bug must degrade, not mis-answer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookAnswer {
    /// No verb was called, the body errored, the budget blew, the hook is
    /// quarantined, or no host is installed. Every one of those is the
    /// family's documented silence.
    Abstain,
    /// `role IDX ROLE` — the complete index→role map, in one invocation.
    Roles(Vec<(u8, ArgRole)>),
    /// `prefix IDX {Exactly N}`.
    Prefixes(Vec<(u8, AppendedArity)>),
    /// `timing IDX SameInvocation|Deferred|ReferenceOnly`.
    Timings(Vec<(u8, ScriptTiming)>),
    /// `fold VALUE`.
    Fold(String),
    /// `sink-suppressed`. (`sink-applies` is the silence, so it arrives as
    /// [`Self::Abstain`] and means the same thing.)
    SinkSuppressed,
    /// `reject MESSAGE`.
    Reject(String),
    /// `invalid …` / `abstain REASON`, already in the registry's typed shape.
    Literal(LiteralArgumentValidation),
    /// `missing-expr` / `missing-body` / `extra-words`.
    ClauseShape(ClauseShapeError),
    /// `invalid SLOT MESSAGE ?-conflict?`, one per report.
    Constraints(Vec<crate::spec::ConstraintReport>),
    /// `consume N ?-invalid MESSAGE?`.
    Consume {
        /// Words consumed, valid or not. `0` is a report, not an abstention.
        words: usize,
        /// The W141 message, when the hook rejected the value.
        invalid: Option<String>,
    },
    /// `fold` / `write` / `preserve` — an `evaluate` body's answer, every
    /// declared target spoken for.
    Evaluation(EvaluationAnswer),
}

/// The hook host, as the registry sees it.
///
/// The one method is deliberately family-blind: the host dispatches on
/// `slot.family()` itself, which is what lets a new family arrive without
/// changing this trait. Implementations must be **total** — a panic here is a
/// panic inside a `CommandSpec` accessor on the LSP's hot path, so the host
/// converts its own failures to [`HookAnswer::Abstain`] before returning
/// (`spec-packs.md`: "a panic, budget blowout, or stack overflow in a hook is
/// converted to abstention, never propagation").
pub trait PackHookHost {
    /// Answer one hook invocation.
    fn invoke(&self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer;

    /// Whether `slot`'s hook can run on this host now: `false` once it is
    /// quarantined or its pack poisoned. A state of the host, never a
    /// verdict on any call's inputs.
    fn is_available(&self, slot: HookSlot) -> bool {
        let _ = slot;
        true
    }
}

/// Per-family allocation counters. Process-global because a slot is baked
/// into a leaked `&'static CommandSpec` that every thread shares.
static NEXT_SLOT: [AtomicU32; HOOK_FAMILIES.len()] =
    [const { AtomicU32::new(0) }; HOOK_FAMILIES.len()];

/// Per-slot cache mode, decided by the declared inputs at allocation.
static CACHE_MODE: [[AtomicU8; SLOTS_PER_FAMILY]; HOOK_FAMILIES.len()] =
    [const { [const { AtomicU8::new(0) }; SLOTS_PER_FAMILY] }; HOOK_FAMILIES.len()];

/// Allocate a slot for a hook of `family` whose body declared `inputs`.
///
/// `None` when the family's slots are exhausted; the caller installs no hook
/// and the command keeps its declarative facts.
#[must_use]
pub fn allocate(family: HookFamily, inputs: &HookInputs) -> Option<HookSlot> {
    let index = NEXT_SLOT[family.index()].fetch_add(1, Ordering::Relaxed);
    let index = usize::try_from(index).ok()?;
    if index >= SLOTS_PER_FAMILY {
        return None;
    }
    CACHE_MODE[family.index()][index].store(CacheMode::of(inputs).as_u8(), Ordering::Relaxed);
    Some(HookSlot {
        family,
        index: u16::try_from(index).ok()?,
    })
}

/// The slot for the hook `identity` names, minting one only the first time
/// that identity is seen and **rebinding** it on every later load.
///
/// # Why identity and not content
///
/// Slots are process-global and never freed — a slot's index is baked into a
/// leaked `CommandSpec` as a function pointer, so nothing can be handed back.
/// Allocating per distinct pack *content* therefore made every edit of a pack
/// spend another slot out of the 64 a family has: a long editing session on
/// one pack silently ran the family dry, and from then on new hooks installed
/// nothing at all until the process restarted. Because the exhaustion is
/// silent and only shows up as a command losing its hook, it is close to
/// undiagnosable from the outside.
///
/// Keying on *which hook this is* — pack, command, owner, family — instead
/// bounds the budget by how many distinct hooks a workspace declares, which is
/// what the 64 was always meant to bound. Re-editing one hook a thousand times
/// costs the one slot it started with, and the newest body is simply what that
/// slot now runs; every registry generation still points at the same slot, and
/// the stale ones were stale regardless.
pub fn allocate_stable(
    family: HookFamily,
    identity: &str,
    inputs: &HookInputs,
) -> Option<HookSlot> {
    static BY_IDENTITY: LazyLock<Mutex<HashMap<(HookFamily, String), HookSlot>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let key = (family, identity.to_owned());
    let mut table = match BY_IDENTITY.lock() {
        Ok(table) => table,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(&slot) = table.get(&key) {
        // Rebind: the body behind this identity may have changed, and with it
        // whether it is shape-cacheable.
        CACHE_MODE[family.index()][usize::from(slot.index)]
            .store(CacheMode::of(inputs).as_u8(), Ordering::Relaxed);
        // A slot's cached answers describe the *old* body.
        clear_cache();
        return Some(slot);
    }
    let slot = allocate(family, inputs)?;
    table.insert(key, slot);
    Some(slot)
}

/// How this slot's declared inputs let its answers be memoised.
#[must_use]
pub fn cache_mode(slot: HookSlot) -> CacheMode {
    CacheMode::from_u8(
        CACHE_MODE[slot.family.index()][usize::from(slot.index)].load(Ordering::Relaxed),
    )
}

/// Whether this slot's answers are memoised at all.
#[must_use]
pub fn is_cacheable(slot: HookSlot) -> bool {
    cache_mode(slot) != CacheMode::None
}

thread_local! {
    /// The host serving this thread's pack hooks. Thread-local because a host
    /// owns per-pack VM instances, which are not `Send`.
    static HOST: RefCell<Option<Rc<dyn PackHookHost>>> = const { RefCell::new(None) };

    /// The shape-keyed answer cache, and its counters.
    static SHAPE_CACHE: RefCell<ShapeCache> = RefCell::new(ShapeCache::default());

    /// This thread's evaluator generation.
    static GENERATION: Cell<EvaluatorGeneration> =
        const { Cell::new(EvaluatorGeneration::NO_HOST) };

    /// Whether this thread's host was built from a published plan
    /// ([`install_plan_host`]), so the registered installer keeps it in step
    /// with the plan; a host installed directly ([`install_host`]) is its
    /// installer's own business.
    static FROM_PLAN: Cell<bool> = const { Cell::new(false) };

    /// The commands the host's engines dispatched since the last take.
    static SPENT: Cell<u64> = const { Cell::new(0) };

    /// The dialect whose registry this thread is currently analysing against,
    /// as [`DialectProfile::name`] spells it (`f5-irules`, `tcl9.0`, …).
    ///
    /// Ambient rather than a parameter because the registry's hook thunks are
    /// plain `fn` pointers with signatures fixed by the spec fields they back
    /// — there is no seam to pass it through. It is per-thread, and scoped by
    /// [`DialectScope`], because one worker analyses documents of different
    /// dialects one after another.
    static DIALECT: RefCell<Option<&'static str>> = const { RefCell::new(None) };
}

/// Sets this thread's ambient dialect for as long as it is held, restoring
/// whatever was there before on drop.
///
/// A guard rather than a setter so an early return, a `?`, or a panic in the
/// analysis it wraps cannot leave the next document on this worker reading a
/// stale dialect.
#[derive(Debug)]
pub struct DialectScope(Option<&'static str>);

impl DialectScope {
    /// Enter a scope in which pack hooks see `dialect`.
    #[must_use]
    pub fn enter(dialect: Option<&'static str>) -> Self {
        let previous = DIALECT.with(|slot| slot.replace(dialect));
        Self(previous)
    }
}

impl Drop for DialectScope {
    fn drop(&mut self) {
        DIALECT.with(|slot| {
            *slot.borrow_mut() = self.0;
        });
    }
}

/// This thread's ambient dialect, if one is in scope.
#[must_use]
pub fn current_dialect() -> Option<&'static str> {
    DIALECT.with(|slot| *slot.borrow())
}

/// `true` while any thread has a host installed — the one check an
/// unaccelerated process pays.
static ANY_HOST: AtomicBool = AtomicBool::new(false);

/// Builds and installs this thread's host on demand, registered by the layer
/// that owns the hook host (`tcl-spectcl`).
///
/// The registry cannot build a host — that needs the loader and a VM, both of
/// which sit above it — so the capability is registered downwards once and
/// called from [`dispatch`].
static INSTALLER: OnceLock<fn()> = OnceLock::new();

/// Register the thread-host installer. Idempotent; the first registration wins.
///
/// # Why this exists
///
/// A host is per **thread** (it owns VMs, which are not `Send`), so every
/// thread that might dispatch a hook has to build one. Relying on each worker
/// closure to do that by hand made hook availability depend on which Tokio
/// worker happened to run the task: the analysis and optimisation closures
/// installed a host, but the semantic-token and minify ones did not, so the
/// same document could resolve a pack's commands or not depending on
/// scheduling. Installing lazily at the one point that actually needs a host —
/// the dispatch — makes that impossible to get wrong by omission.
pub fn set_installer(installer: fn()) {
    let _ = INSTALLER.set(installer);
}

/// The next generation no state has had. Process-wide, so a generation
/// minted on one thread is never minted again on another.
static NEXT_GENERATION: AtomicU32 = AtomicU32::new(1);

/// A generation no other state shares.
fn fresh_generation() -> EvaluatorGeneration {
    // Zero is the host-absent state; a wrapped counter skips it.
    EvaluatorGeneration(NEXT_GENERATION.fetch_add(1, Ordering::Relaxed).max(1))
}

/// The generation of a host built from published plan `plan`: one per plan,
/// shared by every thread whose host was built from it.
fn plan_generation(plan: u64) -> EvaluatorGeneration {
    static PLANS: LazyLock<Mutex<HashMap<u64, EvaluatorGeneration>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut plans = match PLANS.lock() {
        Ok(plans) => plans,
        Err(poisoned) => poisoned.into_inner(),
    };
    *plans.entry(plan).or_insert_with(fresh_generation)
}

/// Install `host` as this thread's pack-hook host, replacing any previous one.
///
/// Every pack-declared hook on this thread abstains until this is called, so
/// a worker thread that never installs a host behaves exactly like a build
/// with no packs loaded. The host gets a generation of its own; a host built
/// from a published plan is installed with [`install_plan_host`] instead, so
/// every worker serving that plan shares its memoised answers.
pub fn install_host(host: Rc<dyn PackHookHost>) {
    install(host, fresh_generation(), false);
}

/// Install `host`, built from the published plan `plan`, as this thread's
/// host: every thread that installs a host for one plan is at one
/// generation.
pub fn install_plan_host(host: Rc<dyn PackHookHost>, plan: u64) {
    install(host, plan_generation(plan), true);
}

fn install(host: Rc<dyn PackHookHost>, generation: EvaluatorGeneration, from_plan: bool) {
    ANY_HOST.store(true, Ordering::Relaxed);
    clear_cache();
    HOST.with(|slot| {
        slot.borrow_mut().replace(host);
    });
    GENERATION.with(|current| current.set(generation));
    FROM_PLAN.with(|current| current.set(from_plan));
}

/// Remove this thread's host; every pack hook abstains again, and the
/// thread is at [`EvaluatorGeneration::NO_HOST`].
pub fn clear_host() {
    clear_cache();
    HOST.with(|slot| {
        slot.borrow_mut().take();
    });
    GENERATION.with(|current| current.set(EvaluatorGeneration::NO_HOST));
    FROM_PLAN.with(|current| current.set(false));
}

/// Record that the host's engine dispatched `commands` answering the call
/// in flight: a declared implementation's evaluation charges them to its
/// budget one-to-one (`docs/design/compiler/value-evaluation.md` § *Units
/// and charges*).
pub fn record_commands_spent(commands: u64) {
    SPENT.with(|spent| spent.set(spent.get().saturating_add(commands)));
}

/// The commands recorded since the last take, which starts the count
/// again.
#[must_use]
pub fn take_commands_spent() -> u64 {
    SPENT.with(|spent| spent.replace(0))
}

/// Record that this thread's host quarantined a hook or poisoned a pack:
/// the cached answers go, the thread takes a generation no other state
/// has, since what its host can still run is its own, and the process's
/// [`evaluator_epoch`] moves.
pub fn note_quarantine() {
    clear_cache();
    GENERATION.with(|current| current.set(fresh_generation()));
    advance_evaluator_epoch();
}

/// The process's evaluator epoch: it moves whenever the evaluators some
/// thread serves change in a way a memo shared between threads must see —
/// a hook plan published ([`advance_evaluator_epoch`], which the plan's
/// owner calls), or a hook quarantined ([`note_quarantine`]).
static EVALUATOR_EPOCH: AtomicU64 = AtomicU64::new(0);

/// The process's evaluator epoch. A thread's own
/// [`evaluator_generation`] keys what it computes; this is the one number a
/// memo shared by every thread — the language server's query database —
/// takes as an input, so a plan reload or a quarantine anywhere re-keys it
/// (`docs/design/lanes/value-transfers.md`, D104).
#[must_use]
pub fn evaluator_epoch() -> u64 {
    EVALUATOR_EPOCH.load(Ordering::Relaxed)
}

/// Move the process's [`evaluator_epoch`]: the evaluators some thread
/// serves have changed.
pub fn advance_evaluator_epoch() {
    EVALUATOR_EPOCH.fetch_add(1, Ordering::Relaxed);
}

/// This thread's evaluator generation, building its host first as
/// [`dispatch`] would, so the generation names the evaluators an analysis
/// that starts now runs with.
#[must_use]
pub fn evaluator_generation() -> EvaluatorGeneration {
    if ANY_HOST.load(Ordering::Relaxed) || INSTALLER.get().is_some() {
        ensure_host();
    }
    GENERATION.with(Cell::get)
}

/// This thread's host, first brought up to the published plan by the
/// registered installer.
///
/// Unless the thread's host was installed directly ([`install_host`]), the
/// installer runs before the host is read, every time: it builds a host for
/// a thread that has none — abstaining there would answer "no packs" on a
/// thread that simply had not been initialised — and rebuilds one whose
/// plan was superseded. A thread that kept whatever plan host it last had
/// would serve the old plan: a pool thread last used under plan N, reached
/// by a query no worker closure re-synced, would compute a lattice through
/// plan N's host and memoise it under plan N+1's pack key and evaluator
/// epoch. When nothing moved the installer returns after one atomic load
/// and one thread-local read.
fn ensure_host() -> Option<Rc<dyn PackHookHost>> {
    let direct = HOST
        .with(|slot| slot.borrow().clone())
        .filter(|_| !FROM_PLAN.with(Cell::get));
    if direct.is_some() {
        return direct;
    }
    if let Some(installer) = INSTALLER.get() {
        installer();
    }
    HOST.with(|slot| slot.borrow().clone())
}

/// Whether this thread has a host installed.
#[must_use]
pub fn has_host() -> bool {
    ANY_HOST.load(Ordering::Relaxed) && HOST.with(|slot| slot.borrow().is_some())
}

/// Whether this thread's host can run `slot`'s hook now, building the host
/// first as [`dispatch`] would. `false` — no host, or the hook quarantined
/// or its pack poisoned — is transient: it says nothing about any call's
/// inputs, so an answer that rests on it must not be kept as a verdict.
#[must_use]
pub fn slot_available(slot: HookSlot) -> bool {
    if !ANY_HOST.load(Ordering::Relaxed) {
        return false;
    }
    ensure_host().is_some_and(|host| host.is_available(slot))
}

/// The shape key: everything a shape-cacheable hook may read, packed.
///
/// `kinds` is two bits per word, so a call with more than 64 words has no
/// shape key and is answered by the host every time (uncached, still
/// correct).
///
/// `content` is `0` for a [`CacheMode::Shape`] slot and a hash of the call's
/// literal content for a [`CacheMode::Content`] one — which is what lets a
/// hook that genuinely reads values still be answered from the cache when
/// nothing at *its* call site changed (E-R14, principle P-B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ShapeKey {
    slot: HookSlot,
    nwords: u16,
    kinds: u128,
    /// `tcl-version` as a stable discriminant — `TclVersion` is `Ord` but not
    /// `Hash`, and only its identity matters here.
    version: Option<&'static str>,
    /// The profile the call is analysed under. A hook body may not read it
    /// and stay cacheable, but a release-pinned body runs on an engine
    /// pinned to it, so one release's answer is never served under another.
    dialect: Option<&'static str>,
    in_event_body: bool,
    content: u64,
}

/// Everything a content-keyed answer rests on beyond its [`ShapeKey`]: the
/// words' values, the `constraints` family's invocation view, and the
/// `evaluate` family's declared targets, budget and dependencies. Kept with
/// the answer and compared on every hit, because the key's `content` is a
/// hash — the bucket, never the proof. The profile the target semantics
/// derive from is the key's `dialect`, compared exactly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct CallContent {
    words: Vec<String>,
    constraints: Option<ConstraintContent>,
    targets: Vec<usize>,
    budget: ImplementationBudget,
    depends: Vec<ContextDependency>,
}

/// An owned copy of a [`ConstraintCallCtx`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConstraintContent {
    options: Vec<(&'static str, Option<String>)>,
    positionals: Vec<Option<String>>,
    complete: bool,
}

impl CallContent {
    /// The content of `call` a `mode` slot's answer may depend on: all of
    /// it for a content-keyed slot, nothing for a shape-keyed one — whose
    /// key already holds everything its body is given.
    fn of(call: &HookCall<'_>, mode: CacheMode) -> Self {
        if mode != CacheMode::Content {
            return Self::default();
        }
        Self {
            words: call
                .words
                .iter()
                .map(|word| word.value.to_owned())
                .collect(),
            constraints: call.constraints.map(|view| ConstraintContent {
                options: view
                    .options
                    .iter()
                    .map(|(name, value)| (*name, value.map(str::to_owned)))
                    .collect(),
                positionals: view
                    .positionals
                    .iter()
                    .map(|word| word.map(str::to_owned))
                    .collect(),
                complete: view.complete,
            }),
            targets: call.targets.to_vec(),
            budget: call.budget,
            depends: call.depends.to_vec(),
        }
    }

    /// Whether `call` has exactly this content, compared without copying.
    fn matches(&self, call: &HookCall<'_>, mode: CacheMode) -> bool {
        if mode != CacheMode::Content {
            return true;
        }
        let constraints_match = match (&self.constraints, call.constraints) {
            (None, None) => true,
            (Some(kept), Some(view)) => {
                kept.complete == view.complete
                    && kept.options.len() == view.options.len()
                    && kept.options.iter().zip(view.options).all(
                        |((name, value), (other, other_value))| {
                            name == other && value.as_deref() == *other_value
                        },
                    )
                    && kept.positionals.len() == view.positionals.len()
                    && kept
                        .positionals
                        .iter()
                        .zip(view.positionals)
                        .all(|(kept, word)| kept.as_deref() == *word)
            }
            _ => false,
        };
        constraints_match
            && self.words.len() == call.words.len()
            && self
                .words
                .iter()
                .zip(call.words)
                .all(|(kept, word)| kept == word.value)
            && self.targets == call.targets
            && self.budget == call.budget
            && self.depends == call.depends
    }
}

/// One cached answer, with the content it was computed for.
struct CacheEntry {
    content: CallContent,
    answer: HookAnswer,
}

#[derive(Default)]
struct ShapeCache {
    entries: HashMap<ShapeKey, CacheEntry>,
    hits: u64,
    misses: u64,
}

/// Hit / miss / entry counts for this thread's shape cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CacheStats {
    /// Invocations answered from the cache.
    pub hits: u64,
    /// Invocations that reached the host.
    pub misses: u64,
    /// Distinct shapes resident.
    pub entries: usize,
}

/// This thread's shape-cache counters.
#[must_use]
pub fn cache_stats() -> CacheStats {
    SHAPE_CACHE.with(|cache| {
        let cache = cache.borrow();
        CacheStats {
            hits: cache.hits,
            misses: cache.misses,
            entries: cache.entries.len(),
        }
    })
}

/// Drop every cached answer and zero the counters. Called on host install /
/// removal, and by the host when a hook is quarantined.
pub fn clear_cache() {
    SHAPE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.entries.clear();
        cache.hits = 0;
        cache.misses = 0;
    });
}

/// A stable hash of everything a [`CacheMode::Content`] slot may read beyond
/// the shape: the words' literal values and the `constraints` family's
/// structured invocation view. The bucket a content-keyed answer is found
/// in; [`CallContent::matches`] is what proves the hit.
fn content_hash(call: &HookCall<'_>) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    #[cfg(test)]
    if let Some(forced) = tests::FORCED_CONTENT_HASH.with(Cell::get) {
        return forced;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for word in call.words {
        word.value.hash(&mut hasher);
    }
    if let Some(constraints) = call.constraints {
        for (name, value) in constraints.options {
            name.hash(&mut hasher);
            value.hash(&mut hasher);
        }
        constraints.positionals.hash(&mut hasher);
        constraints.complete.hash(&mut hasher);
    }
    hasher.finish()
}

fn shape_key(slot: HookSlot, call: &HookCall<'_>, mode: CacheMode) -> Option<ShapeKey> {
    if call.words.len() > 64 {
        return None;
    }
    let mut kinds: u128 = 0;
    for (position, word) in call.words.iter().enumerate() {
        let bits: u128 = match word.kind {
            InvocationWordKind::Literal => 0,
            InvocationWordKind::Dynamic => 1,
            InvocationWordKind::Expanded => 2,
            InvocationWordKind::Opaque => 3,
        };
        kinds |= bits << (position * 2);
    }
    Some(ShapeKey {
        slot,
        nwords: u16::try_from(call.words.len()).ok()?,
        kinds,
        version: call.version.map(TclVersion::version_string),
        dialect: call.dialect,
        in_event_body: call.in_event_body,
        content: match mode {
            CacheMode::Content => content_hash(call),
            CacheMode::None | CacheMode::Shape => 0,
        },
    })
}

/// Resident-entry ceiling for the shape cache.
///
/// A content-keyed entry is per *distinct call content*, so a very large
/// workspace could otherwise grow the table without bound. At the ceiling the
/// table is dropped wholesale rather than evicted one by one: the cache is a
/// pure memo, so losing it costs a re-run and never an answer.
const MAX_CACHE_ENTRIES: usize = 8192;

/// Invoke a pack hook: the cache first when the slot's declared inputs allow
/// it, then this thread's host, then the family's silence.
#[must_use]
pub fn dispatch(slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
    if !ANY_HOST.load(Ordering::Relaxed) {
        return HookAnswer::Abstain;
    }
    let mode = cache_mode(slot);
    let key = (mode != CacheMode::None)
        .then(|| shape_key(slot, call, mode))
        .flatten();
    if let Some(key) = key
        && let Some(hit) = SHAPE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let hit = cache
                .entries
                .get(&key)
                .filter(|entry| entry.content.matches(call, mode))
                .map(|entry| entry.answer.clone());
            if hit.is_some() {
                cache.hits += 1;
            }
            hit
        })
    {
        return hit;
    }
    let Some(host) = ensure_host() else {
        return HookAnswer::Abstain;
    };
    let answer = host.invoke(slot, call);
    if let Some(key) = key {
        SHAPE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.misses += 1;
            if cache.entries.len() >= MAX_CACHE_ENTRIES {
                cache.entries.clear();
            }
            // A colliding bucket holds the latest content's answer.
            cache.entries.insert(
                key,
                CacheEntry {
                    content: CallContent::of(call, mode),
                    answer: answer.clone(),
                },
            );
        });
    }
    answer
}

/// Intern a hook-produced message as `&'static str`.
///
/// The registry's message fields are `&'static` because shipped messages live
/// in `.rodata`. A pack's are minted at load and live for the process, exactly
/// like the pack's leaked spec; the map collapses repeats so a hook emitting
/// the same rejection on every call site leaks once, not once per call.
fn intern(message: &str) -> &'static str {
    static INTERNED: LazyLock<Mutex<HashMap<String, &'static str>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut table = match INTERNED.lock() {
        Ok(table) => table,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(existing) = table.get(message) {
        return existing;
    }
    let leaked: &'static str = Box::leak(message.to_owned().into_boxed_str());
    table.insert(message.to_owned(), leaked);
    leaked
}

/// Build the DSL's `words` view from the compatibility `&[&str]` a shipped
/// hook signature carries: every word is literal, which is what that view
/// means.
///
/// **Fidelity, stated plainly.** Only the families whose shipped Rust
/// signature carries [`InvocationArguments`] — the literal-argument validator
/// and the command-prefix resolver — can report a word as `dynamic`,
/// `expanded`, or `opaque`; the families whose signature is a bare `&[&str]`
/// have no such information to pass on, so their `kinds` reads `literal`
/// throughout. That is exactly as sound as the shipped Rust hooks are, since
/// they receive the same strings — but a body must not read `kinds` as proof
/// of literality on those families. Widening it is a change to the registry's
/// hook signatures, not to this boundary.
fn literal_words<'w>(args: &'w [&'w str]) -> Vec<HookWord<'w>> {
    args.iter()
        .map(|value| HookWord {
            value,
            kind: InvocationWordKind::Literal,
        })
        .collect()
}

fn structured_words(args: InvocationArguments<'_>) -> Vec<HookWord<'_>> {
    (0..args.len())
        .map(|index| {
            let word = args.get(index).unwrap_or(InvocationWord::Opaque);
            HookWord {
                value: word.literal().unwrap_or(""),
                kind: word.kind(),
            }
        })
        .collect()
}

fn call_of<'w>(words: &'w [HookWord<'w>], version: Option<TclVersion>) -> HookCall<'w> {
    HookCall {
        words,
        version,
        in_event_body: false,
        option: None,
        constraints: None,
        dialect: current_dialect(),
        targets: &[],
        budget: ImplementationBudget::default(),
        depends: &[],
    }
}

fn arg_role_thunk<const N: u16>(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let words = literal_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ArgRoleResolver,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Roles(roles) => roles,
        _ => Vec::new(),
    }
}

fn command_prefix_thunk<const N: u16>(
    args: CommandPrefixArguments<'_>,
) -> Vec<(u8, AppendedArity)> {
    let words = structured_words(args.words());
    match dispatch(
        HookSlot {
            family: HookFamily::CommandPrefixResolver,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Prefixes(prefixes) => prefixes,
        _ => Vec::new(),
    }
}

fn script_timing_thunk<const N: u16>(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    let words = literal_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ScriptTimingResolver,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Timings(timings) => timings,
        _ => Vec::new(),
    }
}

fn const_fold_thunk<const N: u16>(args: &[&str]) -> Option<String> {
    let words = literal_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ConstFold,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Fold(value) => Some(value),
        _ => None,
    }
}

fn const_fold_versioned_thunk<const N: u16>(
    args: &[&str],
    version: Option<TclVersion>,
) -> Option<String> {
    let words = literal_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ConstFoldVersioned,
            index: N,
        },
        &call_of(&words, version),
    ) {
        HookAnswer::Fold(value) => Some(value),
        _ => None,
    }
}

fn taint_sink_thunk<const N: u16>(args: &[&str]) -> bool {
    let words = literal_words(args);
    // Silence keeps the finding alive: only an explicit `sink-suppressed`
    // turns the sink off.
    !matches!(
        dispatch(
            HookSlot {
                family: HookFamily::TaintSinkGate,
                index: N,
            },
            &call_of(&words, None),
        ),
        HookAnswer::SinkSuppressed
    )
}

fn context_gate_thunk<const N: u16>(args: &[&str], in_event_body: bool) -> Option<&'static str> {
    let words = literal_words(args);
    let call = HookCall {
        words: &words,
        version: None,
        in_event_body,
        option: None,
        constraints: None,
        dialect: current_dialect(),
        targets: &[],
        budget: ImplementationBudget::default(),
        depends: &[],
    };
    match dispatch(
        HookSlot {
            family: HookFamily::ContextGate,
            index: N,
        },
        &call,
    ) {
        HookAnswer::Reject(message) => Some(intern(&message)),
        _ => None,
    }
}

fn literal_validator_thunk<const N: u16>(
    args: InvocationArguments<'_>,
) -> LiteralArgumentValidation {
    let words = structured_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::LiteralArgumentValidator,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::Literal(validation) => validation,
        _ => LiteralArgumentValidation::Valid,
    }
}

fn clause_shape_thunk<const N: u16>(args: &[&str]) -> Option<ClauseShapeError> {
    let words = literal_words(args);
    match dispatch(
        HookSlot {
            family: HookFamily::ClauseShapeCheck,
            index: N,
        },
        &call_of(&words, None),
    ) {
        HookAnswer::ClauseShape(error) => Some(error),
        _ => None,
    }
}

fn option_arity_thunk<const N: u16>(args: &[&str], start: usize) -> OptionValueOutcome {
    let word_facts = literal_words(args);
    let option = OptionCallCtx {
        option: args.get(start.wrapping_sub(1)).copied().unwrap_or(""),
        index: start.saturating_sub(1),
        value_start: start,
    };
    let call = HookCall {
        words: &word_facts,
        version: None,
        in_event_body: false,
        option: Some(option),
        constraints: None,
        dialect: current_dialect(),
        targets: &[],
        budget: ImplementationBudget::default(),
        depends: &[],
    };
    match dispatch(
        HookSlot {
            family: HookFamily::OptionArity,
            index: N,
        },
        &call,
    ) {
        HookAnswer::Consume { words, invalid } => OptionValueOutcome {
            words,
            invalid: invalid.as_deref().map(intern),
        },
        // Silence still consumes one word — `consume 0` is a report, not an
        // abstention.
        _ => OptionValueOutcome {
            words: 1,
            invalid: None,
        },
    }
}

/// The `constraints` family's thunk (E-R14).
///
/// Reached **only** from a spec that declared a `constraints` hook, and only
/// after the declarative relations reported nothing — so a pack that declares
/// none never runs this, and neither does any shipped command.
fn constraints_thunk<const N: u16>(
    facts: &crate::spec::OptionFacts<'_>,
) -> Vec<crate::spec::ConstraintReport> {
    // The flattened word view is what `words` / `kinds` / `nwords` answer
    // from; the structured view beside it is what the reading verbs use.
    let mut words: Vec<HookWord<'_>> = Vec::with_capacity(facts.options.len() * 2);
    for (name, value) in facts.options {
        words.push(HookWord {
            value: name,
            kind: InvocationWordKind::Literal,
        });
        if let Some(value) = value {
            words.push(HookWord {
                value,
                kind: InvocationWordKind::Literal,
            });
        }
    }
    for positional in facts.positionals {
        words.push(match positional {
            Some(value) => HookWord {
                value,
                kind: InvocationWordKind::Literal,
            },
            None => HookWord {
                value: "",
                kind: InvocationWordKind::Dynamic,
            },
        });
    }
    let call = HookCall {
        words: &words,
        version: None,
        in_event_body: false,
        option: None,
        constraints: Some(ConstraintCallCtx {
            options: facts.options,
            positionals: facts.positionals,
            complete: facts.complete,
        }),
        dialect: current_dialect(),
        targets: &[],
        budget: ImplementationBudget::default(),
        depends: &[],
    };
    match dispatch(
        HookSlot {
            family: HookFamily::Constraints,
            index: N,
        },
        &call,
    ) {
        HookAnswer::Constraints(reports) => reports,
        _ => Vec::new(),
    }
}

/// The 64 slot indices, handed to a macro that needs one item per slot.
macro_rules! with_slot_indices {
    ($table:ident) => {
        $table! {
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63
        }
    };
}

/// One static table per family: `SLOTS_PER_FAMILY` distinct monomorphisations
/// of that family's thunk, so a slot's function pointer carries its identity
/// with no runtime indirection and no change to any shipped signature.
macro_rules! slot_tables {
    ($($index:literal),*) => {
        static ARG_ROLE_THUNKS: [ArgRoleResolver; SLOTS_PER_FAMILY] =
            [$(arg_role_thunk::<$index>),*];
        static COMMAND_PREFIX_THUNKS: [CommandPrefixResolver; SLOTS_PER_FAMILY] =
            [$(command_prefix_thunk::<$index>),*];
        static SCRIPT_TIMING_THUNKS: [ScriptTimingResolver; SLOTS_PER_FAMILY] =
            [$(script_timing_thunk::<$index>),*];
        static CONST_FOLD_THUNKS: [ConstFoldFn; SLOTS_PER_FAMILY] =
            [$(const_fold_thunk::<$index>),*];
        static CONST_FOLD_VERSIONED_THUNKS: [VersionedConstFoldFn; SLOTS_PER_FAMILY] =
            [$(const_fold_versioned_thunk::<$index>),*];
        static TAINT_SINK_THUNKS: [fn(&[&str]) -> bool; SLOTS_PER_FAMILY] =
            [$(taint_sink_thunk::<$index>),*];
        static CONTEXT_GATE_THUNKS: [ContextGate; SLOTS_PER_FAMILY] =
            [$(context_gate_thunk::<$index>),*];
        static LITERAL_VALIDATOR_THUNKS: [LiteralArgumentValidator; SLOTS_PER_FAMILY] =
            [$(literal_validator_thunk::<$index>),*];
        static CLAUSE_SHAPE_THUNKS: [ClauseShapeChecker; SLOTS_PER_FAMILY] =
            [$(clause_shape_thunk::<$index>),*];
        static OPTION_ARITY_THUNKS: [OptionValueHook; SLOTS_PER_FAMILY] =
            [$(option_arity_thunk::<$index>),*];
        static CONSTRAINTS_THUNKS: [crate::spec::ConstraintsHook; SLOTS_PER_FAMILY] =
            [$(constraints_thunk::<$index>),*];
    };
}

with_slot_indices! { slot_tables }

fn thunk_index(slot: HookSlot, family: HookFamily) -> Option<usize> {
    (slot.family == family).then(|| usize::from(slot.index))
}

/// The `arg_role_resolver` function pointer for `slot`.
#[must_use]
pub fn arg_role_resolver_fn(slot: HookSlot) -> Option<ArgRoleResolver> {
    thunk_index(slot, HookFamily::ArgRoleResolver).map(|index| ARG_ROLE_THUNKS[index])
}

/// The `command_prefix_resolver` function pointer for `slot`.
#[must_use]
pub fn command_prefix_resolver_fn(slot: HookSlot) -> Option<CommandPrefixResolver> {
    thunk_index(slot, HookFamily::CommandPrefixResolver).map(|index| COMMAND_PREFIX_THUNKS[index])
}

/// The `script_timing_resolver` function pointer for `slot`.
#[must_use]
pub fn script_timing_resolver_fn(slot: HookSlot) -> Option<ScriptTimingResolver> {
    thunk_index(slot, HookFamily::ScriptTimingResolver).map(|index| SCRIPT_TIMING_THUNKS[index])
}

/// The `const_fold` function pointer for `slot`.
#[must_use]
pub fn const_fold_fn(slot: HookSlot) -> Option<ConstFoldFn> {
    thunk_index(slot, HookFamily::ConstFold).map(|index| CONST_FOLD_THUNKS[index])
}

/// The `const_fold_versioned` function pointer for `slot`.
#[must_use]
pub fn const_fold_versioned_fn(slot: HookSlot) -> Option<VersionedConstFoldFn> {
    thunk_index(slot, HookFamily::ConstFoldVersioned)
        .map(|index| CONST_FOLD_VERSIONED_THUNKS[index])
}

/// The `taint_sink_gate` function pointer for `slot`.
#[must_use]
pub fn taint_sink_gate_fn(slot: HookSlot) -> Option<fn(&[&str]) -> bool> {
    thunk_index(slot, HookFamily::TaintSinkGate).map(|index| TAINT_SINK_THUNKS[index])
}

/// The `context_gate` function pointer for `slot`.
#[must_use]
pub fn context_gate_fn(slot: HookSlot) -> Option<ContextGate> {
    thunk_index(slot, HookFamily::ContextGate).map(|index| CONTEXT_GATE_THUNKS[index])
}

/// The `literal_argument_validator` function pointer for `slot`.
#[must_use]
pub fn literal_argument_validator_fn(slot: HookSlot) -> Option<LiteralArgumentValidator> {
    thunk_index(slot, HookFamily::LiteralArgumentValidator)
        .map(|index| LITERAL_VALIDATOR_THUNKS[index])
}

/// The `clause_shape_check` function pointer for `slot`.
#[must_use]
pub fn clause_shape_check_fn(slot: HookSlot) -> Option<ClauseShapeChecker> {
    thunk_index(slot, HookFamily::ClauseShapeCheck).map(|index| CLAUSE_SHAPE_THUNKS[index])
}

/// The option-arity function pointer for `slot`.
#[must_use]
pub fn option_arity_fn(slot: HookSlot) -> Option<OptionValueHook> {
    thunk_index(slot, HookFamily::OptionArity).map(|index| OPTION_ARITY_THUNKS[index])
}

/// The `constraints` function pointer for `slot`.
#[must_use]
pub fn constraints_fn(slot: HookSlot) -> Option<crate::spec::ConstraintsHook> {
    thunk_index(slot, HookFamily::Constraints).map(|index| CONSTRAINTS_THUNKS[index])
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        /// The content hash every call on this thread is forced to, when
        /// set: two different contents in one bucket.
        pub(super) static FORCED_CONTENT_HASH: Cell<Option<u64>> = const { Cell::new(None) };
    }

    struct FixedHost(HookAnswer);

    impl PackHookHost for FixedHost {
        fn invoke(&self, _slot: HookSlot, _call: &HookCall<'_>) -> HookAnswer {
            self.0.clone()
        }
    }

    struct CountingHost {
        calls: std::cell::Cell<u32>,
    }

    impl PackHookHost for CountingHost {
        fn invoke(&self, _slot: HookSlot, _call: &HookCall<'_>) -> HookAnswer {
            self.calls.set(self.calls.get() + 1);
            HookAnswer::Roles(vec![(0, ArgRole::VarWrite)])
        }
    }

    #[test]
    fn a_thread_with_no_host_abstains() {
        let slot = allocate(HookFamily::ConstFold, &HookInputs::unrestricted())
            .expect("a fold slot is available");
        let fold = const_fold_fn(slot).expect("the slot's family matches");
        assert_eq!(fold(&["abc"]), None);
    }

    #[test]
    fn the_fold_thunk_carries_its_slot_identity() {
        let first = allocate(HookFamily::ConstFold, &HookInputs::unrestricted()).unwrap();
        let second = allocate(HookFamily::ConstFold, &HookInputs::unrestricted()).unwrap();
        assert_ne!(first.index(), second.index());
        let first_fn = const_fold_fn(first).unwrap();
        let second_fn = const_fold_fn(second).unwrap();
        assert!(!std::ptr::fn_addr_eq(first_fn, second_fn));
    }

    #[test]
    fn a_wrong_family_slot_has_no_thunk() {
        let slot = allocate(HookFamily::ConstFold, &HookInputs::unrestricted()).unwrap();
        assert!(arg_role_resolver_fn(slot).is_none());
    }

    #[test]
    fn silence_is_per_family() {
        let sink = allocate(HookFamily::TaintSinkGate, &HookInputs::unrestricted()).unwrap();
        let gate = taint_sink_gate_fn(sink).unwrap();
        install_host(Rc::new(FixedHost(HookAnswer::Abstain)));
        // Silence means the sink applies.
        assert!(gate(&["x"]));
        install_host(Rc::new(FixedHost(HookAnswer::SinkSuppressed)));
        assert!(!gate(&["x"]));
        clear_host();
    }

    #[test]
    fn declared_shape_inputs_are_cacheable_and_words_are_not() {
        assert!(HookInputs::parse(&["nwords", "kinds"]).shape_only());
        assert!(!HookInputs::parse(&["words"]).shape_only());
        assert!(!HookInputs::unrestricted().shape_only());
        // An unknown input word degrades to unrestricted, never to cacheable.
        assert!(!HookInputs::parse(&["nwords", "moon-phase"]).shape_only());
    }

    #[test]
    fn a_shape_cacheable_resolver_reaches_the_host_once_per_shape() {
        let slot = allocate(
            HookFamily::ArgRoleResolver,
            &HookInputs::parse(&["nwords", "kinds"]),
        )
        .unwrap();
        let resolver = arg_role_resolver_fn(slot).unwrap();
        let host = Rc::new(CountingHost {
            calls: std::cell::Cell::new(0),
        });
        install_host(host.clone());
        let first = resolver(&["a", "b"]);
        let second = resolver(&["c", "d"]);
        assert_eq!(first, second);
        assert_eq!(host.calls.get(), 1, "the second call is a shape hit");
        // A different word count is a different shape.
        let _ = resolver(&["a", "b", "c"]);
        assert_eq!(host.calls.get(), 2);
        let stats = cache_stats();
        assert_eq!((stats.hits, stats.misses, stats.entries), (1, 2, 2));
        clear_host();
    }

    /// A host folding its words, counting its calls.
    struct EchoHost {
        calls: Cell<u32>,
    }

    impl PackHookHost for EchoHost {
        fn invoke(&self, _slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
            self.calls.set(self.calls.get() + 1);
            HookAnswer::Fold(
                call.words
                    .iter()
                    .map(|word| word.value)
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        }
    }

    /// A content-keyed answer is proven on every hit, never found by its
    /// hash alone: two calls whose content hashes collide are two misses,
    /// each answered for its own words, and the same content again is a
    /// hit.
    #[test]
    fn a_hash_collision_is_not_a_hit() {
        let slot = allocate(HookFamily::ConstFold, &HookInputs::parse(&["words"])).unwrap();
        assert_eq!(cache_mode(slot), CacheMode::Content);
        let fold = const_fold_fn(slot).unwrap();
        let host = Rc::new(EchoHost {
            calls: Cell::new(0),
        });
        install_host(host.clone());
        FORCED_CONTENT_HASH.with(|forced| forced.set(Some(7)));
        assert_eq!(fold(&["abc"]), Some("abc".to_owned()));
        assert_eq!(
            fold(&["xyz"]),
            Some("xyz".to_owned()),
            "one bucket, another content: not a hit"
        );
        assert_eq!(host.calls.get(), 2);
        assert_eq!(fold(&["xyz"]), Some("xyz".to_owned()));
        assert_eq!(host.calls.get(), 2, "the same content is a hit");
        FORCED_CONTENT_HASH.with(|forced| forced.set(None));
        clear_host();
    }

    /// The evaluator generation names the host and its health: installing
    /// one moves the worker off `NO_HOST`, a quarantine gives it a
    /// generation no other state has and moves the process's epoch, and
    /// clearing the host returns it to `NO_HOST`. Two workers serving one
    /// published plan share a generation, so their memoised answers are
    /// shared; the worker whose host then quarantines a hook shares it with
    /// no one.
    #[test]
    fn host_install_and_quarantine_bump_the_generation() {
        // A plan no other test in this binary publishes.
        const PLAN: u64 = u64::MAX - 7;
        clear_host();
        assert_eq!(evaluator_generation(), EvaluatorGeneration::NO_HOST);
        install_host(Rc::new(FixedHost(HookAnswer::Abstain)));
        let installed = evaluator_generation();
        assert_ne!(installed, EvaluatorGeneration::NO_HOST);
        install_host(Rc::new(FixedHost(HookAnswer::Abstain)));
        let reinstalled = evaluator_generation();
        assert_ne!(reinstalled, installed, "a host without a plan is its own");
        let epoch = evaluator_epoch();
        note_quarantine();
        assert_ne!(evaluator_generation(), reinstalled);
        assert!(
            evaluator_epoch() > epoch,
            "a quarantine moves the process's epoch"
        );
        clear_host();
        assert_eq!(evaluator_generation(), EvaluatorGeneration::NO_HOST);

        install_plan_host(Rc::new(FixedHost(HookAnswer::Abstain)), PLAN);
        let here = evaluator_generation();
        let (there, quarantined) = std::thread::spawn(|| {
            install_plan_host(Rc::new(FixedHost(HookAnswer::Abstain)), PLAN);
            let shared = evaluator_generation();
            note_quarantine();
            (shared, evaluator_generation())
        })
        .join()
        .expect("the worker ran");
        assert_eq!(there, here, "one plan, one generation");
        assert_ne!(quarantined, here, "a quarantine is its worker's own");
        assert_eq!(
            evaluator_generation(),
            here,
            "and this worker keeps its own"
        );
        clear_host();
    }

    #[test]
    fn an_uncacheable_resolver_reaches_the_host_every_time() {
        let slot = allocate(HookFamily::ArgRoleResolver, &HookInputs::parse(&["words"])).unwrap();
        let resolver = arg_role_resolver_fn(slot).unwrap();
        let host = Rc::new(CountingHost {
            calls: std::cell::Cell::new(0),
        });
        install_host(host.clone());
        let _ = resolver(&["a", "b"]);
        let _ = resolver(&["c", "d"]);
        assert_eq!(host.calls.get(), 2);
        clear_host();
    }
}
