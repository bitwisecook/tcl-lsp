//! `COMPRESS::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables compression for the current HTTP response.",
            synopsis: &["COMPRESS::enable (request | response)?"],
            snippet: "COMPRESS::enable\n    Enables compression for the current HTTP response. Note that when using this command, you must set the HTTP profile setting Compression to Selective.\n\nCOMPRESS::enable request\n    Enables compression for the current HTTP request. As with the normal COMPRESS::enable, you must enable the HTTP compression profile setting Selective Compression (version 11.x). You must also enable an HTTP profile with \"request-chunking selective\" selected.",
            source: "https://clouddocs.f5.com/api/irules/COMPRESS__enable.html",
            examples: "when HTTP_RESPONSE {\n  if { [HTTP::header Content-Type] contains \"text/html;charset=UTF-8\"} {\n    COMPRESS::enable\n  }\n}",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "COMPRESS::enable (request | response)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
