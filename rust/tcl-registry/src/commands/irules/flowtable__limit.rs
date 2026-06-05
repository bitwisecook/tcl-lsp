//! `FLOWTABLE::limit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOWTABLE::limit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns configured connection limits.",
            synopsis: &["FLOWTABLE::limit virtual (VIRTUAL_SERVER_OBJ)?", "FLOWTABLE::limit route_domain (ROUTE_DOMAIN_NAME)?"],
            snippet: "This iRules command returns configured connection limits\nNote: When virtual server or route domain name is omitted the commands\nuse virtual or route domain of the current connection. Specifying the\nname incurs significant performance hit.",
            source: "https://clouddocs.f5.com/api/irules/FLOWTABLE__limit.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
