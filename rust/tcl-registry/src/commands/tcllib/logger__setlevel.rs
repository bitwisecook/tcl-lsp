//! `logger::setlevel` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "logger::setlevel level",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::setlevel",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Set the logging level for all logger services.",
            synopsis: &["logger::setlevel level"],
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
