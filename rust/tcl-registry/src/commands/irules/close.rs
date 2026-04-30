//! `close` iRules command.
//!
//! iRules' `close` closes a sideband connection — the first
//! positional argument plays the same channel-id role as Tcl's
//! `close channelId`. Mirroring `arg_roles` here keeps the
//! channel-type diagnostic working when the iRules dialect is
//! loaded (otherwise the iRules override shadows the Tcl spec
//! with empty roles and the diagnostic silently stops firing).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Channel)],
        hover: Some(HoverSnippet::brief(
            "Closes an existing sideband connection.",
            &["close CONNECTION"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
