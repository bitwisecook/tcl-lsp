//! `TclOO` object.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::copy sourceObject ?targetObject? ?targetNamespace?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::copy",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "create copies of objects and classes",
            synopsis: &[
                "oo::copy sourceObject ?targetObject? ?targetNamespace?",
                "oo::copy sourceObject ?targetObject?",
            ],
            snippet: "The oo::copy command creates a copy of an object or class.",
            source: "Tcl man page copy.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
