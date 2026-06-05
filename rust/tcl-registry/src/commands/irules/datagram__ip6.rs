//! `DATAGRAM::ip6` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::ip6",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns ipv6 header information.",
            synopsis: &["DATAGRAM::ip6 hop_limit", "DATAGRAM::ip6 (option | option_count) (IPV6_OPTION)?"],
            snippet: "This iRules command returns ipv6 header information.\nNote: throws an error when used with IPv4\n\nDATAGRAM::ip6 hop_limit\n\n     * This command returns IPv6 hop limit as an integer value.\n\nDATAGRAM::ip6 option\n\n     * This command returns a Tcl list of IPv6 options from reassembled\n       IPv6 datagram. Each option is a Tcl list with one or two values -\n       option code (integer), and option value (byte array) if option has\n       the value. Multiple options with the same code will be returned as\n       separate sublists.",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__ip6.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &["CLIENT_DATA"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DATAGRAM::ip6 hop_limit" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
