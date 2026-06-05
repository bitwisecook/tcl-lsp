//! `MR::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets details in the message routing table.",
            synopsis: &["MR::message clone (CLONE_ID)+", "MR::message clone -count CLONE_COUNT"],
            snippet: "Clones the message a number of times (one for each CLONE_ID) and dispatches each cloned\nmessage as ingress. After the original message has completed the event in which this command\nis executed, each cloned message executes the MR_INGRESS iRule event for itself.\n(CLONE_ID)+ can be one or more strings separated by space.\nProtection against infinite loops should be considered!\nReturns the clone_count, see below, (allowed only at MR_INGRESS).\n            \nClones the message CLONE_COUNT number of times and dispatches each cloned\nmessage as ingress.",
            source: "https://clouddocs.f5.com/api/irules/MR__message.html",
            examples: "# Example 1\nwhen MR_INGRESS {\n    if {[GENERICMESSAGE::message is_request] != 0} {\n        set host [MR::message pick_host peer /Common/mypeer]\n        MR::message route config tcp_tc host $host\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::message clone (CLONE_ID)+" },
        ],
        options: &[
            OptionSpec { name: "-count", takes_value: true, value_hint: "", detail: "Option -count.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
