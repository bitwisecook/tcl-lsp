//! `pid` — return process ID(s).
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "pid ?fileId?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pid",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::BYTE_COMPILED | Traits::PURE,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::Int),
        hover: Some(HoverSnippet {
            summary: "Retrieve process identifiers",
            synopsis: &["pid ?fileId?"],
            snippet: "If the fileId argument is given then it should normally refer to a process pipeline created with the open command.",
            source: "Tcl man page pid.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
