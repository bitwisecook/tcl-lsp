//! `htons` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "htons",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary:
                "Converts the unsigned short integer from host byte order to network byte order.",
            synopsis: &["htons NUMBER"],
            snippet:
                "Convert the unsigned short integer from host byte order to network byte\norder.",
            source: "https://clouddocs.f5.com/api/irules/htons.html",
            examples:
                "when HTTP_REQUEST {\n  set hostshort 1234\n  set netshort [htons $hostshort]\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "htons NUMBER",
        }],
        ..CommandSpec::DEFAULT
    }
}
