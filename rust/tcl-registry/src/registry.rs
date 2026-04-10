//! Command registry — lookup facade.
//!
//! Built once at startup from command spec modules, then queried by
//! every consumer. Supports dialect filtering and trait-indexed
//! queries.

use std::collections::HashMap;

use crate::arg_role::ArgRole;
use crate::dialects::DialectSet;
use crate::spec::CommandSpec;
use crate::traits::Traits;

/// Lookup facade over command specs.
///
/// The registry is built once from the command spec modules and then
/// queried read-only. All command-specific knowledge lives in the
/// specs — consumers never match on command name strings.
pub struct CommandRegistry {
    by_name: HashMap<String, Vec<CommandSpec>>,
    loaded_dialects: DialectSet,
}

impl CommandRegistry {
    /// Build the default registry with core Tcl commands.
    #[must_use]
    pub fn build_default() -> Self {
        let specs = crate::commands::tcl::tcl_command_specs();
        let mut registry = Self {
            by_name: HashMap::new(),
            loaded_dialects: DialectSet::empty(),
        };
        for spec in specs {
            registry.insert(spec);
        }
        registry
    }

    /// Insert a command spec into the registry.
    pub fn insert(&mut self, spec: CommandSpec) {
        self.by_name
            .entry(spec.name.to_owned())
            .or_default()
            .push(spec);
    }

    /// Look up a command spec by name (dialect-agnostic).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CommandSpec> {
        self.by_name.get(name).and_then(|v| v.last())
    }

    /// Look up a command spec filtered by dialect.
    #[must_use]
    pub fn get_for_dialect(&self, name: &str, dialect: DialectSet) -> Option<&CommandSpec> {
        self.by_name
            .get(name)
            .and_then(|specs| specs.iter().rev().find(|s| s.supports_dialect(dialect)))
    }

    /// Return all registered command names.
    pub fn command_names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }

    /// Return all command specs whose traits include `t`.
    #[must_use]
    pub fn commands_with_trait(&self, t: Traits) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .last()
                    .filter(|s| s.traits.contains(t))
                    .map(|_| name.as_str())
            })
            .collect()
    }

    /// Resolve argument indices for a given role.
    ///
    /// For subcommand-based commands (e.g. `dict create`), pass the
    /// subcommand as the first element of `args`. This is the Rust
    /// equivalent of Python's `arg_indices_for_role()`.
    #[must_use]
    pub fn arg_indices_for_role(&self, name: &str, args: &[&str], role: ArgRole) -> Vec<usize> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };

        // Check subcommand
        if !spec.subcommands.is_empty() && !args.is_empty() {
            if let Some(sub) = spec.subcommand(args[0]) {
                // Try dynamic resolver first
                if let Some(resolver) = sub.arg_role_resolver {
                    return resolver(&args[1..])
                        .into_iter()
                        .filter(|(_, r)| *r == role)
                        .map(|(i, _)| i as usize + 1) // +1 for subcommand word
                        .collect();
                }
                // Static roles (offset by +1 for subcommand word)
                return sub
                    .arg_roles
                    .iter()
                    .filter(|(_, r)| *r == role)
                    .map(|(i, _)| *i as usize + 1)
                    .collect();
            }
        }

        // Top-level: try dynamic resolver first
        if let Some(resolver) = spec.arg_role_resolver {
            return resolver(args)
                .into_iter()
                .filter(|(_, r)| *r == role)
                .map(|(i, _)| i as usize)
                .collect();
        }

        // Static roles
        spec.arg_roles
            .iter()
            .filter(|(_, r)| *r == role)
            .map(|(i, _)| *i as usize)
            .collect()
    }

    /// Number of registered commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field("commands", &self.by_name.len())
            .field("loaded_dialects", &self.loaded_dialects)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_default_has_commands() {
        let reg = CommandRegistry::build_default();
        assert!(!reg.is_empty());
        assert!(reg.get("for").is_some());
        assert!(reg.get("set").is_some());
        assert!(reg.get("nonexistent_command").is_none());
    }

    #[test]
    fn lookup_for_command() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get("for").unwrap();
        assert_eq!(spec.name, "for");
        assert!(spec.traits.contains(Traits::CONTROL_FLOW));
        assert!(spec.traits.contains(Traits::HAS_LOOP_BODY));
        assert_eq!(spec.arity, crate::arity::Arity::exact(4));
    }

    #[test]
    fn arg_roles_for_static_command() {
        let reg = CommandRegistry::build_default();
        let bodies =
            reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Body);
        assert!(bodies.contains(&0)); // init
        assert!(bodies.contains(&2)); // next
        assert!(bodies.contains(&3)); // body
        assert!(!bodies.contains(&1)); // cond is Expr, not Body
    }

    #[test]
    fn arg_roles_for_expr() {
        let reg = CommandRegistry::build_default();
        let exprs =
            reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Expr);
        assert_eq!(exprs, vec![1]); // only the condition
    }

    #[test]
    fn commands_with_trait_query() {
        let reg = CommandRegistry::build_default();
        let control_flow = reg.commands_with_trait(Traits::CONTROL_FLOW);
        assert!(control_flow.contains(&"for"));
        assert!(control_flow.contains(&"if"));
        assert!(control_flow.contains(&"while"));
        assert!(!control_flow.contains(&"puts"));
    }

    #[test]
    fn dialect_filter() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get_for_dialect("dict", DialectSet::TCL86);
        assert!(spec.is_some());
        // dict is tcl8.5+ so should NOT be available in 8.4
        let spec84 = reg.get_for_dialect("dict", DialectSet::TCL84);
        assert!(spec84.is_none());
    }

    #[test]
    fn subcommand_arg_roles() {
        let reg = CommandRegistry::build_default();
        let bodies =
            reg.arg_indices_for_role("dict", &["for", "{k v}", "$d", "body"], ArgRole::Body);
        // dict for {varList} dictExpr body → body is at index 3 (subcmd=0, args 1-based +1)
        assert!(bodies.contains(&3));
    }

    #[test]
    fn variable_write_commands() {
        let reg = CommandRegistry::build_default();
        let set_vars = reg.arg_indices_for_role("set", &["x", "1"], ArgRole::VarWrite);
        assert_eq!(set_vars, vec![0]);
    }

    #[test]
    fn dynamic_arg_role_resolution() {
        let reg = CommandRegistry::build_default();
        // if expr body elseif expr body else body
        let roles = reg.arg_indices_for_role(
            "if",
            &[
                "$x", "then", "body1", "elseif", "$y", "body2", "else", "body3",
            ],
            ArgRole::Body,
        );
        // Bodies are at positions 2, 5, 7
        assert!(roles.contains(&2));
        assert!(roles.contains(&5));
        assert!(roles.contains(&7));
    }
}
