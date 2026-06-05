//! `http_host` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_host",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of the HTTP Host header.",
            synopsis: &["http_host"],
            snippet: "Returns the value in the Host: header of the HTTP request. This is a\nBIG-IP version 4.X variable, provided for backward-compatibility. You\ncan use the equivalent 9.X command HTTP::host instead.",
            source: "https://clouddocs.f5.com/api/irules/http_host.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "http_host" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpHeader,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
