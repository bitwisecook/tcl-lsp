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
use crate::hooks::{ArgTypeHint, CodegenHookId, ConstFoldFn, LoweringHookId, WasmCodegenHookId};
use crate::hover::{ArgValue, FormSpec, HoverSnippet, OptionSpec};
use crate::side_effects::{SideEffect, StorageType};
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
    };

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
    };

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
