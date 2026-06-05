//! `CRYPTO::keygen` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::keygen",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Generates keys that can be used to encrypt and sign data.",
            synopsis: &["CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))"],
            snippet: "This iRules command is used to generate keys that can be used to\nencrypt and sign data.\n\nCRYPTO::keygen -alg <> -len <> [-passphrase <> -salt[hex] <> -rounds <>]\n\n     * Used to generate keys that can be used to encrypt and sign data.\n          + -alg (Two options: random or pbkdf2-md5)\n          + -len (Must be a multiple of 8, e.g.",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__keygen.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
