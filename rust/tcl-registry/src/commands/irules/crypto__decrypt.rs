//! `CRYPTO::decrypt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::decrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This iRules command decrypts data.",
            synopsis: &["CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )"],
            snippet: "This iRules command decrypts data.\n\nCRYPTO::decrypt [-alg <>] [-ctx <> [-final]] [-key[hex] <>] [-iv[hex] <>] [<data>]\n                [-padding <\"pkcs\" | \"oaep\" | \"none\">]\n\n     * decrypts data based on several parameters\n          + alg - algorithm. ASCII string from a given list (see below)\n            The spelling is lowercase and the iRule will fail for anything\n            not in the list. In ctx mode, alg must be given in the first\n            CRYPTO::command and cannot be modified.",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__decrypt.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
