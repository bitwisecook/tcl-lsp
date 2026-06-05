//! `X509::verify_cert_error_string` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::verify_cert_error_string",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns an X509 certificate error string.",
            synopsis: &["X509::verify_cert_error_string ERROR_CODE"],
            snippet: "Returns the same result as the OpenSSL function\nX509_verify_cert_error_string(). Values for the <X509 verify error\ncode> argument must be the same values as those that the SSL::verify\nresult command returns.",
            source: "https://clouddocs.f5.com/api/irules/X509__verify_cert_error_string.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n  set cert [SSL::cert 0]\n  log local0. \"Cert subject - [X509::subject $cert]\"\n  set error_code [SSL::verify_result]\n  log local0. \"Cert verify result - [X509::verify_cert_error_string $error_code]\"\n}",
            return_value: "Returns an X509 certificate error string.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::verify_cert_error_string ERROR_CODE" },
        ],
        ..CommandSpec::DEFAULT
    }
}
