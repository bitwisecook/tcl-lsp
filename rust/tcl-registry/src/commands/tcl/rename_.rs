//! `rename` — rename or delete a command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::ProcDefinition,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "rename oldName newName",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "rename",
        traits: Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD,
        arity: Arity::exact(2),
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Name)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Rename or delete a command.",
            &["rename oldName newName"],
            "Tcl rename(1)",
        )),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
