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

//! `SubCommand` and `SubSubCommand`: the fields of one ensemble member and its own purity, availability and const-fold queries.

use super::arguments::VersionedArgValue;
use super::option_relations::{ConstraintsHook, OptionPlacement, OptionRelation};
use super::{
    ArgRoleCountResolver, ArgRoleLayoutResolver, ArgRoleResolver, CommandPrefixResolver,
    ScriptTimingResolver,
};
use crate::abbrev::PrefixMatching;
use crate::arg_role::ArgRole;
use crate::arity::{Arity, ArityWindow};
use crate::body_kind::{BodyInterpreter, BodyKind};
use crate::command_table::CommandTableEffect;
use crate::dispatch_stability::DispatchDependencyDescriptor;
use crate::forms::SubCommandForm;
use crate::hooks::{
    AnalyserHookId, ArgTypeHint, CodegenHookId, ConstFoldFn, InlineCodegenHookId, LoweringHookId,
    TclVersion, VersionedConstFoldFn,
};
use crate::hover::{ArgValue, CallbackTaintInput, HoverSnippet, OptionSpec};
use crate::lifecycle::{Lifecycle, LifecycleState};
use crate::literal_validation::LiteralArgumentValidator;
use crate::patterns::{FormatType, PatternType};
use crate::presentation::ArgPresentation;
use crate::repeated::RepeatedArgLayout;
use crate::representation::RepresentationEffect;
use crate::side_effects::{SideEffect, StorageType};
use crate::state_transition::StateTransitionDescriptor;
use crate::taint::{TaintColour, TaintTransformCondition};
use crate::traits::Traits;
use crate::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
use crate::value_transfer::SemanticsDeclaration;
use crate::world_effect::WorldEffectDescriptor;
use tcl_dialect::model::{SpecSurface, SurfaceQuery, surface_admits};

/// Complete metadata for a single subcommand.
///
/// Carries its own arity, arg roles, return type, effect classification,
/// and hooks — a self-contained metadata bundle.
#[derive(Debug, Clone)]
// `pure` / `mutator` / `loop_list_header` /
// `creates_scope_alias` are subcommand-shaped behavioural facts.
// They could fold into the existing [`Traits`] bitflags field
// above, but doing so is a registry-API change that touches
// every command-spec literal.
#[allow(clippy::struct_excessive_bools)]
pub struct SubCommand {
    /// Subcommand name (e.g. `"create"`, `"for"`, `"length"`).
    pub name: &'static str,

    /// Behavioural trait flags.
    ///
    /// Subcommand-shaped facts (taint sources stamped on `chan gets`
    /// rather than `chan`, side-effect categories specific to one
    /// subcommand form, …) live here. The matched
    /// [`crate::CommandRegistry::resolve_call`] consumer reads
    /// `spec.traits | sub.traits` so subcommand traits compose with
    /// command-level ones rather than replacing them.
    pub traits: Traits,

    /// Argument count constraint (after the subcommand word).
    ///
    /// As on [`CommandSpec::arity`], the fallback whenever
    /// [`Self::arity_windows`] selects nothing.
    pub arity: Arity,

    /// Per-release signature shapes for this subcommand, with
    /// the same contract as [`CommandSpec::arity_windows`].
    pub arity_windows: &'static [ArityWindow],

    /// Short description for completion list.
    pub detail: &'static str,

    /// Invocation synopsis.
    pub synopsis: &'static str,

    /// Hover documentation.
    pub hover: Option<HoverSnippet>,

    /// Static argument roles (after the subcommand word).
    pub arg_roles: &'static [(u8, ArgRole)],

    /// Dynamic argument role resolver.
    pub arg_role_resolver: Option<ArgRoleResolver>,

    /// Cardinality-only role resolver, selected before the value resolver.
    /// Unknown expansion counts retain incomplete roles; no placeholder values
    /// are supplied to either resolver.
    pub arg_role_count_resolver: Option<ArgRoleCountResolver>,
    /// Structured native layout resolver; exclusive with value/count resolvers.
    pub arg_role_layout_resolver: Option<ArgRoleLayoutResolver>,

    /// Roles the dynamic resolver can emit for some invocation. See the
    /// command-level field of the same name.
    pub arg_role_resolver_roles: &'static [ArgRole],

    /// The subcommand's clause grammar (`dict for`, `dict map`, `dict
    /// update`, `array for`) — the subcommand-level twin of
    /// [`CommandSpec::clause_grammar`], walked over the words after the
    /// subcommand word.
    pub clause_grammar: Option<&'static crate::clause_grammar::ClauseGrammarSpec>,

    /// Formatter presentation overrides (after the subcommand word) — the
    /// subcommand-level twin of [`CommandSpec::arg_presentation`]. Empty
    /// means every body argument is laid out as a block, the default.
    pub arg_presentation: &'static [(u8, ArgPresentation)],

    /// Repeated argument-role layouts (after the subcommand word) — the
    /// subcommand-level twin of [`CommandSpec::repeated_args`]
    /// (`namespace upvar NS o l o l`, `dict update d k v k v body`).
    pub repeated_args: &'static [RepeatedArgLayout],

    /// Static command-prefix positions + appended arities (after the
    /// subcommand word), e.g. `trace add variable`'s callback.
    pub command_prefixes: &'static [(u8, crate::arg_role::AppendedArity)],

    /// Dynamic command-prefix resolver (after the subcommand word).
    pub command_prefix_resolver: Option<CommandPrefixResolver>,

    /// Dynamic executable-argument phase (after the subcommand word).
    pub script_timing_resolver: Option<ScriptTimingResolver>,

    /// Lookup frame for selected executable operands, independent of timing.
    /// Consumers must retain the operand producer and the actual entry table;
    /// the receiving command's head or helper namespace supplies neither.
    pub script_lookup_scope: Option<crate::ScriptLookupScope>,

    /// External callback substitutions for deferred executable arguments,
    /// keyed by their index after the subcommand word.  Option values carry
    /// the same fact on [`crate::hover::OptionArg`]; this table covers plain
    /// positional callback scripts such as `bind tag sequence script`.
    pub callback_taint_inputs: &'static [(u8, &'static [CallbackTaintInput])],

    /// Return type.
    pub return_type: Option<TclType>,

    /// How this subcommand types the variable(s) it writes as a side effect,
    /// overriding the parent command's [`CommandSpec::var_write_typing`] when
    /// a subcommand matches (`binary scan` destructures; `binary format` does
    /// not).  See [`VarWriteTyping`].  Default [`VarWriteTyping::ReturnValue`].
    pub var_write_typing: VarWriteTyping,

    /// Result↔element-structure fact for this subcommand (`dict create`
    /// builds a dict of its pairs; `dict get` retrieves one value). See
    /// [`ReturnElements`].
    pub return_elements: Option<ReturnElements>,

    /// In-place element evolution of the written variable for this
    /// subcommand (`dict set var … value`). See [`VarElementsEffect`].
    pub var_elements_effect: Option<VarElementsEffect>,

    /// Effect on Tcl's dual string/internal representation or shared-object
    /// storage. `None` inherits the parent command declaration.
    pub representation_effect: Option<RepresentationEffect>,

    /// Per-argument type hints.
    pub arg_types: &'static [(u8, ArgTypeHint)],

    /// Side-effect-free.
    pub pure: bool,

    /// Mutates state.
    pub mutator: bool,

    /// Compile-time constant folder.
    pub const_fold: Option<ConstFoldFn>,

    /// Tcl-version-aware constant folder (`string is`), taking priority over
    /// [`Self::const_fold`]; see [`SubCommand::run_const_fold`].
    pub const_fold_versioned: Option<VersionedConstFoldFn>,

    /// Lowering hook ID.
    pub lowering_hook: Option<LoweringHookId>,

    /// `TclVM` bytecode codegen hook ID. See
    /// [`CommandSpec::codegen_hook`].
    pub codegen_hook: Option<CodegenHookId>,

    /// Per-release codegen hooks; the contract of
    /// [`CommandSpec::codegen_hook_windows`]. A subcommand whose windows state
    /// nothing at the point inherits its command's hook, and one that declines
    /// does not.
    pub codegen_hook_windows: &'static [crate::stamp_window::StampWindow<CodegenHookId>],

    /// Inline (value-position / catch-body) bytecode codegen hook ID.
    /// See [`CommandSpec::inline_codegen_hook`]. Overrides the
    /// parent's when the call resolves to this subcommand
    /// (`dict get` / `info exists`).
    pub inline_codegen_hook: Option<InlineCodegenHookId>,

    /// Per-release inline codegen hooks; the contract of
    /// [`CommandSpec::codegen_hook_windows`].
    pub inline_codegen_hook_windows:
        &'static [crate::stamp_window::StampWindow<InlineCodegenHookId>],

    /// Analyser handler-family hook ID.
    /// See [`CommandSpec::analyser_hook`]. Overrides the parent's when
    /// the call resolves to this subcommand (`namespace eval` /
    /// `dict for`).
    pub analyser_hook: Option<AnalyserHookId>,

    /// The value-transfer specialisation declared at subcommand scope. See
    /// [`CommandSpec::semantics`]; a declaration here overrides the
    /// command's, and an explicit abstention stops the command's from
    /// applying to this subcommand.
    pub semantics: SemanticsDeclaration,

    /// Command-table mutation descriptor.
    /// See [`CommandSpec::command_table_effect`]. Overrides the
    /// parent's when the call resolves to this subcommand
    /// (`interp alias`).
    pub command_table_effect: Option<CommandTableEffect>,

    /// Per-subcommand options.
    pub options: &'static [OptionSpec],

    /// Fixed positional words before the selected invocation's option run.
    /// Counted after the command head, or after a selected subcommand word.
    pub option_prefix_words: usize,

    /// Trailing positional words excluded from this subcommand's option scan.
    /// This selects the same structural boundary as
    /// [`CommandSpec::reserved_trailing_words`], after the subcommand word.
    pub reserved_trailing_words: usize,

    /// Typed relations between this subcommand's options and arguments
    /// (E-R14), checked natively.
    pub option_relations: &'static [OptionRelation],

    /// The option-effect families this subcommand's options cite — see
    /// [`CommandSpec::option_effect_families`].
    pub option_effect_families: &'static [crate::option_effect::OptionEffectFamily],

    /// The subcommand's `constraints` escape hatch — see
    /// [`CommandSpec::constraints`].
    pub constraints: Option<ConstraintsHook>,

    /// Where this subcommand's options may appear — see
    /// [`CommandSpec::option_placement`].
    pub option_placement: OptionPlacement,

    /// Documented minimum abbreviation length for this subcommand's own
    /// name, when the command promises a longer minimum than uniqueness
    /// alone requires.  `None` = uniqueness is the only constraint.
    pub min_abbrev: Option<u8>,

    /// Whether this subcommand's own keyword tables
    /// ([`Self::sub_subcommands`] and [`Self::options`]) honour
    /// unique-prefix abbreviation.
    pub prefix_matching: PrefixMatching,

    /// Enumerable positional-argument values, keyed by 0-based
    /// argument index *after* the subcommand word.  Drives
    /// value completion — e.g. `string is <class>` declares
    /// `(0, &[alnum, alpha, …])` so the character classes
    /// complete at the first sub-arg.
    pub arg_values: &'static [(u8, &'static [ArgValue])],

    /// Package-version gates for individual literal values in
    /// [`Self::arg_values`].
    pub versioned_arg_values: &'static [VersionedArgValue],

    /// Structured invocation-form descriptors for the subcommand.
    ///
    /// Same shape as [`CommandSpec::command_forms`]; entries are
    /// matched against the argument list *after* the subcommand
    /// word.
    pub subcommand_forms: &'static [SubCommandForm],

    /// Target-neutral semantic-operation override for this subcommand.
    ///
    /// A matching form may override it. `None` inherits the parent command's
    /// semantic operation or its common structural-lowering descriptor.
    pub semantic_operation: Option<crate::semantic_operation::SemanticOperationId>,

    /// Per-release semantic operations; the contract of
    /// [`CommandSpec::codegen_hook_windows`].
    pub semantic_operation_windows: &'static [crate::stamp_window::StampWindow<
        crate::semantic_operation::SemanticOperationId,
    >],

    /// Target-neutral completion semantics for this subcommand.
    ///
    /// A matching subcommand form takes precedence. `Unknown` inherits the
    /// parent command's descriptor.
    pub completion: crate::knowledge::Knowledge<crate::completion::CompletionDescriptor>,

    /// Result-dependency refinement for this subcommand. `Unknown` inherits
    /// the parent command declaration.
    pub result_stability: crate::knowledge::Knowledge<crate::result_stability::ResultStability>,

    /// Native result dependency; runtime identity and completion remain required.
    pub native_result: Option<crate::native_result::NativeResultContract>,

    /// Dialect membership. `None` = inherit from parent `CommandSpec`.
    pub surface: Option<&'static [SpecSurface]>,

    /// Introduction / deprecation / retirement releases of this subcommand on
    /// the parent command's owning-package version axis.
    /// [`Lifecycle::UNSPECIFIED`] means it is present in every package
    /// version.
    pub lifecycle: Lifecycle,

    /// Safe-on-uninit dialect set.
    pub safe_on_uninit: Option<&'static [SpecSurface]>,

    /// CFG header with list-expression args (foreach/lmap subcommand).
    pub loop_list_header: bool,

    /// Creates a scope alias (upvar-like binding).
    pub creates_scope_alias: bool,

    /// Inferred storage type for target variable.
    pub inferred_storage_type: Option<StorageType>,

    /// Body-kind classification for `ArgRole::Body` args declared on
    /// this subcommand.  See [`CommandSpec::body_kind`] for the
    /// semantics; default `Plain`.
    pub body_kind: BodyKind,

    /// Execution grammar for immediate evaluated script regions.
    pub body_execution: Option<crate::body_execution::BodyExecutionSpec>,
    /// Native compiler grammar and body entry for this selected subcommand.
    pub native_compilation: Option<crate::native_compilation::NativeCompilationSpec>,
    /// Audited successful handler transfer, independent of compiler selection.
    pub successful_handler: Option<crate::native_compilation::SuccessfulHandlerSpec>,
    /// Selected variable receiver grammar, indexed after the selected head or
    /// subcommand. This is naming metadata only, independent of successful
    /// execution and whole-array roles. `None` inherits; an empty slice rejects
    /// a receiver grammar for this descriptor.
    pub variable_receivers:
        Option<&'static [(u8, crate::resolved_invocation::VariableReceiverOperandForm)]>,

    /// Interpreter realm for this subcommand's evaluated body arguments.
    /// See [`CommandSpec::body_interpreter`].
    pub body_interpreter: BodyInterpreter,

    /// How this subcommand transforms a byte-array (binary) operand — drives
    /// the S110 byte-array-corruption check for `string`'s value subcommands
    /// (`range` / `map` / `tolower` / …).  See
    /// [`CommandSpec::byte_array_effect`] and [`crate::byte_array_effect`];
    /// default [`ByteArrayEffect::None`].
    pub byte_array_effect: crate::byte_array_effect::ByteArrayEffect,

    /// Subcommand-relative argument indices (0-based, *after* the subcommand
    /// word) whose [`Self::arg_values`] are an **exhaustive** legal set — the
    /// subcommand-level twin of [`CommandSpec::closed_value_args`], for W127.
    /// `string is <class>` marks its class arg (`&[0]`). Default empty.
    pub closed_value_args: &'static [u8],

    /// Whether a [`Self::closed_value_args`] literal is accepted as a *unique
    /// prefix* of an allowed value rather than an exact match — C Tcl's
    /// abbreviation rule for `string is <class>` (`boo` → `boolean`). W127
    /// then fires only for a value that is not a prefix of any allowed value
    /// (`booleanx`). Default `false` (exact match, as on the command-level default).
    pub arg_values_accept_prefix: bool,

    /// Implicit-args count for proc-call arity relaxation.  See
    /// [`CommandSpec::body_arg_implicit_args`].
    pub body_arg_implicit_args: u8,

    // Granular taint / security metadata
    //
    /// Colour bits this subcommand adds to a tainted value it returns
    /// (`file join` ⇒ `PATH_JOINED`, `file normalize` ⇒
    /// `PATH_NORMALISED`). `None` = no transform.
    pub taint_transform: Option<TaintColour>,

    /// Condition on a call's own argument words that must hold before
    /// [`Self::taint_transform`] is claimed — the subcommand-level
    /// [`CommandSpec::taint_transform_when`]. `None` = always claimed.
    pub taint_transform_when: Option<TaintTransformCondition>,

    /// Colour whose presence on the input means this subcommand would
    /// double-encode the value (T106). `None` = none.
    pub taint_double_encode_colour: Option<TaintColour>,

    /// Output-sink diagnostic code for a subcommand-shaped XSS /
    /// header-injection sink (e.g. `"IRULE3002"`). `None` = not a
    /// sink.
    pub taint_output_sink: Option<&'static str>,

    /// Credential-value argument index, counting the selected subcommand word
    /// itself as zero. This field preserves the `SpecTcl` verbatim coordinate,
    /// unlike other subcommand-relative argument fields. `None` = none.
    pub credential_arg: Option<u8>,

    /// Conditional source option relationship; no channel state or completion.
    pub channel_configuration: Option<crate::ChannelConfigurationSpec>,

    /// HTTP header names whose values are secrets, for a
    /// subcommand-shaped header sink. Empty = none.
    pub sensitive_headers: &'static [&'static str],

    // Structured spec fields (subcommand overrides)
    //
    /// Pattern-language override for this subcommand (`string match`
    /// ⇒ `Glob`), taking priority over the parent command's
    /// [`CommandSpec::pattern_type`].
    pub pattern_type: Option<PatternType>,

    /// Format-string override for this subcommand (`clock format`
    /// ⇒ `Clock`, `binary scan` ⇒ `Binary`), taking priority over the
    /// parent command's [`CommandSpec::format_string_type`].
    pub format_string_type: Option<FormatType>,

    /// Structured side-effect declarations for this subcommand, and how
    /// completely they are known.
    pub side_effects: crate::knowledge::Knowledge<&'static [SideEffect]>,

    /// Mutable Tcl-world effects for this subcommand.
    ///
    /// A matching form is applied after this descriptor.  When absent,
    /// resolution simply retains the parent command's descriptor; otherwise
    /// it extends by default or explicitly replaces inherited facts.
    /// How completely it is known is the field's [`crate::knowledge::Knowledge`]:
    /// `Unknown` states nothing here, and only `Complete` closes the domain.
    pub world_effects: crate::knowledge::Knowledge<WorldEffectDescriptor>,

    /// Target-neutral transitions specific to this subcommand.
    pub state_transitions: Option<StateTransitionDescriptor>,

    /// Dispatch-stability dependency refinement for this subcommand.
    /// Irreducible live-interpreter dependencies cannot be removed.
    pub dispatch_dependencies: Option<DispatchDependencyDescriptor>,

    /// Relationship/content validator for statically-known literal arguments.
    /// A matching form may override this callback.
    pub literal_argument_validator: Option<LiteralArgumentValidator>,

    /// Conditional index/container relationship for original source advice.
    /// This descriptor grants no native handler, current value or result.
    pub source_index_bounds: Option<crate::source_index_bounds::SourceIndexBounds>,

    /// Irreversible operation (`file delete`, …).
    pub destructive: bool,

    /// Returns a filesystem path.
    pub returns_path: bool,

    /// Conditional source path algebra operation, independent of runtime
    /// handler, current filename, filesystem state and execution admission.
    pub source_path_operation: Option<crate::source_path::SourcePathOperation>,

    /// Conditional package-query source algebra; custom packs cannot assert
    /// this builtin survey relation. No physical provision or runtime identity.
    pub source_package_query: Option<crate::source_navigation::SourcePackageQuery>,

    /// Conditional configuration-reference layout from the selected source
    /// schema. This grants no accepted argv, runtime object or handler identity.
    pub source_configuration_reference:
        Option<crate::source_configuration_reference::SourceConfigurationReferenceLayout>,

    /// Performs unescaping / decoding.
    pub is_unescape: bool,

    /// CFG-lowered command name for ensemble subcommands rewritten by
    /// the lowering pass.
    pub cfg_rewrite_name: Option<&'static str>,

    /// Nested subcommands for a two-level ensemble, matched at the argument
    /// index immediately after this subcommand word.
    ///
    /// A handful of `info` subcommands are themselves ensembles whose *next*
    /// word selects a further operation — `info object <subcommand> object …`
    /// and `info class <subcommand> class …` (per the `info` man page's OBJECT
    /// INTROSPECTION and CLASS INTROSPECTION sections). Declaring them here lets
    /// the semantic-token pass colour that word as a subcommand keyword, and
    /// drives hover and completion for it. Empty for the
    /// overwhelmingly-common single-level subcommand.
    pub sub_subcommands: &'static [SubSubCommand],

    /// Command-defining name argument (0-based, *after* the subcommand word)
    /// — the subcommand-level twin of [`CommandSpec::defines_command_at`]:
    /// `interp create ?-safe? ?--? ?name?` binds `name` as a callable
    /// command.  A word at the index that is an option flag (leading `-`) or
    /// a missing name (auto-generated at run time) names nothing statically.
    pub defines_command_at: Option<u8>,

    /// How many leading option **words** this subcommand consumes before every
    /// remaining word is positional, however it is spelled.  `None` — the
    /// ordinary rule: keep consuming while each word matches a declared
    /// option.
    ///
    /// `namespace export ?-clear?` and `namespace import ?-force?` parse
    /// exactly one optional leading flag by hand in C (`NamespaceExportCmd` /
    /// `NamespaceImportCmd` in `tclNamesp.c` compare `objv[1]` and nothing
    /// else), so a *second* occurrence is an ordinary argument word.  Oracle
    /// (tclsh 8.6.14 / 9.0.4): `namespace export -clear -clear p` leaves
    /// exactly `-clear p` exported — and a command genuinely named `-clear`
    /// is then importable, `namespace import ::src::*` binding `::dst::-clear`
    /// — while `namespace import -force -force ::src::*` aborts with `no
    /// namespace specified in import pattern "-force"`.  Declaring the limit
    /// here keeps the count in registry data rather than as a loop bound in a
    /// consumer.
    pub max_leading_option_words: Option<u8>,
}

/// A second-level subcommand of a two-level ensemble (`info object <op>`,
/// `info class <op>`).
///
/// Lighter than a full [`SubCommand`]: it carries just what the LSP needs to
/// highlight, hover, and complete the word after the first-level subcommand
/// Resolution accepts a unique prefix, matching how Tcl's own
/// ensemble dispatch abbreviates subcommands.
#[derive(Debug, Clone, Copy)]
pub struct SubSubCommand {
    /// Audited compiler registration of this exact nested worker. Absence is
    /// unknown and never inherits the outer ensemble's compiler protocol.
    pub native_compilation: Option<crate::native_compilation::NativeCompilationSpec>,
    /// Canonical operation name (`"class"`, `"superclasses"`, …).
    pub name: &'static str,
    /// One-line description for hover / completion detail.
    pub detail: &'static str,
    /// Invocation synopsis, e.g. `"info object class object ?className?"`.
    pub synopsis: &'static str,
    /// Dialect membership; `None` inherits from the owning subcommand.
    pub surface: Option<&'static [SpecSurface]>,
    /// Introduction / deprecation / retirement releases of this second-level
    /// subcommand on the owning command's package version axis.
    /// [`Lifecycle::UNSPECIFIED`] means it is present in every package
    /// version — orthogonal to [`Self::dialects`], which gates on the Tcl
    /// *core* version.
    pub lifecycle: Lifecycle,
    /// Options recognised by **this** second-level word only.
    ///
    /// Three-valued on purpose, following the same `None`-inherits idiom as
    /// [`Self::dialects`]:
    ///
    /// * `None` — the usual case. This operation declares nothing about
    ///   options, so the owning [`SubCommand::options`] table is the whole
    ///   answer.
    /// * `Some(&[])` — this operation takes **no options at all**. An empty
    ///   table is a statement, not an absence: the parent's table must not
    ///   leak into it.
    /// * `Some(table)` — this table applies *instead of* the parent's, never
    ///   merged with it, because merging is exactly the bug the field exists
    ///   to fix.
    ///
    /// `namespace ensemble` needs all three. `create` and
    /// `configure` are two different C option tables — `ensembleCreateOptions`
    /// has `-command` and no `-namespace`, `ensembleConfigOptions` the reverse
    /// (`tclEnsemble.c`) — so one merged table simultaneously offers `create`'s
    /// `-command` to `configure` (a guaranteed `bad option "-command"`) and
    /// hides `configure`'s readable `-namespace`. `exists` then needs the
    /// empty table: its C arm takes `cmdname` and nothing else, so inheriting
    /// the parent's union would offer flags that are not options there at all.
    pub options: Option<&'static [OptionSpec]>,
    /// Fixed positional words after this nested selector before its options.
    /// Relevant when this operation declares its own option table.
    pub option_prefix_words: usize,
}

/// Whether `sub_sub` is present in `dialect`, inheriting `parent_surface`
/// (the owning subcommand's gate) when it declares none of its own.
pub(super) fn sub_subcommand_supports_dialect(
    sub_sub: &SubSubCommand,
    dialect: Option<SurfaceQuery<'_>>,
    parent_surface: Option<&'static [SpecSurface]>,
) -> bool {
    sub_sub
        .surface
        .or(parent_surface)
        .is_none_or(|have| surface_admits(have, dialect.as_ref()))
}

impl SubSubCommand {
    /// Whether this second-level subcommand exists given the resolved
    /// *`package_version`*.
    ///
    /// *`package_version`* is the guaranteed-available floor from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]).
    /// `None` is permissive.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        self.lifecycle.available_at(package_version)
    }

    /// This second-level subcommand's lifecycle state at the resolved
    /// *`package_version`*.
    #[must_use]
    pub fn lifecycle_state(&self, package_version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(package_version)
    }
}

impl SubCommand {
    /// Behavioural traits selected for this member. The dedicated purity and
    /// scope-alias declarations compose with its explicit trait flags.
    #[must_use]
    pub const fn semantic_traits(&self) -> Traits {
        self.traits
            .union(if self.pure {
                Traits::PURE
            } else {
                Traits::empty()
            })
            .union(if self.creates_scope_alias {
                Traits::CREATES_SCOPE_ALIAS
            } else {
                Traits::empty()
            })
    }

    /// Whether argument roles have a declared dynamic input contract.
    #[must_use]
    pub const fn has_dynamic_argument_roles(&self) -> bool {
        self.arg_role_resolver.is_some()
            || self.arg_role_count_resolver.is_some()
            || self.arg_role_layout_resolver.is_some()
    }
}

impl SubCommand {
    /// Whether the subcommand is declared free of side effects, by its
    /// `pure` flag or its own `PURE` trait.
    #[must_use]
    pub const fn is_pure(&self) -> bool {
        self.pure || self.traits.contains(Traits::PURE)
    }

    /// The clause plan of a call to this subcommand: its
    /// [`Self::clause_grammar`] walked over `args` — the words **after** the
    /// subcommand word, so the plan's indices are the subcommand's own — with
    /// the rows gated to `dialect`. `None` when the subcommand declares no
    /// grammar or the grammar is unavailable at `dialect`.
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

    /// Whether this subcommand exists at `package_version`.
    ///
    /// `None` is permissive, matching [`CommandSpec::available_for_version`].
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        self.lifecycle.available_at(package_version)
    }

    /// This subcommand's lifecycle state at `package_version`.
    #[must_use]
    pub fn lifecycle_state(&self, package_version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(package_version)
    }

    /// Run this subcommand's constant folder for `args` under a resolved Tcl
    /// `version`, trying [`Self::const_fold_versioned`] first and then the
    /// invariant [`Self::const_fold`]. See [`CommandSpec::run_const_fold`].
    #[must_use]
    pub fn run_const_fold(&self, args: &[&str], version: Option<TclVersion>) -> Option<String> {
        if let Some(vf) = self.const_fold_versioned {
            vf(args, version)
        } else {
            self.const_fold?(args)
        }
    }

    /// Selected native constant-result bytes, separate from object effects.
    /// See [`CommandSpec::run_const_fold_in`] for the actual-policy boundary.
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

    /// Compiler descriptor of an exact frozen nested selector. This projection
    /// supplies no nested operand, result or successful-handler semantics.
    #[must_use]
    pub fn nested_native_compilation(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::native_compilation::NativeCompilationSpec> {
        let crate::InvocationArgument::Word(selector) = arguments.argv_at(0) else {
            return None;
        };
        let selector = selector.literal()?;
        let query = arguments.dialect()?.authoring_query()?;
        let compiler = self
            .sub_subcommands
            .iter()
            .find(|worker| {
                worker.name == selector
                    && sub_subcommand_supports_dialect(worker, Some(query), self.surface)
            })?
            .native_compilation?;
        compiler.compiler_hook_presence(arguments.dialect()?)?;
        Some(compiler)
    }
}
