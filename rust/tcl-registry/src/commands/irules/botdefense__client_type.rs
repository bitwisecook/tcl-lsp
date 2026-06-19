//! `BOTDEFENSE::client_type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::client_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the client type: browser, mobile application or bot.",
            synopsis: &["BOTDEFENSE::client_type"],
            snippet: "Returns the client type. The returned value is one of the following strings:\n    * bot - if the client was detected as a bot.\n    * mobile_app - if the client is a mobile app using F5 Anti Bot mobile SDK.\n    * browser - if the client is a Web browser.\n    * uncategorized - if the client type could not be determined.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__client_type.html",
            examples: "EXAMPLE: Redirect bots to a honeypot page\n when BOTDEFENSE_ACTION {\n     if {[BOTDEFENSE::client_type] eq \"bot\"} {\n         set log \"Request from a Bot on \"\n         append log \"IP [IP::client_addr]\"\n         HSL::send $hsl $log\n         HTTP::redirect \"https://www.example.com/honeypot.html\"\n      }\n }",
            return_value: "A string signifying the client type.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "BOTDEFENSE::client_type",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}
