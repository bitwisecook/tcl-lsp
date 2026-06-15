//! `POP3::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "POP3::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enable POP3 (STARTTLS for POP3).",
            synopsis: &["POP3::enable"],
            snippet: "Enable POP3 (STARTTLS for POP3)",
            source: "https://clouddocs.f5.com/api/irules/POP3__enable.html",
            examples: "when CLIENT_ACCEPTED {\n                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    POP3::enable\n                }\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "POP3::enable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
