//! `while` — loop while a condition is true.

use crate::prelude::*;

/// Command spec for `while`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "while",
        traits: Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_BOOLEAN_COND
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY,
        arity: Arity::exact(2),
        arg_roles: &[(0, ArgRole::Expr), (1, ArgRole::Body)],
        lowering_hook: Some(crate::hooks::LoweringHookId::While),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet::brief(
            "Execute body while test expression is true.",
            &["while test body"],
            "Tcl while(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
