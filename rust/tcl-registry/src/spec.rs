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

//! Command and subcommand specifications.
//!
//! `CommandSpec` is the single source of truth for everything the
//! compiler, analyser, formatter, LSP, and codegen need to know about
//! a Tcl command. One file per command, one `CommandSpec` per file.

use crate::abbrev::PrefixMatching;
use crate::arg_role::ArgRole;
use crate::arity::{Arity, ArityWindow};
use crate::body_kind::{BodyInterpreter, BodyKind};
use crate::clause_shape::{ClauseShapeChecker, ClauseShapeError};
use crate::command_table::CommandTableEffect;
use crate::dispatch_stability::DispatchDependencyDescriptor;
use crate::events::{EventRequirementForm, ResolvedEventRequirements};
use crate::forms::CommandForm;
use crate::frame_effect::FrameEffectSpec;
use crate::hooks::ReturnTypeHookId;
use crate::hooks::{
    AnalyserHookId, ArgTypeHint, CodegenHookId, ConstFoldFn, InlineCodegenHookId, LoweringHookId,
    TclVersion, VersionedConstFoldFn,
};
use crate::hover::{
    ArgValue, CallbackTaintInput, FormSpec, HoverSnippet, OptionSpec, ScriptTiming,
};
use crate::invocation_words::{CommandPrefixArguments, InvocationArguments};
use crate::lifecycle::{Lifecycle, LifecycleState};
use crate::literal_validation::LiteralArgumentValidator;
use crate::patterns::{FormatType, PatternType};
use crate::presentation::ArgPresentation;
use crate::repeated::RepeatedArgLayout;
use crate::representation::RepresentationEffect;
use crate::side_effects::{SideEffect, StorageType};
use crate::state_transition::StateTransitionDescriptor;
use crate::symbol_def::SymbolDef;
use crate::taint::{SetterConstraint, TaintColour, TaintNumericCoercion, TaintTransformCondition};
use crate::traits::Traits;
use crate::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
use crate::value_transfer::SemanticsDeclaration;
use crate::world_effect::WorldEffectDescriptor;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::surface_admits;

/// Dynamic argument role resolver.
///
/// Called for variable-layout commands (`if`, `try`, `switch`, `foreach`)
/// where argument roles depend on the actual argument values (e.g. the
/// position of `elseif`/`else` keywords). Returns a list of
/// `(arg_index, role)` pairs.
pub type ArgRoleResolver = fn(args: &[&str]) -> Vec<(u8, ArgRole)>;

/// Facts supplied to an argument-role resolver. Cardinality carries no values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgRoleResolverInput {
    /// Every operand has its actual frozen literal value.
    LiteralValues,
    /// Only the exact evaluated argument count is required.
    Cardinality,
    /// Structured operands and their selected available option grammar.
    StructuredLayout,
    /// Contradictory authored resolver declarations; no input is admitted.
    ConflictingResolvers,
}

/// Role resolver that depends only on exact evaluated argument cardinality.
pub type ArgRoleCountResolver = fn(argument_count: usize) -> Vec<(u8, ArgRole)>;

/// Role resolver over actual structured operands; unknown values stay unknown.
pub type ArgRoleLayoutResolver = for<'w, 'r, 'q> fn(
    crate::InvocationArguments<'w>,
    crate::resolved_invocation::InvocationOptions<'r, 'q>,
) -> Option<Vec<(u8, ArgRole)>>;

/// Resolver for variable-layout [`ArgRole::CommandPrefix`] positions and their
/// appended arities (`trace add …`, `interp alias`, `selection handle`) where
/// the prefix index depends on the actual arguments. Returns
/// `(arg_index, appended_arity)` pairs.  Paired with the static
/// [`CommandSpec::command_prefixes`] table — either may be set.
pub type CommandPrefixResolver =
    for<'a> fn(CommandPrefixArguments<'a>) -> Vec<(u8, crate::arg_role::AppendedArity)>;

/// Resolver for invocation-sensitive executable-argument timing.
///
/// Receives actual structured argument knowledge; a selector-dependent
/// descriptor must abstain on an unknown selector, while a payload may stay
/// unknown. Expansion requires independently proved argv cardinality.
/// Returns `(arg_index, timing)` pairs in the same coordinates as
/// [`ArgRoleResolver`]. The registry only accepts an answer for an argument
/// currently classified as [`ArgRole::Body`], [`ArgRole::LambdaLiteral`], or
/// [`ArgRole::CommandPrefix`], so a resolver cannot turn ordinary data into
/// executable code by itself.
pub type ScriptTimingResolver =
    for<'w> fn(crate::InvocationArguments<'w>) -> Vec<(u8, ScriptTiming)>;

/// A call-site restriction that depends on *where* the call sits — a
/// lexical/dispatch context arity and dialect gating can't see — rather
/// than on its own argument shape (iRules `return`: bare-only directly
/// inside a `when EVENT { … }` body, full syntax inside any `proc`).
/// `Some(f)`: the analyser calls `f(args, in_event_body)`; a `Some(msg)`
/// result flags `msg` at the call site. `None` = no context-sensitive
/// restriction.
///
/// Deliberately takes only the one fact `return` needs today
/// (`in_event_body`) rather than a general context bag — extend the
/// signature if a second context-sensitive command needs a different
/// fact. Takes plain data rather than any analyser-internal type since
/// this crate has no dependency on `tcl-compiler` and can't reference
/// `Analyser`/`scope_path` directly.
pub type ContextGate =
    fn(args: crate::InvocationArguments<'_>, in_event_body: bool) -> Option<&'static str>;

/// The value shape a *non-subcommand* first word may take for a command
/// whose first word usually dispatches to a subcommand.
///
/// `after` is the canonical case: `after cancel|idle|info …` dispatch on a
/// subcommand, but `after 200 …` selects the default delayed-execution form
/// — an integer first word is not an unknown subcommand. Carrying the shape
/// here keeps that knowledge out of the analyser: the unknown-subcommand
/// check (W001) asks the resolved signature whether the word matches the
/// declared default-form shape instead of naming any command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultFormFirstWord {
    /// An integer first word (any Tcl integer spelling — decimal, `0x…`,
    /// `0o…`, `0b…`, optional sign, `_` separators) selects the default
    /// form.
    Integer,
    /// A first word beginning with `.`, a window path, selects the default
    /// form: `pack .b` and `place .b -x 0` configure `.b` (Tk 8.4-9.1
    /// `tkPack.c`, `tkPlace.c`).
    WindowPath,
    /// A first word beginning with `.`, or with the `x` or `^` relative
    /// placement character, selects the default form: `grid .b` and
    /// `grid x .b` configure their content (Tk 8.4-9.1 `tkGrid.c`).
    GridContent,
}

impl DefaultFormFirstWord {
    /// Match the default form under the retained invocation numeric grammar.
    /// An absent grammar preserves the release-consensus compatibility query.
    #[must_use]
    pub fn matches_syntax(self, word: &str, syntax: Option<tcl_dialect::NumberSyntax>) -> bool {
        let Some(syntax) = syntax else {
            return self.matches(word);
        };
        match self {
            Self::WindowPath | Self::GridContent => self.matches(word),
            Self::Integer => matches!(
                tcl_syntax::number::parse_whole_with(
                    word,
                    tcl_syntax::number::ParseFlags {
                        integer_only: true,
                        ..tcl_syntax::number::ParseFlags::for_syntax(syntax)
                    }
                ),
                Some(tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. })
            ),
        }
    }
    /// Whether `word` matches this default-form shape.
    ///
    /// [`Self::Integer`] accepts exactly what `Tcl_GetIntFromObj` accepts
    /// syntactically, via the canonical [`tcl_syntax::number`] parser
    /// (`TclParseNumber` port) in integer-only, whole-string mode.
    #[must_use]
    pub fn matches(self, word: &str) -> bool {
        match self {
            // No release in hand here, and this decides which *argument form* a
            // command took — so require every release to agree that the word is
            // an integer. A spelling only some releases read as one (`08`,
            // `1_0`, `0d1`) leaves the form ambiguous rather than committing to
            // one release's reading.
            Self::Integer => {
                tcl_syntax::number::Numbers::Unknown
                    .parse_wide(word)
                    .is_some()
                    || tcl_syntax::number::NumberSyntax::every(|n| {
                        matches!(
                            tcl_syntax::number::parse_whole_with(
                                word,
                                tcl_syntax::number::ParseFlags::for_syntax(n),
                            ),
                            Some(tcl_syntax::number::Number::Big { .. })
                        )
                    })
            }
            Self::WindowPath => word.starts_with('.'),
            Self::GridContent => word.starts_with(['.', 'x', '^']),
        }
    }
}

/// Layout of a `<proto>::payload` byte-array command for the S110
/// byte-array-corruption check.
///
/// The getter form (`TCP::payload`, no args) returns raw on-the-wire bytes —
/// a binary source; the `replace` form rewrites them — a byte sink. The
/// `replace` argument layout differs per protocol, so the index of the
/// `<data>` operand is carried here instead of being hardcoded in the
/// analyser.
///
/// - `replace_data_index` — 0-based index, *within the args after the command
///   name*, of the `<data>` operand in the `replace` form. `3` for the common
///   `replace <offset> <length> <data>` layout (TCP/HTTP/…); `1` for
///   `replace <data> …` (MQTT/DIAMETER).
/// - `message_flag_shift` — when `true`, an optional leading `-message <value>`
///   flag (GTP) shifts every positional `replace` operand by two, so the data
///   index becomes `replace_data_index + 2` when the flag is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BytePayloadSpec {
    /// 0-based index of the `<data>` operand in the `replace` form.
    pub replace_data_index: u8,
    /// Whether a leading `-message <value>` flag shifts the operands by two.
    pub message_flag_shift: bool,
}

impl BytePayloadSpec {
    /// The common layout — `replace <offset> <length> <data>` (data at 3),
    /// no `-message` flag.
    pub const DEFAULT: Self = Self {
        replace_data_index: 3,
        message_flag_shift: false,
    };
}

impl Default for BytePayloadSpec {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Option-flag spellings whose value is a credential on *any* command
/// (`open $url -password hunter2`), lower-case for case-insensitive
/// matching.  The generic vocabulary half of the credential-exposure check
/// (W310): unlike [`CommandSpec::credential_options`] — which names the
/// per-command flags whose spelling alone is not secret-suggestive
/// (`http::geturl`'s `-headers`) — these names identify themselves, so they
/// are matched command-independently and the per-command field only adds to
/// them.
pub const DEFAULT_CREDENTIAL_OPTION_NAMES: &[&str] =
    &["-password", "-pass", "-secret", "-token", "-apikey"];

thread_local! {
    static RELATION_STATS: std::cell::Cell<RelationCheckStats> =
        const { std::cell::Cell::new(RelationCheckStats {
            option_bearing_sites: 0,
            judged_sites: 0,
            relations_evaluated: 0,
            hook_entries: 0,
        }) };
}

mod arguments;
mod case_list;
mod defaults;
mod lowering_selection;
mod object_class;
mod option_relations;
mod options;
mod subcommand;
mod subcommand_resolution;

pub use arguments::VersionedArgValue;
#[cfg(test)]
use arguments::optional_trailing_placeholders;
pub use case_list::{
    CaseForceListShape, CaseInvocation, CaseListSpec, CaseMatchMode, InlineCaseClause, PatternWords,
};
pub use object_class::{ObjectClassSpec, OoContextFact};
pub use option_relations::{
    ConstraintReport, ConstraintSlot, ConstraintsHook, OptionFacts, OptionPlacement,
    OptionRelation, OptionTerm, RelationCheckStats, note_relation_check, relation_check_stats,
    reset_relation_check_stats,
};
pub use options::{
    OptionScope, leading_option_word_count, leading_option_word_count_for_arguments,
    leading_option_word_count_with, resolve_option_prefix, resolve_option_prefix_with,
};
pub(crate) use options::{
    keyword_table_for_available_options, resolve_available_option_prefix_with,
};
pub use subcommand::{SubCommand, SubSubCommand};

/// How a selected Registry command exposes source operands to teardown
/// consumers. This is an argument-layout declaration only; it proves no
/// teardown ran, succeeded, or changed host/runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeardownArgumentLayout {
    /// Every effective post-head argument is a possible teardown target.
    /// Unknown argv cardinality or source-word ownership still refuses at the
    /// selected invocation/source owner.
    AllEffectivePostHeadArguments,
}

/// Unified command metadata — the single source of truth.
///
/// Every consumer (compiler, analyser, codegen, LSP, formatter, diagram
/// extractor, taint analyser) reads this struct. No command-specific
/// knowledge is hardcoded elsewhere.
///
/// Fields use `&'static` references where possible — command specs are
/// compile-time constants that live in the binary's `.rodata` section.
/// Use `..CommandSpec::DEFAULT` to fill unset fields with sensible
/// defaults.
#[derive(Debug, Clone)]
// The remaining plain-bool fields are orthogonal config flags on a
// compile-time metadata record (the behavioural set already lives in the
// `traits` bitflags); enum-folding them would churn every command-spec literal.
#[allow(clippy::struct_excessive_bools)]
pub struct CommandSpec {
    /// Command name (e.g. `"for"`, `"dict"`, `"HTTP::header"`).
    pub name: &'static str,

    /// Immutable stock helper retained by source-pack provisioning policy.
    /// Custom source descriptors cannot author this host capability.
    pub pack_source_sandbox_helper: bool,

    /// Behavioural trait flags (replaces ~35 individual boolean fields).
    pub traits: Traits,

    /// Dialects this command is available in. `None` = all dialects.
    pub surface: Option<&'static [SpecSurface]>,

    /// Argument count constraint.
    ///
    /// The shape when the signature never changed, and the fallback whenever
    /// [`Self::arity_windows`] selects nothing (no floor resolved, or no window
    /// covers it).
    pub arity: Arity,

    /// Per-release signature shapes, for a command whose arity changed across
    /// its owning package's releases.
    ///
    /// Empty for almost every command — a signature that never changed needs no
    /// windows, and [`Self::arity`] alone describes it. When non-empty, the
    /// window covering the resolved package floor wins and `arity` is the
    /// fallback; see [`ArityWindow::select`]. Windows must not overlap, which
    /// the loader notices for packs and `registry_sweep` rejects outright for
    /// shipped specs.
    pub arity_windows: &'static [ArityWindow],

    /// Static argument roles (for fixed-layout commands like `for`).
    /// Each tuple is `(arg_index, role)`.
    pub arg_roles: &'static [(u8, ArgRole)],

    /// Dynamic argument role resolver (for variable-layout commands).
    pub arg_role_resolver: Option<ArgRoleResolver>,

    /// Cardinality-only role resolver, selected before the value resolver.
    /// Unknown expansion counts retain incomplete roles; no placeholder values
    /// are supplied to either resolver.
    pub arg_role_count_resolver: Option<ArgRoleCountResolver>,
    /// Structured native layout resolver; exclusive with value/count resolvers.
    pub arg_role_layout_resolver: Option<ArgRoleLayoutResolver>,

    /// Roles the dynamic resolver can emit for some invocation.
    ///
    /// Source-aware projections consult this closed capability set when
    /// expansion or substitution prevents calling the value-dependent
    /// resolver precisely. A command whose clause grammar took its
    /// resolver's place keeps the set as the closed set of roles the
    /// grammar's walk emits.
    pub arg_role_resolver_roles: &'static [ArgRole],

    /// The word grammar of a clause chain — `if` / `elseif` / `else`, `try` /
    /// `on` / `trap` / `finally`, `for`, `while`, `foreach`, `lmap`, `catch`.
    /// See [`crate::clause_grammar`].
    ///
    /// First in the argument-role resolution order (`clause_grammar` →
    /// `arg_role_resolver` → `arg_roles` → `assigns_variable_at`): the walk
    /// answers where the clause structure's keywords, conditions and scripts
    /// sit, and the command's own tables answer the rest. The walk also
    /// derives the chain's [`crate::ClauseShapeError`], which is why
    /// [`Self::clause_shape_check`] is only the escape hatch for a chain no
    /// grammar can spell.
    pub clause_grammar: Option<&'static crate::clause_grammar::ClauseGrammarSpec>,

    /// Formatter **presentation** overrides, keyed by 0-based argument index
    /// — how an argument should be *laid out*, as distinct from what
    /// [`Self::arg_roles`] says it *is*.
    ///
    /// Only overrides are declared: every [`ArgRole::Body`] argument is an
    /// [`ArgPresentation::BlockScript`] by default, so a spec with nothing
    /// unusual to say leaves this empty. `for` declares its `start` (0) and
    /// `next` (2) scripts [`ArgPresentation::InlineScript`], which is what
    /// keeps `for {set i 0} {$i < 3} {incr i} { … }` on one header line
    /// without the formatter having to know the command's name.
    ///
    /// See [`crate::presentation`] for why this is a separate fact rather
    /// than a different [`ArgRole`].
    pub arg_presentation: &'static [(u8, ArgPresentation)],

    /// Repeated argument-role layouts — a role that recurs at a fixed
    /// stride over the argument tail (`global a b c`, `variable n v n v`,
    /// `foreach v1 l1 v2 l2 body`, `upvar ?level? o l o l`).
    ///
    /// Folded into [`crate::CommandRegistry::arg_indices_for_role`] alongside
    /// [`Self::arg_roles`] and [`Self::arg_role_resolver`], so a consumer
    /// asking which arguments carry a role gets the unbounded tail without
    /// knowing the command. See [`crate::repeated`].
    pub repeated_args: &'static [RepeatedArgLayout],

    /// How the command crosses stack frames — which argument is the level
    /// word, which arguments name variables in the selected frame, and
    /// which carry a script that runs there.  See
    /// [`FrameEffectSpec`](crate::frame_effect::FrameEffectSpec).
    ///
    /// `None` (the default) means the command has no frame-crossing
    /// argument grammar.  Distinct from [`Traits::CREATES_SCOPE_ALIAS`] and
    /// [`Traits::EVALUATES_IN_SHIFTED_FRAME`], which say *that* a command
    /// crosses frames; this says *how*, in enough detail for a consumer to
    /// read the level and the affected names without naming the command.
    pub frame_effect: Option<FrameEffectSpec>,

    /// Clause-chain shape validator, for a command whose valid argument
    /// shapes aren't a single `min..=max` [`Arity`] range (`if`'s
    /// `elseif`/`else` chain). See [`crate::clause_shape`]. A command
    /// carrying this hook should also set
    /// [`Traits::STRUCTURALLY_CHECKED_ARITY`] so the generic arity
    /// floor/ceiling check steps aside in its favour.
    pub clause_shape_check: Option<ClauseShapeChecker>,

    /// Static [`ArgRole::CommandPrefix`] positions and their appended arities
    /// (`lsort` positional forms, `socket -server` handled via options).  Each
    /// tuple is `(arg_index, appended_arity)`; the index carries
    /// `ArgRole::CommandPrefix` *and* the arity for the callback check.
    /// `arg_indices_for_role(CommandPrefix)` reports exactly these indices
    /// (unioned with option/resolver prefixes) so highlighting stays in sync.
    pub command_prefixes: &'static [(u8, crate::arg_role::AppendedArity)],

    /// Dynamic command-prefix resolver for variable-layout callbacks
    /// (`trace add …`, `interp alias`, `selection handle`).
    pub command_prefix_resolver: Option<CommandPrefixResolver>,

    /// Dynamic phase for executable arguments whose evaluation depends on the
    /// actual invocation (`send -async`, callback registration/removal forms).
    /// Static option values carry timing directly on [`crate::hover::OptionArg`].
    pub script_timing_resolver: Option<ScriptTimingResolver>,

    /// Lookup frame for selected executable operands, independent of timing.
    /// Consumers must retain the operand producer and the actual entry table;
    /// the receiving command's head or helper namespace supplies neither.
    pub script_lookup_scope: Option<crate::ScriptLookupScope>,

    /// External callback substitutions for deferred executable arguments,
    /// keyed by their argument index. Option values carry the same fact on
    /// [`crate::hover::OptionArg`]; this table covers positional callback
    /// scripts such as `bind tag sequence script`.
    pub callback_taint_inputs: &'static [(u8, &'static [CallbackTaintInput])],

    /// Return type of the command.
    ///
    /// One fact per command — the result of the shape the command is usually
    /// called in. A command whose result *kind* moves with the call names the
    /// algorithm in [`Self::return_type_hook`], which wins over this.
    pub return_type: Option<TclType>,

    /// Names the algorithm that types a call whose result shape moves with
    /// the call, when [`Self::return_type`] alone cannot describe it.
    ///
    /// `None` for almost every command. `Some` for the few whose switches or
    /// positional count change what they hand back (`regexp -inline`,
    /// `regsub` without a `varName`); [`crate::return_type`] keeps the
    /// algorithms and [`Self::return_type_for_call`] dispatches to them.
    pub return_type_hook: Option<ReturnTypeHookId>,

    /// How the command types the variable(s) it writes as a side effect —
    /// distinct from [`Self::return_type`], which types the value `[cmd …]`
    /// yields.  A destructuring writer (`lassign`, `scan`, `regexp`, `gets`)
    /// returns one thing and writes another, so the compiler's type inference
    /// reads this instead of broadcasting the return type onto the written
    /// variables.  See [`VarWriteTyping`].  Default
    /// [`VarWriteTyping::ReturnValue`].
    pub var_write_typing: VarWriteTyping,

    /// Minimum number of post-head argv words at which this command can write
    /// a variable value, including any leading option words.
    ///
    /// `None` makes no sharper claim than the command's argument-role
    /// descriptors.  `Some(n)` is a necessary-condition proof used by
    /// source-aware effect projection: when an exact argv has fewer than `n`
    /// words, a dynamic leading option cannot manufacture a write target and
    /// the caller's whole variable frame need not be widened.  This is
    /// deliberately separate from [`Self::arity`], since value-returning and
    /// variable-writing forms of one command can have different floors.
    pub variable_write_min_args: Option<u16>,

    /// How the result value relates to container element structure
    /// (`list` builds a list of its args; `lindex` retrieves one element) —
    /// the per-element type-inference fact. See [`ReturnElements`].
    pub return_elements: Option<ReturnElements>,

    /// How the command evolves the container *elements* of the variable it
    /// writes in place (`lappend` appends value words as elements). See
    /// [`VarElementsEffect`].
    pub var_elements_effect: Option<VarElementsEffect>,

    /// Effect on Tcl's dual string/internal representation or shared-object
    /// storage. A resolved subcommand or form may override this declaration.
    pub representation_effect: Option<RepresentationEffect>,

    /// Per-argument type hints. Each tuple is `(arg_index, hint)`.
    pub arg_types: &'static [(u8, ArgTypeHint)],

    /// Subcommands (for `dict`, `string`, `info`, etc.).
    pub subcommands: &'static [SubCommand],

    /// Whether this command's keyword tables ([`Self::subcommands`] and
    /// [`Self::options`]) honour unique-prefix abbreviation.  The default,
    /// [`PrefixMatching::Enabled`], is C Tcl's `Tcl_GetIndexFromObj`
    /// behaviour (`string le` ⇒ `string length`).  Set
    /// [`PrefixMatching::Strict`] for a table whose C side opts out
    /// (`TCL_INDEX_STRICT`); the analyser applies the same override for a
    /// script-level ensemble it saw configured with `-prefixes 0`.
    pub prefix_matching: PrefixMatching,

    /// Whether unknown subcommands are accepted (for dialect packs).
    pub allow_unknown_subcommands: bool,

    /// For a subcommand-dispatching command whose first word may instead be
    /// a plain value selecting the default form (`after 200 …`), the value
    /// shape that word takes. `None` = every first word must be a known
    /// subcommand. See [`DefaultFormFirstWord`].
    pub default_form_first_word: Option<DefaultFormFirstWord>,

    /// Hover documentation.
    pub hover: Option<HoverSnippet>,

    /// Worked examples with source-aligned annotations, as a pack's
    /// structured `example { code …; carrier …; note … }` hover block states
    /// them. Built-in commands document through `hover` and need none.
    pub documentation_examples: &'static [crate::documentation::DocumentationExample],

    /// Invocation forms (for completion and arity-dependent lookup).
    pub forms: &'static [FormSpec],

    /// Structured invocation-form descriptors.
    ///
    /// Each entry carries its own arity, argument roles, options,
    /// dialect filter, and lowering / codegen hook IDs. The
    /// registry's resolved-call API (see
    /// [`crate::CommandRegistry::resolve_call`]) picks the matching
    /// form for a concrete argument list. Empty means "no
    /// form-specific routing — use the [`CommandSpec`] level
    /// arity / hooks".
    pub command_forms: &'static [CommandForm],

    /// Target-neutral semantic operation for this command.
    ///
    /// A resolved subcommand or form may override it. `None` preserves the
    /// generic [`crate::SemanticOperationId::Invoke`] fallback, unless the
    /// existing common lowering descriptor supplies a structured operation.
    pub semantic_operation: Option<crate::semantic_operation::SemanticOperationId>,

    /// Per-release semantic operations, for a command whose operation differs
    /// across Tcl releases. See [`Self::codegen_hook_windows`] for the contract
    /// every stamp window list shares.
    pub semantic_operation_windows: &'static [crate::stamp_window::StampWindow<
        crate::semantic_operation::SemanticOperationId,
    >],

    /// Target-neutral completion semantics for this command.
    ///
    /// A resolved subcommand or invocation form can supply a more-specific
    /// descriptor. `None` deliberately leaves the invocation conservative.
    pub completion: Option<crate::completion::CompletionDescriptor>,

    /// Whether equal evaluated arguments are sufficient to reproduce this
    /// command's result. `None` is conservative, not an assertion of purity.
    pub result_stability: Option<crate::result_stability::ResultStability>,

    /// Native result dependency; runtime identity and completion remain required.
    pub native_result: Option<crate::native_result::NativeResultContract>,

    /// Which argument index is a variable name assigned by the command.
    /// `None` = command does not assign a variable.
    pub assigns_variable_at: Option<u8>,

    /// Dialects where this command safely initialises an uninitialised
    /// variable. `None` = not safe. `Some(empty)` = safe in all dialects.
    pub safe_on_uninit: Option<&'static [SpecSurface]>,

    /// Compile-time constant folder.
    pub const_fold: Option<ConstFoldFn>,

    /// Tcl-version-aware constant folder, for commands whose compile-time value
    /// depends on the dialect's Tcl release (`format`, `scan`).  Takes priority
    /// over [`Self::const_fold`] when set; see [`Self::run_const_fold`].
    pub const_fold_versioned: Option<VersionedConstFoldFn>,

    /// Lowering hook ID (index into compiler's dispatch table).
    pub lowering_hook: Option<LoweringHookId>,

    /// Typed BPF-Tcl lowering descriptor — `Some` on every command of the
    /// BPF dialect, describing the core operation or framework declaration
    /// the command stands for (scalar width, packet-load width, map role,
    /// verdict family + compatible program types, effect classification).
    /// The BPF-Tcl front-end (`bpf-tcl-ir`) and its capability policy
    /// dispatch on this descriptor, never on the command name — see
    /// [`crate::bpf_op`]. A shared Tcl spelling may also carry independently
    /// authored static BPF syntax; this never grants Tcl execution evidence.
    pub bpf_op: Option<&'static crate::bpf_op::BpfOpSpec>,

    /// `TclVM` bytecode codegen hook ID — picks the per-command
    /// emitter inside `tcl_codegen::codegen::emitter::bytecoded`
    /// (the path that matches C Tcl 9's bytecode output).
    /// `None` means the generic invoke emitter handles this command.
    pub codegen_hook: Option<CodegenHookId>,

    /// Per-release codegen hooks, for a command whose bytecode emitter differs
    /// across Tcl releases.
    ///
    /// Empty for every shipped command: a stamp that never varied needs no
    /// windows, and [`Self::codegen_hook`] alone answers. When non-empty, the
    /// window covering the primary release wins and the unversioned field
    /// stands where none does; a query that does not settle the release — none
    /// pinned, or the whole ladder across a window's edge — selects nothing and
    /// the call is dispatched plain, never by a guess between windows. See
    /// [`crate::stamp_window::StampSelection`], and read the answer through
    /// [`Self::codegen_hook_at`]. Windows must not overlap, which the loader
    /// notices for packs and `registry_sweep` rejects outright for shipped
    /// specs.
    pub codegen_hook_windows: &'static [crate::stamp_window::StampWindow<CodegenHookId>],

    /// Inline (value-position / catch-body) bytecode codegen hook ID —
    /// picks the per-command emitter on the compiler's
    /// command-substitution and catch-body paths
    /// (`tcl_codegen::codegen::cmd_subst` /
    /// `tcl_codegen::codegen::control_flow`). `None` means those
    /// paths use their generic invoke emission for this command.
    pub inline_codegen_hook: Option<InlineCodegenHookId>,

    /// Per-release inline codegen hooks; the contract of
    /// [`Self::codegen_hook_windows`].
    pub inline_codegen_hook_windows:
        &'static [crate::stamp_window::StampWindow<InlineCodegenHookId>],

    /// Target-neutral native lowering shape — which native code shape the
    /// executable-IR lowering (`tcl_compiler::native_lowering`) gives an
    /// invocation of this command once its registry facts are resolved. A
    /// sibling of [`Self::lowering_hook`], [`Self::codegen_hook`], and
    /// [`Self::inline_codegen_hook`]. `None` means the generic argv
    /// invocation ([`crate::native_lowering::NativeLowering::Generic`]); read
    /// it through [`Self::native_lowering`].
    pub native_lowering: Option<crate::native_lowering::NativeLowering>,

    /// Per-release native lowering shapes; the contract of
    /// [`Self::codegen_hook_windows`], read through [`Self::native_lowering_at`].
    /// A windowed shape is not a basis for a derived value-transfer
    /// specialisation, which reads the unversioned field only.
    pub native_lowering_windows:
        &'static [crate::stamp_window::StampWindow<crate::native_lowering::NativeLowering>],

    /// The value-transfer specialisation declared at command scope — what an
    /// invocation computes, which storage it writes, and the evaluator route
    /// that computes it (`docs/design/compiler/value-transfers.md`).
    /// Three states: [`SemanticsDeclaration::Inherited`] says nothing here,
    /// so a subcommand or form declaration applies or, failing one, a
    /// specialisation is derived from a descriptor stating the same
    /// operation ([`Self::native_lowering`]'s cell read-modify-write, the
    /// `DESTROYS_VARIABLE` trait); [`SemanticsDeclaration::Declared`] names a
    /// registry-owned specialisation; [`SemanticsDeclaration::Declined`]
    /// abstains explicitly, stopping inheritance and derivation. Consumers
    /// read it through the resolved invocation, never by command name.
    pub semantics: SemanticsDeclaration,

    /// Analyser handler-family hook ID — picks the per-command
    /// handler in the analyser's central dispatch
    /// (`tcl_compiler::analyser`). `None` means the analyser has no
    /// command-specific handler for this command; only the generic,
    /// registry-role-driven walks apply.
    pub analyser_hook: Option<AnalyserHookId>,

    /// How this command mutates the interpreter's *command table*
    /// (`proc` defines, `rename` moves, `interp alias` aliases — see
    /// [`CommandTableEffect`]). `None` = the command never rebinds a
    /// command name. Consumed by the command-binding lattice, the
    /// lowerer's alias table, and the analyser's rename / alias
    /// records via [`crate::CommandTableEffect::transitions`], which
    /// resolves this selector to the stock state-transition descriptor a
    /// shipped spec names directly.
    pub command_table_effect: Option<CommandTableEffect>,

    /// Structured side-effect declarations, and how completely they are
    /// known: an empty list is no longer a claim that there are none.
    pub side_effects: crate::knowledge::Knowledge<&'static [SideEffect]>,

    /// Mutable Tcl-world effects for common command analysis.
    ///
    /// A resolved invocation applies this descriptor before any matching
    /// subcommand or form descriptor.  Descriptors extend each other by
    /// default; a narrower descriptor may explicitly replace inherited facts.
    /// The result is target-neutral and is resolved, together with the
    /// established command-table, frame, and side-effect metadata, before a
    /// compiler pass consumes it.
    pub world_effects: Option<WorldEffectDescriptor>,

    /// Target-neutral transitions of command bindings and variable cells.
    ///
    /// A resolved subcommand or form may add to or replace this descriptor.
    /// The resolver receives structured invocation words, keeping dynamic
    /// operands conservatively typed rather than treating source spelling as
    /// a Tcl value.
    pub state_transitions: Option<StateTransitionDescriptor>,

    /// Mutable Tcl domains whose stability is required before a compiler may
    /// treat registry resolution as the live dispatch behaviour.
    ///
    /// An absent descriptor keeps the conservative default. Resolved
    /// subcommands and forms may extend or explicitly replace the refinable
    /// operation-specific portion without teaching a compiler the command's
    /// name. Irreducible live-interpreter dependencies always remain.
    pub dispatch_dependencies: Option<DispatchDependencyDescriptor>,

    /// Relationship/content validator for statically-known literal arguments.
    /// A resolved subcommand or form may override this callback.
    pub literal_argument_validator: Option<LiteralArgumentValidator>,

    /// Conditional index/container relationship for original source advice.
    /// This descriptor grants no native handler, current value or result.
    pub source_index_bounds: Option<crate::source_index_bounds::SourceIndexBounds>,

    /// Inferred storage type for the target variable (`Dict`, `List`, `Array`).
    pub inferred_storage_type: Option<StorageType>,

    /// Package requirement (command only visible when package is `require`d).
    pub required_package: Option<&'static str>,

    /// Tk geometry-manager semantics. `None` means this command does not have
    /// a registry-declared direct geometry-placement form.
    pub tk_geometry: Option<crate::tk_geometry::TkGeometryManagerSpec>,

    /// Excluded iRules events.
    pub excluded_events: &'static [&'static str],

    /// Command is unsafe in sandboxed dialects — it allows context
    /// escalation (e.g. `uplevel`, `history`).  Drives the IRULE2003
    /// "unsafe iRules command" check, read by `CommandRegistry::is_unsafe`.
    pub unsafe_command: bool,

    /// 0-based argument indices whose [`Self::arg_values`] are an
    /// **exhaustive** legal set (not mere completion hints).  A literal
    /// at one of these indices that is not among `arg_values` is invalid
    /// (W127).  Flattened to the command level alongside `arg_values`.
    pub closed_value_args: &'static [u8],

    /// Whether literals in [`Self::closed_value_args`] use the same unique-
    /// prefix keyword policy as [`SubCommand::arg_values_accept_prefix`].
    /// Default `false` keeps exact matching. This is source value-vocabulary
    /// metadata, not runtime lookup, successful argument evaluation or Normal.
    pub arg_values_accept_prefix: bool,

    /// Layer-based iRules event requirements (transport / profiles /
    /// `also_in` / side / init-only / flow / capability) used by the
    /// IRULE1001 event-validity check. `None` = no requirement.
    pub event_requires: Option<crate::events::EventRequires>,

    /// Argument-prefix-specific iRules event contracts. These override
    /// [`Self::event_requires`] for a matching literal call form, allowing a
    /// registry spec to describe commands whose read and configuration forms
    /// have different legal event surfaces without a compiler-side name check.
    pub event_requirement_forms: &'static [EventRequirementForm],

    /// The iRules events this command raises, for its ordinary form.
    ///
    /// The mirror of [`Self::event_requires`]: that says where a command may
    /// be *written*, this says what running it *starts*. Consumers building
    /// event-rooted reachability follow this rather than growing a table of
    /// command names. `None` means the command raises no event.
    pub event_emits: Option<crate::events::EventEmission>,

    /// Argument-prefix-specific emissions, overriding [`Self::event_emits`]
    /// for a matching literal call form.
    ///
    /// `TCP::notify` needs all three: `request` and `response` raise a user
    /// event, `eom` only marks a message boundary.
    pub event_emission_forms: &'static [crate::events::EventEmissionForm],

    /// Data-collection lifecycle fact for an iRules protocol command.
    ///
    /// Consumers use this descriptor rather than inferring behaviour from a
    /// command spelling such as `HTTP::payload`. `None` means the command is
    /// not part of a collect/release/payload lifecycle.
    pub data_collection: Option<crate::events::DataCollectionOperation>,

    /// Conditional iRules source flow grammar, independent of handler effects.
    pub irules_source_flow: Option<crate::events::IrulesSourceFlowSpec>,

    /// Connection-side context selected for this command's body argument.
    ///
    /// `None` means the command does not switch the iRules connection side.
    /// Commands carrying this fact also declare [`Traits::IS_SIDE_SWITCH`]
    /// for trait-based consumers.
    pub side_switch_target: Option<crate::side_effects::SideSwitchTarget>,

    /// Priority policy for an iRules event-handler command.
    ///
    /// `None` means this is not an event-handler priority grammar. The
    /// registry describes both the runtime default and whether omission is a
    /// diagnostic, so consumers do not assume an explicit priority is needed.
    pub event_handler_priority: Option<crate::events::EventHandlerPriority>,

    /// Stateful iRules file-level effect owned by this declaration command.
    pub irules_top_level_effect: Option<crate::events::IrulesTopLevelEffect>,

    /// Options declared on the command (for completion and arity adjustment).
    pub options: &'static [OptionSpec],

    /// Fixed positional words before the selected invocation's option run.
    /// Counted after the command head, or after a selected subcommand word.
    pub option_prefix_words: usize,

    /// Typed relations between this command's options and arguments — the
    /// declarative half of E-R14.  Data for generic invocation validation,
    /// not a command-specific analyser rule, and checked natively with no
    /// hook and no VM entry.
    pub option_relations: &'static [OptionRelation],

    /// The option-effect families this command's options cite — each one
    /// names an axis's starting state and how two of its options combine
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *Options with
    /// semantic effects*). Read through [`Self::option_effects`]; empty for a
    /// command whose options move no declared axis.
    pub option_effect_families: &'static [crate::option_effect::OptionEffectFamily],

    /// The `constraints` escape hatch: a hook consulted **only** when
    /// [`Self::option_relations`] reported nothing, for the rare rule no
    /// declarative relation can express.  `None` for every shipped command.
    pub constraints: Option<ConstraintsHook>,

    /// Where this command's options may appear — what the relation checker
    /// needs to find them.  [`OptionPlacement::Leading`] for everything whose
    /// option parsing stops at the first non-option word.
    pub option_placement: OptionPlacement,

    /// Number of trailing words (after the command name) that C Tcl's own
    /// option-scanning loop never treats as option candidates, regardless
    /// of their syntactic shape — a structural arity fact, not a taint or
    /// dialect concern. `switch`'s C implementation
    /// (`TclNRSwitchObjCmd`/`generic/tclCmdMZ.c`) scans for `-flag` words
    /// only up to `objc - 2`, so the trailing `string` and
    /// pattern-list-or-first-pattern words are *never* mistaken for
    /// options even when they're a tainted variable or command
    /// substitution beginning with `-` — hence `switch $x $caseListVar`
    /// needs no `--` terminator. Consumed by
    /// [`crate::registry::CommandRegistry::resolve_option_terminator`]'s
    /// `reserved_trailing_words` on [`crate::registry::ResolvedTerminator`]
    /// and the source-aware pattern resolver, which both cap their option
    /// scans against it. W304 and T102 use the former path. Default `0` (no
    /// reservation) keeps every existing spec correct.
    pub reserved_trailing_words: usize,

    /// Enumerable positional-argument values, keyed by 0-based
    /// argument index *after* the command name.  Drives command-level
    /// value completion — e.g. iRules `when EVENT timing enable|disable`
    /// declares `(2, &[enable, disable])`, and `HTTP::respond <status>
    /// content|noserver|version` declares `(1, &[…])`.  Flattened from
    /// per-form values to the command level since the completion
    /// consumer keys purely on positional index.
    pub arg_values: &'static [(u8, &'static [ArgValue])],

    /// Package-version gates for individual literal values in
    /// [`Self::arg_values`] — the command-level twin of
    /// [`SubCommand::versioned_arg_values`], indexed 0-based *after the
    /// command name*. A value with no entry here is present in every
    /// package version.
    pub versioned_arg_values: &'static [VersionedArgValue],

    /// Whether `ArgRole::Body` arguments of this command run in the
    /// caller's frame ([`BodyKind::Plain`]) or in a separate
    /// definition / dispatch context ([`BodyKind::Structural`]).
    ///
    /// `Structural` opts every body arg out of the enclosing block's
    /// data flow (SSA, def-use scans, dead-store detection).  Default
    /// `Plain` keeps existing specs unchanged.
    pub body_kind: BodyKind,

    /// Execution grammar for immediate evaluated script regions.
    pub body_execution: Option<crate::body_execution::BodyExecutionSpec>,
    /// Native procedure definition argv and persistent-storage grammar.
    pub procedure_definition: Option<crate::native_procedure::NativeProcedureDefinitionSpec>,
    /// Native compiler-hook grammar and body entry, independent of lowering.
    pub native_compilation: Option<crate::native_compilation::NativeCompilationSpec>,
    /// Audited successful handler transfer, independent of compiler selection.
    pub successful_handler: Option<crate::native_compilation::SuccessfulHandlerSpec>,
    /// Selected variable receiver grammar, indexed after the selected head or
    /// subcommand. This is naming metadata only, independent of successful
    /// execution and whole-array roles. `None` inherits; an empty slice rejects
    /// a receiver grammar for this descriptor.
    pub variable_receivers:
        Option<&'static [(u8, crate::resolved_invocation::VariableReceiverOperandForm)]>,

    /// Which interpreter owns the command's evaluated body arguments.
    ///
    /// This is independent of [`Self::body_kind`], which describes frame and
    /// structural scope within an interpreter. The default is the invoking
    /// interpreter; an ensemble may override it on a body-bearing subcommand.
    pub body_interpreter: BodyInterpreter,

    /// Number of runtime-supplied positional args the body's first
    /// command receives.  Used by proc-call arity checks to relax
    /// static arity bounds on a `Body`-marked argument that is
    /// invoked as a command prefix (e.g.
    /// `fileutil::updateInPlace path cmd` appends file contents to
    /// `cmd` at runtime).
    ///
    /// Default `0` keeps every existing spec correct.
    pub body_arg_implicit_args: u8,

    // Granular taint / security metadata
    //
    // The consumer (`tcl_compiler::taint`) reads these to drive the
    // W102/W103/W300/W301/W303/W309/W310/W312 + T106 + W313 emitters.
    //
    /// Output-sink diagnostic code emitted when tainted data reaches
    /// this command's output position (e.g. `"T101"` for `puts`,
    /// `"IRULE3001"` for `HTTP::respond`). `None` = not an output sink.
    pub taint_output_sink: Option<&'static str>,

    /// When non-empty, restricts [`Self::taint_output_sink`] to apply
    /// only when the first argument (subcommand) is in this set
    /// (e.g. `HTTP::header insert|replace`). Empty = applies to every
    /// invocation.
    pub taint_output_sink_subcommands: &'static [&'static str],

    /// Log-injection sink diagnostic code (e.g. `"IRULE3003"` for the
    /// iRules `log` command). `None` = not a log sink.
    pub taint_log_sink: Option<&'static str>,

    /// Argument indices (0-based after the command name) that take a
    /// network address — SSRF sinks (`socket`, `HTTP::host`, …).
    /// `None` = not a network sink; `Some(&[])` = network sink whose
    /// dangerous-arg positions are unspecified.
    pub taint_network_sink_args: Option<&'static [u8]>,

    /// Positional argument indices (0-based after the command name, option
    /// flags skipped) that are the *actual* code-execution sink for a T100
    /// command — the only slots where a tainted value reaches `eval`-style
    /// evaluation. `apply`'s lambda `func` and `open`'s pipeline-selecting
    /// `fileName` are both `Some(&[0])`: their trailing arguments (the
    /// lambda's bound parameters, `open`'s access mode / permissions) are
    /// ordinary data, never re-evaluated as script text. `None` (the
    /// default) means the whole command is the sink — every tainted
    /// argument reaches evaluation — which is correct for the
    /// `eval`/`uplevel`/`subst`/`exec` "whole tail is one script" shape.
    /// Set this only when a specific slot, not the entire argument tail,
    /// carries the code-execution hazard. `Some(&[])` imposes no filter
    /// (behaves like `None`). Consumed by `tcl_compiler::taint`'s
    /// position-aware T100 sink filter.
    pub taint_code_sink_args: Option<&'static [u8]>,

    /// Subcommands that evaluate code in another interpreter
    /// (`interp eval`, `interp invokehidden`) — cross-interpreter
    /// code-execution sinks (T105). Empty = none.
    pub taint_interp_eval_subcommands: &'static [&'static str],

    /// Colour bits this command's *return value* carries when it acts as
    /// a taint source — the getter-form result. `Some(TAINTED)` is a
    /// plain attacker-controlled source; `Some(TAINTED | PATH_PREFIXED)`
    /// (`HTTP::path`/`HTTP::uri`), `… | IP_ADDRESS` (`IP::client_addr`),
    /// `… | PORT` (`TCP::*_port`), `… | FQDN` (`SSL::sni`) carry the
    /// option-injection-safe mitigations too. `None` = not a source.
    /// The registry collects every spec's `taint_source` into a
    /// dialect-agnostic index ([`crate::CommandRegistry::taint_source`])
    /// at build time.
    pub taint_source: Option<TaintColour>,

    /// Colour bits this command *adds* to a tainted value it returns —
    /// a sanitising transform (`uri::encode` ⇒ `URL_ENCODED`,
    /// `file join` ⇒ `PATH_JOINED`). `None` = no transform.
    pub taint_transform: Option<TaintColour>,

    /// Condition on a call's own argument words that must hold before
    /// [`Self::taint_transform`] is claimed for *that* call. `None` = the
    /// colour always applies (the common case: an encoder guarantees its
    /// colour whatever it is handed).
    ///
    /// The mirror of [`Self::taint_sink_gate`] on the transform side, for a
    /// command whose sanitising effect is a property of the *literal it was
    /// given* rather than of the command itself — `string map` with a mapping
    /// that provably deletes CR and LF proves `CRLF_FREE`, while the same
    /// command with any other mapping proves nothing.
    pub taint_transform_when: Option<TaintTransformCondition>,

    /// Colour whose presence on the *input* means this command would
    /// double-encode the value (T106). `None` = no double-encode
    /// detection.
    pub taint_double_encode_colour: Option<TaintColour>,

    /// Colour that suppresses the dangerous-sink warning (T100) for
    /// this sink — e.g. `SHELL_ATOM` for `exec`, `LIST_CANONICAL` for
    /// `eval`/`uplevel`. `None` = no suppression colour.
    pub taint_sink_safe_colour: Option<TaintColour>,

    /// Whether a call's own option flags make this command's taint-sink
    /// classification live for *this* invocation, given its raw argument
    /// words — checked before any sink code (T100 code-execution,
    /// `taint_output_sink`, `taint_log_sink`) is assigned. `None` = the
    /// sink always applies (the common case: `eval`/`uplevel`/`exec`/`expr`
    /// take no flag that changes their hazard). `Some(f)` calls `f(args)`
    /// (args excluding the command name); a `false` result suppresses sink
    /// classification entirely for that call. Exists for commands like
    /// `subst`, whose `-nocommands` flag (or, from Tcl 9.1, an
    /// `-backslashes`/`-variables` positive form with no `-commands`)
    /// disables the only hazard T100 warns about — see
    /// `tcl_registry::commands::tcl::subst_::subst_evaluates_commands`.
    pub taint_sink_gate: Option<fn(&[&str]) -> bool>,

    /// Which of a call's own argument words this command reads as numbers —
    /// a T100 numeric-coercion sink when one carries taint, as an operand of a
    /// braced `expr` is. `None` (the default) = the command coerces nothing a
    /// caller controls. See [`TaintNumericCoercion`].
    pub taint_numeric_coercion: Option<TaintNumericCoercion>,

    /// Option flags whose value carries a secret (e.g. `-password`,
    /// `-headers`) — drives credential-exposure checks. Empty = none.
    pub credential_options: &'static [&'static str],

    /// Conditional source option relationship; no channel state or completion.
    pub channel_configuration: Option<crate::ChannelConfigurationSpec>,

    /// HTTP header names whose values are secrets (e.g.
    /// `authorization`, `cookie`). Empty = none.
    pub sensitive_headers: &'static [&'static str],

    /// Setter-form argument constraints (IRULE3101). Empty = none.
    /// The registry-driven replacement for the hardcoded
    /// `SETTER_CONSTRAINTS` table in `tcl_compiler::taint`.
    pub setter_constraints: &'static [SetterConstraint],

    // Structured spec fields
    //
    /// Kind of pattern language this command's pattern argument uses
    /// (`regexp`/`regsub` ⇒ `Regex`), for semantic-token sub-tokens and
    /// pattern validation. `None` = not a pattern command.
    pub pattern_type: Option<PatternType>,

    /// Call-specific pattern positions and languages for an option-selected
    /// grammar such as `lsearch -regexp list pattern`.  Static pattern
    /// commands use [`Self::pattern_type`] and [`Self::arg_roles`] instead.
    ///
    /// This is deliberately a registry hook, never an editor-side command
    /// name match: the option grammar remains co-located with its
    /// [`OptionSpec`] descriptors.
    pub pattern_arg_resolver: Option<crate::patterns::PatternArgResolver>,

    /// Kind of format string this command's format argument uses
    /// (`format`/`scan` ⇒ `Sprintf`, …), for inlay-hint parsing and
    /// semantic-token sub-tokens. `None` = not a format command.
    pub format_string_type: Option<FormatType>,

    /// Tcllib package that provides this command, for per-document
    /// activation via `package require`. `None` = core/built-in.
    pub tcllib_package: Option<&'static str>,

    /// Introduction / deprecation / retirement releases of this command on
    /// the version axis of `required_package` / `tcllib_package` — e.g. the
    /// `ttk::*` widgets are introduced in Tk `8.5`.
    /// [`Lifecycle::UNSPECIFIED`] = present in every version of the owning
    /// package. Gated against the version resolved from `package require` via
    /// [`CommandSpec::available_for_version`] /
    /// [`CommandSpec::lifecycle_state`].
    pub lifecycle: Lifecycle,

    /// Whether W120 (missing-import) fires when this package-gated
    /// command is used without a `package require`. Default `true`; set
    /// `false` for Tk commands (`wish` auto-loads Tk).
    pub warn_missing_import: bool,

    /// Whether this command's source namespace exports it via
    /// `namespace export <bare>`, making the bare name eligible after
    /// `namespace import`.
    pub is_namespace_exported: bool,

    /// XC (cross-compile) translatability override: `None` = default
    /// rules, `Some(false)` = never translatable, `Some(true)` =
    /// translatable despite a namespace prefix.
    pub xc_translatable: Option<bool>,

    /// Replacement command name (resolved) for a deprecated command,
    /// surfaced by the deprecation code action. `None` = not deprecated.
    pub deprecated_replacement: Option<&'static str>,

    /// Whether [`Self::deprecated_replacement`] is a drop-in rename: the
    /// replacement command accepts the deprecated command's argument list
    /// unchanged, so a quick fix may mechanically swap the command head
    /// (`client_addr` → `IP::client_addr`). `false` for replacements that
    /// restructure the arguments (`ip_addr` → `IP::addr … mask …`), change
    /// the surrounding syntax (`use pool` → `pool`), or are prose
    /// (`"(removed)"`) — those keep the message-only deprecation warning.
    pub deprecated_replacement_drop_in: bool,

    /// Typed source advice for a deprecated argument shape. The selected
    /// descriptor owns the diagnostic and proposal; consumers require original
    /// written words and offer review edits without a runtime equivalence claim.
    pub source_deprecation_advice: Option<crate::deprecation::SourceDeprecationAdvice>,
    /// The shipped builtin this pack command **is** — the only admissible
    /// source of a builtin identity for a pack command, never inferred
    /// from a realm alias (`rust/tcl-compiler/src/realm.rs` learns
    /// aliases from script statements, which is a candidate, never
    /// proof, so it may seed a Spec Studio suggestion and never admit a
    /// site). `None` for every shipped command and every pack command
    /// that declares no target.
    ///
    /// The stamp rejection rule (`tcl_spectcl::stamps`) reads it: a
    /// codegen-axis stamp (`codegen_hook`, `inline_codegen_hook`,
    /// `semantic_operation Intrinsic(…)`) on a pack command survives the
    /// load only from a bundled pack, and only when this field names the
    /// shipped builtin whose spec carries that same stamp at the same site
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *The
    /// loader's stamp rejection rule*). Codegen reads it through
    /// [`crate::ResolvedCall::stamp_identity`]: a site specialised on a
    /// stamp this field's target carries records the target's identity, so
    /// the VM's alias hop from the pack name to the builtin admits it.
    pub alias_of: Option<&'static str>,

    /// How this command's behaviour arrives at run time: a shipped builtin
    /// attested by its registry identity, a Tcl body and where its text comes
    /// from, a command the host registered natively, or nothing at all
    /// (rung 4 of `docs/design/compiler/registry-consumer-contracts.md`
    /// § *Four rungs of codegen meeting `.tclspec`*). One fact per command,
    /// chosen from by code generation so that the identity a compiled
    /// artefact records is never guessed from the command's name.
    ///
    /// Declared on every core Tcl command (the rows of
    /// `docs/generated/wasm-command-backing.md`); [`RuntimeBacking::None`]
    /// for every command that declares nothing. The take-shipped floor
    /// ([`crate::security_floor`]) keeps a shipped command's backing through
    /// any override. Declared vocabulary so far: nothing yet admits a
    /// specialised site on it.
    pub runtime_backing: crate::runtime_backing::RuntimeBacking,

    /// `<proto>::payload` byte-array layout — `Some` when this command's
    /// getter returns raw bytes (a binary source) and its `replace` form is a
    /// byte sink, for the S110 byte-array-corruption check. `None` = not a
    /// byte-array payload command.
    pub byte_array_payload: Option<BytePayloadSpec>,

    /// How this whole command transforms a byte-array (binary) operand it
    /// derives its result from — drives the S110 byte-array-corruption check
    /// for commands like `format` / `join` / `concat` / `split` / `subst` /
    /// `regsub`. `string`'s per-subcommand effects live on the
    /// [`SubCommand::byte_array_effect`] instead. Default
    /// [`ByteArrayEffect::None`]. See [`crate::byte_array_effect`].
    pub byte_array_effect: crate::byte_array_effect::ByteArrayEffect,

    /// Definition-body grammar — `Some` when this command is a class/type
    /// *definer* whose `ArgRole::Body` argument is a definition script (a
    /// `TclOO` metaclass `create` body, the bare `oo::define` script form, a
    /// `snit::type` / `snit::widget` body).  The grammar describes the body's
    /// member sub-keywords (`method`, `typemethod`, `constructor`, …) so the
    /// definition-body walker (folding + semantic tokens) recurses and
    /// highlights them generically — see [`crate::definer`].  Keeping this in
    /// the registry is what lets a new definer be *data*, not new
    /// `match cmd_name` logic in the compiler / analyser / LSP.
    pub definition_body: Option<&'static crate::definer::DefinitionBodyGrammar>,

    /// Manufacturer methods exposed by this command when it is a registered
    /// class factory but not itself a definition-body command.  Package
    /// classes such as `ticklecharts::chart` use ordinary `TclOO` `new` /
    /// `create` dispatch without carrying a class-definition grammar.
    pub manufacturer_methods: &'static [crate::definer::ManufacturerMethod],

    /// The `{pattern body pattern body …}` clause list this command takes as its
    /// final braced word — `switch … { pat body … }` and Expect's
    /// `expect { pat body … }`.
    ///
    /// A clause list is *not* a script: its bodies must be recursed and its
    /// patterns classified, or the whole block collapses into one opaque
    /// literal.  Describing the shape here (rather than matching the command
    /// name in the LSP) is what lets Expect's `expect` / `expect_before` / …
    /// share the walker `switch` already uses — and is why the walker names no
    /// command.
    pub case_list: Option<&'static CaseListSpec>,

    /// Object-class metadata — `Some` when this command is a `TclOO` /
    /// megawidget class factory whose `new` / `create` returns an object handle
    /// dispatched as `$obj <method> …`.  See [`ObjectClassSpec`].
    pub object_class: Option<&'static ObjectClassSpec>,

    /// Symbol-definer descriptor — `Some` when one of this command's arguments
    /// binds a definition *name* the document outline should list (a
    /// `tcltest::test NAME …` case, …).  The consumer reads which argument
    /// holds the name and what outline category to file it under from the
    /// [`SymbolDef`], never from a command-name check — see
    /// [`crate::symbol_def`].  Distinct from [`Traits::DEFINES_PROCEDURE`] /
    /// [`Self::definition_body`], which carry the richer proc / class records;
    /// this is the lightweight "bind a navigable name" case.  `None` = the
    /// command defines no outline symbol.
    pub defines_symbol: Option<SymbolDef>,

    /// Scoped command environment — `Some` when this command's
    /// [`ArgRole::Body`] argument runs in a context that exposes a curated set
    /// of extra commands available *only* inside that body (a safe interpreter,
    /// a definition DSL).  The archetype is `report::defstyle`, whose style
    /// script exposes the report configuration methods (`top`, `data`,
    /// `columns`, …).  The compiler / LSP push this environment while walking
    /// the body and resolve command heads against it — keeping the scoped
    /// command set registry *data*, never a `match cmd_name` in a walker.  See
    /// [`crate::scoped`].
    pub body_scope: Option<&'static crate::scoped::ScopedCommandEnv>,

    /// Object-handle binding — `Some` when one of this command's arguments
    /// names a **variable that ends up holding an object handle**, with
    /// another argument saying which class (`set NAME [TYPE inst …]`).  The
    /// handle scan reads the two indices (and any required keyword) from the
    /// descriptor rather than matching the command word, so `::set` and a
    /// provable static alias/rename of it bind exactly like the bare spelling.
    /// A member-body-only installer such as snit's `install`
    /// carries the same descriptor on its
    /// [`crate::definer::MemberBodyCommand`] instead, so it stays scoped to
    /// the class system that provides it.  `None` = the command binds no
    /// handle.  See [`crate::handle_binding`].
    pub binds_handle: Option<&'static crate::handle_binding::HandleBindingSpec>,

    /// Cross-language RPC role — `Some` when this command either opens a
    /// handle onto a remote extension (`ILX::init PLUGIN EXTENSION`) or calls
    /// a method that extension implements in another language
    /// (`ILX::call HANDLE ?-timeout ms? ?--? METHOD …`).
    ///
    /// The descriptor says which word is the handle, which word is the method,
    /// and whether the call awaits a reply, so the navigation providers cross
    /// from an iRule to the Node.js `ILXServer.addMethod` registration without
    /// naming a command.  `None` = the command takes part in no
    /// RPC family.  See [`crate::remote_method`].
    pub remote_method: Option<&'static crate::remote_method::RemoteMethodRole>,

    /// Object-factory instance-name argument — `Some(idx)` when this command
    /// creates an object *command* named by its `idx`-th argument (0-based,
    /// after the command name), of the class given by its own
    /// [`Self::object_class`].  Unlike a `TclOO` `create`/`new` factory (driven
    /// by [`Traits::IS_OO_METACLASS`]), a namespace factory such as
    /// `report::report reportName columns …` names its instance positionally,
    /// so the analyser reads the index from here rather than a hardcoded shape.
    /// The bound name resolves later `reportName <method> …` dispatch through
    /// `object_class`.  `None` for a command that creates no object command.
    pub creates_instance_at: Option<u8>,

    /// Command-defining name argument — `Some(idx)` when the *literal* value
    /// of this command's `idx`-th argument (0-based, after the command name)
    /// becomes a callable command name once the call runs: `coroutine NAME
    /// cmd ?arg …?` binds `NAME` (`TclNRCoroutineObjCmd`, `tclBasic.c`).
    /// Lighter than [`Self::creates_instance_at`], which additionally binds
    /// the name to an [`Self::object_class`] for method dispatch — this is
    /// the bare "the name is now a command" fact, consumed generically by
    /// the analyser so later calls to the name don't draw W123
    /// (unknown command).  `None` = no argument names a new command.
    pub defines_command_at: Option<u8>,

    /// Effective source argv layout of a declared fire-and-forget teardown.
    /// Consumers use the selected invocation's argument owner to project
    /// these ordinals back to original words. `None` means this descriptor
    /// supplies no teardown target layout.
    pub teardown_argument_layout: Option<TeardownArgumentLayout>,

    /// Additional validity gate keyed on lexical/dispatch context rather
    /// than argument shape — see [`ContextGate`]. `None` = no
    /// context-sensitive restriction (the common case).
    pub context_gate: Option<ContextGate>,

    /// Fully-`::`-qualified namespace this ensemble's subcommands are
    /// *also* individually, separately callable under — `Some("::tcl::dict")`
    /// for `dict`, whose `exists`/`get`/etc. are genuine standalone commands
    /// (`::tcl::dict::exists`, verified via `info commands ::tcl::dict::*`
    /// against tclsh9.0/8.6), not merely ensemble-dispatch subcommand specs.
    /// A proc lexically defined *inside* that namespace (tcllib's
    /// `dicttool.tcl`: `proc ::tcl::dict::getnull {d args} { if {[exists
    /// $d {*}$args]} {...} }`) resolves a bare `exists` call to the real
    /// command through ordinary current-namespace-then-global lookup, so
    /// W123 needs a registry entry for it (see each ensemble's own
    /// `qualified_specs()`-shaped helper, e.g. `commands::tcl::dict`).  Also
    /// the namespace `dynamic_ensemble_subcommand_known` searches for a
    /// same-named proc when an ensemble's `-map` may have been reconfigured
    /// at runtime (`namespace ensemble configure dict -map …`).  `None` for
    /// every ensemble whose subcommands are C `switch`-statement arms with
    /// no individually-callable backing command (`string`, `array`, …).
    pub implementation_namespace: Option<&'static str>,

    /// Argument-0 keyword words whose value is a compile-time constant fixed
    /// by the enclosing `TclOO` **method frame** rather than by the arguments
    /// — `self`'s `class`, and nothing else in the registry today.
    ///
    /// This is what lets the optimiser answer `[self class]` from the class
    /// whose definition body encloses the method without ever matching the
    /// command or the word by name: it looks the invoked word up here and
    /// asks the frame for the named [`OoContextFact`].  Empty (the default)
    /// for every command whose result no method frame determines.
    ///
    /// Deliberately keyed on the *word*, not the command: `self`'s nine words
    /// differ — only `class` names something the source fixes.  See
    /// [`OoContextFact`] for the oracle transcript and the abstention shapes
    /// the consumer must still apply on top of this declaration.
    pub oo_context_facts: &'static [(&'static str, OoContextFact)],

    /// Argument-0 closed [`ArgValue`] words for which a bracketed command
    /// substitution `[cmd ?word?]` used as a dispatch head denotes the
    /// current `TclOO` *receiving object* — the same target `my` dispatches
    /// on — rather than a value whose class must be inferred structurally.
    /// `self`'s `object` word, and nothing else in the registry today.
    ///
    /// Distinct from [`Self::oo_context_facts`]: that field is about
    /// constant-folding a *value* the source fixes (`self class`); this one
    /// is about *dispatch-candidate resolution* for a value that is never a
    /// source constant (a fresh `::oo::ObjNN` per `new`) but always means
    /// "the enclosing object" regardless. A consumer also treats a *bare*
    /// call (no argument at all) as this same receiver when the command's
    /// own [`Arity`] permits omitting argv[0] — `self` alone is documented
    /// as equivalent to `self object`, so no separate "bare" entry is
    /// needed here. See [`CommandRegistry::is_self_receiver_call`].
    pub self_receiver_words: &'static [&'static str],
}

/// The write classes one of which every `VarWrite` position should declare,
/// so a consumer (the O109/O126 raise proof) knows whether the target is set
/// once the command completes: always written, written on a match, read then
/// written, destroyed, aliased, or a loop variable an empty loop leaves unset.
pub const VARIABLE_WRITE_CLASSES: Traits = Traits::UNCONDITIONAL_VARIABLE_WRITE
    .union(Traits::CONDITIONAL_VARIABLE_WRITE)
    .union(Traits::READS_BEFORE_WRITE)
    .union(Traits::DESTROYS_VARIABLE)
    .union(Traits::CREATES_SCOPE_ALIAS)
    .union(Traits::HAS_LOOP_BODY)
    .union(Traits::LOOP_LIST_HEADER);

fn declares_variable_write(
    arg_roles: &[(u8, ArgRole)],
    resolver_roles: &[ArgRole],
    repeated: &[RepeatedArgLayout],
    options: &[OptionSpec],
) -> bool {
    arg_roles.iter().any(|&(_, role)| role == ArgRole::VarWrite)
        || resolver_roles.contains(&ArgRole::VarWrite)
        || repeated.iter().any(|layout| layout.role == ArgRole::VarWrite)
        || options.iter().any(|option| {
            matches!(option.value, crate::hover::OptionValue::Takes(arg) if arg.role == ArgRole::VarWrite)
        })
}

impl CommandSpec {
    /// Whether the command is declared free of side effects. This is the one
    /// reader of the authored `PURE` trait outside the registry, so purity
    /// can later be derived from a complete effect footprint here alone.
    #[must_use]
    pub const fn is_pure(&self) -> bool {
        self.traits.contains(Traits::PURE)
    }

    /// Whether argument roles have a declared dynamic input contract.
    #[must_use]
    pub const fn has_dynamic_argument_roles(&self) -> bool {
        self.arg_role_resolver.is_some()
            || self.arg_role_count_resolver.is_some()
            || self.arg_role_layout_resolver.is_some()
    }

    /// Positional constructor prefix before its option table. This describes
    /// argv layout only; factory effects and returned objects need separate proof.
    #[must_use]
    pub fn constructor_prefix_words(&self) -> Option<usize> {
        self.object_class?;
        Some(usize::from(self.creates_instance_at?) + 1)
    }

    /// This command's, and each subcommand's, `VarWrite` positions that
    /// declare none of [`VARIABLE_WRITE_CLASSES`], as a trait or through a
    /// pack's `stores` outcome, named `name` or `name sub`. A consumer
    /// treats such a target as possibly unset.
    #[must_use]
    pub fn unclassified_variable_writers(&self) -> Vec<String> {
        let mut out = Vec::new();
        if (self.assigns_variable_at.is_some()
            || declares_variable_write(
                self.arg_roles,
                self.arg_role_resolver_roles,
                self.repeated_args,
                self.options,
            ))
            && !(self.traits | self.semantics.write_class()).intersects(VARIABLE_WRITE_CLASSES)
        {
            out.push(self.name.to_owned());
        }
        for sub in self.subcommands {
            if declares_variable_write(
                sub.arg_roles,
                sub.arg_role_resolver_roles,
                sub.repeated_args,
                sub.options,
            ) && !(sub.traits
                | self.traits
                | sub.semantics.write_class()
                | self.semantics.write_class())
            .intersects(VARIABLE_WRITE_CLASSES)
            {
                out.push(format!("{} {}", self.name, sub.name));
            }
        }
        out
    }
}

impl CommandSpec {
    /// Run this command's constant folder for `args` under the optimiser's
    /// resolved Tcl `version`, or `None` when the caller has no version fact.
    /// The version-aware [`Self::const_fold_versioned`] is tried first,
    /// falling back to the version-invariant [`Self::const_fold`]. Dialect
    /// names are resolved at the ingestion boundary; the registry never
    /// reinterprets a free-form spelling.
    #[must_use]
    pub fn run_const_fold(&self, args: &[&str], version: Option<TclVersion>) -> Option<String> {
        if let Some(vf) = self.const_fold_versioned {
            vf(args, version)
        } else {
            self.const_fold?(args)
        }
    }

    /// Fold bytes under actual invocation axes. A native range uses its
    /// authored result contract and selected serialization; no catalogue
    /// profile or version-only argument supplies that engine proof.
    /// Object effects and executable erasure remain independent obligations.
    #[must_use]
    pub fn run_const_fold_in(
        &self,
        args: &[&str],
        dialect: crate::InvocationDialect,
    ) -> Option<String> {
        if let Some(contract @ crate::native_result::NativeResultContract::ListRange { .. }) =
            self.native_result
        {
            self.const_fold?;
            let arguments = crate::InvocationArguments::literals(args).with_dialect(dialect);
            return contract
                .select(arguments, 0)
                .constant_range_literal_result(arguments);
        }
        self.run_const_fold(args, dialect.tcl_version)
    }

    /// The type `[<this command> args…]` produces for *this* call.
    ///
    /// The single entry point for "what does this call return" — SSA type
    /// propagation, the taint sanitiser test, and the shimmer byte-array
    /// check all resolve a call through here rather than reading
    /// [`Self::return_type`] raw, so none of them can disagree about a
    /// per-form result.
    ///
    /// `args` excludes the command name. `None` means the result's intrep is
    /// unknown for this call — the honest answer for an untypeable form, and
    /// one every caller already handles; a confidently wrong type is the
    /// hazard this guards against.
    ///
    /// A [`Self::return_type_hook`] wins over `return_type`. Subcommands
    /// resolve to their own static `return_type`: no subcommand needs a hook
    /// yet, so [`SubCommand`] carries none, keeping the authoring surface (and
    /// the `SpecTcl` grammar) no wider than the facts require.
    #[must_use]
    pub fn return_type_for_call(&self, args: &[&str]) -> Option<TclType> {
        if !self.subcommands.is_empty() {
            return args
                .first()
                .and_then(|word| self.resolve_subcommand(word))
                .and_then(|sub| sub.return_type);
        }
        // The hook's answer stands, including when it is "unknown" —
        // falling back to `return_type` there would reinstate exactly the
        // confidently-wrong typing the hook exists to prevent.
        match self.return_type_hook {
            Some(hook) => crate::return_type::resolve(hook, self, args),
            None => self.return_type,
        }
    }

    /// Whether this command takes a `{pattern body …}` clause list as its
    /// final braced word.
    ///
    /// Derived from [`Self::case_list`], which describes the clause shape a
    /// consumer actually needs. This replaces the retired `HAS_SWITCH_BODY`
    /// flag, which said strictly less than the descriptor and disagreed with
    /// it: Expect's `expect*` commands declared the clause list without ever
    /// carrying the flag.
    #[must_use]
    pub fn has_switch_body(&self) -> bool {
        self.case_list.is_some()
    }

    /// Whether any of this command's subcommands performs an irreversible
    /// operation.
    ///
    /// Derived from [`SubCommand::destructive`], the per-subcommand truth a
    /// consumer needs in order to say *which* operation is irreversible. This
    /// replaces the retired `HAS_DESTRUCTIVE_OPS` flag, a second command-level
    /// answer that disagreed with the subcommands in both directions.
    #[must_use]
    pub fn has_destructive_ops(&self) -> bool {
        self.subcommands.iter().any(|sub| sub.destructive)
    }

    /// Check if this command is available in a given dialect.
    #[must_use]
    pub fn supports_dialect(&self, dialect: Option<SurfaceQuery<'_>>) -> bool {
        match self.surface {
            None => true,
            Some(ds) => surface_admits(ds, dialect.as_ref()),
        }
    }

    /// Resolve this invocation's iRules event contract from its literal
    /// argument words. The most-specific matching form wins; a dynamic or
    /// otherwise unmatched call falls back to the command-level contract.
    #[must_use]
    pub fn event_requirements_for_args<'a>(
        &'a self,
        args: &[&str],
    ) -> ResolvedEventRequirements<'a> {
        self.event_requirement_forms
            .iter()
            .filter(|form| form.matches(args))
            .max_by_key(|form| form.argument_prefix.len())
            .map_or(
                ResolvedEventRequirements {
                    requires: self.event_requires.as_ref(),
                    only_in: &[],
                    matched_form: false,
                },
                |form| ResolvedEventRequirements {
                    requires: form.requires.as_ref(),
                    only_in: form.only_in,
                    matched_form: true,
                },
            )
    }

    /// Resolve which iRules events this invocation raises, from its literal
    /// argument words. The most-specific matching form wins; a dynamic or
    /// otherwise unmatched call falls back to the command-level fact.
    ///
    /// A matched form with no events is a positive "raises nothing" and stops
    /// the fallback — which is the whole point of declaring `TCP::notify eom`.
    /// `None` means the command declares no emission at all.
    #[must_use]
    pub fn event_emission_for_args(&self, args: &[&str]) -> Option<crate::events::EventEmission> {
        self.event_emission_forms
            .iter()
            .filter(|form| form.matches(args))
            .max_by_key(|form| form.argument_prefix.len())
            .map_or(self.event_emits, |form| Some(form.emission))
    }

    /// Which substitutions this command performs over its own argument text
    /// for one call, or `None` when it performs none — the projection of
    /// [`Self::option_effects`] onto
    /// [`crate::substitution::SubstitutionKinds`]. A
    /// [`Traits::PERFORMS_SUBSTITUTION`] command whose options move no
    /// substitution axis performs every kind on every call; an unreadable
    /// call answers every kind, and so does a call whose option run stops
    /// before the reserved operands (a word there is neither an option nor an
    /// operand).
    #[must_use]
    pub fn substitutions_performed(
        &self,
        args: InvocationArguments<'_>,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::substitution::SubstitutionKinds> {
        if !self.traits.contains(Traits::PERFORMS_SUBSTITUTION) {
            return None;
        }
        let effects = self.option_effects(args, dialect);
        let reaches_operands = args
            .exact_argv_len()
            .is_some_and(|len| effects.option_end + self.reserved_trailing_words >= len);
        Some(if reaches_operands {
            effects.substitution_kinds()
        } else {
            crate::substitution::SubstitutionKinds::ALL
        })
    }

    /// The clause plan of a call to this command: its [`Self::clause_grammar`]
    /// walked over `args` (the words after the command name), with the rows
    /// gated to `dialect`. `None` when the command declares no grammar or the
    /// grammar is unavailable at `dialect`.
    #[must_use]
    pub fn clause_plan(
        &self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<crate::clause_grammar::ClausePlan> {
        let grammar = self.clause_grammar?;
        grammar
            .available(dialect)
            .then(|| grammar.walk_at(args, self.repeated_args, dialect))
    }

    /// The structural defect a consumer reports in place of the generic arity
    /// check (`if`'s E004): the clause grammar's, for a command whose arity is
    /// checked structurally ([`Traits::STRUCTURALLY_CHECKED_ARITY`]), else the
    /// [`Self::clause_shape_check`] escape hatch's.
    ///
    /// A grammar-carrying command without the trait (`for`, `foreach`, …)
    /// keeps its arity range as the diagnostic's owner, so its walk's defect is
    /// never reported a second time here.
    #[must_use]
    pub fn clause_shape_defect(
        &self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ClauseShapeError> {
        self.traits
            .contains(Traits::STRUCTURALLY_CHECKED_ARITY)
            .then(|| self.clause_plan(args, dialect))
            .flatten()
            .and_then(|plan| plan.defect)
            .or_else(|| {
                self.clause_shape_check
                    .and_then(|check| check(crate::InvocationArguments::literals(args)))
                    .map(crate::ClauseShapeIssue::error)
            })
    }

    /// The package whose version gates this command (Tk, a tcllib package, …).
    #[must_use]
    pub fn owning_package(&self) -> Option<&'static str> {
        self.required_package.or(self.tcllib_package)
    }

    /// The signature shape at `release`: the [`Self::arity_windows`] window
    /// covering it, else [`Self::arity`].
    #[must_use]
    pub fn arity_at(&self, release: Option<&str>) -> Arity {
        ArityWindow::select(self.arity_windows, release).map_or(self.arity, |window| window.arity)
    }

    /// Whether this command exists given the resolved *`package_version`*.
    ///
    /// *`package_version`* is the guaranteed-available floor from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]).
    /// `None` is permissive; a command with an unspecified lifecycle is
    /// always available.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        self.lifecycle.available_at(package_version)
    }

    /// This command's lifecycle state at the resolved *`package_version`*.
    #[must_use]
    pub fn lifecycle_state(&self, package_version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(package_version)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod command_closed_value_prefix_controls;
