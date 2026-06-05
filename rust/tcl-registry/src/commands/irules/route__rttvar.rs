//! `ROUTE::rttvar` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::rttvar",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the cached round-trip-time variance (rttvar) estimate.",
            synopsis: &["ROUTE::rttvar DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns the cached round-trip-time variance (rttvar) for the\ndestination and/or gateway if the relevant TCP profile enables\ncmetrics-cache.\n\nThe return value only applies to the TMM executing the command. It\ndoes not consider cache entries on other TMMs.\n\nROUTE::rttvar returns a value of 0 when there are no statistics\navailable.\n\nNOTE: The returned value is scaled to units of 100ns; to express it\nin the same units as TCP::rttvar multiply it by 16/10000.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__rttvar.html",
            examples: "when CLIENT_ACCEPTED {\n    log local0. \"Cached rttvar is: [ROUTE::rttvar [IP::remote_addr]]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ROUTE::rttvar DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?" },
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
