//! `clone` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clone",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Causes the system to clone traffic to the specified pool, pool member or vlan regardless of monitor status.",
            synopsis: &["clone pool POOL_OBJ (member IP_ADDR (PORT)?)?", "clone nexthop VLAN_OBJ"],
            snippet: "Causes the system to clone traffic to the specified pool, pool member or vlan regardless of monitor status. (Pool member status may be determined by the use of the LB::status command. Failure to select a server because none are available may be prevented by using the active_members command to test the number of active members in the target pool before choosing it.) Any responses to cloned traffic from pool members will be ignored.",
            source: "https://clouddocs.f5.com/api/irules/clone.html",
            examples: "when CLIENT_ACCEPTED {\n   clone nexthop tap_vlan\n}",
            return_value: "clone pool <pool_name> Specifies the pool to which you want to send the cloned traffic.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "clone pool POOL_OBJ (member IP_ADDR (PORT)?)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
