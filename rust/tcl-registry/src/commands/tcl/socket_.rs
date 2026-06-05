//! `socket` — open a TCP network connection.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "socket",
        traits: Traits::BYTE_COMPILED | Traits::OPENS_CHANNEL | Traits::TAINT_SOURCE,
        arity: Arity::at_least(2),
        return_type: Some(TclType::Channel),
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Open a TCP network connection.",
            &[
                "socket ?-option value ...? host port",
                "socket -server command ?-option value ...? port",
            ],
            "Tcl socket(1)",
        )),
        // GAP-D2: `host`/`port` are network-address args — SSRF sink
        // (T104). Mirrors `tcl/socket_.py`.
        taint_network_sink_args: Some(&[0, 1]),
        ..CommandSpec::DEFAULT
    }
}
