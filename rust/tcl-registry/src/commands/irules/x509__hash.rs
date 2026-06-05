//! `X509::hash` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::hash",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the MD5 hash (fingerprint) of an X509 certificate.",
            synopsis: &["X509::hash CERTIFICATE"],
            snippet: "Returns the MD5 hash (fingerprint) of the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__hash.html",
            examples: "when HTTP_REQUEST {\n  if { [info exist cert_hash] } {\n    if { $cert_hash equals \"XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX\"} {\n      HTTP::redirect \"https://somesite/\"\n    } else {\n      HTTP::redirect \"https://someothersite/\"\n    }\n  }\n}",
            return_value: "Returns the MD5 hash (fingerprint) of an X509 certificate.",
        }),
        ..CommandSpec::DEFAULT
    }
}
