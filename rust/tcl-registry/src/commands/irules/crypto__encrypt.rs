//! `CRYPTO::encrypt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::encrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This iRules command encrypts data.",
            synopsis: &["CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )"],
            snippet: "This iRules command encrypts data. A ciphertext encrypted with this\ncommand should be decryptable by third party software.\n\nCRYPTO::encrypt [-alg <>] [-ctx <> [-final]] [-key[hex] <>] [-iv[hex] <>] [<data>]\n                [-padding <\"pkcs\" | \"oaep\" | \"none\">]\n\n     * encrypts data based on several parameters\n          + alg - algorithm. ASCII string from a given list (see below)\n            The spelling is lowercase and the iRule will fail for anything\n            not in the list. In ctx mode, alg must be given in the first\n            CRYPTO:: command and cannot be modified.",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__encrypt.html",
            examples: "Encrypt an MSISDN header\n# Encrypt the MSISDN header for each request.\n# The encryption is deliberately designed to be insecure;\n# that is, the same MSISDN will always be encrypted to\n# the same ciphertext. And since the IV will always be\n# the same for each encryption, there's no need to send\n# it out with the ciphertext.\n#\nwhen SIP_REQUEST {\n    set key \"abed1ddc04fbb05856bca4a0ca60f21e\"\n    set iv \"d78d86d9084eb9239694c9a733904037\"",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
