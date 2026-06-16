//! `exec` — invoke subprocesses.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exec ?switches? arg ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exec",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::BYTE_COMPILED | Traits::TAINT_SINK | Traits::TAINT_SOURCE | Traits::UNSAFE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Process,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::None,
            },
            // Mirrors Python `exec_.py` (INTERP_STATE).
            SideEffect {
                target: SideEffectTarget::InterpState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
            },
        ],
        // Mirrors ``core/commands/registry/tcl/exec_.py``.  ``--``
        // is the option terminator that drives W304's
        // ``resolve_option_terminator`` lookup; the registry also
        // surfaces the two boolean switches for completion.
        options: &[
            OptionSpec {
                name: "-ignorestderr",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-keepnewline",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet::brief(
            "Invoke subprocesses.",
            &["exec ?-option ...? arg ?arg ...?"],
            "Tcl exec(1)",
        )),
        // GAP-D2: a `SHELL_ATOM`-coloured value is token-safe and
        // suppresses T100. Mirrors `tcl/exec_.py`.
        taint_sink_safe_colour: Some(TaintColour::SHELL_ATOM),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
