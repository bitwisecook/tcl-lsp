//! `X509::extensions` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::extensions",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the X509 extensions set on an X509 certificate.",
            synopsis: &["X509::extensions CERTIFICATE"],
            snippet: "Returns the X509 extensions set on the specified X509 certificate.",
            source: "https://clouddocs.f5.com/api/irules/X509__extensions.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n    set myCert [SSL::cert 0]\n    set result [X509::extensions $myCert]\n    log local0. \"X509::extensions $result\"\n\n    if { $result matches_glob \"*X509v3 extensions:*X509v3 Basic*\" } {\n        log local0. \"match\"\n    } else {\n        log local0. \"no match\"\n    }\n}",
            return_value: "Returns the X509 extensions set on an X509 certificate.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "X509::extensions CERTIFICATE" },
        ],
        ..CommandSpec::DEFAULT
    }
}
