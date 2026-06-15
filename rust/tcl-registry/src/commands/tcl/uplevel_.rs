//! `uplevel` — execute a script in a different stack frame.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uplevel ?level? arg ?arg ...?",
}];

/// Command spec for `uplevel`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uplevel",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::EVALUATES_CODE
            | Traits::TAINT_SINK
            | Traits::UNSAFE
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        lowering_hook: Some(crate::hooks::LoweringHookId::Uplevel),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Execute a script in a different stack frame",
            synopsis: &["uplevel ?level? arg ?arg ...?"],
            snippet: "All of the arg arguments are concatenated as if they had been passed to concat; the result is then evaluated in the variable context indicated by level.",
            source: "Tcl man page uplevel.n",
            examples: "",
            return_value: "",
        }),
        // GAP-D2: a `LIST_CANONICAL` value preserves element
        // boundaries and suppresses T100. Mirrors `tcl/uplevel.py`.
        taint_sink_safe_colour: Some(TaintColour::LIST_CANONICAL),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        xc_translatable: Some(false),
        ..CommandSpec::DEFAULT
    }
}
