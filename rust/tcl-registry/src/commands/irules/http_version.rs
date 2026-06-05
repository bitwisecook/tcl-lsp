//! `http_version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the HTTP protocol version.",
            synopsis: &["http_version"],
            snippet: "Returns the HTTP protocol version. Possible values are \"HTTP/1.0\" or\n\"HTTP/1.1\". This is a BIG-IP version 4.X variable, provided for\nbackward compatibility. You can use the equivalent 9.X command,\nHTTP::version instead.",
            source: "https://clouddocs.f5.com/api/irules/http_version.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "http_version" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpStatus,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
