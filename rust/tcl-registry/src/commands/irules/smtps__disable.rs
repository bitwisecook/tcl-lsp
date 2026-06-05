//! `SMTPS::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SMTPS::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disable SMTPS (STARTTLS for SMTP).",
            synopsis: &["SMTPS::disable"],
            snippet: "Disable SMTPS (STARTTLS for SMTP)",
            source: "https://clouddocs.f5.com/api/irules/SMTPS__disable.html",
            examples: "when CLIENT_ACCEPTED {\n                if { !([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    SMTPS::disable\n                }\n            }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SMTPS::disable" },
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
