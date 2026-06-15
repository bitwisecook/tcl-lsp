//! `AES::encrypt` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AES::encrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Encrypts the data using the previously-created AES key.",
            synopsis: &["AES::encrypt KEY DATA"],
            snippet: "Encrypt the data using an AES key.",
            source: "https://clouddocs.f5.com/api/irules/AES__encrypt.html",
            examples: "when SERVER_DATA {\n  set key \"AES 128 43047ad71173be644498b98de6a32fe3\"\n  set encryptedData [AES::encrypt $key [TCP::payload]]\n  TCP::payload replace 0 [TCP::payload length] $encryptedData\n}",
            return_value: "Returns the encrypted data.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AES::encrypt KEY DATA",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        ..CommandSpec::DEFAULT
    }
}
