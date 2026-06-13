//! `logger::walk` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "logger::walk service command",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::walk",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Walk the logger tree applying a command to each service.",
            synopsis: &["logger::walk service command"],
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
