//! `GENERICMESSAGE::peer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GENERICMESSAGE::peer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets the peer's route name.",
            synopsis: &["GENERICMESSAGE::peer name (NAME)?"],
            snippet: "The GENERICMESSAGE::peer command returns or sets the peer's route name\nin the message routing framework. The peer name will be automatically\nset as the source address of each message.",
            source: "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__peer.html",
            examples: "when CLIENT_ACCEPTED {\n    GENERICMESSAGE::peer name \"[IP::remote_addr]:[TCP::remote_port]\"\n}",
            return_value: "Returns the peer's route name.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GENERICMESSAGE::peer name (NAME)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
