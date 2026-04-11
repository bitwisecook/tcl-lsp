//! `exec` — invoke subprocesses.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exec",
        traits: Traits::TAINT_SINK | Traits::UNSAFE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Process,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Invoke subprocesses.",
            &["exec ?-option ...? arg ?arg ...?"],
            "Tcl exec(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
