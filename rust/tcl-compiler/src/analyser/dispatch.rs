//! Command-signature lookup — Rust port of
//! ``_signature_for_command`` in
//! ``core/analysis/_analyser/_commands.py:74-93``.
//!
//! Per-handler dispatch consults the command registry to learn
//! how many arguments a command accepts and what role each
//! argument plays. The Python helper returns one of three
//! shapes:
//!
//! - [`CommandSig`] — a simple command (`set`, `proc`, `puts`).
//! - [`SubcommandSig`] — a command that dispatches on its first
//!   argument (`namespace eval`, `dict get`, `string length`,
//!   `info args`).
//! - `None` — the command isn't in the registry.
//!
//! The Python source also falls back to a module-level
//! ``SIGNATURES`` dict for commands not in the registry; that
//! dict lives in ``core.commands.registry.runtime`` and is
//! initialised empty (an extension point that nothing currently
//! populates), so the Rust port skips it. If a future Python
//! change starts populating ``SIGNATURES``, mirror the data here.

use std::collections::HashMap;

use tcl_registry::prelude::DialectSet;
use tcl_registry::{ArgRole, Arity, CommandRegistry};

/// Signature for a simple Tcl command.
///
/// Mirrors ``CommandSig`` in
/// ``core/commands/registry/signatures.py:60``.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSig {
    /// Argument-count bounds.
    pub arity: Arity,
    /// Static arg-index → role map (0-based, after the command
    /// name). Args not listed default to ``ArgRole::Value``.
    pub arg_roles: HashMap<u8, ArgRole>,
}

/// Signature for a command that dispatches on a subcommand word.
///
/// Mirrors ``SubcommandSig`` in
/// ``core/commands/registry/signatures.py:86``.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubcommandSig {
    /// Subcommand name → [`CommandSig`] mapping. Empty for
    /// commands that haven't yet had their subcommand table
    /// populated in the registry.
    pub subcommands: HashMap<String, CommandSig>,
    /// When `true`, unknown subcommands are not flagged as
    /// diagnostics — used for generated dialect packs.
    pub allow_unknown: bool,
}

/// What ``signature_for_command`` returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandSignature {
    /// A simple command.
    Simple(CommandSig),
    /// A command with subcommands.
    WithSubcommands(SubcommandSig),
}

/// Look up signature metadata for a command.
///
/// Mirrors ``_signature_for_command`` in
/// ``core/analysis/_analyser/_commands.py:74-93``. Returns:
///
/// - [`CommandSignature::WithSubcommands`] when the spec has
///   non-empty subcommands.
/// - [`CommandSignature::Simple`] when the spec exists but
///   has no subcommands.
/// - `None` when the registry doesn't know the command.
///
/// The `dialect` argument selects which dialect-specific subcommand
/// set is materialised; pass `DialectSet::ALL_TCL` when the
/// caller has no specific dialect context.
#[must_use]
pub fn signature_for_command(
    registry: &CommandRegistry,
    cmd_name: &str,
    dialect: DialectSet,
) -> Option<CommandSignature> {
    let spec = registry.get_for_dialect(cmd_name, dialect)?;

    if !spec.subcommands.is_empty() {
        let mut subs: HashMap<String, CommandSig> = HashMap::new();
        for sub in spec.subcommands {
            // `dialects` filters out subcommands not available in
            // the current dialect — mirrors
            // `subcommands_for_dialect` in Python.
            if let Some(spec_dialects) = sub.dialects {
                if !spec_dialects.intersects(dialect) {
                    continue;
                }
            }
            let arg_roles = sub
                .arg_roles
                .iter()
                .map(|(idx, role)| (*idx, *role))
                .collect();
            subs.insert(
                sub.name.to_string(),
                CommandSig {
                    arity: sub.arity,
                    arg_roles,
                },
            );
        }
        return Some(CommandSignature::WithSubcommands(SubcommandSig {
            subcommands: subs,
            allow_unknown: spec.allow_unknown_subcommands,
        }));
    }

    let arg_roles = spec
        .arg_roles
        .iter()
        .map(|(idx, role)| (*idx, *role))
        .collect();
    Some(CommandSignature::Simple(CommandSig {
        arity: spec.arity,
        arg_roles,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn unknown_command_returns_none() {
        let reg = registry();
        let sig = signature_for_command(&reg, "definitely_not_a_command_xyz", DialectSet::ALL_TCL);
        assert!(sig.is_none());
    }

    #[test]
    fn simple_command_returns_simple_sig() {
        let reg = registry();
        let sig = signature_for_command(&reg, "set", DialectSet::ALL_TCL)
            .expect("set should be in registry");
        let CommandSignature::Simple(cs) = sig else {
            panic!("expected Simple, got {sig:?}");
        };
        // `set var ?value?` — arity is 1..=2.
        assert!(cs.arity.accepts(1) || cs.arity.accepts(2));
    }

    #[test]
    fn subcommand_command_returns_with_subcommands() {
        let reg = registry();
        let sig = signature_for_command(&reg, "string", DialectSet::ALL_TCL)
            .expect("string should be in registry");
        let CommandSignature::WithSubcommands(scs) = sig else {
            panic!("expected WithSubcommands, got {sig:?}");
        };
        // `string length`, `string index`, etc. should be
        // populated for any 8.5+ dialect.
        assert!(
            !scs.subcommands.is_empty(),
            "expected non-empty subcommands for `string`"
        );
    }

    #[test]
    fn proc_returns_simple_sig_with_arity() {
        let reg = registry();
        let sig =
            signature_for_command(&reg, "proc", DialectSet::ALL_TCL).expect("proc should be there");
        let CommandSignature::Simple(cs) = sig else {
            panic!("proc should be Simple");
        };
        // `proc name args body` — exactly 3 args.
        assert!(cs.arity.accepts(3));
    }

    #[test]
    fn dialect_filter_changes_subcommand_visibility() {
        let reg = registry();
        // `info` exists in every Tcl dialect; we just verify the
        // helper returns a non-empty subcommand map under a
        // narrow dialect.
        let sig =
            signature_for_command(&reg, "info", DialectSet::TCL84).expect("info present in 8.4");
        let CommandSignature::WithSubcommands(scs) = sig else {
            panic!("info should have subcommands");
        };
        assert!(scs.subcommands.contains_key("body"));
    }
}
