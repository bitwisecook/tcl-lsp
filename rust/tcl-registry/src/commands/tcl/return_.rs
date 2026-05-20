//! `return` — return from the current procedure or script.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Command spec for `return`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "return",
        traits: Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::TERMINATES_BLOCK
            | Traits::NEEDS_START_CMD,
        arity: Arity::any(),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Return from the current procedure/script with optional control-code metadata.",
            &["return ?-code code? ?-level level? ?result?"],
            "Tcl return(1)",
        )),
        lowering_hook: Some(LoweringHookId::Return),
        ..CommandSpec::DEFAULT
    }
}
