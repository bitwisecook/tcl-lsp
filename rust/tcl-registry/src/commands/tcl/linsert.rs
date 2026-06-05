//! `linsert` — insert elements into a list.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "linsert list index ?element element ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "linsert",
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED | Traits::PURE,
        arity: Arity::at_least(2),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet::brief(
            "Insert elements into a list.",
            &["linsert list index ?element ...?"],
            "Tcl linsert(1)",
        )),
        codegen_hook: Some(CodegenHookId::Linsert),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
