//! `CRYPTO::hash` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::hash",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Generates a hash on a piece of data.",
            synopsis: &["CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'"],
            snippet: "This iRules command generates a hash on a piece of data\n\nCRYPTO::hash [-alg <>] [-ctx <> [-final]] [<data>]\n\n     * Generates a hash on a piece of data\n\nAlgorithm List\n\n     * md5\n     * ripemd160\n     * sha1\n     * sha224\n     * sha256\n     * sha384\n     * sha512",
            source: "https://clouddocs.f5.com/api/irules/CRYPTO__hash.html",
            examples: "when HTTP_REQUEST {\nif {[class match [b64encode [CRYPTO::hash -alg sha384 [HTTP::host][HTTP::path]]] equals HASH ]} {\n    log local0. \" this FQDN + PATH is mathing - [HTTP::host][HTTP::path]\"\n}\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'" },
        ],
        options: &[
            OptionSpec { name: "-alg", takes_value: true, value_hint: "ALG", detail: "Hash algorithm.", dialects: None },
            OptionSpec { name: "-ctx", takes_value: true, value_hint: "CTX_VAR", detail: "Context variable for multi-step operations.", dialects: None },
            OptionSpec { name: "-final", takes_value: false, value_hint: "", detail: "Finalize context-based operation.", dialects: None },
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
