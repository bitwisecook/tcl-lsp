//! `msgcat::mc` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mc",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Translate a source string according to the current locale.",
            synopsis: &["msgcat::mc src-string ?arg arg ...?"],
            snippet: "Looks up *src-string* in the message catalogue for the calling namespace and current locale.  Any additional arguments are substituted into the translated string via ``format``.",
            source: "Tcl stdlib msgcat package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("msgcat"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
