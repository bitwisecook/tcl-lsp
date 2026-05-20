//! Command registry — lookup facade.
//!
//! Built once at startup from command spec modules, then queried by
//! every consumer. Supports dialect filtering and trait-membership
//! queries.

use std::collections::HashMap;

use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::dialects::DialectSet;
use crate::forms::CommandForm;
use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::spec::{CommandSpec, SubCommand};
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

    /// Whether `name` is a core Tcl built-in carrying the
    /// [`Traits::BYTE_COMPILED`] trait — i.e. the minifier must not
    /// rewrite this command head to a `$var` alias.  Checks every
    /// registered spec for the name (not just the dialect-preferred
    /// one) so the core stamp is honoured even when a dialect layers
    /// an additional spec under the same name.
    #[must_use]
    pub fn is_byte_compiled(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::BYTE_COMPILED))
        })
    }

    /// Whether `name` carries the [`Traits::NOT_PROC_FACTORY`] trait —
    /// a registered command head that incidentally matches the
    /// proc-factory token shape but is not a factory wrapper.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under
    /// the name.
    #[must_use]
    pub fn is_not_proc_factory(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::NOT_PROC_FACTORY))
        })
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

    /// Resolve a concrete call to its registry-described form.
    ///
    /// Given the command head `name` and the literal argument words
    /// `args`, returns a [`ResolvedCall`] describing which
    /// [`CommandSpec`] / [`SubCommand`] / [`CommandForm`] the call
    /// matches and which lowering / codegen hook applies. The
    /// returned reference borrows from the registry, so callers must
    /// not retain it across registry mutation.
    ///
    /// Returns `None` when the command is unknown to the registry.
    #[must_use]
    pub fn resolve_call<'r>(
        &'r self,
        name: &str,
        args: &[&str],
        dialect: DialectSet,
    ) -> Option<ResolvedCall<'r>> {
        let spec = if dialect.is_empty() {
            self.get(name)?
        } else {
            self.get_for_dialect(name, dialect)?
        };

        let mut resolved = ResolvedCall {
            spec,
            sub: None,
            form: None,
            lowering_hook: spec.lowering_hook,
            codegen_hook: spec.codegen_hook,
        };

        if !spec.subcommands.is_empty() {
            if let Some(first) = args.first() {
                if let Some(sub) = spec.subcommand(first) {
                    // Re-slice rather than allocating a fresh `Vec<&str>`
                    // — `resolve_call` is on the lowering / codegen /
                    // analysis hot path.
                    let sub_args: &[&str] = args.get(1..).unwrap_or(&[]);
                    let form = pick_form(sub.subcommand_forms, sub_args, dialect);
                    resolved.sub = Some(sub);
                    resolved.lowering_hook = form
                        .and_then(|f| f.lowering_hook)
                        .or(sub.lowering_hook)
                        .or(spec.lowering_hook);
                    resolved.codegen_hook = form
                        .and_then(|f| f.codegen_hook)
                        .or(sub.codegen_hook)
                        .or(spec.codegen_hook);
                    resolved.form = form;
                    return Some(resolved);
                }
            }
        }

        let form = pick_form(spec.command_forms, args, dialect);
        if let Some(f) = form {
            resolved.lowering_hook = f.lowering_hook.or(spec.lowering_hook);
            resolved.codegen_hook = f.codegen_hook.or(spec.codegen_hook);
            resolved.form = Some(f);
        }
        Some(resolved)
    }

    /// Resolve the option-terminator profile for a command invocation.
    ///
    /// Mirrors Python's `core/commands/registry/command_registry.py::resolve_option_terminator`.
    /// Matches the invocation's first argument against subcommands
    /// that declare an [`OptionSpec`](crate::hover::OptionSpec) with
    /// `name == "--"`, then falls back to form-level `--` declarations.
    /// Returns `None` when the command does not declare a `--`
    /// terminator at all (subcommand-scoped or form-scoped).
    ///
    /// Drives the W304 ("missing option terminator") diagnostic — the
    /// returned `scan_start` index, `subcommand` label, and
    /// `options` slice tell the caller where to start scanning for
    /// the first positional argument and which option specs to
    /// consult for value-consuming options (so they're not mistaken
    /// for positionals).  Returning a borrowed `&'static [OptionSpec]`
    /// rather than a freshly-allocated `HashSet` keeps the resolver
    /// allocation-free on the analyser hot path; per-command option
    /// counts are small (typically 1-3 value-consuming options per
    /// command), so a linear scan at the call site is cheaper than
    /// a `HashSet` build.
    ///
    /// `warn_without_terminator` lifts the
    /// [`Traits::WARN_WITHOUT_TERMINATOR`] flag from the matched
    /// command spec and surfaces it on `ResolvedTerminator` for
    /// parity with the Python registry, but the current Rust W304
    /// emitter does not consume it (mirroring Python's
    /// `_style.py`, which also stores but never reads it).  Kept
    /// on the resolver for future emit logic and so the registry
    /// API doesn't need to change when consumers start gating on
    /// it.
    #[must_use]
    pub fn resolve_option_terminator(
        &self,
        name: &str,
        args: &[&str],
        dialect: DialectSet,
    ) -> Option<ResolvedTerminator> {
        let spec = if dialect.is_empty() {
            self.get(name)?
        } else {
            self.get_for_dialect(name, dialect)?
        };

        let warn_flag = spec.traits.contains(Traits::WARN_WITHOUT_TERMINATOR);

        // Subcommand-scoped first.
        if let Some(first) = args.first() {
            if let Some(sub) = spec.subcommand(first) {
                if sub.options.iter().any(|o| o.name == "--") {
                    return Some(ResolvedTerminator {
                        scan_start: 1,
                        subcommand: Some(sub.name),
                        options: sub.options,
                        warn_without_terminator: warn_flag,
                    });
                }
            }
        }

        // Form-level fallback — Python iterates `spec.forms`; the
        // Rust port stores option specs at the `CommandSpec.options`
        // level (single set per spec) so we consult that directly
        // when no subcommand match was found.
        if spec.options.iter().any(|o| o.name == "--") {
            return Some(ResolvedTerminator {
                scan_start: 0,
                subcommand: None,
                options: spec.options,
                warn_without_terminator: warn_flag,
            });
        }

        None
    }

    /// Whether `name` (or the compound key `"cmd sub"`) produces a
    /// canonical Tcl list — a list whose elements are properly
    /// quoted so re-parsing by ``eval`` / ``uplevel`` /
    /// ``interp eval`` doesn't trigger unwanted substitution.
    ///
    /// Mirrors `core/commands/registry/runtime.py::canonical_list_commands`:
    /// derived from `return_type == TclType::List` on the command
    /// (or its subcommand entry), minus the explicit exclusion
    /// `concat` whose join-strip-grouping semantics can leave
    /// unquoted specials in the output.
    ///
    /// Drives the W101 (`eval` with string concatenation) safe-
    /// idiom suppression — `eval [list ...]`, `eval [linsert ...]`,
    /// `eval [split ...]`, etc. shouldn't fire because the inner
    /// command's output is a properly-quoted list.
    #[must_use]
    pub fn is_canonical_list_command(&self, name: &str) -> bool {
        // Exclusion: concat returns LIST but isn't canonical.
        if name == "concat" {
            return false;
        }
        // Compound form ``"cmd sub"`` — split into head + sub.
        if let Some((head, sub_name)) = name.split_once(' ') {
            if let Some(spec) = self.get(head) {
                if let Some(sub) = spec.subcommand(sub_name) {
                    return sub.return_type == Some(crate::types::TclType::List);
                }
            }
            return false;
        }
        // Bare command name.
        self.get(name)
            .and_then(|spec| spec.return_type)
            .is_some_and(|t| t == crate::types::TclType::List)
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

/// Resolved option-terminator profile for a command invocation.
///
/// Returned by [`CommandRegistry::resolve_option_terminator`].  Drives
/// the W304 ("missing option terminator") diagnostic.  Carries
/// borrowed references into the registry's static spec table; do
/// not retain across registry mutation.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedTerminator {
    /// Index in the `args` slice where positional-argument scanning
    /// begins.  `0` for form-level matches; `1` for subcommand-scoped
    /// matches (the first arg is the subcommand keyword).
    pub scan_start: usize,
    /// Subcommand keyword that owns the `--` declaration, if the
    /// match was subcommand-scoped.  `None` for form-level matches.
    pub subcommand: Option<&'static str>,
    /// Borrowed slice of every option declared on the matched
    /// command (or subcommand).  Callers consult [`crate::hover::OptionSpec::takes_value`]
    /// on each entry to determine whether an option name consumes a
    /// following value argument — done at the call site to avoid the
    /// per-resolve `HashSet` allocation a precomputed name set would
    /// require.  Per-command counts are small; a linear scan is
    /// cheaper than a heap-allocated set on the analyser hot path.
    pub options: &'static [crate::hover::OptionSpec],
    /// Lifted from [`Traits::WARN_WITHOUT_TERMINATOR`] on the matched
    /// command spec.  Surfaced here for parity with the Python
    /// registry's `ResolvedTerminator`; the current Rust W304
    /// emitter does not consume the flag (mirroring Python's
    /// `_style.py`, which stores but never reads it).  Kept on the
    /// type so future emit logic can gate on it without an API
    /// change.
    pub warn_without_terminator: bool,
}

/// Outcome of [`CommandRegistry::resolve_call`].
///
/// Carries borrowed references into the registry's spec table; the
/// resolved call describes the matched command spec, optionally a
/// matched subcommand and form, and the effective lowering / codegen
/// hook identifiers (form-level wins over subcommand-level wins
/// over command-level).
#[derive(Debug, Clone, Copy)]
pub struct ResolvedCall<'r> {
    /// The matched top-level command spec.
    pub spec: &'r CommandSpec,
    /// The matched subcommand, if the call has one.
    pub sub: Option<&'r SubCommand>,
    /// The matched form descriptor, if any.
    pub form: Option<&'r CommandForm>,
    /// Effective lowering hook identifier.
    pub lowering_hook: Option<LoweringHookId>,
    /// Effective codegen hook identifier.
    pub codegen_hook: Option<CodegenHookId>,
}

impl ResolvedCall<'_> {
    /// Effective arity for this resolved call: the form arity if a
    /// form matched, otherwise the subcommand arity, otherwise the
    /// top-level [`CommandSpec`] arity.
    #[must_use]
    pub fn arity(&self) -> Arity {
        if let Some(f) = self.form {
            return f.arity;
        }
        if let Some(s) = self.sub {
            return s.arity;
        }
        self.spec.arity
    }
}

fn pick_form<'r>(
    forms: &'r [CommandForm],
    args: &[&str],
    dialect: DialectSet,
) -> Option<&'r CommandForm> {
    let n = u16::try_from(args.len()).unwrap_or(u16::MAX);
    forms.iter().find(|f| {
        if !f.arity.accepts(n) {
            return false;
        }
        match f.dialects {
            Some(d) if !dialect.is_empty() => d.intersects(dialect),
            _ => true,
        }
    })
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
    fn tcl9_commands_from_pr_433_are_registered() {
        // SYNC-MAY19-tcl9-commands: mirrors PR #433 (0f9288d2).
        let reg = CommandRegistry::build_default();
        for name in [
            "foreachLine",
            "readFile",
            "writeFile",
            "lpop",
            "const",
            "tcl::idna",
            "::tcl::idna",
            "tcl::process",
            "::tcl::process",
        ] {
            assert!(
                reg.get(name).is_some(),
                "{name} not registered after SYNC-MAY19-tcl9-commands",
            );
        }
    }

    #[test]
    fn coroinject_coroprobe_registered() {
        // Python PR #433 also fixed the missing import that made
        // these two commands invisible to the LSP.  Rust always
        // registered them — verify they remain registered.
        let reg = CommandRegistry::build_default();
        assert!(reg.get("coroinject").is_some());
        assert!(reg.get("coroprobe").is_some());
    }

    #[test]
    fn tcl9_commands_gated_to_tcl90() {
        use crate::dialects::DialectSet;
        let reg = CommandRegistry::build_default();
        for name in ["foreachLine", "readFile", "writeFile", "lpop", "const"] {
            let spec = reg.get(name).expect("registered");
            assert_eq!(
                spec.dialects,
                Some(DialectSet::TCL90),
                "{name} should be Tcl 9.0-only",
            );
        }
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
    fn byte_compiled_covers_the_core_builtins() {
        let reg = CommandRegistry::build_default();
        // Every registered command the minifier must never alias.
        // Mirrors the former `_BUILTIN_SKIP` list (minus the
        // non-command keywords `else` / `elseif` and the unregistered
        // `pwd`).
        let expected = [
            "set",
            "unset",
            "proc",
            "if",
            "while",
            "for",
            "foreach",
            "switch",
            "return",
            "break",
            "continue",
            "expr",
            "catch",
            "try",
            "throw",
            "package",
            "namespace",
            "upvar",
            "uplevel",
            "variable",
            "global",
            "append",
            "lappend",
            "incr",
            "info",
            "string",
            "list",
            "llength",
            "lindex",
            "lrange",
            "lsort",
            "lsearch",
            "lreplace",
            "linsert",
            "dict",
            "array",
            "regexp",
            "regsub",
            "scan",
            "format",
            "open",
            "close",
            "read",
            "gets",
            "eof",
            "flush",
            "seek",
            "tell",
            "fconfigure",
            "fcopy",
            "fileevent",
            "socket",
            "after",
            "update",
            "vwait",
            "rename",
            "source",
            "eval",
            "apply",
            "tailcall",
            "error",
            "cd",
            "file",
            "glob",
            "clock",
            "binary",
            "encoding",
            "interp",
            "load",
            "exit",
            "pid",
            "exec",
            "chan",
            "puts",
        ];
        for name in expected {
            assert!(
                reg.is_byte_compiled(name),
                "{name} should carry Traits::BYTE_COMPILED"
            );
        }
        // A user-proc-like name and a command outside the curated
        // skip set must not carry the trait.
        assert!(!reg.is_byte_compiled("my_helper_proc"));
        assert!(!reg.is_byte_compiled("split"));
    }

    #[test]
    fn not_proc_factory_covers_registered_skip_heads() {
        let reg = CommandRegistry::build_default();
        // Registered heads from the former `_FACTORY_SKIP_HEADS` list
        // (the four non-command heads method / classmethod /
        // itcl::class / ::itcl::class are handled by the scanner's
        // residual set, not the registry).
        let expected = [
            "proc",
            "namespace",
            "if",
            "switch",
            "while",
            "for",
            "foreach",
            "try",
            "catch",
            "eval",
            "apply",
            "expr",
            "uplevel",
            "upvar",
            "variable",
            "set",
            "lappend",
            "dict",
            "array",
            "string",
            "list",
            "lindex",
            "package",
            "source",
            "interp",
            "oo::class",
            "oo::define",
            "oo::objdefine",
        ];
        for name in expected {
            assert!(
                reg.is_not_proc_factory(name),
                "{name} should carry Traits::NOT_PROC_FACTORY"
            );
        }
        // A real factory-wrapper head must not be skipped.
        assert!(!reg.is_not_proc_factory("my_factory"));
    }

    #[test]
    fn frameless_runtime_covers_the_audited_allow_list() {
        let reg = CommandRegistry::build_default();
        let got: std::collections::HashSet<&str> = reg
            .commands_with_trait(Traits::FRAMELESS_RUNTIME)
            .into_iter()
            .collect();
        let expected: std::collections::HashSet<&str> = [
            "list",
            "lindex",
            "lrange",
            "linsert",
            "llength",
            "lsort",
            "lsearch",
            "lappend",
            "lreverse",
            "lreplace",
            "lrepeat",
            "lassign",
            "concat",
            "split",
            "join",
            "string",
            "expr",
            "global",
            "variable",
            "upvar",
            "namespace",
            "set",
            "incr",
            "append",
            "unset",
            "puts",
            "return",
            "error",
            "continue",
            "break",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            got, expected,
            "FRAMELESS_RUNTIME stamps drifted from the audited allow-list"
        );
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

    /// SYNC4: `trace add variable name ops body` declares arg 1
    /// (the variable name) as `VarWrite` via the registry.
    #[test]
    fn arg_indices_for_role_trace_add_variable_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["add", "variable", "x", "write", "body"],
            ArgRole::VarWrite,
        );
        // +1 for subcommand offset.  arg "x" is at idx 2 in the
        // full args list (sub args[1] + 1).
        assert!(writes.contains(&2), "VarWrite writes={writes:?}");
    }

    /// SYNC4: `trace add execution` does NOT declare `VarWrite`
    /// (the second arg is a command name, not a variable).
    #[test]
    fn arg_indices_for_role_trace_add_execution_no_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["add", "execution", "foo", "enter", "body"],
            ArgRole::VarWrite,
        );
        assert!(writes.is_empty(), "VarWrite writes={writes:?}");
    }

    /// SYNC4: `trace remove variable` mirrors `trace add variable`
    /// for registry parity (alias spellings flow through the same
    /// `VarWrite` query).
    #[test]
    fn arg_indices_for_role_trace_remove_variable_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["remove", "variable", "y", "write", "body"],
            ArgRole::VarWrite,
        );
        assert!(writes.contains(&2), "VarWrite writes={writes:?}");
    }

    /// SYNC5: `global` / `variable` / `upvar` carry
    /// `CREATES_DYNAMIC_BARRIER` so SSA's barrier-def walk knows the
    /// per-arg list belongs to `var_scoping`, not the role-driven
    /// `VarWrite` query.
    #[test]
    fn creates_dynamic_barrier_trait_marks_scope_aliases() {
        let reg = CommandRegistry::build_default();
        for cmd in &["global", "variable", "upvar"] {
            assert!(
                reg.get(cmd)
                    .unwrap()
                    .traits
                    .contains(Traits::CREATES_DYNAMIC_BARRIER),
                "{cmd} should carry CREATES_DYNAMIC_BARRIER",
            );
        }
        // `set` does NOT carry the trait — its VarWrite at arg 0 is
        // a single-target def, not a vararg list.
        assert!(!reg
            .get("set")
            .unwrap()
            .traits
            .contains(Traits::CREATES_DYNAMIC_BARRIER));
    }

    /// SYNC1 acceptance: `dict with` / `dict update` arg 0 (the dict
    /// variable) plays both `VarRead` and `VarWrite` roles. Mirrors
    /// Python's `frozenset({VAR_READ, VAR_WRITE})` post-`8c95c2ee`.
    /// The Rust port emits this via duplicate `(idx, role)` rows in
    /// the resolver; the type widening to an explicit `ArgRoleSet`
    /// is deferred (the multi-role observable behaviour is already
    /// what consumers query).
    /// SYNC2: every spec defaults to `BodyKind::Plain` unless it
    /// opts into `Structural`.
    #[test]
    fn body_kind_default_plain() {
        use crate::body_kind::BodyKind;
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("set").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("if").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("while").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("foreach").unwrap().body_kind, BodyKind::Plain);
    }

    /// SYNC2: `proc` / `oo::class` / `oo::define` / `oo::objdefine`
    /// stamp `Structural` so SSA skips their body args from the
    /// enclosing block's data flow.
    #[test]
    fn body_kind_structural_marks() {
        use crate::body_kind::BodyKind;
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
        assert_eq!(
            reg.get("oo::class").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("oo::define").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("oo::objdefine").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("snit::method").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("snit::typemethod").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("uri::register").unwrap().body_kind,
            BodyKind::Structural
        );
    }

    /// SYNC2: iRules `when` event handler bodies are structural.
    #[test]
    fn body_kind_irules_when_structural() {
        use crate::body_kind::BodyKind;
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        assert_eq!(reg.get("when").unwrap().body_kind, BodyKind::Structural);
    }

    /// SYNC3: `body_arg_implicit_args` defaults to 0 and is set on
    /// `fileutil::updateInPlace` (which appends file contents to
    /// the body's first command at runtime).
    #[test]
    fn body_arg_implicit_args_defaults_zero_except_fileutil_updateinplace() {
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("set").unwrap().body_arg_implicit_args, 0);
        assert_eq!(reg.get("proc").unwrap().body_arg_implicit_args, 0);
        assert_eq!(
            reg.get("fileutil::updateInPlace")
                .unwrap()
                .body_arg_implicit_args,
            1,
        );
    }

    #[test]
    fn arg_indices_for_role_dict_with_multirole() {
        let reg = CommandRegistry::build_default();
        let reads = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarRead);
        let writes = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarWrite);
        assert!(reads.contains(&1), "VarRead reads={reads:?}");
        assert!(writes.contains(&1), "VarWrite writes={writes:?}");
    }

    #[test]
    fn arg_indices_for_role_dict_update_multirole() {
        let reg = CommandRegistry::build_default();
        let reads = reg.arg_indices_for_role(
            "dict",
            &["update", "$var", "k", "vname", "body"],
            ArgRole::VarRead,
        );
        let writes = reg.arg_indices_for_role(
            "dict",
            &["update", "$var", "k", "vname", "body"],
            ArgRole::VarWrite,
        );
        assert!(reads.contains(&1), "VarRead reads={reads:?}");
        assert!(writes.contains(&1), "VarWrite writes={writes:?}");
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

    #[test]
    fn resolve_call_unknown_command_returns_none() {
        let reg = CommandRegistry::build_default();
        assert!(reg
            .resolve_call("no_such_cmd", &[], DialectSet::empty())
            .is_none());
    }

    #[test]
    fn resolve_call_top_level_command() {
        let reg = CommandRegistry::build_default();
        let resolved = reg
            .resolve_call("set", &["x", "1"], DialectSet::empty())
            .unwrap();
        assert_eq!(resolved.spec.name, "set");
        assert!(resolved.sub.is_none());
    }

    #[test]
    fn resolve_call_subcommand() {
        let reg = CommandRegistry::build_default();
        let resolved = reg
            .resolve_call("dict", &["create", "k", "v"], DialectSet::TCL86)
            .unwrap();
        assert_eq!(resolved.spec.name, "dict");
        let sub = resolved.sub.expect("dict create resolves to a subcommand");
        assert_eq!(sub.name, "create");
    }

    #[test]
    fn resolve_call_dialect_filter_blocks_tcl84_dict() {
        let reg = CommandRegistry::build_default();
        // dict is tcl8.5+; resolving against tcl8.4 must fail.
        assert!(reg
            .resolve_call("dict", &["create"], DialectSet::TCL84)
            .is_none());
    }

    #[test]
    fn resolve_call_picks_arity_matched_command_form_for_incr() {
        let reg = CommandRegistry::build_default();
        // `incr counter` — arity 1 → matches the implicit form.
        let r1 = reg
            .resolve_call("incr", &["counter"], DialectSet::empty())
            .unwrap();
        let f1 = r1.form.expect("incr should match a CommandForm");
        assert_eq!(f1.name, "implicit");
        assert_eq!(f1.arity, crate::arity::Arity::exact(1));

        // `incr counter 5` — arity 2 → matches the explicit form.
        let r2 = reg
            .resolve_call("incr", &["counter", "5"], DialectSet::empty())
            .unwrap();
        let f2 = r2.form.expect("incr counter 5 should match a CommandForm");
        assert_eq!(f2.name, "explicit");
        assert_eq!(f2.arity, crate::arity::Arity::exact(2));
    }

    #[test]
    fn resolve_call_picks_arity_matched_command_form_for_lset() {
        let reg = CommandRegistry::build_default();
        // `lset lst value` — arity 2 → replace form.
        let replace = reg
            .resolve_call("lset", &["lst", "value"], DialectSet::TCL86)
            .unwrap();
        assert_eq!(replace.form.unwrap().name, "replace");

        // `lset lst 0 value` — arity 3 → single_index form.
        let single = reg
            .resolve_call("lset", &["lst", "0", "value"], DialectSet::TCL86)
            .unwrap();
        assert_eq!(single.form.unwrap().name, "single_index");

        // `lset lst 0 1 2 value` — arity 5 → flat_path form.
        let flat = reg
            .resolve_call("lset", &["lst", "0", "1", "2", "value"], DialectSet::TCL86)
            .unwrap();
        assert_eq!(flat.form.unwrap().name, "flat_path");
    }

    // -- ``resolve_option_terminator`` (W304 driver)
    //
    // Mirrors ``core/commands/registry/command_registry.py::resolve_option_terminator``.
    // Each W304 fixture in ``tests/test_checks.py::TestMissingOptionTerminator``
    // is rooted in one of these resolver outcomes; the resolver tests
    // here pin the per-command shape, the analyser tests pin the
    // tristate-severity / two-diagnostic / code-fix behaviour.

    #[test]
    fn resolve_option_terminator_returns_none_for_unknown_command() {
        let reg = CommandRegistry::build_default();
        assert!(reg
            .resolve_option_terminator("unknownthing", &[], DialectSet::empty())
            .is_none());
    }

    #[test]
    fn resolve_option_terminator_returns_none_for_command_without_terminator() {
        let reg = CommandRegistry::build_default();
        // ``set`` does not declare a ``--`` terminator option.
        assert!(reg
            .resolve_option_terminator("set", &["x", "1"], DialectSet::empty())
            .is_none());
    }

    #[test]
    fn resolve_option_terminator_form_level_for_regexp() {
        let reg = CommandRegistry::build_default();
        let profile = reg
            .resolve_option_terminator("regexp", &[], DialectSet::empty())
            .expect("regexp declares -- at the form level");
        assert_eq!(profile.scan_start, 0);
        assert!(profile.subcommand.is_none());
        assert!(profile.warn_without_terminator);
        // ``-start`` takes a value; ``-nocase`` does not.
        // ``-start`` takes a value; ``-nocase`` does not.  The
        // resolver returns the borrowed options slice; callers
        // filter via ``OptionSpec::takes_value``.
        assert!(profile
            .options
            .iter()
            .any(|o| o.name == "-start" && o.takes_value));
        assert!(profile
            .options
            .iter()
            .any(|o| o.name == "-nocase" && !o.takes_value));
    }

    #[test]
    fn resolve_option_terminator_subcommand_scoped_for_file_delete() {
        let reg = CommandRegistry::build_default();
        let profile = reg
            .resolve_option_terminator("file", &["delete", "$path"], DialectSet::empty())
            .expect("file delete declares -- at the subcommand level");
        assert_eq!(profile.scan_start, 1);
        assert_eq!(profile.subcommand, Some("delete"));
    }

    #[test]
    fn resolve_option_terminator_subcommand_without_terminator_returns_none() {
        let reg = CommandRegistry::build_default();
        // ``file mtime`` has no ``--`` terminator.
        let profile = reg.resolve_option_terminator("file", &["mtime", "$p"], DialectSet::empty());
        assert!(profile.is_none(), "got {profile:?}");
    }

    #[test]
    fn resolve_option_terminator_warn_flag_off_for_non_regexp() {
        let reg = CommandRegistry::build_default();
        // ``unset`` declares ``--`` but does not carry the
        // ``WARN_WITHOUT_TERMINATOR`` trait — only ``regexp`` does.
        let profile = reg
            .resolve_option_terminator("unset", &["$x"], DialectSet::empty())
            .expect("unset declares --");
        assert!(!profile.warn_without_terminator);
    }

    // -- ``is_canonical_list_command`` (W101 safe-idiom driver)

    #[test]
    fn is_canonical_list_command_includes_list_and_split_excludes_concat() {
        let reg = CommandRegistry::build_default();
        assert!(reg.is_canonical_list_command("list"));
        assert!(reg.is_canonical_list_command("linsert"));
        assert!(reg.is_canonical_list_command("split"));
        assert!(reg.is_canonical_list_command("lreverse"));
        // ``concat`` returns LIST but is the explicit non-canonical
        // exclusion (mirrors Python's ``_NON_CANONICAL_LIST_COMMANDS``).
        assert!(!reg.is_canonical_list_command("concat"));
        // Non-list commands (e.g. ``set``) are filtered out.
        assert!(!reg.is_canonical_list_command("set"));
        // Unknown commands return false.
        assert!(!reg.is_canonical_list_command("unknownthing"));
    }

    #[test]
    fn is_canonical_list_command_handles_compound_subcommand_keys() {
        let reg = CommandRegistry::build_default();
        // ``dict keys`` returns LIST.
        assert!(reg.is_canonical_list_command("dict keys"));
        // ``dict get`` returns String (or unspecified) — not canonical.
        assert!(!reg.is_canonical_list_command("dict froob"));
    }

    #[test]
    fn irules_sink_commands_carry_structural_options() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();

        let respond = reg.get("HTTP::respond").expect("HTTP::respond loaded");
        let opts: Vec<&str> = respond.options.iter().map(|o| o.name).collect();
        assert!(
            opts.contains(&"-version") && opts.contains(&"-status") && opts.contains(&"-noserver"),
            "HTTP::respond options {opts:?} should include -version / -status / -noserver",
        );
        let noserver = respond
            .options
            .iter()
            .find(|o| o.name == "-noserver")
            .unwrap();
        assert!(!noserver.takes_value);
        let version = respond
            .options
            .iter()
            .find(|o| o.name == "-version")
            .unwrap();
        assert!(version.takes_value);

        let header = reg.get("HTTP::header").expect("HTTP::header loaded");
        let header_opts: Vec<&str> = header.options.iter().map(|o| o.name).collect();
        assert!(
            header_opts.contains(&"-noupdate"),
            "HTTP::header options {header_opts:?} should include -noupdate",
        );
    }
}
