//! `proc` iRules command.
//!
//! Structurally identical to Tcl's `proc` — same arity, same
//! argument roles. Carrying the same `arg_roles` here means when
//! the iRules dialect is loaded into a shared registry, body-role
//! lookups (folding, document symbols, …) keep finding the body at
//! index 2 instead of falling off because the iRules override
//! shadows the Tcl spec with empty roles.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "proc",
        traits: Traits::DEFINES_PROCEDURE | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(3),
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        hover: Some(HoverSnippet::brief(
            "Define an iRule proc.",
            &["proc NAME ARGUMENT_N_DEFAULT PROC_SCRIPT"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
