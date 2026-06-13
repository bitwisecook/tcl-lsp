//! `LINK::qos` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LINK::qos",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the QoS level set for the current packet.",
            synopsis: &["LINK::qos"],
            snippet: "Returns the QoS level set for the current packet.\nThe Quality of Service (QoS) standard is a means by which network\nequipment can identify and treat traffic differently based on an\nidentifier.\nThis command can be used to direct traffic based on the QoS level\nwithin a packet.\nThis command is equivalent to the BIG-IP 4.X variable link_qos.",
            source: "https://clouddocs.f5.com/api/irules/LINK__qos.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [LINK::qos] > 2 } {\n     pool fast_pool\n  } else {\n     pool slow_pool\n }\n}",
            return_value: "LINK::qos",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LINK::qos" },
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
