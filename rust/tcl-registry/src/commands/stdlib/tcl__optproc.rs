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
hover: Some(HoverSnippet {
            summary: "Define a proc with automatic option parsing.",
            synopsis: &["tcl::OptProc name optlist body"],
            snippet: "Defines a procedure *name* whose arguments are parsed according to *optlist*, a list of option descriptions.  Inside *body*, option values are available as local variables.",
            source: "Tcl stdlib opt package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("opt"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
