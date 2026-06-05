//! `X509::cert_fields` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::cert_fields",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a list of X509 certificate fields to be added to HTTP headers for ModSSL behavior.",
            synopsis: &["X509::cert_fields CERTIFICATE ERROR_CODE ((hash"],
            snippet: "When given a valid certificate, returns a TCL list of field names and\nvalues which can be added to the HTTP headers in order to emulate\nModSSL behavior. The output can be passed to 'HTTP::header insert\n$list' as a list for insertion in the HTTP request or response.",
            source: "https://clouddocs.f5.com/api/irules/X509__cert_fields.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n    if { [SSL::cert count] > 0 } {\n        session add ssl [SSL::sessionid] [X509::cert_fields [SSL::cert 0] [SSL::verify_result] whole] $timeout\n    }\n}",
            return_value: "Returns a list of X509 certificate fields to be added to HTTP headers.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::cert_fields CERTIFICATE ERROR_CODE ((hash" },
        ],
        ..CommandSpec::DEFAULT
    }
}
