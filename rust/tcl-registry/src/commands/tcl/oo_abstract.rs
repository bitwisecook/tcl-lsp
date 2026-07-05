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
    synopsis: "oo::abstract method ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::abstract",
        traits: Traits::IS_OO_METACLASS | Traits::LANGUAGE_KEYWORD | Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(oo_class_arg_roles),
        return_type: Some(TclType::String),
        // Bodies of `oo::abstract create / new / createWithNamespace`
        // run in a TclOO definition context, exactly like `oo::class`.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "metaclass for abstract classes",
            synopsis: &[
                "oo::abstract method ?arg ...?",
                "oo::abstract create name ?definition?",
            ],
            snippet: "The oo::abstract command creates a class that cannot be directly instantiated. Only subclasses of an abstract class may be instantiated.",
            source: "Tcl man page abstract.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}
