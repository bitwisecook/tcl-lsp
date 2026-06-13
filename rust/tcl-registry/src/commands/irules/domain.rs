//! `domain` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "domain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Parses the specified string as a dotted domain name and returns the last portions of the domain name.",
            synopsis: &["domain DOMAIN COUNT"],
            snippet: "A custom iRule function which parses the specified string as a\ndotted domain name and returns the last <count> portions of the domain\nname.",
            source: "https://clouddocs.f5.com/api/irules/domain.html",
            examples: "when HTTP_REQUEST\nif { [HTTP::uri] ends_with \".html\" } {\n      pool cache_pool\n      set key [crc32 [concat [domain [HTTP::host] 2] [HTTP::uri]]]\n}\n...\n\nThis code:\n\n log local0. [domain www.sub.my.domain.com 1]   ; # result: com\n log local0. [domain www.sub.my.domain.com 2]   ; # result: domain.com\n log local0. [domain www.sub.my.domain.com 3]   ; # result: my.domain.com\n log local0. [domain www.sub.my.domain.com 4]   ; # result: sub.my.domain.com",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "domain DOMAIN COUNT" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
