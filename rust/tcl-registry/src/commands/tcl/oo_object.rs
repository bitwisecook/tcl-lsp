//! `TclOO` object.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::object method ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::object",
        traits: Traits::IS_OO_METACLASS,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "root class of the class hierarchy",
            synopsis: &["oo::object method ?arg ...?", "oo::object"],
            snippet: "The oo::object class is the root class of the object hierarchy; every object is an instance of this class.",
            source: "Tcl man page object.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
