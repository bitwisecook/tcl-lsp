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
hover: Some(HoverSnippet {
            summary: "Closes an existing sideband connection.",
            synopsis: &["close CONNECTION"],
            snippet: "This command closes an existing sideband connection. It is one of several commands that make up the ability to create sideband connections from iRules.",
            source: "https://clouddocs.f5.com/api/irules/close.html",
            examples: "# Open a sideband connection with a connection timeout of 100 ms and an idle timeout of 30 seconds\n#   to a local virtual server name sideband_virtual_server\nset conn_id [connect -timeout 100 -idle 30 -status conn_status sideband_virtual_server]\n\n# Same as above, but use an external host IP:port instead of a virtual server name\nset conn_id [connect -timeout 100 -idle 30 -status conn_status 10.0.0.10:80]\n\n# close the connection\nclose conn_id",
            return_value: "close <connection> closes an existing connection",
        }),
        ..CommandSpec::DEFAULT
    }
}
