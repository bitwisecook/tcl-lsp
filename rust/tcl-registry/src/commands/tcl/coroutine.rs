//! `coroutine` — create a coroutine.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "coroutine name command ?arg...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroutine",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name)],
        return_type: Some(TclType::String),
hover: Some(HoverSnippet {
    summary: "Create and produce values from coroutines",
    synopsis: &["coroutine name command ?arg...?", "coroutine name command ?arg ...?"],
    snippet: "The coroutine command creates a new coroutine context (with associated command) named name and executes that context by calling command, passing in the other remaining arguments without further interpretation.",
    source: "Tcl man page coroutine.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
