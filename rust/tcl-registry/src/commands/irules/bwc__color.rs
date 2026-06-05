//! `BWC::color` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::color",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command is used to classify a traffic flow to a particular color (application category).",
            synopsis: &["BWC::color ('set' | 'unset') POLICY_NAME APPLICATION_NAME"],
            snippet: "After a flow has been assigned a policy, at some later time when the traffic is classified the user can assign an application to this flow. This uses the bwc config to create a bwc policy with the categories keyword.",
            source: "https://clouddocs.f5.com/api/irules/BWC__color.html",
            examples: "when CLIENT_ACCEPTED {\n    set mycookie [IP::remote_addr]:[TCP::remote_port]\n    BWC::policy attach gold_user $mycookie\n    BWC::color set gold_user p2p\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
