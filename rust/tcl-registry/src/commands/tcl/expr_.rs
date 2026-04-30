//! `expr` — evaluate a mathematical expression.

use crate::prelude::*;

/// Command spec for `expr`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expr",
        traits: Traits::PURE_EVALUATION | Traits::NEEDS_START_CMD | Traits::TAINT_SINK,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Expr)],
        return_type: Some(TclType::Numeric),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Numeric),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet::brief(
            "Evaluate an expression.",
            &["expr arg ?arg ...?"],
            "Tcl expr(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
