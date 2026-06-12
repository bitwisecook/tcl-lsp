//! `ROUTE::expiration` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::expiration",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the remaining time for a route or congestion metrics cache entry.",
            synopsis: &["ROUTE::expiration DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?"],
            snippet: "Returns the remaining time in seconds. The lifetime of an entry may\nhave been set by the route.metrics.timeout sys db variable, the\ncmetrics-cache-timeout TCP profile attribute, or a\nTCP::rt_metrics_timeout iRule.\n\nThe return value only applies to the TMM executing the command. It\ndoes not consider cache entries on other TMMs.",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__expiration.html",
            examples: "when CLIENT_CLOSED {\n    # If the entry almost timed out, keep it a little longer next time.\n    set time_remaining [ROUTE::expiration [IP::remote_addr]]\n    if { $time_remaining > 0 && $time_remaining < 100 } {\n         # Default value is 600\n         TCP::rt_metrics_timeout 700\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ROUTE::expiration DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?" },
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
