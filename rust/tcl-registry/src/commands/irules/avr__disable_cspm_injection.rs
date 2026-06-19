//! `AVR::disable_cspm_injection` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AVR::disable_cspm_injection",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables CSPM injection for the current connection.",
            synopsis: &["AVR::disable_cspm_injection"],
            snippet: "The CSPM (Client Side Performance Monitoring) feature injects JavaScript into HTTP responses to track the Page Load Time metric. This command disables CSPM JavaScropt injection.",
            source: "https://clouddocs.f5.com/api/irules/AVR__disable_cspm_injection.html",
            examples: "when HTTP_RESPONSE {\n    if { [HTTP::status] == 404 } {\n        AVR::disable_cspm_injection\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &["AVR_CSPM_INJECTION"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AVR::disable_cspm_injection",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LogIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
