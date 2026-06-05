//! `AUTH::response_data` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::response_data",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns pairwise auth query results.",
            synopsis: &["AUTH::response_data (AUTH_ID)?"],
            snippet: "AUTH::response_data returns the a set of name/value query results from\nthe most recent query. This command would normally be called from the\nAUTH_RESULT event. The format of the data returned is suitable for\nsetting as the value of a TCL array.\nAUTH::subscribe must first be called to register interest in query\nresults prior to calling AUTH::authenticate. As a convenience when\nusing the builtin system auth rules, these rules will call\nAUTH::subscribe if the variable tmm_auth_subscription is set.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__response_data.html",
            examples: "when CLIENT_ACCEPTED {\n        set tmm_auth_subscription \"*\"\n    }",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
