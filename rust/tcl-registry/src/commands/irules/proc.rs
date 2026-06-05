//! `proc` iRules command.
//!
//! Structurally identical to Tcl's `proc` — same arity, same
//! argument roles. Carrying the same `arg_roles` here means when
//! the iRules dialect is loaded into a shared registry, body-role
//! lookups (folding, document symbols, …) keep finding the body at
//! index 2 instead of falling off because the iRules override
//! shadows the Tcl spec with empty roles.
use crate::hooks::LoweringHookId;
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
        lowering_hook: Some(LoweringHookId::Proc),
        body_kind: BodyKind::Structural,
hover: Some(HoverSnippet {
            summary: "Define an iRule proc.",
            synopsis: &["proc NAME ARGUMENT_N_DEFAULT PROC_SCRIPT"],
            snippet: "Define an iRule proc which is called by iRule command call.\n\nThe syntax is same as basic TCL proc command.",
            source: "https://clouddocs.f5.com/api/irules/proc.html",
            examples: "when CLIENT_DATA {\n    call logme \"Coming to CLIENT_DATA\"\n}",
            return_value: "Returns the value in the return command, if any, in the proc script.",
        }),
        ..CommandSpec::DEFAULT
    }
}
