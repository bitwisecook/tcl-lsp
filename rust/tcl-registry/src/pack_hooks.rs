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

//! Pack-declared hooks: the registry side of the SpecTcl hook host.
//!
//! A shipped spec's `const_fold` / `arg_role_resolver` / … is a Rust function
//! pointer.  A **pack**-declared one is a Tcl body running on a sandboxed VM,
//! which the registry must not know about — the hook host owns that
//! (`docs/design/spec-packs.md`, "Two layers, deliberately").  This module is
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
//!   host.
//!
//! Nothing here evaluates anything. If no host is installed — the default in
//! every process that has not loaded a pack — every thunk answers its family's
//! documented silence, and the cost is one relaxed atomic load.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{LazyLock, Mutex};

use tcl_dialect::TclVersion;

use crate::arg_role::{AppendedArity, ArgRole};
use crate::clause_shape::{ClauseShapeChecker, ClauseShapeError};
use crate::hooks::{ConstFoldFn, VersionedConstFoldFn};
use crate::hover::{OptionValueHook, OptionValueOutcome};
use crate::invocation_words::{
    CommandPrefixArguments, InvocationArguments, InvocationWord, InvocationWordKind,
};
use crate::literal_validation::{LiteralArgumentValidation, LiteralArgumentValidator};
use crate::spec::{ArgRoleResolver, CommandPrefixResolver, ContextGate};

/// How many hooks of one family a process may install.
///
/// The thunk tables are static arrays of distinct monomorphised functions, so
/// this is a compile-time bound, not a budget: the shipped registry sets 45
/// const-folders and 44 role resolvers across ~2,210 commands, so 64 per
/// family is generous for the pack that motivates it. Allocation past the end
/// returns `None` and the loader installs no hook — a pack that declares 65
/// folders loses the 65th's behaviour, never its facts.
pub const SLOTS_PER_FAMILY: usize = 64;

/// The nine hook families, which is what fixes a hook's calling convention:
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
}

/// Every family, in declaration order — the index a slot's family contributes
/// to the per-family tables.
pub const HOOK_FAMILIES: [HookFamily; 9] = [
    HookFamily::ArgRoleResolver,
    HookFamily::CommandPrefixResolver,
    HookFamily::ConstFold,
    HookFamily::ConstFoldVersioned,
    HookFamily::TaintSinkGate,
    HookFamily::ContextGate,
    HookFamily::LiteralArgumentValidator,
    HookFamily::ClauseShapeCheck,
    HookFamily::OptionArity,
];

impl HookFamily {
    /// The emitter verbs this family injects into the sandbox.
    #[must_use]
    pub fn verbs(self) -> &'static [&'static str] {
        match self {
            Self::ArgRoleResolver => &["role"],
            Self::CommandPrefixResolver => &["prefix"],
            Self::ConstFold | Self::ConstFoldVersioned => &["fold"],
            Self::TaintSinkGate => &["sink-applies", "sink-suppressed"],
            Self::ContextGate => &["reject"],
            Self::LiteralArgumentValidator => &["invalid", "abstain"],
            Self::ClauseShapeCheck => &["missing-expr", "missing-body", "extra-words"],
            Self::OptionArity => &["consume"],
        }
    }

    /// What calling no verb at all means — the per-field conservative answer.
    #[must_use]
    pub fn silence(self) -> &'static str {
        match self {
            Self::ArgRoleResolver => "no roles (fall back to arg_roles)",
            Self::CommandPrefixResolver => "no prefix positions",
            Self::ConstFold | Self::ConstFoldVersioned => "no fold",
            // The one family whose silence is not "no opinion": a security
            // finding must survive a hook that says nothing.
            Self::TaintSinkGate => "the sink applies",
            Self::ContextGate => "the call is allowed",
            Self::LiteralArgumentValidator => "valid",
            Self::ClauseShapeCheck => "the shape is accepted",
            Self::OptionArity => "consume one word",
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
            Self::ConstFold | Self::ConstFoldVersioned | Self::OptionArity
        )
    }

    /// The DSL property word this family fills.
    #[must_use]
    pub fn field(self) -> &'static str {
        match self {
            Self::ArgRoleResolver => "arg_role_resolver",
            Self::CommandPrefixResolver => "command_prefix_resolver",
            Self::ConstFold => "const_fold",
            Self::ConstFoldVersioned => "const_fold_versioned",
            Self::TaintSinkGate => "taint_sink_gate",
            Self::ContextGate => "context_gate",
            Self::LiteralArgumentValidator => "literal_argument_validator",
            Self::ClauseShapeCheck => "clause_shape_check",
            Self::OptionArity => "options.arity_hook",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::ArgRoleResolver => 0,
            Self::CommandPrefixResolver => 1,
            Self::ConstFold => 2,
            Self::ConstFoldVersioned => 3,
            Self::TaintSinkGate => 4,
            Self::ContextGate => 5,
            Self::LiteralArgumentValidator => 6,
            Self::ClauseShapeCheck => 7,
            Self::OptionArity => 8,
        }
    }
}

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
                _ => return Self::unrestricted(),
            };
            inputs.push(input);
        }
        Self::declared(inputs)
    }

    /// **The cacheability rule.** A hook is shape-cacheable when it declared
    /// its inputs and none of them is the words' content: everything else it
    /// may read is either fixed for the slot (`command`, `subcommand`) or part
    /// of the shape key (`nwords`, `kinds`, `tcl-version`, `in-event-body`).
    ///
    /// `dialect` and `option` are *not* in the key, so declaring either is
    /// uncacheable today; widening the key is the change to make if a pack
    /// needs it.
    #[must_use]
    pub fn shape_only(&self) -> bool {
        self.declared.as_ref().is_some_and(|inputs| {
            inputs.iter().all(|input| {
                matches!(
                    input,
                    HookInput::Nwords
                        | HookInput::Kinds
                        | HookInput::Command
                        | HookInput::Subcommand
                        | HookInput::TclVersion
                        | HookInput::InEventBody
                )
            })
        })
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
}

impl<'w> HookCall<'w> {
    /// Whether every word is literal — the precondition the fold families and
    /// the option-arity hook run under.
    #[must_use]
    pub fn all_literal(&self) -> bool {
        self.words
            .iter()
            .all(|word| word.kind == InvocationWordKind::Literal)
    }
}

/// What a hook invocation produced: the emitter verb it called, or an
/// abstention.
///
/// One enum for all nine families because the *protocol* is one protocol; the
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
    /// `consume N ?-invalid MESSAGE?`.
    Consume {
        /// Words consumed, valid or not. `0` is a report, not an abstention.
        words: usize,
        /// The W141 message, when the hook rejected the value.
        invalid: Option<String>,
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
}

/// Per-family allocation counters. Process-global because a slot is baked
/// into a leaked `&'static CommandSpec` that every thread shares.
static NEXT_SLOT: [AtomicU32; 9] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

/// Per-slot cacheability, decided by the declared inputs at allocation.
static CACHEABLE: [[AtomicBool; SLOTS_PER_FAMILY]; 9] =
    [const { [const { AtomicBool::new(false) }; SLOTS_PER_FAMILY] }; 9];

/// Allocate a slot for a hook of `family` whose body declared `inputs`.
///
/// `None` when the family's slots are exhausted; the caller installs no hook
/// and the command keeps its declarative facts.
pub fn allocate(family: HookFamily, inputs: &HookInputs) -> Option<HookSlot> {
    let index = NEXT_SLOT[family.index()].fetch_add(1, Ordering::Relaxed);
    let index = usize::try_from(index).ok()?;
    if index >= SLOTS_PER_FAMILY {
        return None;
    }
    CACHEABLE[family.index()][index].store(inputs.shape_only(), Ordering::Relaxed);
    Some(HookSlot {
        family,
        index: u16::try_from(index).ok()?,
    })
}

/// Whether this slot's declared inputs make it shape-cacheable.
#[must_use]
pub fn is_cacheable(slot: HookSlot) -> bool {
    CACHEABLE[slot.family.index()][usize::from(slot.index)].load(Ordering::Relaxed)
}

thread_local! {
    /// The host serving this thread's pack hooks. Thread-local because a host
    /// owns per-pack VM instances, which are not `Send`.
    static HOST: RefCell<Option<Rc<dyn PackHookHost>>> = const { RefCell::new(None) };

    /// The shape-keyed answer cache, and its counters.
    static SHAPE_CACHE: RefCell<ShapeCache> = RefCell::new(ShapeCache::default());
}

/// `true` while any thread has a host installed — the one check an
/// unaccelerated process pays.
static ANY_HOST: AtomicBool = AtomicBool::new(false);

/// Install `host` as this thread's pack-hook host, returning the previous one.
///
/// Every pack-declared hook on this thread abstains until this is called, so
/// a worker thread that never installs a host behaves exactly like a build
/// with no packs loaded.
pub fn install_host(host: Rc<dyn PackHookHost>) -> Option<Rc<dyn PackHookHost>> {
    ANY_HOST.store(true, Ordering::Relaxed);
    clear_cache();
    HOST.with(|slot| slot.borrow_mut().replace(host))
}

/// Remove this thread's host; every pack hook abstains again.
pub fn clear_host() -> Option<Rc<dyn PackHookHost>> {
    clear_cache();
    HOST.with(|slot| slot.borrow_mut().take())
}

/// Whether this thread has a host installed.
#[must_use]
pub fn has_host() -> bool {
    ANY_HOST.load(Ordering::Relaxed) && HOST.with(|slot| slot.borrow().is_some())
}

/// The shape key: everything a shape-cacheable hook may read, packed.
///
/// `kinds` is two bits per word, so a call with more than 64 words has no
/// shape key and is answered by the host every time (uncached, still
/// correct).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ShapeKey {
    slot: HookSlot,
    nwords: u16,
    kinds: u128,
    /// `tcl-version` as a stable discriminant — `TclVersion` is `Ord` but not
    /// `Hash`, and only its identity matters here.
    version: Option<&'static str>,
    in_event_body: bool,
}

#[derive(Default)]
struct ShapeCache {
    entries: HashMap<ShapeKey, HookAnswer>,
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

fn shape_key(slot: HookSlot, call: &HookCall<'_>) -> Option<ShapeKey> {
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
        in_event_body: call.in_event_body,
    })
}

/// Invoke a pack hook: the cache first when the slot's declared inputs allow
/// it, then this thread's host, then the family's silence.
#[must_use]
pub fn dispatch(slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
    if !ANY_HOST.load(Ordering::Relaxed) {
        return HookAnswer::Abstain;
    }
    let key = is_cacheable(slot).then(|| shape_key(slot, call)).flatten();
    if let Some(key) = key
        && let Some(hit) = SHAPE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let hit = cache.entries.get(&key).cloned();
            if hit.is_some() {
                cache.hits += 1;
            }
            hit
        })
    {
        return hit;
    }
    let Some(host) = HOST.with(|slot| slot.borrow().clone()) else {
        return HookAnswer::Abstain;
    };
    let answer = host.invoke(slot, call);
    if let Some(key) = key {
        SHAPE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.misses += 1;
            cache.entries.insert(key, answer.clone());
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
fn literal_words<'w>(args: &'w [&'w str]) -> Vec<HookWord<'w>> {
    args.iter()
        .map(|value| HookWord {
            value,
            kind: InvocationWordKind::Literal,
        })
        .collect()
}

fn structured_words<'w>(args: InvocationArguments<'w>) -> Vec<HookWord<'w>> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
