//! `TCP::rttvar` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rttvar",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns TCP's smoothed RTT variance estimate.",
            synopsis: &["TCP::rttvar"],
            snippet: "Returns the Round Trip Time Variance, which is an indication of path jitter. TCP uses this figure, combined with RTT, to compute the RTO.\n\nNote that the value returned is in units of \"1/16 of a millisecond\". Divide the returned value by 16 to get the actual variance in milliseconds.",
            source: "https://clouddocs.f5.com/api/irules/TCP__rttvar.html",
            examples: "when CLIENT_CLOSED {\n    # Log rttvar.\n    log local0. \"rttvar: [TCP::rttvar]\"\n}",
            return_value: "The measured RTT variance in units of \"1/16 of a millisecond\". Divide the returned value by 16 to get the actual variance in milliseconds.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::rttvar" },
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
