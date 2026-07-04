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
//! * Recursing into the body of an *outer* OO definition command
//!   ([`is_outer_oo_definition_command`] — the metaclasses' `create` /
//!   `new` forms plus `oo::define` / `oo::objdefine`) enters OO-body
//!   context (`inside_oo_body = true`).
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

use tcl_registry::CommandRegistry;
use tcl_registry::prelude::Traits;

/// Outer (context-establishing) OO definition commands: any registry
/// command carrying the `IS_OO_METACLASS` trait (`oo::class`,
/// `oo::configurable`, `oo::abstract`, `oo::singleton`, plus any
/// dialect-registered metaclass) and the two definition commands
/// `oo::define` / `oo::objdefine`.
///
/// The body of one of these runs as a `TclOO` definition script; recursing
/// into it switches the walker into "inside OO body" mode where the
/// inner-OO commands (`method`, `constructor`, …) become body-bearing.
///
/// Driven by the registry trait rather than a hardcoded name list so a
/// newly registered metaclass is covered automatically.
#[must_use]
pub fn is_outer_oo_definition_command(name: &str, registry: &CommandRegistry) -> bool {
    if matches!(name, "oo::define" | "oo::objdefine") {
        return true;
    }
    registry
        .get(name)
        .is_some_and(|spec| spec.traits.contains(Traits::IS_OO_METACLASS))
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
/// should carry, given the current flag `cur`:
///
/// * An outer OO definition command's body *is* the OO definition script →
///   `true`.
/// * An inner OO definition command's body (while already inside an OO
///   body) is plain Tcl code → `false`.
/// * Everything else inherits `cur` — control-flow nesting inside a class
///   body stays in OO context.
#[must_use]
pub fn next_inside_oo_body(command: &str, cur: bool, registry: &CommandRegistry) -> bool {
    if is_outer_oo_definition_command(command, registry) {
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
    fn metaclasses_are_outer_commands() {
        let reg = registry();
        for name in [
            "oo::class",
            "oo::configurable",
            "oo::abstract",
            "oo::singleton",
            "oo::define",
            "oo::objdefine",
        ] {
            assert!(
                is_outer_oo_definition_command(name, &reg),
                "{name} must be an outer OO definition command"
            );
        }
    }

    #[test]
    fn ordinary_commands_are_not_outer() {
        let reg = registry();
        for name in ["proc", "set", "if", "namespace", "method"] {
            assert!(
                !is_outer_oo_definition_command(name, &reg),
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
        // Entering an outer body switches on.
        assert!(next_inside_oo_body("oo::class", false, &reg));
        assert!(next_inside_oo_body("oo::configurable", false, &reg));
        // A method body inside a class body switches off.
        assert!(!next_inside_oo_body("method", true, &reg));
        // Control flow inherits.
        assert!(next_inside_oo_body("if", true, &reg));
        assert!(!next_inside_oo_body("if", false, &reg));
        // Inner commands at top level (not inside an OO body) don't fire.
        assert!(!next_inside_oo_body("method", false, &reg));
    }
}
