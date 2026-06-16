//! `safe::interpCreate` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpCreate",
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Create a safe child interpreter with restricted capabilities.",
            synopsis: &["safe::interpCreate ?child? ?options...?"],
            snippet: "Creates a safe interpreter.  Options include ``-accessPath``, ``-statics``, ``-noStatics``, ``-nested``, ``-noNested``, ``-deleteHook``.",
            source: "Tcl stdlib Safe Base",
            examples: "",
            return_value: "",
        }),
        required_package: Some("safe"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
