//! `platform::identify` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::identify",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
hover: Some(HoverSnippet {
            summary: "Return the platform identifier for the current machine.",
            synopsis: &["platform::identify"],
            snippet: "Returns a string like ``linux-x86_64`` or ``macosx-arm`` that specifically identifies the current platform, including CPU details and libc version where relevant.",
            source: "Tcl stdlib platform package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("platform"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
