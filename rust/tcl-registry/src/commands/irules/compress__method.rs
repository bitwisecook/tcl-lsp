//! `COMPRESS::method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Specifies the preferred compression algorithm.",
            synopsis: &["COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')"],
            snippet: "Specifies the preferred compression algorithm.\n\nCOMPRESS::method prefer [ ’gzip’ | ’deflate’ ]\n    Specifies the preferred compression algorithm, either gzip or deflate.",
            source: "https://clouddocs.f5.com/api/irules/COMPRESS__method.html",
            examples: "when HTTP_REQUEST_SEND {\n COMPRESS::method prefer gzip\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "COMPRESS::method (request | response)? prefer ('gzip' | 'deflate')" },
        ],
        ..CommandSpec::DEFAULT
    }
}
