//! `CRYPTO::keygen` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
        }],
        options: &[
            OptionSpec {
                name: "-alg",
                takes_value: true,
                value_hint: "ALG",
                detail: "Key generation algorithm.",
                dialects: None,
            },
            OptionSpec {
                name: "-len",
                takes_value: true,
                value_hint: "LENGTH",
                detail: "Key length (must be multiple of 8).",
                dialects: None,
            },
            OptionSpec {
                name: "-exp",
                takes_value: true,
                value_hint: "EXPONENT",
                detail: "Exponent (for RSA).",
                dialects: None,
            },
            OptionSpec {
                name: "-passphrase",
                takes_value: true,
                value_hint: "PASSPHRASE",
                detail: "Passphrase for key derivation.",
                dialects: None,
            },
            OptionSpec {
                name: "-salt",
                takes_value: true,
                value_hint: "SALT",
                detail: "Binary salt.",
                dialects: None,
            },
            OptionSpec {
                name: "-salthex",
                takes_value: true,
                value_hint: "SALT_HEX",
                detail: "Hex-encoded salt.",
                dialects: None,
            },
            OptionSpec {
                name: "-rounds",
                takes_value: true,
                value_hint: "ROUNDS",
                detail: "Rounds for PBKDF2.",
                dialects: None,
            },
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
