//! `writeFile` — write contents to a text or binary file (Tcl 9.0+, TIP 670).

use crate::prelude::*;

/// Command spec for `writeFile`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "writeFile",
        dialects: Some(DialectSet::TCL90),
        arity: Arity::new(2, 3),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Write contents to a file.",
            &["writeFile filename ?text|binary? contents"],
            "Tcl man page library.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
