//! `DATAGRAM::ip` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::ip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns ip header information.",
            synopsis: &["DATAGRAM::ip (tos | ttl | flags)", "DATAGRAM::ip (option | option_count) (IPV4_OPTION)?"],
            snippet: "This iRules command returns ip header information.\n\nDATAGRAM::ip tos\n\n     * Returns IP header ToS as an integer value.\n\nDATAGRAM::ip ttl\n\n     * Returns IP header TTL as an integer value.\n\nDATAGRAM::ip flags\n\n     * Returns IP header flags as an integer value. The flags are from the\n       IP datagram after IP fragment reassembly. Any MF flags that were\n       present in indivdual fragments will not be returned. DF flag is\n       preserved if it was set.\n\nDATAGRAM::ip option\n\n     * This command returns a Tcl list of IP options from reassembled IP\n       datagram.",
            source: "https://clouddocs.f5.com/api/irules/DATAGRAM__ip.html",
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
            FormSpec { kind: FormKind::Default, synopsis: "DATAGRAM::ip (tos | ttl | flags)" },
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
