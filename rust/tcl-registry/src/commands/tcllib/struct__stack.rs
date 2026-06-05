//! `struct::stack` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "struct::stack ?stackName?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "struct::stack",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate LIFO stack objects.",
            synopsis: &["struct::stack ?stackName?"],
            snippet: "Creates a new stack object. Elements are pushed and popped from the top.",
            source: "tcllib struct::stack package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("struct::stack"),
        required_package: Some("struct::stack"),
        ..CommandSpec::DEFAULT
    }
}
