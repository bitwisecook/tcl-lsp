//! `AES::key` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AES::key",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Creates an AES key to encrypt/decrypt data.",
            synopsis: &["AES::key ('128' | '192' | '256')?"],
            snippet: "Creates an AES key of the specified length for use in\nencryption/decryption operations.",
            source: "https://clouddocs.f5.com/api/irules/AES__key.html",
            examples: "when RULE_INIT {\n    set ::key [AES::key 128]\n}",
            return_value: "Returns the created key.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AES::key ('128' | '192' | '256')?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
