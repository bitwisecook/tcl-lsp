//! `ACL::eval` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACL::eval",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enforce ACLs in your connections.",
            synopsis: &["ACL::eval ('-l7')?"],
            snippet: "The ACL::eval command allows admin to enforce ACLs for a\ngiven connection through APM network access tunnels.\n\n * Requires APM module and network access",
            source: "https://clouddocs.f5.com/api/irules/ACL__eval.html",
            examples: "when CLIENT_ACCEPTED {\n    ACL::eval\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
