//! `lassign` — assign list elements to variables.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lassign",
        traits: Traits::FRAMELESS_RUNTIME,
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        arg_roles: &[
            (1, ArgRole::VarWrite),
            (2, ArgRole::VarWrite),
            (3, ArgRole::VarWrite),
            (4, ArgRole::VarWrite),
            (5, ArgRole::VarWrite),
        ],
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Assign list elements to variables.",
            &["lassign list ?varName ...?"],
            "Tcl lassign(1)",
        )),
        codegen_hook: Some(CodegenHookId::Lassign),
        ..CommandSpec::DEFAULT
    }
}
