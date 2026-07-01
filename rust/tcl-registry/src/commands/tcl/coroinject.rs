//! `coroinject` — inject a command into a coroutine.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "coroinject coroName command ?arg ...?",
}];

// `coroinject coroName command ?arg...?` schedules an arbitrary command to run
// the next time the coroutine resumes (its result becomes the resumption
// value).  Deferred arbitrary code with unknown reads/writes — treated like
// `eval` so the optimiser never eliminates or reorders it.
static SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroinject",
        traits: Traits::EVALUATES_CODE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Inject a command into a suspended coroutine.",
            synopsis: &["coroinject coroName command ?arg ...?"],
            snippet: "",
            source: "Tcl coroinject(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
