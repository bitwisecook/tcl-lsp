//! `COMPRESS::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables compression for the current HTTP response.",
            synopsis: &["COMPRESS::disable (request | response)?"],
            snippet: "Disables compression for the current HTTP response. Note that when using this command, you must set the HTTP profile setting Compression to Selective.\n\nCOMPRESS::disable\n    Disables compression for the current HTTP response. Note that when using this command, you must set the HTTP profile setting Compression to Selective.",
            source: "https://clouddocs.f5.com/api/irules/COMPRESS__disable.html",
            examples: "when HTTP_REQUEST {\n  if { [TCP::mss] >= 1280 } {\n    COMPRESS::disable\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
