//! `socket` — open a TCP network connection.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-myaddr",
        takes_value: true,
        value_hint: "addr",
        detail: "Client-side local interface.",
        dialects: None,
    },
    OptionSpec {
        name: "-myport",
        takes_value: true,
        value_hint: "port",
        detail: "Client-side local port.",
        dialects: None,
    },
    OptionSpec {
        name: "-async",
        takes_value: false,
        value_hint: "",
        detail: "Connect asynchronously.",
        dialects: None,
    },
    OptionSpec {
        name: "-server",
        takes_value: true,
        value_hint: "command",
        detail: "Server accept callback.",
        dialects: None,
    },
    OptionSpec {
        name: "-reuseaddr",
        takes_value: true,
        value_hint: "boolean",
        detail: "Allow server address reuse (default 1).",
        dialects: None,
    },
    OptionSpec {
        name: "-reuseport",
        takes_value: true,
        value_hint: "boolean",
        detail: "Allow server port reuse (default 0).",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "socket ?options? host port",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "socket -server command ?options? port",
    },
];

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
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
