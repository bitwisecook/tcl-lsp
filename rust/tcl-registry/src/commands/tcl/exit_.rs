//! `exit` — end the application.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exit ?returnCode?",
}];

/// Command spec for `exit`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exit",
        traits: Traits::BYTE_COMPILED | Traits::TERMINATES_BLOCK,
        arity: Arity::new(0, 1),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "End the application",
            synopsis: &["exit ?returnCode?"],
            snippet:
                "Terminate the process, returning returnCode to the system as the exit status.",
            source: "Tcl man page exit.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
