//! `POP3::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "POP3::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disable POP3 (STARTTLS for POP3).",
            synopsis: &["POP3::disable"],
            snippet: "Disable POP3 (STARTTLS for POP3)",
            source: "https://clouddocs.f5.com/api/irules/POP3__disable.html",
            examples: "when CLIENT_ACCEPTED {\n                if { !([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    POP3::disable\n                }\n            }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "POP3::disable" },
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
