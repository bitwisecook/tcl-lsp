//! `uplevel` — execute a script in a different stack frame.

use crate::prelude::*;

/// Command spec for `uplevel`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uplevel",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::EVALUATES_CODE
            | Traits::TAINT_SINK
            | Traits::UNSAFE,
        arity: Arity::at_least(1),
        lowering_hook: Some(crate::hooks::LoweringHookId::Uplevel),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Execute a script in a different stack frame.",
            &["uplevel ?level? arg ?arg ...?"],
            "Tcl uplevel(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
