//! `GENERICMESSAGE::route` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GENERICMESSAGE::route",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Adds, deletes, or looks up message routes.",
            synopsis: &["GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)"],
            snippet: "The GENERICMESSAGE::route command allows you to add, delete, or lookup\nmessage routes.",
            source: "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__route.html",
            examples: "when CLIENT_ACCEPTED {\n    GENERICMESSAGE::route add dst \"client-[IP::remote_addr]\" host \"[IP::remote_addr]:[TCP::remote_port]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
