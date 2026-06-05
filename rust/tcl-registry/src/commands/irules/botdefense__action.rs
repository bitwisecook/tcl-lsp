//! `BOTDEFENSE::action` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::action",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or overrides the action to be taken by Bot Defense.",
            synopsis: &["BOTDEFENSE::action (allow |"],
            snippet: "Returns or overrides the action to be taken by Bot Defense.\n\nOverriding the action may fail on certain cases. For example, overriding to the \"browser_challenge\" action, may only be done on requests to which the value of BOTDEFENSE::cs_possible is \"true\". When overriding the action, the command returns \"ok\" if the action was successfully set. Otherwise, the action is not changed, and the reason for failure is returned.\n\nAfter a successful action override (resulting in the \"ok\" string), the action cannot be overridden again.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__action.html",
            examples: "when HTTP_RESPONSE_RELEASE {\n    if {[info exists botdefense_responded]} {\n        HTTP::header insert \"myheader\" \"blocked request\"\n    }\n}",
            return_value: "* When called without any arguments: Returns a string signifying the action to be taken by Bot Defense. If the action was overridden, the returned action is the overridden one. * When called with an argument for overriding the action, the return value is a status string.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
