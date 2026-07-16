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

use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::body_kind::BodyKind;
use crate::clause_shape::ClauseShapeChecker;
use crate::command_table::CommandTableEffect;
use crate::dialects::DialectSet;
use crate::forms::{CommandForm, SubCommandForm};
use crate::hooks::{
    AnalyserHookId, ArgTypeHint, CodegenHookId, ConstFoldFn, InlineCodegenHookId, LoweringHookId,
    TclVersion, VersionedConstFoldFn, WasmCodegenHookId,
};
use crate::hover::{ArgValue, FormSpec, HoverSnippet, OptionSpec};
use crate::patterns::{FormatType, PatternType};
use crate::side_effects::{SideEffect, StorageType};
use crate::symbol_def::SymbolDef;
use crate::taint::{SetterConstraint, TaintColour};
use crate::traits::Traits;
use crate::types::{TclType, VarWriteTyping};

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
pub type CommandPrefixResolver = fn(args: &[&str]) -> Vec<(u8, crate::arg_role::AppendedArity)>;

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
            Self::Integer => matches!(
                tcl_syntax::number::parse_whole(word),
                Some(tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. })
            ),
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
}

impl ObjectClassSpec {
    /// Look up an instance method by name (this class only — no superclass
    /// walk; the registry's [`crate::CommandRegistry::instance_method`] does
    /// the inherited resolution).
    #[must_use]
    pub fn instance_method(&self, name: &str) -> Option<&SubCommand> {
        self.instance_methods.iter().find(|m| m.name == name)
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
    /// A *command* option that makes every pattern a regex (`switch -regexp`).
    pub regex_option: Option<&'static str>,
    /// Command options that consume a following value word (`switch -matchvar
    /// var`), so the value is not mistaken for the subject.
    pub value_options: &'static [&'static str],
    /// Flags that may precede a pattern *inside* the list (Expect's `-re`,
    /// `-gl`, `-ex`, `-nocase`, `-timeout`).  Empty means no clause flags.
    pub clause_flags: &'static [&'static str],
    /// The clause flag that makes its pattern a regex (Expect's `-re`).
    pub clause_regex_flag: Option<&'static str>,
    /// Clause flags that consume a following value word (`-timeout 5`).
    pub clause_value_flags: &'static [&'static str],
    /// Patterns that are keywords, not match text (`default`; Expect's
    /// `timeout` / `eof` / `full_buffer`).
    pub keyword_patterns: &'static [&'static str],
}

impl CaseListSpec {
    /// The `switch … { pat body … }` shape.
    pub const SWITCH: Self = Self {
        subject_args: 1,
        regex_option: Some("-regexp"),
        value_options: &["-matchvar", "-indexvar"],
        clause_flags: &[],
        clause_regex_flag: None,
        clause_value_flags: &[],
        keyword_patterns: &["default"],
    };

    /// The Expect `expect { ?-flags? pat body … }` shape.
    pub const EXPECT: Self = Self {
        subject_args: 0,
        regex_option: None,
        // `-timeout` / `-i` also appear as *command-level* options ahead of the
        // list (`expect -timeout 5 { … }`, `expect -i $spawn { … }`).  Leaving
        // this empty stopped the option scan on the value word, so the braced
        // list was never recognised as a clause list and its bodies were never
        // recursed.
        value_options: &["-timeout", "-i"],
        clause_flags: &["-re", "-gl", "-ex", "-nocase", "-timeout", "-i", "--"],
        clause_regex_flag: Some("-re"),
        clause_value_flags: &["-timeout", "-i"],
        keyword_patterns: &["timeout", "eof", "default", "full_buffer"],
    };
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
    pub dialects: Option<DialectSet>,

    /// Argument count constraint.
    pub arity: Arity,

    /// Static argument roles (for fixed-layout commands like `for`).
    /// Each tuple is `(arg_index, role)`.
    pub arg_roles: &'static [(u8, ArgRole)],

    /// Dynamic argument role resolver (for variable-layout commands).
    pub arg_role_resolver: Option<ArgRoleResolver>,

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

    /// Return type of the command.
    pub return_type: Option<TclType>,

    /// How the command types the variable(s) it writes as a side effect —
    /// distinct from [`Self::return_type`], which types the value `[cmd …]`
    /// yields.  A destructuring writer (`lassign`, `scan`, `regexp`, `gets`)
    /// returns one thing and writes another, so the compiler's type inference
    /// reads this instead of broadcasting the return type onto the written
    /// variables.  See [`VarWriteTyping`].  Default
    /// [`VarWriteTyping::ReturnValue`].
    pub var_write_typing: VarWriteTyping,

    /// Per-argument type hints. Each tuple is `(arg_index, hint)`.
    pub arg_types: &'static [(u8, ArgTypeHint)],

    /// Subcommands (for `dict`, `string`, `info`, etc.).
    pub subcommands: &'static [SubCommand],

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

    /// Which argument index is a variable name assigned by the command.
    /// `None` = command does not assign a variable.
    pub assigns_variable_at: Option<u8>,

    /// Dialects where this command safely initialises an uninitialised
    /// variable. `None` = not safe. `Some(empty)` = safe in all dialects.
    pub safe_on_uninit: Option<DialectSet>,

    /// Compile-time constant folder.
    pub const_fold: Option<ConstFoldFn>,

    /// Tcl-version-aware constant folder, for commands whose compile-time value
    /// depends on the dialect's Tcl release (`format`, `scan`).  Takes priority
    /// over [`Self::const_fold`] when set; see [`Self::run_const_fold`].
    pub const_fold_versioned: Option<VersionedConstFoldFn>,

    /// Lowering hook ID (index into compiler's dispatch table).
    pub lowering_hook: Option<LoweringHookId>,

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

    /// WASM-runtime codegen hook ID — picks the per-command
    /// emitter on the WASM target. Currently always `None`
    /// (no WASM-specific emitters yet); the field exists so
    /// the per-command coverage audit can track WASM hook stamping.
    pub wasm_codegen_hook: Option<WasmCodegenHookId>,

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
    /// records via [`crate::CommandRegistry::command_table_effect`].
    pub command_table_effect: Option<CommandTableEffect>,

    /// Structured side-effect declarations.
    pub side_effects: &'static [SideEffect],

    /// Inferred storage type for the target variable (`Dict`, `List`, `Array`).
    pub inferred_storage_type: Option<StorageType>,

    /// Package requirement (command only visible when package is `require`d).
    pub required_package: Option<&'static str>,

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

    /// Options declared on the command (for completion and arity adjustment).
    pub options: &'static [OptionSpec],

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
    /// `reserved_trailing_words` on [`crate::registry::ResolvedTerminator`],
    /// which both W304 and T102's option-scan-region walk cap their scan
    /// against. Default `0` (no reservation) keeps every existing spec
    /// correct.
    pub reserved_trailing_words: usize,

    /// Enumerable positional-argument values, keyed by 0-based
    /// argument index *after* the command name.  Drives command-level
    /// value completion — e.g. iRules `when EVENT timing enable|disable`
    /// declares `(2, &[enable, disable])`, and `HTTP::respond <status>
    /// content|noserver|version` declares `(1, &[…])`.  Flattened from
    /// per-form values to the command level since the completion
    /// consumer keys purely on positional index.
    pub arg_values: &'static [(u8, &'static [ArgValue])],

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

    /// Kind of format string this command's format argument uses
    /// (`format`/`scan` ⇒ `Sprintf`, …), for inlay-hint parsing and
    /// semantic-token sub-tokens. `None` = not a format command.
    pub format_string_type: Option<FormatType>,

    /// Tcllib package that provides this command, for per-document
    /// activation via `package require`. `None` = core/built-in.
    pub tcllib_package: Option<&'static str>,

    /// Minimum version of `required_package` / `tcllib_package` that
    /// introduced this command, as a dotted Tcl version string — e.g. the
    /// `ttk::*` widgets need Tk `8.5`. `None` = present in every version of
    /// the owning package. Gated against the version resolved from
    /// `package require` via [`CommandSpec::available_for_version`].
    pub min_version: Option<&'static str>,
    /// The last package version that still provides this command, or
    /// `None` while it remains present (the open maximum — nothing
    /// modelled is removed yet). Checked alongside `min_version` by
    /// [`CommandSpec::available_for_version`].
    pub max_version: Option<&'static str>,

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

    /// XC operation this command maps to, when it is translatable.
    /// `None` = no explicit mapping.
    pub xc_operation: Option<&'static str>,

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
}

impl CommandSpec {
    /// Default value for all fields — used with `..CommandSpec::DEFAULT`.
    pub const DEFAULT: Self = Self {
        name: "",
        traits: Traits::empty(),
        dialects: None,
        arity: Arity::any(),
        arg_roles: &[],
        arg_role_resolver: None,
        clause_shape_check: None,
        command_prefixes: &[],
        command_prefix_resolver: None,
        return_type: None,
        var_write_typing: VarWriteTyping::ReturnValue,
        arg_types: &[],
        subcommands: &[],
        allow_unknown_subcommands: false,
        default_form_first_word: None,
        hover: None,
        forms: &[],
        command_forms: &[],
        assigns_variable_at: None,
        safe_on_uninit: None,
        const_fold: None,
        const_fold_versioned: None,
        lowering_hook: None,
        codegen_hook: None,
        inline_codegen_hook: None,
        wasm_codegen_hook: None,
        analyser_hook: None,
        command_table_effect: None,
        side_effects: &[],
        inferred_storage_type: None,
        required_package: None,
        excluded_events: &[],
        unsafe_command: false,
        closed_value_args: &[],
        event_requires: None,
        options: &[],
        reserved_trailing_words: 0,
        arg_values: &[],
        body_kind: BodyKind::Plain,
        body_arg_implicit_args: 0,
        taint_output_sink: None,
        taint_output_sink_subcommands: &[],
        taint_log_sink: None,
        taint_network_sink_args: None,
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
        format_string_type: None,
        tcllib_package: None,
        min_version: None,
        max_version: None,
        warn_missing_import: true,
        is_namespace_exported: false,
        xc_translatable: None,
        xc_operation: None,
        deprecated_replacement: None,
        deprecated_replacement_drop_in: false,
        byte_array_payload: None,
        byte_array_effect: crate::byte_array_effect::ByteArrayEffect::None,
        definition_body: None,
        case_list: None,
        object_class: None,
        defines_symbol: None,
        body_scope: None,
        creates_instance_at: None,
        defines_command_at: None,
    };

    /// Run this command's constant folder for `args` under the optimiser's
    /// `dialect` (`"tcl8.4"` … `"tcl9.0"`, or `None`/unversioned).  All the
    /// dialect interpretation lives here in the registry layer: the
    /// version-aware [`Self::const_fold_versioned`] is tried first (the dialect
    /// is mapped to a [`TclVersion`] for it), falling back to the
    /// version-invariant [`Self::const_fold`].  Downstream consumers just pass
    /// the dialect they already have.
    #[must_use]
    pub fn run_const_fold(&self, args: &[&str], dialect: Option<&str>) -> Option<String> {
        if let Some(vf) = self.const_fold_versioned {
            vf(args, TclVersion::from_dialect(dialect))
        } else {
            self.const_fold?(args)
        }
    }

    /// Look up a subcommand by exact name.
    #[must_use]
    pub fn subcommand(&self, name: &str) -> Option<&SubCommand> {
        self.subcommands.iter().find(|s| s.name == name)
    }

    /// The command's primary invocation synopsis — the first non-empty
    /// [`Self::forms`] entry, falling back to the first non-empty hover
    /// synopsis line. `None` when the spec declares neither. Used for the
    /// "usage: …" suffix on arity diagnostics (E002/E003/E005).
    #[must_use]
    pub fn primary_synopsis(&self) -> Option<&'static str> {
        self.forms
            .iter()
            .map(|f| f.synopsis)
            .chain(self.hover.iter().flat_map(|h| h.synopsis.iter().copied()))
            .find(|s| !s.is_empty())
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
        dialect: DialectSet,
    ) -> Option<&SubCommand> {
        let parent = self.dialects;
        self.resolve_subcommand_filtered(word, |s| match s.dialects.or(parent) {
            Some(d) => d.intersects(dialect),
            None => true,
        })
    }

    fn resolve_subcommand_filtered(
        &self,
        word: &str,
        avail: impl Fn(&SubCommand) -> bool,
    ) -> Option<&SubCommand> {
        if word.is_empty() {
            return None;
        }
        if let Some(exact) = self.subcommands.iter().find(|s| s.name == word && avail(s)) {
            return Some(exact);
        }
        let mut hits = self
            .subcommands
            .iter()
            .filter(|s| s.name.starts_with(word) && avail(s));
        let first = hits.next()?;
        if hits.next().is_some() {
            return None; // ambiguous prefix
        }
        Some(first)
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

    /// Check if this command is available in a given dialect.
    #[must_use]
    pub fn supports_dialect(&self, dialect: DialectSet) -> bool {
        match self.dialects {
            None => true,
            Some(ds) => ds.intersects(dialect),
        }
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
    pub fn switch_names(&self, dialect: Option<DialectSet>) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        let consider = |opt: &OptionSpec, names: &mut Vec<&'static str>| {
            if opt.supports_dialect(dialect, self.dialects) && !names.contains(&opt.name) {
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
    pub fn option_specs(&self, dialect: Option<DialectSet>) -> Vec<&'static OptionSpec> {
        let mut specs: Vec<&'static OptionSpec> = Vec::new();
        let mut consider = |opt: &'static OptionSpec| {
            if opt.supports_dialect(dialect, self.dialects)
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

    /// Like [`Self::switch_names`], but optionally including documented
    /// abbreviation aliases (`-bd` for `-borderwidth`) and filtering by the
    /// resolved package version (dropping options whose `min_version` is
    /// newer than *`package_version`*).
    ///
    /// `include_aliases` is for validation callers that must accept `-bd`;
    /// completion passes `false` so only canonical spellings are offered.
    /// `package_version` is the guaranteed-available floor from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]);
    /// `None` keeps every option.
    #[must_use]
    pub fn switch_names_ext(
        &self,
        dialect: Option<DialectSet>,
        include_aliases: bool,
        package_version: Option<&str>,
    ) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        let consider = |opt: &OptionSpec, names: &mut Vec<&'static str>| {
            if !opt.supports_dialect(dialect, self.dialects) {
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
        dialect: Option<DialectSet>,
        package_version: Option<&str>,
    ) -> Option<&OptionSpec> {
        let matches = |opt: &&OptionSpec| {
            opt.matches(option_name)
                && opt.supports_dialect(dialect, self.dialects)
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
    /// `None` is permissive; a command with no `min_version` is always
    /// available.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        if let (Some(max), Some(version)) = (self.max_version, package_version)
            && crate::version::compare(version, max).is_gt()
        {
            return false;
        }
        match (self.min_version, package_version) {
            (Some(min), Some(have)) => crate::version::meets_min(have, min),
            _ => true,
        }
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
    pub arity: Arity,

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

    /// Static command-prefix positions + appended arities (after the
    /// subcommand word), e.g. `trace add variable`'s callback.
    pub command_prefixes: &'static [(u8, crate::arg_role::AppendedArity)],

    /// Dynamic command-prefix resolver (after the subcommand word).
    pub command_prefix_resolver: Option<CommandPrefixResolver>,

    /// Return type.
    pub return_type: Option<TclType>,

    /// How this subcommand types the variable(s) it writes as a side effect,
    /// overriding the parent command's [`CommandSpec::var_write_typing`] when
    /// a subcommand matches (`binary scan` destructures; `binary format` does
    /// not).  See [`VarWriteTyping`].  Default [`VarWriteTyping::ReturnValue`].
    pub var_write_typing: VarWriteTyping,

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

    /// WASM-runtime codegen hook ID. See
    /// [`CommandSpec::wasm_codegen_hook`].
    pub wasm_codegen_hook: Option<WasmCodegenHookId>,

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

    /// Enumerable positional-argument values, keyed by 0-based
    /// argument index *after* the subcommand word.  Drives
    /// value completion — e.g. `string is <class>` declares
    /// `(0, &[alnum, alpha, …])` so the character classes
    /// complete at the first sub-arg.
    pub arg_values: &'static [(u8, &'static [ArgValue])],

    /// Structured invocation-form descriptors for the subcommand.
    ///
    /// Same shape as [`CommandSpec::command_forms`]; entries are
    /// matched against the argument list *after* the subcommand
    /// word.
    pub subcommand_forms: &'static [SubCommandForm],

    /// Dialect membership. `None` = inherit from parent `CommandSpec`.
    pub dialects: Option<DialectSet>,

    /// Safe-on-uninit dialect set.
    pub safe_on_uninit: Option<DialectSet>,

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

    /// XC operation this subcommand maps to. `None` = no explicit
    /// mapping.
    pub xc_operation: Option<&'static str>,

    /// Structured side-effect declarations for this subcommand.
    pub side_effects: &'static [SideEffect],

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
    pub dialects: Option<DialectSet>,
}

impl SubCommand {
    /// Default value for all fields.
    pub const DEFAULT: Self = Self {
        name: "",
        traits: Traits::empty(),
        arity: Arity::any(),
        detail: "",
        synopsis: "",
        hover: None,
        arg_roles: &[],
        arg_role_resolver: None,
        command_prefixes: &[],
        command_prefix_resolver: None,
        return_type: None,
        var_write_typing: VarWriteTyping::ReturnValue,
        arg_types: &[],
        pure: false,
        mutator: false,
        const_fold: None,
        const_fold_versioned: None,
        lowering_hook: None,
        codegen_hook: None,
        inline_codegen_hook: None,
        wasm_codegen_hook: None,
        analyser_hook: None,
        command_table_effect: None,
        options: &[],
        arg_values: &[],
        subcommand_forms: &[],
        dialects: None,
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
        xc_operation: None,
        side_effects: &[],
        destructive: false,
        returns_path: false,
        is_unescape: false,
        cfg_rewrite_name: None,
        sub_subcommands: &[],
        defines_command_at: None,
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

    /// Run this subcommand's constant folder for `args` under `dialect` —
    /// version-aware [`Self::const_fold_versioned`] first (mapping the dialect
    /// to a [`TclVersion`]), else the invariant [`Self::const_fold`].  See
    /// [`CommandSpec::run_const_fold`].
    #[must_use]
    pub fn run_const_fold(&self, args: &[&str], dialect: Option<&str>) -> Option<String> {
        if let Some(vf) = self.const_fold_versioned {
            vf(args, TclVersion::from_dialect(dialect))
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
        dialect: DialectSet,
    ) -> Option<&'static SubSubCommand> {
        let parent = self.dialects;
        self.resolve_sub_subcommand_filtered(word, |s| match s.dialects.or(parent) {
            Some(d) => d.intersects(dialect),
            None => true,
        })
    }

    fn resolve_sub_subcommand_filtered(
        &self,
        word: &str,
        avail: impl Fn(&SubSubCommand) -> bool,
    ) -> Option<&'static SubSubCommand> {
        if word.is_empty() {
            return None;
        }
        let subs: &'static [SubSubCommand] = self.sub_subcommands;
        if let Some(exact) = subs.iter().find(|s| s.name == word && avail(s)) {
            return Some(exact);
        }
        let mut hits = subs.iter().filter(|s| s.name.starts_with(word) && avail(s));
        let first = hits.next()?;
        // Unique prefix only — bail if a second candidate also matches.
        if hits.next().is_some() {
            return None;
        }
        Some(first)
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
    /// (falling back to *`parent_dialects`*, the parent
    /// [`CommandSpec::dialects`]) when the option itself does not pin a
    /// dialect.
    #[must_use]
    pub fn switch_names(
        &self,
        dialect: Option<DialectSet>,
        parent_dialects: Option<DialectSet>,
    ) -> Vec<&'static str> {
        let effective_parent = self.dialects.or(parent_dialects);
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
        dialect: Option<DialectSet>,
        parent_dialects: Option<DialectSet>,
    ) -> Vec<&'static OptionSpec> {
        let effective_parent = self.dialects.or(parent_dialects);
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
