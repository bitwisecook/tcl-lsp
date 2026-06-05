//! `X509::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the version number of an X509 certificate.",
            synopsis: &["X509::version CERTIFICATE"],
            snippet: "Returns the version number of the specified X509 certificate (an\ninteger).",
            source: "https://clouddocs.f5.com/api/irules/X509__version.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"Cert version - [X509::version ssl_cert]\"\n  if { [X509::version ssl_cert] eq 3 } {\n    pool v3_pool\n  } else {\n    pool default_pool\n  }\n}",
            return_value: "Returns the version number of an X509 certificate.",
        }),
        ..CommandSpec::DEFAULT
    }
}
