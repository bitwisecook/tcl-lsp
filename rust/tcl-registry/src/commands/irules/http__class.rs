//! `HTTP::class` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::class",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Returns or sets the HTTP class selected by the HTTP selector.",
            synopsis: &[
                "HTTP::class",
                "HTTP::class [enable | disable]",
                "HTTP::class [asm | wa]",
                "HTTP::class select <name>",
            ],
            snippet: "Deprecated in v11.4 — replaced by POLICY commands. See sol14381 for details.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__class.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &["HTTP_CLASS_FAILED", "HTTP_CLASS_SELECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
