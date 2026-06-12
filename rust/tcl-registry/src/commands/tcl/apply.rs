//! `apply` — apply an anonymous procedure.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::ProcDefinition,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "apply func ?arg1 arg2 ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "apply",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Body)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Apply an anonymous function",
            synopsis: &["apply func ?arg1 arg2 ...?", "apply func ?arg ...?"],
            snippet: "The command apply applies the function func to the arguments arg1 arg2 ...",
            source: "Tcl man page apply.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
