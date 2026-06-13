//! `SCTP::respond` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sends the specified data directly to the peer.",
            synopsis: &["SCTP::respond (PAYLOAD_DATA (PAYLOAD_OFFSET)? (PAYLOAD_LENGTH)?)"],
            snippet: "Sends the specified data directly to the peer. This command is used to complete a protocol handshake with an iRule.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__respond.html",
            examples: "when CLIENT_ACCEPTED {\n    SCTP::respond \"sctpdata\" 0 1\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SCTP::respond (PAYLOAD_DATA (PAYLOAD_OFFSET)? (PAYLOAD_LENGTH)?)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
