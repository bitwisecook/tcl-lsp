//! `error` — generate an error.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "error message ?info? ?code?",
}];

/// Command spec for `error`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "error",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::TERMINATES_BLOCK
            | Traits::NEEDS_START_CMD,
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Generate an error",
            synopsis: &["error message ?info? ?code?"],
            snippet: "Returns a TCL_ERROR code, which causes command interpretation to be unwound.",
            source: "Tcl man page error.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
