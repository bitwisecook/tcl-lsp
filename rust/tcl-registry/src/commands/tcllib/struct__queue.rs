//! `struct::queue` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "struct::queue ?queueName? ?=|:=|as|deserialize source?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "struct::queue",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
    summary: "Create and manipulate FIFO queue objects.",
    synopsis: &["struct::queue ?queueName? ?=|:=|as|deserialize source?"],
    snippet: "Creates a new queue object. Elements are added at the back and removed from the front.",
    source: "tcllib struct::queue package",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
