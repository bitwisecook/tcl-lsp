//! `IP::version` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the IP version of a connection.",
            synopsis: &["IP::version"],
            snippet: "Returns the IP version of a connection. When called in a clientside event, this command returns the IP version for the clientside connection. When called in a serverside event, this command returns the IP version for the serverside connection.",
            source: "https://clouddocs.f5.com/api/irules/IP__version.html",
            examples: "when CLIENT_ACCEPTED {\n   log local0. \"Client [IP::client_addr], VS: [IP::local_addr],\\\n      \\[IP::version\\]: [IP::version], \\[IP::protocol\\]: [IP::protocol]\"\n}",
            return_value: "IP version of a connection",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IP::version",
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
