//! `lrange` — return a range of list elements.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lrange list first last",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lrange",
        const_fold: Some(crate::const_fold::fold_lrange),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::exact(3),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Return one or more adjacent elements from a list",
            synopsis: &["lrange list first last"],
            snippet: "List must be a valid Tcl list.",
            source: "Tcl man page lrange.n",
            examples: "",
            return_value: "",
        }),
        codegen_hook: Some(CodegenHookId::Lrange),
        forms: FORMS,
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::List),
                    shimmers: true,
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..CommandSpec::DEFAULT
    }
}
