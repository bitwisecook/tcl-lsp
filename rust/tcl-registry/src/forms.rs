//! Command and subcommand form descriptors.
//!
//! A *form* is one concrete invocation shape of a command — for
//! example `dict create ...` and `dict for ...` are two forms of
//! `dict`, and `lset name index value` and `lset name idx1 idx2 value`
//! are two forms of `lset`. Forms carry their own arity, argument
//! roles, options, dialect applicability, and lowering / codegen hook
//! identifiers, so consumers can ask the registry "which form does
//! this call match?" rather than reimplementing the dispatch table.
//!
//! The thin [`crate::hover::FormSpec`] survives for hover and
//! completion text, where only the synopsis matters; the descriptors
//! defined here drive compiler routing in ARCH2.

use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::dialects::DialectSet;
use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::hover::OptionSpec;

/// Self-contained metadata for one invocation form of a top-level
/// command.
///
/// `arity` and `arg_roles` are expressed against the argument list
/// *after* the command name, matching how
/// [`crate::CommandSpec::arg_roles`] is laid out.
#[derive(Debug, Clone, Copy)]
pub struct CommandForm {
    /// Form display name (`"default"`, `"with-amount"`, …) — used by
    /// completion and diagnostics.
    pub name: &'static str,

    /// Argument count constraint after the command name.
    pub arity: Arity,

    /// Static argument roles (no dynamic resolver — forms are
    /// already disambiguated by arity / option presence, so role
    /// resolution is positional).
    pub arg_roles: &'static [(u8, ArgRole)],

    /// Options recognised on this form.
    pub options: &'static [OptionSpec],

    /// Dialects in which this form applies. `None` = inherit from
    /// the parent [`crate::CommandSpec`] / [`crate::SubCommand`].
    pub dialects: Option<DialectSet>,

    /// Lowering hook identifier for this form.
    pub lowering_hook: Option<LoweringHookId>,

    /// Codegen hook identifier for this form.
    pub codegen_hook: Option<CodegenHookId>,
}

impl CommandForm {
    /// Default value for all fields — used with `..CommandForm::DEFAULT`.
    pub const DEFAULT: Self = Self {
        name: "",
        arity: Arity::any(),
        arg_roles: &[],
        options: &[],
        dialects: None,
        lowering_hook: None,
        codegen_hook: None,
    };
}

/// Self-contained metadata for one invocation form of a subcommand.
///
/// Identical layout to [`CommandForm`]; the type alias keeps the
/// call sites self-documenting at the point of use.
pub type SubCommandForm = CommandForm;
