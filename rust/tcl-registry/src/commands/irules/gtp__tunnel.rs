//! `GTP::tunnel` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::tunnel",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "These commands parse the payload of G-PDU as IP datagram and return the values from IP header and TCP/UDP header.",
            synopsis: &["GTP::tunnel ('is_ip'"],
            snippet: "These commands parse the payload of G-PDU as IP datagram and return the\nvalues from IP header and TCP/UDP header.\nWhen parsed payload contains a value other than 4 or 6 for IP version,\nthe commands return an empty value. \"is_ip\" can be used to confirm if\nparser is considering the payload as ip-datagram or not. The commands\nreturn empty for non G-PDU messages.\ntcp_ and udp_ commands return empty value if the ip-proto in the\nip-datagram does not match. \"GTP::tunnel ip_proto\" may be used to\nverify before calling transport level commands.",
            source: "https://clouddocs.f5.com/api/irules/GTP__tunnel.html",
            examples: "when GTP_SIGNALLING_INGRESS {\n    log local0. \"GTP tunnel TCP src port [GTP::tunnel tcp_src_port]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "GTP::tunnel <subcommand> ?-message MESSAGE?",
        }],
        options: &[OptionSpec {
            name: "-message",
            takes_value: true,
            value_hint: "MESSAGE",
            detail: "Operate on a specific GTP message object.",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
