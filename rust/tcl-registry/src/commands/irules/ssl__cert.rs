//! `SSL::cert` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::cert",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns data about an X509 SSL certificate, or sets the certificate mode.",
            synopsis: &["SSL::cert <index>", "SSL::cert count", "SSL::cert issuer <index>", "SSL::cert mode ?ignore|request|require?"],
            snippet: "Returns data about an X509 SSL certificate, or sets the certificate mode.",
            source: "https://clouddocs.f5.com/api/irules/SSL__cert.html",
            examples: "when RULE_INIT {\n    set ::key [AES::key 128]\n}",
            return_value: "SSL::cert <index> Returns the X509 SSL certificate at the specified index in the peer certificate chain, where index is a value greater than or equal to zero. A value of zero denotes the first certificate in the chain, a value of one denotes the next, and so on.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::cert <subcommand|index> ?args?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
