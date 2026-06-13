//! `SSL::secure_renegotiation` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::secure_renegotiation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Controls the SSL Secure Renegotiation mode.",
            synopsis: &["SSL::secure_renegotiation (request | require | require-strict)?"],
            snippet: "Controls the SSL Secure Renegotiation mode.",
            source: "https://clouddocs.f5.com/api/irules/SSL__secure_renegotiation.html",
            examples: "when CLIENTSSL_CLIENTHELLO {\n                if { [SSL::secure_renegotiation] != 2 } {\n                    SSL::secure_renegotiation require-strict\n                }\n            }",
            return_value: "SSL::secure_renegotiation¶ Get the current Secure Renegotiation mode for the flow. A return value of zero denotes request mode. A value of one denotes require mode. A value of two denotes require-strict mode.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::secure_renegotiation (request | require | require-strict)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
