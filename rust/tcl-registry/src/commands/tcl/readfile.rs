//! `readFile` — read the contents of a text or binary file (Tcl 9.0+, TIP 670).

use crate::prelude::*;

/// Command spec for `readFile`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "readFile",
        traits: Traits::TAINT_SOURCE,
        dialects: Some(DialectSet::TCL90),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Read the contents of a file and return them.",
            &["readFile filename ?text|binary?"],
            "Tcl man page library.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
