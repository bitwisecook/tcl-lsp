//! `http_header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Evaluates the string following an HTTP header tag that you specify.",
            synopsis: &["http_header"],
            snippet: "Evaluates the string following an HTTP header tag that you specify.\nThis command is a BIG-IP version 4.X variable, provided for\nbackward-compatibility. You can use the equivalent 9.X command\nHTTP::header, instead.",
            source: "https://clouddocs.f5.com/api/irules/http_header.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "http_header" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpHeader,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
