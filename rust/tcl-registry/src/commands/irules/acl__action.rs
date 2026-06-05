//! `ACL::action` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACL::action",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets or retrieves the current ACL action.",
            synopsis: &["ACL::action (default |"],
            snippet: "The ACL::action command allows you to determine the ACL action in the\nFLOW_INIT event. This command requires the Advanced Firewall\nManager module.",
            source: "https://clouddocs.f5.com/api/irules/ACL__action.html",
            examples: "when FLOW_INIT {\n  if { [IP::addr [IP::client_addr] equals 172.29.97.151] } {\n    ACL::action allow\n    virtual /Common/my_http_vs\n    log \"FLOW_INIT: ACL allow to /Common/my_http_vs\"\n  }\n}",
            return_value: "When no argument is provided, the command will return an integer value corresponding to an action that will be taken: + 0 is a drop + 1 is reset (or reject) + 2 is allow (or accept) + 3 is allow-final (or accept-decisively)",
        }),
        ..CommandSpec::DEFAULT
    }
}
