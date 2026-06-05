//! `AES::decrypt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AES::decrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Decrypts the data using the previously-created AES key.",
            synopsis: &["AES::decrypt KEY DATA"],
            snippet: "Decrypt the data using an AES key.",
            source: "https://clouddocs.f5.com/api/irules/AES__decrypt.html",
            examples: "when HTTP_REQUEST {\n  set key \"AES 128 43047ad71173be644498b98de6a32fe3\"\n  set decryptedData [AES::decrypt $key $encryptedData]\n  log local0. \"The decrypted data is $decryptedData\"\n}",
            return_value: "Returns the decrypted data.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AES::decrypt KEY DATA" },
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
