//! `coroprobe` — probe a suspended coroutine.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "coroprobe coroName command ?arg ...?",
}];

// `coroprobe coroName command ?arg...?` evaluates an arbitrary command *now* in
// the paused coroutine's context and returns its result — so it runs unknown
// code with unknown reads/writes, matching `eval` / `uplevel`.
static SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroprobe",
        traits: Traits::EVALUATES_CODE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Evaluate a command in a suspended coroutine.",
            synopsis: &["coroprobe coroName command ?arg ...?"],
            snippet: "",
            source: "Tcl coroprobe(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
