//! `llength` — return the number of elements in a list.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "llength list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "llength",
        const_fold: Some(crate::const_fold::fold_llength),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::exact(1),
        return_type: Some(TclType::Int),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet::brief(
            "Return the number of elements in a list.",
            &["llength list"],
            "Tcl llength(1)",
        )),
        codegen_hook: Some(CodegenHookId::Llength),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
