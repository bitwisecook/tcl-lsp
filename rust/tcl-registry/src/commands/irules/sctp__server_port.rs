//! `SCTP::server_port` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::server_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the SCTP port/service number of the specified server.",
            synopsis: &["SCTP::server_port"],
            snippet: "Returns the SCTP port/service number of the specified server. This command is equivalent to the command serverside { SCTP::remote_port }.\n\nSCTP::server_port\n    Returns the SCTP port/service number of the specified server.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__server_port.html",
            examples: "when SERVER_CONNECTED {\n    if { [SCTP::server_port] > 1000 } {\n        pool slow_pool\n     }\n      else {\n         pool fast_pool\n       }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SCTP::server_port",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
