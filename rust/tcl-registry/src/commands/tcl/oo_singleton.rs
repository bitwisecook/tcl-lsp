//! `TclOO` class.
use super::oo_class::oo_class_arg_roles;
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::singleton method ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::singleton",
        traits: Traits::IS_OO_METACLASS | Traits::LANGUAGE_KEYWORD | Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(oo_class_arg_roles),
        return_type: Some(TclType::String),
        // Bodies of `oo::singleton create / new / createWithNamespace`
        // run in a TclOO definition context, exactly like `oo::class`.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "metaclass for singleton classes",
            synopsis: &[
                "oo::singleton method ?arg ...?",
                "oo::singleton create name ?definition?",
            ],
            snippet: "The oo::singleton command creates a class that will only ever have one instance. Attempts to create more instances will return the existing instance.",
            source: "Tcl man page singleton.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
