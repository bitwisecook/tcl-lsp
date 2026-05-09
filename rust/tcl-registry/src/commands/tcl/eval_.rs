//! `eval` — evaluate a Tcl script dynamically.

use crate::prelude::*;

/// Command spec for `eval`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "eval",
        traits: Traits::CREATES_BARRIER | Traits::EVALUATES_CODE | Traits::TAINT_SINK,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Body)],
        lowering_hook: Some(crate::hooks::LoweringHookId::Eval),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Evaluate a Tcl script.",
            &["eval arg ?arg ...?"],
            "Tcl eval(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
