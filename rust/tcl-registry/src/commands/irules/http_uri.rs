//! `http_uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a URL, but does not include the protocol and the fully qualified domain name (FQDN).",
            synopsis: &["http_uri"],
            snippet: "Returns a URL, but does not include the protocol and the fully\nqualified domain name (FQDN). For example, if the URL is\nhttp://www.mysite.com/buy.asp, then the URI is /buy.asp. This command\nis a BIG-IP 4.X variable, provided for backward-compatibility. You can\nuse the equivalent 9.x command HTTP::uri instead.",
            source: "https://clouddocs.f5.com/api/irules/http_uri.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "http_uri" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpUri,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        deprecated_replacement: Some("HTTP::uri"),
        ..CommandSpec::DEFAULT
    }
}
