//! `throw` — generate a machine-readable error.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "throw type message",
}];

/// Command spec for `throw`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "throw",
        traits: Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD | Traits::TERMINATES_BLOCK,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::exact(2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Generate a machine-readable error",
            synopsis: &["throw type message"],
            snippet: "This command causes the current evaluation to be unwound with an error.",
            source: "Tcl man page throw.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
