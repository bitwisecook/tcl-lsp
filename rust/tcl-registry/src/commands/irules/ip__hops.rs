//! `IP::hops` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::hops",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gives you the estimated number of hops the peer takes to get to you.",
            synopsis: &["IP::hops"],
            snippet: "This command is used to give you the estimated number of hops between the peer in question, and the client machine making the request.",
            source: "https://clouddocs.f5.com/api/irules/IP__hops.html",
            examples: "when HTTP_REQUEST {\n  if { [IP::hops] >= 10 } {\n      COMPRESS::disable\n  }\n}",
            return_value: "Number of hops",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IP::hops",
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
