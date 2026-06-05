//! `SSL::verify_result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::verify_result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the result code for peer certificate verification.",
            synopsis: &["SSL::verify_result (RESULT_CODE)?"],
            snippet: "Gets or sets the result code for peer certificate verification. Result codes use the same values as those of OpenSSL's X509 verify_result (X509_V_ERR_*) definitions.",
            source: "https://clouddocs.f5.com/api/irules/SSL__verify_result.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n    set cert [X509::verify_cert_error_string [SSL::verify_result]]\n}",
            return_value: "SSL::verify_result Gets the result code from peer certificate verification. The returned code uses the same values as those of OpenSSL's X509 verify_result (X509_V_ERR_) definitions.",
        }),
        ..CommandSpec::DEFAULT
    }
}
