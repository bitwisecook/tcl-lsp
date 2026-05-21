//! `upvar` — create link to variable in a different stack frame.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Command spec for `upvar`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "upvar",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::CREATES_DYNAMIC_BARRIER,
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Create link to variable in a different stack frame.",
            &["upvar ?level? otherVar myVar ?otherVar myVar ...?"],
            "Tcl upvar(1)",
        )),
        lowering_hook: Some(LoweringHookId::Upvar),
        ..CommandSpec::DEFAULT
    }
}
