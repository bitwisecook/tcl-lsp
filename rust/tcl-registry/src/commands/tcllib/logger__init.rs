//! `logger::init` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::LogIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "logger::init service",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::init",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
hover: Some(HoverSnippet {
    summary: "Initialise a logger for a service.",
    synopsis: &["logger::init service"],
    snippet: "Creates a new logger instance for the given service name. Returns a logger command for controlling log levels and output.",
    source: "tcllib logger package",
    examples: "set log [logger::init myapp]",
    return_value: "A logger command token.",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("logger"),
        required_package: Some("logger"),
        ..CommandSpec::DEFAULT
    }
}
