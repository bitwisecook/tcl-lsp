//! `imid` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "imid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns an i-mode identifier string.",
            synopsis: &["imid"],
            snippet: "Parses the BIG-IP 4.X http_uri variable and the user-agent header field to return an i-mode identifier string that can be used for i-mode session persistence. This is a BIG-IP 4.X function, provided for backward compatibility.\n\nThe imid function takes no arguments and simply returns the string representing the i-mode identifier or the empty string, if none is found.",
            source: "https://clouddocs.f5.com/api/irules/imid.html",
            examples: "",
            return_value: "Parses the BIG-IP 4.X http_uri variable and the '''user-agent* header field to return an i-mode identifier string that can be used for i-mode session persistence.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "imid" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        deprecated_replacement: Some("IMID::id"),
        ..CommandSpec::DEFAULT
    }
}
