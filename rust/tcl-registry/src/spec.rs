//! Command and subcommand specifications.
//!
//! `CommandSpec` is the single source of truth for everything the
//! compiler, analyser, formatter, LSP, and codegen need to know about
//! a Tcl command. One file per command, one `CommandSpec` per file.

use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::body_kind::BodyKind;
use crate::dialects::DialectSet;
use crate::forms::{CommandForm, SubCommandForm};
use crate::hooks::{
    ArgTypeHint, CodegenHookId, ConstFoldFn, LoweringHookId, TclVersion, VersionedConstFoldFn,
    WasmCodegenHookId,
};
use crate::hover::{ArgValue, FormSpec, HoverSnippet, OptionSpec};
use crate::patterns::{FormatType, PatternType};
use crate::side_effects::{SideEffect, StorageType};
use crate::taint::{SetterConstraint, TaintColour};
use crate::traits::Traits;
use crate::types::TclType;

/// Dynamic argument role resolver.
///
/// Called for variable-layout commands (`if`, `try`, `switch`, `foreach`)
/// where argument roles depend on the actual argument values (e.g. the
/// position of `elseif`/`else` keywords). Returns a list of
/// `(arg_index, role)` pairs.
pub type ArgRoleResolver = fn(args: &[&str]) -> Vec<(u8, ArgRole)>;

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
pub struct CommandSpec {
    /// Command name (e.g. `"for"`, `"dict"`, `"HTTP::header"`).
    pub name: &'static str,

    /// Behavioural trait flags (replaces ~35 Python boolean fields).
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

    /// Return type of the command.
    pub return_type: Option<TclType>,

    /// Per-argument type hints. Each tuple is `(arg_index, hint)`.
    pub arg_types: &'static [(u8, ArgTypeHint)],

    /// Subcommands (for `dict`, `string`, `info`, etc.).
    pub subcommands: &'static [SubCommand],

    /// Whether unknown subcommands are accepted (for dialect packs).
    pub allow_unknown_subcommands: bool,

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

    /// WASM-runtime codegen hook ID — picks the per-command
    /// emitter on the WASM target. Currently always `None`
    /// (no WASM-specific emitters landed yet); the field exists so
    /// the per-command coverage audit can track WASM hook stamping
    /// without a follow-up registry refactor.
    pub wasm_codegen_hook: Option<WasmCodegenHookId>,

    /// Structured side-effect declarations.
    pub side_effects: &'static [SideEffect],

    /// Inferred storage type for the target variable (`Dict`, `List`, `Array`).
    pub inferred_storage_type: Option<StorageType>,

    /// Package requirement (command only visible when package is `require`d).
    pub required_package: Option<&'static str>,

    /// Excluded iRules events.
    pub excluded_events: &'static [&'static str],

    /// Options declared on the command (for completion and arity adjustment).
    pub options: &'static [OptionSpec],

    /// Whether `ArgRole::Body` arguments of this command run in the
    /// caller's frame ([`BodyKind::Plain`]) or in a separate
    /// definition / dispatch context ([`BodyKind::Structural`]).
    ///
    /// `Structural` opts every body arg out of the enclosing block's
    /// data flow (SSA, def-use scans, dead-store detection).  Default
    /// `Plain` keeps existing specs unchanged.  Mirrors Python's
    /// `body_kind` field on the command spec (introduced in
    /// `88970edc` / `91daf5c2`, closes `#250`).
    pub body_kind: BodyKind,

    /// Number of runtime-supplied positional args the body's first
    /// command receives.  Used by proc-call arity checks to relax
    /// static arity bounds on a `Body`-marked argument that is
    /// invoked as a command prefix (e.g.
    /// `fileutil::updateInPlace path cmd` appends file contents to
    /// `cmd` at runtime).
    ///
    /// Default `0` keeps every existing spec correct.  Mirrors Python's
    /// `body_arg_implicit_args` (introduced in `e30b6ae9`, closes
    /// `#308`).
    pub body_arg_implicit_args: u8,

    // --- GAP-D2: granular taint / security metadata ---------------------
    //
    // Ports the per-command taint fields from the Python `CommandSpec`
    // (`core/commands/registry/models.py`) and the `TAINT_HINTS` /
    // `_sinks.py` substrate. The consumer chat
    // (`tcl_compiler::taint`) reads these to drive the
    // W102/W103/W300/W301/W303/W309/W310/W312 + T106 + W313 emitters.
    //
    /// Output-sink diagnostic code emitted when tainted data reaches
    /// this command's output position (e.g. `"T101"` for `puts`,
    /// `"IRULE3001"` for `HTTP::respond`). `None` = not an output sink.
    /// Mirrors Python `taint_output_sink`.
    pub taint_output_sink: Option<&'static str>,

    /// When non-empty, restricts [`Self::taint_output_sink`] to apply
    /// only when the first argument (subcommand) is in this set
    /// (e.g. `HTTP::header insert|replace`). Empty = applies to every
    /// invocation. Mirrors Python `taint_output_sink_subcommands`
    /// (`None` ⇒ empty slice here).
    pub taint_output_sink_subcommands: &'static [&'static str],

    /// Log-injection sink diagnostic code (e.g. `"IRULE3003"` for the
    /// iRules `log` command). `None` = not a log sink. Mirrors Python
    /// `taint_log_sink`.
    pub taint_log_sink: Option<&'static str>,

    /// Argument indices (0-based after the command name) that take a
    /// network address — SSRF sinks (`socket`, `HTTP::host`, …).
    /// `None` = not a network sink; `Some(&[])` = network sink whose
    /// dangerous-arg positions are unspecified. Mirrors Python
    /// `taint_network_sink_args`.
    pub taint_network_sink_args: Option<&'static [u8]>,

    /// Subcommands that evaluate code in another interpreter
    /// (`interp eval`, `interp invokehidden`) — cross-interpreter
    /// code-execution sinks (T105). Empty = none. Mirrors Python
    /// `taint_interp_eval_subcommands`.
    pub taint_interp_eval_subcommands: &'static [&'static str],

    /// Colour bits this command *adds* to a tainted value it returns —
    /// a sanitising transform (`uri::encode` ⇒ `URL_ENCODED`,
    /// `file join` ⇒ `PATH_JOINED`). `None` = no transform. Mirrors
    /// Python `taint_transform`.
    pub taint_transform: Option<TaintColour>,

    /// Colour whose presence on the *input* means this command would
    /// double-encode the value (T106). `None` = no double-encode
    /// detection. Mirrors Python `taint_double_encode_colour`.
    pub taint_double_encode_colour: Option<TaintColour>,

    /// Colour that suppresses the dangerous-sink warning (T100) for
    /// this sink — e.g. `SHELL_ATOM` for `exec`, `LIST_CANONICAL` for
    /// `eval`/`uplevel`. `None` = no suppression colour. Mirrors Python
    /// `taint_sink_safe_colour`.
    pub taint_sink_safe_colour: Option<TaintColour>,

    /// Option flags whose value carries a secret (e.g. `-password`,
    /// `-headers`) — drives credential-exposure checks. Empty = none.
    /// Mirrors Python `credential_options`.
    pub credential_options: &'static [&'static str],

    /// HTTP header names whose values are secrets (e.g.
    /// `authorization`, `cookie`). Empty = none. Mirrors Python
    /// `sensitive_headers`.
    pub sensitive_headers: &'static [&'static str],

    /// Setter-form argument constraints (IRULE3101). Empty = none.
    /// The registry-driven replacement for the hardcoded
    /// `SETTER_CONSTRAINTS` table in `tcl_compiler::taint`. Mirrors the
    /// Python `TaintHint.setter_constraints`.
    pub setter_constraints: &'static [SetterConstraint],

    // --- GAP-D1: structured spec fields ---------------------------------
    //
    /// Kind of pattern language this command's pattern argument uses
    /// (`regexp`/`regsub` ⇒ `Regex`), for semantic-token sub-tokens and
    /// pattern validation. `None` = not a pattern command. Mirrors
    /// Python `pattern_type`.
    pub pattern_type: Option<PatternType>,

    /// Kind of format string this command's format argument uses
    /// (`format`/`scan` ⇒ `Sprintf`, …), for inlay-hint parsing and
    /// semantic-token sub-tokens. `None` = not a format command.
    /// Mirrors Python `format_string_type`.
    pub format_string_type: Option<FormatType>,

    /// Tcllib package that provides this command, for per-document
    /// activation via `package require`. `None` = core/built-in.
    /// Mirrors Python `tcllib_package`.
    pub tcllib_package: Option<&'static str>,

    /// Whether W120 (missing-import) fires when this package-gated
    /// command is used without a `package require`. Default `true`; set
    /// `false` for Tk commands (`wish` auto-loads Tk). Mirrors Python
    /// `warn_missing_import`.
    pub warn_missing_import: bool,

    /// Whether this command's source namespace exports it via
    /// `namespace export <bare>`, making the bare name eligible after
    /// `namespace import`. Mirrors Python `is_namespace_exported`.
    pub is_namespace_exported: bool,

    /// XC (cross-compile) translatability override: `None` = default
    /// rules, `Some(false)` = never translatable, `Some(true)` =
    /// translatable despite a namespace prefix. Mirrors Python
    /// `xc_translatable`.
    pub xc_translatable: Option<bool>,

    /// XC operation this command maps to, when it is translatable.
    /// `None` = no explicit mapping. Mirrors Python `xc_operation`.
    pub xc_operation: Option<&'static str>,

    /// Replacement command name for a deprecated command, surfaced by
    /// the deprecation code action. `None` = not deprecated. Mirrors
    /// Python `deprecated_replacement` (the resolved name).
    pub deprecated_replacement: Option<&'static str>,
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
        return_type: None,
        arg_types: &[],
        subcommands: &[],
        allow_unknown_subcommands: false,
        hover: None,
        forms: &[],
        command_forms: &[],
        assigns_variable_at: None,
        safe_on_uninit: None,
        const_fold: None,
        const_fold_versioned: None,
        lowering_hook: None,
        codegen_hook: None,
        wasm_codegen_hook: None,
        side_effects: &[],
        inferred_storage_type: None,
        required_package: None,
        excluded_events: &[],
        options: &[],
        body_kind: BodyKind::Plain,
        body_arg_implicit_args: 0,
        taint_output_sink: None,
        taint_output_sink_subcommands: &[],
        taint_log_sink: None,
        taint_network_sink_args: None,
        taint_interp_eval_subcommands: &[],
        taint_transform: None,
        taint_double_encode_colour: None,
        taint_sink_safe_colour: None,
        credential_options: &[],
        sensitive_headers: &[],
        setter_constraints: &[],
        pattern_type: None,
        format_string_type: None,
        tcllib_package: None,
        warn_missing_import: true,
        is_namespace_exported: false,
        xc_translatable: None,
        xc_operation: None,
        deprecated_replacement: None,
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

    /// Look up a subcommand by name.
    #[must_use]
    pub fn subcommand(&self, name: &str) -> Option<&SubCommand> {
        self.subcommands.iter().find(|s| s.name == name)
    }

    /// Return static arg role for a given index, if declared.
    #[must_use]
    pub fn arg_role_at(&self, index: u8) -> Option<ArgRole> {
        self.arg_roles
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, r)| *r)
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
    /// Mirrors `CommandSpec.switch_names` in
    /// `core/commands/registry/models.py`: walks the command's
    /// declared options (both the flat [`Self::options`] list and
    /// every [`CommandForm`](crate::CommandForm)'s options) and keeps
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
// every command-spec literal; deferred to its own chunk.
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

    /// Return type.
    pub return_type: Option<TclType>,

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

    /// WASM-runtime codegen hook ID. See
    /// [`CommandSpec::wasm_codegen_hook`].
    pub wasm_codegen_hook: Option<WasmCodegenHookId>,

    /// Per-subcommand options.
    pub options: &'static [OptionSpec],

    /// Enumerable positional-argument values, keyed by 0-based
    /// argument index *after* the subcommand word.  Drives
    /// value completion — e.g. `string is <class>` declares
    /// `(0, &[alnum, alpha, …])` so the character classes
    /// complete at the first sub-arg.  Mirrors
    /// `SubCommand.arg_values` in
    /// `core/commands/registry/models.py`.
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

    /// Implicit-args count for proc-call arity relaxation.  See
    /// [`CommandSpec::body_arg_implicit_args`].
    pub body_arg_implicit_args: u8,

    // --- GAP-D2: granular taint / security metadata ---------------------
    //
    /// Colour bits this subcommand adds to a tainted value it returns
    /// (`file join` ⇒ `PATH_JOINED`, `file normalize` ⇒
    /// `PATH_NORMALISED`). `None` = no transform. Mirrors Python
    /// `SubCommand.taint_transform`.
    pub taint_transform: Option<TaintColour>,

    /// Colour whose presence on the input means this subcommand would
    /// double-encode the value (T106). `None` = none. Mirrors Python
    /// `SubCommand.taint_double_encode_colour`.
    pub taint_double_encode_colour: Option<TaintColour>,

    /// Output-sink diagnostic code for a subcommand-shaped XSS /
    /// header-injection sink (e.g. `"IRULE3002"`). `None` = not a
    /// sink. Mirrors Python `SubCommand.taint_output_sink`.
    pub taint_output_sink: Option<&'static str>,

    /// Argument index (0-based after the subcommand word) carrying a
    /// credential value, for credential-exposure checks. `None` =
    /// none. Mirrors Python `SubCommand.credential_arg`.
    pub credential_arg: Option<u8>,

    /// HTTP header names whose values are secrets, for a
    /// subcommand-shaped header sink. Empty = none. Mirrors Python
    /// `SubCommand.sensitive_headers`.
    pub sensitive_headers: &'static [&'static str],

    // --- GAP-D1: structured spec fields (subcommand overrides) ----------
    //
    /// Pattern-language override for this subcommand (`string match`
    /// ⇒ `Glob`), taking priority over the parent command's
    /// [`CommandSpec::pattern_type`]. Mirrors Python
    /// `SubCommand.pattern_type`.
    pub pattern_type: Option<PatternType>,

    /// Format-string override for this subcommand (`clock format`
    /// ⇒ `Clock`, `binary scan` ⇒ `Binary`), taking priority over the
    /// parent command's [`CommandSpec::format_string_type`]. Mirrors
    /// Python `SubCommand.format_string_type`.
    pub format_string_type: Option<FormatType>,

    /// XC operation this subcommand maps to. `None` = no explicit
    /// mapping. Mirrors Python `SubCommand.xc_operation`.
    pub xc_operation: Option<&'static str>,
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
        return_type: None,
        arg_types: &[],
        pure: false,
        mutator: false,
        const_fold: None,
        const_fold_versioned: None,
        lowering_hook: None,
        codegen_hook: None,
        wasm_codegen_hook: None,
        options: &[],
        arg_values: &[],
        subcommand_forms: &[],
        dialects: None,
        safe_on_uninit: None,
        loop_list_header: false,
        creates_scope_alias: false,
        inferred_storage_type: None,
        body_kind: BodyKind::Plain,
        body_arg_implicit_args: 0,
        taint_transform: None,
        taint_double_encode_colour: None,
        taint_output_sink: None,
        credential_arg: None,
        sensitive_headers: &[],
        pattern_type: None,
        format_string_type: None,
        xc_operation: None,
    };

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

    /// Look up a static arg role by index.
    #[must_use]
    pub fn arg_role_at(&self, index: u8) -> Option<ArgRole> {
        self.arg_roles
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, r)| *r)
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
