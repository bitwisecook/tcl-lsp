//! `exec` — invoke subprocesses.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exec",
        traits: Traits::TAINT_SINK | Traits::TAINT_SOURCE | Traits::UNSAFE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Process,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
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
            },
            OptionSpec {
                name: "-keepnewline",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
            },
        ],
        hover: Some(HoverSnippet::brief(
            "Invoke subprocesses.",
            &["exec ?-option ...? arg ?arg ...?"],
            "Tcl exec(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
