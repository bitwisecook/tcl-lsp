//! `PyO3` bindings for the command registry.
//!
//! Exposes registry queries to Python so the transition period can
//! use the Rust registry from Python consumer code.

use std::collections::HashMap;
use std::sync::OnceLock;

use pyo3::prelude::*;

use tcl_registry::dialects::DialectSet;
use tcl_registry::registry::CommandRegistry;
use tcl_registry::traits::Traits;
use tcl_registry::ArgRole;

/// Cached default registry — built once, reused across all `PyO3` calls.
///
/// The full registry has ~118 Tcl + stdlib + tcllib specs; building
/// it eagerly (~100k `HashMap` inserts in aggregate when called per
/// document on a large workspace) was a hot-path waste in every
/// other binding module that consumed `CommandRegistry::build_default()`
/// per call. This `OnceLock` is the shared cache; new bindings should
/// call this rather than `build_default()` directly.
pub(crate) fn default_registry() -> &'static CommandRegistry {
    static REGISTRY: OnceLock<CommandRegistry> = OnceLock::new();
    REGISTRY.get_or_init(CommandRegistry::build_default)
}

/// Per-dialect cached registry.
///
/// Loads exactly the dialect parsed from `dialect` (e.g.
/// `"f5-irules"` → `DialectSet::IRULES`) on top of the default
/// Tcl + stdlib + tcllib specs. Each dialect gets its own cached
/// instance so dialect-specific overrides (e.g. iRules's
/// command-shape constraints) don't bleed across requests for
/// unrelated dialects. Mirrors the original per-call behaviour
/// of `CommandRegistry::build_default(); load_dialect(d)` without
/// the per-call build cost.
///
/// Unparseable dialect strings collapse to the empty-string key
/// so a stream of typos can't leak one registry per typo — they
/// all share a single cached "plain Tcl" registry.
///
/// Python callers don't pass a registry through; this is the
/// shared instance every binding consults when it needs
/// dialect-aware behaviour.
pub(crate) fn default_registry_for_dialect(dialect: &str) -> &'static CommandRegistry {
    static REGISTRIES: OnceLock<std::sync::Mutex<HashMap<String, &'static CommandRegistry>>> =
        OnceLock::new();
    let parsed = DialectSet::parse(dialect);
    // Canonicalise the cache key: parseable dialects keep their
    // string; unparseable ones share the plain-Tcl entry.
    let key = if parsed.is_some() { dialect } else { "" };
    let map = REGISTRIES.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let mut guard = map.lock().expect("registry cache mutex");
    if let Some(r) = guard.get(key) {
        return r;
    }
    // First request for this dialect: build the per-dialect
    // registry, leak it, and cache the resulting `&'static`
    // reference. The leak is bounded by the small set of
    // canonical dialects any Python process will see.
    let mut r = CommandRegistry::build_default();
    if let Some(d) = parsed {
        r.load_dialect(d);
    }
    let leaked: &'static CommandRegistry = Box::leak(Box::new(r));
    guard.insert(key.to_owned(), leaked);
    leaked
}

/// Query the Rust registry for a command's traits.
///
/// Returns a list of `(trait_name, bool)` pairs for the given command.
#[pyfunction]
#[pyo3(signature = (name, /))]
pub fn registry_command_traits(name: &str) -> Option<Vec<(&'static str, bool)>> {
    let spec = default_registry().get(name)?;
    let t = spec.traits;
    Some(vec![
        ("control_flow", t.contains(Traits::CONTROL_FLOW)),
        ("language_keyword", t.contains(Traits::LANGUAGE_KEYWORD)),
        ("has_loop_body", t.contains(Traits::HAS_LOOP_BODY)),
        ("has_boolean_cond", t.contains(Traits::HAS_BOOLEAN_COND)),
        ("never_inline_body", t.contains(Traits::NEVER_INLINE_BODY)),
        ("creates_barrier", t.contains(Traits::CREATES_BARRIER)),
        (
            "creates_scope_alias",
            t.contains(Traits::CREATES_SCOPE_ALIAS),
        ),
        ("evaluates_code", t.contains(Traits::EVALUATES_CODE)),
        ("pure", t.contains(Traits::PURE)),
        ("terminates_block", t.contains(Traits::TERMINATES_BLOCK)),
        ("defines_procedure", t.contains(Traits::DEFINES_PROCEDURE)),
        ("destroys_variable", t.contains(Traits::DESTROYS_VARIABLE)),
        ("reads_before_write", t.contains(Traits::READS_BEFORE_WRITE)),
        ("taint_sink", t.contains(Traits::TAINT_SINK)),
    ])
}

/// Query the Rust registry for a command's arity.
///
/// Returns (min, max) where max=65535 means unlimited.
#[pyfunction]
#[pyo3(signature = (name, /))]
pub fn registry_command_arity(name: &str) -> Option<(u16, u16)> {
    let spec = default_registry().get(name)?;
    Some((spec.arity.min, spec.arity.max))
}

/// Query the Rust registry for argument indices with a given role.
///
/// `role` is one of: `body`, `expr`, `var_write`, `var_read`, `name`,
/// `param_list`, `pattern`, `channel`, `index`.
#[pyfunction]
#[pyo3(signature = (name, args, role, /))]
#[allow(clippy::needless_pass_by_value)] // PyO3 requires owned Vec
pub fn registry_arg_indices_for_role(name: &str, args: Vec<String>, role: &str) -> Vec<usize> {
    let role = match role {
        "body" => ArgRole::Body,
        "expr" => ArgRole::Expr,
        "var_write" => ArgRole::VarWrite,
        "var_read" => ArgRole::VarRead,
        "name" => ArgRole::Name,
        "param_list" => ArgRole::ParamList,
        "pattern" => ArgRole::Pattern,
        "channel" => ArgRole::Channel,
        "index" => ArgRole::Index,
        _ => return Vec::new(),
    };
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    default_registry().arg_indices_for_role(name, &args_ref, role)
}

/// Return the version of the registry crate.
#[pyfunction]
pub fn registry_version() -> &'static str {
    tcl_registry::VERSION
}

pub(crate) fn register_with(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(registry_command_traits, m)?)?;
    m.add_function(wrap_pyfunction!(registry_command_arity, m)?)?;
    m.add_function(wrap_pyfunction!(registry_arg_indices_for_role, m)?)?;
    m.add_function(wrap_pyfunction!(registry_version, m)?)?;
    Ok(())
}
