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
    /// Build the default registry with core Tcl + stdlib + tcllib commands.
    #[must_use]
    pub fn build_default() -> Self {
        let mut registry = Self {
            by_name: HashMap::new(),
            loaded_dialects: DialectSet::empty(),
        };
        for spec in crate::commands::tcl::tcl_command_specs() {
            registry.insert(spec);
        }
        for spec in crate::commands::stdlib::stdlib_command_specs() {
            registry.insert(spec);
        }
        for spec in crate::commands::tcllib::tcllib_command_specs() {
            registry.insert(spec);
        }
        registry
    }

    /// Load a dialect's commands into the registry (idempotent).
    pub fn load_dialect(&mut self, dialect: DialectSet) {
        if self.loaded_dialects.contains(dialect) {
            return;
        }
        let specs: Vec<CommandSpec> = match dialect {
            d if d == DialectSet::IRULES => crate::commands::irules::irules_command_specs(),
            d if d == DialectSet::IAPPS => crate::commands::iapps::iapps_command_specs(),
            d if d == DialectSet::TK => crate::commands::tk::tk_command_specs(),
            d if d == DialectSet::EXPECT => crate::commands::expect::expect_command_specs(),
            d if d == DialectSet::SYNOPSYS => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_synopsys::eda_synopsys_command_specs());
                v
            }
            d if d == DialectSet::CADENCE => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_cadence::eda_cadence_command_specs());
                v
            }
            d if d == DialectSet::XILINX => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_xilinx::eda_xilinx_command_specs());
                v
            }
            d if d == DialectSet::QUARTUS => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_quartus::eda_quartus_command_specs());
                v
            }
            d if d == DialectSet::MENTOR => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_mentor::eda_mentor_command_specs());
                v
            }
            _ => Vec::new(),
        };
        for spec in specs {
            self.insert(spec);
        }
        self.loaded_dialects |= dialect;
    }

    /// Load iRules dialect commands (convenience wrapper).
    pub fn load_irules(&mut self) {
        self.load_dialect(DialectSet::IRULES);
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
        let n = args.len();

        // Check subcommand
        if !spec.subcommands.is_empty() && !args.is_empty() {
            if let Some(sub) = spec.subcommand(args[0]) {
                // Try dynamic resolver first
                if let Some(resolver) = sub.arg_role_resolver {
                    return resolver(&args[1..])
                        .into_iter()
                        .filter(|(_, r)| *r == role)
                        .map(|(i, _)| i as usize + 1) // +1 for subcommand word
                        .filter(|&idx| idx < n)
                        .collect();
                }
                // Static roles (offset by +1 for subcommand word)
                return sub
                    .arg_roles
                    .iter()
                    .filter(|(_, r)| *r == role)
                    .map(|(i, _)| *i as usize + 1)
                    .filter(|&idx| idx < n)
                    .collect();
            }
        }

        // Top-level: try dynamic resolver first
        if let Some(resolver) = spec.arg_role_resolver {
            return resolver(args)
                .into_iter()
                .filter(|(_, r)| *r == role)
                .map(|(i, _)| i as usize)
                .filter(|&idx| idx < n)
                .collect();
        }

        // Static roles — filter by args length to avoid out-of-range indices
        spec.arg_roles
            .iter()
            .filter(|(_, r)| *r == role)
            .map(|(i, _)| *i as usize)
            .filter(|&idx| idx < n)
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

    #[test]
    fn load_irules_dialect() {
        let mut reg = CommandRegistry::build_default();
        assert!(reg.get("HTTP::header").is_none()); // not loaded yet
        reg.load_irules();
        assert!(reg.get("HTTP::header").is_some());
        assert!(reg.len() > 200); // should have 1000+ commands now
    }

    #[test]
    fn irules_command_has_irules_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let spec = reg.get("HTTP::header").unwrap();
        assert_eq!(spec.dialects, Some(DialectSet::IRULES));
    }

    #[test]
    fn irules_idempotent_load() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let count1 = reg.len();
        reg.load_irules(); // second load should be no-op
        assert_eq!(reg.len(), count1);
    }

    #[test]
    fn default_includes_stdlib_and_tcllib() {
        let reg = CommandRegistry::build_default();
        // stdlib and tcllib are loaded by default
        assert!(reg.len() > 200);
    }

    #[test]
    fn load_tk_dialect() {
        let base = CommandRegistry::build_default();
        let base_count = base.len();
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(DialectSet::TK);
        assert!(reg.len() > base_count, "Tk should add commands");
    }

    #[test]
    fn load_iapps_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(DialectSet::IAPPS);
        assert!(reg.len() > 100);
    }

    #[test]
    fn load_expect_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(DialectSet::EXPECT);
        assert!(reg.get("expect").is_some() || reg.get("spawn").is_some());
    }

    #[test]
    fn load_eda_synopsys() {
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(DialectSet::SYNOPSYS);
        assert!(reg.len() > 100);
    }
}
