//! `tailcall` — replace the current procedure with another command.

use crate::hooks::CodegenHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tailcall command ?arg ...?",
}];

/// Command spec for `tailcall`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tailcall",
        traits: Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        // Tcl 9.0: ``tailcall`` with no args clears any scheduled
        // tailcall; with args it replaces it.  Real arity is 0..∞.
        arity: Arity::any(),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Replace the current procedure with another command",
            synopsis: &["tailcall command ?arg ...?"],
            snippet: "The tailcall command replaces the currently executing procedure, lambda application, or method with another command.",
            source: "Tcl man page tailcall.n",
            examples: "",
            return_value: "",
        }),
        codegen_hook: Some(CodegenHookId::Tailcall),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
