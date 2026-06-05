//! `catch` — evaluate script and trap exceptional returns.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "catch script ?resultVarName? ?optionsVarName?",
}];

/// Command spec for `catch`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "catch",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD,
        arity: Arity::new(1, 3),
        arg_roles: &[
            (0, ArgRole::Body),
            (1, ArgRole::VarWrite),
            (2, ArgRole::VarWrite),
        ],
        lowering_hook: Some(crate::hooks::LoweringHookId::Catch),
        return_type: Some(TclType::Int),
        hover: Some(HoverSnippet::brief(
            "Evaluate script and trap exceptional returns.",
            &["catch script ?resultVarName? ?optionsVarName?"],
            "Tcl catch(1)",
        )),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
