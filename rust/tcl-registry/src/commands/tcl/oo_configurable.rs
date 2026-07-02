//! `TclOO` class.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::configurable method ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::configurable",
        traits: Traits::IS_OO_METACLASS | Traits::LANGUAGE_KEYWORD | Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "class that supports configurable properties",
            synopsis: &[
                "oo::configurable method ?arg ...?",
                "oo::configurable create name ?definition?",
            ],
            snippet: "The oo::configurable command creates a class that automatically supports the property definition command and a configure method for getting and setting property values on instances.",
            source: "Tcl man page configurable.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
