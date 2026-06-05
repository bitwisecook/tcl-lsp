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
hover: Some(HoverSnippet {
    summary: "Insert elements into a list",
    synopsis: &["linsert list index ?element element ...?", "linsert list index ?element ...?"],
    snippet: "This command produces a new list from list by inserting all of the element arguments just before the index'th element of list.",
    source: "Tcl man page linsert.n",
    examples: "",
    return_value: "",
}),
        codegen_hook: Some(CodegenHookId::Linsert),
        forms: FORMS,
        arg_types: &[(0, ArgTypeHint { expected: Some(TclType::List), shimmers: true }), (1, ArgTypeHint { expected: Some(TclType::Int), shimmers: true })],
        ..CommandSpec::DEFAULT
    }
}
