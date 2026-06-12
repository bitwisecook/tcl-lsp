//! `SCTP::client_port` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::client_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the SCTP port/service number of the specified client.",
            synopsis: &["SCTP::client_port"],
            snippet: "Returns the SCTP port/service number of the specified client. This command is equivalent to the command clientside { SCTP::remote_port }.\n\nSCTP::client_port\n    Returns the SCTP port/service number of the specified client.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__client_port.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [SCTP::client_port] > 1000 } {\n        pool slow_pool\n     }\n      else {\n         pool fast_pool\n       }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SCTP::client_port" },
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
