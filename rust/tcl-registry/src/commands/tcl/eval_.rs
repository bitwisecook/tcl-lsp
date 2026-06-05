//! `eval` — evaluate a Tcl script dynamically.

use crate::prelude::*;

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
        hover: Some(HoverSnippet::brief(
            "Evaluate a Tcl script.",
            &["eval arg ?arg ...?"],
            "Tcl eval(1)",
        )),
        // GAP-D2: a `LIST_CANONICAL` value preserves element
        // boundaries and suppresses T100. Mirrors `tcl/eval.py`.
        taint_sink_safe_colour: Some(TaintColour::LIST_CANONICAL),
        ..CommandSpec::DEFAULT
    }
}
