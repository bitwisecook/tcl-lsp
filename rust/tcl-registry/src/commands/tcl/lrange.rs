//! `lrange` — return a range of list elements.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lrange",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::exact(3),
        return_type: Some(TclType::List),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet::brief(
            "Return a range of list elements.",
            &["lrange list first last"],
            "Tcl lrange(1)",
        )),
        codegen_hook: Some(CodegenHookId::Lrange),
        ..CommandSpec::DEFAULT
    }
}
