//! `ROUTE::bandwidth` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::bandwidth",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a bandwidth estimate for a destination derived from entries in the congestion metrics cache.",
            synopsis: &["ROUTE::bandwidth DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns a bandwidth estimate for a destination derived from\nentries in the congestion metrics cache.\n\nAs of v12.0, divides the cached congestion window (cwnd) value\nby the cached round-trip-time (RTT ) to obtain a bandwidth\nestimate in kbps. If there is no entry, it returns 0.\n\nNote: The return value only applies to the TMM executing the command.\nIt does not consider cache entries on other TMMs.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__bandwidth.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [ROUTE::bandwidth [IP::remote_addr]] > 0 } {\n        log local0. \"cached bandwidth is: [ROUTE::bandwidth [IP::remote_addr]]\"\n    }\n}",
            return_value: "The bandwidth estimate to the destination and/or gateway in kbps.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ROUTE::bandwidth DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
