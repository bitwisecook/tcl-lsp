//! `PyO3` bindings for the command registry.
//!
//! Exposes registry queries to Python so the transition period can
//! use the Rust registry from Python consumer code.

use pyo3::prelude::*;

use tcl_registry::registry::CommandRegistry;
use tcl_registry::traits::Traits;
use tcl_registry::ArgRole;

/// Query the Rust registry for a command's traits.
///
/// Returns a dict of trait name → bool for the given command.
#[pyfunction]
#[pyo3(signature = (name, /))]
pub fn registry_command_traits(name: &str) -> Option<Vec<(&'static str, bool)>> {
    let reg = CommandRegistry::build_default();
    let spec = reg.get(name)?;
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
    let reg = CommandRegistry::build_default();
    let spec = reg.get(name)?;
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
    let reg = CommandRegistry::build_default();
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
    reg.arg_indices_for_role(name, &args_ref, role)
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
