//! `connect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "connect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Establishes a sideband connection.",
            synopsis: &["connect info (", "connect (("],
            snippet: "This command establishes a sideband connection. It is one of several commands that make up the ability to use sideband connections from iRules.",
            source: "https://clouddocs.f5.com/api/irules/connect.html",
            examples: "# Open a sideband connection with a connection timeout of 100 ms and an idle timeout of 30 seconds\n#   to a local virtual server name sideband_virtual_server\nset conn_id [connect -timeout 100 -idle 30 -status conn_status sideband_virtual_server]\n\n# Same as above, but use an external host IP:port instead of a virtual server name\nset conn_id [connect -timeout 100 -idle 30 -status conn_status 10.0.0.10:80]\n\n\nExample with more complete error handling:",
            return_value: "This command opens a sideband connection to the specified destination.",
        }),
        // GAP-D2: sideband `connect` is a network sink (SSRF, T104);
        // the address-bearing arg positions are not pinned. Mirrors
        // `irules/connect.py` (`taint_network_sink_args=()`).
        taint_network_sink_args: Some(&[]),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: true,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "connect ?options? destination" },
        ],
        options: &[
            OptionSpec { name: "-protocol", takes_value: true, value_hint: "PROTO", detail: "IP protocol (default TCP).", dialects: None },
            OptionSpec { name: "-myaddr", takes_value: true, value_hint: "IP_ADDR", detail: "Source address for the connection.", dialects: None },
            OptionSpec { name: "-myport", takes_value: true, value_hint: "PORT", detail: "Source port for the connection.", dialects: None },
            OptionSpec { name: "-timeout", takes_value: true, value_hint: "MSEC", detail: "Time in ms to wait for connection.", dialects: None },
            OptionSpec { name: "-idle", takes_value: true, value_hint: "SEC", detail: "Idle timeout in seconds (default 300).", dialects: None },
            OptionSpec { name: "-tos", takes_value: true, value_hint: "TOS", detail: "IP TOS value.", dialects: None },
            OptionSpec { name: "-status", takes_value: true, value_hint: "VARIABLE", detail: "Save connection status into variable.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
