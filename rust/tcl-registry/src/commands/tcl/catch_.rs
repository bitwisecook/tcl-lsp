//! `catch` — evaluate script and trap exceptional returns.

use crate::prelude::*;

/// Command spec for `catch`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "catch",
        traits: Traits::CONTROL_FLOW | Traits::LANGUAGE_KEYWORD,
        arity: Arity::new(1, 3),
        arg_roles: &[
            (0, ArgRole::Body),
            (1, ArgRole::VarWrite),
            (2, ArgRole::VarWrite),
        ],
        lowering_hook: Some(crate::hooks::LoweringHookId::Catch),
        return_type: Some(TclType::Int),
        hover: Some(HoverSnippet::brief(
            "Evaluate script and trap exceptional returns.",
            &["catch script ?resultVarName? ?optionsVarName?"],
            "Tcl catch(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
