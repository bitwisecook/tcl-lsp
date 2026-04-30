//! `lset` — change an element in a list variable.
use crate::forms::CommandForm;
use crate::hooks::CodegenHookId;
use crate::prelude::*;

/// `lset varName newValue` — replace the entire list (no index).
const LSET_REPLACE: CommandForm = CommandForm {
    name: "replace",
    arity: Arity::exact(2),
    arg_roles: &[(0, ArgRole::VarWrite)],
    codegen_hook: Some(CodegenHookId::Lset),
    ..CommandForm::DEFAULT
};

/// `lset varName index newValue` — single-level update.
const LSET_SINGLE_INDEX: CommandForm = CommandForm {
    name: "single_index",
    arity: Arity::exact(3),
    arg_roles: &[(0, ArgRole::VarWrite)],
    codegen_hook: Some(CodegenHookId::Lset),
    ..CommandForm::DEFAULT
};

/// `lset varName index1 ?index2 ...? newValue` — multi-level path.
const LSET_FLAT_PATH: CommandForm = CommandForm {
    name: "flat_path",
    arity: Arity::at_least(4),
    arg_roles: &[(0, ArgRole::VarWrite)],
    codegen_hook: Some(CodegenHookId::Lset),
    ..CommandForm::DEFAULT
};

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
        command_forms: &[LSET_REPLACE, LSET_SINGLE_INDEX, LSET_FLAT_PATH],
        ..CommandSpec::DEFAULT
    }
}
