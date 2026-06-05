//! `LINK::lasthop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::lasthop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the MAC address of the last hop.",
            synopsis: &["LINK::lasthop ('id' | 'type' | 'name')?"],
            snippet: "Returns the MAC address of the last hop.\nNote:\n  * In 11.4, you can extend LINK::lasthop with sub-commands to retrieve\n    the lasthop id, type, name, respectively. Without sub-command,\n    LINK::lasthop returns the MAC address as before.",
            source: "https://clouddocs.f5.com/api/irules/LINK__lasthop.html",
            examples: "when CLIENT_ACCEPTED {\n  set lastmac [LINK::lasthop]\n  session add uie [IP::client_addr] $lastmac 180\n}",
            return_value: "LINK::lasthop [id]",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LINK::lasthop ('id' | 'type' | 'name')?" },
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
