//! `yieldto` — yield to a command from a coroutine.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "yieldto command ?arg...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yieldto",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Create and produce values from coroutines",
            synopsis: &["yieldto command ?arg...?", "yieldto command ?arg ...?"],
            snippet: "The coroutine command creates a new coroutine context (with associated command) named name and executes that context by calling command, passing in the other remaining arguments without further interpretation.",
            source: "Tcl man page coroutine.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        traits: Traits::LANGUAGE_KEYWORD,
        ..CommandSpec::DEFAULT
    }
}
