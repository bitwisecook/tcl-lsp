//! `LINK::vlan_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::vlan_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the VLAN tag of the packet.",
            synopsis: &["LINK::vlan_id"],
            snippet: "Returns the VLAN tag of the packet. This command is equivalent to the\nBIG-IP 4.X variable vlan_id.",
            source: "https://clouddocs.f5.com/api/irules/LINK__vlan_id.html",
            examples: "# log requests\nwhen CLIENT_ACCEPTED {\n    set info \"client { [IP::client_addr]:[TCP::client_port] -> [IP::local_addr]:[TCP::local_port] }\"\n    append info \" ethernet \"\n    append info \" { [string range [LINK::lasthop] 0 16] -> [string range [LINK::nexthop] 0 16] \"\n    append info \"tag [LINK::vlan_id] qos [LINK::qos] }\"\n    log local0. $info\n}",
            return_value: "LINK::vlan_id",
        }),
        ..CommandSpec::DEFAULT
    }
}
