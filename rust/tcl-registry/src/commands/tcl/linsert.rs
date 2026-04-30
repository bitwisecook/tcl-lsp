//! `linsert` — insert elements into a list.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "linsert",
        traits: Traits::PURE,
        arity: Arity::at_least(2),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet::brief(
            "Insert elements into a list.",
            &["linsert list index ?element ...?"],
            "Tcl linsert(1)",
        )),
        codegen_hook: Some(CodegenHookId::Linsert),
        ..CommandSpec::DEFAULT
    }
}
