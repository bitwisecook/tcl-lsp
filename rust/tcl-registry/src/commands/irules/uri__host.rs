//! `URI::host` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::host",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the host portion of a given URI.",
            synopsis: &["URI::host URI_STRING"],
            snippet: "Returns the host portion of a given URI.",
            source: "https://clouddocs.f5.com/api/irules/URI__host.html",
            examples: "when RULE_INIT {\n        # Loop through some test URLs and URIs and log the URI::host value\n        foreach uri [list \\\n                http://example.com/file.ext \\\n                http://example.com:80/file.ext \\\n                https://example.com:443/file.ext \\\n                ftp://example.com/file.ext \\\n                sip://example.com/file.ext \\\n                myproto://example.com/file.ext \\\n                /example.com \\\n                /uri?url=http://example.com/uri \\\n        ] {",
            return_value: "Returns the host portion of a given URI.",
        }),
        ..CommandSpec::DEFAULT
    }
}
