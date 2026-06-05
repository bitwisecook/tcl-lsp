//! `IP::ingress_rate_limit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::ingress_rate_limit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `IP::ingress_rate_limit`.",
            synopsis: &["IP::ingress_rate_limit"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/IP__ingress_rate_limit.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
