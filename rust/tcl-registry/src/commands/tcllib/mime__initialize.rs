//! `mime::initialize` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::initialize ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::initialize",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
    summary: "Create a MIME part from data, a file, or a channel.",
    synopsis: &["mime::initialize ?-canonical type? ?-params dict? ?-encoding enc? ?-header dict? ?-string text | -file path | -chan channel?"],
    snippet: "",
    source: "tcllib mime package",
    examples: "",
    return_value: "A MIME token.",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
