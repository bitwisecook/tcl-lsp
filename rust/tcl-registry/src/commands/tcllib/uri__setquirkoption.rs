//! `uri::setQuirkOption` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uri::setQuirkOption option ?value...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::setQuirkOption",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Set a quirk option for URI processing.",
            synopsis: &["uri::setQuirkOption option ?value...?"],
            snippet: "",
            source: "tcllib uri package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
