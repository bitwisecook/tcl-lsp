//! `CRYPTO::decrypt` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )" },
        ],
        options: &[
            OptionSpec { name: "-alg", takes_value: true, value_hint: "ALG", detail: "Decryption algorithm.", dialects: None },
            OptionSpec { name: "-ctx", takes_value: true, value_hint: "CTX_VAR", detail: "Context variable for multi-step operations.", dialects: None },
            OptionSpec { name: "-final", takes_value: false, value_hint: "", detail: "Finalize context-based operation.", dialects: None },
            OptionSpec { name: "-key", takes_value: true, value_hint: "KEY", detail: "Binary key.", dialects: None },
            OptionSpec { name: "-keyhex", takes_value: true, value_hint: "KEY_HEX", detail: "Hex-encoded key.", dialects: None },
            OptionSpec { name: "-iv", takes_value: true, value_hint: "IV", detail: "Initialization vector (binary).", dialects: None },
            OptionSpec { name: "-ivhex", takes_value: true, value_hint: "IV_HEX", detail: "Initialization vector (hex).", dialects: None },
            OptionSpec { name: "-padding", takes_value: true, value_hint: "PADDING", detail: "Padding mode (pkcs, oaep, none).", dialects: None },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
