//! `upvar` — create link to variable in a different stack frame.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "upvar ?level? otherVar myVar ?otherVar myVar ...?",
}];

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
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN,
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
hover: Some(HoverSnippet {
    summary: "Create link to variable in a different stack frame",
    synopsis: &["upvar ?level? otherVar myVar ?otherVar myVar ...?"],
    snippet: "This command arranges for one or more local variables in the current procedure to refer to variables in an enclosing procedure call or to global variables.",
    source: "Tcl man page upvar.n",
    examples: "",
    return_value: "",
}),
        lowering_hook: Some(LoweringHookId::Upvar),
        forms: FORMS,
        xc_translatable: Some(false),
        ..CommandSpec::DEFAULT
    }
}
