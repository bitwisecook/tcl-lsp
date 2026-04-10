//! `proc` — define a procedure.

use crate::prelude::*;

/// Command spec for `proc`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "proc",
        traits: Traits::LANGUAGE_KEYWORD
            | Traits::DEFINES_PROCEDURE
            | Traits::NEVER_INLINE_BODY
            | Traits::IRULES_TOP_LEVEL_ONLY,
        arity: Arity::exact(3),
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Define a procedure.",
            &["proc name args body"],
            "Tcl proc(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
