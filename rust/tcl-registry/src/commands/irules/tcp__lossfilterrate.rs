//! `TCP::lossfilterrate` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::lossfilterrate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the TCP Loss Ignore Rate Parameter.",
            synopsis: &["TCP::lossfilterrate"],
            snippet: "Gets the maximum number of packets per million lost before triggering congestion response.\n  * Rate range is valid from 0 to 1,000,000. Rate is X packets lost per\n    million before congestion control kicks in.",
            source: "https://clouddocs.f5.com/api/irules/TCP__lossfilterrate.html",
            examples: "when SERVER_CONNECTED {\n    # Remove loss filter if present\n    if { [TCP::lossfilterrate] > 0 } {\n        TCP::lossfilter 0 0\n    }\n}",
            return_value: "TCP Loss Ignore Rate in packets per million.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::lossfilterrate",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
