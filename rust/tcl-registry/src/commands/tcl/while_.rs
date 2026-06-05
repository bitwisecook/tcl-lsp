//! `while` — loop while a condition is true.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "while test body",
}];

/// Command spec for `while`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "while",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
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
hover: Some(HoverSnippet {
    summary: "Execute script repeatedly as long as a condition is met",
    synopsis: &["while test body"],
    snippet: "The while command evaluates test as an expression (in the same way that expr evaluates its argument).",
    source: "Tcl man page while.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
