//! `vlan_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vlan_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the VLAN tag of the packet.",
            synopsis: &["vlan_id"],
            snippet: "Returns the VLAN tag of the packet. This is a BIG-IP 4.X variable,\nprovided for backward-compatibility. You can use the equivalent 9.X\ncommand LINK::vlan_id instead.",
            source: "https://clouddocs.f5.com/api/irules/vlan_id.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "vlan_id" },
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
