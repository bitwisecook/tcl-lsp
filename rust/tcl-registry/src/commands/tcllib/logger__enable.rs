//! `logger::enable` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "logger::enable level",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::enable",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Enable logging at the specified level.",
            synopsis: &["logger::enable level"],
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
