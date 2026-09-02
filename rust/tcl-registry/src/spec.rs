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

use crate::abbrev::{Keyword, KeywordMatch, KeywordTable, PrefixMatching};
use crate::arg_role::ArgRole;
use crate::arity::{Arity, ArityWindow};
use crate::body_kind::BodyKind;
use crate::clause_shape::ClauseShapeChecker;
use crate::command_table::CommandTableEffect;
use crate::dispatch_stability::{DispatchDependencies, DispatchDependencyDescriptor};
use crate::events::{EventRequirementForm, ResolvedEventRequirements};
use crate::forms::{CommandForm, SubCommandForm};
use crate::frame_effect::FrameEffectSpec;
use crate::hooks::ReturnTypeHookId;
use crate::hooks::{
    AnalyserHookId, ArgTypeHint, CodegenHookId, ConstFoldFn, InlineCodegenHookId, LoweringHookId,
    TclVersion, VersionedConstFoldFn,
};
use crate::hover::{
    ArgValue, CallbackTaintInput, FormSpec, HoverSnippet, OptionSpec, ScriptTiming,
};
use crate::invocation_words::CommandPrefixArguments;
use crate::lifecycle::{Lifecycle, LifecycleState};
use crate::literal_validation::LiteralArgumentValidator;
use crate::patterns::{FormatType, PatternType};
use crate::presentation::ArgPresentation;
use crate::relation::{Relation, RelationFactSource, RelationTermKind, TermHolds};
use crate::repeated::RepeatedArgLayout;
use crate::representation::RepresentationEffect;
use crate::side_effects::{SideEffect, StorageType};
use crate::state_transition::StateTransitionDescriptor;
use crate::symbol_def::SymbolDef;
use crate::taint::{SetterConstraint, TaintColour};
use crate::traits::Traits;
use crate::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
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

/// Resolver for variable-layout [`ArgRole::CommandPrefix`] positions and their
/// appended arities (`trace add …`, `interp alias`, `selection handle`) where
/// the prefix index depends on the actual arguments. Returns
/// `(arg_index, appended_arity)` pairs.  Paired with the static
/// [`CommandSpec::command_prefixes`] table — either may be set.
pub type CommandPrefixResolver =
    for<'a> fn(CommandPrefixArguments<'a>) -> Vec<(u8, crate::arg_role::AppendedArity)>;

/// Resolver for invocation-sensitive executable-argument timing.
///
/// Returns `(arg_index, timing)` pairs in the same coordinates as
/// [`ArgRoleResolver`]. The registry only accepts an answer for an argument
/// currently classified as [`ArgRole::Body`], [`ArgRole::LambdaLiteral`], or
/// [`ArgRole::CommandPrefix`], so a resolver cannot turn ordinary data into
/// executable code by itself.
pub type ScriptTimingResolver = fn(args: &[&str]) -> Vec<(u8, ScriptTiming)>;

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
pub type ContextGate = fn(args: &[&str], in_event_body: bool) -> Option<&'static str>;

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
}

impl DefaultFormFirstWord {
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

/// A compile-time-constant fact about the **enclosing `TclOO` method frame**
/// that an introspection keyword answers with.
///
/// Declared per keyword word on [`CommandSpec::oo_context_facts`], so a
/// consumer folding `[self class]` reads registry data rather than matching
/// the command name or the subcommand word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OoContextFact {
    /// The class that **defines** the currently executing method
    /// implementation — what `self class` returns.
    ///
    /// The source fixes it: it is the class whose definition body lexically
    /// encloses the method, and it stays that class through inheritance,
    /// mixins, and `next`.  Oracle, byte-identical on tclsh 9.0.4 and 8.6.16:
    ///
    /// ```tcl
    /// oo::class create ::A { method m {} { self class } }
    /// [::A new] m                       ;# -> ::A
    /// oo::class create ::B { superclass ::A }
    /// [::B new] m                       ;# -> ::A   (the definer, not the receiver)
    /// oo::define ::B { method n {} { self class } }
    /// [::B new] n                       ;# -> ::B
    /// oo::class create ::M { method mx {} { self class } }
    /// oo::define ::B { mixin ::M } ; [::B new] mx
    ///                                   ;# -> ::M   (a mixin still reports its definer)
    /// oo::class create ::S { method q {} { list S:[self class] } }
    /// oo::class create ::T { superclass ::S
    ///                        method q {} { list T:[self class] {*}[next] } }
    /// [::T new] q                       ;# -> T:::T S:::S  (per chain link)
    /// oo::class create ::E { constructor {} { self class } }   ;# ctor -> ::E
    /// oo::class create ::G { method real {} {return real}
    ///                        method filt args { list [self class] {*}[next {*}$args] }
    ///                        filter filt }
    /// [::G new] real                    ;# -> ::G real       (a filter is a method)
    /// namespace eval ::N { oo::class create C { method m {} { self class } } }
    /// [::N::C new] m                    ;# -> ::N::C
    /// ```
    ///
    /// Two shapes make it **not** a source constant.  A consumer must abstain
    /// on both — the first is not even a value:
    ///
    /// ```tcl
    /// # (1) The implementation is not defined by a class: an `oo::objdefine`
    /// #     instance method, or a method on the class object itself
    /// #     (`self method` / `classmethod`).  `self class` raises.
    /// oo::objdefine $obj { method p {} { self class } }
    /// $obj p                ;# -> error: method not defined by a class
    /// oo::class create ::U { self method cm {} { self class } }
    /// ::U cm                ;# -> error: method not defined by a class
    ///
    /// # (2) The class command was renamed: `self class` answers with the
    /// #     class's *current* name, not the one written at definition —
    /// #     the same rename-captures-identity rule `indirection.rs` applies.
    /// oo::class create ::R { method r {} { self class } }
    /// set r [::R new] ; rename ::R ::R2
    /// $r r                  ;# -> ::R2
    /// ```
    ///
    /// **Downstream of the fold.**  The folded class name is an ordinary
    /// constant word, so it feeds the ordinary `const_fold` callbacks: this is
    /// what makes the real-corpus ticklecharts chain
    /// `set ns [namespace qualifiers [self class]] ; ${ns}::setdef …` resolve
    /// in one step (issue #1096).  `namespace qualifiers` / `namespace tail`
    /// are pure string operations — they split a string at its last `::` and
    /// never consult the interpreter's namespace table — so no namespace has
    /// to exist for the chain to be sound.
    DefiningClass,
}

/// Metadata for a `TclOO` / megawidget class whose instances are dispatched as
/// `$obj <method> …`.
///
/// Attached to the class *command* spec (the factory — e.g.
/// `ticklecharts::chart`, created by `oo::class create`) via
/// [`CommandSpec::object_class`].  For a `TclOO` class the class name **is** the
/// factory command name, so the registry resolves a class spec by looking the
/// name up in the ordinary command table (no separate index) — see
/// [`crate::CommandRegistry::object_class`].
///
/// The class's `new` / `create` constructor returns an object handle of
/// `class_name`; a later `$handle method …` dispatch resolves `method` against
/// [`Self::instance_methods`], which reuse the [`SubCommand`] shape so option /
/// enum / arg-value highlighting, arity, and hover work identically to an
/// ensemble subcommand.  This is the registry half of the object-method
/// pattern: knowing the class of an object handle (via the compiler's
/// object-type tracking) plus the class's methods lets `$chart Xaxis -name …`
/// light up its options precisely rather than by shape alone (issue #748).
#[derive(Debug)]
pub struct ObjectClassSpec {
    /// Fully-qualified class name — equal to the factory command name for a
    /// `TclOO` class (`"ticklecharts::chart"`).
    pub class_name: &'static str,

    /// Instance methods dispatched on an object handle (`Xaxis`, `Add`,
    /// `SetOptions`, …), in declaration order.  Reuses [`SubCommand`].
    pub instance_methods: &'static [SubCommand],

    /// Direct superclass names, for inherited-method resolution.  Each is
    /// itself a class command name resolvable via
    /// [`crate::CommandRegistry::object_class`].  Empty = none.
    pub superclasses: &'static [&'static str],

    /// Whether an unrecognised instance method is accepted without complaint
    /// (a class with a dynamic `unknown` handler or runtime-generated
    /// methods).  Highlighting-only today; reserved for a future
    /// unknown-method diagnostic.
    pub allow_unknown_methods: bool,

    /// Whether the instance-method command table accepts unique prefixes.
    ///
    /// Object commands are not ensembles by default: `TclOO` and most
    /// extension object APIs dispatch the method word exactly.  Tk widget
    /// command tables are the deliberately documented exception and opt in
    /// where the Tk source proves `Tk_GetIndexFromObj`-style dispatch.
    /// Keeping this strict by default prevents a guessed object method from
    /// becoming a completion, hover, or taint fact.
    pub method_prefix_matching: PrefixMatching,
}

impl ObjectClassSpec {
    /// Look up an instance method by name (this class only — no superclass
    /// walk; the registry's [`crate::CommandRegistry::instance_method`] does
    /// the inherited resolution). Lookup follows this class's declared
    /// [`Self::method_prefix_matching`] policy: strict classes require the
    /// complete spelling, while an enabled table accepts a unique non-empty
    /// prefix. An exact spelling wins and an ambiguous prefix abstains.
    #[must_use]
    pub fn instance_method(&self, name: &str) -> Option<&SubCommand> {
        let table = KeywordTable::from_keywords(
            self.instance_methods.iter().map(|method| Keyword {
                name: method.name,
                min_abbrev: method.min_abbrev,
            }),
            self.method_prefix_matching,
        );
        let canonical = table.resolve(name).unique()?;
        self.instance_methods
            .iter()
            .find(|method| method.name == canonical)
    }
}

/// The shape of a `{pattern body …}` clause list (see
/// [`CommandSpec::case_list`]).
///
/// `switch` and Expect's `expect` are the same construct with different
/// spellings: `switch` decides regex-ness once for the whole list (`-regexp`)
/// and takes a subject argument; `expect` decides it per clause (`-re`) and has
/// no subject.  Both have patterns that are keywords rather than match text.
#[derive(Debug, Clone, Copy)]
pub struct CaseListSpec {
    /// Non-option words between the command's options and the clause list —
    /// `switch`'s subject string is 1; `expect` has none.
    pub subject_args: u8,
    /// Tcl release availability for the two-post-command-word form that
    /// suppresses option scanning. Tcl 8.5 changed `switch` so
    /// `switch -regexp {pattern body}` treats `-regexp` as the subject;
    /// Tcl 8.4 still scans it as an option and rejects the missing subject.
    /// `None` means this case-list descriptor has no such exception.
    pub two_arg_optionless_surface: Option<&'static [SpecSurface]>,
    /// A *command* option that makes every pattern a regex (`switch -regexp`).
    pub regex_option: Option<&'static str>,
    /// Command option selecting literal equality (the default switch mode).
    pub exact_option: Option<&'static str>,
    /// Command option selecting Tcl glob matching.
    pub glob_option: Option<&'static str>,
    /// Command option making the selected comparison mode case-insensitive.
    pub nocase_option: Option<&'static str>,
    /// Command option ending command-level option parsing.
    pub end_options_option: Option<&'static str>,
    /// Body word that falls through to the following clause's body.
    pub fallthrough_body: Option<&'static str>,
    /// Value-taking options legal only in regular-expression mode.
    pub value_options_require_regex: &'static [&'static str],
    /// Zero-value command options selecting a specialised comparison mode.
    pub special_match_options: &'static [&'static str],
    /// Flags that may precede a pattern *inside* the list (Expect's `-re`,
    /// `-gl`, `-ex`, `-nocase`, `-timeout`).  Empty means no clause flags.
    pub clause_flags: &'static [&'static str],
    /// The clause flag that makes its pattern a regex (Expect's `-re`).
    pub clause_regex_flag: Option<&'static str>,
    /// Clause flags that consume a following value word (`-timeout 5`).
    pub clause_value_flags: &'static [&'static str],
    /// Clause flag that ends flag parsing, making a following hyphenated word
    /// a pattern rather than an option.
    pub clause_end_options_flag: Option<&'static str>,
    /// Clause flag that makes a following braced word one pattern rather than
    /// a braced clause list (Expect's `-nobrace`).
    pub clause_force_inline_flag: Option<&'static str>,
    /// Exact-only outer-shape flag selecting a braced clause list (`-brace`).
    pub clause_force_list_flag: Option<&'static str>,
    /// Shape required by [`Self::clause_force_list_flag`], when present.
    pub clause_force_list_shape: Option<CaseForceListShape>,
    /// Whether a final pattern may omit its action (Expect permits this).
    pub allow_omitted_final_body: bool,
    /// Patterns that are keywords, not match text (`default`; Expect's
    /// `timeout` / `eof` / `full_buffer`).
    pub keyword_patterns: &'static [&'static str],
    /// Whether a keyword pattern is special only in the final clause.
    pub keyword_patterns_require_final: bool,
    /// Whether unbraced action bodies carry the `switch` substitution warning.
    /// Expect actions deliberately use a different evaluation model, so its
    /// descriptor leaves this off rather than making a consumer name `switch`.
    pub warn_unbraced_bodies: bool,
    /// A literal word that may sit between the subject and the clause list and
    /// is syntax rather than a pattern — Tcl 8.x's `case string ?in? …`, whose
    /// `Tcl_CaseObjCmd` consumes an `in` at exactly that position
    /// (`generic/tclCmdAH.c`). Recognised only as an exact spelling, because a
    /// substituted word cannot be told apart from a first pattern. `None` for
    /// every descriptor without one.
    pub optional_subject_separator: Option<&'static str>,
}

/// Registry-owned position/arity shape for an outer clause-list selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseForceListShape {
    /// The selector is the first argument and must have exactly one following
    /// argument containing the clause list.
    FirstArgOnlyRemainder,
}

impl CaseForceListShape {
    fn matches(self, args: &[&str], selector: Option<&str>, index: usize) -> bool {
        match self {
            Self::FirstArgOnlyRemainder => {
                index == 0 && args.len() == 2 && selector == args.first().copied()
            }
        }
    }
}

/// Registry-owned comparison mode for a case-list invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMatchMode {
    /// Literal string equality.
    Exact,
    /// Tcl glob matching.
    Glob,
    /// Regular-expression matching; static consumers may abstain.
    Regexp,
    /// A registry-recognised specialised comparison option not represented by
    /// the common string modes (for example Tcl 9.1 `switch -integer`).
    /// Structural consumers may still traverse bodies; evaluators abstain.
    Other,
}

/// Validated command-level layout of a case-list invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaseInvocation {
    /// Subject word index, when the command has one.
    pub subject_index: Option<usize>,
    /// Single braced clause-list word, or `None` for inline pairs.
    pub clause_list_index: Option<usize>,
    /// First inline pattern index when not using a clause-list word.
    pub inline_clause_start: Option<usize>,
    /// Comparison mode selected by registry-declared options.
    pub mode: CaseMatchMode,
    /// Whether matching is case-insensitive.
    pub nocase: bool,
}

/// A validated inline pattern/action pair.  Indices are post-command-name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineCaseClause {
    /// Leading clause-flag word indices (values of value-taking flags omitted).
    pub flag_indices: Vec<usize>,
    /// Pattern word index in the invocation.
    pub pattern_index: usize,
    /// Action-body word index in the invocation.
    pub body_index: Option<usize>,
    /// Matching mode selected by this clause's leading flags.
    pub mode: CaseMatchMode,
}

impl CaseListSpec {
    /// The `switch … { pat body … }` shape.
    pub const SWITCH: Self = Self {
        subject_args: 1,
        two_arg_optionless_surface: Some(SpecSurface::TCL85_PLUS),
        regex_option: Some("-regexp"),
        exact_option: Some("-exact"),
        glob_option: Some("-glob"),
        nocase_option: Some("-nocase"),
        end_options_option: Some("--"),
        fallthrough_body: Some("-"),
        value_options_require_regex: &["-matchvar", "-indexvar"],
        special_match_options: &["-integer"],
        clause_flags: &[],
        clause_regex_flag: None,
        clause_value_flags: &[],
        clause_end_options_flag: None,
        clause_force_inline_flag: None,
        clause_force_list_flag: None,
        clause_force_list_shape: None,
        allow_omitted_final_body: false,
        keyword_patterns: &["default"],
        keyword_patterns_require_final: true,
        optional_subject_separator: None,
        warn_unbraced_bodies: true,
    };

    /// The obsolete Tcl 8.x `case string ?in? { pat body … }` shape.
    ///
    /// `switch`'s ancestor, with none of its options: `Tcl_CaseObjCmd`
    /// (`generic/tclCmdAH.c`) scans no leading flags at all, always matches
    /// with `Tcl_StringMatch` glob semantics, and consumes an exact literal
    /// `in` between the subject and the clauses.  `default` is honoured
    /// wherever it appears rather than only last, and there is no `-`
    /// fall-through body.
    pub const CASE: Self = Self {
        subject_args: 1,
        two_arg_optionless_surface: None,
        regex_option: None,
        exact_option: None,
        glob_option: None,
        nocase_option: None,
        end_options_option: None,
        fallthrough_body: None,
        value_options_require_regex: &[],
        special_match_options: &[],
        clause_flags: &[],
        clause_regex_flag: None,
        clause_value_flags: &[],
        clause_end_options_flag: None,
        clause_force_inline_flag: None,
        clause_force_list_flag: None,
        clause_force_list_shape: None,
        allow_omitted_final_body: false,
        keyword_patterns: &["default"],
        // `Tcl_CaseObjCmd` records a `default` clause's body as it walks and
        // only falls back to it after every pattern has failed, so a
        // non-final `default` is honoured — unlike `switch`, whose manpage
        // requires it last.
        keyword_patterns_require_final: false,
        optional_subject_separator: Some("in"),
        warn_unbraced_bodies: true,
    };

    /// The Expect `expect { ?-flags? pat body … }` shape.
    pub const EXPECT: Self = Self {
        subject_args: 0,
        two_arg_optionless_surface: None,
        regex_option: None,
        exact_option: None,
        glob_option: None,
        nocase_option: Some("-nocase"),
        end_options_option: Some("--"),
        fallthrough_body: None,
        // `-timeout` / `-i` are command-level, value-taking options.  In
        // Expect 5.45.4, after either option and its value, one braced word is
        // the final action-less *pattern*, not a clause list: `expect -timeout
        // 5 {ready {action}}` does not execute `action`.  A clause list is
        // only the sole argument (`expect { … }`) or the exact first-argument
        // `-brace { … }` shape. Keep these options out of a speculative
        // list-remainder grammar so every consumer abstains consistently.
        value_options_require_regex: &[],
        special_match_options: &[],
        clause_flags: &[
            "-glob",
            "-regexp",
            "-exact",
            "-notransfer",
            "-nocase",
            "-i",
            "-indices",
            "-iread",
            "-timestamp",
            "-timeout",
            "-nobrace",
            "--",
        ],
        clause_regex_flag: Some("-regexp"),
        clause_value_flags: &["-timeout", "-i"],
        clause_end_options_flag: Some("--"),
        clause_force_inline_flag: Some("-nobrace"),
        clause_force_list_flag: Some("-brace"),
        clause_force_list_shape: Some(CaseForceListShape::FirstArgOnlyRemainder),
        allow_omitted_final_body: true,
        keyword_patterns: &["timeout", "eof", "default", "full_buffer", "null"],
        keyword_patterns_require_final: false,
        optional_subject_separator: None,
        warn_unbraced_bodies: false,
    };

    /// Parse and validate this case-list command's option/subject/clause
    /// layout. The two-argument switch exception, `--`, option values, and
    /// pair arity are all registry-owned here.
    #[must_use]
    #[allow(clippy::too_many_lines)] // option and outer-shape grammar are one descriptor operation
    pub fn invocation(
        self,
        args: &[&str],
        options: &[&crate::hover::OptionSpec],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<CaseInvocation> {
        let mut mode = CaseMatchMode::Exact;
        let mut saw_match_mode = false;
        let mut nocase = false;
        let mut saw_regex_value_option = false;
        let mut i = 0usize;
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: self.clause_flags,
            clause_value_flags: self.clause_value_flags,
        };
        let mut outer_options_ended = false;
        // Segmenters pass a braced Expect clause list as one content word;
        // its first element may itself begin with `-re`/`-timeout`, which is
        // clause grammar, not a command-level option.  A flag-bearing,
        // subject-less case list with one remaining word is therefore already
        // in list form and must bypass the command-option scan.
        let per_clause_flags = self.subject_args == 0 && !self.clause_flags.is_empty();
        let sole_clause_list = per_clause_flags && args.len() == 1;
        let two_arg_optionless = self.two_arg_optionless_form_is_available(args, dialect);
        if !(sole_clause_list || self.subject_args == 1 && args.len() == 2 && two_arg_optionless) {
            while let Some(word) = args.get(i).copied() {
                if !word.starts_with('-') {
                    break;
                }
                let option = Self::resolve_option(options, word);
                // A subjectless descriptor has two option vocabularies at
                // this position. Its clause flags own the first inline
                // clause, including `--`; only a matching, value-taking
                // command option remains an outer option. This keeps `-re`
                // and friends attached to their pattern while still making
                // `-timeout 5 {pattern body}` an action-less final pattern.
                // The outer-shape selectors are exact descriptor spellings:
                // `-brace` is deliberately not a prefix abbreviation.
                let clause_flag = shape.resolve_flag(word);
                let force_selector = self.clause_force_inline_flag == Some(word)
                    || self
                        .clause_force_list_shape
                        .is_some_and(|shape| shape.matches(args, self.clause_force_list_flag, i));
                if self.subject_args == 0
                    && (force_selector
                        || clause_flag.is_some_and(|flag| {
                            !shape.flag_takes_value(flag)
                                || !option.is_some_and(OptionSpec::takes_value)
                        }))
                {
                    break;
                }
                // `--` is a descriptor-owned terminator, not necessarily a
                // documented option entry. Recognise it before looking up a
                // command option so the following hyphenated subject remains
                // positional (notably `switch -- -x {...}`).
                if self.end_options_option == Some(word) {
                    i += 1;
                    outer_options_ended = true;
                    break;
                }
                let option = option?;
                let option_name = option.name;
                if self.exact_option == Some(option_name) {
                    if saw_match_mode {
                        return None;
                    }
                    saw_match_mode = true;
                    mode = CaseMatchMode::Exact;
                    i += 1;
                } else if self.glob_option == Some(option_name) {
                    if saw_match_mode {
                        return None;
                    }
                    saw_match_mode = true;
                    mode = CaseMatchMode::Glob;
                    i += 1;
                } else if self.regex_option == Some(option_name) {
                    if saw_match_mode {
                        return None;
                    }
                    saw_match_mode = true;
                    mode = CaseMatchMode::Regexp;
                    i += 1;
                } else if self.nocase_option == Some(option_name) {
                    nocase = true;
                    i += 1;
                } else {
                    let consumed = option.value_word_count(args, i);
                    if consumed == 0 {
                        if self.special_match_options.contains(&option_name) {
                            if saw_match_mode {
                                return None;
                            }
                            saw_match_mode = true;
                            mode = CaseMatchMode::Other;
                            i += 1;
                            continue;
                        }
                        return None;
                    }
                    saw_regex_value_option |=
                        self.value_options_require_regex.contains(&option_name);
                    i += 1 + consumed;
                }
            }
        }
        if saw_regex_value_option && mode != CaseMatchMode::Regexp {
            return None;
        }
        // Descriptor-specialised match modes (currently Tcl 9.1's integer
        // switch mode) compare values intrinsically and cannot be combined
        // with the string-only case-folding modifier.
        if mode == CaseMatchMode::Other && nocase {
            return None;
        }
        // Selectors are descriptor syntax, not ordinary command options.
        // They are considered only while the outer scan is active: after an
        // outer `--`, an option-shaped word is positional data instead.
        let force_list = !outer_options_ended
            && self
                .clause_force_list_shape
                .is_some_and(|shape| shape.matches(args, self.clause_force_list_flag, i));
        let force_inline = !outer_options_ended
            && self
                .clause_force_inline_flag
                .is_some_and(|flag| args.get(i).copied() == Some(flag));
        if force_list {
            i += 1;
        } else if force_inline
            && args
                .get(i)
                .and_then(|word| shape.resolve_flag(word))
                .is_none()
        {
            // When the inline selector is itself a clause flag (Expect's
            // `-nobrace`), leave it to `inline_clauses` so consumers retain
            // its decorator location. A selector outside that vocabulary is
            // shape-only and is not part of the first clause.
            i += 1;
        }
        let subject_index = (self.subject_args == 1).then_some(i);
        i += usize::from(self.subject_args);
        // A descriptor-declared literal separator between the subject and the
        // clauses (Tcl 8.x `case string ?in? …`).  Matched exactly, mirroring
        // `Tcl_CaseObjCmd`'s own `strcmp(arg, "in")`: a substituted word at
        // this position is a first pattern, not the separator.
        if self
            .optional_subject_separator
            .is_some_and(|sep| args.get(i).copied() == Some(sep))
        {
            i += 1;
        }
        if i >= args.len() {
            return None;
        }
        let remaining = args.len() - i;
        if force_list {
            if remaining != 1 || !self.valid_clause_list(args[i]) {
                return None;
            }
            Some(CaseInvocation {
                subject_index: None,
                clause_list_index: Some(i),
                inline_clause_start: None,
                mode,
                nocase,
            })
        } else if remaining == 1 && !force_inline && (sole_clause_list || self.subject_args == 1) {
            // Validate the list grammar first, then its registry-declared
            // clause grammar.  Strict pattern/body parity is sufficient for
            // `switch`, but not for Expect: a clause may start with `-re` or
            // a value-taking `-timeout 5`, so counting raw list elements
            // rejects valid clauses and leaves generic consumers unable to
            // recurse their bodies.
            if !self.valid_clause_list(args[i]) {
                return None;
            }
            Some(CaseInvocation {
                subject_index,
                clause_list_index: Some(i),
                inline_clause_start: None,
                mode,
                nocase,
            })
        } else if per_clause_flags {
            // Expect's flags belong to the following pattern, not to the
            // command as a whole.  Parse them from this descriptor before
            // pairing bodies, including unique flag abbreviations.
            self.inline_clauses(args, i)?;
            Some(CaseInvocation {
                subject_index,
                clause_list_index: None,
                inline_clause_start: Some(i),
                mode,
                nocase,
            })
        } else if remaining >= 2
            && remaining.is_multiple_of(2)
            && self.fallthrough_body != args.last().copied()
        {
            Some(CaseInvocation {
                subject_index,
                clause_list_index: None,
                inline_clause_start: Some(i),
                mode,
                nocase,
            })
        } else {
            None
        }
    }

    /// Whether the descriptor's two-word optionless form may assign a body
    /// role for `dialect`. A non-option subject remains valid in every
    /// release; the gate only applies when Tcl 8.4 would scan an option-like
    /// first word and consume it.
    #[must_use]
    pub(crate) fn two_arg_body_roles_allowed(
        self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> bool {
        self.two_arg_optionless_form_is_available(args, dialect)
    }

    fn two_arg_optionless_form_is_available(
        self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> bool {
        args.len() != 2
            || self.subject_args != 1
            || args.first().is_none_or(|word| !word.starts_with('-'))
            || self
                .two_arg_optionless_surface
                .is_none_or(|available| surface_admits(available, dialect.as_ref()))
    }

    /// Adjust a command's static trailing-word reservation for the
    /// optionless two-word form. The regular switch shape reserves its
    /// subject and clause-list words from option scanning, but Tcl 8.4 still
    /// scans both words when that shape begins with an option-like subject.
    #[must_use]
    pub(crate) fn option_scan_reserved_trailing_words(
        self,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
        default: usize,
    ) -> usize {
        if self.two_arg_body_roles_allowed(args, dialect) {
            default
        } else {
            0
        }
    }

    /// Resolve one outer option exactly or by its unique declared prefix.
    ///
    /// The result is an `OptionSpec`, rather than just its spelling, because
    /// aliases and canonical names are one option for ambiguity purposes.
    /// That makes descriptor consumers agree with the registry's usual
    /// `KeywordTable` option semantics without treating two aliases of one
    /// option as two ambiguous candidates.
    fn resolve_option<'a>(options: &'a [&'a OptionSpec], word: &str) -> Option<&'a OptionSpec> {
        if let Some(option) = options.iter().copied().find(|option| option.matches(word)) {
            return Some(option);
        }
        if word.len() < 2 {
            return None;
        }
        let mut matches = options.iter().copied().filter(|option| {
            (option.name.starts_with(word)
                || option.aliases.iter().any(|alias| alias.starts_with(word)))
                && option
                    .min_abbrev
                    .is_none_or(|minimum| word.len() >= usize::from(minimum))
        });
        let option = matches.next()?;
        matches.next().is_none().then_some(option)
    }

    fn valid_clause_list(self, text: &str) -> bool {
        let Ok(elements) = tcl_syntax::list::split_list(text) else {
            return false;
        };
        // Tcl's case-list commands require at least one pattern clause.  An
        // empty value is a syntactically valid Tcl list, but not a valid
        // `switch`/Expect case list (Tcl reports wrong # args rather than
        // evaluating a zero-clause dispatch).  Keep that distinction in the
        // shared descriptor boundary so every semantic consumer abstains;
        // formatter recovery can still present an incomplete list locally.
        if elements.is_empty() {
            return false;
        }
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: self.clause_flags,
            clause_value_flags: self.clause_value_flags,
        };
        let clauses = tcl_syntax::case_list::split_case_list(text, &shape);
        !clauses.iter().enumerate().any(|(index, clause)| {
            !clause.valid
                || clause.pattern.is_none()
                || (clause.body.is_none()
                    && !(self.allow_omitted_final_body && index + 1 == clauses.len()))
        }) && !elements.last().is_some_and(|body| {
            self.fallthrough_body
                .is_some_and(|marker| body.as_ref() == marker)
        })
    }

    /// Parse descriptor-declared inline pattern/action clauses.  A future
    /// case-list command gets the same body locations by changing only its
    /// descriptor; consumers never infer flag or value arity themselves.
    #[must_use]
    pub fn inline_clauses(self, args: &[&str], start: usize) -> Option<Vec<InlineCaseClause>> {
        let shape = tcl_syntax::case_list::CaseListShape {
            clause_flags: self.clause_flags,
            clause_value_flags: self.clause_value_flags,
        };
        let mut clauses = Vec::new();
        let mut i = start;
        while i < args.len() {
            let mut mode = CaseMatchMode::Exact;
            let mut options_ended = false;
            let mut flag_indices = Vec::new();
            while let Some(word) = args.get(i).copied() {
                if options_ended || !word.starts_with('-') {
                    break;
                }
                let flag = shape.resolve_flag(word)?;
                flag_indices.push(i);
                i += 1;
                if self.clause_end_options_flag == Some(flag) {
                    options_ended = true;
                    continue;
                }
                if shape.flag_takes_value(flag) {
                    args.get(i)?;
                    i += 1;
                }
                if self.clause_regex_flag == Some(flag) {
                    mode = CaseMatchMode::Regexp;
                }
            }
            let pattern_index = i;
            args.get(pattern_index)?;
            let body_index = pattern_index + 1;
            let body_index = args.get(body_index).is_some().then_some(body_index);
            if body_index.is_none() && !self.allow_omitted_final_body {
                return None;
            }
            clauses.push(InlineCaseClause {
                flag_indices,
                pattern_index,
                body_index,
                mode,
            });
            let Some(body_index) = body_index else {
                break;
            };
            i = body_index + 1;
        }
        (!clauses.is_empty()).then_some(clauses)
    }

    /// Whether `pattern` is a special keyword at this clause position.
    #[must_use]
    pub fn is_keyword_pattern(self, pattern: &str, index: usize, total: usize) -> bool {
        self.keyword_patterns.contains(&pattern)
            && (!self.keyword_patterns_require_final || index + 1 == total)
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

/// One operand of an [`OptionRelation`] — what a term names in an invocation.
///
/// The four shapes are the four things a real library's option table talks
/// about: an option, an option *carrying a particular value*, a positional
/// argument, and a positional argument carrying a particular value. `bibtex`
/// needs the first and the third (`-command` requires `-channel`, and excludes
/// the inline `text` word); `struct::tree walk` needs the second
/// (`-order in` is illegal with `-type bfs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionTerm {
    /// The option is present, whatever value it carries (`-channel`).
    Option(&'static str),
    /// The option is present **and** its first value word is this literal
    /// (`-type bfs`). Canonical option name first, value second.
    OptionValue(&'static str, &'static str),
    /// A positional argument exists at this 0-based index, counted *after*
    /// the leading option run.
    Argument(u8),
    /// The positional argument at this index is this literal.
    ArgumentValue(u8, &'static str),
}

impl RelationTermKind for OptionTerm {
    fn collective_noun() -> &'static str {
        "Options"
    }

    fn describe(self) -> String {
        match self {
            Self::Option(name) => name.to_owned(),
            Self::OptionValue(name, value) => format!("{name} {value}"),
            Self::Argument(index) => format!("argument {}", index + 1),
            Self::ArgumentValue(index, value) => format!("argument {} = {value}", index + 1),
        }
    }

    /// A bare `-name` is the option; a braced `{-name value}` pins its value;
    /// `{arg N}` is the positional at `N`, and `{arg N value}` pins its value.
    fn spelling(self) -> String {
        match self {
            Self::Option(name) => name.to_owned(),
            Self::OptionValue(name, value) => format!("{{{name} {value}}}"),
            Self::Argument(index) => format!("{{arg {index}}}"),
            Self::ArgumentValue(index, value) => format!("{{arg {index} {value}}}"),
        }
    }
}

impl OptionTerm {
    /// The canonical option name this term names, when it names one.
    #[must_use]
    pub const fn option_name(self) -> Option<&'static str> {
        match self {
            Self::Option(name) | Self::OptionValue(name, _) => Some(name),
            Self::Argument(_) | Self::ArgumentValue(_, _) => None,
        }
    }
}

/// A registry-declared relation between an invocation's options and arguments
/// — the option domain's instance of the one relation mechanism (R11).
pub type OptionRelation = Relation<OptionTerm>;

/// One invocation as the relation checker reads it.
///
/// Built once per call site by the analyser from the leading-option walk it
/// already performs, and borrowed by every relation on that command — the
/// scan is not repeated per relation.
#[derive(Debug, Clone, Copy)]
pub struct OptionFacts<'a> {
    /// The canonical name of each declared option the call supplied, in call
    /// order, paired with its first value word when that word is a literal.
    pub options: &'a [(&'static str, Option<&'a str>)],
    /// The positional words after the leading option run, `None` where a word
    /// is not statically known.
    pub positionals: &'a [Option<&'a str>],
    /// Whether the invocation was read to its end with nothing unreadable in
    /// it — no `{*}` expansion, and no substituted word where an option could
    /// have been. **Only** this licenses proving a term *absent*.
    pub complete: bool,
}

/// What a call site says about one option — three cases, not two, because
/// "supplied with a value nobody can read" answers differently from both
/// "absent" and "supplied with this value".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionPresence<'a> {
    /// The call did not supply the option here.
    Absent,
    /// Supplied, but its value word is not statically known.
    PresentUnknownValue,
    /// Supplied, carrying this literal value.
    PresentWith(&'a str),
}

impl OptionFacts<'_> {
    /// What the call said about `name`.
    fn option_presence(&self, name: &str) -> OptionPresence<'_> {
        match self.options.iter().find(|(option, _)| *option == name) {
            None => OptionPresence::Absent,
            Some((_, None)) => OptionPresence::PresentUnknownValue,
            Some((_, Some(value))) => OptionPresence::PresentWith(value),
        }
    }

    /// `Yes` when the word is proven to be `wanted`, `No` when proven not to
    /// be, `Unknown` when the word is not statically known — the shared
    /// answer for an option value and a positional value alike.
    fn matches_literal(word: Option<&str>, wanted: &str) -> TermHolds {
        match word {
            Some(actual) if actual == wanted => TermHolds::Yes,
            Some(_) => TermHolds::No,
            None => TermHolds::Unknown,
        }
    }

    /// The verdict for a term the call did not supply: proven absent only
    /// when the invocation was read to its end.
    const fn absent(&self) -> TermHolds {
        if self.complete {
            TermHolds::No
        } else {
            TermHolds::Unknown
        }
    }
}

impl RelationFactSource<OptionTerm> for OptionFacts<'_> {
    /// Presence is provable whatever the value is; absence needs `complete`.
    fn holds(&self, term: OptionTerm) -> TermHolds {
        match term {
            OptionTerm::Option(name) => match self.option_presence(name) {
                OptionPresence::Absent => self.absent(),
                OptionPresence::PresentUnknownValue | OptionPresence::PresentWith(_) => {
                    TermHolds::Yes
                }
            },
            OptionTerm::OptionValue(name, wanted) => match self.option_presence(name) {
                OptionPresence::Absent => self.absent(),
                // Present, but its value could be anything at run time —
                // including `wanted`.
                OptionPresence::PresentUnknownValue => TermHolds::Unknown,
                OptionPresence::PresentWith(actual) => Self::matches_literal(Some(actual), wanted),
            },
            OptionTerm::Argument(index) => {
                if usize::from(index) < self.positionals.len() {
                    TermHolds::Yes
                } else {
                    self.absent()
                }
            }
            OptionTerm::ArgumentValue(index, wanted) => {
                match self.positionals.get(usize::from(index)) {
                    Some(word) => Self::matches_literal(*word, wanted),
                    None => self.absent(),
                }
            }
        }
    }
}

impl OptionRelation {
    /// Every option name this relation mentions, subject included — what a
    /// consumer needs to decide whether the relation is worth evaluating at
    /// all.
    #[must_use]
    pub fn option_names(&self) -> Vec<&'static str> {
        self.mentioned_terms()
            .into_iter()
            .filter_map(OptionTerm::option_name)
            .collect()
    }
}

/// Where in an invocation a command's declared options may appear — the fact
/// the E-R14 relation checker needs to find them.
///
/// Not a style preference: core Tcl's C option loops almost all `break` on the
/// first word that is not a declared option (`Tcl_GlobObjCmd`, `source`,
/// `lsort`), so a later option-shaped word there is a *positional*, and
/// reading it as an option would invent a conflict the interpreter never
/// raises. A script-level command like `http::geturl`, whose parser is
/// `foreach {flag value} $args` after taking the URL, is the other shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptionPlacement {
    /// A leading run: option parsing stops at the first non-option word.
    /// The default, and what every core Tcl command does.
    #[default]
    Leading,
    /// Options are recognised anywhere in the argument list, after and
    /// between positional words, up to an explicit `--`.
    Anywhere,
}

impl OptionPlacement {
    /// Every placement, in declaration order — the loader's and the studio's
    /// enumeration.
    pub const ALL: &'static [(&'static str, Self)] =
        &[("Leading", Self::Leading), ("Anywhere", Self::Anywhere)];

    /// The DSL spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Leading => "Leading",
            Self::Anywhere => "Anywhere",
        }
    }
}

/// Where a `constraints` hook's report points.
///
/// The hook's `invalid SLOT MESSAGE` verb names one of these: an option
/// spelling, a 0-based positional index (`arg 2`), or the command word itself
/// when the report is about the invocation as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintSlot {
    /// A canonical option name.
    Option(&'static str),
    /// A 0-based positional index, counted after the leading option run.
    Argument(u8),
    /// The command word — for a report about the whole invocation.
    Command,
}

/// One thing a `constraints` hook reported.
///
/// Deliberately smaller than [`RelationViolation`]: a hook says *where* and
/// *what*, and the analyser owns the diagnostic code and the span. `conflict`
/// selects W147 ("these cannot go together") over the default W152 ("this one
/// needs that one"), which is the only structural choice a hook makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintReport {
    /// The slot the report points at.
    pub slot: ConstraintSlot,
    /// The message, as the hook wrote it.
    pub message: String,
    /// Report as an exclusion (W147) rather than a requirement (W152).
    pub conflict: bool,
}

/// What the option-relation checker did, for the P-B measurement.
///
/// Principle P-B's claim — "every relation the declarative vocabulary can
/// express is checked natively with no VM entry" — is only worth as much as
/// the number behind it, so the checker counts rather than asserting. Three
/// thread-local counters, incremented once per call site: cheap enough to
/// leave on (they are `Cell<u64>` adds on a path that already allocates a
/// `Vec` per violation), and the only way a regression that starts entering
/// the VM shows up as anything but a slowdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RelationCheckStats {
    /// Call sites whose walk found at least one declared option word — the
    /// population the check is *about*.
    pub option_bearing_sites: u64,
    /// Of those, the sites whose command declares at least one relation or a
    /// `constraints` hook, and so were actually judged.
    pub judged_sites: u64,
    /// Relations evaluated natively across those call sites — the work that
    /// never enters a VM.
    pub relations_evaluated: u64,
    /// Call sites that reached a `constraints` hook. **This is the number the
    /// design is about**: it must stay 0 for a corpus of shipped commands.
    pub hook_entries: u64,
}

thread_local! {
    static RELATION_STATS: std::cell::Cell<RelationCheckStats> =
        const { std::cell::Cell::new(RelationCheckStats {
            option_bearing_sites: 0,
            judged_sites: 0,
            relations_evaluated: 0,
            hook_entries: 0,
        }) };
}

/// Record one call site the option walk read.
///
/// `judged` is false for a call site whose command declares no relation at
/// all — it is still an option-bearing site, and counting it is what makes
/// the ratio meaningful.
pub fn note_relation_check(
    option_bearing: bool,
    judged: bool,
    relations_evaluated: usize,
    entered_hook: bool,
) {
    RELATION_STATS.with(|stats| {
        let mut current = stats.get();
        current.option_bearing_sites += u64::from(option_bearing);
        current.judged_sites += u64::from(judged);
        current.relations_evaluated += relations_evaluated as u64;
        current.hook_entries += u64::from(entered_hook);
        stats.set(current);
    });
}

/// This thread's option-relation counters.
#[must_use]
pub fn relation_check_stats() -> RelationCheckStats {
    RELATION_STATS.with(std::cell::Cell::get)
}

/// Zero this thread's option-relation counters.
pub fn reset_relation_check_stats() {
    RELATION_STATS.with(|stats| stats.set(RelationCheckStats::default()));
}

/// A `constraints` hook: the E-R14 escape hatch.
///
/// **Called only when a spec declares one and the declarative relations found
/// nothing to report** (principle P-B). A shipped spec's hook is an ordinary
/// Rust function; a pack's is a Tcl body on the sandboxed VM, reached through
/// [`crate::pack_hooks::constraints_fn`] — the consumer cannot tell which.
///
/// Silence (an empty vector) means "no report", which is also what a crashed,
/// quarantined, budget-blown or hostless pack hook answers.
pub type ConstraintsHook = fn(&OptionFacts<'_>) -> Vec<ConstraintReport>;

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
    /// its owning package's releases (issue #1627).
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

    /// Formatter **presentation** overrides, keyed by 0-based argument index
    /// — how an argument should be *laid out*, as distinct from what
    /// [`Self::arg_roles`] says it *is* (issue #1186).
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
    /// knowing the command (issue #1185). See [`crate::repeated`].
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

    /// Target-neutral completion semantics for this command.
    ///
    /// A resolved subcommand or invocation form can supply a more-specific
    /// descriptor. `None` deliberately leaves the invocation conservative.
    pub completion: Option<crate::completion::CompletionDescriptor>,

    /// Whether equal evaluated arguments are sufficient to reproduce this
    /// command's result. `None` is conservative, not an assertion of purity.
    pub result_stability: Option<crate::result_stability::ResultStability>,

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
    /// [`crate::bpf_op`].  `None` for every non-BPF command.
    pub bpf_op: Option<&'static crate::bpf_op::BpfOpSpec>,

    /// `TclVM` bytecode codegen hook ID — picks the per-command
    /// emitter inside `tcl_compiler::codegen::emitter::bytecoded`
    /// (the path that matches C Tcl 9's bytecode output).
    /// `None` means the generic invoke emitter handles this command.
    pub codegen_hook: Option<CodegenHookId>,

    /// Inline (value-position / catch-body) bytecode codegen hook ID —
    /// picks the per-command emitter on the compiler's
    /// command-substitution and catch-body paths
    /// (`tcl_compiler::codegen::cmd_subst` /
    /// `tcl_compiler::codegen::control_flow`). `None` means those
    /// paths use their generic invoke emission for this command.
    pub inline_codegen_hook: Option<InlineCodegenHookId>,

    /// Target-neutral native lowering shape — which native code shape the
    /// executable-IR lowering (`tcl_compiler::native_lowering`) gives an
    /// invocation of this command once its registry facts are resolved. A
    /// sibling of [`Self::lowering_hook`], [`Self::codegen_hook`], and
    /// [`Self::inline_codegen_hook`]. `None` means the generic argv
    /// invocation ([`crate::native_lowering::NativeLowering::Generic`]); read
    /// it through [`Self::native_lowering`].
    pub native_lowering: Option<crate::native_lowering::NativeLowering>,

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
    /// shipped spec names directly (centralisation ledger C8).
    pub command_table_effect: Option<CommandTableEffect>,

    /// Structured side-effect declarations.
    pub side_effects: &'static [SideEffect],

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

    /// Layer-based iRules event requirements (transport / profiles /
    /// `also_in` / side / init-only / flow / capability) used by the
    /// IRULE1001 event-validity check. `None` = no requirement.
    pub event_requires: Option<crate::events::EventRequires>,

    /// Argument-prefix-specific iRules event contracts. These override
    /// [`Self::event_requires`] for a matching literal call form, allowing a
    /// registry spec to describe commands whose read and configuration forms
    /// have different legal event surfaces without a compiler-side name check.
    pub event_requirement_forms: &'static [EventRequirementForm],

    /// Data-collection lifecycle fact for an iRules protocol command.
    ///
    /// Consumers use this descriptor rather than inferring behaviour from a
    /// command spelling such as `HTTP::payload`. `None` means the command is
    /// not part of a collect/release/payload lifecycle.
    pub data_collection: Option<crate::events::DataCollectionOperation>,

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

    /// Typed relations between this command's options and arguments — the
    /// declarative half of E-R14.  Data for generic invocation validation,
    /// not a command-specific analyser rule, and checked natively with no
    /// hook and no VM entry.
    pub option_relations: &'static [OptionRelation],

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

    /// Option flags whose value carries a secret (e.g. `-password`,
    /// `-headers`) — drives credential-exposure checks. Empty = none.
    pub credential_options: &'static [&'static str],

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
    /// provable static alias/rename of it bind exactly like the bare spelling
    /// — issue #1185.  A member-body-only installer such as snit's `install`
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
    /// naming a command — issue #1707.  `None` = the command takes part in no
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

/// Count the leading words of `args` that are declared options in
/// `options` — the `?-flag? ?-opt val?` prefix many commands/subcommands
/// accept before their positional arguments. Shared core of
/// [`CommandSpec::leading_option_word_count`] /
/// [`SubCommand::leading_option_word_count`], so every consumer that needs
/// "where do the true positionals start" (an `arg_types` hint, a
/// [`CommandSpec::defines_command_at`] / [`SubCommand::defines_command_at`]
/// name lookup, …) agrees with the one declared option table instead of
/// assuming a fixed offset that a `-safe`/`--`-style flag silently shifts.
///
/// A word counts as an option when it begins with `-`, is at least two
/// characters, and resolves to exactly one declared option — by exact name,
/// declared alias, or unique prefix (C Tcl's option tables accept any
/// unambiguous prefix of two or more characters: `string map -noc …`
/// behaves like `-nocase`, verified in `tclCmdMZ.c`'s
/// `strncmp(string, "-nocase", length)` loops). A value-taking option also
/// consumes its following word. Counting stops at the first
/// non-option-shaped word; an empty `options` table returns 0
/// unconditionally, so purely positional commands/subcommands are
/// untouched. Dialect-agnostic — callers that need dialect gating (e.g. an
/// option added in a later Tcl release) should filter `options` first.
/// Build an option [`KeywordTable`] from an option iterator, applying the
/// dialect and lifecycle filters and including declared aliases (Tcl's own
/// option table holds them, so `-bd` prefix-matches exactly like
/// `-borderwidth`).
fn option_table_from<'a>(
    options: impl Iterator<Item = &'a OptionSpec>,
    dialect: Option<SurfaceQuery<'_>>,
    parent_surface: Option<&'static [SpecSurface]>,
    package_version: Option<&str>,
    prefix_matching: PrefixMatching,
) -> KeywordTable<'static> {
    let mut keywords: Vec<Keyword<'static>> = Vec::new();
    for opt in options {
        if !opt.supports_dialect(dialect, parent_surface)
            || !opt.available_for_version(package_version)
        {
            continue;
        }
        for name in std::iter::once(opt.name).chain(opt.aliases.iter().copied()) {
            if !keywords.iter().any(|k| k.name == name) {
                keywords.push(Keyword {
                    name,
                    min_abbrev: opt.min_abbrev,
                });
            }
        }
    }
    KeywordTable::from_keywords(keywords, prefix_matching)
}

/// Resolve one leading option word against its descriptor table.
///
/// Tcl accepts an exact canonical spelling or alias, then an unambiguous
/// abbreviation of either spelling.  Keeping that rule beside
/// [`OptionSpec`] means the layout resolvers, option-value role walk, and
/// declaration helpers all consume precisely the words Tcl's option parser
/// consumes; no command consumer needs its own list of value-taking switches.
#[must_use]
pub fn resolve_option_prefix<'a>(options: &'a [OptionSpec], word: &str) -> Option<&'a OptionSpec> {
    resolve_option_prefix_with(options, word, PrefixMatching::Enabled)
}

/// [`resolve_option_prefix`] with the table's declared prefix policy.
#[must_use]
pub fn resolve_option_prefix_with<'a>(
    options: &'a [OptionSpec],
    word: &str,
    prefix_matching: PrefixMatching,
) -> Option<&'a OptionSpec> {
    if let Some(exact) = options.iter().find(|option| option.matches(word)) {
        return Some(exact);
    }
    if !prefix_matching.accepts_prefixes() || !word.starts_with('-') || word.len() < 2 {
        return None;
    }
    let mut matched: Option<&OptionSpec> = None;
    for option in options {
        if std::iter::once(option.name)
            .chain(option.aliases.iter().copied())
            .any(|spelling| spelling.starts_with(word))
        {
            match matched {
                None => matched = Some(option),
                Some(previous) if std::ptr::eq(previous, option) => {}
                Some(_) => return None,
            }
        }
    }
    matched
}

/// Return the index immediately after a declared leading option prefix.
///
/// The descriptor-driven walk honours unique abbreviations, aliases, each
/// option's actual value arity, and a declared `--` terminator.  It is the
/// common layout primitive for commands whose positional grammar starts only
/// after their option prefix.
#[must_use]
pub fn leading_option_word_count(options: &[OptionSpec], args: &[&str]) -> usize {
    leading_option_word_count_with(options, args, PrefixMatching::Enabled)
}

/// [`leading_option_word_count`] with the table's declared prefix policy.
#[must_use]
pub fn leading_option_word_count_with(
    options: &[OptionSpec],
    args: &[&str],
    prefix_matching: PrefixMatching,
) -> usize {
    if options.is_empty() {
        return 0;
    }
    let mut i = 0;
    while let Some(&word) = args.get(i) {
        if !word.starts_with('-') || word.len() < 2 {
            break;
        }
        let resolved = resolve_option_prefix_with(options, word, prefix_matching);
        let Some(opt) = resolved else { break };
        i += 1 + opt.value_word_count(args, i);
        if opt.name == "--" {
            // `--` ends option parsing (Tcl's own `Tcl_ParseArgsObjv`-style
            // convention: every subsequent word — even one that looks like a
            // declared option, e.g. `interp create -- -safe` — is positional).
            // Without this, the loop above would keep matching option-shaped
            // words past the terminator and swallow a literal name meant to
            // land in a positional slot such as `defines_command_at`.
            break;
        }
    }
    i
}

/// The trailing run of `?placeholder?` words in a synopsis, `?` stripped.
///
/// A synopsis is a whitespace-separated word list whose optional arguments
/// are wrapped in `?…?` (`catch script ?resultVarName? ?optionsVarName?`).
/// Scanning from the right and stopping at the first word that is not so
/// wrapped yields exactly the words a caller may append to a call that
/// supplies every earlier word — the question a splice-a-trailing-argument
/// quick-fix asks.  A variadic tail (`?arg ...?`, `?arg arg ...?`) is not a
/// fixed slot list, so a placeholder containing whitespace or an ellipsis
/// terminates the run rather than being offered as a name.
fn optional_trailing_placeholders(synopsis: &'static str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = synopsis
        .split_whitespace()
        .rev()
        .map_while(|word| {
            let inner = word.strip_prefix('?')?.strip_suffix('?')?;
            (!inner.is_empty() && inner != "..." && !inner.contains("...")).then_some(inner)
        })
        .collect();
    names.reverse();
    names
}

impl CommandSpec {
    /// Default value for all fields — used with `..CommandSpec::DEFAULT`.
    pub const DEFAULT: Self = Self {
        name: "",
        traits: Traits::empty(),
        surface: None,
        arity: Arity::any(),
        arity_windows: &[],
        arg_roles: &[],
        arg_role_resolver: None,
        arg_presentation: &[],
        repeated_args: &[],
        frame_effect: None,
        clause_shape_check: None,
        command_prefixes: &[],
        command_prefix_resolver: None,
        script_timing_resolver: None,
        callback_taint_inputs: &[],
        return_type: None,
        return_type_hook: None,
        var_write_typing: VarWriteTyping::ReturnValue,
        return_elements: None,
        var_elements_effect: None,
        representation_effect: None,
        arg_types: &[],
        subcommands: &[],
        prefix_matching: PrefixMatching::Enabled,
        allow_unknown_subcommands: false,
        default_form_first_word: None,
        hover: None,
        forms: &[],
        command_forms: &[],
        semantic_operation: None,
        completion: None,
        result_stability: None,
        assigns_variable_at: None,
        safe_on_uninit: None,
        const_fold: None,
        const_fold_versioned: None,
        lowering_hook: None,
        bpf_op: None,
        codegen_hook: None,
        inline_codegen_hook: None,
        native_lowering: None,
        analyser_hook: None,
        command_table_effect: None,
        side_effects: &[],
        world_effects: None,
        state_transitions: None,
        dispatch_dependencies: None,
        literal_argument_validator: None,
        inferred_storage_type: None,
        required_package: None,
        tk_geometry: None,
        excluded_events: &[],
        unsafe_command: false,
        closed_value_args: &[],
        event_requires: None,
        event_requirement_forms: &[],
        data_collection: None,
        side_switch_target: None,
        event_handler_priority: None,
        irules_top_level_effect: None,
        options: &[],
        option_relations: &[],
        constraints: None,
        option_placement: OptionPlacement::Leading,
        reserved_trailing_words: 0,
        arg_values: &[],
        versioned_arg_values: &[],
        body_kind: BodyKind::Plain,
        body_arg_implicit_args: 0,
        taint_output_sink: None,
        taint_output_sink_subcommands: &[],
        taint_log_sink: None,
        taint_network_sink_args: None,
        taint_code_sink_args: None,
        taint_interp_eval_subcommands: &[],
        taint_source: None,
        taint_transform: None,
        taint_double_encode_colour: None,
        taint_sink_safe_colour: None,
        taint_sink_gate: None,
        credential_options: &[],
        sensitive_headers: &[],
        setter_constraints: &[],
        pattern_type: None,
        pattern_arg_resolver: None,
        format_string_type: None,
        tcllib_package: None,
        lifecycle: Lifecycle::UNSPECIFIED,
        warn_missing_import: true,
        is_namespace_exported: false,
        xc_translatable: None,
        deprecated_replacement: None,
        deprecated_replacement_drop_in: false,
        byte_array_payload: None,
        byte_array_effect: crate::byte_array_effect::ByteArrayEffect::None,
        definition_body: None,
        manufacturer_methods: &[],
        case_list: None,
        object_class: None,
        defines_symbol: None,
        body_scope: None,
        binds_handle: None,
        remote_method: None,
        creates_instance_at: None,
        defines_command_at: None,
        context_gate: None,
        implementation_namespace: None,
        oo_context_facts: &[],
        self_receiver_words: &[],
    };

    /// Reusable base for a command whose successful result depends only on
    /// its evaluated arguments and which has no mutable-world effects or
    /// tracked state transitions.
    ///
    /// Command specs still declare their behavioural traits explicitly. This
    /// base only closes the four independent semantic proof obligations, so
    /// `PURE` and `CSE_CANDIDATE` cannot be acquired accidentally.
    ///
    /// The dispatch declaration replaces the conservative default with
    /// [`DispatchDependencies::NONE`], which
    /// [`crate::ResolvedDispatchDependencies::resolve`] restores to
    /// [`DispatchDependencies::BASE`]. That is the accurate claim for such a
    /// command: its dispatch depends only on the irreducible domains — the
    /// command binding, namespace lookup, command/execution traces, and
    /// interpreter policy. It is not an object method, and, being resolvable,
    /// it never reaches `unknown` fallback handling.
    pub const CLOSED_REFERENTIALLY_TRANSPARENT: Self = Self {
        result_stability: Some(crate::result_stability::ResultStability::ReferentiallyTransparent),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(StateTransitionDescriptor::EMPTY),
        dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(
            DispatchDependencies::NONE,
        )),
        ..Self::DEFAULT
    };

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

    /// Look up a subcommand by exact name.
    #[must_use]
    pub fn subcommand(&self, name: &str) -> Option<&SubCommand> {
        self.subcommands.iter().find(|s| s.name == name)
    }

    /// All target-neutral intrinsic identities declared anywhere in this
    /// command's command, subcommand, or invocation-form descriptors.
    ///
    /// The result is deduplicated and ordered by [`crate::IntrinsicId`], so a
    /// runtime can attach every semantic identity to one live implementation
    /// The native lowering shape this command's invocations take, defaulting
    /// to the generic argv invocation when no descriptor is stamped.
    #[must_use]
    pub const fn native_lowering(&self) -> crate::native_lowering::NativeLowering {
        match self.native_lowering {
            Some(shape) => shape,
            None => crate::native_lowering::NativeLowering::Generic,
        }
    }

    /// without knowing the command's subcommand layout.
    #[must_use]
    pub fn intrinsic_ids(&self) -> Vec<crate::IntrinsicId> {
        let mut ids = std::collections::BTreeSet::new();
        let mut add = |semantic, lowering, codegen, inline_codegen| {
            if let Some(id) =
                crate::IntrinsicId::from_descriptors(semantic, lowering, codegen, inline_codegen)
            {
                ids.insert(id);
            }
        };
        add(
            self.semantic_operation,
            self.lowering_hook,
            self.codegen_hook,
            self.inline_codegen_hook,
        );
        for form in self.command_forms {
            add(
                form.semantic_operation,
                form.lowering_hook,
                form.codegen_hook,
                None,
            );
        }
        for subcommand in self.subcommands {
            add(
                subcommand.semantic_operation,
                subcommand.lowering_hook,
                subcommand.codegen_hook,
                subcommand.inline_codegen_hook,
            );
            for form in subcommand.subcommand_forms {
                add(
                    form.semantic_operation,
                    form.lowering_hook,
                    form.codegen_hook,
                    None,
                );
            }
        }
        ids.into_iter().collect()
    }

    /// The command's primary invocation synopsis — the first non-empty
    /// [`Self::forms`] entry *available at `package_version`*, falling back to
    /// the first non-empty hover synopsis line. `None` when the spec declares
    /// neither. Used for the "usage: …" suffix on arity diagnostics
    /// (E002/E003/E005).
    ///
    /// A [`FormSpec`] carries its own [`Lifecycle`], so a form a later release
    /// added — or withdrew — is skipped when `package_version` excludes it: a
    /// document pinned to the older release is told the usage *it* has, not a
    /// synopsis it cannot call. `None` is permissive (every form is eligible),
    /// matching the rest of the `*_for_version` family; the hover fallback is
    /// never gated, because a hover line carries no lifecycle to gate on.
    ///
    /// [`FormSpec`]: crate::hover::FormSpec
    #[must_use]
    pub fn primary_synopsis(&self, package_version: Option<&str>) -> Option<&'static str> {
        self.forms
            .iter()
            .filter(|form| form.available_for_version(package_version))
            .map(|f| f.synopsis)
            .chain(self.hover.iter().flat_map(|h| h.synopsis.iter().copied()))
            .find(|s| !s.is_empty())
    }

    /// The names of the optional trailing words the *dialect-applicable*
    /// synopses document — the run of `?placeholder?` words at the end of a
    /// [`Self::forms`] entry, with the `?` markers stripped.
    ///
    /// This is how a quick-fix that offers to *append* a documented optional
    /// argument learns both how many words it may append and what to call
    /// them, without the consumer knowing the command.  `catch script
    /// ?resultVarName? ?optionsVarName?` yields
    /// `["resultVarName", "optionsVarName"]`; the narrower Tcl 8.4 / iRules
    /// form `catch script ?varName?` yields `["varName"]`, so a consumer
    /// running under those dialects never offers the options-dictionary word
    /// that release has no argument slot for.
    ///
    /// The widest applicable form wins, since a command may declare several
    /// overlapping forms for one dialect and the widest documents every slot
    /// the narrower ones do.  A form whose optional tail is interrupted by a
    /// required word contributes only its trailing run: the words a caller
    /// can append without also having to supply something in between.
    ///
    /// A form is additionally skipped when its [`Lifecycle`] excludes
    /// `package_version` — appending a word that only exists from a later
    /// release is exactly the fix a pinned document must not be offered.
    /// `None` is permissive (every form is eligible).
    ///
    /// Returns an empty vector when the command declares no forms, none apply
    /// to *dialect* and *`package_version`*, or the applicable ones end in a
    /// required word.
    #[must_use]
    pub fn optional_trailing_arg_names(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
    ) -> Vec<&'static str> {
        self.forms
            .iter()
            .filter(|form| {
                form.surface
                    .or(self.surface)
                    .is_none_or(|allowed| surface_admits(allowed, dialect.as_ref()))
            })
            .filter(|form| form.available_for_version(package_version))
            .map(|form| optional_trailing_placeholders(form.synopsis))
            .max_by_key(Vec::len)
            .unwrap_or_default()
    }

    /// This command's subcommand keyword table, filtered to what exists for
    /// `dialect` and `package_version`.
    ///
    /// The table is what [`crate::abbrev`] resolves against, so an
    /// abbreviated word is judged against exactly the keywords that exist at
    /// the target release — a prefix unique in 8.5 can be ambiguous in 8.6.
    ///
    /// `prefix_override` lets a caller force [`PrefixMatching::Strict`] for a
    /// call site the analyser saw configured with
    /// `namespace ensemble … -prefixes 0`; `None` uses the declared
    /// [`Self::prefix_matching`].
    #[must_use]
    pub fn subcommand_table(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordTable<'static> {
        let parent = self.surface;
        KeywordTable::from_keywords(
            self.subcommands
                .iter()
                .filter(|sub| {
                    sub.surface
                        .or(parent)
                        .is_none_or(|have| surface_admits(have, dialect.as_ref()))
                })
                .filter(|sub| sub.available_for_version(package_version))
                .map(|sub| Keyword {
                    name: sub.name,
                    min_abbrev: sub.min_abbrev,
                }),
            prefix_override.unwrap_or(self.prefix_matching),
        )
    }

    /// This command's option keyword table (canonical spellings and declared
    /// aliases), filtered to what exists for `dialect` and `package_version`.
    #[must_use]
    pub fn option_table(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordTable<'static> {
        option_table_from(
            self.options
                .iter()
                .chain(self.command_forms.iter().flat_map(|f| f.options.iter())),
            dialect,
            self.surface,
            package_version,
            prefix_override.unwrap_or(self.prefix_matching),
        )
    }

    /// Resolve a subcommand *word* — exact, abbreviated, ambiguous, or
    /// unknown — against this command's table for `dialect` and
    /// `package_version`.
    ///
    /// This is the one entry point every consumer uses, so the analyser,
    /// semantic tokens, hover, completion, signature help, the formatter, and
    /// the minifier cannot drift apart on what an abbreviation means.
    #[must_use]
    pub fn resolve_subcommand_word(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordMatch<'static> {
        self.subcommand_table(dialect, package_version, prefix_override)
            .resolve(word)
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
    /// one every caller already handles; a confidently wrong type is what
    /// issue #1720 was.
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

    /// The canonical names of the switches in front of this call's first
    /// positional argument.
    ///
    /// Words resolve through the command's own option table, so an
    /// abbreviation counts exactly when the real command accepts one
    /// (`lsearch -al` is `-all`; `regexp -inl` is an error, because
    /// `Tcl_RegexpObjCmd` reads its table with `TCL_EXACT`), and a value word
    /// is consumed by the switch that takes it. The scan also stops short of
    /// the operands C Tcl reserves ([`Self::reserved_trailing_words`]), so a
    /// mandatory word is never read as a switch: `Tcl_LsearchObjCmd` scans
    /// `i < objc - 2`, which makes `lsearch -all foo` a search for `foo` in
    /// the one-element list `-all` — it returns -1, an int, not the list
    /// `-all` would otherwise select (verified on tclsh 9.0.4).
    #[must_use]
    pub fn leading_switch_names(&self, args: &[&str]) -> Vec<&'static str> {
        args[..self.switch_word_count(args)]
            .iter()
            .filter_map(|word| resolve_option_prefix_with(self.options, word, self.prefix_matching))
            .map(|option| option.name)
            .collect()
    }

    /// How many positional words this call supplies — every word that is not
    /// a leading switch or one of their values. The optional-trailing-argument
    /// split (`regsub`'s `varName`, `pid`'s `fileId`) reads this.
    #[must_use]
    pub fn positional_word_count(&self, args: &[&str]) -> usize {
        args.len() - self.switch_word_count(args)
    }

    /// Words consumed by the leading switch run, values included.
    pub(crate) fn switch_word_count(&self, args: &[&str]) -> usize {
        let option_scan_end = args.len().saturating_sub(self.reserved_trailing_words);
        leading_option_word_count_with(self.options, &args[..option_scan_end], self.prefix_matching)
    }

    /// Resolve an option word against this command's option table.
    #[must_use]
    pub fn resolve_option_word(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordMatch<'static> {
        self.option_table(dialect, package_version, prefix_override)
            .resolve(word)
    }

    /// Resolve a subcommand word to its [`SubCommand`], accepting a unique
    /// non-empty prefix the way Tcl's ensemble dispatch (`Tcl_GetIndexFromObj`)
    /// does: `string le` ⇒ `length`, `info ex` ⇒ `exists`. An exact match
    /// always wins over a prefix; an ambiguous prefix (several candidates, e.g.
    /// `string t`) resolves to `None`.
    ///
    /// Dialect-agnostic — every declared subcommand is a candidate. Prefer
    /// [`Self::resolve_subcommand_for_dialect`] where the active Tcl version is
    /// known, since a prefix's uniqueness can change between versions
    /// (`info class def` is `definition` in 8.6 but ambiguous with
    /// `definitionnamespace` in 9.0).
    #[must_use]
    pub fn resolve_subcommand(&self, word: &str) -> Option<&SubCommand> {
        self.resolve_subcommand_filtered(word, |_| true)
    }

    /// Like [`Self::resolve_subcommand`] but only considers subcommands
    /// available in `dialect`, so prefix uniqueness matches the given Tcl
    /// version exactly.
    #[must_use]
    pub fn resolve_subcommand_for_dialect(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&SubCommand> {
        let parent = self.surface;
        self.resolve_subcommand_filtered(word, |s| match s.surface.or(parent) {
            Some(d) => surface_admits(d, dialect.as_ref()),
            None => true,
        })
    }

    fn resolve_subcommand_filtered(
        &self,
        word: &str,
        avail: impl Fn(&SubCommand) -> bool,
    ) -> Option<&SubCommand> {
        // One matcher for the whole codebase: build the visible table and ask
        // `crate::abbrev`. `Ambiguous` and `Unknown` both collapse to `None`
        // here because this accessor's contract is "the subcommand or
        // nothing"; callers that need to tell the two apart (the ambiguity
        // diagnostic, the formatter's expansion) use
        // [`Self::resolve_subcommand_word`] instead.
        let table = KeywordTable::from_keywords(
            self.subcommands
                .iter()
                .filter(|s| avail(s))
                .map(|s| Keyword {
                    name: s.name,
                    min_abbrev: s.min_abbrev,
                }),
            self.prefix_matching,
        );
        let canonical = table.resolve(word).unique()?;
        self.subcommands
            .iter()
            .find(|s| s.name == canonical && avail(s))
    }

    /// Return static arg role for a given index, if declared.
    #[must_use]
    pub fn arg_role_at(&self, index: u8) -> Option<ArgRole> {
        self.arg_roles
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, r)| *r)
    }

    /// Look up enumerable argument values for the 0-based `index`
    /// *after* the command name.  Returns an empty slice when this
    /// argument has no fixed value set.  Mirrors
    /// [`SubCommand::arg_values_at`].
    #[must_use]
    pub fn arg_values_at(&self, index: u8) -> &'static [ArgValue] {
        self.arg_values
            .iter()
            .find(|(i, _)| *i == index)
            .map_or(&[], |(_, vs)| vs)
    }

    /// Whether an enumerable positional argument value exists at
    /// `package_version` — the command-level twin of
    /// [`SubCommand::arg_value_available_for_version`].
    ///
    /// `index` is 0-based *after the command name*. Two gates apply, and a
    /// value must pass both: the value's own [`ArgValue::lifecycle`], and any
    /// [`Self::versioned_arg_values`] entry naming it. A value with neither is
    /// ungated, and an ungated value is always available.
    #[must_use]
    pub fn arg_value_available_for_version(
        &self,
        index: u8,
        value: &str,
        package_version: Option<&str>,
    ) -> bool {
        self.arg_values_at(index)
            .iter()
            .find(|v| v.value == value)
            .is_none_or(|v| v.available_for_version(package_version))
            && self
                .versioned_arg_values
                .iter()
                .find(|gate| gate.index == index && gate.value == value)
                .is_none_or(|gate| gate.available_for_version(package_version))
    }

    /// The enumerable values for the 0-based `index` *after the command name*
    /// that exist at `package_version` — [`Self::arg_values_at`] with both
    /// version gates applied, so completion offers exactly what the target
    /// release accepts.
    #[must_use]
    pub fn available_arg_values_at(
        &self,
        index: u8,
        package_version: Option<&str>,
    ) -> Vec<&'static ArgValue> {
        self.arg_values_at(index)
            .iter()
            .filter(|value| {
                self.arg_value_available_for_version(index, value.value, package_version)
            })
            .collect()
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

    /// Declared option / switch names valid in `dialect`, in
    /// declaration order with duplicates removed.
    ///
    /// Walks the command's
    /// declared options (both the flat [`Self::options`] list and
    /// every [`CommandForm`]'s options) and keeps
    /// only those whose [`OptionSpec::supports_dialect`] holds for
    /// `dialect`, inheriting the command's own [`Self::dialects`] as
    /// the parent set. `dialect == None` means "no dialect filter"
    /// (every declared option is returned).
    ///
    /// Used by the analyser's option-aware arity check: leading
    /// arguments that match one of these names are skipped before
    /// counting positional args, so option flags introduced in a
    /// later Tcl release don't leak into an earlier dialect's
    /// signature and get wrongly skipped (e.g. `regsub -command` is
    /// 9.0-only).
    #[must_use]
    pub fn switch_names(&self, dialect: Option<SurfaceQuery<'_>>) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        let consider = |opt: &OptionSpec, names: &mut Vec<&'static str>| {
            if opt.supports_dialect(dialect, self.surface) && !names.contains(&opt.name) {
                names.push(opt.name);
            }
        };
        for opt in self.options {
            consider(opt, &mut names);
        }
        for form in self.command_forms {
            for opt in form.options {
                consider(opt, &mut names);
            }
        }
        names
    }

    /// The declared [`OptionSpec`]s available in *dialect* (canonical
    /// options plus every command-form option), deduped by name.
    ///
    /// The name-only [`Self::switch_names`] is enough to *recognise* a
    /// leading option, but the analyser's arity check also needs to know
    /// how many *value* words each option consumes (`-start 0` is two
    /// words, not one), so a value word is not miscounted as a positional
    /// argument. Returning the specs lets a consumer call
    /// [`OptionSpec::value_word_count`]. Kept next to `switch_names` so the
    /// two stay dialect-consistent.
    #[must_use]
    pub fn option_specs(&self, dialect: Option<SurfaceQuery<'_>>) -> Vec<&'static OptionSpec> {
        let mut specs: Vec<&'static OptionSpec> = Vec::new();
        let mut consider = |opt: &'static OptionSpec| {
            if opt.supports_dialect(dialect, self.surface)
                && !specs.iter().any(|o| o.name == opt.name)
            {
                specs.push(opt);
            }
        };
        for opt in self.options {
            consider(opt);
        }
        for form in self.command_forms {
            for opt in form.options {
                consider(opt);
            }
        }
        specs
    }

    /// [`leading_option_word_count`] against this command's own
    /// [`Self::options`] — how many of `args`' leading words are declared
    /// flags/options, so a positional argument index (e.g.
    /// [`Self::defines_command_at`]) can be shifted past them.
    #[must_use]
    pub fn leading_option_word_count(&self, args: &[&str]) -> usize {
        leading_option_word_count_with(self.options, args, self.prefix_matching)
    }

    /// Like [`Self::switch_names`], but optionally including documented
    /// abbreviation aliases (`-bd` for `-borderwidth`) and filtering by the
    /// resolved package version (dropping options not yet introduced at, or
    /// already retired by, *`package_version`*).
    ///
    /// `include_aliases` is for validation callers that must accept `-bd`;
    /// completion passes `false` so only canonical spellings are offered.
    /// `package_version` is the guaranteed-available floor from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]);
    /// `None` keeps every option.
    #[must_use]
    pub fn switch_names_ext(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        include_aliases: bool,
        package_version: Option<&str>,
    ) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        let consider = |opt: &OptionSpec, names: &mut Vec<&'static str>| {
            if !opt.supports_dialect(dialect, self.surface) {
                return;
            }
            if !opt.available_for_version(package_version) {
                return;
            }
            if !names.contains(&opt.name) {
                names.push(opt.name);
            }
            if include_aliases {
                for alias in opt.aliases {
                    if !names.contains(alias) {
                        names.push(alias);
                    }
                }
            }
        };
        for opt in self.options {
            consider(opt, &mut names);
        }
        for form in self.command_forms {
            for opt in form.options {
                consider(opt, &mut names);
            }
        }
        names
    }

    /// Look up an option by its canonical name or any documented alias,
    /// honouring the `dialect` and `package_version` gates.
    #[must_use]
    pub fn find_option(
        &self,
        option_name: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
    ) -> Option<&OptionSpec> {
        let matches = |opt: &&OptionSpec| {
            opt.matches(option_name)
                && opt.supports_dialect(dialect, self.surface)
                && opt.available_for_version(package_version)
        };
        self.options.iter().find(matches).or_else(|| {
            self.command_forms
                .iter()
                .flat_map(|f| f.options.iter())
                .find(matches)
        })
    }

    /// The package whose version gates this command (Tk, a tcllib package, …).
    #[must_use]
    pub fn owning_package(&self) -> Option<&'static str> {
        self.required_package.or(self.tcllib_package)
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

/// A lifecycle gate on one literal positional argument value of a
/// subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionedArgValue {
    /// Argument index after the subcommand word.
    pub index: u8,
    /// Literal value whose availability is gated.
    pub value: &'static str,
    /// Introduction / deprecation / retirement releases of this literal value
    /// on the owning package's version axis.
    pub lifecycle: Lifecycle,
}

impl VersionedArgValue {
    /// Whether this value exists at `package_version`.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        self.lifecycle.available_at(package_version)
    }

    /// This value's lifecycle state at `package_version`.
    #[must_use]
    pub fn lifecycle_state(&self, package_version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(package_version)
    }
}

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

    /// Per-release signature shapes for this subcommand (issue #1627), with
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

    /// Inline (value-position / catch-body) bytecode codegen hook ID.
    /// See [`CommandSpec::inline_codegen_hook`]. Overrides the
    /// parent's when the call resolves to this subcommand
    /// (`dict get` / `info exists`).
    pub inline_codegen_hook: Option<InlineCodegenHookId>,

    /// Analyser handler-family hook ID.
    /// See [`CommandSpec::analyser_hook`]. Overrides the parent's when
    /// the call resolves to this subcommand (`namespace eval` /
    /// `dict for`).
    pub analyser_hook: Option<AnalyserHookId>,

    /// Command-table mutation descriptor.
    /// See [`CommandSpec::command_table_effect`]. Overrides the
    /// parent's when the call resolves to this subcommand
    /// (`interp alias`).
    pub command_table_effect: Option<CommandTableEffect>,

    /// Per-subcommand options.
    pub options: &'static [OptionSpec],

    /// Typed relations between this subcommand's options and arguments
    /// (E-R14), checked natively.
    pub option_relations: &'static [OptionRelation],

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

    /// Target-neutral completion semantics for this subcommand.
    ///
    /// A matching subcommand form takes precedence. `None` inherits the
    /// parent command's descriptor.
    pub completion: Option<crate::completion::CompletionDescriptor>,

    /// Result-dependency refinement for this subcommand. `None` inherits the
    /// parent command declaration.
    pub result_stability: Option<crate::result_stability::ResultStability>,

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
    /// (`booleanx`). Default `false` (exact match, as the top-level path uses).
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

    /// Colour whose presence on the input means this subcommand would
    /// double-encode the value (T106). `None` = none.
    pub taint_double_encode_colour: Option<TaintColour>,

    /// Output-sink diagnostic code for a subcommand-shaped XSS /
    /// header-injection sink (e.g. `"IRULE3002"`). `None` = not a
    /// sink.
    pub taint_output_sink: Option<&'static str>,

    /// Argument index (0-based after the subcommand word) carrying a
    /// credential value, for credential-exposure checks. `None` =
    /// none.
    pub credential_arg: Option<u8>,

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

    /// Structured side-effect declarations for this subcommand.
    pub side_effects: &'static [SideEffect],

    /// Mutable Tcl-world effects for this subcommand.
    ///
    /// A matching form is applied after this descriptor.  When absent,
    /// resolution simply retains the parent command's descriptor; otherwise
    /// it extends by default or explicitly replaces inherited facts.
    pub world_effects: Option<WorldEffectDescriptor>,

    /// Target-neutral transitions specific to this subcommand.
    pub state_transitions: Option<StateTransitionDescriptor>,

    /// Dispatch-stability dependency refinement for this subcommand.
    /// Irreducible live-interpreter dependencies cannot be removed.
    pub dispatch_dependencies: Option<DispatchDependencyDescriptor>,

    /// Relationship/content validator for statically-known literal arguments.
    /// A matching form may override this callback.
    pub literal_argument_validator: Option<LiteralArgumentValidator>,

    /// Irreversible operation (`file delete`, …).
    pub destructive: bool,

    /// Returns a filesystem path.
    pub returns_path: bool,

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
    /// the semantic-token pass colour that word as a subcommand keyword (issue
    /// #798), and drives hover and completion for it. Empty for the
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
/// (issue #798). Resolution accepts a unique prefix, matching how Tcl's own
/// ensemble dispatch abbreviates subcommands.
#[derive(Debug, Clone, Copy)]
pub struct SubSubCommand {
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
    /// `namespace ensemble` needs all three (issue #1610). `create` and
    /// `configure` are two different C option tables — `ensembleCreateOptions`
    /// has `-command` and no `-namespace`, `ensembleConfigOptions` the reverse
    /// (`tclEnsemble.c`) — so one merged table simultaneously offers `create`'s
    /// `-command` to `configure` (a guaranteed `bad option "-command"`) and
    /// hides `configure`'s readable `-namespace`. `exists` then needs the
    /// empty table: its C arm takes `cmdname` and nothing else, so inheriting
    /// the parent's union would offer flags that are not options there at all.
    pub options: Option<&'static [OptionSpec]>,
}

/// The option table one call dispatches on, as resolved by
/// [`SubCommand::option_scope`] — the table itself, the dialect gate its
/// options inherit, and which second-level operation (if any) supplied it.
#[derive(Debug, Clone, Copy)]
pub struct OptionScope {
    /// The options this call recognises.
    pub options: &'static [OptionSpec],
    /// The gate an option declaring none of its own inherits.
    pub surface: Option<&'static [SpecSurface]>,
    /// The second-level operation whose own table this is; `None` when the
    /// answer came from the subcommand's table.
    pub sub_subcommand: Option<&'static str>,
}

/// Whether `sub_sub` is present in `dialect`, inheriting `parent_surface`
/// (the owning subcommand's gate) when it declares none of its own.
fn sub_subcommand_supports_dialect(
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
    /// Default value for all fields — used with `..SubSubCommand::DEFAULT`.
    pub const DEFAULT: Self = Self {
        name: "",
        detail: "",
        synopsis: "",
        surface: None,
        lifecycle: Lifecycle::UNSPECIFIED,
        options: None,
    };

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
    /// Default value for all fields.
    pub const DEFAULT: Self = Self {
        name: "",
        traits: Traits::empty(),
        arity: Arity::any(),
        arity_windows: &[],
        detail: "",
        synopsis: "",
        hover: None,
        arg_roles: &[],
        arg_role_resolver: None,
        arg_presentation: &[],
        repeated_args: &[],
        command_prefixes: &[],
        command_prefix_resolver: None,
        script_timing_resolver: None,
        callback_taint_inputs: &[],
        return_type: None,
        var_write_typing: VarWriteTyping::ReturnValue,
        return_elements: None,
        var_elements_effect: None,
        representation_effect: None,
        arg_types: &[],
        pure: false,
        mutator: false,
        const_fold: None,
        const_fold_versioned: None,
        lowering_hook: None,
        codegen_hook: None,
        inline_codegen_hook: None,
        analyser_hook: None,
        command_table_effect: None,
        options: &[],
        option_relations: &[],
        constraints: None,
        option_placement: OptionPlacement::Leading,
        min_abbrev: None,
        prefix_matching: PrefixMatching::Enabled,
        arg_values: &[],
        versioned_arg_values: &[],
        subcommand_forms: &[],
        semantic_operation: None,
        completion: None,
        result_stability: None,
        surface: None,
        lifecycle: Lifecycle::UNSPECIFIED,
        safe_on_uninit: None,
        loop_list_header: false,
        creates_scope_alias: false,
        inferred_storage_type: None,
        body_kind: BodyKind::Plain,
        byte_array_effect: crate::byte_array_effect::ByteArrayEffect::None,
        closed_value_args: &[],
        arg_values_accept_prefix: false,
        body_arg_implicit_args: 0,
        taint_transform: None,
        taint_double_encode_colour: None,
        taint_output_sink: None,
        credential_arg: None,
        sensitive_headers: &[],
        pattern_type: None,
        format_string_type: None,
        side_effects: &[],
        world_effects: None,
        state_transitions: None,
        dispatch_dependencies: None,
        literal_argument_validator: None,
        destructive: false,
        returns_path: false,
        is_unescape: false,
        cfg_rewrite_name: None,
        sub_subcommands: &[],
        defines_command_at: None,
        max_leading_option_words: None,
    };

    /// Reusable base for a subcommand whose successful result depends only on
    /// its evaluated arguments and which has no mutable-world effects or
    /// tracked state transitions.
    ///
    /// As with [`CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT`], the dispatch
    /// declaration replaces the conservative default with
    /// [`DispatchDependencies::NONE`], which resolution restores to
    /// [`DispatchDependencies::BASE`] — the irreducible binding, namespace
    /// lookup, trace, and interpreter-policy domains. Such a subcommand is not
    /// an object method and, being resolvable, never reaches `unknown`
    /// fallback handling.
    pub const CLOSED_REFERENTIALLY_TRANSPARENT: Self = Self {
        result_stability: Some(crate::result_stability::ResultStability::ReferentiallyTransparent),
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(StateTransitionDescriptor::EMPTY),
        dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(
            DispatchDependencies::NONE,
        )),
        ..Self::DEFAULT
    };

    /// Reusable base for a subcommand whose result changes outside the tracked
    /// Tcl world, such as wall-clock or entropy observations.
    ///
    /// This closes only the result-dependency question. Effects and state
    /// transitions remain conservative unless the command spec independently
    /// declares them.
    pub const VOLATILE_RESULT: Self = Self {
        result_stability: Some(crate::result_stability::ResultStability::Volatile),
        ..Self::DEFAULT
    };

    /// The subcommand's primary invocation synopsis — its own
    /// [`Self::synopsis`] when non-empty, falling back to the first
    /// non-empty hover synopsis line. `None` when neither is declared.
    /// The subcommand counterpart of [`CommandSpec::primary_synopsis`].
    #[must_use]
    pub fn primary_synopsis(&self) -> Option<&'static str> {
        std::iter::once(self.synopsis)
            .chain(self.hover.iter().flat_map(|h| h.synopsis.iter().copied()))
            .find(|s| !s.is_empty())
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

    /// This subcommand's option keyword table, filtered to what exists for
    /// `dialect` and `package_version`. See
    /// [`CommandSpec::option_table`].
    #[must_use]
    pub fn option_table(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordTable<'static> {
        option_table_from(
            self.options
                .iter()
                .chain(self.subcommand_forms.iter().flat_map(|f| f.options.iter())),
            dialect,
            self.surface,
            package_version,
            prefix_override.unwrap_or(self.prefix_matching),
        )
    }

    /// Which option table a call against this subcommand actually dispatches
    /// on, once the word *after* the subcommand name is taken into account.
    ///
    /// For the overwhelmingly common single-level subcommand this is just
    /// [`Self::options`]. For a two-level ensemble whose operations disagree
    /// about their options — `namespace ensemble create` vs `configure`,
    /// issue #1610 — it is the resolved [`SubSubCommand::options`] instead,
    /// and *instead* is the point: merging the two tables reintroduces the
    /// bug, since each table's distinctive entry is the other's error.
    ///
    /// `next_word` is the literal word after the subcommand name, or `None`
    /// when the caller has no literal there (nothing typed yet, a `$var`, a
    /// `{*}` expansion). `None` and an unresolvable word fall back to this
    /// subcommand's own table — the permissive answer, so a dynamic dispatch
    /// word never narrows what is offered or accepted.
    ///
    /// A resolved operation answers with whatever it declares, **including an
    /// explicitly empty table**: `Some(&[])` means "no options here" and must
    /// not fall back (issue #1610, Codex review). Only
    /// [`SubSubCommand::options`] `== None` — declaring nothing either way —
    /// inherits.
    ///
    /// `spec_surface` is the owning [`CommandSpec::dialects`], for the
    /// standard option-gate inheritance chain (option → sub-sub → sub →
    /// command).
    #[must_use]
    pub fn option_scope(
        &self,
        next_word: Option<&str>,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        spec_surface: Option<&'static [SpecSurface]>,
    ) -> OptionScope {
        let inherited = self.surface.or(spec_surface);
        if let Some(word) = next_word.filter(|w| !w.is_empty())
            && !self.sub_subcommands.is_empty()
            && let Some(op) = self.resolve_sub_subcommand_gated(word, dialect, package_version)
            && let Some(options) = op.options
        {
            return OptionScope {
                options,
                surface: op.surface.or(inherited),
                sub_subcommand: Some(op.name),
            };
        }
        OptionScope {
            options: self.options,
            surface: inherited,
            sub_subcommand: None,
        }
    }

    /// Resolve an option word against this subcommand's option table.
    #[must_use]
    pub fn resolve_option_word(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
        prefix_override: Option<PrefixMatching>,
    ) -> KeywordMatch<'static> {
        self.option_table(dialect, package_version, prefix_override)
            .resolve(word)
    }

    /// Whether an enumerable positional argument value exists at
    /// `package_version`.
    ///
    /// Both gates apply: the value's own [`ArgValue::lifecycle`] and any
    /// [`Self::versioned_arg_values`] entry naming it.
    #[must_use]
    pub fn arg_value_available_for_version(
        &self,
        index: u8,
        value: &str,
        package_version: Option<&str>,
    ) -> bool {
        self.arg_values_at(index)
            .iter()
            .find(|v| v.value == value)
            .is_none_or(|v| v.available_for_version(package_version))
            && self
                .versioned_arg_values
                .iter()
                .find(|gate| gate.index == index && gate.value == value)
                .is_none_or(|gate| gate.available_for_version(package_version))
    }

    /// The enumerable values for the 0-based `index` *after the subcommand
    /// word* that exist at `package_version` — [`Self::arg_values_at`] with
    /// both version gates applied.
    #[must_use]
    pub fn available_arg_values_at(
        &self,
        index: u8,
        package_version: Option<&str>,
    ) -> Vec<&'static ArgValue> {
        self.arg_values_at(index)
            .iter()
            .filter(|value| {
                self.arg_value_available_for_version(index, value.value, package_version)
            })
            .collect()
    }

    /// [`leading_option_word_count`] against this subcommand's own
    /// [`Self::options`] — how many of `args`' leading words (*after* the
    /// subcommand word itself) are declared flags/options, so a positional
    /// argument index (e.g. [`Self::defines_command_at`]) can be shifted
    /// past them. The subcommand counterpart of
    /// [`CommandSpec::leading_option_word_count`].
    ///
    /// Capped by [`Self::max_leading_option_words`] when the subcommand
    /// declares one: a subcommand that hand-parses a single optional flag
    /// treats every further option-shaped word as positional, so counting
    /// them all would shift a positional index past real arguments.
    #[must_use]
    pub fn leading_option_word_count(&self, args: &[&str]) -> usize {
        let counted = leading_option_word_count_with(self.options, args, self.prefix_matching);
        match self.max_leading_option_words {
            Some(cap) => counted.min(usize::from(cap)),
            None => counted,
        }
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

    /// Resolve a second-level subcommand word to its [`SubSubCommand`],
    /// accepting a unique non-empty prefix (`info object cl` ⇒ `class`) the way
    /// Tcl's ensemble dispatch does. An exact match always wins; an ambiguous
    /// prefix (several candidates) resolves to `None`.
    #[must_use]
    pub fn resolve_sub_subcommand(&self, word: &str) -> Option<&'static SubSubCommand> {
        self.resolve_sub_subcommand_filtered(word, |_| true)
    }

    /// Like [`Self::resolve_sub_subcommand`] but only considers second-level
    /// subcommands available in `dialect`, so a prefix's uniqueness matches the
    /// given Tcl version (`info class def` is `definition` in 8.6 but ambiguous
    /// with `definitionnamespace` in 9.0).
    #[must_use]
    pub fn resolve_sub_subcommand_for_dialect(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static SubSubCommand> {
        self.resolve_sub_subcommand_gated(word, dialect, None)
    }

    /// The second-level subcommands available for `dialect` and
    /// `package_version`, in declaration order — the table completion and the
    /// analyser judge a third-level word against, so an operation the target
    /// release does not have is neither offered nor accepted.
    ///
    /// `None` on either gate is permissive, matching every other availability
    /// query in this module.
    #[must_use]
    pub fn available_sub_subcommands(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
    ) -> Vec<&'static SubSubCommand> {
        let parent = self.surface;
        self.sub_subcommands
            .iter()
            .filter(|s| sub_subcommand_supports_dialect(s, dialect, parent))
            .filter(|s| s.available_for_version(package_version))
            .collect()
    }

    /// [`Self::resolve_sub_subcommand`] narrowed by both availability axes:
    /// the Tcl-core `dialect` and the owning package's `package_version`
    /// floor.
    ///
    /// Prefix uniqueness is judged against exactly the operations that exist
    /// at the target, so `info class def` can be unambiguous on one release
    /// and ambiguous on the next.
    #[must_use]
    pub fn resolve_sub_subcommand_gated(
        &self,
        word: &str,
        dialect: Option<SurfaceQuery<'_>>,
        package_version: Option<&str>,
    ) -> Option<&'static SubSubCommand> {
        let parent = self.surface;
        self.resolve_sub_subcommand_filtered(word, |s| {
            sub_subcommand_supports_dialect(s, dialect, parent)
                && s.available_for_version(package_version)
        })
    }

    fn resolve_sub_subcommand_filtered(
        &self,
        word: &str,
        avail: impl Fn(&SubSubCommand) -> bool,
    ) -> Option<&'static SubSubCommand> {
        let available: Vec<&'static SubSubCommand> = self
            .sub_subcommands
            .iter()
            .filter(|sub| avail(sub))
            .collect();
        let names: Vec<&'static str> = available.iter().map(|sub| sub.name).collect();
        tcl_cmd_core::ensemble::resolve_subcommand(&names, word.as_bytes(), true)
            .map(|index| available[index])
    }

    /// Whether `word` resolves to a second-level subcommand of this
    /// two-level-ensemble subcommand (exact or unique-prefix; see
    /// [`Self::resolve_sub_subcommand`]).
    #[must_use]
    pub fn is_sub_subcommand(&self, word: &str) -> bool {
        self.resolve_sub_subcommand(word).is_some()
    }

    /// Look up a static arg role by index.
    #[must_use]
    pub fn arg_role_at(&self, index: u8) -> Option<ArgRole> {
        self.arg_roles
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, r)| *r)
    }

    /// Declared option-flag names for this subcommand, filtered by
    /// *dialect*.
    ///
    /// Per-subcommand options
    /// (e.g. `-symbolic` / `-hard` on `file link`) flow into the
    /// subcommand's [`crate::analyser`]-side `leading_options` so the
    /// arity check skips them before counting positionals.  An option's
    /// dialect membership inherits from this subcommand's `dialects`
    /// (falling back to *`parent_surface`*, the parent
    /// [`CommandSpec::dialects`]) when the option itself does not pin a
    /// dialect.
    #[must_use]
    pub fn switch_names(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        parent_surface: Option<&'static [SpecSurface]>,
    ) -> Vec<&'static str> {
        let effective_parent = self.surface.or(parent_surface);
        let mut names: Vec<&'static str> = Vec::new();
        for opt in self.options {
            if opt.supports_dialect(dialect, effective_parent) && !names.contains(&opt.name) {
                names.push(opt.name);
            }
        }
        names
    }

    /// The declared [`OptionSpec`]s for this subcommand available in
    /// *dialect* (see [`CommandSpec::option_specs`]). Carries the value
    /// arity the analyser's arity check needs so a value-taking option's
    /// value word (`file link -symbolic dst src`) is not miscounted as a
    /// positional argument.
    #[must_use]
    pub fn option_specs(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
        parent_surface: Option<&'static [SpecSurface]>,
    ) -> Vec<&'static OptionSpec> {
        let effective_parent = self.surface.or(parent_surface);
        let mut specs: Vec<&'static OptionSpec> = Vec::new();
        for opt in self.options {
            if opt.supports_dialect(dialect, effective_parent)
                && !specs.iter().any(|o| o.name == opt.name)
            {
                specs.push(opt);
            }
        }
        specs
    }

    /// Look up enumerable argument values for the 0-based
    /// `index` *after* the subcommand word.  Returns an empty
    /// slice when this argument has no fixed value set.
    #[must_use]
    pub fn arg_values_at(&self, index: u8) -> &'static [ArgValue] {
        self.arg_values
            .iter()
            .find(|(i, _)| *i == index)
            .map_or(&[], |(_, vs)| vs)
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family, SurfaceQuery};

    use super::*;
    use crate::registry::CommandRegistry;

    // -- optional_trailing_arg_names (issue #1190) ------------------------
    //
    // A quick fix that appends a documented optional argument learns both how
    // many words it may append and what to call them from the synopsis, so a
    // dialect whose form documents fewer words gets a narrower offer.

    #[test]
    fn optional_trailing_placeholders_takes_the_trailing_run() {
        assert_eq!(
            optional_trailing_placeholders("catch script ?resultVarName? ?optionsVarName?"),
            vec!["resultVarName", "optionsVarName"]
        );
        // A required word terminates the run — these are the words a caller
        // may append *without* also having to supply something in between.
        assert_eq!(
            optional_trailing_placeholders("cmd ?a? required ?b?"),
            vec!["b"]
        );
        // No optional tail at all.
        assert!(optional_trailing_placeholders("puts string").is_empty());
        // A variadic tail is not a fixed slot list.
        assert!(optional_trailing_placeholders("cmd ?arg ...?").is_empty());
        assert!(optional_trailing_placeholders("cmd ?...?").is_empty());
    }

    #[test]
    fn optional_trailing_arg_names_narrows_with_the_dialect() {
        let registry = CommandRegistry::build_default();
        let spec = registry.get("catch").expect("catch is a registry command");
        // Tcl 8.5 onward documents `catch script ?resultVarName? ?optionsVarName?`.
        assert_eq!(
            spec.optional_trailing_arg_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")), None),
            vec!["resultVarName", "optionsVarName"]
        );
        // Tcl 8.4's `catch script ?varName?` has no options dictionary, so a
        // consumer running under that dialect never offers a word the release
        // has no argument slot for.
        assert_eq!(
            spec.optional_trailing_arg_names(Some(SurfaceQuery::core(Family::Tcl, "8.4")), None),
            vec!["varName"]
        );
    }

    // -- FormSpec lifecycle in the form-selection queries -----------------
    //
    // A `FormSpec` carries its own lifecycle, so a synopsis a later release
    // added must not be shown to — nor its optional words offered to — a
    // document pinned below it.  The widest form is declared first, matching
    // the shipped convention (`lassign`'s 8.6+ shape leads its 8.5 one).

    const VERSIONED_FORMS: &[crate::hover::FormSpec] = &[
        crate::hover::FormSpec {
            synopsis: "fauxform target ?resultVar? ?optionsVar?",
            lifecycle: Lifecycle::introduced_in("2.0"),
            ..crate::hover::FormSpec::DEFAULT
        },
        crate::hover::FormSpec {
            synopsis: "fauxform target ?resultVar?",
            ..crate::hover::FormSpec::DEFAULT
        },
    ];

    fn versioned_form_spec() -> CommandSpec {
        CommandSpec {
            name: "fauxform",
            forms: VERSIONED_FORMS,
            ..CommandSpec::DEFAULT
        }
    }

    #[test]
    fn primary_synopsis_skips_a_form_the_floor_predates() {
        let spec = versioned_form_spec();
        // At 1.0 the two-optional-word form does not exist yet, so the usage
        // hint shows the shape that release actually has.
        assert_eq!(
            spec.primary_synopsis(Some("1.0")),
            Some("fauxform target ?resultVar?")
        );
        // From 2.0 the newer form leads, exactly as declaration order says.
        assert_eq!(
            spec.primary_synopsis(Some("2.0")),
            Some("fauxform target ?resultVar? ?optionsVar?")
        );
        // No floor resolved = permissive, i.e. the pre-existing behaviour.
        assert_eq!(
            spec.primary_synopsis(None),
            Some("fauxform target ?resultVar? ?optionsVar?")
        );
    }

    #[test]
    fn optional_trailing_arg_names_skips_a_form_the_floor_predates() {
        let spec = versioned_form_spec();
        // A quick fix must not offer to append `optionsVar` to a document
        // whose Fauxpkg is 1.x: that release has no argument slot for it.
        assert_eq!(
            spec.optional_trailing_arg_names(
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                Some("1.0")
            ),
            vec!["resultVar"]
        );
        assert_eq!(
            spec.optional_trailing_arg_names(
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                Some("2.0")
            ),
            vec!["resultVar", "optionsVar"]
        );
        assert_eq!(
            spec.optional_trailing_arg_names(Some(SurfaceQuery::core(Family::Tcl, "9.0")), None),
            vec!["resultVar", "optionsVar"]
        );
    }

    #[test]
    fn a_retired_form_stops_being_documented() {
        const RETIRED: &[crate::hover::FormSpec] = &[crate::hover::FormSpec {
            synopsis: "fauxform target ?resultVar?",
            lifecycle: Lifecycle::UNSPECIFIED.retired_from("3.0"),
            ..crate::hover::FormSpec::DEFAULT
        }];
        let spec = CommandSpec {
            name: "fauxform",
            forms: RETIRED,
            ..CommandSpec::DEFAULT
        };
        assert!(spec.primary_synopsis(Some("3.0")).is_none());
        assert!(
            spec.optional_trailing_arg_names(
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
                Some("3.0")
            )
            .is_empty()
        );
        // Retirement is exclusive, so 2.9 still documents it.
        assert_eq!(
            spec.primary_synopsis(Some("2.9")),
            Some("fauxform target ?resultVar?")
        );
    }

    // -- command-level argument-value gates ------------------------------
    //
    // A literal value can be gated by its own `ArgValue::lifecycle`, by a
    // positional `versioned_arg_values` entry, or by both; the accessor must
    // apply every gate that names it.

    const GATED_VALUES: &[ArgValue] = &[
        ArgValue {
            value: "old",
            ..ArgValue::DEFAULT
        },
        ArgValue {
            value: "new",
            lifecycle: Lifecycle::introduced_in("2.0"),
            ..ArgValue::DEFAULT
        },
    ];

    const UNDECLARED_GATE: &[VersionedArgValue] = &[VersionedArgValue {
        index: 0,
        value: "undeclared",
        lifecycle: Lifecycle::introduced_in("3.0"),
    }];

    fn gated_spec() -> CommandSpec {
        CommandSpec {
            name: "widget",
            arg_values: &[(0, GATED_VALUES)],
            versioned_arg_values: UNDECLARED_GATE,
            ..CommandSpec::DEFAULT
        }
    }

    #[test]
    fn command_arg_value_availability_honours_both_gates() {
        let spec = gated_spec();
        // The value's own lifecycle …
        assert!(!spec.arg_value_available_for_version(0, "new", Some("1.0")));
        assert!(spec.arg_value_available_for_version(0, "new", Some("2.0")));
        // … a positional gate for a value the table does not declare …
        assert!(!spec.arg_value_available_for_version(0, "undeclared", Some("2.0")));
        assert!(spec.arg_value_available_for_version(0, "undeclared", Some("3.0")));
        // … an ungated value, an unknown value, and an unknown target are
        // all permissive.
        assert!(spec.arg_value_available_for_version(0, "old", Some("1.0")));
        assert!(spec.arg_value_available_for_version(0, "anything", Some("1.0")));
        assert!(spec.arg_value_available_for_version(0, "new", None));
        // A different argument index shares no gates.
        assert!(spec.arg_value_available_for_version(1, "new", Some("1.0")));
    }

    #[test]
    fn available_arg_values_filters_the_declared_table() {
        let spec = gated_spec();
        let names = |version: Option<&str>| -> Vec<&str> {
            spec.available_arg_values_at(0, version)
                .iter()
                .map(|value| value.value)
                .collect()
        };
        assert_eq!(names(Some("1.0")), vec!["old"]);
        assert_eq!(names(Some("2.0")), vec!["old", "new"]);
        assert_eq!(names(None), vec!["old", "new"]);
    }

    // -- second-level subcommand gates -----------------------------------

    const OPS: &[SubSubCommand] = &[
        SubSubCommand {
            name: "classic",
            ..SubSubCommand::DEFAULT
        },
        SubSubCommand {
            name: "modern",
            lifecycle: Lifecycle::introduced_in("2.0"),
            ..SubSubCommand::DEFAULT
        },
    ];

    const OP_HOLDER: SubCommand = SubCommand {
        name: "op",
        sub_subcommands: OPS,
        ..SubCommand::DEFAULT
    };

    #[test]
    fn sub_subcommand_resolution_honours_the_package_floor() {
        // Below the introduction the operation does not exist, so it neither
        // resolves nor competes for a prefix …
        assert!(
            OP_HOLDER
                .resolve_sub_subcommand_gated("modern", None, Some("1.0"))
                .is_none()
        );
        assert_eq!(
            OP_HOLDER
                .resolve_sub_subcommand_gated("modern", None, Some("2.0"))
                .map(|op| op.name),
            Some("modern")
        );
        // … and an unknown target stays permissive, matching every other
        // availability query.
        assert_eq!(
            OP_HOLDER
                .resolve_sub_subcommand_gated("modern", None, None)
                .map(|op| op.name),
            Some("modern")
        );
        let names = |version: Option<&str>| -> Vec<&str> {
            OP_HOLDER
                .available_sub_subcommands(None, version)
                .iter()
                .map(|op| op.name)
                .collect()
        };
        assert_eq!(names(Some("1.0")), vec!["classic"]);
        assert_eq!(names(Some("2.0")), vec!["classic", "modern"]);
    }
}
