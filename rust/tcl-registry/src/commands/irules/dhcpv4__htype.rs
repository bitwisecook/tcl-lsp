//! `DHCPv4::htype` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::htype",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns htype (hardware type) field from DHCPv4 message.",
            synopsis: &["DHCPv4::htype"],
            snippet: "This command returns htype (hardware type) field from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::htype",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__htype.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Htype [DHCPv4::htype]\"\n    }",
            return_value: "This command returns htype (hardware type) field from DHCPv4 message",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::htype" },
        ],
        ..CommandSpec::DEFAULT
    }
}
