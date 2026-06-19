//! `return` — return from the current procedure or script.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "return ?-code code? ?-level level? ?result?",
}];

/// Command spec for `return`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "return",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
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
        hover: Some(HoverSnippet {
            summary: "Return from the current procedure/script with optional control-code metadata.",
            synopsis: &["return ?-code code? ?-level level? ?result?"],
            snippet: "Advanced forms can emulate `break`, `continue`, or custom return codes.",
            source: "Tcl return(1)",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Return),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
