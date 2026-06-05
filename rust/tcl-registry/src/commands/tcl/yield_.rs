//! `yield` — yield a value from a coroutine.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "yield ?value?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yield",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
hover: Some(HoverSnippet {
    summary: "Create and produce values from coroutines",
    synopsis: &["yield ?value?"],
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
