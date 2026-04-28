//! `lset` — change an element in a list variable.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lset",
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet::brief(
            "Change an element in a list variable.",
            &["lset varName ?index ...? newValue"],
            "Tcl lset(1)",
        )),
        codegen_hook: Some(CodegenHookId::Lset),
        ..CommandSpec::DEFAULT
    }
}
