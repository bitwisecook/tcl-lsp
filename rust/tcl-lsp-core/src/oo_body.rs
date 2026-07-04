//! Context-sensitive `TclOO` definition-body helpers, shared by the
//! recursive script walkers ([`crate::folding`] and
//! [`crate::semantic_tokens`]).
//!
//! ## The problem
//!
//! A `TclOO` class body —
//!
//! ```tcl
//! oo::class create Point {
//!     superclass Shape
//!     variable x y
//!     constructor {ax ay} { set x $ax; set y $ay }
//!     method move {dx dy} { incr x $dx; incr y $dy }
//! }
//! ```
//!
//! — is a *definition script*: its top-level words (`superclass`,
//! `constructor`, `method`, `property`, `self`, …) are **not** ordinary
//! commands.  They have no [`tcl_registry::CommandSpec`], so a registry
//! lookup can't tell a walker that `method move {dx dy} { … }`'s final
//! word is a script body to recurse into.  Without that, folding stops at
//! the method keyword and semantic highlighting renders the whole method
//! body as one opaque string.
//!
//! Worse, the sub-keywords are context-sensitive: a top-level user proc
//! named `method` outside any class body must **not** be treated as an OO
//! method definition.
//!
//! ## The model
//!
//! A recursive walker threads an `inside_oo_body` flag:
//!
//! * Recursing into the body of an *outer* OO definition body
//!   ([`is_outer_oo_definition_command`] — a metaclass `create` / `new`
//!   body, or the bare `oo::define` / `oo::objdefine` *script* form) enters
//!   OO-body context (`inside_oo_body = true`).  The *member* forms of
//!   `oo::define` / `oo::objdefine` (`… method m {} { body }`, …) do not:
//!   their body is ordinary method code, not a definition script.
//! * While in that context, an *inner* OO definition command
//!   ([`is_inner_oo_definition_command`]) contributes body arguments via
//!   [`inner_oo_body_indices`]; recursing into one of those bodies leaves
//!   OO-body context (a method body holds ordinary Tcl code).
//! * Every other command inherits the current flag, so control-flow
//!   nesting (`if` / `foreach` / …) around a `method` keeps the class
//!   body's context.
//!
//! Outside `inside_oo_body`, none of the inner helpers fire, so a
//! same-named user proc is never misclassified.

use tcl_registry::prelude::Traits;
use tcl_registry::{ArgRole, CommandRegistry};

/// Whether `command` carries the `IS_OO_METACLASS` trait (`oo::class`,
/// `oo::configurable`, `oo::abstract`, `oo::singleton`, plus any
/// dialect-registered metaclass).  Every body a metaclass takes is the
/// definition script of a `create` / `new` / `createWithNamespace` form, so
/// recursing into it always enters OO-body context.
///
/// Driven by the registry trait rather than a hardcoded name list so a
/// newly registered metaclass is covered automatically.
#[must_use]
pub fn is_oo_metaclass(command: &str, registry: &CommandRegistry) -> bool {
    registry
        .get(command)
        .is_some_and(|spec| spec.traits.contains(Traits::IS_OO_METACLASS))
}

/// Whether an `oo::define` / `oo::objdefine` call is the *bare script form*
/// (`oo::define Target { script }`) rather than a member/subcommand form
/// (`oo::define Target method m {} { body }`, `constructor …`, `self …`,
/// `property …`, …).
///
/// Only the script form's body is an OO definition script; the member forms
/// carry ordinary method bodies that must **not** switch the walker into OO
/// context (issue #747 review — a nested `method a b { … }` inside a real
/// method body would otherwise be mis-highlighted / mis-folded as an OO
/// member definition).
///
/// Detected via the registry's authoritative body-role resolver: the script
/// form resolves its definition body at argument index 1, while every member
/// form resolves a (method) body at index ≥ 2.  `args` excludes the command
/// head (`["Target", …]`), matching the resolver's calling convention.
#[must_use]
fn is_oo_define_script_form(command: &str, args: &[&str], registry: &CommandRegistry) -> bool {
    registry
        .arg_indices_for_role(command, args, ArgRole::Body)
        .contains(&1)
}

/// Whether recursing into `command`'s body/bodies enters OO-body context —
/// i.e. the body is a `TclOO` definition script whose top-level words are
/// the definition sub-keywords (`method`, `superclass`, `self …`, …).
///
/// True for metaclass `create` / `new` bodies and for the bare
/// `oo::define` / `oo::objdefine` script form; false for the member forms
/// of `oo::define` / `oo::objdefine` (their bodies are ordinary method
/// code).  `args` excludes the command head.
#[must_use]
pub fn is_outer_oo_definition_command(
    command: &str,
    args: &[&str],
    registry: &CommandRegistry,
) -> bool {
    if is_oo_metaclass(command, registry) {
        return true;
    }
    if matches!(command, "oo::define" | "oo::objdefine") {
        return is_oo_define_script_form(command, args, registry);
    }
    false
}

/// Inner OO definition-script commands.  These are context-sensitive: a
/// top-level user proc named `method` outside an OO block must not be
/// treated as an OO `method` definition.  A walker only consults
/// [`inner_oo_body_indices`] when it is inside an outer OO body.
///
/// Recursing into one of these inner bodies leaves OO definition context —
/// methods / constructors / destructors hold regular Tcl code.
#[must_use]
pub fn is_inner_oo_definition_command(name: &str) -> bool {
    matches!(
        name,
        "method"
            | "classmethod"
            | "constructor"
            | "destructor"
            | "initialise"
            | "initialize"
            | "private"
            | "self"
            | "property"
    )
}

/// Collect the `-set BODY` / `-get BODY` flag-value indices of an inner
/// `property NAME ?-set BODY? ?-get BODY?` invocation.  Always called with
/// `args` from the inner (unprefixed) form, so option scanning starts at
/// index 0.
fn collect_property_body_indices(args: &[&str]) -> Vec<usize> {
    let n = args.len();
    if n == 0 {
        return Vec::new();
    }
    args.iter()
        .enumerate()
        .take(n.saturating_sub(1))
        .filter_map(|(i, &a)| ((a == "-set" || a == "-get") && i + 1 < n).then_some(i + 1))
        .collect()
}

/// Return BODY argument indices (into `args`, i.e. excluding the
/// command-head word) for an inner OO definition-script command.  Only
/// meaningful when the caller is inside an outer OO body — outside that
/// context these words are ordinary calls and this must not be consulted.
///
/// Shapes (mirroring [`tcl_registry`]'s `oo_define_arg_roles`, minus the
/// leading target word that the `oo::define Target …` form carries):
///
/// * `constructor ARGS BODY` → body at index 1.
/// * `destructor BODY` / `initialise BODY` / `initialize BODY` /
///   `private BODY` → body at index 0.
/// * `method NAME ARGS BODY` / `classmethod NAME ARGS BODY` → body at the
///   last index.
/// * `self constructor ARGS BODY` → index 2; `self destructor BODY` →
///   index 1; `self method NAME ARGS BODY` → last index.
/// * `property NAME ?-set BODY? ?-get BODY?` → each flag value.
#[must_use]
pub fn inner_oo_body_indices(command: &str, args: &[&str]) -> Vec<usize> {
    let n = args.len();
    match command {
        "constructor" if n >= 2 => vec![1],
        "destructor" | "initialise" | "initialize" | "private" if n >= 1 => vec![0],
        "method" | "classmethod" if n >= 3 => vec![n - 1],
        "self" if n >= 1 => match args[0] {
            "constructor" if n >= 3 => vec![2],
            "destructor" if n >= 2 => vec![1],
            "method" | "classmethod" if n >= 4 => vec![n - 1],
            _ => Vec::new(),
        },
        "property" => collect_property_body_indices(args),
        _ => Vec::new(),
    }
}

/// The `inside_oo_body` flag the recursion into `command`'s body arguments
/// should carry, given the current flag `cur`.  `args` excludes the command
/// head.
///
/// * An outer OO definition body (metaclass `create`/`new`, or the bare
///   `oo::define`/`oo::objdefine` script form) *is* the OO definition
///   script → `true`.
/// * An inner OO definition command's body (while already inside an OO
///   body) is plain Tcl code → `false`.
/// * Everything else — including the *member* forms of `oo::define` /
///   `oo::objdefine`, whose bodies are ordinary method code — inherits
///   `cur`, so control-flow nesting inside a class body stays in OO
///   context while a method body drops out of it.
#[must_use]
pub fn next_inside_oo_body(
    command: &str,
    args: &[&str],
    cur: bool,
    registry: &CommandRegistry,
) -> bool {
    if is_outer_oo_definition_command(command, args, registry) {
        true
    } else if cur && is_inner_oo_definition_command(command) {
        false
    } else {
        cur
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn metaclass_create_bodies_are_outer() {
        let reg = registry();
        // Every metaclass `create`/`new` body is a definition script.
        for name in [
            "oo::class",
            "oo::configurable",
            "oo::abstract",
            "oo::singleton",
        ] {
            assert!(
                is_outer_oo_definition_command(name, &["create", "C", "{body}"], &reg),
                "{name} create body must be an outer OO definition body"
            );
        }
    }

    #[test]
    fn oo_define_script_form_is_outer_but_member_form_is_not() {
        let reg = registry();
        // Bare script form: `oo::define Target { script }` → outer body.
        for cmd in ["oo::define", "oo::objdefine"] {
            assert!(
                is_outer_oo_definition_command(cmd, &["Target", "{script}"], &reg),
                "{cmd} script form must be an outer OO definition body"
            );
            // Member forms carry ordinary method bodies → NOT outer (issue
            // #747 review): a nested `method a b { … }` inside them must not
            // be treated as an OO member definition.
            assert!(
                !is_outer_oo_definition_command(
                    cmd,
                    &["Target", "method", "m", "{}", "{body}"],
                    &reg
                ),
                "{cmd} member (method) form must not be an outer OO body"
            );
            assert!(
                !is_outer_oo_definition_command(
                    cmd,
                    &["Target", "constructor", "{}", "{body}"],
                    &reg
                ),
                "{cmd} constructor form must not be an outer OO body"
            );
            assert!(
                !is_outer_oo_definition_command(
                    cmd,
                    &["Target", "self", "method", "m", "{}", "{body}"],
                    &reg
                ),
                "{cmd} self-method form must not be an outer OO body"
            );
        }
    }

    #[test]
    fn ordinary_commands_are_not_outer() {
        let reg = registry();
        for name in ["proc", "set", "if", "namespace", "method"] {
            assert!(
                !is_outer_oo_definition_command(name, &["a", "b"], &reg),
                "{name} must not be an outer OO definition command"
            );
        }
    }

    #[test]
    fn inner_body_indices_cover_every_shape() {
        assert_eq!(
            inner_oo_body_indices("constructor", &["{}", "body"]),
            vec![1]
        );
        assert_eq!(inner_oo_body_indices("destructor", &["body"]), vec![0]);
        assert_eq!(inner_oo_body_indices("initialise", &["body"]), vec![0]);
        assert_eq!(inner_oo_body_indices("initialize", &["body"]), vec![0]);
        assert_eq!(inner_oo_body_indices("private", &["body"]), vec![0]);
        assert_eq!(
            inner_oo_body_indices("method", &["name", "{}", "body"]),
            vec![2]
        );
        assert_eq!(
            inner_oo_body_indices("classmethod", &["name", "{a}", "body"]),
            vec![2]
        );
        assert_eq!(
            inner_oo_body_indices("self", &["constructor", "{}", "body"]),
            vec![2]
        );
        assert_eq!(
            inner_oo_body_indices("self", &["destructor", "body"]),
            vec![1]
        );
        assert_eq!(
            inner_oo_body_indices("self", &["method", "name", "{}", "body"]),
            vec![3]
        );
        assert_eq!(
            inner_oo_body_indices("property", &["name", "-set", "s", "-get", "g"]),
            vec![2, 4]
        );
    }

    #[test]
    fn inner_body_indices_reject_short_forms() {
        assert!(inner_oo_body_indices("method", &["name"]).is_empty());
        assert!(inner_oo_body_indices("destructor", &[]).is_empty());
        assert!(inner_oo_body_indices("set", &["x", "1"]).is_empty());
    }

    #[test]
    fn context_transitions() {
        let reg = registry();
        // Entering an outer (metaclass create) body switches on.
        assert!(next_inside_oo_body(
            "oo::class",
            &["create", "C", "{b}"],
            false,
            &reg
        ));
        assert!(next_inside_oo_body(
            "oo::configurable",
            &["create", "C", "{b}"],
            false,
            &reg
        ));
        // `oo::define` script form switches on; member form does not.
        assert!(next_inside_oo_body(
            "oo::define",
            &["C", "{script}"],
            false,
            &reg
        ));
        assert!(!next_inside_oo_body(
            "oo::define",
            &["C", "method", "m", "{}", "{body}"],
            false,
            &reg
        ));
        // A method body inside a class body switches off.
        assert!(!next_inside_oo_body(
            "method",
            &["m", "{}", "{b}"],
            true,
            &reg
        ));
        // Control flow inherits.
        assert!(next_inside_oo_body("if", &["{c}", "{b}"], true, &reg));
        assert!(!next_inside_oo_body("if", &["{c}", "{b}"], false, &reg));
        // Inner commands at top level (not inside an OO body) don't fire.
        assert!(!next_inside_oo_body(
            "method",
            &["m", "{}", "{b}"],
            false,
            &reg
        ));
    }
}
