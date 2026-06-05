//! `expr` — evaluate a mathematical expression.
//
// VERIFIED: Tcl 9.0.3 manpage expr(n) (man3/expr.n).

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "expr arg ?arg ...?",
}];

/// Command spec for `expr`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expr",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE_EVALUATION
            | Traits::NEEDS_START_CMD
            | Traits::TAINT_SINK,
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
        lowering_hook: Some(LoweringHookId::Expr),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
