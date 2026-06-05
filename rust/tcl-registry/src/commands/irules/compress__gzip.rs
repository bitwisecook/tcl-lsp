//! `COMPRESS::gzip` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::gzip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets HTTP data compression criteria.",
            synopsis: &["COMPRESS::gzip (request | response)? ("],
            snippet: "Sets criteria for compressing HTTP responses.\n\nCOMPRESS::gzip memory_level <level>\n    Sets the gzip memory level.\n\nCOMPRESS::gzip window_size <size>\n    Sets the gzip window size.\n\nCOMPRESS::gzip level <level>\n    Specifies the amount and rate of compression.",
            source: "https://clouddocs.f5.com/api/irules/COMPRESS__gzip.html",
            examples: "when HTTP_RESPONSE {\n  COMPRESS::gzip level 9\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
