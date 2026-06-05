//! `SMTPS::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SMTPS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enable SMTPS (STARTTLS for SMTP).",
            synopsis: &["SMTPS::enable"],
            snippet: "Enable SMTPS (STARTTLS for SMTP)",
            source: "https://clouddocs.f5.com/api/irules/SMTPS__enable.html",
            examples: "when CLIENT_ACCEPTED {\n                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    SMTPS::enable\n                }\n            }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SMTPS::enable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
