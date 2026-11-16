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
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::sync::{LazyLock, Mutex};

use tcl_dialect::TclVersion;

use crate::arg_role::{AppendedArity, ArgRole};
use crate::clause_shape::{ClauseShapeChecker, ClauseShapeError};
use crate::hooks::{ConstFoldFn, VersionedConstFoldFn};
use crate::hover::{OptionValueHook, ScriptTiming};
use crate::invocation_words::InvocationWordKind;
#[cfg(test)]
use crate::invocation_words::{InvocationArguments, InvocationWord};
use crate::literal_validation::{LiteralArgumentValidation, LiteralArgumentValidator};
use crate::spec::{ArgRoleResolver, CommandPrefixResolver, ContextGate, ScriptTimingResolver};
use crate::state_transition::StateTransitionResolver;
#[cfg(test)]
use crate::state_transition::{
    AliasWords, CallerFrameSelection, StateTransition, TransitionSubject, VARIABLE_ALIAS_DOMAINS,
    VariableAliasDestination, VariableAliasTarget, VariableCellAliasTransition,
};
use crate::value_transfer::{ContextDependency, EvaluatorGeneration, ImplementationBudget};

/// How many hooks of one family a process may install.
///
/// The thunk tables are static arrays of distinct monomorphised functions, so
/// this is a compile-time bound, not a budget: the shipped registry sets 45
/// const-folders and 44 role resolvers across ~2,210 commands, so 64 per
/// family is generous for the pack that motivates it. Allocation past the end
/// returns `None` and the loader installs no hook — a pack that declares 65
/// folders loses the 65th's behaviour, never its facts.
pub const SLOTS_PER_FAMILY: usize = 64;

/// The hook families, which is what fixes a hook's calling convention:
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
    /// `state_transitions { resolver … }` — emits `alias LOCAL TARGET
    /// ?-level LEVEL?` and `namespace-variable NAME`, word indices each, and
    /// so states variable-cell alias facts and nothing else
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *The two hook
    /// bodies that remain*): no verb reaches the command-binding,
    /// interpreter, object-dispatch or trace families.
    StateTransitionResolver,
}

/// Every family, in declaration order — the index a slot's family contributes
/// to the per-family tables.
pub const HOOK_FAMILIES: [HookFamily; 13] = [
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
    HookFamily::StateTransitionResolver,
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
            Self::StateTransitionResolver => &["alias", "namespace-variable"],
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
            Self::StateTransitionResolver => "no transitions",
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
            Self::StateTransitionResolver => "state_transitions.resolver",
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
            Self::StateTransitionResolver => 12,
        }
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
/// One enum for every family because the *protocol* is one protocol; the
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
    /// `alias …` / `namespace-variable …` — a `state_transitions` resolver
    /// body's facts, by word index, in call order.
    Transitions(Vec<PackTransition>),
}

/// One fact a `state_transitions` resolver body stated, by the index of the
/// words it names. The thunk reads each index against the call's own words,
/// so a computed word is known for what it is: its fact abstains and widens
/// [`VARIABLE_ALIAS_DOMAINS`] rather than naming a cell. Only the variable
/// alias family has a verb, which is what keeps a pack body from stating a
/// command-binding, interpreter, object-dispatch or trace fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackTransition {
    /// `alias LOCAL TARGET ?-level LEVEL?` — the word at `local` names a
    /// current-frame variable bound to the variable the word at `target`
    /// names in the caller's frame, or in the frame the level word at
    /// `level` selects (`upvar`'s fact).
    Alias {
        /// The local variable's word.
        local: usize,
        /// The target variable's word.
        target: usize,
        /// The level word, when the call names one.
        level: Option<usize>,
    },
    /// `namespace-variable NAME` — the word at `name` names a variable of the
    /// current namespace the call binds locally under its tail (`variable`'s
    /// fact).
    NamespaceVariable {
        /// The variable's word.
        name: usize,
    },
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

    /// Dispatch every request through this receiver before consulting the shared
    /// slot cache. An owning receiver uses this to check its one-shot request;
    /// opting out of caching grants no execution or closed-consumer authority.
    fn requires_uncached_dispatch(&self) -> bool {
        false
    }

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

/// Authored source field a hook occupies, preserving its complete layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HookSourceOwner {
    /// A command-level hook field.
    Command,
    /// A selected subcommand field.
    Subcommand(String),
    /// A selected option field, independently of its enclosing subcommand.
    Option {
        /// Actual enclosing subcommand, if present.
        subcommand: Option<String>,
        /// Actual option component, never omitted from slot identity.
        option: String,
    },
}

/// Stable authored hook source tuple, not a rendered compound name or receipt.
/// Re-editing its body preserves its slot; selected declaration membership is
/// retained separately by the actual loader plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HookSourceIdentity {
    /// Original declaring pack component.
    pub pack: String,
    /// Actual provider trust/provenance channel.
    pub provenance: tcl_dialect::model::Provenance,
    /// Actual declaring source path, retaining opaque OS units.
    pub file: std::path::PathBuf,
    /// Original declaring command component.
    pub command: String,
    /// Complete actual source field owner.
    pub owner: HookSourceOwner,
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
    identity: &HookSourceIdentity,
    inputs: &HookInputs,
) -> Option<HookSlot> {
    static BY_IDENTITY: LazyLock<Mutex<HashMap<(HookFamily, HookSourceIdentity), HookSlot>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let key = (family, identity.clone());
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
    // Monotone and destructor-free: no callback service reopens at worker teardown.
    static PRIVATE_EVALUATORS_DENIED: Cell<bool> = const { Cell::new(false) };

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

macro_rules! n17_live_trace {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var("TCL_LSP_TRACE_N17_FOLD").as_deref() == Ok("first-gate-v1") {
            eprintln!("N17G_LIVE {}", format_args!($($arg)*));
        }
    };
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
        static STATE_TRANSITION_THUNKS: [StateTransitionResolver; SLOTS_PER_FAMILY] =
            [$(state_transition_thunk::<$index>),*];
    };
}

with_slot_indices! { slot_tables }

mod dispatch;
mod hook_inputs;
mod host_installation;
mod native_catalogues;
mod tabulation;
mod thunks;

use dispatch::ShapeCache;
#[cfg(test)]
use dispatch::{CacheEntry, CallContent, shape_key};
pub use dispatch::{CacheStats, cache_stats, clear_cache, dispatch};
pub use hook_inputs::{HookInput, HookInputs, kind_word};
#[cfg(test)]
use host_installation::ensure_host_with_installer;
#[cfg(test)]
use host_installation::external_evaluators_blocked;
pub use host_installation::{
    PendingHost, advance_evaluator_epoch, clear_current_host_if, clear_host, current_host_is,
    deny_external_evaluators_on_private_thread, evaluator_epoch, evaluator_generation, has_host,
    install_host, install_plan_host, note_quarantine, private_thread_evaluators_are_closed,
    record_commands_spent, set_installer, set_pending_host, slot_available, take_commands_spent,
};
pub use native_catalogues::{
    ARG_ROLE_RESOLVER_NATIVE, CLAUSE_SHAPE_CHECK_NATIVE, COMMAND_PREFIX_RESOLVER_NATIVE,
    CONST_FOLD_NATIVE, CONST_FOLD_VERSIONED_NATIVE, CONSTRAINTS_NATIVE, CONTEXT_GATE_NATIVE,
    EVALUATE_NATIVE, FACTS_NATIVE, LITERAL_ARGUMENT_VALIDATOR_NATIVE, OPTION_ARITY_NATIVE,
    SCRIPT_TIMING_RESOLVER_NATIVE, SEMANTICS_NATIVE, STATE_TRANSITION_RESOLVER_NATIVE,
    TAINT_SINK_GATE_NATIVE, TaintSinkGateFn,
};
pub use tabulation::{
    TabulationPlan, activate_tabulation_plan, begin_tabulation_plan, quarantine_tabulations,
    tabulate,
};
use thunks::{
    arg_role_thunk, clause_shape_thunk, command_prefix_thunk, const_fold_thunk,
    const_fold_versioned_thunk, constraints_thunk, context_gate_thunk, literal_validator_thunk,
    option_arity_thunk, script_timing_thunk, state_transition_thunk, taint_sink_thunk,
};
#[cfg(test)]
use thunks::{call_of, literal_words, pack_transitions};

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

/// The `state_transitions` resolver function pointer for `slot`.
#[must_use]
pub fn state_transition_resolver_fn(slot: HookSlot) -> Option<StateTransitionResolver> {
    thunk_index(slot, HookFamily::StateTransitionResolver)
        .map(|index| STATE_TRANSITION_THUNKS[index])
}

#[cfg(test)]
mod tests;
