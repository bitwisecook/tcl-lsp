//! `tcl::OptProc` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptProc",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Define a proc with automatic option parsing.",
            &["tcl::OptProc name optlist body"],
            "F5",
        )),
        required_package: Some("opt"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
