//! `X509::subject` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the subject of an X509 certificate.",
            synopsis: &["X509::subject CERTIFICATE (commonName)?"],
            snippet: "Returns the subject of the specified X509 certificate.\nIf commonName RDN is specified, returns the Subject CN in UTF8 format.",
            source: "https://clouddocs.f5.com/api/irules/X509__subject.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n\n  # Check if the client supplied one or more client certs\n  if {[SSL::cert count] > 0}{\n\n    # Check the first client cert subject\n    if { [X509::subject [SSL::cert 0]] equals \"someSubject\" } {\n      log local0. \"X509 Certificate Subject [X509::subject [SSL::cert 0]]\"\n      pool my_pool\n    }\n    # Check the first client cert subject commonName\n    if { [X509::subject [SSL::cert 0] commonName] equals \"someCommonName\" } {",
            return_value: "Returns the subject of an X509 certificate. If commonName RDN is specified, returns the Subject CN in UTF8 format.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::subject CERTIFICATE (commonName)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
