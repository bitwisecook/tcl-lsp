//! `for` — C-style loop with init, test, and next scripts.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "for start test next body",
}];

/// Command spec for `for`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "for",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_BOOLEAN_COND
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY,
        arity: Arity::exact(4),
        arg_roles: &[
            (0, ArgRole::Body),
            (1, ArgRole::Expr),
            (2, ArgRole::Body),
            (3, ArgRole::Body),
        ],
        lowering_hook: Some(crate::hooks::LoweringHookId::For),
        return_type: Some(TclType::String),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet::brief(
            "C-style loop with init, test, and next scripts.",
            &["for start test next body"],
            "Tcl for(1)",
        )),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
