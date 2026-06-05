//! `break` — abort looping command.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "break",
}];

/// Command spec for `break`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "break",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::NEEDS_START_CMD,
        arity: Arity::exact(0),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
hover: Some(HoverSnippet {
    summary: "Abort looping command",
    synopsis: &["break"],
    snippet: "This command is typically invoked inside the body of a looping command such as for or foreach or while.",
    source: "Tcl man page break.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
