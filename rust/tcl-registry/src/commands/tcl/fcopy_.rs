//! `fcopy` — copy data from one channel to another.

use crate::prelude::*;

/// Command spec for `fcopy`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fcopy",
        // C Tcl 9.0 ``Tcl_FcopyObjCmd`` accepts up to four optional
        // option-pair flags after the two channels (``-size N``,
        // ``-command cb``).  Args after command name: 2..6.
        arity: Arity::new(2, 6),
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::Channel)],
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Copy data from one channel to another.",
            &["fcopy inputChan outputChan ?-size size? ?-command callback?"],
            "Tcl fcopy(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
