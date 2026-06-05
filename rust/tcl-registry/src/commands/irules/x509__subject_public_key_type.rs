//! `X509::subject_public_key_type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject_public_key_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the subjectXs public key type of an X509 certificate.",
            synopsis: &["X509::subject_public_key_type CERTIFICATE"],
            snippet: "Returns the subject’s public key type of the specified X509\ncertificate. The returned value can be either RSA, DSA, or unknown.",
            source: "https://clouddocs.f5.com/api/irules/X509__subject_public_key_type.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n  set client_cert [SSL::cert 0]\n  log local0. \"Cert subject - [X509::subject $client_cert]\"\n  log local0. \"Cert public key type - [X509::subject_public_key_type $client_cert]\"\n  if { [X509::subject_public_key_type $client_cert] equals \"unknown\" } {\n    SSL::verify_result 50\n  }\n  set error_code [SSL::verify_result]\n  log local0. \"Cert verify result - [X509::verify_cert_error_string $error_code]\"\n}",
            return_value: "Returns the subject’s public key type of an X509 certificate.",
        }),
        ..CommandSpec::DEFAULT
    }
}
