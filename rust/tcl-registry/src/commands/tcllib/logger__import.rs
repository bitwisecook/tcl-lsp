//! `logger::import` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "logger::import ?-all? ?-force? ?-prefix prefix? ?-namespace namespace? service",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::import",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Import logger commands into the current namespace.",
            synopsis: &[
                "logger::import ?-all? ?-force? ?-prefix prefix? ?-namespace namespace? service",
            ],
            snippet: "",
            source: "tcllib logger package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("logger"),
        required_package: Some("logger"),
        ..CommandSpec::DEFAULT
    }
}
