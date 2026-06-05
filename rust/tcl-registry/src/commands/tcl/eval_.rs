//! `eval` — evaluate a Tcl script dynamically.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "eval arg ?arg ...?",
}];

/// Command spec for `eval`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "eval",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CREATES_BARRIER
            | Traits::EVALUATES_CODE
            | Traits::TAINT_SINK,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Body)],
        lowering_hook: Some(crate::hooks::LoweringHookId::Eval),
        return_type: Some(TclType::String),
hover: Some(HoverSnippet {
    summary: "Evaluate a Tcl script.",
    synopsis: &["eval arg ?arg ...?"],
    snippet: "Concatenates its arguments and executes the result as a Tcl script.\n\n**Security**: If any argument contains user-controlled data, this enables arbitrary code injection. Prefer `{*}$cmdList` (Tcl 8.5+) to expand pre-built command lists safely, or use direct invocation.",
    source: "Tcl man page eval.n",
    examples: "",
    return_value: "",
}),
        // GAP-D2: a `LIST_CANONICAL` value preserves element
        // boundaries and suppresses T100. Mirrors `tcl/eval.py`.
        taint_sink_safe_colour: Some(TaintColour::LIST_CANONICAL),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
