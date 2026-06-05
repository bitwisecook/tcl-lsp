//! `ROUTE::cwnd` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::cwnd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the cached congestion window (cwnd) value.",
            synopsis: &["ROUTE::cwnd DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns the cached congestion window (cwnd) value for a given\ndestination IP and/or gateway.\n\nThe return value only applies to the TMM executing the command. It\ndoes not consider cache entries on other TMMs.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__cwnd.html",
            examples: "when CLIENT_ACCEPTED {\n    set cwnd [ROUTE::cwnd [IP::remote_addr]]\n    if { $cwnd > 0 } {\n        log local0. \"Destination found in cache. Initializing cwnd to $cwnd\"\n    } else {\n        log local0. \"Destination not found in cache.\"\n    }\n}",
            return_value: "The cached congestion window in bytes.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ROUTE::cwnd DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
