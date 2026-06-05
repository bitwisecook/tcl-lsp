//! `SCTP::remote_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::remote_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the remote SCTP port/service number.",
            synopsis: &["SCTP::remote_port (clientside | serverside)?"],
            snippet: "Returns the remote SCTP port/service number. Can specify the port value on clientside or serverside.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__remote_port.html",
            examples: "when CLIENT_ACCEPTED {\n    SCTP::remote_port\n    set x [SCTP::remote_port]\n    SCTP::remote_port clientside\n    SCTP::remote_port serverside\n    SCTP::remote_port client\n    SCTP::remote_port server\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SCTP::remote_port (clientside | serverside)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
