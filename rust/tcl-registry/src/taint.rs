//! Registry-side taint metadata queries.
//!
//! Owns the small subcommand-shaped facts (`chan gets`, `chan read`,
//! `encoding convertfrom`) and the iRules namespace-prefix table that
//! used to live as hardcoded lists inside the compiler's taint
//! analyser. The compiler's [`tcl_compiler::taint`] module now asks
//! the registry "is this call a source / sink / sanitiser?" rather
//! than maintaining its own command-name set.

use crate::dialects::DialectSet;
use crate::registry::CommandRegistry;
use crate::traits::Traits;
use crate::types::TclType;

/// Namespaces whose commands return attacker-controlled data when
/// invoked under the iRules dialect.
///
/// Keeping the prefix table here means any consumer (LSP feature
/// providers, future native server, alternate diagnostics) sees the
/// same iRules source classification without duplicating the table.
pub const IRULES_TAINT_SOURCE_PREFIXES: &[&str] = &[
    "HTTP::", "URI::", "IP::", "TCP::", "UDP::", "SSL::", "STREAM::",
];

/// Return `true` when invoking `command` with `args` under `dialect`
/// produces attacker-controlled data.
///
/// Sources are identified by:
///
/// * the [`Traits::TAINT_SOURCE`] flag (pure trait dispatch — `gets`,
///   `read`, `exec`, `socket`);
/// * the [`Traits::UNNORMALISED_HTTP_GETTER`] flag (registry-driven
///   HTTP getter);
/// * subcommand-shaped sources such as `chan gets` / `chan read` /
///   `encoding convertfrom`, identified by inspecting the matched
///   subcommand spec rather than a parallel command-name table; and
/// * iRules namespace-prefixed getters (`HTTP::*`, `URI::*`, …)
///   when `dialect` is iRules.
#[must_use]
pub fn is_taint_source(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    dialect: DialectSet,
) -> bool {
    let Some(spec) = registry.get(command) else {
        return iRules_dialect_only_source(command, dialect);
    };

    if spec
        .traits
        .intersects(Traits::TAINT_SOURCE | Traits::UNNORMALISED_HTTP_GETTER)
    {
        return true;
    }

    // Subcommand-shaped sources: chan gets / chan read / encoding
    // convertfrom. Match by command + first-arg subcommand so the
    // analyser does not need to know the table.
    if let Some(sub_name) = args.first().copied() {
        if matches!(command, "chan") && matches!(sub_name, "gets" | "read") {
            return true;
        }
        if matches!(command, "encoding") && sub_name == "convertfrom" {
            return true;
        }
    }

    iRules_dialect_only_source(command, dialect)
}

/// Return `true` when `command` carries the iRules data-getter trait
/// or starts with one of the [`IRULES_TAINT_SOURCE_PREFIXES`]
/// namespaces. The prefix fallback covers iRules commands that are
/// registered without the explicit trait.
#[must_use]
pub fn is_irules_data_getter(registry: &CommandRegistry, command: &str) -> bool {
    if let Some(spec) = registry.get(command) {
        if spec.traits.contains(Traits::IRULES_DATA_GETTER) {
            return true;
        }
    }
    IRULES_TAINT_SOURCE_PREFIXES
        .iter()
        .any(|p| command.starts_with(p))
}

#[allow(non_snake_case)]
fn iRules_dialect_only_source(command: &str, dialect: DialectSet) -> bool {
    if dialect.contains(DialectSet::IRULES) {
        IRULES_TAINT_SOURCE_PREFIXES
            .iter()
            .any(|p| command.starts_with(p))
    } else {
        false
    }
}

/// Return `true` when `command` (with optional subcommand in `args`)
/// is a sanitiser — its return value is a fixed numeric type that
/// cannot carry taint through it.
///
/// Subcommand specs are checked first so `string length` and
/// `string is integer` register as sanitisers even though the
/// top-level `string` command has no return type.
#[must_use]
pub fn is_sanitiser(registry: &CommandRegistry, command: &str, args: &[&str]) -> bool {
    fn is_fixed_numeric(t: Option<TclType>) -> bool {
        matches!(t, Some(TclType::Int | TclType::Boolean))
    }
    let Some(spec) = registry.get(command) else {
        return false;
    };
    if let Some(sub_name) = args.first().copied() {
        if let Some(sub) = spec.subcommand(sub_name) {
            if is_fixed_numeric(sub.return_type) {
                return true;
            }
        }
    }
    is_fixed_numeric(spec.return_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_is_a_taint_source() {
        let registry = CommandRegistry::build_default();
        assert!(is_taint_source(
            &registry,
            "gets",
            &["stdin"],
            DialectSet::empty()
        ));
    }

    #[test]
    fn chan_gets_is_a_taint_source() {
        let registry = CommandRegistry::build_default();
        assert!(is_taint_source(
            &registry,
            "chan",
            &["gets", "stdin"],
            DialectSet::empty()
        ));
    }

    #[test]
    fn chan_configure_is_not_a_taint_source() {
        let registry = CommandRegistry::build_default();
        assert!(!is_taint_source(
            &registry,
            "chan",
            &["configure", "$ch"],
            DialectSet::empty()
        ));
    }

    #[test]
    fn http_uri_is_an_irules_source() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        assert!(is_taint_source(
            &registry,
            "HTTP::uri",
            &[],
            DialectSet::IRULES
        ));
    }

    #[test]
    fn http_uri_is_not_a_source_outside_irules() {
        // HTTP::* outside the iRules dialect must not be treated as
        // a taint source — regular Tcl scripts can define their own
        // HTTP::uri proc.
        let registry = CommandRegistry::build_default();
        assert!(!is_taint_source(
            &registry,
            "HTTP::uri",
            &[],
            DialectSet::empty()
        ));
    }

    #[test]
    fn string_length_is_a_sanitiser() {
        let registry = CommandRegistry::build_default();
        assert!(is_sanitiser(&registry, "string", &["length", "$x"]));
    }
}
