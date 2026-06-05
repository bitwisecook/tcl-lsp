//! `CRYPTO::verify` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::verify",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Verifies a signed block of data.",
            synopsis: &["CRYPTO::verify (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'"],
            snippet: "This iRules command is used to verify a signed block of data.\n\nCRYPTO::verify [-alg <>] [-ctx <> [-final]] [-key[hex] [-signature <>] [<data>]\n\n     * Used to provide a digital signature of a block of data. Notes on\n       the flags:\n          + alg - algorithm. ASCII string from a given list (see below)\n            The spelling is lowercase and the iRule will fail for anything\n            not in the list. In ctx mode, alg must be given in the first\n            CRYPTO::command and cannot be modified.",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__verify.html",
            examples: "set secret_key \"foobar1234\"\n\nset data \"This is my data\"\n\nset signed_data [CRYPTO::sign -alg hmac-sha1 -key $secret_key $data]\n\nif { [CRYPTO::verify -alg hmac-sha1 -key $secret_key -signature $signed_data $data] } {\n    log local0. \"Data verified\"\n}\n\nThe secret key will normally be some large string, size generally\ndictated by algorithm. The data is just whatever content you want to\nsign. The result of the CRYPTO::sign command will be a binary value, so",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CRYPTO::verify (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'" },
        ],
        options: &[
            OptionSpec { name: "-alg", takes_value: true, value_hint: "ALG", detail: "Verification algorithm.", dialects: None },
            OptionSpec { name: "-ctx", takes_value: true, value_hint: "CTX_VAR", detail: "Context variable for multi-step operations.", dialects: None },
            OptionSpec { name: "-final", takes_value: false, value_hint: "", detail: "Finalize context-based operation.", dialects: None },
            OptionSpec { name: "-key", takes_value: true, value_hint: "KEY", detail: "Binary key.", dialects: None },
            OptionSpec { name: "-keyhex", takes_value: true, value_hint: "KEY_HEX", detail: "Hex-encoded key.", dialects: None },
            OptionSpec { name: "-signature", takes_value: true, value_hint: "SIGNATURE", detail: "Signature to verify against.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
