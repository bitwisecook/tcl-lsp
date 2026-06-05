//! `POLICY::rules` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::rules",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the policy rules of the supplied policy that had actions executed.",
            synopsis: &["POLICY::rules ('matched')? POLICY_NAME"],
            snippet: "Returns the policy rules of the supplied policy that had actions\nexecuted.",
            source: "https://clouddocs.f5.com/api/irules/policy__rules.html",
            examples: "# Log the policy targets for this virtual server\nwhen HTTP_REQUEST {\n\n        log local0. \"Looping through \\[POLICY::names matched\\]: [POLICY::names matched]\"\n        foreach policy [POLICY::names matched] {\n                log local0. \"\\[POLICY::rules matched $policy\\]: [POLICY::rules matched $policy]\"\n        }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
