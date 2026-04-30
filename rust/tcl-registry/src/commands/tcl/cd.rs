//! `cd` — change working directory.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cd",
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Change working directory.",
            &["cd ?dirName?"],
            "Tcl cd(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
