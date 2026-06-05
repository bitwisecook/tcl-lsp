//! `LINK::nexthop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::nexthop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the MAC address of the next hop.",
            synopsis: &["LINK::nexthop ('id' | 'type' | 'name')?"],
            snippet: "Returns the MAC address of the next hop. Returns the broadcast address\nff:ff:ff:ff:ff:ff when called before a serverside connection has been\nestablished.\nNote:\n  * In 11.4, you can use LINK::nexthop with sub-commands to retrieve\n    the id, type and name of the next hop, respectively. Without\n    sub-commands, LINK::nexthop returns the MAC address as before.",
            source: "https://clouddocs.f5.com/api/irules/LINK__nexthop.html",
            examples: "# Logging example\nwhen CLIENT_ACCEPTED {\n        log local0. \"\\[LINK::lasthop\\]: [LINK::lasthop], \\[LINK::nexhop\\]: [LINK::nexthop]\"\n}",
            return_value: "LINK::nexthop [id]",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LINK::nexthop ('id' | 'type' | 'name')?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
