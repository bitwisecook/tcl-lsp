//! `decode_uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "decode_uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Decodes the specified string using HTTP URI encoding.",
            synopsis: &["decode_uri ANY_CHARS"],
            snippet: "Decodes the specified string using HTTP URI encoding per RFC2616 and\nreturns the result. This is a BIG-IP 4.x variable, provided for\nbackward-compatibiliy. You can use the equivalent 9.X commmand\nURI::decode instead.",
            source: "https://clouddocs.f5.com/api/irules/decode_uri.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "decode_uri ANY_CHARS" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        deprecated_replacement: Some("URI::decode"),
        traits: Traits::IS_UNESCAPE,
        ..CommandSpec::DEFAULT
    }
}
